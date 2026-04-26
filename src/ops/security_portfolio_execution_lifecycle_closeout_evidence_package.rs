use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_portfolio_execution_lifecycle_closeout_readiness::{
    SecurityPortfolioExecutionLifecycleCloseoutReadinessDocument,
    SecurityPortfolioExecutionLifecycleCloseoutReadinessRow,
};
use crate::runtime::security_execution_store::{
    SecurityExecutionStore, SecurityExecutionStoreError,
};

const P20B_DOCUMENT_TYPE: &str = "security_portfolio_execution_lifecycle_closeout_evidence_package";
const P20B_CONTRACT_VERSION: &str =
    "security_portfolio_execution_lifecycle_closeout_evidence_package.v1";

// 2026-04-26 CST: Added because P20A readiness still needs closed runtime evidence.
// Purpose: prove closeout evidence with point reads without creating archive or runtime writes.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageRequest {
    pub portfolio_execution_lifecycle_closeout_readiness:
        SecurityPortfolioExecutionLifecycleCloseoutReadinessDocument,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageRow {
    pub symbol: String,
    pub source_p20a_readiness_status: String,
    pub evidence_status: String,
    pub commit_idempotency_key: String,
    pub canonical_commit_payload_hash: String,
    pub source_p19c_ref: String,
    pub target_execution_record_ref: String,
    #[serde(default)]
    pub runtime_execution_record_ref: Option<String>,
    #[serde(default)]
    pub runtime_position_state: Option<String>,
    #[serde(default)]
    pub runtime_actual_exit_date: Option<String>,
    #[serde(default)]
    pub runtime_actual_exit_price: Option<f64>,
    #[serde(default)]
    pub runtime_exit_reason: Option<String>,
    #[serde(default)]
    pub runtime_replay_commit_idempotency_key: Option<String>,
    #[serde(default)]
    pub runtime_replay_commit_payload_hash: Option<String>,
    #[serde(default)]
    pub runtime_replay_commit_source_p19c_ref: Option<String>,
    pub runtime_record_present: bool,
    pub closeout_evidence_ready: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageDocument {
    pub portfolio_execution_lifecycle_closeout_evidence_package_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub analysis_date: String,
    pub account_id: String,
    pub source_p20a_ref: String,
    pub source_p19e_ref: String,
    pub source_p19d_ref: String,
    pub source_p19c_ref: String,
    pub source_non_atomicity_notice: String,
    pub evidence_rows: Vec<SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageRow>,
    pub evidence_row_count: usize,
    pub evidence_ready_for_closeout_archive_preflight_count: usize,
    pub blocked_p20a_not_eligible_count: usize,
    pub blocked_missing_runtime_record_count: usize,
    pub blocked_runtime_record_identity_mismatch_count: usize,
    pub blocked_runtime_record_not_closed_count: usize,
    pub blocked_missing_exit_evidence_count: usize,
    pub blocked_replay_metadata_mismatch_count: usize,
    pub blocked_account_or_symbol_mismatch_count: usize,
    pub blocked_unknown_p20a_readiness_status_count: usize,
    pub runtime_read_count: usize,
    pub runtime_write_count: usize,
    pub evidence_status: String,
    pub blockers: Vec<String>,
    pub evidence_rationale: Vec<String>,
    pub evidence_summary: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageResult {
    pub portfolio_execution_lifecycle_closeout_evidence_package:
        SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageDocument,
}

#[derive(Debug, Error)]
pub enum SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageError {
    #[error(
        "security portfolio execution lifecycle closeout evidence package build failed: unsupported P20A document type `{0}`"
    )]
    UnsupportedReadinessDocumentType(String),
    #[error(
        "security portfolio execution lifecycle closeout evidence package build failed: unsupported P20A contract version `{0}`"
    )]
    UnsupportedReadinessContractVersion(String),
    #[error(
        "security portfolio execution lifecycle closeout evidence package build failed: P20A runtime write count must be zero, observed `{0}`"
    )]
    ReadinessRuntimeWriteCount(usize),
    #[error(
        "security portfolio execution lifecycle closeout evidence package build failed: source P19E ref is required"
    )]
    MissingSourceP19ERef,
    #[error(
        "security portfolio execution lifecycle closeout evidence package build failed: source P19D ref is required"
    )]
    MissingSourceP19DRef,
    #[error(
        "security portfolio execution lifecycle closeout evidence package build failed: source P19C ref is required"
    )]
    MissingSourceP19CRef,
    #[error(
        "security portfolio execution lifecycle closeout evidence package build failed: source non-atomicity notice is required"
    )]
    MissingSourceNonAtomicityNotice,
    #[error(
        "security portfolio execution lifecycle closeout evidence package build failed: eligible row `{0}` is missing machine-readable readiness evidence"
    )]
    MissingEligibleRowEvidence(String),
    #[error(
        "security portfolio execution lifecycle closeout evidence package runtime read failed: {0}"
    )]
    Store(#[from] SecurityExecutionStoreError),
}

