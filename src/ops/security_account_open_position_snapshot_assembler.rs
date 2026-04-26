use crate::ops::stock::security_account_open_position_snapshot::{
    SecurityAccountOpenPositionSnapshotDocument, SecurityAccountOpenPositionSnapshotError,
    SecurityAccountOpenPositionSnapshotRequest, normalize_created_at,
};
use crate::ops::stock::security_execution_record::SecurityExecutionRecordDocument;
use crate::ops::stock::security_open_position_corporate_action_summary::build_open_position_corporate_action_summary;
use crate::ops::stock::security_portfolio_position_plan::PortfolioOpenPositionSnapshotInput;
use crate::runtime::security_corporate_action_store::SecurityCorporateActionStore;
use crate::runtime::stock_history_store::StockHistoryStore;

// 2026-04-14 CST: Extracted from security_account_open_position_snapshot.rs because
// round 2 plan B needs one dedicated snapshot assembler boundary.
// Purpose: keep the account snapshot entry focused on runtime loading while one
// governed builder owns the runtime-record to contract-document mapping.
pub(crate) struct SecurityAccountOpenPositionSnapshotAssembler<'a> {
    request: &'a SecurityAccountOpenPositionSnapshotRequest,
    execution_records: &'a [SecurityExecutionRecordDocument],
    stock_store: &'a StockHistoryStore,
    corporate_action_store: &'a SecurityCorporateActionStore,
}

impl<'a> SecurityAccountOpenPositionSnapshotAssembler<'a> {
    pub(crate) fn new(
        request: &'a SecurityAccountOpenPositionSnapshotRequest,
        execution_records: &'a [SecurityExecutionRecordDocument],
        stock_store: &'a StockHistoryStore,
        corporate_action_store: &'a SecurityCorporateActionStore,
    ) -> Self {
        Self {
            request,
            execution_records,
            stock_store,
            corporate_action_store,
        }
    }

    pub(crate) fn assemble(
        &self,
    ) -> Result<SecurityAccountOpenPositionSnapshotDocument, SecurityAccountOpenPositionSnapshotError>
    {
        let requested_as_of_date = derive_requested_as_of_date(&self.request.created_at);
        let mut open_position_snapshots = Vec::new();
        for record in self.execution_records {
            let corporate_action_summary = build_open_position_corporate_action_summary(
                record,
                &requested_as_of_date,
                self.stock_store,
                self.corporate_action_store,
            )?;
            open_position_snapshots.push(PortfolioOpenPositionSnapshotInput {
                symbol: record.symbol.clone(),
                position_state: record.position_state.clone(),
                current_position_pct: record.current_position_pct,
                price_as_of_date: corporate_action_summary
                    .as_ref()
                    .map(|summary| summary.price_as_of_date.clone()),
                resolved_trade_date: corporate_action_summary
                    .as_ref()
                    .map(|summary| summary.resolved_trade_date.clone()),
                current_price: corporate_action_summary
                    .as_ref()
                    .map(|summary| summary.current_price),
                share_adjustment_factor: corporate_action_summary
                    .as_ref()
                    .map(|summary| summary.share_adjustment_factor),
                cumulative_cash_dividend_per_share: corporate_action_summary
                    .as_ref()
                    .map(|summary| summary.cumulative_cash_dividend_per_share),
                dividend_adjusted_cost_basis: corporate_action_summary
                    .as_ref()
                    .map(|summary| summary.dividend_adjusted_cost_basis),
                holding_total_return_pct: corporate_action_summary
                    .as_ref()
                    .map(|summary| summary.holding_total_return_pct),
                breakeven_price: corporate_action_summary
                    .as_ref()
                    .map(|summary| summary.breakeven_price),
                corporate_action_summary: corporate_action_summary
                    .as_ref()
                    .map(|summary| summary.corporate_action_summary.clone()),
                sector_tag: record.sector_tag.clone(),
                source_execution_record_ref: Some(record.execution_record_id.clone()),
            });
        }
        let source_execution_record_refs = self
            .execution_records
            .iter()
            .map(|record| record.execution_record_id.clone())
            .collect::<Vec<_>>();

        Ok(SecurityAccountOpenPositionSnapshotDocument {
            account_open_position_snapshot_id: format!(
                "account-open-position-snapshot-{}-{}",
                self.request.account_id.trim(),
                self.request.created_at.replace(':', "-")
            ),
            contract_version: "security_account_open_position_snapshot.v1".to_string(),
            document_type: "security_account_open_position_snapshot".to_string(),
            generated_at: normalize_created_at(&self.request.created_at),
            account_id: self.request.account_id.trim().to_string(),
            snapshot_summary: format!(
                "account {} currently has {} open execution records feeding the next account layer",
                self.request.account_id.trim(),
                open_position_snapshots.len()
            ),
            open_position_snapshots,
            source_execution_record_refs,
        })
    }
}

