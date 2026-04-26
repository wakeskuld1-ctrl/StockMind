use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_portfolio_execution_replay_commit_writer::{
    SecurityPortfolioExecutionReplayCommitWriterDocument,
    SecurityPortfolioExecutionReplayCommitWriterRow,
};
use crate::runtime::security_execution_store::{
    SecurityExecutionStore, SecurityExecutionStoreError,
};

const P19E_DOCUMENT_TYPE: &str = "security_portfolio_execution_replay_commit_audit";
const P19E_CONTRACT_VERSION: &str = "security_portfolio_execution_replay_commit_audit.v1";

// 2026-04-26 CST: Added because P19D runtime replay commits need an independent read-only verifier.
// Purpose: keep audit evidence separate from commit authority and later lifecycle work.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityPortfolioExecutionReplayCommitAuditRequest {
    pub portfolio_execution_replay_commit_writer:
        SecurityPortfolioExecutionReplayCommitWriterDocument,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionReplayCommitAuditRow {
    pub symbol: String,
    pub source_p19d_row_status: String,
    pub audit_status: String,
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
    pub runtime_record_present: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionReplayCommitAuditDocument {
    pub portfolio_execution_replay_commit_audit_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub analysis_date: String,
    pub account_id: String,
    pub source_p19d_ref: String,
    pub source_p19c_ref: String,
    pub source_non_atomicity_notice: String,
    pub audit_rows: Vec<SecurityPortfolioExecutionReplayCommitAuditRow>,
    pub audit_row_count: usize,
    pub verified_count: usize,
    pub already_committed_verified_count: usize,
    pub missing_runtime_record_count: usize,
    pub metadata_mismatch_count: usize,
    pub idempotency_conflict_confirmed_count: usize,
    pub commit_failed_preserved_count: usize,
    pub not_auditable_count: usize,
    pub runtime_write_count: usize,
    pub audit_status: String,
    pub blockers: Vec<String>,
    pub audit_rationale: Vec<String>,
    pub audit_summary: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionReplayCommitAuditResult {
    pub portfolio_execution_replay_commit_audit:
        SecurityPortfolioExecutionReplayCommitAuditDocument,
}

#[derive(Debug, Error)]
pub enum SecurityPortfolioExecutionReplayCommitAuditError {
    #[error(
        "security portfolio execution replay commit audit build failed: unsupported P19D document type `{0}`"
    )]
    UnsupportedWriterDocumentType(String),
    #[error(
        "security portfolio execution replay commit audit build failed: unsupported P19D contract version `{0}`"
    )]
    UnsupportedWriterContractVersion(String),
    #[error(
        "security portfolio execution replay commit audit build failed: P19D non_atomicity_notice is required"
    )]
    MissingNonAtomicityNotice,
    #[error(
        "security portfolio execution replay commit audit build failed: committed row `{0}` is missing target execution record ref"
    )]
    MissingTargetExecutionRecordRef(String),
    #[error(
        "security portfolio execution replay commit audit build failed: committed row `{0}` is missing replay metadata"
    )]
    MissingReplayMetadata(String),
    #[error("security portfolio execution replay commit audit runtime read failed: {0}")]
    Store(#[from] SecurityExecutionStoreError),
}

pub fn security_portfolio_execution_replay_commit_audit(
    request: &SecurityPortfolioExecutionReplayCommitAuditRequest,
) -> Result<
    SecurityPortfolioExecutionReplayCommitAuditResult,
    SecurityPortfolioExecutionReplayCommitAuditError,
> {
    build_security_portfolio_execution_replay_commit_audit(request)
}

pub fn build_security_portfolio_execution_replay_commit_audit(
    request: &SecurityPortfolioExecutionReplayCommitAuditRequest,
) -> Result<
    SecurityPortfolioExecutionReplayCommitAuditResult,
    SecurityPortfolioExecutionReplayCommitAuditError,
