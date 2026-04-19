use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_account_open_position_snapshot_assembler::SecurityAccountOpenPositionSnapshotAssembler;
use crate::ops::stock::security_open_position_corporate_action_summary::OpenPositionCorporateActionSummaryError;
use crate::ops::stock::security_portfolio_position_plan::PortfolioOpenPositionSnapshotInput;
use crate::runtime::security_corporate_action_store::{
    SecurityCorporateActionStore, SecurityCorporateActionStoreError,
};
use crate::runtime::security_execution_store::{
    SecurityExecutionStore, SecurityExecutionStoreError,
};
use crate::runtime::stock_history_store::{StockHistoryStore, StockHistoryStoreError};

// 2026-04-10 CST: Added because the formal account snapshot chain needs a stable request
// contract; purpose: stop runtime-loading assumptions from leaking into callers.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityAccountOpenPositionSnapshotRequest {
    pub account_id: String,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-10 CST: Added because the account layer needs a formal document instead of a raw
// vector; purpose: keep runtime refs and summary fields stable for downstream planning.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityAccountOpenPositionSnapshotDocument {
    pub account_open_position_snapshot_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub account_id: String,
    pub open_position_snapshots: Vec<PortfolioOpenPositionSnapshotInput>,
    pub source_execution_record_refs: Vec<String>,
    pub snapshot_summary: String,
}

// 2026-04-18 CST: Added because Task 3 needs one explicit active-position view
// that later monitoring layers can consume without reinterpreting snapshot semantics.
// Reason: the approved design evolves the compatibility snapshot into a clearer
// active-position-book layer while keeping the old snapshot shell intact.
// Purpose: define one stable per-position entry for the live active book.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityActivePositionDocument {
    pub symbol: String,
    pub position_state: String,
    pub current_weight_pct: f64,
    #[serde(default)]
    pub price_as_of_date: Option<String>,
    #[serde(default)]
    pub resolved_trade_date: Option<String>,
    #[serde(default)]
    pub current_price: Option<f64>,
    #[serde(default)]
    pub share_adjustment_factor: Option<f64>,
    #[serde(default)]
    pub cumulative_cash_dividend_per_share: Option<f64>,
    #[serde(default)]
    pub dividend_adjusted_cost_basis: Option<f64>,
    #[serde(default)]
    pub holding_total_return_pct: Option<f64>,
    #[serde(default)]
    pub breakeven_price: Option<f64>,
    #[serde(default)]
    pub corporate_action_summary: Option<String>,
    #[serde(default)]
    pub sector_tag: Option<String>,
    #[serde(default)]
    pub source_execution_record_ref: Option<String>,
}

// 2026-04-18 CST: Added because Task 3 promotes the existing open snapshot into
// one explicit active-position-book semantics layer.
// Reason: later per-position evaluation should consume a named active book
// rather than rebuilding that view from the compatibility snapshot shell.
// Purpose: define the first formal active-position-book document.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityActivePositionBookDocument {
    pub active_position_book_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub account_id: String,
    pub source_snapshot_ref: String,
    pub active_position_count: usize,
    pub active_positions: Vec<SecurityActivePositionDocument>,
    pub source_execution_record_refs: Vec<String>,
    pub book_summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityAccountOpenPositionSnapshotResult {
    pub account_open_position_snapshot: SecurityAccountOpenPositionSnapshotDocument,
    pub active_position_book: SecurityActivePositionBookDocument,
}

