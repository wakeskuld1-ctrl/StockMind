use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_portfolio_execution_reconciliation_bridge::{
    SecurityPortfolioExecutionReconciliationBridgeDocument,
    SecurityPortfolioExecutionReconciliationRow,
};

const SECURITY_PORTFOLIO_EXECUTION_REPAIR_PACKAGE_DOCUMENT_TYPE: &str =
    "security_portfolio_execution_repair_package";
const SECURITY_PORTFOLIO_EXECUTION_REPAIR_PACKAGE_VERSION: &str =
    "security_portfolio_execution_repair_package.v1";

// 2026-04-25 CST: Added because P18 recovery needs one formal repair-intent
// request downstream of the recovered P17 reconciliation artifact.
// Reason: repair classification must consume frozen reconciliation truth, not raw status rows.
// Purpose: define the public request shell for a side-effect-free repair package.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityPortfolioExecutionRepairPackageRequest {
    pub portfolio_execution_reconciliation_bridge:
        SecurityPortfolioExecutionReconciliationBridgeDocument,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-25 CST: Added because P18 must freeze the next repair intent without
// running replay, retry, broker execution, or lifecycle closeout.
// Reason: P19 needs typed intent rows instead of prose blockers.
// Purpose: represent one unresolved P17 row as an explicit repair class.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionRepairRow {
    pub symbol: String,
    pub request_action: String,
    pub requested_gross_pct: f64,
    pub reconciliation_status: String,
    pub repair_class: String,
    #[serde(default)]
    pub execution_record_ref: Option<String>,
    #[serde(default)]
    pub execution_journal_ref: Option<String>,
    pub repair_blockers: Vec<String>,
    pub repair_summary: String,
}

// 2026-04-25 CST: Added because P18 is a formal package boundary after P17,
// not an executor hidden in reconciliation code.
// Reason: keeping repair intent separate prevents replay and lifecycle logic from leaking into P17.
// Purpose: define the recovered P18 repair-intent document.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionRepairPackageDocument {
    pub portfolio_execution_repair_package_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub analysis_date: String,
    pub account_id: String,
    pub portfolio_execution_reconciliation_bridge_ref: String,
    pub portfolio_execution_status_bridge_ref: String,
    pub portfolio_execution_apply_bridge_ref: String,
    pub portfolio_execution_request_enrichment_ref: String,
    pub portfolio_execution_request_package_ref: String,
    pub portfolio_execution_preview_ref: String,
    pub portfolio_allocation_decision_ref: String,
    pub repair_rows: Vec<SecurityPortfolioExecutionRepairRow>,
    pub manual_follow_up_count: usize,
    pub governed_retry_candidate_count: usize,
    pub blocked_pending_decision_count: usize,
    pub repair_required_count: usize,
    pub repair_status: String,
    pub blockers: Vec<String>,
    pub repair_rationale: Vec<String>,
    pub repair_summary: String,
}

// 2026-04-25 CST: Added because public stock tools return named wrappers
// instead of bare documents.
// Reason: stable top-level response keys make CLI outputs easier to consume.
// Purpose: wrap the P18 document under one governed key.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionRepairPackageResult {
    pub portfolio_execution_repair_package: SecurityPortfolioExecutionRepairPackageDocument,
}

// 2026-04-25 CST: Added because P18 must refuse ambiguous or drifted repair
// inputs instead of guessing the next action.
// Reason: repair-intent classification is a governance boundary, not a heuristic sink.
// Purpose: keep malformed P17 inputs hard-failing and traceable.
#[derive(Debug, Error)]
pub enum SecurityPortfolioExecutionRepairPackageError {
    #[error("security portfolio execution repair package build failed: missing lineage ref `{0}`")]
    MissingLineageRef(&'static str),
    #[error(
        "security portfolio execution repair package build failed: unsupported reconciliation status `{0}`"
    )]
    UnsupportedReconciliationStatus(String),
    #[error(
        "security portfolio execution repair package build failed: summary count mismatch `{0}` observed `{1}` expected `{2}`"
    )]
    SummaryCountMismatch(&'static str, usize, usize),
    #[error(
        "security portfolio execution repair package build failed: ambiguous repair classification on `{0}`"
    )]
    AmbiguousRepairClassification(String),
}

