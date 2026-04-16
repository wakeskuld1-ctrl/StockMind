use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_analysis_fullstack::fetch_live_disclosure_history_rows_for_governed_history;
use crate::ops::stock::security_disclosure_history_backfill::{
    SecurityDisclosureHistoryBackfillError, SecurityDisclosureHistoryBackfillRecordInput,
    SecurityDisclosureHistoryBackfillRequest, security_disclosure_history_backfill,
};

const DEFAULT_DISCLOSURE_PAGE_SIZE: usize = 20;
const DEFAULT_DISCLOSURE_MAX_PAGES: usize = 3;

// 2026-04-12 CST: Add a live disclosure-history backfill request, because Historical Data
// Phase 1 now needs one formal bridge from paged announcement payloads into governed
// disclosure storage instead of hand-built record arrays.
// Purpose: let operators fetch and persist stock disclosure history in one stock tool call.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityDisclosureHistoryLiveBackfillRequest {
    pub symbol: String,
    pub batch_id: String,
    pub created_at: String,
    #[serde(default)]
    pub history_runtime_root: Option<String>,
    #[serde(default = "default_disclosure_page_size")]
    pub page_size: usize,
    #[serde(default = "default_disclosure_max_pages")]
    pub max_pages: usize,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityDisclosureHistoryLiveBackfillResult {
    pub contract_version: String,
    pub document_type: String,
    pub symbol: String,
    pub fetched_record_count: usize,
    pub imported_record_count: usize,
    pub covered_symbol_count: usize,
    pub covered_published_dates: Vec<String>,
    pub storage_path: String,
    pub backfill_result_path: String,
}

#[derive(Debug, Error)]
pub enum SecurityDisclosureHistoryLiveBackfillError {
    #[error("security disclosure live backfill build failed: {0}")]
    Build(String),
    #[error("security disclosure live backfill fetch failed: {0}")]
    Fetch(String),
    #[error("security disclosure live backfill persist failed: {0}")]
    Persist(#[from] SecurityDisclosureHistoryBackfillError),
}

// 2026-04-12 CST: Fetch governed multi-page announcement rows and persist them in one flow,
// because stock validation and later shadow runs should stop depending on one-page-only
// disclosure context.
// Purpose: create one formal live-to-governed bridge for stock disclosure history.
pub fn security_disclosure_history_live_backfill(
    request: &SecurityDisclosureHistoryLiveBackfillRequest,
) -> Result<SecurityDisclosureHistoryLiveBackfillResult, SecurityDisclosureHistoryLiveBackfillError>
{
    validate_request(request)?;

    let fetched_rows = fetch_live_disclosure_history_rows_for_governed_history(
        &request.symbol,
        request.page_size,
        request.max_pages,
    )
    .map_err(SecurityDisclosureHistoryLiveBackfillError::Fetch)?;
    let records = fetched_rows
        .iter()
        .map(|row| SecurityDisclosureHistoryBackfillRecordInput {
            symbol: request.symbol.trim().to_string(),
            published_at: row.published_at.clone(),
            title: row.title.clone(),
            article_code: row.article_code.clone(),
            category: row.category.clone(),
            source: row.source.clone(),
        })
        .collect::<Vec<_>>();
    let persisted =
        security_disclosure_history_backfill(&SecurityDisclosureHistoryBackfillRequest {
            batch_id: request.batch_id.trim().to_string(),
            created_at: request.created_at.trim().to_string(),
            history_runtime_root: request.history_runtime_root.clone(),
            records,
        })?;

    Ok(SecurityDisclosureHistoryLiveBackfillResult {
        contract_version: "security_disclosure_history_live_backfill.v1".to_string(),
        document_type: "security_disclosure_history_live_backfill_result".to_string(),
        symbol: request.symbol.trim().to_string(),
        fetched_record_count: fetched_rows.len(),
        imported_record_count: persisted.imported_record_count,
        covered_symbol_count: persisted.covered_symbol_count,
        covered_published_dates: persisted.covered_published_dates,
        storage_path: persisted.storage_path,
        backfill_result_path: persisted.backfill_result_path,
    })
}

// 2026-04-12 CST: Keep request validation local, because paged live disclosure imports
// should fail before any network or storage side effect when identifiers are incomplete.
// Purpose: preserve deterministic operator semantics for governed disclosure backfill.
fn validate_request(
    request: &SecurityDisclosureHistoryLiveBackfillRequest,
) -> Result<(), SecurityDisclosureHistoryLiveBackfillError> {
    if request.symbol.trim().is_empty() {
        return Err(SecurityDisclosureHistoryLiveBackfillError::Build(
            "symbol cannot be empty".to_string(),
        ));
    }
    if request.batch_id.trim().is_empty() {
        return Err(SecurityDisclosureHistoryLiveBackfillError::Build(
            "batch_id cannot be empty".to_string(),
        ));
    }
    if request.created_at.trim().is_empty() {
        return Err(SecurityDisclosureHistoryLiveBackfillError::Build(
            "created_at cannot be empty".to_string(),
        ));
    }

    Ok(())
}

fn default_disclosure_page_size() -> usize {
    DEFAULT_DISCLOSURE_PAGE_SIZE
}

fn default_disclosure_max_pages() -> usize {
    DEFAULT_DISCLOSURE_MAX_PAGES
}
