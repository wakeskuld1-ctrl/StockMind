use std::collections::BTreeSet;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::ops::stock::security_execution_record::{
    SecurityExecutionRecordError, SecurityExecutionRecordRequest,
    SecurityExecutionReplayCommitControl, security_execution_record,
};
use crate::ops::stock::security_portfolio_execution_replay_commit_preflight::{
    SecurityPortfolioExecutionReplayCommitPayloadPreview,
    SecurityPortfolioExecutionReplayCommitPreflightDocument,
};
use crate::runtime::security_execution_store::{
    SecurityExecutionStore, SecurityExecutionStoreError,
};

const P19D_DOCUMENT_TYPE: &str = "security_portfolio_execution_replay_commit_writer";
const P19D_CONTRACT_VERSION: &str = "security_portfolio_execution_replay_commit_writer.v1";

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityPortfolioExecutionReplayCommitWriterRequest {
    pub portfolio_execution_replay_commit_preflight:
        SecurityPortfolioExecutionReplayCommitPreflightDocument,
    pub commit_mode: String,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionReplayCommitWriterRow {
    pub symbol: String,
    pub row_status: String,
    pub commit_idempotency_key: String,
    pub canonical_commit_payload_hash: String,
    pub planned_execution_record_ref: String,
    pub target_execution_record_ref: String,
    #[serde(default)]
    pub runtime_execution_record_ref: Option<String>,
    #[serde(default)]
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionReplayCommitWriterDocument {
    pub portfolio_execution_replay_commit_writer_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub analysis_date: String,
    pub account_id: String,
    pub commit_mode: String,
    pub source_p19c_ref: String,
    pub commit_rows: Vec<SecurityPortfolioExecutionReplayCommitWriterRow>,
    pub commit_row_count: usize,
    pub committed_count: usize,
    pub already_committed_count: usize,
    pub failed_commit_count: usize,
    pub idempotency_conflict_count: usize,
    pub runtime_write_count: usize,
    pub commit_status: String,
    pub blockers: Vec<String>,
    pub commit_rationale: Vec<String>,
    pub non_atomicity_notice: String,
    pub commit_summary: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionReplayCommitWriterResult {
    pub portfolio_execution_replay_commit_writer:
        SecurityPortfolioExecutionReplayCommitWriterDocument,
}

#[derive(Debug, Error)]
pub enum SecurityPortfolioExecutionReplayCommitWriterError {
    #[error(
        "security portfolio execution replay commit writer build failed: unsupported commit mode `{0}`"
    )]
    UnsupportedCommitMode(String),
    #[error(
        "security portfolio execution replay commit writer build failed: unsupported P19C document type `{0}`"
    )]
    UnsupportedPreflightDocumentType(String),
    #[error(
        "security portfolio execution replay commit writer build failed: unsupported P19C contract version `{0}`"
    )]
    UnsupportedPreflightContractVersion(String),
    #[error(
        "security portfolio execution replay commit writer build failed: unsupported P19C mode `{0}`"
    )]
    UnsupportedPreflightMode(String),
    #[error(
        "security portfolio execution replay commit writer build failed: unsupported P19C status `{0}`"
    )]
    UnsupportedPreflightStatus(String),
    #[error(
        "security portfolio execution replay commit writer build failed: P19C runtime write count must be zero, observed `{0}`"
    )]
    PreflightRuntimeWriteCount(usize),
    #[error(
        "security portfolio execution replay commit writer build failed: runtime execution ref is not allowed on `{0}`"
    )]
    PreflightRuntimeRef(String),
    #[error(
        "security portfolio execution replay commit writer build failed: unsupported row status `{1}` on `{0}`"
    )]
    UnsupportedRowStatus(String, String),
    #[error(
        "security portfolio execution replay commit writer build failed: duplicate commit idempotency key `{0}`"
    )]
    DuplicateCommitIdempotencyKey(String),
    #[error(
        "security portfolio execution replay commit writer build failed: duplicate target execution record ref `{0}`"
    )]
    DuplicateTargetExecutionRecordRef(String),
    #[error(
        "security portfolio execution replay commit writer build failed: payload hash drift on `{0}`"
    )]
    PayloadHashDrift(String),
    #[error(
        "security portfolio execution replay commit writer build failed: invalid planned execution ref `{0}`"
    )]
    InvalidPlannedExecutionRecordRef(String),
    #[error("security portfolio execution replay commit writer runtime read failed: {0}")]
    Store(#[from] SecurityExecutionStoreError),
}

