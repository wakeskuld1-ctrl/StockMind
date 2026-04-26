use rusqlite::Connection;

use crate::ops::stock::security_execution_record::SecurityExecutionRecordDocument;
use crate::runtime::security_execution_store::SecurityExecutionStoreError;
use crate::runtime::security_execution_store_adjustment_event_repository::{
    load_security_adjustment_event, upsert_security_adjustment_event,
};
use crate::runtime::security_execution_store_execution_record_repository::{
    load_latest_open_security_execution_records, load_security_execution_record,
    upsert_security_execution_record,
};
use crate::runtime::security_execution_store_position_plan_repository::{
    load_security_position_plan_record, upsert_security_position_plan_record,
};
use crate::tools::contracts::{
    SecurityPositionPlanRecordResult, SecurityRecordPositionAdjustmentResult,
};

// 2026-04-15 CST: Added because round 2 plan B needs one internal repository
// context between the store facade and repository modules.
// Purpose: freeze a formal repository boundary before later transaction/session
// work arrives, while keeping today's facade API unchanged.
pub(crate) struct SecurityExecutionStoreRepositoryContext<'connection> {
    connection: &'connection Connection,
}

impl<'connection> SecurityExecutionStoreRepositoryContext<'connection> {
    pub(crate) fn new(connection: &'connection Connection) -> Self {
        Self { connection }
    }

    pub(crate) fn upsert_position_plan(
        &self,
        record: &SecurityPositionPlanRecordResult,
    ) -> Result<(), SecurityExecutionStoreError> {
        upsert_security_position_plan_record(self.connection, record)
    }

    pub(crate) fn load_position_plan(
        &self,
        position_plan_ref: &str,
    ) -> Result<Option<SecurityPositionPlanRecordResult>, SecurityExecutionStoreError> {
        load_security_position_plan_record(self.connection, position_plan_ref)
    }

    pub(crate) fn upsert_adjustment_event(
        &self,
        record: &SecurityRecordPositionAdjustmentResult,
    ) -> Result<(), SecurityExecutionStoreError> {
        upsert_security_adjustment_event(self.connection, record)
    }

    pub(crate) fn load_adjustment_event(
        &self,
        adjustment_event_ref: &str,
    ) -> Result<Option<SecurityRecordPositionAdjustmentResult>, SecurityExecutionStoreError> {
        load_security_adjustment_event(self.connection, adjustment_event_ref)
    }

    pub(crate) fn upsert_execution_record(
        &self,
        record: &SecurityExecutionRecordDocument,
    ) -> Result<(), SecurityExecutionStoreError> {
        upsert_security_execution_record(self.connection, record)
    }

    pub(crate) fn load_latest_open_execution_records(
        &self,
        account_id: &str,
    ) -> Result<Vec<SecurityExecutionRecordDocument>, SecurityExecutionStoreError> {
        load_latest_open_security_execution_records(self.connection, account_id)
    }

