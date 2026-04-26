use crate::ops::stock::security_execution_account_binding_resolver::SecurityExecutionAccountBindingResolver;
use crate::ops::stock::security_execution_journal::SecurityExecutionJournalDocument;
use crate::ops::stock::security_execution_record::{
    SecurityExecutionRecordDocument, SecurityExecutionRecordError,
    SecurityExecutionRecordOutcomeBinding, SecurityExecutionRecordRequest,
    build_attribution_summary, classify_execution_quality, compute_holding_days,
    load_planned_entry_price, normalize_created_at, resolve_exit_reason, rounded_pct,
};
use crate::ops::stock::security_open_position_corporate_action_summary::build_open_position_corporate_action_summary;
use crate::ops::stock::security_position_plan::SecurityPositionPlanResult;
use crate::runtime::security_corporate_action_store::SecurityCorporateActionStore;
use crate::runtime::stock_history_store::StockHistoryStore;

// 2026-04-14 CST: Extracted from security_execution_record.rs because round 2 plan B
// requires one dedicated formal assembler module.
// Purpose: keep execution record orchestration thin while preserving one governed
// build path from journal aggregate to runtime-persisted record DTO.
pub(crate) struct SecurityExecutionRecordAssembler<'a> {
    position_plan_result: &'a SecurityPositionPlanResult,
    outcome_binding: &'a SecurityExecutionRecordOutcomeBinding,
    execution_journal: &'a SecurityExecutionJournalDocument,
    request: &'a SecurityExecutionRecordRequest,
}

impl<'a> SecurityExecutionRecordAssembler<'a> {
    pub(crate) fn new(
        position_plan_result: &'a SecurityPositionPlanResult,
        outcome_binding: &'a SecurityExecutionRecordOutcomeBinding,
        execution_journal: &'a SecurityExecutionJournalDocument,
        request: &'a SecurityExecutionRecordRequest,
    ) -> Self {
        Self {
            position_plan_result,
            outcome_binding,
            execution_journal,
            request,
        }
    }

