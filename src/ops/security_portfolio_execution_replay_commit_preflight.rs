use std::collections::{BTreeMap, BTreeSet};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::ops::stock::security_portfolio_execution_replay_executor::{
    SecurityPortfolioExecutionReplayExecutorDocument, SecurityPortfolioExecutionReplayExecutorRow,
};
use crate::ops::stock::security_portfolio_execution_request_enrichment::{
    SecurityPortfolioEnrichedExecutionRequestRow,
    SecurityPortfolioExecutionRequestEnrichmentDocument,
};

const SECURITY_PORTFOLIO_EXECUTION_REPLAY_COMMIT_PREFLIGHT_DOCUMENT_TYPE: &str =
    "security_portfolio_execution_replay_commit_preflight";
const SECURITY_PORTFOLIO_EXECUTION_REPLAY_COMMIT_PREFLIGHT_VERSION: &str =
    "security_portfolio_execution_replay_commit_preflight.v1";

// 2026-04-26 CST: Added because P19C freezes commit inputs before any P19D
// runtime writer exists.
// Reason: preflight must not leak commit-mode writes into the P19B dry-run executor.
// Purpose: define the public P19C request shell with explicit side-effect-free mode.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityPortfolioExecutionReplayCommitPreflightRequest {
    pub portfolio_execution_replay_executor: SecurityPortfolioExecutionReplayExecutorDocument,
    pub portfolio_execution_request_enrichment: SecurityPortfolioExecutionRequestEnrichmentDocument,
    pub preflight_mode: String,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-26 CST: Added because P19C output needs a canonical future commit
// preview without constructing or persisting a runtime execution record.
// Reason: P19D needs structured payload evidence and hashes before writes are approved.
// Purpose: carry the minimal execution-record-aligned fields as document data only.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionReplayCommitPayloadPreview {
    pub symbol: String,
    pub analysis_date: String,
    pub decision_ref: String,
    pub execution_action: String,
    pub execution_status: String,
    pub executed_gross_pct: f64,
    pub account_id: String,
    pub as_of_date: String,
    #[serde(default)]
    pub market_symbol: Option<String>,
    #[serde(default)]
    pub sector_symbol: Option<String>,
    pub market_regime: String,
    pub sector_template: String,
    #[serde(default)]
    pub market_profile: Option<String>,
    #[serde(default)]
    pub sector_profile: Option<String>,
    pub replay_evidence_refs: Vec<String>,
    pub source_p19b_idempotency_key: String,
}

// 2026-04-26 CST: Added because P19C rows are readiness evidence, not runtime
// execution facts.
// Reason: every row must expose future idempotency and hash evidence while runtime refs stay empty.
// Purpose: represent one replay row after structured commit preflight.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionReplayCommitPreflightRow {
    pub symbol: String,
    pub request_action: String,
    pub requested_gross_pct: f64,
    pub preflight_status: String,
    pub source_p19b_idempotency_key: String,
    pub commit_idempotency_key: String,
    pub canonical_commit_payload_hash: String,
    pub planned_execution_record_ref: String,
    #[serde(default)]
    pub runtime_execution_record_ref: Option<String>,
    pub commit_payload_preview: SecurityPortfolioExecutionReplayCommitPayloadPreview,
    pub preflight_summary: String,
}

// 2026-04-26 CST: Added because P19C must freeze commit-readiness without
// claiming actual replay commit.
// Reason: runtime write count must remain zero until a separately approved P19D writer exists.
// Purpose: define the formal P19C preflight document.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionReplayCommitPreflightDocument {
    pub portfolio_execution_replay_commit_preflight_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub analysis_date: String,
    pub account_id: String,
    pub preflight_mode: String,
    pub portfolio_execution_replay_executor_ref: String,
    pub portfolio_execution_replay_request_package_ref: String,
    pub portfolio_execution_request_enrichment_ref: String,
    pub portfolio_execution_request_package_ref: String,
    pub portfolio_execution_preview_ref: String,
    pub portfolio_allocation_decision_ref: String,
    pub preflight_rows: Vec<SecurityPortfolioExecutionReplayCommitPreflightRow>,
    pub preflight_row_count: usize,
    pub runtime_write_count: usize,
    pub preflight_status: String,
    pub blockers: Vec<String>,
    pub preflight_rationale: Vec<String>,
    pub preflight_summary: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionReplayCommitPreflightResult {
    pub portfolio_execution_replay_commit_preflight:
        SecurityPortfolioExecutionReplayCommitPreflightDocument,
}