    pub(crate) fn load_execution_record(
        &self,
        execution_record_id: &str,
    ) -> Result<Option<SecurityExecutionRecordDocument>, SecurityExecutionStoreError> {
        load_security_execution_record(self.connection, execution_record_id)
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::SecurityExecutionStoreRepositoryContext;
    use crate::ops::stock::security_decision_briefing::PositionPlan;
    use crate::ops::stock::security_execution_record::SecurityExecutionRecordDocument;
    use crate::runtime::security_execution_store_schema::bootstrap_security_execution_schema;
    use crate::tools::contracts::{
        PositionAdjustmentEventType, PositionPlanAlignment, SecurityPositionPlanRecordResult,
        SecurityRecordPositionAdjustmentResult,
    };

    #[test]
    fn repository_context_round_trips_position_and_adjustment_records() {
        let connection = Connection::open_in_memory().expect("in-memory sqlite should open");
        bootstrap_security_execution_schema(&connection)
            .expect("execution schema should bootstrap");
        let context = SecurityExecutionStoreRepositoryContext::new(&connection);
        let position_plan = SecurityPositionPlanRecordResult {
            position_plan_ref: "plan-ref-ctx-1".to_string(),
            decision_ref: "decision-ctx-1".to_string(),
            approval_ref: "approval-ctx-1".to_string(),
            evidence_version: "v1".to_string(),
            symbol: "601916.SH".to_string(),
            analysis_date: "2026-04-15".to_string(),
            position_action: "build".to_string(),
            starter_position_pct: 0.05,
            max_position_pct: 0.12,
            position_plan: PositionPlan::default(),
        };
        let adjustment = SecurityRecordPositionAdjustmentResult {
            adjustment_event_ref: "adjustment-ctx-1".to_string(),
            decision_ref: "decision-ctx-1".to_string(),
            approval_ref: "approval-ctx-1".to_string(),
            evidence_version: "v1".to_string(),
            position_plan_ref: "plan-ref-ctx-1".to_string(),
            symbol: "601916.SH".to_string(),
            event_type: PositionAdjustmentEventType::Add,
            event_date: "2026-04-15".to_string(),
            before_position_pct: 0.05,
            after_position_pct: 0.08,
            trigger_reason: "trend_confirmation".to_string(),
            plan_alignment: PositionPlanAlignment::OnPlan,
        };

        context
            .upsert_position_plan(&position_plan)
            .expect("position plan record should persist through context");
        context
            .upsert_adjustment_event(&adjustment)
            .expect("adjustment event should persist through context");

        assert_eq!(
            context
                .load_position_plan("plan-ref-ctx-1")
                .expect("position plan should load"),
            Some(position_plan)
        );
        assert_eq!(
            context
                .load_adjustment_event("adjustment-ctx-1")
                .expect("adjustment event should load"),
            Some(adjustment)
        );
    }

    #[test]
    fn repository_context_round_trips_open_execution_records() {
        let connection = Connection::open_in_memory().expect("in-memory sqlite should open");
        bootstrap_security_execution_schema(&connection)
            .expect("execution schema should bootstrap");
        let context = SecurityExecutionStoreRepositoryContext::new(&connection);
        let record = SecurityExecutionRecordDocument {
            execution_record_id: "record-ctx-1".to_string(),
            contract_version: "security_execution_record.v1".to_string(),
            document_type: "security_execution_record".to_string(),
            generated_at: "2026-04-15T10:00:00+08:00".to_string(),
            symbol: "601916.SH".to_string(),
            analysis_date: "2026-04-15".to_string(),
            account_id: Some("acct-ctx-1".to_string()),
            sector_tag: Some("bank".to_string()),
            position_state: "open".to_string(),
            portfolio_position_plan_ref: Some("portfolio-plan-ctx-1".to_string()),
            execution_journal_ref: "journal-ctx-1".to_string(),
            position_plan_ref: "plan-ctx-1".to_string(),
            snapshot_ref: "snapshot-ctx-1".to_string(),
            outcome_ref: "outcome-ctx-1".to_string(),
            planned_entry_date: "2026-04-14".to_string(),
            planned_entry_price: 10.0,
            planned_position_pct: 0.05,
            planned_max_position_pct: 0.12,
            actual_entry_date: "2026-04-15".to_string(),
            actual_entry_price: 10.1,
            actual_position_pct: 0.08,
            current_position_pct: 0.08,
            actual_exit_date: String::new(),
            actual_exit_price: 0.0,
            exit_reason: "position_still_open".to_string(),
            holding_days: 1,
            planned_forward_return: 0.06,
            actual_return: 0.0,
            entry_slippage_pct: 0.01,
            position_size_gap_pct: 0.03,
            planned_tranche_action: Some("entry_tranche".to_string()),
            planned_tranche_pct: Some(0.05),
            planned_peak_position_pct: Some(0.12),
            actual_tranche_action: Some("entry_tranche".to_string()),
            actual_tranche_pct: Some(0.08),
            actual_peak_position_pct: Some(0.08),
            tranche_count_drift: Some(0),
            account_budget_alignment: Some("aligned".to_string()),
            execution_return_gap: -0.06,
            execution_quality: "open_position_pending".to_string(),
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
            execution_record_notes: vec!["fixture".to_string()],
            attribution_summary: "fixture".to_string(),
        };

        context
            .upsert_execution_record(&record)
            .expect("execution record should persist through context");

        assert_eq!(
            context
                .load_latest_open_execution_records("acct-ctx-1")
                .expect("execution records should load"),
            vec![record]
        );
        assert_eq!(
            context
                .load_execution_record("record-ctx-1")
                .expect("execution record should load by ref"),
            Some(SecurityExecutionRecordDocument {
                execution_record_id: "record-ctx-1".to_string(),
                contract_version: "security_execution_record.v1".to_string(),
                document_type: "security_execution_record".to_string(),
                generated_at: "2026-04-15T10:00:00+08:00".to_string(),
                symbol: "601916.SH".to_string(),
                analysis_date: "2026-04-15".to_string(),
                account_id: Some("acct-ctx-1".to_string()),
                sector_tag: Some("bank".to_string()),
                position_state: "open".to_string(),
                portfolio_position_plan_ref: Some("portfolio-plan-ctx-1".to_string()),
                execution_journal_ref: "journal-ctx-1".to_string(),
                position_plan_ref: "plan-ctx-1".to_string(),
                snapshot_ref: "snapshot-ctx-1".to_string(),
                outcome_ref: "outcome-ctx-1".to_string(),
                planned_entry_date: "2026-04-14".to_string(),
                planned_entry_price: 10.0,
                planned_position_pct: 0.05,
                planned_max_position_pct: 0.12,
                actual_entry_date: "2026-04-15".to_string(),
                actual_entry_price: 10.1,
                actual_position_pct: 0.08,
                current_position_pct: 0.08,
                actual_exit_date: String::new(),
                actual_exit_price: 0.0,
                exit_reason: "position_still_open".to_string(),
                holding_days: 1,
                planned_forward_return: 0.06,
                actual_return: 0.0,
                entry_slippage_pct: 0.01,
                position_size_gap_pct: 0.03,
                planned_tranche_action: Some("entry_tranche".to_string()),
                planned_tranche_pct: Some(0.05),
                planned_peak_position_pct: Some(0.12),
                actual_tranche_action: Some("entry_tranche".to_string()),
                actual_tranche_pct: Some(0.08),
                actual_peak_position_pct: Some(0.08),
                tranche_count_drift: Some(0),
                account_budget_alignment: Some("aligned".to_string()),
                execution_return_gap: -0.06,
                execution_quality: "open_position_pending".to_string(),
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
                execution_record_notes: vec!["fixture".to_string()],
                attribution_summary: "fixture".to_string(),
            })
        );
    }
}
