use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_portfolio_execution_repair_package::{
    SecurityPortfolioExecutionRepairPackageDocument, SecurityPortfolioExecutionRepairRow,
};

const SECURITY_PORTFOLIO_EXECUTION_REPLAY_REQUEST_PACKAGE_DOCUMENT_TYPE: &str =
    "security_portfolio_execution_replay_request_package";
const SECURITY_PORTFOLIO_EXECUTION_REPLAY_REQUEST_PACKAGE_VERSION: &str =
    "security_portfolio_execution_replay_request_package.v1";

// 2026-04-25 CST: Added because P19A needs a strict side-effect-free request
// boundary after the recovered P18 repair-intent package.
// Reason: replay intent must be frozen before any later executor can be designed.
// Purpose: define the public request shell for replay-request packaging.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityPortfolioExecutionReplayRequestPackageRequest {
    pub portfolio_execution_repair_package: SecurityPortfolioExecutionRepairPackageDocument,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-25 CST: Added because P19A must pass only governed retry candidates
// to a future executor contract.
// Reason: manual and governance-blocked repair rows are not replayable work.
// Purpose: represent one eligible P18 row as an auditable replay request row.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionReplayRequestRow {
    pub symbol: String,
    pub request_action: String,
    pub requested_gross_pct: f64,
    pub repair_class: String,
    pub replay_request_status: String,
    pub portfolio_execution_repair_package_ref: String,
    #[serde(default)]
    pub execution_record_ref: Option<String>,
    #[serde(default)]
    pub execution_journal_ref: Option<String>,
    pub replay_evidence_refs: Vec<String>,
    pub replay_blockers: Vec<String>,
    pub replay_request_summary: String,
}

// 2026-04-25 CST: Added because P19A is a request-freeze document, not a
// replay executor hidden inside P18.
// Reason: keeping replay request separate prevents runtime writes and lifecycle
// semantics from leaking into repair classification.
// Purpose: define the formal replay-request package document.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionReplayRequestPackageDocument {
    pub portfolio_execution_replay_request_package_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub analysis_date: String,
    pub account_id: String,
    pub portfolio_execution_repair_package_ref: String,
    pub portfolio_execution_reconciliation_bridge_ref: String,
    pub portfolio_execution_status_bridge_ref: String,
    pub portfolio_execution_apply_bridge_ref: String,
    pub portfolio_execution_request_enrichment_ref: String,
    pub portfolio_execution_request_package_ref: String,
    pub portfolio_execution_preview_ref: String,
    pub portfolio_allocation_decision_ref: String,
    pub replay_request_rows: Vec<SecurityPortfolioExecutionReplayRequestRow>,
    pub governed_retry_candidate_count: usize,
    pub excluded_manual_follow_up_count: usize,
    pub excluded_blocked_pending_decision_count: usize,
    pub replay_request_count: usize,
    pub replay_request_status: String,
    pub blockers: Vec<String>,
    pub replay_request_rationale: Vec<String>,
    pub replay_request_summary: String,
}

// 2026-04-25 CST: Added because public stock tools return named wrappers
// instead of bare documents.
// Reason: stable top-level response keys make CLI outputs easier to consume.
// Purpose: wrap the P19A document under one governed key.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionReplayRequestPackageResult {
    pub portfolio_execution_replay_request_package:
        SecurityPortfolioExecutionReplayRequestPackageDocument,
}

// 2026-04-25 CST: Added because P19A must reject drifted P18 truth instead of
// inventing replay work.
// Reason: a replay request package is an executor input contract, not a heuristic sink.
// Purpose: keep malformed P18 inputs hard-failing and traceable.
#[derive(Debug, Error)]
pub enum SecurityPortfolioExecutionReplayRequestPackageError {
    #[error(
        "security portfolio execution replay request package build failed: missing lineage ref `{0}`"
    )]
    MissingLineageRef(&'static str),
    #[error(
        "security portfolio execution replay request package build failed: unsupported repair status `{0}`"
    )]
    UnsupportedRepairStatus(String),
    #[error(
        "security portfolio execution replay request package build failed: unsupported repair class `{0}`"
    )]
    UnsupportedRepairClass(String),
    #[error(
        "security portfolio execution replay request package build failed: summary count mismatch `{0}` observed `{1}` expected `{2}`"
    )]
    SummaryCountMismatch(&'static str, usize, usize),
    #[error(
        "security portfolio execution replay request package build failed: missing replay evidence for `{0}`"
    )]
    MissingReplayEvidence(String),
}

// 2026-04-25 CST: Added because the P19A tool must be callable from the
// public stock bus immediately after P18.
// Reason: downstream callers should not construct replay request rows outside the formal boundary.
// Purpose: expose the P18-to-P19A replay-request package entry point.
pub fn security_portfolio_execution_replay_request_package(
    request: &SecurityPortfolioExecutionReplayRequestPackageRequest,
) -> Result<
    SecurityPortfolioExecutionReplayRequestPackageResult,
    SecurityPortfolioExecutionReplayRequestPackageError,