> {
    validate_writer_document(&request.portfolio_execution_replay_commit_writer)?;

    let generated_at = normalize_created_at(&request.created_at);
    let writer = &request.portfolio_execution_replay_commit_writer;
    let store = SecurityExecutionStore::workspace_default()?;
    let mut audit_rows = Vec::new();
    let mut verified_count = 0_usize;
    let mut already_committed_verified_count = 0_usize;
    let mut missing_runtime_record_count = 0_usize;
    let mut metadata_mismatch_count = 0_usize;
    let mut idempotency_conflict_confirmed_count = 0_usize;
    let mut commit_failed_preserved_count = 0_usize;
    let mut not_auditable_count = 0_usize;

    for row in &writer.commit_rows {
        let audit_row = audit_row(row, &writer.source_p19c_ref, &store)?;
        match audit_row.audit_status.as_str() {
            "verified" => verified_count += 1,
            "already_committed_verified" => already_committed_verified_count += 1,
            "missing_runtime_record" => missing_runtime_record_count += 1,
            "metadata_mismatch" => metadata_mismatch_count += 1,
            "idempotency_conflict_confirmed" => idempotency_conflict_confirmed_count += 1,
            "commit_failed_preserved" => commit_failed_preserved_count += 1,
            "not_auditable" => not_auditable_count += 1,
            _ => {}
        }
        audit_rows.push(audit_row);
    }

    let audit_status = resolve_audit_status(
        audit_rows.len(),
        missing_runtime_record_count,
        metadata_mismatch_count,
        not_auditable_count,
        idempotency_conflict_confirmed_count,
        commit_failed_preserved_count,
    );

    Ok(SecurityPortfolioExecutionReplayCommitAuditResult {
        portfolio_execution_replay_commit_audit:
            SecurityPortfolioExecutionReplayCommitAuditDocument {
                portfolio_execution_replay_commit_audit_id: format!(
                    "portfolio-execution-replay-commit-audit:{}:{}",
                    writer.account_id, generated_at
                ),
                contract_version: P19E_CONTRACT_VERSION.to_string(),
                document_type: P19E_DOCUMENT_TYPE.to_string(),
                generated_at,
                analysis_date: writer.analysis_date.clone(),
                account_id: writer.account_id.clone(),
                source_p19d_ref: writer
                    .portfolio_execution_replay_commit_writer_id
                    .clone(),
                source_p19c_ref: writer.source_p19c_ref.clone(),
                source_non_atomicity_notice: writer.non_atomicity_notice.clone(),
                audit_row_count: audit_rows.len(),
                verified_count,
                already_committed_verified_count,
                missing_runtime_record_count,
                metadata_mismatch_count,
                idempotency_conflict_confirmed_count,
                commit_failed_preserved_count,
                not_auditable_count,
                runtime_write_count: 0,
                audit_rows,
                audit_status: audit_status.clone(),
                blockers: build_blockers(
                    missing_runtime_record_count,
                    metadata_mismatch_count,
                    not_auditable_count,
                ),
                audit_rationale: vec![
                    "P19E consumed P19D replay commit output".to_string(),
                    "P19E read runtime execution records and compared machine-readable replay metadata".to_string(),
                    "P19E performs no runtime writes and does not treat replay commits as broker fills or closed position truth".to_string(),
                ],
                audit_summary: format!(
                    "account {} P19E audit status {} with {} verified, {} already committed verified, {} missing runtime records, {} metadata mismatches",
                    writer.account_id,
                    audit_status,
                    verified_count,
                    already_committed_verified_count,
                    missing_runtime_record_count,
                    metadata_mismatch_count
                ),
            },
    })
}

fn validate_writer_document(
    writer: &SecurityPortfolioExecutionReplayCommitWriterDocument,
) -> Result<(), SecurityPortfolioExecutionReplayCommitAuditError> {
    if writer.document_type != "security_portfolio_execution_replay_commit_writer" {
        return Err(
            SecurityPortfolioExecutionReplayCommitAuditError::UnsupportedWriterDocumentType(
                writer.document_type.clone(),
            ),
        );
    }
    if writer.contract_version != "security_portfolio_execution_replay_commit_writer.v1" {
        return Err(
            SecurityPortfolioExecutionReplayCommitAuditError::UnsupportedWriterContractVersion(
                writer.contract_version.clone(),
            ),
        );
    }
    if writer.non_atomicity_notice.trim().is_empty() {
        return Err(SecurityPortfolioExecutionReplayCommitAuditError::MissingNonAtomicityNotice);
    }

    for row in &writer.commit_rows {
        if matches!(row.row_status.as_str(), "committed" | "already_committed") {
            if row.target_execution_record_ref.trim().is_empty() {
                return Err(
                    SecurityPortfolioExecutionReplayCommitAuditError::MissingTargetExecutionRecordRef(
                        row.symbol.clone(),
                    ),
                );
            }
            if row.commit_idempotency_key.trim().is_empty()
                || row.canonical_commit_payload_hash.trim().is_empty()
            {
                return Err(
                    SecurityPortfolioExecutionReplayCommitAuditError::MissingReplayMetadata(
                        row.symbol.clone(),
                    ),
                );
            }
        }
    }

    Ok(())
}

fn audit_row(
    row: &SecurityPortfolioExecutionReplayCommitWriterRow,
    source_p19c_ref: &str,
    store: &SecurityExecutionStore,
) -> Result<
    SecurityPortfolioExecutionReplayCommitAuditRow,
    SecurityPortfolioExecutionReplayCommitAuditError,
> {
    match row.row_status.as_str() {
        "committed" | "already_committed" => audit_runtime_row(row, source_p19c_ref, store),
        "commit_failed" => Ok(static_audit_row(
            row,
            source_p19c_ref,
            "commit_failed_preserved",
            false,
            Vec::new(),
        )),
        "idempotency_conflict" => Ok(static_audit_row(
            row,
            source_p19c_ref,
            "idempotency_conflict_confirmed",
            false,
            Vec::new(),
        )),
        "skipped_no_commit_work" => Ok(static_audit_row(
            row,
            source_p19c_ref,
            "skipped_no_commit_work_preserved",
            false,
            Vec::new(),
        )),
        _ => Ok(static_audit_row(
            row,
            source_p19c_ref,
            "not_auditable",
            false,
            vec![format!("unsupported P19D row status `{}`", row.row_status)],
        )),
    }
}