// 2026-04-25 CST: Added because the recovered P18 tool must be callable from
// the public stock bus after P17.
// Reason: callers should not classify repair actions outside the formal package boundary.
// Purpose: expose the P17-to-P18 repair-intent package entry point.
pub fn security_portfolio_execution_repair_package(
    request: &SecurityPortfolioExecutionRepairPackageRequest,
) -> Result<
    SecurityPortfolioExecutionRepairPackageResult,
    SecurityPortfolioExecutionRepairPackageError,
> {
    build_security_portfolio_execution_repair_package(request)
}

pub fn build_security_portfolio_execution_repair_package(
    request: &SecurityPortfolioExecutionRepairPackageRequest,
) -> Result<
    SecurityPortfolioExecutionRepairPackageResult,
    SecurityPortfolioExecutionRepairPackageError,
> {
    let generated_at = normalize_created_at(&request.created_at);
    let reconciliation_document = &request.portfolio_execution_reconciliation_bridge;
    validate_lineage(reconciliation_document)?;
    validate_summary_counts(reconciliation_document)?;
    validate_reconciliation_status(reconciliation_document)?;

    let repair_rows = reconciliation_document
        .reconciliation_rows
        .iter()
        .filter(|row| row.reconciliation_status == "reconciliation_required")
        .map(build_repair_row)
        .collect::<Result<Vec<_>, _>>()?;
    let manual_follow_up_count = repair_rows
        .iter()
        .filter(|row| row.repair_class == "manual_follow_up")
        .count();
    let governed_retry_candidate_count = repair_rows
        .iter()
        .filter(|row| row.repair_class == "governed_retry_candidate")
        .count();
    let blocked_pending_decision_count = repair_rows
        .iter()
        .filter(|row| row.repair_class == "blocked_pending_decision")
        .count();
    let repair_required_count = repair_rows.len();
    let repair_status = if repair_required_count == 0 {
        "no_repair_required"
    } else {
        "repair_required"
    };

    Ok(SecurityPortfolioExecutionRepairPackageResult {
        portfolio_execution_repair_package: SecurityPortfolioExecutionRepairPackageDocument {
            portfolio_execution_repair_package_id: format!(
                "portfolio-execution-repair-package:{}:{}",
                reconciliation_document.account_id, generated_at
            ),
            contract_version: SECURITY_PORTFOLIO_EXECUTION_REPAIR_PACKAGE_VERSION.to_string(),
            document_type: SECURITY_PORTFOLIO_EXECUTION_REPAIR_PACKAGE_DOCUMENT_TYPE.to_string(),
            generated_at,
            analysis_date: reconciliation_document.analysis_date.clone(),
            account_id: reconciliation_document.account_id.clone(),
            portfolio_execution_reconciliation_bridge_ref: reconciliation_document
                .portfolio_execution_reconciliation_bridge_id
                .clone(),
            portfolio_execution_status_bridge_ref: reconciliation_document
                .portfolio_execution_status_bridge_ref
                .clone(),
            portfolio_execution_apply_bridge_ref: reconciliation_document
                .portfolio_execution_apply_bridge_ref
                .clone(),
            portfolio_execution_request_enrichment_ref: reconciliation_document
                .portfolio_execution_request_enrichment_ref
                .clone(),
            portfolio_execution_request_package_ref: reconciliation_document
                .portfolio_execution_request_package_ref
                .clone(),
            portfolio_execution_preview_ref: reconciliation_document
                .portfolio_execution_preview_ref
                .clone(),
            portfolio_allocation_decision_ref: reconciliation_document
                .portfolio_allocation_decision_ref
                .clone(),
            repair_rows,
            manual_follow_up_count,
            governed_retry_candidate_count,
            blocked_pending_decision_count,
            repair_required_count,
            repair_status: repair_status.to_string(),
            blockers: reconciliation_document.blockers.clone(),
            repair_rationale: build_repair_rationale(reconciliation_document),
            repair_summary: format!(
                "account {} froze reconciliation bridge {} as repair status {}",
                reconciliation_document.account_id,
                reconciliation_document.portfolio_execution_reconciliation_bridge_id,
                repair_status
            ),
        },
    })
}