> {
    build_security_portfolio_execution_replay_request_package(request)
}

pub fn build_security_portfolio_execution_replay_request_package(
    request: &SecurityPortfolioExecutionReplayRequestPackageRequest,
) -> Result<
    SecurityPortfolioExecutionReplayRequestPackageResult,
    SecurityPortfolioExecutionReplayRequestPackageError,
> {
    let generated_at = normalize_created_at(&request.created_at);
    let repair_document = &request.portfolio_execution_repair_package;
    validate_lineage(repair_document)?;
    validate_repair_status(repair_document)?;
    validate_supported_repair_classes(repair_document)?;
    validate_summary_counts(repair_document)?;

    let replay_request_rows = repair_document
        .repair_rows
        .iter()
        .filter(|row| row.repair_class == "governed_retry_candidate")
        .map(|row| build_replay_request_row(row, repair_document))
        .collect::<Result<Vec<_>, _>>()?;
    let governed_retry_candidate_count = replay_request_rows.len();
    let excluded_manual_follow_up_count = repair_document
        .repair_rows
        .iter()
        .filter(|row| row.repair_class == "manual_follow_up")
        .count();
    let excluded_blocked_pending_decision_count = repair_document
        .repair_rows
        .iter()
        .filter(|row| row.repair_class == "blocked_pending_decision")
        .count();
    let replay_request_count = replay_request_rows.len();
    let replay_request_status = if replay_request_count == 0 {
        "no_replay_requested"
    } else {
        "replay_requested"
    };

    Ok(SecurityPortfolioExecutionReplayRequestPackageResult {
        portfolio_execution_replay_request_package:
            SecurityPortfolioExecutionReplayRequestPackageDocument {
                portfolio_execution_replay_request_package_id: format!(
                    "portfolio-execution-replay-request-package:{}:{}",
                    repair_document.account_id, generated_at
                ),
                contract_version: SECURITY_PORTFOLIO_EXECUTION_REPLAY_REQUEST_PACKAGE_VERSION
                    .to_string(),
                document_type: SECURITY_PORTFOLIO_EXECUTION_REPLAY_REQUEST_PACKAGE_DOCUMENT_TYPE
                    .to_string(),
                generated_at,
                analysis_date: repair_document.analysis_date.clone(),
                account_id: repair_document.account_id.clone(),
                portfolio_execution_repair_package_ref: repair_document
                    .portfolio_execution_repair_package_id
                    .clone(),
                portfolio_execution_reconciliation_bridge_ref: repair_document
                    .portfolio_execution_reconciliation_bridge_ref
                    .clone(),
                portfolio_execution_status_bridge_ref: repair_document
                    .portfolio_execution_status_bridge_ref
                    .clone(),
                portfolio_execution_apply_bridge_ref: repair_document
                    .portfolio_execution_apply_bridge_ref
                    .clone(),
                portfolio_execution_request_enrichment_ref: repair_document
                    .portfolio_execution_request_enrichment_ref
                    .clone(),
                portfolio_execution_request_package_ref: repair_document
                    .portfolio_execution_request_package_ref
                    .clone(),
                portfolio_execution_preview_ref: repair_document
                    .portfolio_execution_preview_ref
                    .clone(),
                portfolio_allocation_decision_ref: repair_document
                    .portfolio_allocation_decision_ref
                    .clone(),
                replay_request_rows,
                governed_retry_candidate_count,
                excluded_manual_follow_up_count,
                excluded_blocked_pending_decision_count,
                replay_request_count,
                replay_request_status: replay_request_status.to_string(),
                blockers: repair_document.blockers.clone(),
                replay_request_rationale: build_replay_request_rationale(repair_document),
                replay_request_summary: format!(
                    "account {} froze repair package {} as replay request status {}",
                    repair_document.account_id,
                    repair_document.portfolio_execution_repair_package_id,
                    replay_request_status
                ),
            },
    })
}

fn validate_lineage(
    document: &SecurityPortfolioExecutionRepairPackageDocument,
) -> Result<(), SecurityPortfolioExecutionReplayRequestPackageError> {
    for (name, value) in [
        (
            "portfolio_execution_repair_package_id",
            document.portfolio_execution_repair_package_id.as_str(),
        ),
        (
            "portfolio_execution_reconciliation_bridge_ref",
            document
                .portfolio_execution_reconciliation_bridge_ref
                .as_str(),
        ),
        (
            "portfolio_execution_status_bridge_ref",
            document.portfolio_execution_status_bridge_ref.as_str(),
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
                SecurityPortfolioExecutionReplayRequestPackageError::MissingLineageRef(name),
            );
        }
    }
    Ok(())
}

fn validate_repair_status(
    document: &SecurityPortfolioExecutionRepairPackageDocument,
) -> Result<(), SecurityPortfolioExecutionReplayRequestPackageError> {
    match document.repair_status.as_str() {
        "no_repair_required" | "repair_required" => Ok(()),
        other => Err(
            SecurityPortfolioExecutionReplayRequestPackageError::UnsupportedRepairStatus(
                other.to_string(),
            ),
        ),
    }
}