    pub(crate) fn assemble(
        &self,
    ) -> Result<SecurityExecutionRecordDocument, SecurityExecutionRecordError> {
        let store = StockHistoryStore::workspace_default()?;
        let planned_entry_price = load_planned_entry_price(
            &store,
            &self.position_plan_result.position_plan_document.symbol,
            &self.outcome_binding.snapshot.as_of_date,
        )?;
        let planned_position_pct = self
            .position_plan_result
            .position_plan_document
            .starter_position_pct;
        let planned_max_position_pct = self
            .position_plan_result
            .position_plan_document
            .max_position_pct;
        let account_plan_binding = SecurityExecutionAccountBindingResolver::bind(
            self.request,
            &self.position_plan_result.position_plan_document.symbol,
        )?;
        let account_execution_summary = account_plan_binding.as_ref().map(|binding| {
            SecurityExecutionAccountBindingResolver::summarize_execution(
                binding,
                self.execution_journal,
            )
        });
        let position_state = self.execution_journal.position_state.clone();
        let account_id = self.resolve_account_id();
        let sector_tag = account_plan_binding
            .as_ref()
            .map(|binding| binding.allocation.sector_tag.clone())
            .or_else(|| self.request.sector_tag.clone());
        let actual_return = self.execution_journal.realized_return;
        let planned_forward_return = self.outcome_binding.selected_outcome.forward_return;
        let entry_slippage_pct =
            self.execution_journal.weighted_entry_price / planned_entry_price - 1.0;
        let position_size_gap_pct = self.execution_journal.peak_position_pct - planned_position_pct;
        let execution_return_gap = actual_return - planned_forward_return;
        let execution_quality = classify_execution_quality(
            &position_state,
            entry_slippage_pct,
            position_size_gap_pct,
            actual_return,
            planned_forward_return,
            self.execution_journal.peak_position_pct,
            planned_max_position_pct,
        );
        let holding_days = compute_holding_days(
            &self.execution_journal.holding_start_date,
            &self.execution_journal.holding_end_date,
        )?;
        let execution_record_notes = self.resolve_execution_record_notes();
        let exit_reason = resolve_exit_reason(
            &position_state,
            &self.execution_journal.trades,
            &self.request.exit_reason,
        );
        let attribution_summary = build_attribution_summary(
            self.execution_journal.final_position_pct,
            actual_return,
            execution_return_gap,
            entry_slippage_pct,
            position_size_gap_pct,
            &execution_quality,
        );

        let mut execution_record = SecurityExecutionRecordDocument {
            execution_record_id: format!(
                "execution-record-{}-{}",
                self.position_plan_result
                    .position_plan_document
                    .position_plan_id,
                self.execution_journal.holding_start_date
            ),
            contract_version: "security_execution_record.v1".to_string(),
            document_type: "security_execution_record".to_string(),
            generated_at: normalize_created_at(&self.request.created_at),
            symbol: self
                .position_plan_result
                .position_plan_document
                .symbol
                .clone(),
            analysis_date: self
                .position_plan_result
                .position_plan_document
                .analysis_date
                .clone(),
            account_id,
            sector_tag,
            position_state: position_state.clone(),
            portfolio_position_plan_ref: account_plan_binding
                .as_ref()
                .map(|binding| binding.portfolio_position_plan_ref.clone()),
            execution_journal_ref: self.execution_journal.execution_journal_id.clone(),
            position_plan_ref: self
                .position_plan_result
                .position_plan_document
                .position_plan_id
                .clone(),
            snapshot_ref: self.outcome_binding.snapshot.snapshot_id.clone(),
            outcome_ref: self.outcome_binding.selected_outcome.outcome_id.clone(),
            planned_entry_date: self.outcome_binding.snapshot.as_of_date.clone(),
            planned_entry_price,
            planned_position_pct,
            planned_max_position_pct,
            actual_entry_date: self.execution_journal.holding_start_date.clone(),
            actual_entry_price: self.execution_journal.weighted_entry_price,
            actual_position_pct: self.execution_journal.peak_position_pct,
            current_position_pct: self.execution_journal.final_position_pct,
            actual_exit_date: if position_state == "open" {
                String::new()
            } else {
                self.execution_journal.holding_end_date.clone()
            },
            actual_exit_price: if position_state == "open" {
                0.0
            } else {
                self.execution_journal.weighted_exit_price
            },
            exit_reason,
            holding_days,
            planned_forward_return,
            actual_return,
            entry_slippage_pct,
            position_size_gap_pct,
            planned_tranche_action: account_execution_summary
                .as_ref()
                .map(|summary| summary.planned_tranche_action.clone()),
            planned_tranche_pct: account_execution_summary
                .as_ref()
                .map(|summary| rounded_pct(summary.planned_tranche_pct)),
            planned_peak_position_pct: account_execution_summary
                .as_ref()
                .map(|summary| rounded_pct(summary.planned_peak_position_pct)),
            actual_tranche_action: account_execution_summary
                .as_ref()
                .map(|summary| summary.actual_tranche_action.clone()),
            actual_tranche_pct: account_execution_summary
                .as_ref()
                .map(|summary| rounded_pct(summary.actual_tranche_pct)),
            actual_peak_position_pct: account_execution_summary
                .as_ref()
                .map(|summary| rounded_pct(summary.actual_peak_position_pct)),
            tranche_count_drift: account_execution_summary
                .as_ref()
                .map(|summary| summary.tranche_count_drift),
            account_budget_alignment: account_execution_summary
                .as_ref()
                .map(|summary| summary.account_budget_alignment.clone()),
            execution_return_gap,
            execution_quality,
            price_as_of_date: None,
            resolved_trade_date: None,
            current_price: None,
            share_adjustment_factor: None,
            cumulative_cash_dividend_per_share: None,
            dividend_adjusted_cost_basis: None,
            holding_total_return_pct: None,
            breakeven_price: None,
            corporate_action_summary: None,
            replay_commit_idempotency_key: None,
            replay_commit_payload_hash: None,
            replay_commit_source_p19c_ref: None,
            execution_record_notes,
            attribution_summary,
        };
        // 2026-04-16 CST: Added because P0-3 needs execution_record to reuse the formal
        // holding-economics helper for still-open positions.
        // Purpose: align execution_record and snapshot semantics without rewriting the
        // historical actual_return contract in this round.
        self.attach_open_position_holding_economics(&mut execution_record, &store)?;
        Ok(execution_record)
    }

    // 2026-04-16 CST: Added because only open execution records should expose current
    // holding economics on the execution layer.
    // Purpose: keep the enrichment narrow and avoid blending closed-trade attribution
    // with live holding math during the refactor period.
    fn attach_open_position_holding_economics(
        &self,
        execution_record: &mut SecurityExecutionRecordDocument,
        stock_store: &StockHistoryStore,
    ) -> Result<(), SecurityExecutionRecordError> {
        if execution_record.position_state != "open" {
            return Ok(());
        }

        let corporate_action_store = SecurityCorporateActionStore::workspace_default()?;
        apply_open_position_holding_economics(
            execution_record,
            &derive_requested_as_of_date(&self.request.created_at),
            stock_store,
            &corporate_action_store,
        )
    }

    fn resolve_account_id(&self) -> Option<String> {
        self.request.account_id.clone().or_else(|| {
            self.request
                .portfolio_position_plan_document
                .as_ref()
                .map(|document| document.account_id.clone())
        })
    }

