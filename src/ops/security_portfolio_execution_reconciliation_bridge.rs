use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_portfolio_execution_status_bridge::{
    SecurityPortfolioExecutionStatusBridgeDocument, SecurityPortfolioExecutionStatusRow,
};

const SECURITY_PORTFOLIO_EXECUTION_RECONCILIATION_BRIDGE_DOCUMENT_TYPE: &str =
    "security_portfolio_execution_reconciliation_bridge";
const SECURITY_PORTFOLIO_EXECUTION_RECONCILIATION_BRIDGE_VERSION: &str =
    "security_portfolio_execution_reconciliation_bridge.v1";

// 2026-04-25 CST: Added because D:\SM had P16 status freezing but lacked the
// downstream P17 reconciliation artifact recorded in handoff notes.
// Reason: reconciliation must consume only the formal P16 status document.
// Purpose: define the public request shell for a side-effect-free reconciliation bridge.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityPortfolioExecutionReconciliationBridgeRequest {
    pub portfolio_execution_status_bridge: SecurityPortfolioExecutionStatusBridgeDocument,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-25 CST: Added because P18 needs row-level unresolved execution truth
// without rereading P15 apply rows or runtime execution stores.
// Reason: P17 is the final truth-freeze before repair intent is classified.
// Purpose: expose settled, skipped, and reconciliation-required row states explicitly.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionReconciliationRow {
    pub symbol: String,
    pub request_action: String,
    pub requested_gross_pct: f64,
    pub enrichment_status: String,
    pub apply_status: String,
    pub execution_status: String,
    pub reconciliation_status: String,
    #[serde(default)]
    pub execution_record_ref: Option<String>,
    #[serde(default)]
    pub execution_journal_ref: Option<String>,
    pub requires_manual_follow_up: bool,
    pub blockers: Vec<String>,
    pub reconciliation_summary: String,
}

// 2026-04-25 CST: Added because downstream repair work must start from one
// named reconciliation document instead of raw P16 status rows.
// Reason: P17 freezes unresolved execution truth but still must not execute or repair anything.
// Purpose: define the recovered P17 reconciliation artifact.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionReconciliationBridgeDocument {
    pub portfolio_execution_reconciliation_bridge_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub analysis_date: String,
    pub account_id: String,
    pub portfolio_execution_status_bridge_ref: String,
    pub portfolio_execution_apply_bridge_ref: String,
    pub portfolio_execution_request_enrichment_ref: String,
    pub portfolio_execution_request_package_ref: String,
    pub portfolio_execution_preview_ref: String,
    pub portfolio_allocation_decision_ref: String,
    pub reconciliation_rows: Vec<SecurityPortfolioExecutionReconciliationRow>,
    pub settled_count: usize,
    pub skipped_hold_count: usize,
    pub reconciliation_required_count: usize,
    pub manual_follow_up_count: usize,
    pub unresolved_count: usize,
    pub reconciliation_status: String,
    pub pending_items: Vec<String>,
    pub blockers: Vec<String>,
    pub reconciliation_rationale: Vec<String>,
    pub reconciliation_summary: String,
}

// 2026-04-25 CST: Added because the public stock dispatcher expects one named
// result wrapper matching the existing P13-P16 response pattern.
// Reason: stable top-level keys keep CLI consumers independent from module internals.
// Purpose: return the P17 document under a governed wrapper key.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionReconciliationBridgeResult {
    pub portfolio_execution_reconciliation_bridge:
        SecurityPortfolioExecutionReconciliationBridgeDocument,
}

// 2026-04-25 CST: Added because P17 must reject contract drift at the P16/P17
// boundary instead of guessing downstream reconciliation behavior.
// Reason: status summary and lineage corruption would make repair intent unsafe.
// Purpose: keep malformed P16 inputs as hard dispatcher-visible errors.
#[derive(Debug, Error)]
pub enum SecurityPortfolioExecutionReconciliationBridgeError {
    #[error(
        "security portfolio execution reconciliation bridge build failed: unsupported execution status `{0}`"
    )]
    UnsupportedExecutionStatus(String),
    #[error(
        "security portfolio execution reconciliation bridge build failed: missing lineage ref `{0}`"
    )]
    MissingLineageRef(&'static str),
    #[error(
        "security portfolio execution reconciliation bridge build failed: summary count mismatch `{0}` observed `{1}` expected `{2}`"
    )]
    SummaryCountMismatch(&'static str, usize, usize),
    #[error(
        "security portfolio execution reconciliation bridge build failed: unsupported row execution status `{1}` on `{0}`"
    )]
    UnsupportedRowExecutionStatus(String, String),
}

// 2026-04-25 CST: Added because the recovered P17 tool must be callable from
// the public stock bus after P16.
// Reason: callers should not rebuild reconciliation truth outside the formal boundary.
// Purpose: expose the P16-to-P17 reconciliation bridge entry point.
pub fn security_portfolio_execution_reconciliation_bridge(
    request: &SecurityPortfolioExecutionReconciliationBridgeRequest,
) -> Result<
    SecurityPortfolioExecutionReconciliationBridgeResult,
    SecurityPortfolioExecutionReconciliationBridgeError,