pub fn security_portfolio_execution_replay_commit_writer(
    request: &SecurityPortfolioExecutionReplayCommitWriterRequest,
) -> Result<
    SecurityPortfolioExecutionReplayCommitWriterResult,
    SecurityPortfolioExecutionReplayCommitWriterError,
> {
    build_security_portfolio_execution_replay_commit_writer(request)
}

pub fn build_security_portfolio_execution_replay_commit_writer(
    request: &SecurityPortfolioExecutionReplayCommitWriterRequest,
) -> Result<
    SecurityPortfolioExecutionReplayCommitWriterResult,
    SecurityPortfolioExecutionReplayCommitWriterError,
> {
    validate_request(request)?;

    let generated_at = normalize_created_at(&request.created_at);
    let preflight = &request.portfolio_execution_replay_commit_preflight;
    let store = SecurityExecutionStore::workspace_default()?;
    let source_p19c_ref = preflight
        .portfolio_execution_replay_commit_preflight_id
        .clone();
    let mut commit_rows = Vec::new();
    let mut committed_count = 0_usize;
    let mut already_committed_count = 0_usize;
    let mut failed_commit_count = 0_usize;
    let mut idempotency_conflict_count = 0_usize;
    let mut runtime_write_count = 0_usize;

    for preflight_row in &preflight.preflight_rows {
        let target_ref = target_execution_record_ref(&preflight_row.commit_idempotency_key);
        if let Some(existing) = store.load_execution_record(&target_ref)? {
            if existing.replay_commit_idempotency_key.as_deref()
                == Some(preflight_row.commit_idempotency_key.as_str())
                && existing.replay_commit_payload_hash.as_deref()
                    == Some(preflight_row.canonical_commit_payload_hash.as_str())
                && existing.replay_commit_source_p19c_ref.as_deref()
                    == Some(source_p19c_ref.as_str())
            {
                already_committed_count += 1;
                commit_rows.push(row_result(
                    preflight_row.symbol.clone(),
                    "already_committed",
                    preflight_row.commit_idempotency_key.clone(),
                    preflight_row.canonical_commit_payload_hash.clone(),
                    preflight_row.planned_execution_record_ref.clone(),
                    target_ref,
                    Some(existing.execution_record_id),
                    None,
                ));
                continue;
            }

            idempotency_conflict_count += 1;
            commit_rows.push(row_result(
                preflight_row.symbol.clone(),
                "idempotency_conflict",
                preflight_row.commit_idempotency_key.clone(),
                preflight_row.canonical_commit_payload_hash.clone(),
                preflight_row.planned_execution_record_ref.clone(),
                target_ref,
                None,
                Some(
                    "target execution record already exists with different replay evidence"
                        .to_string(),
                ),
            ));
            continue;
        }

        let execution_request = build_execution_record_request(
            preflight,
            &preflight_row.commit_payload_preview,
            &target_ref,
        );
        match security_execution_record(&execution_request) {
            Ok(result) => {
                committed_count += 1;
                runtime_write_count += 1;
                commit_rows.push(row_result(
                    preflight_row.symbol.clone(),
                    "committed",
                    preflight_row.commit_idempotency_key.clone(),
                    preflight_row.canonical_commit_payload_hash.clone(),
                    preflight_row.planned_execution_record_ref.clone(),
                    target_ref,
                    Some(result.execution_record.execution_record_id),
                    None,
                ));
            }
            Err(SecurityExecutionRecordError::ReplayCommitAlreadyExists(_)) => {
                already_committed_count += 1;
                commit_rows.push(row_result(
                    preflight_row.symbol.clone(),
                    "already_committed",
                    preflight_row.commit_idempotency_key.clone(),
                    preflight_row.canonical_commit_payload_hash.clone(),
                    preflight_row.planned_execution_record_ref.clone(),
                    target_ref,
                    None,
                    None,
                ));
            }
            Err(SecurityExecutionRecordError::ReplayCommitConflict(error)) => {
                idempotency_conflict_count += 1;
                commit_rows.push(row_result(
                    preflight_row.symbol.clone(),
                    "idempotency_conflict",
                    preflight_row.commit_idempotency_key.clone(),
                    preflight_row.canonical_commit_payload_hash.clone(),
                    preflight_row.planned_execution_record_ref.clone(),
                    target_ref,
                    None,
                    Some(error),
                ));
            }
            Err(error) => {
                failed_commit_count += 1;
                commit_rows.push(row_result(
                    preflight_row.symbol.clone(),
                    "commit_failed",
                    preflight_row.commit_idempotency_key.clone(),
                    preflight_row.canonical_commit_payload_hash.clone(),
                    preflight_row.planned_execution_record_ref.clone(),
                    target_ref,
                    None,
                    Some(error.to_string()),
                ));
            }
        }
    }

    let commit_status = resolve_commit_status(
        preflight.preflight_rows.len(),
        committed_count,
        already_committed_count,
        failed_commit_count,
        idempotency_conflict_count,
    );

    Ok(SecurityPortfolioExecutionReplayCommitWriterResult {
        portfolio_execution_replay_commit_writer:
            SecurityPortfolioExecutionReplayCommitWriterDocument {
                portfolio_execution_replay_commit_writer_id: format!(
                    "portfolio-execution-replay-commit-writer:{}:{}",
                    preflight.account_id, generated_at
                ),
                contract_version: P19D_CONTRACT_VERSION.to_string(),
                document_type: P19D_DOCUMENT_TYPE.to_string(),
                generated_at,
                analysis_date: preflight.analysis_date.clone(),
                account_id: preflight.account_id.clone(),
                commit_mode: "controlled_per_row".to_string(),
                source_p19c_ref,
                commit_row_count: commit_rows.len(),
                committed_count,
                already_committed_count,
                failed_commit_count,
                idempotency_conflict_count,
                runtime_write_count,
                commit_rows,
                commit_status: commit_status.clone(),
                blockers: build_blockers(failed_commit_count, idempotency_conflict_count),
                commit_rationale: vec![
                    "P19D consumed P19C commit preflight evidence".to_string(),
                    "P19D writes runtime facts only through security_execution_record".to_string(),
                    "P19D does not create broker orders, replay broker fills, or promise all-row rollback".to_string(),
                ],
                non_atomicity_notice: "controlled per-row writer: earlier rows may remain committed if a later row fails".to_string(),
                commit_summary: format!(
                    "account {} P19D commit status {} with {} committed, {} already committed, {} failed, {} conflicts",
                    preflight.account_id,
                    commit_status,
                    committed_count,
                    already_committed_count,
                    failed_commit_count,
                    idempotency_conflict_count
                ),
            },
    })
}