#[derive(Debug, Error)]
pub enum SecurityPortfolioExecutionReplayCommitPreflightError {
    #[error(
        "security portfolio execution replay commit preflight build failed: unsupported preflight mode `{0}`"
    )]
    UnsupportedPreflightMode(String),
    #[error(
        "security portfolio execution replay commit preflight build failed: unsupported replay executor mode `{0}`"
    )]
    UnsupportedReplayExecutorMode(String),
    #[error(
        "security portfolio execution replay commit preflight build failed: unsupported replay executor status `{0}`"
    )]
    UnsupportedReplayExecutorStatus(String),
    #[error(
        "security portfolio execution replay commit preflight build failed: unsupported replay executor document type `{0}`"
    )]
    UnsupportedReplayExecutorDocumentType(String),
    #[error(
        "security portfolio execution replay commit preflight build failed: unsupported replay executor contract version `{0}`"
    )]
    UnsupportedReplayExecutorContractVersion(String),
    #[error(
        "security portfolio execution replay commit preflight build failed: unsupported request enrichment document type `{0}`"
    )]
    UnsupportedRequestEnrichmentDocumentType(String),
    #[error(
        "security portfolio execution replay commit preflight build failed: unsupported request enrichment contract version `{0}`"
    )]
    UnsupportedRequestEnrichmentContractVersion(String),
    #[error(
        "security portfolio execution replay commit preflight build failed: unsupported request enrichment readiness status `{0}`"
    )]
    UnsupportedRequestEnrichmentReadinessStatus(String),
    #[error(
        "security portfolio execution replay commit preflight build failed: runtime execution ref is not allowed on `{0}`"
    )]
    RuntimeExecutionRefNotAllowed(String),
    #[error(
        "security portfolio execution replay commit preflight build failed: lineage mismatch `{0}`"
    )]
    LineageMismatch(&'static str),
    #[error(
        "security portfolio execution replay commit preflight build failed: summary count mismatch `{0}` observed `{1}` expected `{2}`"
    )]
    SummaryCountMismatch(&'static str, usize, usize),
    #[error(
        "security portfolio execution replay commit preflight build failed: missing enrichment match for `{0}`"
    )]
    MissingEnrichmentMatch(String),
    #[error(
        "security portfolio execution replay commit preflight build failed: ambiguous enrichment match for `{0}`"
    )]
    AmbiguousEnrichmentMatch(String),
    #[error(
        "security portfolio execution replay commit preflight build failed: unsupported enrichment status `{1}` on `{0}`"
    )]
    UnsupportedEnrichmentStatus(String, String),
    #[error(
        "security portfolio execution replay commit preflight build failed: missing apply context `{1}` on `{0}`"
    )]
    MissingApplyContext(String, &'static str),
    #[error(
        "security portfolio execution replay commit preflight build failed: missing replay evidence for `{0}`"
    )]
    MissingReplayEvidence(String),
    #[error(
        "security portfolio execution replay commit preflight build failed: duplicate commit idempotency key `{0}`"
    )]
    DuplicateCommitIdempotencyKey(String),
    #[error(
        "security portfolio execution replay commit preflight build failed: conflicting commit payload hash for `{0}`"
    )]
    ConflictingCommitPayloadHash(String),
}