// 2026-04-16 CST: Added because the snapshot request currently carries RFC3339 timestamps while
// stock-history resolution is keyed by trading date.
// Purpose: keep one stable request-date normalization path for the new holding-economics helper.
fn derive_requested_as_of_date(created_at: &str) -> String {
    let trimmed = created_at.trim();
    if let Some((date_text, _)) = trimmed.split_once('T') {
        return date_text.to_string();
    }
    trimmed.chars().take(10).collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;
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

    #[test]
    fn snapshot_assembler_maps_runtime_records_into_contract() {
        let request = SecurityAccountOpenPositionSnapshotRequest {
            account_id: "acct-1".to_string(),
            created_at: "2026-04-14T17:00:00+08:00".to_string(),
        };
        let stock_db_path = temp_db_path("snapshot_assembler_stock");
        let action_db_path = temp_db_path("snapshot_assembler_actions");
        let stock_store = StockHistoryStore::new(stock_db_path.clone());
        let corporate_action_store = SecurityCorporateActionStore::new(action_db_path.clone());
        stock_store
            .import_rows(
                "601916.SH",
                "fixture",
                &[StockHistoryRow {
                    trade_date: "2026-04-14".to_string(),
                    open: 3.18,
                    high: 3.22,
                    low: 3.15,
                    close: 3.20,
                    adj_close: 3.20,
                    volume: 800_000,
                }],
            )
            .expect("stock rows should persist");
        corporate_action_store
            .upsert_rows(&[SecurityCorporateActionRow {
                symbol: "601916.SH".to_string(),
                effective_date: "2025-12-31".to_string(),
                action_type: "cash_dividend".to_string(),
                cash_dividend_per_share: 0.12,
                split_ratio: 1.0,
                bonus_ratio: 0.0,
                source: "fixture".to_string(),
                payload_json: "{}".to_string(),
            }])
            .expect("corporate action rows should persist");
        let records = vec![SecurityExecutionRecordDocument {
            execution_record_id: "record-1".to_string(),
            contract_version: "security_execution_record.v1".to_string(),
            document_type: "security_execution_record".to_string(),
            generated_at: "2026-04-14T17:00:00+08:00".to_string(),
            symbol: "601916.SH".to_string(),
            analysis_date: "2025-09-17".to_string(),
            account_id: Some("acct-1".to_string()),
            sector_tag: Some("bank".to_string()),
            position_state: "open".to_string(),
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
            actual_exit_date: String::new(),
            actual_exit_price: 0.0,
            exit_reason: "position_still_open".to_string(),
            holding_days: 1,
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
            execution_record_notes: Vec::new(),
            attribution_summary: "fixture".to_string(),
        }];

        let document = SecurityAccountOpenPositionSnapshotAssembler::new(
            &request,
            &records,
            &stock_store,
            &corporate_action_store,
        )
        .assemble()
        .expect("snapshot document should build");

        assert_eq!(document.account_id, "acct-1");
        assert_eq!(document.open_position_snapshots.len(), 1);
        assert_eq!(document.open_position_snapshots[0].symbol, "601916.SH");
        assert_eq!(
            document.open_position_snapshots[0]
                .resolved_trade_date
                .as_deref(),
            Some("2026-04-14")
        );
        assert_eq!(
            document.open_position_snapshots[0]
                .cumulative_cash_dividend_per_share
                .expect("cumulative dividend should exist"),
            0.12
        );
        assert_eq!(
            document.open_position_snapshots[0]
                .share_adjustment_factor
                .expect("share adjustment factor should exist"),
            1.0
        );

        let _ = fs::remove_file(stock_db_path);
        let _ = fs::remove_file(action_db_path);
    }
}