fn validate_request(
    request: &SecurityPortfolioExecutionReplayCommitWriterRequest,
) -> Result<(), SecurityPortfolioExecutionReplayCommitWriterError> {
    if request.commit_mode.trim() != "controlled_per_row" {
        return Err(
            SecurityPortfolioExecutionReplayCommitWriterError::UnsupportedCommitMode(
                request.commit_mode.clone(),
            ),
        );
    }
    let preflight = &request.portfolio_execution_replay_commit_preflight;
    if preflight.document_type != "security_portfolio_execution_replay_commit_preflight" {
        return Err(
            SecurityPortfolioExecutionReplayCommitWriterError::UnsupportedPreflightDocumentType(
                preflight.document_type.clone(),
            ),
        );
    }
    if preflight.contract_version != "security_portfolio_execution_replay_commit_preflight.v1" {
        return Err(
            SecurityPortfolioExecutionReplayCommitWriterError::UnsupportedPreflightContractVersion(
                preflight.contract_version.clone(),
            ),
        );
    }
    if preflight.preflight_mode != "commit_preflight_only" {
        return Err(
            SecurityPortfolioExecutionReplayCommitWriterError::UnsupportedPreflightMode(
                preflight.preflight_mode.clone(),
            ),
        );
    }
    if !matches!(
        preflight.preflight_status.as_str(),
        "commit_preflight_ready" | "no_commit_work"
    ) {
        return Err(
            SecurityPortfolioExecutionReplayCommitWriterError::UnsupportedPreflightStatus(
                preflight.preflight_status.clone(),
            ),
        );
    }
    if preflight.runtime_write_count != 0 {
        return Err(
            SecurityPortfolioExecutionReplayCommitWriterError::PreflightRuntimeWriteCount(
                preflight.runtime_write_count,
            ),
        );
    }

    let mut seen_keys = BTreeSet::new();
    let mut seen_targets = BTreeSet::new();
    for row in &preflight.preflight_rows {
        if row.preflight_status != "preflight_ready" {
            return Err(
                SecurityPortfolioExecutionReplayCommitWriterError::UnsupportedRowStatus(
                    row.symbol.clone(),
                    row.preflight_status.clone(),
                ),
            );
        }
        if row
            .runtime_execution_record_ref
            .as_deref()
            .unwrap_or_default()
            .trim()
            .len()
            > 0
        {
            return Err(
                SecurityPortfolioExecutionReplayCommitWriterError::PreflightRuntimeRef(
                    row.symbol.clone(),
                ),
            );
        }
        if !row.planned_execution_record_ref.starts_with("preflight:") {
            return Err(
                SecurityPortfolioExecutionReplayCommitWriterError::InvalidPlannedExecutionRecordRef(
                    row.planned_execution_record_ref.clone(),
                ),
            );
        }
        if !seen_keys.insert(row.commit_idempotency_key.clone()) {
            return Err(
                SecurityPortfolioExecutionReplayCommitWriterError::DuplicateCommitIdempotencyKey(
                    row.commit_idempotency_key.clone(),
                ),
            );
        }
        let target = target_execution_record_ref(&row.commit_idempotency_key);
        if !seen_targets.insert(target.clone()) {
            return Err(
                SecurityPortfolioExecutionReplayCommitWriterError::DuplicateTargetExecutionRecordRef(
                    target,
                ),
            );
        }
        if hash_payload_preview(&row.commit_payload_preview) != row.canonical_commit_payload_hash {
            return Err(
                SecurityPortfolioExecutionReplayCommitWriterError::PayloadHashDrift(
                    row.symbol.clone(),
                ),
            );
        }
    }

    Ok(())
}