    fn resolve_execution_record_notes(&self) -> Vec<String> {
        if self.request.execution_record_notes.is_empty() {
            self.execution_journal.execution_journal_notes.clone()
        } else {
            self.request
                .execution_record_notes
                .iter()
                .map(|item| item.trim())
                .filter(|item| !item.is_empty())
                .map(|item| item.to_string())
                .collect()
        }
    }
}

// 2026-04-16 CST: Added because execution_record created_at is the best available formal
// proxy for "evaluate the still-open holding as of when this record was generated".
// Purpose: avoid incorrectly reusing the original analysis_date as the live holding
// economics date anchor.
fn derive_requested_as_of_date(created_at: &str) -> String {
    let trimmed = created_at.trim();
    if let Some((date_text, _)) = trimmed.split_once('T') {
        return date_text.to_string();
    }
    trimmed.chars().take(10).collect()
}

// 2026-04-16 CST: Added because the P0-3 enrichment needs one store-injected helper that can
// be tested without going through the heavier execution_record CLI chain.
// Purpose: keep the new holding-economics attachment independently verifiable while the outer
// CLI path still has unrelated committee-chain blockers.
fn apply_open_position_holding_economics(
    execution_record: &mut SecurityExecutionRecordDocument,
    requested_as_of_date: &str,
    stock_store: &StockHistoryStore,
    corporate_action_store: &SecurityCorporateActionStore,
) -> Result<(), SecurityExecutionRecordError> {
    if execution_record.position_state != "open" {
        return Ok(());
    }
    let Some(holding_summary) = build_open_position_corporate_action_summary(
        execution_record,
        requested_as_of_date,
        stock_store,
        corporate_action_store,
    )?
    else {
        return Ok(());
    };

    execution_record.price_as_of_date = Some(holding_summary.price_as_of_date);
    execution_record.resolved_trade_date = Some(holding_summary.resolved_trade_date);
    execution_record.current_price = Some(holding_summary.current_price);
    execution_record.share_adjustment_factor = Some(holding_summary.share_adjustment_factor);
    execution_record.cumulative_cash_dividend_per_share =
        Some(holding_summary.cumulative_cash_dividend_per_share);
    execution_record.dividend_adjusted_cost_basis =
        Some(holding_summary.dividend_adjusted_cost_basis);
    execution_record.holding_total_return_pct = Some(holding_summary.holding_total_return_pct);
    execution_record.breakeven_price = Some(holding_summary.breakeven_price);
    execution_record.corporate_action_summary = Some(holding_summary.corporate_action_summary);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::{apply_open_position_holding_economics, derive_requested_as_of_date};
    use crate::ops::stock::security_execution_record::SecurityExecutionRecordDocument;
    use crate::runtime::security_corporate_action_store::{
        SecurityCorporateActionRow, SecurityCorporateActionStore,
    };
    use crate::runtime::stock_history_store::{StockHistoryRow, StockHistoryStore};

    fn temp_db_path(file_name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{file_name}_{nanos}.db"))
    }

    fn open_record_fixture(position_state: &str) -> SecurityExecutionRecordDocument {
        SecurityExecutionRecordDocument {
            execution_record_id: format!("record-{position_state}"),
            contract_version: "security_execution_record.v1".to_string(),
            document_type: "security_execution_record".to_string(),
            generated_at: "2026-04-16T12:00:00+08:00".to_string(),
            symbol: "601916.SH".to_string(),
            analysis_date: "2025-09-17".to_string(),
            account_id: Some("acct-1".to_string()),
            sector_tag: Some("bank".to_string()),
            position_state: position_state.to_string(),
            portfolio_position_plan_ref: None,
            execution_journal_ref: "journal-1".to_string(),
            position_plan_ref: "plan-1".to_string(),
            snapshot_ref: "snapshot-1".to_string(),
            outcome_ref: "outcome-1".to_string(),
            planned_entry_date: "2025-09-17".to_string(),
            planned_entry_price: 62.0,
            planned_position_pct: 0.06,
            planned_max_position_pct: 0.15,
            actual_entry_date: "2025-09-18".to_string(),
            actual_entry_price: 62.4,
            actual_position_pct: 0.12,
            current_position_pct: 0.12,
            actual_exit_date: if position_state == "open" {
                String::new()
            } else {
                "2025-10-02".to_string()
            },
            actual_exit_price: if position_state == "open" { 0.0 } else { 66.1 },
            exit_reason: if position_state == "open" {
                "position_still_open".to_string()
            } else {
                "target_hit".to_string()
            },
            holding_days: 14,
            planned_forward_return: 0.08,
            actual_return: 0.0,
            entry_slippage_pct: 0.0,
            position_size_gap_pct: 0.06,
            planned_tranche_action: None,
            planned_tranche_pct: None,
            planned_peak_position_pct: None,
            actual_tranche_action: None,
            actual_tranche_pct: None,
            actual_peak_position_pct: None,
            tranche_count_drift: None,
            account_budget_alignment: None,
            execution_return_gap: -0.08,
            execution_quality: if position_state == "open" {
                "open_position_pending".to_string()
            } else {
                "aligned".to_string()
            },
            price_as_of_date: None,
            resolved_trade_date: None,
            current_price: None,
            share_adjustment_factor: None,
            cumulative_cash_dividend_per_share: None,
            dividend_adjusted_cost_basis: None,
            holding_total_return_pct: None,
            breakeven_price: None,
            corporate_action_summary: None,
            replay_commit_idempotency_key: None,
            replay_commit_payload_hash: None,
            replay_commit_source_p19c_ref: None,
            execution_record_notes: Vec::new(),
            attribution_summary: "fixture".to_string(),
        }
    }

    #[test]
    fn derive_requested_as_of_date_prefers_rfc3339_date_component() {
        assert_eq!(
            derive_requested_as_of_date("2025-09-22T12:00:00+08:00"),
            "2025-09-22"
        );
    }

    #[test]
    fn apply_open_position_holding_economics_populates_live_fields_for_open_record() {
        let stock_db_path = temp_db_path("execution_record_holding_stock");
        let action_db_path = temp_db_path("execution_record_holding_action");
        let stock_store = StockHistoryStore::new(stock_db_path.clone());
        let corporate_action_store = SecurityCorporateActionStore::new(action_db_path.clone());
        stock_store
            .import_rows(
                "601916.SH",
                "fixture",
                &[StockHistoryRow {
                    trade_date: "2025-09-22".to_string(),
                    open: 63.1,
                    high: 63.6,
                    low: 63.0,
                    close: 63.4,
                    adj_close: 63.4,
                    volume: 800_000,
                }],
            )
            .expect("stock rows should persist");
        corporate_action_store
            .upsert_rows(&[SecurityCorporateActionRow {
                symbol: "601916.SH".to_string(),
                effective_date: "2025-09-20".to_string(),
                action_type: "cash_dividend".to_string(),
                cash_dividend_per_share: 0.5,
                split_ratio: 1.0,
                bonus_ratio: 0.0,
                source: "fixture".to_string(),
                payload_json: "{}".to_string(),
            }])
            .expect("corporate action rows should persist");
        let mut record = open_record_fixture("open");

        apply_open_position_holding_economics(
            &mut record,
            "2025-09-22",
            &stock_store,
            &corporate_action_store,
        )
        .expect("holding economics should attach");

        assert_eq!(record.price_as_of_date.as_deref(), Some("2025-09-22"));
        assert_eq!(record.resolved_trade_date.as_deref(), Some("2025-09-22"));
        assert_eq!(record.current_price, Some(63.4));
        assert_eq!(record.share_adjustment_factor, Some(1.0));
        assert_eq!(record.cumulative_cash_dividend_per_share, Some(0.5));
        assert!(
            record.breakeven_price.expect("breakeven should exist") < record.actual_entry_price,
            "cash dividend should lower the breakeven"
        );
        assert!(
            record
                .corporate_action_summary
                .as_deref()
                .expect("summary should exist")
                .contains("accumulated cash dividend"),
            "summary should explain the dividend effect"
        );

        let _ = fs::remove_file(stock_db_path);
        let _ = fs::remove_file(action_db_path);
    }

    #[test]
    fn apply_open_position_holding_economics_keeps_closed_record_empty() {
        let stock_db_path = temp_db_path("execution_record_closed_holding_stock");
        let action_db_path = temp_db_path("execution_record_closed_holding_action");
        let stock_store = StockHistoryStore::new(stock_db_path.clone());
        let corporate_action_store = SecurityCorporateActionStore::new(action_db_path.clone());
        let mut record = open_record_fixture("closed");

        apply_open_position_holding_economics(
            &mut record,
            "2025-09-22",
            &stock_store,
            &corporate_action_store,
        )
        .expect("closed record helper path should return cleanly");

        assert_eq!(record.price_as_of_date, None);
        assert_eq!(record.resolved_trade_date, None);
        assert_eq!(record.current_price, None);
        assert_eq!(record.share_adjustment_factor, None);
        assert_eq!(record.cumulative_cash_dividend_per_share, None);
        assert_eq!(record.dividend_adjusted_cost_basis, None);
        assert_eq!(record.holding_total_return_pct, None);
        assert_eq!(record.breakeven_price, None);
        assert_eq!(record.corporate_action_summary, None);

        let _ = fs::remove_file(stock_db_path);
        let _ = fs::remove_file(action_db_path);
    }
}