> {
    build_security_portfolio_execution_reconciliation_bridge(request)
}

pub fn build_security_portfolio_execution_reconciliation_bridge(
    request: &SecurityPortfolioExecutionReconciliationBridgeRequest,
) -> Result<
    SecurityPortfolioExecutionReconciliationBridgeResult,
    SecurityPortfolioExecutionReconciliationBridgeError,
> {
    let generated_at = normalize_created_at(&request.created_at);
    let status_document = &request.portfolio_execution_status_bridge;
    validate_lineage(status_document)?;
    validate_summary_counts(status_document)?;

    let reconciliation_rows = status_document
        .status_rows
        .iter()
        .map(|row| build_reconciliation_row(status_document, row))
        .collect::<Result<Vec<_>, _>>()?;
    let settled_count = reconciliation_rows
        .iter()
        .filter(|row| row.reconciliation_status == "settled")
        .count();
    let skipped_hold_count = reconciliation_rows
        .iter()
        .filter(|row| row.reconciliation_status == "skipped_hold")
        .count();
    let reconciliation_required_count = reconciliation_rows
        .iter()
        .filter(|row| row.reconciliation_status == "reconciliation_required")
        .count();
    let manual_follow_up_count = reconciliation_rows
        .iter()
        .filter(|row| row.requires_manual_follow_up)
        .count();
    let reconciliation_status = map_reconciliation_status(status_document)?;
    let pending_items = build_pending_items(status_document, &reconciliation_rows);
    let blockers = build_blockers(status_document, &reconciliation_rows);

    Ok(SecurityPortfolioExecutionReconciliationBridgeResult {
        portfolio_execution_reconciliation_bridge:
            SecurityPortfolioExecutionReconciliationBridgeDocument {
                portfolio_execution_reconciliation_bridge_id: format!(
                    "portfolio-execution-reconciliation-bridge:{}:{}",
                    status_document.account_id, generated_at
                ),
                contract_version: SECURITY_PORTFOLIO_EXECUTION_RECONCILIATION_BRIDGE_VERSION
                    .to_string(),
                document_type: SECURITY_PORTFOLIO_EXECUTION_RECONCILIATION_BRIDGE_DOCUMENT_TYPE
                    .to_string(),
                generated_at,
                analysis_date: status_document.analysis_date.clone(),
                account_id: status_document.account_id.clone(),
                portfolio_execution_status_bridge_ref: status_document
                    .portfolio_execution_status_bridge_id
                    .clone(),
                portfolio_execution_apply_bridge_ref: status_document
                    .portfolio_execution_apply_bridge_ref
                    .clone(),
                portfolio_execution_request_enrichment_ref: status_document
                    .portfolio_execution_request_enrichment_ref
                    .clone(),
                portfolio_execution_request_package_ref: status_document
                    .portfolio_execution_request_package_ref
                    .clone(),
                portfolio_execution_preview_ref: status_document
                    .portfolio_execution_preview_ref
                    .clone(),
                portfolio_allocation_decision_ref: status_document
                    .portfolio_allocation_decision_ref
                    .clone(),
                reconciliation_rows,
                settled_count,
                skipped_hold_count,
                reconciliation_required_count,
                manual_follow_up_count,
                unresolved_count: reconciliation_required_count + manual_follow_up_count,
                reconciliation_status: reconciliation_status.clone(),
                pending_items,
                blockers,
                reconciliation_rationale: build_reconciliation_rationale(status_document),
                reconciliation_summary: format!(
                    "account {} froze status bridge {} as reconciliation status {}",
                    status_document.account_id,
                    status_document.portfolio_execution_status_bridge_id,
                    reconciliation_status
                ),
            },
    })
}

fn validate_lineage(
    document: &SecurityPortfolioExecutionStatusBridgeDocument,
) -> Result<(), SecurityPortfolioExecutionReconciliationBridgeError> {
    for (name, value) in [
        (
            "portfolio_execution_status_bridge_id",
            document.portfolio_execution_status_bridge_id.as_str(),
        ),
        (
            "portfolio_execution_apply_bridge_ref",
            document.portfolio_execution_apply_bridge_ref.as_str(),
        ),
        (
            "portfolio_execution_request_enrichment_ref",
            document.portfolio_execution_request_enrichment_ref.as_str(),
        ),
        (
            "portfolio_execution_request_package_ref",
            document.portfolio_execution_request_package_ref.as_str(),
        ),
        (
            "portfolio_execution_preview_ref",
            document.portfolio_execution_preview_ref.as_str(),
        ),
        (
            "portfolio_allocation_decision_ref",
            document.portfolio_allocation_decision_ref.as_str(),
        ),
    ] {
        if value.trim().is_empty() {
            return Err(
                SecurityPortfolioExecutionReconciliationBridgeError::MissingLineageRef(name),
            );
        }
    }
    Ok(())
}