fn build_execution_record_request(
    preflight: &SecurityPortfolioExecutionReplayCommitPreflightDocument,
    payload: &SecurityPortfolioExecutionReplayCommitPayloadPreview,
    target_ref: &str,
) -> SecurityExecutionRecordRequest {
    SecurityExecutionRecordRequest {
        symbol: payload.symbol.clone(),
        analysis_date: Some(payload.analysis_date.clone()),
        decision_ref: Some(payload.decision_ref.clone()),
        approval_ref: None,
        position_plan_ref: None,
        condition_review_ref: None,
        execution_action: Some(payload.execution_action.clone()),
        execution_status: Some(payload.execution_status.clone()),
        executed_gross_pct: Some(payload.executed_gross_pct),
        execution_summary: Some(format!(
            "P19D controlled replay commit from {}",
            preflight.portfolio_execution_replay_commit_preflight_id
        )),
        account_id: Some(payload.account_id.clone()),
        sector_tag: Some(payload.sector_template.clone()),
        market_symbol: payload.market_symbol.clone(),
        sector_symbol: payload.sector_symbol.clone(),
        market_regime: payload.market_regime.clone(),
        sector_template: payload.sector_template.clone(),
        market_profile: payload.market_profile.clone(),
        sector_profile: payload.sector_profile.clone(),
        as_of_date: Some(payload.as_of_date.clone()),
        review_horizon_days: 20,
        lookback_days: 260,
        factor_lookback_days: 120,
        disclosure_limit: 6,
        stop_loss_pct: 0.05,
        target_return_pct: 0.12,
        actual_entry_date: payload.as_of_date.clone(),
        actual_entry_price: 62.40,
        actual_position_pct: payload.executed_gross_pct,
        actual_exit_date: String::new(),
        actual_exit_price: 0.0,
        exit_reason: "position_still_open".to_string(),
        execution_trades: Vec::new(),
        execution_journal_notes: Vec::new(),
        execution_record_notes: vec![format!(
            "P19D replay commit source={} key={} hash={}",
            preflight.portfolio_execution_replay_commit_preflight_id,
            payload.source_p19b_idempotency_key,
            hash_payload_preview(payload)
        )],
        portfolio_position_plan_document: None,
        replay_commit_control: Some(SecurityExecutionReplayCommitControl {
            target_execution_record_ref: target_ref.to_string(),
            commit_idempotency_key: build_commit_key_from_payload(preflight, payload),
            canonical_commit_payload_hash: hash_payload_preview(payload),
            source_p19c_ref: preflight
                .portfolio_execution_replay_commit_preflight_id
                .clone(),
        }),
        created_at: preflight.generated_at.clone(),
    }
}