pub fn security_portfolio_execution_lifecycle_closeout_evidence_package(
    request: &SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageRequest,
) -> Result<
    SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageResult,
    SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageError,
> {
    build_security_portfolio_execution_lifecycle_closeout_evidence_package(request)
}

pub fn build_security_portfolio_execution_lifecycle_closeout_evidence_package(
    request: &SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageRequest,
) -> Result<
    SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageResult,
    SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageError,
> {
    let readiness = &request.portfolio_execution_lifecycle_closeout_readiness;
    validate_readiness_document(readiness)?;

    let store = SecurityExecutionStore::workspace_default()?;
    let generated_at = normalize_created_at(&request.created_at);
    let mut evidence_rows = Vec::new();
    let mut evidence_ready_for_closeout_archive_preflight_count = 0_usize;
    let mut blocked_p20a_not_eligible_count = 0_usize;
    let mut blocked_missing_runtime_record_count = 0_usize;
    let mut blocked_runtime_record_identity_mismatch_count = 0_usize;
    let mut blocked_runtime_record_not_closed_count = 0_usize;
    let mut blocked_missing_exit_evidence_count = 0_usize;
    let mut blocked_replay_metadata_mismatch_count = 0_usize;
    let mut blocked_account_or_symbol_mismatch_count = 0_usize;
    let mut blocked_unknown_p20a_readiness_status_count = 0_usize;
    let mut runtime_read_count = 0_usize;

    for row in &readiness.readiness_rows {
        let (evidence_row, did_read_runtime) = evidence_row(row, readiness, &store)?;
        if did_read_runtime {
            runtime_read_count += 1;
        }
        match evidence_row.evidence_status.as_str() {
            "evidence_ready_for_closeout_archive_preflight" => {
                evidence_ready_for_closeout_archive_preflight_count += 1
            }
            "blocked_p20a_not_eligible" => blocked_p20a_not_eligible_count += 1,
            "blocked_missing_runtime_record" => blocked_missing_runtime_record_count += 1,
            "blocked_runtime_record_identity_mismatch" => {
                blocked_runtime_record_identity_mismatch_count += 1
            }
            "blocked_runtime_record_not_closed" => blocked_runtime_record_not_closed_count += 1,
            "blocked_missing_exit_evidence" => blocked_missing_exit_evidence_count += 1,
            "blocked_replay_metadata_mismatch" => blocked_replay_metadata_mismatch_count += 1,
            "blocked_account_or_symbol_mismatch" => blocked_account_or_symbol_mismatch_count += 1,
            "blocked_unknown_p20a_readiness_status" => {
                blocked_unknown_p20a_readiness_status_count += 1
            }
            _ => {}
        }
        evidence_rows.push(evidence_row);
    }

    let blocked_count = blocked_p20a_not_eligible_count
        + blocked_missing_runtime_record_count
        + blocked_runtime_record_identity_mismatch_count
        + blocked_runtime_record_not_closed_count
        + blocked_missing_exit_evidence_count
        + blocked_replay_metadata_mismatch_count
        + blocked_account_or_symbol_mismatch_count
        + blocked_unknown_p20a_readiness_status_count;
    let evidence_status = resolve_evidence_status(
        evidence_rows.len(),
        evidence_ready_for_closeout_archive_preflight_count,
        blocked_count,
    );

    Ok(
        SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageResult {
            portfolio_execution_lifecycle_closeout_evidence_package:
                SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageDocument {
                    portfolio_execution_lifecycle_closeout_evidence_package_id: format!(
                        "portfolio-execution-lifecycle-closeout-evidence-package:{}:{}",
                        readiness.account_id, generated_at
                    ),
                    contract_version: P20B_CONTRACT_VERSION.to_string(),
                    document_type: P20B_DOCUMENT_TYPE.to_string(),
                    generated_at,
                    analysis_date: readiness.analysis_date.clone(),
                    account_id: readiness.account_id.clone(),
                    source_p20a_ref: readiness
                        .portfolio_execution_lifecycle_closeout_readiness_id
                        .clone(),
                    source_p19e_ref: readiness.source_p19e_ref.clone(),
                    source_p19d_ref: readiness.source_p19d_ref.clone(),
                    source_p19c_ref: readiness.source_p19c_ref.clone(),
                    source_non_atomicity_notice: readiness.source_non_atomicity_notice.clone(),
                    evidence_row_count: evidence_rows.len(),
                    evidence_ready_for_closeout_archive_preflight_count,
                    blocked_p20a_not_eligible_count,
                    blocked_missing_runtime_record_count,
                    blocked_runtime_record_identity_mismatch_count,
                    blocked_runtime_record_not_closed_count,
                    blocked_missing_exit_evidence_count,
                    blocked_replay_metadata_mismatch_count,
                    blocked_account_or_symbol_mismatch_count,
                    blocked_unknown_p20a_readiness_status_count,
                    runtime_read_count,
                    runtime_write_count: 0,
                    evidence_rows,
                    evidence_status: evidence_status.clone(),
                    blockers: build_blockers(blocked_count),
                    evidence_rationale: vec![
                        "P20B consumed P20A closeout readiness truth".to_string(),
                        "P20B point-read runtime execution records for eligible rows only".to_string(),
                        "P20B produces evidence readiness only; it is not lifecycle closure, archive production, broker-fill replay, or position materialization".to_string(),
                    ],
                    evidence_summary: format!(
                        "account {} P20B evidence status {} with {} evidence-ready rows and {} blocked rows; this is not lifecycle closure or archive production",
                        readiness.account_id,
                        evidence_status,
                        evidence_ready_for_closeout_archive_preflight_count,
                        blocked_count
                    ),
                },
        },
    )
}

