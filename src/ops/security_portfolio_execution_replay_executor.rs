use std::collections::BTreeSet;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_portfolio_execution_replay_request_package::{
    SecurityPortfolioExecutionReplayRequestPackageDocument,
    SecurityPortfolioExecutionReplayRequestRow,
};

const SECURITY_PORTFOLIO_EXECUTION_REPLAY_EXECUTOR_DOCUMENT_TYPE: &str =
    "security_portfolio_execution_replay_executor";
const SECURITY_PORTFOLIO_EXECUTION_REPLAY_EXECUTOR_VERSION: &str =
    "security_portfolio_execution_replay_executor.v1";

// 2026-04-25 CST: Added because P19B needs one executor-shaped boundary after
// P19A while remaining dry-run-only in this phase.
// Reason: replay validation and idempotency must be frozen before runtime writes are approved.
// Purpose: define the public dry-run executor request shell.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityPortfolioExecutionReplayExecutorRequest {
    pub portfolio_execution_replay_request_package:
        SecurityPortfolioExecutionReplayRequestPackageDocument,
    pub execution_mode: String,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-25 CST: Added because dry-run output needs deterministic row-level
// executor truth without creating runtime records.
// Reason: later commit work needs stable idempotency evidence from this phase.
// Purpose: represent one replay request row as a dry-run executor row.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionReplayExecutorRow {
    pub symbol: String,
    pub request_action: String,
    pub requested_gross_pct: f64,
    pub repair_class: String,
    pub replay_request_status: String,
    pub dry_run_status: String,
    pub idempotency_key: String,
    pub planned_execution_record_ref: String,
    #[serde(default)]
    pub runtime_execution_record_ref: Option<String>,
    pub replay_evidence_refs: Vec<String>,
    pub executor_summary: String,
}

// 2026-04-25 CST: Added because P19B must produce executor validation truth
// without becoming runtime replay.
// Reason: dry-run documents separate executor readiness from commit semantics.
// Purpose: define the formal P19B dry-run executor document.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionReplayExecutorDocument {
    pub portfolio_execution_replay_executor_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub analysis_date: String,
    pub account_id: String,
    pub execution_mode: String,
    pub portfolio_execution_replay_request_package_ref: String,
    pub portfolio_execution_repair_package_ref: String,
    pub portfolio_execution_reconciliation_bridge_ref: String,
    pub portfolio_execution_status_bridge_ref: String,
    pub portfolio_execution_apply_bridge_ref: String,
    pub portfolio_execution_request_enrichment_ref: String,
    pub portfolio_execution_request_package_ref: String,
    pub portfolio_execution_preview_ref: String,
    pub portfolio_allocation_decision_ref: String,
    pub executor_rows: Vec<SecurityPortfolioExecutionReplayExecutorRow>,
    pub dry_run_row_count: usize,
    pub runtime_write_count: usize,
    pub dry_run_status: String,
    pub blockers: Vec<String>,
    pub executor_rationale: Vec<String>,
    pub executor_summary: String,
}

// 2026-04-25 CST: Added because public stock tools return named wrappers.
// Reason: stable response keys make CLI and downstream harness outputs deterministic.
// Purpose: wrap the P19B dry-run executor document under one governed key.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionReplayExecutorResult {
    pub portfolio_execution_replay_executor: SecurityPortfolioExecutionReplayExecutorDocument,
}

// 2026-04-25 CST: Added because P19B must reject commit-mode and malformed
// replay requests instead of degrading into runtime side effects.
// Reason: this phase owns executor validation only, not execution persistence.
// Purpose: keep dry-run executor failures explicit and traceable.
#[derive(Debug, Error)]
pub enum SecurityPortfolioExecutionReplayExecutorError {
    #[error("security portfolio execution replay executor build failed: missing lineage ref `{0}`")]
    MissingLineageRef(&'static str),
    #[error(
        "security portfolio execution replay executor build failed: unsupported execution mode `{0}`"
    )]
    UnsupportedExecutionMode(String),
    #[error(
        "security portfolio execution replay executor build failed: unsupported replay request status `{0}`"
    )]
    UnsupportedReplayRequestStatus(String),
    #[error(
        "security portfolio execution replay executor build failed: summary count mismatch `{0}` observed `{1}` expected `{2}`"
    )]
    SummaryCountMismatch(&'static str, usize, usize),
    #[error(
        "security portfolio execution replay executor build failed: unsupported replay request row status `{0}`"
    )]
    UnsupportedReplayRequestRowStatus(String),
    #[error(
        "security portfolio execution replay executor build failed: unsupported repair class `{0}`"
    )]
    UnsupportedRepairClass(String),
    #[error(
        "security portfolio execution replay executor build failed: missing replay evidence for `{0}`"
    )]
    MissingReplayEvidence(String),
    #[error(
        "security portfolio execution replay executor build failed: duplicate idempotency key `{0}`"
    )]
    DuplicateIdempotencyKey(String),
}

