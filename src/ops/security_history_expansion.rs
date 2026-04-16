use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_external_proxy_backfill::{
    SecurityExternalProxyBackfillError, SecurityExternalProxyBackfillResult,
    load_security_external_proxy_backfill_result,
};

// 2026-04-11 CST: Add a governed history-expansion request contract, because P5
// needs historical proxy-coverage growth to be auditable instead of remaining
// implied by scattered backfill batches.
// Purpose: let CLI and later promotion governance point to one explicit expansion record.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityHistoryExpansionRequest {
    pub created_at: String,
    #[serde(default)]
    pub history_runtime_root: Option<String>,
    pub market_scope: String,
    pub instrument_scope: String,
    #[serde(default)]
    pub instrument_subscope: Option<String>,
    pub proxy_fields: Vec<String>,
    pub date_range: String,
    pub symbol_list: Vec<String>,
    #[serde(default)]
    pub backfill_result_paths: Vec<String>,
    pub coverage_summary: SecurityHistoryExpansionCoverageSummary,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityHistoryExpansionCoverageSummary {
    pub horizon_days: Vec<usize>,
    pub coverage_note: String,
    // 2026-04-11 CST: Add standardized coverage metadata, because P6 needs history
    // expansion records to feed governance directly instead of remaining free-form notes.
    // Purpose: let shadow/champion promotion read stable readiness hints from one contract.
    #[serde(default = "default_coverage_tier")]
    pub coverage_tier: String,
    #[serde(default = "default_shadow_readiness_hint")]
    pub shadow_readiness_hint: String,
    #[serde(default = "default_champion_readiness_hint")]
    pub champion_readiness_hint: String,
    #[serde(default)]
    pub proxy_field_coverage: Vec<SecurityHistoryExpansionProxyFieldCoverage>,
    #[serde(default)]
    pub consumed_backfill_batch_refs: Vec<String>,
    #[serde(default)]
    pub covered_dates: Vec<String>,
    #[serde(default)]
    pub imported_record_count: usize,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityHistoryExpansionProxyFieldCoverage {
    pub proxy_field: String,
    pub coverage_status: String,
    pub covered_horizons: Vec<usize>,
}

// 2026-04-11 CST: Add a governed history-expansion document, because P5 needs one
// durable object that later shadow evaluation can read when deciding whether proxy
// coverage is ready for promotion.
// Purpose: keep history expansion reviewable without re-reading raw backfill batches.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityHistoryExpansionDocument {
    pub history_expansion_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub created_at: String,
    pub market_scope: String,
    pub instrument_scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument_subscope: Option<String>,
    pub date_range: String,
    pub proxy_fields: Vec<String>,
    pub symbol_list: Vec<String>,
    pub coverage_summary: SecurityHistoryExpansionCoverageSummary,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityHistoryExpansionResult {
    pub history_expansion: SecurityHistoryExpansionDocument,
    pub history_expansion_path: String,
}

#[derive(Debug, Error)]
pub enum SecurityHistoryExpansionError {
    #[error("security history expansion build failed: {0}")]
    Build(String),
    #[error("security history expansion persist failed: {0}")]
    Persist(String),
    #[error("security history expansion backfill result failed: {0}")]
    BackfillResult(#[from] SecurityExternalProxyBackfillError),
}

// 2026-04-11 CST: Persist a governed history-expansion document, because P5 needs
// proxy-history growth to become a first-class auditable object before promotion
// logic starts consuming coverage evidence.
// Purpose: create one stable record per expansion scope/date-range instead of relying on notes.
pub fn security_history_expansion(
    request: &SecurityHistoryExpansionRequest,
) -> Result<SecurityHistoryExpansionResult, SecurityHistoryExpansionError> {
    validate_request(request)?;
    let backfill_results = request
        .backfill_result_paths
        .iter()
        .map(|path| load_security_external_proxy_backfill_result(path))
        .collect::<Result<Vec<_>, _>>()?;

    let document = build_history_expansion_document(request, &backfill_results);
    let runtime_root = resolve_runtime_root(request);
    let path = runtime_root.join("history_expansions").join(format!(
        "{}.json",
        sanitize_identifier(&document.history_expansion_id)
    ));
    persist_json(&path, &document)?;

    Ok(SecurityHistoryExpansionResult {
        history_expansion: document,
        history_expansion_path: path.to_string_lossy().to_string(),
    })
}

// 2026-04-11 CST: Add a small loader for governed history-expansion documents,
// because shadow evaluation should reuse the exact persisted contract rather than
// rolling its own loose JSON parsing.
// Purpose: centralize the document boundary for later governance consumers.
pub fn load_security_history_expansion_document(
    path: &str,
) -> Result<SecurityHistoryExpansionDocument, SecurityHistoryExpansionError> {
    let payload = fs::read(path).map_err(|error| {
        SecurityHistoryExpansionError::Persist(format!(
            "failed to read history expansion `{path}`: {error}"
        ))
    })?;
    serde_json::from_slice::<SecurityHistoryExpansionDocument>(&payload).map_err(|error| {
        SecurityHistoryExpansionError::Build(format!(
            "failed to parse history expansion `{path}`: {error}"
        ))
    })
}

fn build_history_expansion_document(
    request: &SecurityHistoryExpansionRequest,
    backfill_results: &[SecurityExternalProxyBackfillResult],
) -> SecurityHistoryExpansionDocument {
    // 2026-04-11 CST: Build one deterministic expansion id, because P5 governance
    // will later link evaluation and promotion decisions back to specific history
    // coverage windows.
    // Purpose: keep references stable across CLI output, persisted files, and approval notes.
    let instrument_subscope = request
        .instrument_subscope
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let subscope_for_id = instrument_subscope
        .as_deref()
        .map(sanitize_identifier)
        .unwrap_or_else(|| "none".to_string());

    let coverage_summary = build_standardized_coverage_summary(
        &request.coverage_summary,
        &request.proxy_fields,
        backfill_results,
    );

    SecurityHistoryExpansionDocument {
        history_expansion_id: format!(
            "history-expansion:{}:{}:{}:{}:v1",
            request.market_scope.trim(),
            request.instrument_scope.trim(),
            subscope_for_id,
            request.date_range.trim().replace("..", "_")
        ),
        contract_version: "security_history_expansion.v1".to_string(),
        document_type: "security_history_expansion".to_string(),
        created_at: request.created_at.trim().to_string(),
        market_scope: request.market_scope.trim().to_string(),
        instrument_scope: request.instrument_scope.trim().to_string(),
        instrument_subscope,
        date_range: request.date_range.trim().to_string(),
        proxy_fields: dedup_sorted_strings(&request.proxy_fields),
        symbol_list: dedup_sorted_strings(&request.symbol_list),
        coverage_summary,
    }
}

// 2026-04-11 CST: Standardize coverage output during document build, because P6
// needs every history-expansion record to expose reusable readiness hints even
// when the caller only supplied the minimal note + horizons payload.
// Purpose: avoid leaving shadow/champion governance dependent on ad-hoc text parsing.
fn build_standardized_coverage_summary(
    summary: &SecurityHistoryExpansionCoverageSummary,
    proxy_fields: &[String],
    backfill_results: &[SecurityExternalProxyBackfillResult],
) -> SecurityHistoryExpansionCoverageSummary {
    let mut standardized = summary.clone();
    standardized.coverage_tier = default_coverage_tier();
    standardized.shadow_readiness_hint = default_shadow_readiness_hint();
    standardized.champion_readiness_hint = default_champion_readiness_hint();
    standardized.proxy_field_coverage = dedup_sorted_strings(proxy_fields)
        .into_iter()
        .map(|proxy_field| SecurityHistoryExpansionProxyFieldCoverage {
            proxy_field,
            coverage_status: "covered_in_expansion".to_string(),
            covered_horizons: dedup_sorted_horizons(&summary.horizon_days),
        })
        .collect();
    standardized.consumed_backfill_batch_refs = dedup_sorted_strings(
        &backfill_results
            .iter()
            .map(|result| result.batch_ref.clone())
            .collect::<Vec<_>>(),
    );
    standardized.covered_dates = dedup_sorted_strings(
        &backfill_results
            .iter()
            .flat_map(|result| result.covered_dates.clone())
            .collect::<Vec<_>>(),
    );
    standardized.imported_record_count = backfill_results
        .iter()
        .map(|result| result.imported_record_count)
        .sum();
    standardized
}

fn validate_request(
    request: &SecurityHistoryExpansionRequest,
) -> Result<(), SecurityHistoryExpansionError> {
    for (field_name, field_value) in [
        ("created_at", request.created_at.trim()),
        ("market_scope", request.market_scope.trim()),
        ("instrument_scope", request.instrument_scope.trim()),
        ("date_range", request.date_range.trim()),
        (
            "coverage_summary.coverage_note",
            request.coverage_summary.coverage_note.trim(),
        ),
    ] {
        if field_value.is_empty() {
            return Err(SecurityHistoryExpansionError::Build(format!(
                "{field_name} cannot be empty"
            )));
        }
    }
    if request.proxy_fields.is_empty() {
        return Err(SecurityHistoryExpansionError::Build(
            "proxy_fields cannot be empty".to_string(),
        ));
    }
    if request.symbol_list.is_empty() {
        return Err(SecurityHistoryExpansionError::Build(
            "symbol_list cannot be empty".to_string(),
        ));
    }
    if request.coverage_summary.horizon_days.is_empty() {
        return Err(SecurityHistoryExpansionError::Build(
            "coverage_summary.horizon_days cannot be empty".to_string(),
        ));
    }
    Ok(())
}

fn persist_json<T: Serialize>(path: &Path, value: &T) -> Result<(), SecurityHistoryExpansionError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| SecurityHistoryExpansionError::Persist(error.to_string()))?;
    }
    let payload = serde_json::to_vec_pretty(value)
        .map_err(|error| SecurityHistoryExpansionError::Persist(error.to_string()))?;
    fs::write(path, payload)
        .map_err(|error| SecurityHistoryExpansionError::Persist(error.to_string()))
}

fn resolve_runtime_root(request: &SecurityHistoryExpansionRequest) -> PathBuf {
    request
        .history_runtime_root
        .as_ref()
        .map(|value| PathBuf::from(value.trim()))
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(default_runtime_root)
}

fn default_runtime_root() -> PathBuf {
    std::env::var("EXCEL_SKILL_RUNTIME_DB")
        .ok()
        .map(PathBuf::from)
        .and_then(|path| path.parent().map(|value| value.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from(".excel_skill_runtime"))
}

fn dedup_sorted_strings(values: &[String]) -> Vec<String> {
    let mut normalized = values
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn dedup_sorted_horizons(values: &[usize]) -> Vec<usize> {
    let mut normalized = values.to_vec();
    normalized.sort_unstable();
    normalized.dedup();
    normalized
}

fn default_coverage_tier() -> String {
    "standardized_ready".to_string()
}

fn default_shadow_readiness_hint() -> String {
    "shadow_coverage_ready".to_string()
}

fn default_champion_readiness_hint() -> String {
    "champion_coverage_ready".to_string()
}

pub fn sanitize_identifier(raw: &str) -> String {
    raw.chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch,
            _ => '_',
        })
        .collect()
}