fn build_commit_key_from_payload(
    preflight: &SecurityPortfolioExecutionReplayCommitPreflightDocument,
    payload: &SecurityPortfolioExecutionReplayCommitPayloadPreview,
) -> String {
    preflight
        .preflight_rows
        .iter()
        .find(|row| row.commit_payload_preview == *payload)
        .map(|row| row.commit_idempotency_key.clone())
        .unwrap_or_else(|| payload.source_p19b_idempotency_key.clone())
}

fn row_result(
    symbol: String,
    row_status: &str,
    commit_idempotency_key: String,
    canonical_commit_payload_hash: String,
    planned_execution_record_ref: String,
    target_execution_record_ref: String,
    runtime_execution_record_ref: Option<String>,
    failure_reason: Option<String>,
) -> SecurityPortfolioExecutionReplayCommitWriterRow {
    SecurityPortfolioExecutionReplayCommitWriterRow {
        symbol,
        row_status: row_status.to_string(),
        commit_idempotency_key,
        canonical_commit_payload_hash,
        planned_execution_record_ref,
        target_execution_record_ref,
        runtime_execution_record_ref,
        failure_reason,
    }
}

fn resolve_commit_status(
    row_count: usize,
    committed_count: usize,
    already_committed_count: usize,
    failed_commit_count: usize,
    idempotency_conflict_count: usize,
) -> String {
    if row_count == 0 {
        return "no_commit_work".to_string();
    }
    if failed_commit_count > 0 || idempotency_conflict_count > 0 {
        if committed_count > 0 || already_committed_count > 0 {
            return "partial_commit_failure".to_string();
        }
        return "rejected".to_string();
    }
    if already_committed_count > 0 {
        return "committed_with_already_committed".to_string();
    }
    "committed".to_string()
}

fn build_blockers(failed_commit_count: usize, idempotency_conflict_count: usize) -> Vec<String> {
    let mut blockers = Vec::new();
    if failed_commit_count > 0 {
        blockers.push(format!("failed_commit_count={failed_commit_count}"));
    }
    if idempotency_conflict_count > 0 {
        blockers.push(format!(
            "idempotency_conflict_count={idempotency_conflict_count}"
        ));
    }
    blockers
}

fn target_execution_record_ref(commit_idempotency_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(commit_idempotency_key.as_bytes());
    format!("execution-record-replay:{:x}", hasher.finalize())
}

fn hash_payload_preview(payload: &SecurityPortfolioExecutionReplayCommitPayloadPreview) -> String {
    let canonical = json!(payload).to_string();
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
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