fn validate_readiness_document(
    readiness: &SecurityPortfolioExecutionLifecycleCloseoutReadinessDocument,
) -> Result<(), SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageError> {
    if readiness.document_type != "security_portfolio_execution_lifecycle_closeout_readiness" {
        return Err(
            SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageError::UnsupportedReadinessDocumentType(
                readiness.document_type.clone(),
            ),
        );
    }
    if readiness.contract_version != "security_portfolio_execution_lifecycle_closeout_readiness.v1"
    {
        return Err(
            SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageError::UnsupportedReadinessContractVersion(
                readiness.contract_version.clone(),
            ),
        );
    }
    if readiness.runtime_write_count != 0 {
        return Err(
            SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageError::ReadinessRuntimeWriteCount(
                readiness.runtime_write_count,
            ),
        );
    }
    if readiness.source_p19e_ref.trim().is_empty() {
        return Err(
            SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageError::MissingSourceP19ERef,
        );
    }
    if readiness.source_p19d_ref.trim().is_empty() {
        return Err(
            SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageError::MissingSourceP19DRef,
        );
    }
    if readiness.source_p19c_ref.trim().is_empty() {
        return Err(
            SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageError::MissingSourceP19CRef,
        );
    }
    if readiness.source_non_atomicity_notice.trim().is_empty() {
        return Err(
            SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageError::MissingSourceNonAtomicityNotice,
        );
    }
    for row in &readiness.readiness_rows {
        if row.readiness_status == "eligible_for_closeout_preflight" {
            validate_eligible_row(row)?;
        }
    }
    Ok(())
}

fn evidence_row(
    row: &SecurityPortfolioExecutionLifecycleCloseoutReadinessRow,
    readiness: &SecurityPortfolioExecutionLifecycleCloseoutReadinessDocument,
    store: &SecurityExecutionStore,
) -> Result<
    (
        SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageRow,
        bool,
    ),
    SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageError,