// 2026-04-26 CST: Added because P19C must be callable from the public stock bus
// as a preflight-only stage after P19B.
// Reason: future runtime replay must consume frozen commit-readiness evidence rather than ad hoc dry-run rows.
// Purpose: expose the P19B-to-P19C preflight entry point without runtime writes.
pub fn security_portfolio_execution_replay_commit_preflight(
    request: &SecurityPortfolioExecutionReplayCommitPreflightRequest,
) -> Result<
    SecurityPortfolioExecutionReplayCommitPreflightResult,
    SecurityPortfolioExecutionReplayCommitPreflightError,
> {
    build_security_portfolio_execution_replay_commit_preflight(request)
}

pub fn build_security_portfolio_execution_replay_commit_preflight(
    request: &SecurityPortfolioExecutionReplayCommitPreflightRequest,
) -> Result<
    SecurityPortfolioExecutionReplayCommitPreflightResult,
    SecurityPortfolioExecutionReplayCommitPreflightError,
> {
    let generated_at = normalize_created_at(&request.created_at);
    validate_preflight_mode(&request.preflight_mode)?;
    let executor = &request.portfolio_execution_replay_executor;
    let enrichment = &request.portfolio_execution_request_enrichment;
    validate_executor_document(executor)?;
    validate_enrichment_document(enrichment)?;
    validate_lineage(executor, enrichment)?;

    let mut seen_commit_keys = BTreeSet::new();
    let mut seen_payload_hashes = BTreeMap::<String, String>::new();
    let mut preflight_rows = Vec::new();

    for executor_row in &executor.executor_rows {
        let enrichment_row = find_matching_enrichment_row(executor_row, enrichment)?;
        let preflight_row = build_preflight_row(executor, executor_row, enrichment_row)?;
        if !seen_commit_keys.insert(preflight_row.commit_idempotency_key.clone()) {
            return Err(
                SecurityPortfolioExecutionReplayCommitPreflightError::DuplicateCommitIdempotencyKey(
                    preflight_row.commit_idempotency_key,
                ),
            );
        }
        if let Some(existing_hash) = seen_payload_hashes.insert(
            preflight_row.commit_idempotency_key.clone(),
            preflight_row.canonical_commit_payload_hash.clone(),
        ) {
            if existing_hash != preflight_row.canonical_commit_payload_hash {
                return Err(
                    SecurityPortfolioExecutionReplayCommitPreflightError::ConflictingCommitPayloadHash(
                        preflight_row.commit_idempotency_key,
                    ),
                );
            }
        }
        preflight_rows.push(preflight_row);
    }

    let preflight_row_count = preflight_rows.len();
    let preflight_status = if preflight_row_count == 0 {
        "no_commit_work"
    } else {
        "commit_preflight_ready"
    };

    Ok(SecurityPortfolioExecutionReplayCommitPreflightResult {
        portfolio_execution_replay_commit_preflight:
            SecurityPortfolioExecutionReplayCommitPreflightDocument {
                portfolio_execution_replay_commit_preflight_id: format!(
                    "portfolio-execution-replay-commit-preflight:{}:{}",
                    executor.account_id, generated_at
                ),
                contract_version: SECURITY_PORTFOLIO_EXECUTION_REPLAY_COMMIT_PREFLIGHT_VERSION
                    .to_string(),
                document_type: SECURITY_PORTFOLIO_EXECUTION_REPLAY_COMMIT_PREFLIGHT_DOCUMENT_TYPE
                    .to_string(),
                generated_at,
                analysis_date: executor.analysis_date.clone(),
                account_id: executor.account_id.clone(),
                preflight_mode: "commit_preflight_only".to_string(),
                portfolio_execution_replay_executor_ref: executor
                    .portfolio_execution_replay_executor_id
                    .clone(),
                portfolio_execution_replay_request_package_ref: executor
                    .portfolio_execution_replay_request_package_ref
                    .clone(),
                portfolio_execution_request_enrichment_ref: executor
                    .portfolio_execution_request_enrichment_ref
                    .clone(),
                portfolio_execution_request_package_ref: executor
                    .portfolio_execution_request_package_ref
                    .clone(),
                portfolio_execution_preview_ref: executor.portfolio_execution_preview_ref.clone(),
                portfolio_allocation_decision_ref: executor
                    .portfolio_allocation_decision_ref
                    .clone(),
                preflight_rows,
                preflight_row_count,
                runtime_write_count: 0,
                preflight_status: preflight_status.to_string(),
                blockers: executor.blockers.clone(),
                preflight_rationale: build_preflight_rationale(executor),
                preflight_summary: format!(
                    "account {} froze replay executor {} as {}",
                    executor.account_id,
                    executor.portfolio_execution_replay_executor_id,
                    preflight_status
                ),
            },
    })
}