fn validate_lineage(
    document: &SecurityPortfolioExecutionReconciliationBridgeDocument,
) -> Result<(), SecurityPortfolioExecutionRepairPackageError> {
    for (name, value) in [
        (
            "portfolio_execution_reconciliation_bridge_id",
            document
                .portfolio_execution_reconciliation_bridge_id
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
            return Err(SecurityPortfolioExecutionRepairPackageError::MissingLineageRef(name));
        }
    }
    Ok(())
}

fn validate_summary_counts(
    document: &SecurityPortfolioExecutionReconciliationBridgeDocument,
) -> Result<(), SecurityPortfolioExecutionRepairPackageError> {
    let observed_settled = document
        .reconciliation_rows
        .iter()
        .filter(|row| row.reconciliation_status == "settled")
        .count();
    let observed_skipped = document
        .reconciliation_rows
        .iter()
        .filter(|row| row.reconciliation_status == "skipped_hold")
        .count();
    let observed_required = document
        .reconciliation_rows
        .iter()
        .filter(|row| row.reconciliation_status == "reconciliation_required")
        .count();
    let observed_manual = document
        .reconciliation_rows
        .iter()
        .filter(|row| row.requires_manual_follow_up)
        .count();

    for (name, observed, expected) in [
        ("settled_count", observed_settled, document.settled_count),
        (
            "skipped_hold_count",
            observed_skipped,
            document.skipped_hold_count,
        ),
        (
            "reconciliation_required_count",
            observed_required,
            document.reconciliation_required_count,
        ),
        (
            "manual_follow_up_count",
            observed_manual,
            document.manual_follow_up_count,
        ),
    ] {
        if observed != expected {
            return Err(
                SecurityPortfolioExecutionRepairPackageError::SummaryCountMismatch(
                    name, observed, expected,
                ),
            );
        }
    }
    Ok(())
}

fn validate_reconciliation_status(
    document: &SecurityPortfolioExecutionReconciliationBridgeDocument,
) -> Result<(), SecurityPortfolioExecutionRepairPackageError> {
    match document.reconciliation_status.as_str() {
        "fully_settled" | "reconciliation_required" | "blocked" => Ok(()),
        other => Err(
            SecurityPortfolioExecutionRepairPackageError::UnsupportedReconciliationStatus(
                other.to_string(),
            ),
        ),
    }
}

fn build_repair_row(
    row: &SecurityPortfolioExecutionReconciliationRow,
) -> Result<SecurityPortfolioExecutionRepairRow, SecurityPortfolioExecutionRepairPackageError> {
    let repair_class = classify_repair(row)?;
    Ok(SecurityPortfolioExecutionRepairRow {
        symbol: row.symbol.clone(),
        request_action: row.request_action.clone(),
        requested_gross_pct: row.requested_gross_pct,
        reconciliation_status: row.reconciliation_status.clone(),
        repair_class: repair_class.clone(),
        execution_record_ref: row.execution_record_ref.clone(),
        execution_journal_ref: row.execution_journal_ref.clone(),
        repair_blockers: row.blockers.clone(),
        repair_summary: format!("{} classified as {}", row.symbol, repair_class),
    })
}

fn classify_repair(
    row: &SecurityPortfolioExecutionReconciliationRow,
) -> Result<String, SecurityPortfolioExecutionRepairPackageError> {
    if row.requires_manual_follow_up {
        return Ok("manual_follow_up".to_string());
    }

    let joined = row.blockers.join(" ").to_ascii_lowercase();
    if joined.contains("blocked") || joined.contains("pending governance") {
        return Ok("blocked_pending_decision".to_string());
    }
    if row.execution_record_ref.is_some()
        || row.execution_journal_ref.is_some()
        || joined.contains("retryable")
        || joined.contains("retry")
    {
        return Ok("governed_retry_candidate".to_string());
    }

    Err(
        SecurityPortfolioExecutionRepairPackageError::AmbiguousRepairClassification(
            row.symbol.clone(),
        ),
    )
}

fn build_repair_rationale(
    document: &SecurityPortfolioExecutionReconciliationBridgeDocument,
) -> Vec<String> {
    vec![
        format!(
            "execution repair package consumed reconciliation bridge {}",
            document.portfolio_execution_reconciliation_bridge_id
        ),
        "execution repair package only freezes repair intent".to_string(),
        "execution repair package does not replay, broker-execute, materialize positions, or close lifecycle"
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