> {
    if row.readiness_status != "eligible_for_closeout_preflight" {
        let status = if row.readiness_status.starts_with("blocked_") {
            "blocked_p20a_not_eligible"
        } else {
            "blocked_unknown_p20a_readiness_status"
        };
        return Ok((
            static_evidence_row(
                row,
                status,
                false,
                false,
                vec![format!(
                    "P20A readiness status `{}` is not eligible",
                    row.readiness_status
                )],
            ),
            false,
        ));
    }

    // 2026-04-26 CST: Added because P20B needs closed runtime proof after P20A readiness.
    // Purpose: point-read one execution record and preserve blockers without opening write authority.
    let Some(runtime_record) = store.load_execution_record(&row.target_execution_record_ref)?
    else {
        return Ok((
            static_evidence_row(
                row,
                "blocked_missing_runtime_record",
                true,
                false,
                vec![format!(
                    "runtime execution record `{}` was not found",
                    row.target_execution_record_ref
                )],
            ),
            true,
        ));
    };

    let mut blockers = Vec::new();
    let evidence_status;
    let closeout_evidence_ready;

    if runtime_record.execution_record_id != row.target_execution_record_ref {
        evidence_status = "blocked_runtime_record_identity_mismatch";
        closeout_evidence_ready = false;
        blockers.push("runtime execution record identity mismatch".to_string());
    } else if runtime_record.account_id.as_deref() != Some(readiness.account_id.as_str())
        || runtime_record.symbol != row.symbol
    {
        evidence_status = "blocked_account_or_symbol_mismatch";
        closeout_evidence_ready = false;
        blockers.push("runtime account or symbol mismatch".to_string());
    } else if runtime_record.replay_commit_idempotency_key.as_deref()
        != Some(row.commit_idempotency_key.as_str())
        || runtime_record.replay_commit_payload_hash.as_deref()
            != Some(row.canonical_commit_payload_hash.as_str())
        || runtime_record.replay_commit_source_p19c_ref.as_deref()
            != Some(row.source_p19c_ref.as_str())
    {
        evidence_status = "blocked_replay_metadata_mismatch";
        closeout_evidence_ready = false;
        blockers.push("runtime replay metadata mismatch".to_string());
    } else if runtime_record.position_state != "closed" {
        evidence_status = "blocked_runtime_record_not_closed";
        closeout_evidence_ready = false;
        blockers.push(format!(
            "runtime position state `{}` is not closed",
            runtime_record.position_state
        ));
    } else if runtime_record.actual_exit_date.trim().is_empty()
        || runtime_record.actual_exit_price <= 0.0
        || runtime_record.exit_reason.trim().is_empty()
        || runtime_record.exit_reason == "position_still_open"
    {
        evidence_status = "blocked_missing_exit_evidence";
        closeout_evidence_ready = false;
        blockers.push("runtime exit evidence is incomplete".to_string());
    } else {
        evidence_status = "evidence_ready_for_closeout_archive_preflight";
        closeout_evidence_ready = true;
    }

    Ok((
        SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageRow {
            symbol: row.symbol.clone(),
            source_p20a_readiness_status: row.readiness_status.clone(),
            evidence_status: evidence_status.to_string(),
            commit_idempotency_key: row.commit_idempotency_key.clone(),
            canonical_commit_payload_hash: row.canonical_commit_payload_hash.clone(),
            source_p19c_ref: row.source_p19c_ref.clone(),
            target_execution_record_ref: row.target_execution_record_ref.clone(),
            runtime_execution_record_ref: Some(runtime_record.execution_record_id),
            runtime_position_state: Some(runtime_record.position_state),
            runtime_actual_exit_date: Some(runtime_record.actual_exit_date),
            runtime_actual_exit_price: Some(runtime_record.actual_exit_price),
            runtime_exit_reason: Some(runtime_record.exit_reason),
            runtime_replay_commit_idempotency_key: runtime_record.replay_commit_idempotency_key,
            runtime_replay_commit_payload_hash: runtime_record.replay_commit_payload_hash,
            runtime_replay_commit_source_p19c_ref: runtime_record.replay_commit_source_p19c_ref,
            runtime_record_present: true,
            closeout_evidence_ready,
            blockers,
        },
        true,
    ))
}

fn validate_eligible_row(
    row: &SecurityPortfolioExecutionLifecycleCloseoutReadinessRow,
) -> Result<(), SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageError> {
    let has_required_evidence = row.closeout_preflight_eligible
        && !row.target_execution_record_ref.trim().is_empty()
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
            SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageError::MissingEligibleRowEvidence(
                row.symbol.clone(),
            ),
        )
    }
}

fn static_evidence_row(
    row: &SecurityPortfolioExecutionLifecycleCloseoutReadinessRow,
    evidence_status: &str,
    runtime_record_present: bool,
    closeout_evidence_ready: bool,
    blockers: Vec<String>,
) -> SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageRow {
    SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageRow {
        symbol: row.symbol.clone(),
        source_p20a_readiness_status: row.readiness_status.clone(),
        evidence_status: evidence_status.to_string(),
        commit_idempotency_key: row.commit_idempotency_key.clone(),
        canonical_commit_payload_hash: row.canonical_commit_payload_hash.clone(),
        source_p19c_ref: row.source_p19c_ref.clone(),
        target_execution_record_ref: row.target_execution_record_ref.clone(),
        runtime_execution_record_ref: row.runtime_execution_record_ref.clone(),
        runtime_position_state: None,
        runtime_actual_exit_date: None,
        runtime_actual_exit_price: None,
        runtime_exit_reason: None,
        runtime_replay_commit_idempotency_key: None,
        runtime_replay_commit_payload_hash: None,
        runtime_replay_commit_source_p19c_ref: None,
        runtime_record_present,
        closeout_evidence_ready,
        blockers,
    }
}

fn resolve_evidence_status(
    row_count: usize,
    evidence_ready_count: usize,
    blocked_count: usize,
) -> String {
    if row_count == 0 {
        return "no_closeout_evidence_candidates".to_string();
    }
    if evidence_ready_count > 0 && blocked_count == 0 {
        return "closeout_evidence_ready".to_string();
    }
    if evidence_ready_count > 0 && blocked_count > 0 {
        return "partial_closeout_evidence_ready".to_string();
    }
    "blocked".to_string()
}

fn build_blockers(blocked_count: usize) -> Vec<String> {
    if blocked_count == 0 {
        Vec::new()
    } else {
        vec![format!("blocked_evidence_row_count={blocked_count}")]
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