fn validate_preflight_mode(
    preflight_mode: &str,
) -> Result<(), SecurityPortfolioExecutionReplayCommitPreflightError> {
    match preflight_mode.trim() {
        "commit_preflight_only" => Ok(()),
        other => Err(
            SecurityPortfolioExecutionReplayCommitPreflightError::UnsupportedPreflightMode(
                other.to_string(),
            ),
        ),
    }
}

fn validate_executor_document(
    executor: &SecurityPortfolioExecutionReplayExecutorDocument,
) -> Result<(), SecurityPortfolioExecutionReplayCommitPreflightError> {
    // 2026-04-26 CST: Added after the independent risk pass found P19C accepted
    // structure-compatible non-P19B artifacts.
    // Reason: commit preflight must consume only the formal P19B dry-run executor contract.
    // Purpose: fail fast before any lineage or row matching when document identity drifts.
    if executor.document_type != "security_portfolio_execution_replay_executor" {
        return Err(SecurityPortfolioExecutionReplayCommitPreflightError::UnsupportedReplayExecutorDocumentType(
            executor.document_type.clone(),
        ));
    }
    if executor.contract_version != "security_portfolio_execution_replay_executor.v1" {
        return Err(SecurityPortfolioExecutionReplayCommitPreflightError::UnsupportedReplayExecutorContractVersion(
            executor.contract_version.clone(),
        ));
    }
    if executor.execution_mode != "dry_run" {
        return Err(
            SecurityPortfolioExecutionReplayCommitPreflightError::UnsupportedReplayExecutorMode(
                executor.execution_mode.clone(),
            ),
        );
    }
    match executor.dry_run_status.as_str() {
        "no_replay_work" | "validated_for_dry_run" => {}
        other => {
            return Err(
                SecurityPortfolioExecutionReplayCommitPreflightError::UnsupportedReplayExecutorStatus(
                    other.to_string(),
                ),
            );
        }
    }
    if executor.runtime_write_count != 0 {
        return Err(
            SecurityPortfolioExecutionReplayCommitPreflightError::SummaryCountMismatch(
                "runtime_write_count",
                executor.runtime_write_count,
                0,
            ),
        );
    }
    if executor.executor_rows.len() != executor.dry_run_row_count {
        return Err(
            SecurityPortfolioExecutionReplayCommitPreflightError::SummaryCountMismatch(
                "dry_run_row_count",
                executor.executor_rows.len(),
                executor.dry_run_row_count,
            ),
        );
    }
    if executor.dry_run_status == "no_replay_work" && !executor.executor_rows.is_empty() {
        return Err(
            SecurityPortfolioExecutionReplayCommitPreflightError::SummaryCountMismatch(
                "no_replay_work_executor_rows",
                executor.executor_rows.len(),
                0,
            ),
        );
    }
    for row in &executor.executor_rows {
        if row.runtime_execution_record_ref.is_some() {
            return Err(
                SecurityPortfolioExecutionReplayCommitPreflightError::RuntimeExecutionRefNotAllowed(
                    row.symbol.clone(),
                ),
            );
        }
        if row.dry_run_status != "validated_for_dry_run" {
            return Err(
                SecurityPortfolioExecutionReplayCommitPreflightError::UnsupportedReplayExecutorStatus(
                    row.dry_run_status.clone(),
                ),
            );
        }
    }
    Ok(())
}