fn validate_summary_counts(
    document: &SecurityPortfolioExecutionStatusBridgeDocument,
) -> Result<(), SecurityPortfolioExecutionReconciliationBridgeError> {
    let observed_applied = document
        .status_rows
        .iter()
        .filter(|row| row.execution_status == "applied")
        .count();
    let observed_skipped = document
        .status_rows
        .iter()
        .filter(|row| row.execution_status == "skipped_non_executable_hold")
        .count();
    let observed_failed = document
        .status_rows
        .iter()
        .filter(|row| row.execution_status == "apply_failed")
        .count();

    for (name, observed, expected) in [
        ("applied_count", observed_applied, document.applied_count),
        (
            "skipped_hold_count",
            observed_skipped,
            document.skipped_hold_count,
        ),
        (
            "failed_apply_count",
            observed_failed,
            document.failed_apply_count,
        ),
    ] {
        if observed != expected {
            return Err(
                SecurityPortfolioExecutionReconciliationBridgeError::SummaryCountMismatch(
                    name, observed, expected,
                ),
            );
        }
    }
    Ok(())
}

fn build_reconciliation_row(
    document: &SecurityPortfolioExecutionStatusBridgeDocument,
    row: &SecurityPortfolioExecutionStatusRow,
) -> Result<
    SecurityPortfolioExecutionReconciliationRow,
    SecurityPortfolioExecutionReconciliationBridgeError,
> {
    let reconciliation_status = match row.execution_status.as_str() {
        "applied" => "settled",
        "skipped_non_executable_hold" => "skipped_hold",
        "apply_failed" => "reconciliation_required",
        other => {
            return Err(
                SecurityPortfolioExecutionReconciliationBridgeError::UnsupportedRowExecutionStatus(
                    row.symbol.clone(),
                    other.to_string(),
                ),
            );
        }
    };
    let blockers = document
        .blockers
        .iter()
        .chain(document.pending_items.iter())
        .filter(|item| item.contains(&row.symbol))
        .cloned()
        .collect::<Vec<_>>();
    let requires_manual_follow_up = row.execution_status == "apply_failed"
        && blockers.iter().any(|item| {
            let normalized = item.to_ascii_lowercase();
            normalized.contains("manual_follow_up")
                || normalized.contains("manual follow")
                || normalized.contains("manual-follow")
        });

    Ok(SecurityPortfolioExecutionReconciliationRow {
        symbol: row.symbol.clone(),
        request_action: row.request_action.clone(),
        requested_gross_pct: row.requested_gross_pct,
        enrichment_status: row.enrichment_status.clone(),
        apply_status: row.apply_status.clone(),
        execution_status: row.execution_status.clone(),
        reconciliation_status: reconciliation_status.to_string(),
        execution_record_ref: row.execution_record_ref.clone(),
        execution_journal_ref: row.execution_journal_ref.clone(),
        requires_manual_follow_up,
        blockers,
        reconciliation_summary: format!(
            "{} reconciled execution status {} into {}",
            row.symbol, row.execution_status, reconciliation_status
        ),
    })
}

fn map_reconciliation_status(
    document: &SecurityPortfolioExecutionStatusBridgeDocument,
) -> Result<String, SecurityPortfolioExecutionReconciliationBridgeError> {
    match document.execution_status.as_str() {
        "fully_applied" => Ok("fully_settled".to_string()),
        "applied_with_open_items" | "applied_with_skipped_holds" | "partial_failure" => {
            Ok("reconciliation_required".to_string())
        }
        "rejected" => Ok("blocked".to_string()),
        other => Err(
            SecurityPortfolioExecutionReconciliationBridgeError::UnsupportedExecutionStatus(
                other.to_string(),
            ),
        ),
    }
}

fn build_pending_items(
    document: &SecurityPortfolioExecutionStatusBridgeDocument,
    rows: &[SecurityPortfolioExecutionReconciliationRow],
) -> Vec<String> {
    let mut pending_items = document.pending_items.clone();
    pending_items.extend(
        rows.iter()
            .filter(|row| row.reconciliation_status == "reconciliation_required")
            .map(|row| {
                format!(
                    "{} requires reconciliation after status bridge {}",
                    row.symbol, document.portfolio_execution_status_bridge_id
                )
            }),
    );
    pending_items
}

fn build_blockers(
    document: &SecurityPortfolioExecutionStatusBridgeDocument,
    rows: &[SecurityPortfolioExecutionReconciliationRow],
) -> Vec<String> {
    let mut blockers = document.blockers.clone();
    blockers.extend(
        rows.iter()
            .filter(|row| row.reconciliation_status == "reconciliation_required")
            .flat_map(|row| row.blockers.iter().cloned()),
    );
    blockers.sort();
    blockers.dedup();
    blockers
}

fn build_reconciliation_rationale(
    document: &SecurityPortfolioExecutionStatusBridgeDocument,
) -> Vec<String> {
    vec![
        format!(
            "execution reconciliation bridge consumed status bridge {}",
            document.portfolio_execution_status_bridge_id
        ),
        "execution reconciliation bridge only freezes settled and unresolved execution truth"
            .to_string(),
        "execution reconciliation bridge does not repair, replay, broker-execute, or materialize positions"
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
