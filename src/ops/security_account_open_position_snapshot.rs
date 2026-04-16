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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityAccountOpenPositionSnapshotResult {
    pub account_open_position_snapshot: SecurityAccountOpenPositionSnapshotDocument,
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

    Ok(SecurityAccountOpenPositionSnapshotResult {
        account_open_position_snapshot: SecurityAccountOpenPositionSnapshotAssembler::new(
            request,
            &execution_records,
            &stock_store,
            &corporate_action_store,
        )
        .assemble()?,
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