fn audit_runtime_row(
    row: &SecurityPortfolioExecutionReplayCommitWriterRow,
    source_p19c_ref: &str,
    store: &SecurityExecutionStore,
) -> Result<
    SecurityPortfolioExecutionReplayCommitAuditRow,
    SecurityPortfolioExecutionReplayCommitAuditError,
> {
    // 2026-04-26 CST: Added because P19E must verify runtime truth without opening write paths.
    // Purpose: use only machine-readable replay fields and preserve mismatches as audit facts.
    let Some(runtime_record) = store.load_execution_record(&row.target_execution_record_ref)?
    else {
        return Ok(static_audit_row(
            row,
            source_p19c_ref,
            "missing_runtime_record",
            false,
            vec![format!(
                "runtime execution record `{}` was not found",
                row.target_execution_record_ref
            )],
        ));
    };

    let idempotency_matches = runtime_record.replay_commit_idempotency_key.as_deref()
        == Some(row.commit_idempotency_key.as_str());
    let payload_matches = runtime_record.replay_commit_payload_hash.as_deref()
        == Some(row.canonical_commit_payload_hash.as_str());
    let source_matches =
        runtime_record.replay_commit_source_p19c_ref.as_deref() == Some(source_p19c_ref);
    let audit_status = if idempotency_matches && payload_matches && source_matches {
        if row.row_status == "already_committed" {
            "already_committed_verified"
        } else {
            "verified"
        }
    } else {
        "metadata_mismatch"
    };

    let mut blockers = Vec::new();
    if !idempotency_matches {
        blockers.push("runtime replay idempotency key mismatch".to_string());
    }
    if !payload_matches {
        blockers.push("runtime replay payload hash mismatch".to_string());
    }
    if !source_matches {
        blockers.push("runtime replay source P19C ref mismatch".to_string());
    }

    Ok(SecurityPortfolioExecutionReplayCommitAuditRow {
        symbol: row.symbol.clone(),
        source_p19d_row_status: row.row_status.clone(),
        audit_status: audit_status.to_string(),
        commit_idempotency_key: row.commit_idempotency_key.clone(),
        canonical_commit_payload_hash: row.canonical_commit_payload_hash.clone(),
        source_p19c_ref: source_p19c_ref.to_string(),
        target_execution_record_ref: row.target_execution_record_ref.clone(),
        runtime_execution_record_ref: Some(runtime_record.execution_record_id),
        runtime_replay_commit_idempotency_key: runtime_record.replay_commit_idempotency_key,
        runtime_replay_commit_payload_hash: runtime_record.replay_commit_payload_hash,
        runtime_replay_commit_source_p19c_ref: runtime_record.replay_commit_source_p19c_ref,
        runtime_record_present: true,
        blockers,
    })
}

fn static_audit_row(
    row: &SecurityPortfolioExecutionReplayCommitWriterRow,
    source_p19c_ref: &str,
    audit_status: &str,
    runtime_record_present: bool,
    blockers: Vec<String>,
) -> SecurityPortfolioExecutionReplayCommitAuditRow {
    SecurityPortfolioExecutionReplayCommitAuditRow {
        symbol: row.symbol.clone(),
        source_p19d_row_status: row.row_status.clone(),
        audit_status: audit_status.to_string(),
        commit_idempotency_key: row.commit_idempotency_key.clone(),
        canonical_commit_payload_hash: row.canonical_commit_payload_hash.clone(),
        source_p19c_ref: source_p19c_ref.to_string(),
        target_execution_record_ref: row.target_execution_record_ref.clone(),
        runtime_execution_record_ref: row.runtime_execution_record_ref.clone(),
        runtime_replay_commit_idempotency_key: None,
        runtime_replay_commit_payload_hash: None,
        runtime_replay_commit_source_p19c_ref: None,
        runtime_record_present,
        blockers,
    }
}

fn resolve_audit_status(
    row_count: usize,
    missing_runtime_record_count: usize,
    metadata_mismatch_count: usize,
    not_auditable_count: usize,
    idempotency_conflict_confirmed_count: usize,
    commit_failed_preserved_count: usize,
) -> String {
    if row_count == 0 {
        return "no_commit_work".to_string();
    }
    if missing_runtime_record_count > 0 || metadata_mismatch_count > 0 || not_auditable_count > 0 {
        return "partial_audit_failure".to_string();
    }
    if idempotency_conflict_confirmed_count > 0 || commit_failed_preserved_count > 0 {
        return "verified_with_preserved_failures".to_string();
    }
    "verified".to_string()
}

fn build_blockers(
    missing_runtime_record_count: usize,
    metadata_mismatch_count: usize,
    not_auditable_count: usize,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if missing_runtime_record_count > 0 {
        blockers.push(format!(
            "missing_runtime_record_count={missing_runtime_record_count}"
        ));
    }
    if metadata_mismatch_count > 0 {
        blockers.push(format!("metadata_mismatch_count={metadata_mismatch_count}"));
    }
    if not_auditable_count > 0 {
        blockers.push(format!("not_auditable_count={not_auditable_count}"));
    }
    blockers
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
