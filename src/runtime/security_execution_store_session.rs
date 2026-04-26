use rusqlite::Connection;

use crate::ops::stock::security_execution_record::SecurityExecutionRecordDocument;
use crate::runtime::security_execution_store::SecurityExecutionStoreError;
use crate::runtime::security_execution_store_repository_context::SecurityExecutionStoreRepositoryContext;
use crate::tools::contracts::{
    SecurityPositionPlanRecordResult, SecurityRecordPositionAdjustmentResult,
};

// 2026-04-15 CST: Added because round 2 plan B now needs one minimal explicit
// transaction boundary for future multi-repository execution-store writes.
// Purpose: keep transaction ownership out of the facade while reusing the
// existing repository-context boundary instead of introducing trait-heavy
// unit-of-work abstractions.
pub(crate) struct SecurityExecutionStoreSession {
    connection: Connection,
    finished: bool,
}

impl SecurityExecutionStoreSession {
    pub(crate) fn new(connection: Connection) -> Result<Self, SecurityExecutionStoreError> {
        connection
            .execute_batch("BEGIN IMMEDIATE TRANSACTION")
            .map_err(|error| SecurityExecutionStoreError::BeginTransaction(error.to_string()))?;
        Ok(Self {
            connection,
            finished: false,
        })
    }

    fn with_repository_context<T, F>(&self, action: F) -> Result<T, SecurityExecutionStoreError>
    where
        F: FnOnce(
            &SecurityExecutionStoreRepositoryContext<'_>,
        ) -> Result<T, SecurityExecutionStoreError>,
    {
        let context = SecurityExecutionStoreRepositoryContext::new(&self.connection);
        action(&context)
    }

    pub(crate) fn upsert_position_plan(
        &self,
        record: &SecurityPositionPlanRecordResult,
    ) -> Result<(), SecurityExecutionStoreError> {
        self.with_repository_context(|context| context.upsert_position_plan(record))
    }

    pub(crate) fn upsert_adjustment_event(
        &self,
        record: &SecurityRecordPositionAdjustmentResult,
    ) -> Result<(), SecurityExecutionStoreError> {
        self.with_repository_context(|context| context.upsert_adjustment_event(record))
    }

    pub(crate) fn upsert_execution_record(
        &self,
        record: &SecurityExecutionRecordDocument,
    ) -> Result<(), SecurityExecutionStoreError> {
        self.with_repository_context(|context| context.upsert_execution_record(record))
    }

    // 2026-04-26 CST: Added because P19D replay-control writes must check the target ref
    // inside the same transaction before upsert. Purpose: prevent repository-level
    // ON CONFLICT updates from overwriting replay evidence.
    pub(crate) fn load_execution_record(
        &self,
        execution_record_id: &str,
    ) -> Result<Option<SecurityExecutionRecordDocument>, SecurityExecutionStoreError> {
        self.with_repository_context(|context| context.load_execution_record(execution_record_id))
    }

    pub(crate) fn commit(mut self) -> Result<(), SecurityExecutionStoreError> {
        self.connection
            .execute_batch("COMMIT")
            .map_err(|error| SecurityExecutionStoreError::CommitTransaction(error.to_string()))?;
        self.finished = true;
        Ok(())
    }
}

impl Drop for SecurityExecutionStoreSession {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.connection.execute_batch("ROLLBACK");
            self.finished = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::ops::stock::security_decision_briefing::PositionPlan;
    use crate::ops::stock::security_execution_record::SecurityExecutionRecordDocument;
    use crate::runtime::security_execution_store::SecurityExecutionStore;
    use crate::tools::contracts::{
        PositionAdjustmentEventType, PositionPlanAlignment, SecurityPositionPlanRecordResult,
        SecurityRecordPositionAdjustmentResult,
    };

