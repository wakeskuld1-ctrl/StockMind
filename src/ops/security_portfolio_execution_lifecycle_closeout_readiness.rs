use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_portfolio_execution_replay_commit_audit::{
    SecurityPortfolioExecutionReplayCommitAuditDocument,
    SecurityPortfolioExecutionReplayCommitAuditRow,
};

const P20A_DOCUMENT_TYPE: &str = "security_portfolio_execution_lifecycle_closeout_readiness";
const P20A_CONTRACT_VERSION: &str = "security_portfolio_execution_lifecycle_closeout_readiness.v1";

// 2026-04-26 CST: Added because P19E audit truth needs one side-effect-free readiness gate.
// Purpose: separate closeout eligibility from any later writer or archive-producing phase.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityPortfolioExecutionLifecycleCloseoutReadinessRequest {
    pub portfolio_execution_replay_commit_audit:
        SecurityPortfolioExecutionReplayCommitAuditDocument,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionLifecycleCloseoutReadinessRow {
    pub symbol: String,
    pub source_p19e_audit_status: String,
    pub readiness_status: String,
    pub commit_idempotency_key: String,
    pub canonical_commit_payload_hash: String,
    pub source_p19c_ref: String,
    pub target_execution_record_ref: String,
    #[serde(default)]
    pub runtime_execution_record_ref: Option<String>,
    #[serde(default)]
    pub runtime_replay_commit_idempotency_key: Option<String>,
    #[serde(default)]
    pub runtime_replay_commit_payload_hash: Option<String>,
    #[serde(default)]
    pub runtime_replay_commit_source_p19c_ref: Option<String>,
    pub closeout_preflight_eligible: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionLifecycleCloseoutReadinessDocument {
    pub portfolio_execution_lifecycle_closeout_readiness_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub analysis_date: String,
    pub account_id: String,
    pub source_p19e_ref: String,
    pub source_p19d_ref: String,
    pub source_p19c_ref: String,
    pub source_non_atomicity_notice: String,
    pub readiness_rows: Vec<SecurityPortfolioExecutionLifecycleCloseoutReadinessRow>,
    pub readiness_row_count: usize,
    pub eligible_for_closeout_preflight_count: usize,
    pub blocked_missing_runtime_record_count: usize,
    pub blocked_metadata_mismatch_count: usize,
    pub blocked_commit_failed_count: usize,
    pub blocked_idempotency_conflict_count: usize,
    pub blocked_no_commit_work_count: usize,
    pub blocked_not_auditable_count: usize,
    pub blocked_unknown_audit_status_count: usize,
    pub runtime_write_count: usize,
    pub readiness_status: String,
    pub blockers: Vec<String>,
    pub readiness_rationale: Vec<String>,
    pub readiness_summary: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionLifecycleCloseoutReadinessResult {
    pub portfolio_execution_lifecycle_closeout_readiness:
        SecurityPortfolioExecutionLifecycleCloseoutReadinessDocument,
}

#[derive(Debug, Error)]
pub enum SecurityPortfolioExecutionLifecycleCloseoutReadinessError {
    #[error(
        "security portfolio execution lifecycle closeout readiness build failed: unsupported P19E document type `{0}`"
    )]
    UnsupportedAuditDocumentType(String),
    #[error(
        "security portfolio execution lifecycle closeout readiness build failed: unsupported P19E contract version `{0}`"
    )]
    UnsupportedAuditContractVersion(String),
    #[error(
        "security portfolio execution lifecycle closeout readiness build failed: P19E runtime write count must be zero, observed `{0}`"
    )]
    AuditRuntimeWriteCount(usize),
    #[error(
        "security portfolio execution lifecycle closeout readiness build failed: source P19D ref is required"
    )]
    MissingSourceP19DRef,
    #[error(
        "security portfolio execution lifecycle closeout readiness build failed: source P19C ref is required"
    )]
    MissingSourceP19CRef,
    #[error(
        "security portfolio execution lifecycle closeout readiness build failed: source non-atomicity notice is required"
    )]
    MissingSourceNonAtomicityNotice,
    #[error(
        "security portfolio execution lifecycle closeout readiness build failed: eligible row `{0}` is missing machine-readable replay evidence"
    )]
    MissingEligibleRowEvidence(String),
}

pub fn security_portfolio_execution_lifecycle_closeout_readiness(
    request: &SecurityPortfolioExecutionLifecycleCloseoutReadinessRequest,
) -> Result<
    SecurityPortfolioExecutionLifecycleCloseoutReadinessResult,
    SecurityPortfolioExecutionLifecycleCloseoutReadinessError,
> {
    build_security_portfolio_execution_lifecycle_closeout_readiness(request)
}

pub fn build_security_portfolio_execution_lifecycle_closeout_readiness(
    request: &SecurityPortfolioExecutionLifecycleCloseoutReadinessRequest,
) -> Result<
    SecurityPortfolioExecutionLifecycleCloseoutReadinessResult,
    SecurityPortfolioExecutionLifecycleCloseoutReadinessError,
