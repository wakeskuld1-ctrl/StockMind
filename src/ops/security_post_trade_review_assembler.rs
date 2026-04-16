use serde_json::json;

use crate::ops::stock::security_execution_record::SecurityExecutionRecordResult;
use crate::ops::stock::security_post_trade_review::{
    SecurityPostTradeReviewDocument, SecurityPostTradeReviewError,
    SecurityPostTradeReviewOutcomeBinding, SecurityPostTradeReviewRequest, normalize_created_at,
};
use crate::ops::stock::security_post_trade_review_policy::SecurityPostTradeReviewPolicy;

// 2026-04-14 CST: Extracted from security_post_trade_review.rs because round 2
// plan B needs one dedicated formal review assembler module.
// Purpose: keep post-trade review orchestration thin while one governed builder
// owns review-document assembly from execution and outcome aggregates.
pub(crate) struct SecurityPostTradeReviewAssembler<'a> {
    execution_record_result: &'a SecurityExecutionRecordResult,
    outcome_binding: &'a SecurityPostTradeReviewOutcomeBinding,
    request: &'a SecurityPostTradeReviewRequest,
}

impl<'a> SecurityPostTradeReviewAssembler<'a> {
    pub(crate) fn new(
        execution_record_result: &'a SecurityExecutionRecordResult,
        outcome_binding: &'a SecurityPostTradeReviewOutcomeBinding,
        request: &'a SecurityPostTradeReviewRequest,
    ) -> Self {
        Self {
            execution_record_result,
            outcome_binding,
            request,
        }
    }

    pub(crate) fn assemble(
        &self,
    ) -> Result<SecurityPostTradeReviewDocument, SecurityPostTradeReviewError> {
        let position_plan_document = &self
            .execution_record_result
            .position_plan_result
            .position_plan_document;
        let selected_outcome = &self.outcome_binding.selected_outcome;
        let execution_record = &self.execution_record_result.execution_record;
        let review_status =
            SecurityPostTradeReviewPolicy::review_status(&execution_record.position_state);
        let thesis_status =
            SecurityPostTradeReviewPolicy::thesis_status(&review_status, selected_outcome);
        let execution_deviation = SecurityPostTradeReviewPolicy::execution_deviation(
            &review_status,
            &execution_record.execution_quality,
        );
        let account_plan_alignment = execution_record.account_budget_alignment.clone();
        let tranche_discipline = account_plan_alignment
            .as_ref()
            .map(|alignment| SecurityPostTradeReviewPolicy::tranche_discipline(alignment));
        let budget_drift_reason = account_plan_alignment
            .as_ref()
            .map(|alignment| SecurityPostTradeReviewPolicy::budget_drift_reason(alignment));
        let next_account_adjustment_hint = account_plan_alignment.as_ref().map(|alignment| {
            SecurityPostTradeReviewPolicy::next_account_adjustment_hint(alignment)
        });
        let model_miss_reason = SecurityPostTradeReviewPolicy::model_miss_reason(
            &review_status,
            selected_outcome,
            &thesis_status,
            &execution_deviation,
        );
        let next_adjustment_hint = SecurityPostTradeReviewPolicy::next_adjustment_hint(
            &review_status,
            &thesis_status,
            position_plan_document.position_risk_grade.as_str(),
            selected_outcome,
            &execution_deviation,
        );
        let planned_position = json!({
            "position_action": position_plan_document.position_action,
            "entry_mode": position_plan_document.entry_mode,
            "starter_position_pct": position_plan_document.starter_position_pct,
            "max_position_pct": position_plan_document.max_position_pct,
            "position_risk_grade": position_plan_document.position_risk_grade,
        });

        Ok(SecurityPostTradeReviewDocument {
            review_id: format!(
                "post-trade-review-{}-{}d",
                position_plan_document.position_plan_id, self.request.review_horizon_days
            ),
            contract_version: "security_post_trade_review.v1".to_string(),
            document_type: "security_post_trade_review".to_string(),
            generated_at: normalize_created_at(&self.request.created_at),
            symbol: position_plan_document.symbol.clone(),
            analysis_date: position_plan_document.analysis_date.clone(),
            snapshot_date: self.outcome_binding.snapshot.as_of_date.clone(),
            review_horizon_days: selected_outcome.horizon_days,
            review_status: review_status.clone(),
            position_plan_ref: position_plan_document.position_plan_id.clone(),
            snapshot_ref: self.outcome_binding.snapshot.snapshot_id.clone(),
            outcome_ref: selected_outcome.outcome_id.clone(),
            execution_journal_ref: execution_record.execution_journal_ref.clone(),
            execution_record_ref: execution_record.execution_record_id.clone(),
            planned_position,
            actual_result_window: format!("{}d", selected_outcome.horizon_days),
            realized_return: selected_outcome.forward_return,
            executed_return: execution_record.actual_return,
            max_drawdown_realized: selected_outcome.max_drawdown,
            max_runup_realized: selected_outcome.max_runup,
            thesis_status: thesis_status.clone(),
            execution_deviation,
            execution_return_gap: execution_record.execution_return_gap,
            account_plan_alignment,
            tranche_discipline,
            budget_drift_reason,
            model_miss_reason,
            next_account_adjustment_hint,
            next_adjustment_hint: next_adjustment_hint.clone(),
            review_summary: format!(
                "Post-trade review for {} over {}d: planned_return={:.2}%, executed_return={:.2}%, max_drawdown={:.2}%, thesis={}, execution_deviation={}, next_hint={}",
                position_plan_document.symbol,
                selected_outcome.horizon_days,
                selected_outcome.forward_return * 100.0,
                execution_record.actual_return * 100.0,
                selected_outcome.max_drawdown * 100.0,
                thesis_status,
                execution_record.execution_quality,
                next_adjustment_hint
            ),
        })
    }
}