fn validate_enrichment_document(
    enrichment: &SecurityPortfolioExecutionRequestEnrichmentDocument,
) -> Result<(), SecurityPortfolioExecutionReplayCommitPreflightError> {
    // 2026-04-26 CST: Added after the independent risk pass found P19C accepted
    // lookalike or blocked P14 artifacts.
    // Reason: future commit payload hashes require the exact P14 enrichment contract and a non-blocked bundle.
    // Purpose: keep empty P19B no-work preflight from masking upstream blocked enrichment truth.
    if enrichment.document_type != "security_portfolio_execution_request_enrichment" {
        return Err(SecurityPortfolioExecutionReplayCommitPreflightError::UnsupportedRequestEnrichmentDocumentType(
            enrichment.document_type.clone(),
        ));
    }
    if enrichment.contract_version != "security_portfolio_execution_request_enrichment.v1" {
        return Err(SecurityPortfolioExecutionReplayCommitPreflightError::UnsupportedRequestEnrichmentContractVersion(
            enrichment.contract_version.clone(),
        ));
    }
    if enrichment.readiness_status == "blocked" {
        return Err(SecurityPortfolioExecutionReplayCommitPreflightError::UnsupportedRequestEnrichmentReadinessStatus(
            enrichment.readiness_status.clone(),
        ));
    }
    let observed_ready = enrichment
        .enriched_request_rows
        .iter()
        .filter(|row| row.enrichment_status == "ready_for_apply")
        .count();
    let observed_hold = enrichment
        .enriched_request_rows
        .iter()
        .filter(|row| row.enrichment_status == "non_executable_hold")
        .count();
    let observed_blocked = enrichment
        .enriched_request_rows
        .iter()
        .filter(|row| row.enrichment_status == "blocked")
        .count();
    for (name, observed, expected) in [
        (
            "ready_for_apply_count",
            observed_ready,
            enrichment.ready_for_apply_count,
        ),
        (
            "non_executable_hold_count",
            observed_hold,
            enrichment.non_executable_hold_count,
        ),
        (
            "blocked_enrichment_count",
            observed_blocked,
            enrichment.blocked_enrichment_count,
        ),
    ] {
        if observed != expected {
            return Err(
                SecurityPortfolioExecutionReplayCommitPreflightError::SummaryCountMismatch(
                    name, observed, expected,
                ),
            );
        }
    }
    Ok(())
}

fn validate_lineage(
    executor: &SecurityPortfolioExecutionReplayExecutorDocument,
    enrichment: &SecurityPortfolioExecutionRequestEnrichmentDocument,
) -> Result<(), SecurityPortfolioExecutionReplayCommitPreflightError> {
    for (name, left, right) in [
        (
            "account_id",
            executor.account_id.as_str(),
            enrichment.account_id.as_str(),
        ),
        (
            "analysis_date",
            executor.analysis_date.as_str(),
            enrichment.analysis_date.as_str(),
        ),
        (
            "portfolio_execution_request_enrichment_ref",
            executor.portfolio_execution_request_enrichment_ref.as_str(),
            enrichment
                .portfolio_execution_request_enrichment_id
                .as_str(),
        ),
        (
            "portfolio_execution_request_package_ref",
            executor.portfolio_execution_request_package_ref.as_str(),
            enrichment.portfolio_execution_request_package_ref.as_str(),
        ),
        (
            "portfolio_execution_preview_ref",
            executor.portfolio_execution_preview_ref.as_str(),
            enrichment.portfolio_execution_preview_ref.as_str(),
        ),
        (
            "portfolio_allocation_decision_ref",
            executor.portfolio_allocation_decision_ref.as_str(),
            enrichment.portfolio_allocation_decision_ref.as_str(),
        ),
    ] {
        if left != right {
            return Err(
                SecurityPortfolioExecutionReplayCommitPreflightError::LineageMismatch(name),
            );
        }
    }
    Ok(())
}