> {
    validate_audit_document(&request.portfolio_execution_replay_commit_audit)?;

    let generated_at = normalize_created_at(&request.created_at);
    let audit = &request.portfolio_execution_replay_commit_audit;
    let mut readiness_rows = Vec::new();
    let mut eligible_for_closeout_preflight_count = 0_usize;
    let mut blocked_missing_runtime_record_count = 0_usize;
    let mut blocked_metadata_mismatch_count = 0_usize;
    let mut blocked_commit_failed_count = 0_usize;
    let mut blocked_idempotency_conflict_count = 0_usize;
    let mut blocked_no_commit_work_count = 0_usize;
    let mut blocked_not_auditable_count = 0_usize;
    let mut blocked_unknown_audit_status_count = 0_usize;

    for row in &audit.audit_rows {
        let readiness_row = readiness_row(row)?;
        match readiness_row.readiness_status.as_str() {
            "eligible_for_closeout_preflight" => eligible_for_closeout_preflight_count += 1,
            "blocked_missing_runtime_record" => blocked_missing_runtime_record_count += 1,
            "blocked_metadata_mismatch" => blocked_metadata_mismatch_count += 1,
            "blocked_commit_failed" => blocked_commit_failed_count += 1,
            "blocked_idempotency_conflict" => blocked_idempotency_conflict_count += 1,
            "blocked_no_commit_work" => blocked_no_commit_work_count += 1,
            "blocked_not_auditable" => blocked_not_auditable_count += 1,
            "blocked_unknown_audit_status" => blocked_unknown_audit_status_count += 1,
            _ => {}
        }
        readiness_rows.push(readiness_row);
    }

    let blocked_count = blocked_missing_runtime_record_count
        + blocked_metadata_mismatch_count
        + blocked_commit_failed_count
        + blocked_idempotency_conflict_count
        + blocked_no_commit_work_count
        + blocked_not_auditable_count
        + blocked_unknown_audit_status_count;
    let readiness_status = resolve_readiness_status(
        readiness_rows.len(),
        eligible_for_closeout_preflight_count,
        blocked_count,
    );

    Ok(SecurityPortfolioExecutionLifecycleCloseoutReadinessResult {
        portfolio_execution_lifecycle_closeout_readiness:
            SecurityPortfolioExecutionLifecycleCloseoutReadinessDocument {
                portfolio_execution_lifecycle_closeout_readiness_id: format!(
                    "portfolio-execution-lifecycle-closeout-readiness:{}:{}",
                    audit.account_id, generated_at
                ),
                contract_version: P20A_CONTRACT_VERSION.to_string(),
                document_type: P20A_DOCUMENT_TYPE.to_string(),
                generated_at,
                analysis_date: audit.analysis_date.clone(),
                account_id: audit.account_id.clone(),
                source_p19e_ref: audit
                    .portfolio_execution_replay_commit_audit_id
                    .clone(),
                source_p19d_ref: audit.source_p19d_ref.clone(),
                source_p19c_ref: audit.source_p19c_ref.clone(),
                source_non_atomicity_notice: audit.source_non_atomicity_notice.clone(),
                readiness_row_count: readiness_rows.len(),
                eligible_for_closeout_preflight_count,
                blocked_missing_runtime_record_count,
                blocked_metadata_mismatch_count,
                blocked_commit_failed_count,
                blocked_idempotency_conflict_count,
                blocked_no_commit_work_count,
                blocked_not_auditable_count,
                blocked_unknown_audit_status_count,
                runtime_write_count: 0,
                readiness_rows,
                readiness_status: readiness_status.clone(),
                blockers: build_blockers(blocked_count),
                readiness_rationale: vec![
                    "P20A consumed P19E replay commit audit truth".to_string(),
                    "P20A maps only verified replay metadata rows to closeout preflight eligibility".to_string(),
                    "P20A produces readiness only; it is not lifecycle closure, broker-fill replay, or position materialization".to_string(),
                ],
                readiness_summary: format!(
                    "account {} P20A readiness status {} with {} eligible rows and {} blocked rows; this is not lifecycle closure",
                    audit.account_id,
                    readiness_status,
                    eligible_for_closeout_preflight_count,
                    blocked_count
                ),
            },
    })
}

fn validate_audit_document(
    audit: &SecurityPortfolioExecutionReplayCommitAuditDocument,
) -> Result<(), SecurityPortfolioExecutionLifecycleCloseoutReadinessError> {
    if audit.document_type != "security_portfolio_execution_replay_commit_audit" {
        return Err(
            SecurityPortfolioExecutionLifecycleCloseoutReadinessError::UnsupportedAuditDocumentType(
                audit.document_type.clone(),
            ),
        );
    }
    if audit.contract_version != "security_portfolio_execution_replay_commit_audit.v1" {
        return Err(
            SecurityPortfolioExecutionLifecycleCloseoutReadinessError::UnsupportedAuditContractVersion(
                audit.contract_version.clone(),
            ),
        );
    }
    if audit.runtime_write_count != 0 {
        return Err(
            SecurityPortfolioExecutionLifecycleCloseoutReadinessError::AuditRuntimeWriteCount(
                audit.runtime_write_count,
            ),
        );
    }
    if audit.source_p19d_ref.trim().is_empty() {
        return Err(
            SecurityPortfolioExecutionLifecycleCloseoutReadinessError::MissingSourceP19DRef,
        );
    }
    if audit.source_p19c_ref.trim().is_empty() {
        return Err(
            SecurityPortfolioExecutionLifecycleCloseoutReadinessError::MissingSourceP19CRef,
        );
    }
    if audit.source_non_atomicity_notice.trim().is_empty() {
        return Err(
            SecurityPortfolioExecutionLifecycleCloseoutReadinessError::MissingSourceNonAtomicityNotice,
        );
    }

    Ok(())
}