// 2026-04-25 CST: Added because P19B must be callable from the public stock bus
// after P19A request packaging.
// Reason: executor validation should not be assembled ad hoc by callers.
// Purpose: expose the P19A-to-P19B dry-run executor entry point.
pub fn security_portfolio_execution_replay_executor(
    request: &SecurityPortfolioExecutionReplayExecutorRequest,
) -> Result<
    SecurityPortfolioExecutionReplayExecutorResult,
    SecurityPortfolioExecutionReplayExecutorError,
> {
    build_security_portfolio_execution_replay_executor(request)
}

pub fn build_security_portfolio_execution_replay_executor(
    request: &SecurityPortfolioExecutionReplayExecutorRequest,
) -> Result<
    SecurityPortfolioExecutionReplayExecutorResult,
    SecurityPortfolioExecutionReplayExecutorError,
> {
    let generated_at = normalize_created_at(&request.created_at);
    let document = &request.portfolio_execution_replay_request_package;
    validate_execution_mode(&request.execution_mode)?;
    validate_lineage(document)?;
    validate_replay_request_status(document)?;
    validate_summary_counts(document)?;

    let mut seen_idempotency_keys = BTreeSet::new();
    let mut executor_rows = Vec::new();
    for row in &document.replay_request_rows {
        let executor_row = build_executor_row(row, document)?;
        if !seen_idempotency_keys.insert(executor_row.idempotency_key.clone()) {
            return Err(
                SecurityPortfolioExecutionReplayExecutorError::DuplicateIdempotencyKey(
                    executor_row.idempotency_key,
                ),
            );
        }
        executor_rows.push(executor_row);
    }

    let dry_run_row_count = executor_rows.len();
    let dry_run_status = if dry_run_row_count == 0 {
        "no_replay_work"
    } else {
        "validated_for_dry_run"
    };

    Ok(SecurityPortfolioExecutionReplayExecutorResult {
        portfolio_execution_replay_executor: SecurityPortfolioExecutionReplayExecutorDocument {
            portfolio_execution_replay_executor_id: format!(
                "portfolio-execution-replay-executor:{}:{}",
                document.account_id, generated_at
            ),
            contract_version: SECURITY_PORTFOLIO_EXECUTION_REPLAY_EXECUTOR_VERSION.to_string(),
            document_type: SECURITY_PORTFOLIO_EXECUTION_REPLAY_EXECUTOR_DOCUMENT_TYPE.to_string(),
            generated_at,
            analysis_date: document.analysis_date.clone(),
            account_id: document.account_id.clone(),
            execution_mode: "dry_run".to_string(),
            portfolio_execution_replay_request_package_ref: document
                .portfolio_execution_replay_request_package_id
                .clone(),
            portfolio_execution_repair_package_ref: document
                .portfolio_execution_repair_package_ref
                .clone(),
            portfolio_execution_reconciliation_bridge_ref: document
                .portfolio_execution_reconciliation_bridge_ref
                .clone(),
            portfolio_execution_status_bridge_ref: document
                .portfolio_execution_status_bridge_ref
                .clone(),
            portfolio_execution_apply_bridge_ref: document
                .portfolio_execution_apply_bridge_ref
                .clone(),
            portfolio_execution_request_enrichment_ref: document
                .portfolio_execution_request_enrichment_ref
                .clone(),
            portfolio_execution_request_package_ref: document
                .portfolio_execution_request_package_ref
                .clone(),
            portfolio_execution_preview_ref: document.portfolio_execution_preview_ref.clone(),
            portfolio_allocation_decision_ref: document.portfolio_allocation_decision_ref.clone(),
            executor_rows,
            dry_run_row_count,
            runtime_write_count: 0,
            dry_run_status: dry_run_status.to_string(),
            blockers: document.blockers.clone(),
            executor_rationale: build_executor_rationale(document),
            executor_summary: format!(
                "account {} validated replay request package {} as {}",
                document.account_id,
                document.portfolio_execution_replay_request_package_id,
                dry_run_status
            ),
        },
    })
}

fn validate_execution_mode(
    execution_mode: &str,
) -> Result<(), SecurityPortfolioExecutionReplayExecutorError> {
    match execution_mode.trim() {
        "dry_run" => Ok(()),
        other => Err(
            SecurityPortfolioExecutionReplayExecutorError::UnsupportedExecutionMode(
                other.to_string(),
            ),
        ),
    }
}