fn validate_supported_repair_classes(
    document: &SecurityPortfolioExecutionRepairPackageDocument,
) -> Result<(), SecurityPortfolioExecutionReplayRequestPackageError> {
    for row in &document.repair_rows {
        match row.repair_class.as_str() {
            "manual_follow_up" | "governed_retry_candidate" | "blocked_pending_decision" => {}
            other => {
                return Err(
                    SecurityPortfolioExecutionReplayRequestPackageError::UnsupportedRepairClass(
                        other.to_string(),
                    ),
                );
            }
        }
    }
    Ok(())
}

fn validate_summary_counts(
    document: &SecurityPortfolioExecutionRepairPackageDocument,
) -> Result<(), SecurityPortfolioExecutionReplayRequestPackageError> {
    let observed_manual = document
        .repair_rows
        .iter()
        .filter(|row| row.repair_class == "manual_follow_up")
        .count();
    let observed_retry = document
        .repair_rows
        .iter()
        .filter(|row| row.repair_class == "governed_retry_candidate")
        .count();
    let observed_blocked = document
        .repair_rows
        .iter()
        .filter(|row| row.repair_class == "blocked_pending_decision")
        .count();

    for (name, observed, expected) in [
        (
            "manual_follow_up_count",
            observed_manual,
            document.manual_follow_up_count,
        ),
        (
            "governed_retry_candidate_count",
            observed_retry,
            document.governed_retry_candidate_count,
        ),
        (
            "blocked_pending_decision_count",
            observed_blocked,
            document.blocked_pending_decision_count,
        ),
        (
            "repair_required_count",
            document.repair_rows.len(),
            document.repair_required_count,
        ),
    ] {
        if observed != expected {
            return Err(
                SecurityPortfolioExecutionReplayRequestPackageError::SummaryCountMismatch(
                    name, observed, expected,
                ),
            );
        }
    }

    if document.repair_status == "no_repair_required" && !document.repair_rows.is_empty() {
        return Err(
            SecurityPortfolioExecutionReplayRequestPackageError::SummaryCountMismatch(
                "no_repair_required_repair_rows",
                document.repair_rows.len(),
                0,
            ),
        );
    }
    Ok(())
}

fn build_replay_request_row(
    row: &SecurityPortfolioExecutionRepairRow,
    document: &SecurityPortfolioExecutionRepairPackageDocument,
) -> Result<
    SecurityPortfolioExecutionReplayRequestRow,
    SecurityPortfolioExecutionReplayRequestPackageError,
> {
    let evidence_refs = collect_replay_evidence_refs(row);
    if evidence_refs.is_empty() {
        return Err(
            SecurityPortfolioExecutionReplayRequestPackageError::MissingReplayEvidence(
                row.symbol.clone(),
            ),
        );
    }

    Ok(SecurityPortfolioExecutionReplayRequestRow {
        symbol: row.symbol.clone(),
        request_action: row.request_action.clone(),
        requested_gross_pct: row.requested_gross_pct,
        repair_class: row.repair_class.clone(),
        replay_request_status: "ready_for_replay_request".to_string(),
        portfolio_execution_repair_package_ref: document
            .portfolio_execution_repair_package_id
            .clone(),
        execution_record_ref: row.execution_record_ref.clone(),
        execution_journal_ref: row.execution_journal_ref.clone(),
        replay_evidence_refs: evidence_refs,
        replay_blockers: row.repair_blockers.clone(),
        replay_request_summary: format!("{} frozen as governed replay request", row.symbol),
    })
}

fn collect_replay_evidence_refs(row: &SecurityPortfolioExecutionRepairRow) -> Vec<String> {
    let mut evidence_refs = Vec::new();
    if let Some(execution_record_ref) = row.execution_record_ref.as_deref() {
        evidence_refs.push(format!("execution_record_ref:{execution_record_ref}"));
    }
    if let Some(execution_journal_ref) = row.execution_journal_ref.as_deref() {
        evidence_refs.push(format!("execution_journal_ref:{execution_journal_ref}"));
    }
    if row.repair_blockers.iter().any(|blocker| {
        let lower = blocker.to_ascii_lowercase();
        lower.contains("retry") || lower.contains("replay")
    }) {
        evidence_refs.push("repair_blocker_retry_or_replay_signal".to_string());
    }
    evidence_refs
}

fn build_replay_request_rationale(
    document: &SecurityPortfolioExecutionRepairPackageDocument,
) -> Vec<String> {
    vec![
        format!(
            "execution replay request package consumed repair package {}",
            document.portfolio_execution_repair_package_id
        ),
        "execution replay request package includes only governed retry candidates".to_string(),
        "execution replay request package does not write runtime facts, replay broker fills, materialize positions, or close lifecycle"
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