fn readiness_row(
    row: &SecurityPortfolioExecutionReplayCommitAuditRow,
) -> Result<
    SecurityPortfolioExecutionLifecycleCloseoutReadinessRow,
    SecurityPortfolioExecutionLifecycleCloseoutReadinessError,
> {
    let (readiness_status, closeout_preflight_eligible, blockers) = match row.audit_status.as_str()
    {
        "verified" | "already_committed_verified" => {
            validate_eligible_row(row)?;
            (
                "eligible_for_closeout_preflight",
                true,
                Vec::<String>::new(),
            )
        }
        "missing_runtime_record" => (
            "blocked_missing_runtime_record",
            false,
            vec!["P19E did not find the target runtime record".to_string()],
        ),
        "metadata_mismatch" => (
            "blocked_metadata_mismatch",
            false,
            vec!["P19E found replay metadata drift".to_string()],
        ),
        "commit_failed_preserved" => (
            "blocked_commit_failed",
            false,
            vec!["P19D commit failure is preserved".to_string()],
        ),
        "idempotency_conflict_confirmed" => (
            "blocked_idempotency_conflict",
            false,
            vec!["P19D idempotency conflict is preserved".to_string()],
        ),
        "skipped_no_commit_work_preserved" => (
            "blocked_no_commit_work",
            false,
            vec!["P19E row had no commit work".to_string()],
        ),
        "not_auditable" => (
            "blocked_not_auditable",
            false,
            vec!["P19E row is not auditable".to_string()],
        ),
        other => (
            "blocked_unknown_audit_status",
            false,
            vec![format!("unknown P19E audit status `{other}`")],
        ),
    };

    Ok(SecurityPortfolioExecutionLifecycleCloseoutReadinessRow {
        symbol: row.symbol.clone(),
        source_p19e_audit_status: row.audit_status.clone(),
        readiness_status: readiness_status.to_string(),
        commit_idempotency_key: row.commit_idempotency_key.clone(),
        canonical_commit_payload_hash: row.canonical_commit_payload_hash.clone(),
        source_p19c_ref: row.source_p19c_ref.clone(),
        target_execution_record_ref: row.target_execution_record_ref.clone(),
        runtime_execution_record_ref: row.runtime_execution_record_ref.clone(),
        runtime_replay_commit_idempotency_key: row.runtime_replay_commit_idempotency_key.clone(),
        runtime_replay_commit_payload_hash: row.runtime_replay_commit_payload_hash.clone(),
        runtime_replay_commit_source_p19c_ref: row.runtime_replay_commit_source_p19c_ref.clone(),
        closeout_preflight_eligible,
        blockers,
    })
}

fn validate_eligible_row(
    row: &SecurityPortfolioExecutionReplayCommitAuditRow,
) -> Result<(), SecurityPortfolioExecutionLifecycleCloseoutReadinessError> {
    let has_required_evidence = !row.target_execution_record_ref.trim().is_empty()
        && !row.commit_idempotency_key.trim().is_empty()
        && !row.canonical_commit_payload_hash.trim().is_empty()
        && !row.source_p19c_ref.trim().is_empty()
        && row
            .runtime_replay_commit_idempotency_key
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && row
            .runtime_replay_commit_payload_hash
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && row
            .runtime_replay_commit_source_p19c_ref
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());

    if has_required_evidence {
        Ok(())
    } else {
        Err(
            SecurityPortfolioExecutionLifecycleCloseoutReadinessError::MissingEligibleRowEvidence(
                row.symbol.clone(),
            ),
        )
    }
}

fn resolve_readiness_status(
    row_count: usize,
    eligible_count: usize,
    blocked_count: usize,
) -> String {
    if row_count == 0 {
        return "no_closeout_candidates".to_string();
    }
    if eligible_count > 0 && blocked_count == 0 {
        return "closeout_preflight_ready".to_string();
    }
    if eligible_count > 0 && blocked_count > 0 {
        return "partial_closeout_preflight_ready".to_string();
    }
    "blocked".to_string()
}

fn build_blockers(blocked_count: usize) -> Vec<String> {
    if blocked_count == 0 {
        Vec::new()
    } else {
        vec![format!("blocked_readiness_row_count={blocked_count}")]
    }
}

fn normalize_created_at(created_at: &str) -> String {
    let trimmed = created_at.trim();
    if trimmed.is_empty() {
        default_created_at()
    } else {
        trimmed.to_string()
    }
}

fn default_created_at() -> String {
    Utc::now().to_rfc3339()
}