fn find_matching_enrichment_row<'a>(
    executor_row: &SecurityPortfolioExecutionReplayExecutorRow,
    enrichment: &'a SecurityPortfolioExecutionRequestEnrichmentDocument,
) -> Result<
    &'a SecurityPortfolioEnrichedExecutionRequestRow,
    SecurityPortfolioExecutionReplayCommitPreflightError,
> {
    let matches = enrichment
        .enriched_request_rows
        .iter()
        .filter(|row| {
            row.symbol == executor_row.symbol
                && row.request_action == executor_row.request_action
                && floats_match(row.requested_gross_pct, executor_row.requested_gross_pct)
                && floats_match(row.executed_gross_pct, executor_row.requested_gross_pct)
        })
        .collect::<Vec<_>>();
    match matches.len() {
        0 => Err(
            SecurityPortfolioExecutionReplayCommitPreflightError::MissingEnrichmentMatch(
                executor_row.symbol.clone(),
            ),
        ),
        1 => Ok(matches[0]),
        _ => Err(
            SecurityPortfolioExecutionReplayCommitPreflightError::AmbiguousEnrichmentMatch(
                executor_row.symbol.clone(),
            ),
        ),
    }
}

fn build_preflight_row(
    executor: &SecurityPortfolioExecutionReplayExecutorDocument,
    executor_row: &SecurityPortfolioExecutionReplayExecutorRow,
    enrichment_row: &SecurityPortfolioEnrichedExecutionRequestRow,
) -> Result<
    SecurityPortfolioExecutionReplayCommitPreflightRow,
    SecurityPortfolioExecutionReplayCommitPreflightError,
> {
    if enrichment_row.enrichment_status != "ready_for_apply" {
        return Err(
            SecurityPortfolioExecutionReplayCommitPreflightError::UnsupportedEnrichmentStatus(
                enrichment_row.symbol.clone(),
                enrichment_row.enrichment_status.clone(),
            ),
        );
    }
    if executor_row.replay_evidence_refs.is_empty() {
        return Err(
            SecurityPortfolioExecutionReplayCommitPreflightError::MissingReplayEvidence(
                executor_row.symbol.clone(),
            ),
        );
    }
    validate_apply_context(enrichment_row)?;

    let payload_preview = build_payload_preview(executor, executor_row, enrichment_row);
    let canonical_commit_payload_hash = hash_payload_preview(&payload_preview);
    let commit_idempotency_key = build_commit_idempotency_key(executor, executor_row);

    Ok(SecurityPortfolioExecutionReplayCommitPreflightRow {
        symbol: executor_row.symbol.clone(),
        request_action: executor_row.request_action.clone(),
        requested_gross_pct: executor_row.requested_gross_pct,
        preflight_status: "preflight_ready".to_string(),
        source_p19b_idempotency_key: executor_row.idempotency_key.clone(),
        commit_idempotency_key: commit_idempotency_key.clone(),
        canonical_commit_payload_hash,
        planned_execution_record_ref: format!("preflight:{commit_idempotency_key}"),
        runtime_execution_record_ref: None,
        commit_payload_preview: payload_preview,
        preflight_summary: format!("{} frozen for replay commit preflight", executor_row.symbol),
    })
}

