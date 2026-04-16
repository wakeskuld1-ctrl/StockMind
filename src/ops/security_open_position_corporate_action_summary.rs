use thiserror::Error;

use crate::ops::stock::security_execution_record::SecurityExecutionRecordDocument;
use crate::runtime::security_corporate_action_store::{
    SecurityCorporateActionRow, SecurityCorporateActionStore, SecurityCorporateActionStoreError,
};
use crate::runtime::stock_history_store::{StockHistoryStore, StockHistoryStoreError};

// 2026-04-16 CST: Added because P0-1 needs one formal summary object for "holding economics"
// instead of recomputing dividend-adjusted breakeven ad hoc in every caller.
// Purpose: separate structural open-position reconstruction from return and breakeven math.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenPositionCorporateActionSummary {
    pub price_as_of_date: String,
    pub resolved_trade_date: String,
    pub current_price: f64,
    pub share_adjustment_factor: f64,
    pub cumulative_cash_dividend_per_share: f64,
    pub dividend_adjusted_cost_basis: f64,
    pub holding_total_return_pct: f64,
    pub breakeven_price: f64,
    pub corporate_action_summary: String,
}

#[derive(Debug, Error)]
pub enum OpenPositionCorporateActionSummaryError {
    #[error("open position corporate action history load failed: {0}")]
    History(#[from] StockHistoryStoreError),
    #[error("open position corporate action runtime load failed: {0}")]
    CorporateAction(#[from] SecurityCorporateActionStoreError),
}

// 2026-04-16 CST: Added because the open-position snapshot layer now needs one canonical place
// to answer "what is the dividend-adjusted breakeven as of the current local trade date?"
// Purpose: keep the first corporate-action integration stable without reopening execution or
// forward-outcome write paths.
pub(crate) fn build_open_position_corporate_action_summary(
    execution_record: &SecurityExecutionRecordDocument,
    requested_as_of_date: &str,
    stock_store: &StockHistoryStore,
    action_store: &SecurityCorporateActionStore,
) -> Result<Option<OpenPositionCorporateActionSummary>, OpenPositionCorporateActionSummaryError> {
    let latest_rows =
        stock_store.load_recent_rows(&execution_record.symbol, Some(requested_as_of_date), 1)?;
    let Some(latest_row) = latest_rows.last() else {
        return Ok(None);
    };

    let action_rows =
        action_store.load_rows_on_or_before(&execution_record.symbol, &latest_row.trade_date)?;
    let corporate_action_effect = evaluate_corporate_action_effect(
        &action_rows,
        &execution_record.actual_entry_date,
        &latest_row.trade_date,
    );
    let dividend_adjusted_cost_basis = if corporate_action_effect.share_adjustment_factor > 0.0 {
        (execution_record.actual_entry_price
            - corporate_action_effect.cumulative_cash_dividend_per_share)
            / corporate_action_effect.share_adjustment_factor
    } else {
        0.0
    }
    .max(0.0);
    let holding_total_return_pct = if execution_record.actual_entry_price > 0.0 {
        ((latest_row.close * corporate_action_effect.share_adjustment_factor)
            + corporate_action_effect.cumulative_cash_dividend_per_share)
            / execution_record.actual_entry_price
            - 1.0
    } else {
        0.0
    };
    let corporate_action_summary = if corporate_action_effect.share_adjustment_factor > 1.0
        || corporate_action_effect.cumulative_cash_dividend_per_share > 0.0
    {
        format!(
            "resolved {} using latest local trade date {}; share adjustment factor {:.4}; accumulated cash dividend {:.4} per original share since entry; corporate-action-adjusted breakeven {:.4}",
            requested_as_of_date,
            latest_row.trade_date,
            corporate_action_effect.share_adjustment_factor,
            corporate_action_effect.cumulative_cash_dividend_per_share,
            dividend_adjusted_cost_basis
        )
    } else {
        format!(
            "resolved {} using latest local trade date {}; no effective cash dividend found since entry",
            requested_as_of_date, latest_row.trade_date
        )
    };

    Ok(Some(OpenPositionCorporateActionSummary {
        price_as_of_date: requested_as_of_date.to_string(),
        resolved_trade_date: latest_row.trade_date.clone(),
        current_price: latest_row.close,
        share_adjustment_factor: corporate_action_effect.share_adjustment_factor,
        cumulative_cash_dividend_per_share: corporate_action_effect
            .cumulative_cash_dividend_per_share,
        dividend_adjusted_cost_basis,
        holding_total_return_pct,
        breakeven_price: dividend_adjusted_cost_basis,
        corporate_action_summary,
    }))
}

// 2026-04-16 CST: Added because P0-2 now needs split and bonus events to change the live
// holding economics instead of remaining storage-only fields.
// Purpose: evaluate one per-original-share holding path so breakeven and total return can
// reflect both cash payouts and post-entry share-count changes.
#[derive(Debug, Clone, PartialEq)]
struct CorporateActionEffect {
    share_adjustment_factor: f64,
    cumulative_cash_dividend_per_share: f64,
}

fn evaluate_corporate_action_effect(
    rows: &[SecurityCorporateActionRow],
    actual_entry_date: &str,
    resolved_trade_date: &str,
) -> CorporateActionEffect {
    let mut share_adjustment_factor = 1.0;
    let mut cumulative_cash_dividend_per_share = 0.0;

    for row in rows
        .iter()
        .filter(|row| row.effective_date.as_str() > actual_entry_date)
        .filter(|row| row.effective_date.as_str() <= resolved_trade_date)
    {
        match row.action_type.as_str() {
            "cash_dividend" => {
                cumulative_cash_dividend_per_share +=
                    row.cash_dividend_per_share * share_adjustment_factor;
            }
            "split" => {
                share_adjustment_factor *= normalized_split_factor(row);
            }
            "bonus" => {
                share_adjustment_factor *= 1.0 + row.bonus_ratio.max(0.0);
            }
            _ => {}
        }
    }

    CorporateActionEffect {
        share_adjustment_factor,
        cumulative_cash_dividend_per_share,
    }
}

// 2026-04-16 CST: Added because runtime rows may use either a direct split factor or leave it
// at the neutral default when no split happened.
// Purpose: keep the first split-aware round tolerant to incomplete legacy rows.
fn normalized_split_factor(row: &SecurityCorporateActionRow) -> f64 {
    if row.split_ratio > 0.0 {
        row.split_ratio
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use crate::runtime::security_corporate_action_store::SecurityCorporateActionRow;
    use crate::runtime::stock_history_store::{StockHistoryRow, StockHistoryStore};

    fn temp_db_path(file_name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{file_name}_{nanos}.db"))
    }

    fn fixture_record() -> SecurityExecutionRecordDocument {
        SecurityExecutionRecordDocument {
            execution_record_id: "record-1".to_string(),
            contract_version: "security_execution_record.v1".to_string(),
            document_type: "security_execution_record".to_string(),
            generated_at: "2026-04-16T10:00:00+08:00".to_string(),
            symbol: "002352.SZ".to_string(),
            analysis_date: "2025-04-01".to_string(),
            account_id: Some("acct-1".to_string()),
            sector_tag: Some("logistics".to_string()),
            position_state: "open".to_string(),
            portfolio_position_plan_ref: None,
            execution_journal_ref: "journal-1".to_string(),
            position_plan_ref: "plan-1".to_string(),
            snapshot_ref: "snapshot-1".to_string(),
            outcome_ref: "outcome-1".to_string(),
            planned_entry_date: "2025-04-01".to_string(),
            planned_entry_price: 10.0,
            planned_position_pct: 0.1,
            planned_max_position_pct: 0.2,
            actual_entry_date: "2025-04-02".to_string(),
            actual_entry_price: 10.0,
            actual_position_pct: 0.1,
            current_position_pct: 0.1,
            actual_exit_date: String::new(),
            actual_exit_price: 0.0,
            exit_reason: "position_still_open".to_string(),
            holding_days: 10,
            planned_forward_return: 0.08,
            actual_return: 0.0,
            entry_slippage_pct: 0.0,
            position_size_gap_pct: 0.0,
            planned_tranche_action: None,
            planned_tranche_pct: None,
            planned_peak_position_pct: None,
            actual_tranche_action: None,
            actual_tranche_pct: None,
            actual_peak_position_pct: None,
            tranche_count_drift: None,
            account_budget_alignment: None,
            execution_return_gap: 0.0,
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
            execution_record_notes: Vec::new(),
            attribution_summary: "fixture".to_string(),
        }
    }

    #[test]
    fn holding_summary_applies_cash_dividend_to_breakeven_and_total_return() {
        let stock_db_path = temp_db_path("open_position_corporate_action_stock");
        let action_db_path = temp_db_path("open_position_corporate_action_events");
        let stock_store = StockHistoryStore::new(stock_db_path.clone());
        let action_store = SecurityCorporateActionStore::new(action_db_path.clone());

        stock_store
            .import_rows(
                "002352.SZ",
                "fixture",
                &[StockHistoryRow {
                    trade_date: "2025-04-18".to_string(),
                    open: 9.7,
                    high: 9.9,
                    low: 9.6,
                    close: 9.8,
                    adj_close: 9.8,
                    volume: 1_000_000,
                }],
            )
            .expect("stock rows should persist");
        action_store
            .upsert_rows(&[SecurityCorporateActionRow {
                symbol: "002352.SZ".to_string(),
                effective_date: "2025-04-10".to_string(),
                action_type: "cash_dividend".to_string(),
                cash_dividend_per_share: 0.5,
                split_ratio: 1.0,
                bonus_ratio: 0.0,
                source: "fixture".to_string(),
                payload_json: "{}".to_string(),
            }])
            .expect("corporate action rows should persist");

        let summary = build_open_position_corporate_action_summary(
            &fixture_record(),
            "2025-04-20",
            &stock_store,
            &action_store,
        )
        .expect("summary should build")
        .expect("summary should exist");

        assert_eq!(summary.resolved_trade_date, "2025-04-18");
        assert!((summary.share_adjustment_factor - 1.0).abs() <= 1e-9);
        assert!((summary.dividend_adjusted_cost_basis - 9.5).abs() <= 1e-9);
        assert!((summary.holding_total_return_pct - 0.03).abs() <= 1e-9);
        assert!(
            summary.breakeven_price < fixture_record().actual_entry_price,
            "cash dividend should lower breakeven"
        );

        let _ = fs::remove_file(stock_db_path);
        let _ = fs::remove_file(action_db_path);
    }

    #[test]
    fn holding_summary_applies_split_bonus_and_dividend_to_live_position_math() {
        let stock_db_path = temp_db_path("open_position_corporate_action_share_factor_stock");
        let action_db_path = temp_db_path("open_position_corporate_action_share_factor_events");
        let stock_store = StockHistoryStore::new(stock_db_path.clone());
        let action_store = SecurityCorporateActionStore::new(action_db_path.clone());

        stock_store
            .import_rows(
                "002352.SZ",
                "fixture",
                &[StockHistoryRow {
                    trade_date: "2025-04-18".to_string(),
                    open: 5.1,
                    high: 5.3,
                    low: 5.0,
                    close: 5.2,
                    adj_close: 5.2,
                    volume: 1_200_000,
                }],
            )
            .expect("stock rows should persist");
        action_store
            .upsert_rows(&[
                SecurityCorporateActionRow {
                    symbol: "002352.SZ".to_string(),
                    effective_date: "2025-04-08".to_string(),
                    action_type: "split".to_string(),
                    cash_dividend_per_share: 0.0,
                    split_ratio: 2.0,
                    bonus_ratio: 0.0,
                    source: "fixture".to_string(),
                    payload_json: "{\"split_ratio\":2.0}".to_string(),
                },
                SecurityCorporateActionRow {
                    symbol: "002352.SZ".to_string(),
                    effective_date: "2025-04-10".to_string(),
                    action_type: "cash_dividend".to_string(),
                    cash_dividend_per_share: 0.2,
                    split_ratio: 1.0,
                    bonus_ratio: 0.0,
                    source: "fixture".to_string(),
                    payload_json: "{}".to_string(),
                },
                SecurityCorporateActionRow {
                    symbol: "002352.SZ".to_string(),
                    effective_date: "2025-04-12".to_string(),
                    action_type: "bonus".to_string(),
                    cash_dividend_per_share: 0.0,
                    split_ratio: 1.0,
                    bonus_ratio: 0.5,
                    source: "fixture".to_string(),
                    payload_json: "{\"bonus_ratio\":0.5}".to_string(),
                },
            ])
            .expect("corporate action rows should persist");

        let summary = build_open_position_corporate_action_summary(
            &fixture_record(),
            "2025-04-20",
            &stock_store,
            &action_store,
        )
        .expect("summary should build")
        .expect("summary should exist");

        assert!((summary.share_adjustment_factor - 3.0).abs() <= 1e-9);
        assert!((summary.cumulative_cash_dividend_per_share - 0.4).abs() <= 1e-9);
        assert!((summary.dividend_adjusted_cost_basis - 3.2).abs() <= 1e-9);
        assert!((summary.holding_total_return_pct - 0.6).abs() <= 1e-9);
        assert!(
            summary
                .corporate_action_summary
                .contains("share adjustment factor 3.0000")
        );

        let _ = fs::remove_file(stock_db_path);
        let _ = fs::remove_file(action_db_path);
    }
}