fn validate_lineage(
    document: &SecurityPortfolioExecutionReplayRequestPackageDocument,
) -> Result<(), SecurityPortfolioExecutionReplayExecutorError> {
    for (name, value) in [
        (
            "portfolio_execution_replay_request_package_id",
            document
                .portfolio_execution_replay_request_package_id
                .as_str(),
        ),
        (
            "portfolio_execution_repair_package_ref",
            document.portfolio_execution_repair_package_ref.as_str(),
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
            return Err(SecurityPortfolioExecutionReplayExecutorError::MissingLineageRef(name));
        }
    }
    Ok(())
}

fn validate_replay_request_status(
    document: &SecurityPortfolioExecutionReplayRequestPackageDocument,
) -> Result<(), SecurityPortfolioExecutionReplayExecutorError> {
    match document.replay_request_status.as_str() {
        "no_replay_requested" | "replay_requested" => Ok(()),
        other => Err(
            SecurityPortfolioExecutionReplayExecutorError::UnsupportedReplayRequestStatus(
                other.to_string(),
            ),
        ),
    }
}

fn validate_summary_counts(
    document: &SecurityPortfolioExecutionReplayRequestPackageDocument,
) -> Result<(), SecurityPortfolioExecutionReplayExecutorError> {
    let observed_retry = document
        .replay_request_rows
        .iter()
        .filter(|row| row.repair_class == "governed_retry_candidate")
        .count();
    for (name, observed, expected) in [
        (
            "governed_retry_candidate_count",
            observed_retry,
            document.governed_retry_candidate_count,
        ),
        (
            "replay_request_count",
            document.replay_request_rows.len(),
            document.replay_request_count,
        ),
    ] {
        if observed != expected {
            return Err(
                SecurityPortfolioExecutionReplayExecutorError::SummaryCountMismatch(
                    name, observed, expected,
                ),
            );
        }
    }

    if document.replay_request_status == "no_replay_requested"
        && !document.replay_request_rows.is_empty()
    {
        return Err(
            SecurityPortfolioExecutionReplayExecutorError::SummaryCountMismatch(
                "no_replay_requested_replay_rows",
                document.replay_request_rows.len(),
                0,
            ),
        );
    }
    Ok(())
}

fn build_executor_row(
    row: &SecurityPortfolioExecutionReplayRequestRow,
    document: &SecurityPortfolioExecutionReplayRequestPackageDocument,
) -> Result<
    SecurityPortfolioExecutionReplayExecutorRow,
    SecurityPortfolioExecutionReplayExecutorError,
> {
    if row.repair_class != "governed_retry_candidate" {
        return Err(
            SecurityPortfolioExecutionReplayExecutorError::UnsupportedRepairClass(
                row.repair_class.clone(),
            ),
        );
    }
    if row.replay_request_status != "ready_for_replay_request" {
        return Err(
            SecurityPortfolioExecutionReplayExecutorError::UnsupportedReplayRequestRowStatus(
                row.replay_request_status.clone(),
            ),
        );
    }
    if row.replay_evidence_refs.is_empty() {
        return Err(
            SecurityPortfolioExecutionReplayExecutorError::MissingReplayEvidence(
                row.symbol.clone(),
            ),
        );
    }

    let idempotency_key = build_idempotency_key(row, document);
    Ok(SecurityPortfolioExecutionReplayExecutorRow {
        symbol: row.symbol.clone(),
        request_action: row.request_action.clone(),
        requested_gross_pct: row.requested_gross_pct,
        repair_class: row.repair_class.clone(),
        replay_request_status: row.replay_request_status.clone(),
        dry_run_status: "validated_for_dry_run".to_string(),
        idempotency_key,
        planned_execution_record_ref: format!(
            "dry-run:{}:{}",
            document.portfolio_execution_replay_request_package_id, row.symbol
        ),
        runtime_execution_record_ref: None,
        replay_evidence_refs: row.replay_evidence_refs.clone(),
        executor_summary: format!("{} validated for replay dry-run", row.symbol),
    })
}

fn build_idempotency_key(
    row: &SecurityPortfolioExecutionReplayRequestRow,
    document: &SecurityPortfolioExecutionReplayRequestPackageDocument,
) -> String {
    let evidence_refs = row.replay_evidence_refs.join(",");
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        document.account_id,
        document.analysis_date,
        row.symbol,
        row.request_action,
        row.requested_gross_pct,
        document.portfolio_execution_replay_request_package_id,
        evidence_refs
    )
}

fn build_executor_rationale(
    document: &SecurityPortfolioExecutionReplayRequestPackageDocument,
) -> Vec<String> {
    vec![
        format!(
            "execution replay executor consumed replay request package {}",
            document.portfolio_execution_replay_request_package_id
        ),
        "execution replay executor is dry-run-only in this phase".to_string(),
        "execution replay executor does not write runtime facts, replay broker fills, materialize positions, or close lifecycle"
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
