use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_portfolio_execution_apply_bridge::{
    SecurityPortfolioExecutionApplyBridgeDocument, SecurityPortfolioExecutionApplyRow,
};

const SECURITY_PORTFOLIO_EXECUTION_STATUS_BRIDGE_DOCUMENT_TYPE: &str =
    "security_portfolio_execution_status_bridge";
const SECURITY_PORTFOLIO_EXECUTION_STATUS_BRIDGE_VERSION: &str =
    "security_portfolio_execution_status_bridge.v1";

// 2026-04-25 CST: Added because P16 references existed on the stock boundary
// while the implementation file was absent after branch consolidation.
// Reason: P16 must consume only the governed P15 apply document.
// Purpose: define the public request shell for a pure status-freeze bridge.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityPortfolioExecutionStatusBridgeRequest {
    pub portfolio_execution_apply_bridge: SecurityPortfolioExecutionApplyBridgeDocument,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-25 CST: Added because downstream reconciliation needs row-level
// execution truth without rereading or mutating runtime execution facts.
// Reason: P16 is a status artifact, not a repair, replay, or materialization layer.
// Purpose: freeze every P15 apply row into an explicit status row.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionStatusRow {
    pub symbol: String,
    pub request_action: String,
    pub requested_gross_pct: f64,
    pub enrichment_status: String,
    pub apply_status: String,
    pub execution_status: String,
    #[serde(default)]
    pub execution_record_ref: Option<String>,
    #[serde(default)]
    pub execution_journal_ref: Option<String>,
    pub status_summary: String,
}

// 2026-04-25 CST: Added because P16 must expose one stable batch artifact
// downstream of P15 instead of forcing callers to infer state from apply rows.
// Reason: later repair/reconciliation stages require explicit blockers and pending items.
// Purpose: define the governed execution-status bridge document.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionStatusBridgeDocument {
    pub portfolio_execution_status_bridge_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub analysis_date: String,
    pub account_id: String,
    pub portfolio_execution_apply_bridge_ref: String,
    pub portfolio_execution_request_enrichment_ref: String,
    pub portfolio_execution_request_package_ref: String,
    pub portfolio_execution_preview_ref: String,
    pub portfolio_allocation_decision_ref: String,
    pub status_rows: Vec<SecurityPortfolioExecutionStatusRow>,
    pub applied_count: usize,
    pub skipped_hold_count: usize,
    pub failed_apply_count: usize,
    pub pending_item_count: usize,
    pub execution_status: String,
    pub pending_items: Vec<String>,
    pub blockers: Vec<String>,
    pub status_rationale: Vec<String>,
    pub status_summary: String,
}

// 2026-04-25 CST: Added because the public stock dispatcher expects one named
// result wrapper matching the existing stock tool response pattern.
// Reason: preserving wrapper shape keeps P16 consistent with P13-P15 contracts.
// Purpose: return the frozen status document under a stable top-level key.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionStatusBridgeResult {
    pub portfolio_execution_status_bridge: SecurityPortfolioExecutionStatusBridgeDocument,
}

// 2026-04-25 CST: Added because malformed upstream status values should be
// surfaced as dispatcher errors instead of silently normalized.
// Reason: status vocabulary drift is contract corruption at the P15/P16 boundary.
// Purpose: keep unsupported P15 apply states explicit.
#[derive(Debug, Error)]
pub enum SecurityPortfolioExecutionStatusBridgeError {
    #[error(
        "security portfolio execution status bridge build failed: unsupported apply status `{0}`"
    )]
    UnsupportedApplyStatus(String),
}

// 2026-04-25 CST: Added because P16 must have one official callable entry
// point on the stock bus after the missing module was restored.
// Reason: callers should not rebuild status-freeze semantics outside the formal boundary.
// Purpose: expose the pure P15-to-P16 status bridge.
pub fn security_portfolio_execution_status_bridge(
    request: &SecurityPortfolioExecutionStatusBridgeRequest,
) -> Result<SecurityPortfolioExecutionStatusBridgeResult, SecurityPortfolioExecutionStatusBridgeError>
{
    build_security_portfolio_execution_status_bridge(request)
}