    // 2026-04-15 CST: Added because the new session boundary needs one
    // reproducible file-backed fixture path for commit/rollback verification.
    // Purpose: keep session tests aligned with the real execution-store SQLite
    // lifecycle instead of relying only on in-memory coverage.
    fn build_temp_db_path(test_name: &str) -> PathBuf {
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let base_dir = std::env::temp_dir()
            .join("excel_skill_security_execution_store_session_tests")
            .join(format!("{test_name}_{unique_suffix}"));
        fs::create_dir_all(&base_dir).expect("temporary test directory should be created");
        base_dir.join("security_execution.db")
    }

    #[test]
    fn session_commit_persists_grouped_plan_and_adjustment_writes() {
        let db_path = build_temp_db_path("session_commit");
        let store = SecurityExecutionStore::new(db_path.clone());
        let session = store
            .open_session()
            .expect("session should open transaction");
        let position_plan = SecurityPositionPlanRecordResult {
            position_plan_ref: "session-plan-1".to_string(),
            decision_ref: "session-decision-1".to_string(),
            approval_ref: "session-approval-1".to_string(),
            evidence_version: "v1".to_string(),
            symbol: "601916.SH".to_string(),
            analysis_date: "2026-04-15".to_string(),
            position_action: "build".to_string(),
            starter_position_pct: 0.05,
            max_position_pct: 0.12,
            position_plan: PositionPlan::default(),
        };
        let adjustment = SecurityRecordPositionAdjustmentResult {
            adjustment_event_ref: "session-adjustment-1".to_string(),
            decision_ref: "session-decision-1".to_string(),
            approval_ref: "session-approval-1".to_string(),
            evidence_version: "v1".to_string(),
            position_plan_ref: "session-plan-1".to_string(),
            symbol: "601916.SH".to_string(),
            event_type: PositionAdjustmentEventType::Add,
            event_date: "2026-04-15".to_string(),
            before_position_pct: 0.05,
            after_position_pct: 0.08,
            trigger_reason: "trend_confirmation".to_string(),
            plan_alignment: PositionPlanAlignment::OnPlan,
        };

        session
            .upsert_position_plan(&position_plan)
            .expect("session should persist position plan");
        session
            .upsert_adjustment_event(&adjustment)
            .expect("session should persist adjustment event");
        session.commit().expect("session should commit");

        assert_eq!(
            store
                .load_position_plan("session-plan-1")
                .expect("committed position plan should load"),
            Some(position_plan)
        );
        assert_eq!(
            store
                .load_adjustment_event("session-adjustment-1")
                .expect("committed adjustment event should load"),
            Some(adjustment)
        );

        let _ = fs::remove_file(&db_path);
        let _ = db_path.parent().map(fs::remove_dir_all).transpose();
    }

    #[test]
    fn session_drop_rolls_back_uncommitted_execution_record_write() {
        let db_path = build_temp_db_path("session_rollback");
        let store = SecurityExecutionStore::new(db_path.clone());
        let session = store
            .open_session()
            .expect("session should open transaction");
        let record = SecurityExecutionRecordDocument {
            execution_record_id: "session-record-1".to_string(),
            contract_version: "security_execution_record.v1".to_string(),
            document_type: "security_execution_record".to_string(),
            generated_at: "2026-04-15T10:00:00+08:00".to_string(),
            symbol: "601916.SH".to_string(),
            analysis_date: "2026-04-15".to_string(),
            account_id: Some("session-account-1".to_string()),
            sector_tag: Some("bank".to_string()),
            position_state: "open".to_string(),
            portfolio_position_plan_ref: Some("session-portfolio-plan-1".to_string()),
            execution_journal_ref: "session-journal-1".to_string(),
            position_plan_ref: "session-plan-1".to_string(),
            snapshot_ref: "session-snapshot-1".to_string(),
            outcome_ref: "session-outcome-1".to_string(),
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

        session
            .upsert_execution_record(&record)
            .expect("session should queue execution record write");
        drop(session);

        assert!(
            store
                .load_latest_open_execution_records("session-account-1")
                .expect("uncommitted records should still query")
                .is_empty(),
            "dropping the session without commit should roll back the write"
        );

        let _ = fs::remove_file(&db_path);
        let _ = db_path.parent().map(fs::remove_dir_all).transpose();
    }
}