fn validate_apply_context(
    row: &SecurityPortfolioEnrichedExecutionRequestRow,
) -> Result<(), SecurityPortfolioExecutionReplayCommitPreflightError> {
    if row.execution_action.trim().is_empty() {
        return Err(
            SecurityPortfolioExecutionReplayCommitPreflightError::MissingApplyContext(
                row.symbol.clone(),
                "execution_action",
            ),
        );
    }
    if row.executed_gross_pct <= 0.0 {
        return Err(
            SecurityPortfolioExecutionReplayCommitPreflightError::MissingApplyContext(
                row.symbol.clone(),
                "executed_gross_pct",
            ),
        );
    }
    if row.execution_apply_context.as_of_date.trim().is_empty() {
        return Err(
            SecurityPortfolioExecutionReplayCommitPreflightError::MissingApplyContext(
                row.symbol.clone(),
                "as_of_date",
            ),
        );
    }
    if row.execution_apply_context.market_regime.trim().is_empty() {
        return Err(
            SecurityPortfolioExecutionReplayCommitPreflightError::MissingApplyContext(
                row.symbol.clone(),
                "market_regime",
            ),
        );
    }
    if row
        .execution_apply_context
        .sector_template
        .trim()
        .is_empty()
    {
        return Err(
            SecurityPortfolioExecutionReplayCommitPreflightError::MissingApplyContext(
                row.symbol.clone(),
                "sector_template",
            ),
        );
    }
    Ok(())
}

fn build_payload_preview(
    executor: &SecurityPortfolioExecutionReplayExecutorDocument,
    executor_row: &SecurityPortfolioExecutionReplayExecutorRow,
    enrichment_row: &SecurityPortfolioEnrichedExecutionRequestRow,
) -> SecurityPortfolioExecutionReplayCommitPayloadPreview {
    SecurityPortfolioExecutionReplayCommitPayloadPreview {
        symbol: enrichment_row.symbol.clone(),
        analysis_date: enrichment_row.analysis_date.clone(),
        decision_ref: enrichment_row.decision_ref.clone(),
        execution_action: enrichment_row.execution_action.clone(),
        execution_status: "preflight_ready".to_string(),
        executed_gross_pct: enrichment_row.executed_gross_pct,
        account_id: executor.account_id.clone(),
        as_of_date: enrichment_row.execution_apply_context.as_of_date.clone(),
        market_symbol: enrichment_row.execution_apply_context.market_symbol.clone(),
        sector_symbol: enrichment_row.execution_apply_context.sector_symbol.clone(),
        market_regime: enrichment_row.execution_apply_context.market_regime.clone(),
        sector_template: enrichment_row
            .execution_apply_context
            .sector_template
            .clone(),
        market_profile: enrichment_row
            .execution_apply_context
            .market_profile
            .clone(),
        sector_profile: enrichment_row
            .execution_apply_context
            .sector_profile
            .clone(),
        replay_evidence_refs: executor_row.replay_evidence_refs.clone(),
        source_p19b_idempotency_key: executor_row.idempotency_key.clone(),
    }
}

fn build_commit_idempotency_key(
    executor: &SecurityPortfolioExecutionReplayExecutorDocument,
    row: &SecurityPortfolioExecutionReplayExecutorRow,
) -> String {
    format!(
        "p19c|{}|{}|{}|{}|{}|{}|{}",
        executor.account_id,
        executor.analysis_date,
        row.symbol,
        row.request_action,
        row.requested_gross_pct,
        executor.portfolio_execution_replay_executor_id,
        row.idempotency_key
    )
}

fn hash_payload_preview(payload: &SecurityPortfolioExecutionReplayCommitPayloadPreview) -> String {
    let canonical = json!(payload).to_string();
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn build_preflight_rationale(
    executor: &SecurityPortfolioExecutionReplayExecutorDocument,
) -> Vec<String> {
    vec![
        format!(
            "execution replay commit preflight consumed replay executor {}",
            executor.portfolio_execution_replay_executor_id
        ),
        "execution replay commit preflight freezes commit payload hashes without writing runtime facts"
            .to_string(),
        "execution replay commit preflight does not call security_execution_record, write ledgers, replay broker fills, materialize positions, or close lifecycle"
            .to_string(),
    ]
}

fn floats_match(left: f64, right: f64) -> bool {
    (left - right).abs() <= 0.000_000_1
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
