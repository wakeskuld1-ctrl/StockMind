use crate::ops::stock::security_execution_journal::SecurityExecutionJournalDocument;
use crate::ops::stock::security_execution_record::{
    SecurityExecutionRecordError, SecurityExecutionRecordRequest,
};
use crate::ops::stock::security_portfolio_position_plan::PortfolioAllocationRecommendation;

// 2026-04-14 CST: Extracted from security_execution_record.rs because round 2 needs
// formal account-plan lookup isolated from the execution-record assembler.
// Purpose: keep account binding and budget-drift rules on one dedicated boundary
// so later account/runtime cleanup does not reopen the record builder body.
#[derive(Debug, Clone)]
pub(crate) struct AccountPlanBinding {
    pub(crate) portfolio_position_plan_ref: String,
    pub(crate) allocation: PortfolioAllocationRecommendation,
}

// 2026-04-14 CST: Extracted from security_execution_record.rs because round 2 needs
// planned-vs-actual account execution drift summarized outside the assembler.
// Purpose: let review/package/account layers reuse one governed drift object.
#[derive(Debug, Clone)]
pub(crate) struct AccountExecutionSummary {
    pub(crate) planned_tranche_action: String,
    pub(crate) planned_tranche_pct: f64,
    pub(crate) planned_peak_position_pct: f64,
    pub(crate) actual_tranche_action: String,
    pub(crate) actual_tranche_pct: f64,
    pub(crate) actual_peak_position_pct: f64,
    pub(crate) tranche_count_drift: i32,
    pub(crate) account_budget_alignment: String,
}

// 2026-04-14 CST: Extracted because round 2 separates portfolio-account binding
// from document assembly; purpose: make the verified mainline read like
// orchestrator -> assembler -> resolver instead of one large blended file.
pub(crate) struct SecurityExecutionAccountBindingResolver;

impl SecurityExecutionAccountBindingResolver {
    pub(crate) fn bind(
        request: &SecurityExecutionRecordRequest,
        symbol: &str,
    ) -> Result<Option<AccountPlanBinding>, SecurityExecutionRecordError> {
        let Some(portfolio_position_plan_document) = &request.portfolio_position_plan_document
        else {
            return Ok(None);
        };
        let allocation = portfolio_position_plan_document
            .allocations
            .iter()
            .find(|item| item.symbol == symbol)
            .cloned()
            .ok_or_else(|| {
                SecurityExecutionRecordError::Build(format!(
                    "portfolio position plan {} missing allocation for symbol {}",
                    portfolio_position_plan_document.portfolio_position_plan_id, symbol
                ))
            })?;
        Ok(Some(AccountPlanBinding {
            portfolio_position_plan_ref: portfolio_position_plan_document
                .portfolio_position_plan_id
                .clone(),
            allocation,
        }))
    }

    pub(crate) fn summarize_execution(
        binding: &AccountPlanBinding,
        execution_journal: &SecurityExecutionJournalDocument,
    ) -> AccountExecutionSummary {
        let planned_tranche_action = binding.allocation.suggested_tranche_action.clone();
        let planned_tranche_pct = binding.allocation.suggested_tranche_pct.max(0.0);
        let planned_peak_position_pct = binding.allocation.target_position_pct.max(0.0);
        let actual_peak_position_pct = (binding.allocation.current_position_pct
            + execution_journal.peak_position_pct)
            .max(0.0);
        let actual_tranche_pct =
            (actual_peak_position_pct - binding.allocation.current_position_pct).max(0.0);
        let actual_tranche_action = if actual_tranche_pct <= 1e-9 {
            "hold".to_string()
        } else if binding.allocation.current_position_pct > 1e-9 {
            "add_tranche".to_string()
        } else {
            "entry_tranche".to_string()
        };
        let planned_tranche_units = if planned_tranche_pct > 1e-9 { 1 } else { 0 };
        let actual_tranche_units =
            tranche_units_for_account_plan(planned_tranche_pct, actual_tranche_pct);
        let tranche_count_drift = actual_tranche_units as i32 - planned_tranche_units as i32;
        let account_budget_alignment = classify_account_budget_alignment(
            &planned_tranche_action,
            planned_tranche_pct,
            &actual_tranche_action,
            actual_tranche_pct,
        );

        AccountExecutionSummary {
            planned_tranche_action,
            planned_tranche_pct,
            planned_peak_position_pct,
            actual_tranche_action,
            actual_tranche_pct,
            actual_peak_position_pct,
            tranche_count_drift,
            account_budget_alignment,
        }
    }
}

fn classify_account_budget_alignment(
    planned_tranche_action: &str,
    planned_tranche_pct: f64,
    actual_tranche_action: &str,
    actual_tranche_pct: f64,
) -> String {
    if planned_tranche_action != actual_tranche_action {
        return "direction_mismatch".to_string();
    }
    let tranche_gap = actual_tranche_pct - planned_tranche_pct;
    if tranche_gap > 0.005 {
        "over_budget".to_string()
    } else if tranche_gap < -0.005 {
        "under_budget".to_string()
    } else {
        "aligned".to_string()
    }
}

fn tranche_units_for_account_plan(planned_tranche_pct: f64, actual_tranche_pct: f64) -> usize {
    if planned_tranche_pct <= 1e-9 || actual_tranche_pct <= 1e-9 {
        return 0;
    }
    (actual_tranche_pct / planned_tranche_pct).ceil() as usize
}
