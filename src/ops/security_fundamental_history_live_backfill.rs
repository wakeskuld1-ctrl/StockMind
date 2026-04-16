use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_analysis_fullstack::fetch_live_fundamental_history_rows_for_governed_history;
use crate::ops::stock::security_fundamental_history_backfill::{
    SecurityFundamentalHistoryBackfillError, SecurityFundamentalHistoryBackfillRecordInput,
    SecurityFundamentalHistoryBackfillRequest, security_fundamental_history_backfill,
};

// 2026-04-12 CST: Add a live financial-history backfill request, because Historical Data
// Phase 1 now needs one formal bridge from provider payloads into governed multi-period
// financial storage rather than hand-built record arrays.
// Purpose: let operators fetch and persist stock financial history in one stock tool call.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityFundamentalHistoryLiveBackfillRequest {
    pub symbol: String,
    pub batch_id: String,
    pub created_at: String,
    #[serde(default)]
    pub history_runtime_root: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityFundamentalHistoryLiveBackfillResult {
    pub contract_version: String,
    pub document_type: String,
    pub symbol: String,
    pub fetched_record_count: usize,
    pub imported_record_count: usize,
    pub covered_symbol_count: usize,
    pub covered_report_periods: Vec<String>,
    pub storage_path: String,
    pub backfill_result_path: String,
}

#[derive(Debug, Error)]
pub enum SecurityFundamentalHistoryLiveBackfillError {
    #[error("security fundamental live backfill build failed: {0}")]
    Build(String),
    #[error("security fundamental live backfill fetch failed: {0}")]
    Fetch(String),
    #[error("security fundamental live backfill persist failed: {0}")]
    Persist(#[from] SecurityFundamentalHistoryBackfillError),
}

// 2026-04-12 CST: Fetch governed multi-period financial rows and persist them in one flow,
// because stock validation and later shadow runs should stop depending on latest-snapshot-only
// financial context.
// Purpose: create one formal live-to-governed bridge for stock fundamental history.
pub fn security_fundamental_history_live_backfill(
    request: &SecurityFundamentalHistoryLiveBackfillRequest,
) -> Result<SecurityFundamentalHistoryLiveBackfillResult, SecurityFundamentalHistoryLiveBackfillError>
{
    validate_request(request)?;

    let fetched_rows = fetch_live_fundamental_history_rows_for_governed_history(&request.symbol)
        .map_err(SecurityFundamentalHistoryLiveBackfillError::Fetch)?;
    let records = fetched_rows
        .iter()
        .map(|row| SecurityFundamentalHistoryBackfillRecordInput {
            symbol: request.symbol.trim().to_string(),
            report_period: row.report_period.clone(),
            notice_date: row.notice_date.clone(),
            source: row.source.clone(),
            report_metrics: row.report_metrics.clone(),
        })
        .collect::<Vec<_>>();
    let persisted =
        security_fundamental_history_backfill(&SecurityFundamentalHistoryBackfillRequest {
            batch_id: request.batch_id.trim().to_string(),
            created_at: request.created_at.trim().to_string(),
            history_runtime_root: request.history_runtime_root.clone(),
            records,
        })?;

    Ok(SecurityFundamentalHistoryLiveBackfillResult {
        contract_version: "security_fundamental_history_live_backfill.v1".to_string(),
        document_type: "security_fundamental_history_live_backfill_result".to_string(),
        symbol: request.symbol.trim().to_string(),
        fetched_record_count: fetched_rows.len(),
        imported_record_count: persisted.imported_record_count,
        covered_symbol_count: persisted.covered_symbol_count,
        covered_report_periods: persisted.covered_report_periods,
        storage_path: persisted.storage_path,
        backfill_result_path: persisted.backfill_result_path,
    })
}

// 2026-04-12 CST: Keep request validation local, because live history imports should
// fail before any network or storage side effect when identifiers are incomplete.
// Purpose: preserve deterministic operator semantics for governed backfill runs.
fn validate_request(
    request: &SecurityFundamentalHistoryLiveBackfillRequest,
) -> Result<(), SecurityFundamentalHistoryLiveBackfillError> {
    if request.symbol.trim().is_empty() {
        return Err(SecurityFundamentalHistoryLiveBackfillError::Build(
            "symbol cannot be empty".to_string(),
        ));
    }
    if request.batch_id.trim().is_empty() {
        return Err(SecurityFundamentalHistoryLiveBackfillError::Build(
            "batch_id cannot be empty".to_string(),
        ));
    }
    if request.created_at.trim().is_empty() {
        return Err(SecurityFundamentalHistoryLiveBackfillError::Build(
            "created_at cannot be empty".to_string(),
        ));
    }

    Ok(())
}