pub fn build_security_portfolio_execution_status_bridge(
    request: &SecurityPortfolioExecutionStatusBridgeRequest,
) -> Result<SecurityPortfolioExecutionStatusBridgeResult, SecurityPortfolioExecutionStatusBridgeError>
{
    let generated_at = normalize_created_at(&request.created_at);
    let apply_document = &request.portfolio_execution_apply_bridge;
    let execution_status = map_execution_status(apply_document)?;
    let status_rows = apply_document
        .apply_rows
        .iter()
        .map(build_status_row)
        .collect::<Vec<_>>();
    let pending_items = build_pending_items(apply_document, &status_rows);

    Ok(SecurityPortfolioExecutionStatusBridgeResult {
        portfolio_execution_status_bridge: SecurityPortfolioExecutionStatusBridgeDocument {
            portfolio_execution_status_bridge_id: format!(
                "portfolio-execution-status-bridge:{}:{}",
                apply_document.account_id, generated_at
            ),
            contract_version: SECURITY_PORTFOLIO_EXECUTION_STATUS_BRIDGE_VERSION.to_string(),
            document_type: SECURITY_PORTFOLIO_EXECUTION_STATUS_BRIDGE_DOCUMENT_TYPE.to_string(),
            generated_at,
            analysis_date: apply_document.analysis_date.clone(),
            account_id: apply_document.account_id.clone(),
            portfolio_execution_apply_bridge_ref: apply_document
                .portfolio_execution_apply_bridge_id
                .clone(),
            portfolio_execution_request_enrichment_ref: apply_document
                .portfolio_execution_request_enrichment_ref
                .clone(),
            portfolio_execution_request_package_ref: apply_document
                .portfolio_execution_request_package_ref
                .clone(),
            portfolio_execution_preview_ref: apply_document.portfolio_execution_preview_ref.clone(),
            portfolio_allocation_decision_ref: apply_document
                .portfolio_allocation_decision_ref
                .clone(),
            status_rows,
            applied_count: apply_document.applied_count,
            skipped_hold_count: apply_document.skipped_hold_count,
            failed_apply_count: apply_document.failed_apply_count,
            pending_item_count: pending_items.len(),
            execution_status,
            pending_items,
            blockers: apply_document.blockers.clone(),
            status_rationale: build_status_rationale(apply_document),
            status_summary: format!(
                "account {} froze apply bridge {} as execution status {}",
                apply_document.account_id,
                apply_document.portfolio_execution_apply_bridge_id,
                map_execution_status(apply_document)?
            ),
        },
    })
}

fn build_status_row(
    row: &SecurityPortfolioExecutionApplyRow,
) -> SecurityPortfolioExecutionStatusRow {
    let execution_status = match row.apply_status.as_str() {
        "applied" => "applied",
        "skipped_non_executable_hold" => "skipped_non_executable_hold",
        "apply_failed" => "apply_failed",
        _ => "unknown_apply_status",
    };

    SecurityPortfolioExecutionStatusRow {
        symbol: row.symbol.clone(),
        request_action: row.request_action.clone(),
        requested_gross_pct: row.requested_gross_pct,
        enrichment_status: row.enrichment_status.clone(),
        apply_status: row.apply_status.clone(),
        execution_status: execution_status.to_string(),
        execution_record_ref: row.execution_record_ref.clone(),
        execution_journal_ref: row.execution_journal_ref.clone(),
        status_summary: format!(
            "{} froze apply status {} into execution status {}",
            row.symbol, row.apply_status, execution_status
        ),
    }
}

fn map_execution_status(
    document: &SecurityPortfolioExecutionApplyBridgeDocument,
) -> Result<String, SecurityPortfolioExecutionStatusBridgeError> {
    match document.apply_status.as_str() {
        "applied" if document.failed_apply_count == 0 && document.skipped_hold_count == 0 => {
            Ok("fully_applied".to_string())
        }
        "applied" => Ok("applied_with_open_items".to_string()),
        "applied_with_skipped_holds" => Ok("applied_with_skipped_holds".to_string()),
        "partial_apply_failure" => Ok("partial_failure".to_string()),
        "rejected" => Ok("rejected".to_string()),
        other => Err(
            SecurityPortfolioExecutionStatusBridgeError::UnsupportedApplyStatus(other.to_string()),
        ),
    }
}

fn build_pending_items(
    document: &SecurityPortfolioExecutionApplyBridgeDocument,
    status_rows: &[SecurityPortfolioExecutionStatusRow],
) -> Vec<String> {
    let mut pending_items = Vec::new();

    pending_items.extend(document.blockers.iter().cloned());
    pending_items.extend(
        status_rows
            .iter()
            .filter(|row| row.execution_status != "applied")
            .map(|row| {
                format!(
                    "{} remains {} after apply bridge {}",
                    row.symbol, row.execution_status, document.portfolio_execution_apply_bridge_id
                )
            }),
    );

    pending_items
}

fn build_status_rationale(document: &SecurityPortfolioExecutionApplyBridgeDocument) -> Vec<String> {
    vec![
        format!(
            "execution status bridge consumed apply bridge {}",
            document.portfolio_execution_apply_bridge_id
        ),
        "execution status bridge only normalizes and freezes P15 apply status".to_string(),
        "execution status bridge does not reconcile, replay, broker-execute, or materialize positions"
            .to_string(),
    ]
}

fn normalize_created_at(created_at: &str) -> String {
    let trimmed = created_at.trim();
    if trimmed.is_empty() {
        Utc::now().to_rfc3339()
    } else {
        trimmed.to_string()
    }
}

fn default_created_at() -> String {
    Utc::now().to_rfc3339()
}