#[derive(Debug, Error)]
pub enum SecurityAccountOpenPositionSnapshotError {
    #[error("security account open position snapshot build failed: account_id must not be empty")]
    MissingAccountId,
    #[error("security account open position snapshot runtime loading failed: {0}")]
    Store(#[from] SecurityExecutionStoreError),
    #[error("security account open position snapshot stock history loading failed: {0}")]
    History(#[from] StockHistoryStoreError),
    #[error("security account open position snapshot corporate action loading failed: {0}")]
    CorporateAction(#[from] SecurityCorporateActionStoreError),
    #[error("security account open position snapshot holding summary failed: {0}")]
    Summary(#[from] OpenPositionCorporateActionSummaryError),
}

pub fn security_account_open_position_snapshot(
    request: &SecurityAccountOpenPositionSnapshotRequest,
) -> Result<SecurityAccountOpenPositionSnapshotResult, SecurityAccountOpenPositionSnapshotError> {
    if request.account_id.trim().is_empty() {
        return Err(SecurityAccountOpenPositionSnapshotError::MissingAccountId);
    }

    let store = SecurityExecutionStore::workspace_default()?;
    // 2026-04-16 CST: Added because P0-1 now enriches open-position snapshots with
    // dividend-adjusted holding economics.
    // Purpose: resolve governed price and corporate-action facts on the same formal entry path.
    let stock_store = StockHistoryStore::workspace_default()?;
    let corporate_action_store = SecurityCorporateActionStore::workspace_default()?;
    let execution_records = store.load_latest_open_execution_records(request.account_id.trim())?;

    let account_open_position_snapshot = SecurityAccountOpenPositionSnapshotAssembler::new(
        request,
        &execution_records,
        &stock_store,
        &corporate_action_store,
    )
    .assemble()?;
    let active_position_book = build_active_position_book(&account_open_position_snapshot);

    Ok(SecurityAccountOpenPositionSnapshotResult {
        account_open_position_snapshot,
        active_position_book,
    })
}

pub(crate) fn normalize_created_at(value: &str) -> String {
    if value.trim().is_empty() {
        Utc::now().to_rfc3339()
    } else {
        value.trim().to_string()
    }
}

fn default_created_at() -> String {
    Utc::now().to_rfc3339()
}

// 2026-04-18 CST: Added because Task 3 needs a deterministic projection from
// the compatibility snapshot shell into the new active-position-book semantics.
// Reason: later monitoring layers should not duplicate filtering, sorting, and
// field projection every time they need the current live position view.
// Purpose: centralize the first active-book builder on top of the snapshot document.
pub fn build_active_position_book(
    snapshot: &SecurityAccountOpenPositionSnapshotDocument,
) -> SecurityActivePositionBookDocument {
    let mut active_positions = snapshot
        .open_position_snapshots
        .iter()
        .filter(|position| position.current_position_pct > 0.0)
        .map(|position| SecurityActivePositionDocument {
            symbol: position.symbol.clone(),
            position_state: position.position_state.clone(),
            current_weight_pct: position.current_position_pct,
            price_as_of_date: position.price_as_of_date.clone(),
            resolved_trade_date: position.resolved_trade_date.clone(),
            current_price: position.current_price,
            share_adjustment_factor: position.share_adjustment_factor,
            cumulative_cash_dividend_per_share: position.cumulative_cash_dividend_per_share,
            dividend_adjusted_cost_basis: position.dividend_adjusted_cost_basis,
            holding_total_return_pct: position.holding_total_return_pct,
            breakeven_price: position.breakeven_price,
            corporate_action_summary: position.corporate_action_summary.clone(),
            sector_tag: position.sector_tag.clone(),
            source_execution_record_ref: position.source_execution_record_ref.clone(),
        })
        .collect::<Vec<_>>();
    active_positions.sort_by(|left, right| left.symbol.cmp(&right.symbol));

    let mut source_execution_record_refs = active_positions
        .iter()
        .filter_map(|position| position.source_execution_record_ref.clone())
        .collect::<Vec<_>>();
    source_execution_record_refs.sort();
    let active_position_count = active_positions.len();

    SecurityActivePositionBookDocument {
        active_position_book_id: format!(
            "active-position-book:{}:{}",
            snapshot.account_id, snapshot.generated_at
        ),
        contract_version: "security_active_position_book.v1".to_string(),
        document_type: "security_active_position_book".to_string(),
        generated_at: snapshot.generated_at.clone(),
        account_id: snapshot.account_id.clone(),
        source_snapshot_ref: snapshot.account_open_position_snapshot_id.clone(),
        active_position_count,
        active_positions,
        source_execution_record_refs,
        book_summary: format!(
            "account {} currently has {} active positions ready for monitoring",
            snapshot.account_id, active_position_count
        ),
    }
}

// 2026-04-18 CST: Added because Task 3 also needs one explicit refresh helper
// for the active-position-book semantics.
// Reason: later tasks should refresh the live book through one governed path
// instead of reconstructing it from runtime stores manually.
// Purpose: expose a stable refresh entry for the new active book layer.
pub fn refresh_active_position_book(
    request: &SecurityAccountOpenPositionSnapshotRequest,
) -> Result<SecurityActivePositionBookDocument, SecurityAccountOpenPositionSnapshotError> {
    let snapshot_result = security_account_open_position_snapshot(request)?;
    Ok(snapshot_result.active_position_book)
}
