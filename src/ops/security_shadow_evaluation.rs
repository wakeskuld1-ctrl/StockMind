use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_history_expansion::{
    SecurityHistoryExpansionDocument, SecurityHistoryExpansionError,
    load_security_history_expansion_document,
};
use crate::ops::stock::security_scorecard_model_registry::SecurityScorecardModelRegistry;

// 2026-04-11 CST: Add a governed shadow-evaluation request contract, because P5
// needs one explicit review step between raw candidate metrics and any promotion
// decision.
// Purpose: freeze the coverage and readiness evidence that supports a shadow-grade move.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityShadowEvaluationRequest {
    pub created_at: String,
    #[serde(default)]
    pub shadow_runtime_root: Option<String>,
    pub market_scope: String,
    pub instrument_scope: String,
    #[serde(default)]
    pub instrument_subscope: Option<String>,
    pub model_registry_path: String,
    #[serde(default)]
    pub comparison_model_registry_paths: Vec<String>,
    #[serde(default)]
    pub history_expansion_paths: Vec<String>,
    #[serde(default)]
    pub prior_shadow_evaluation_paths: Vec<String>,
    #[serde(default)]
    pub evaluation_notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityShadowEvaluationDocument {
    pub shadow_evaluation_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub created_at: String,
    pub market_scope: String,
    pub instrument_scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument_subscope: Option<String>,
    pub model_registry_ref: String,
    pub sample_readiness_status: String,
    pub class_balance_status: String,
    pub path_event_coverage_status: String,
    pub proxy_coverage_status: String,
    pub production_readiness: String,
    pub recommended_model_grade: String,
    // 2026-04-11 CST: Add repeated-observation governance fields, because P6
    // needs champion promotion to depend on durable shadow behavior rather than
    // a single green snapshot.
    // Purpose: preserve explicit count, consistency, and blockers for approval and audit consumers.
    #[serde(default)]
    pub shadow_observation_count: usize,
    #[serde(default = "default_shadow_consistency_status")]
    pub shadow_consistency_status: String,
    #[serde(default)]
    pub shadow_window_count: usize,
    #[serde(default = "default_oot_stability_status")]
    pub oot_stability_status: String,
    #[serde(default = "default_window_consistency_status")]
    pub window_consistency_status: String,
    #[serde(default)]
    pub promotion_blockers: Vec<String>,
    #[serde(default)]
    pub promotion_evidence_notes: Vec<String>,
    pub evaluation_notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityShadowEvaluationResult {
    pub shadow_evaluation: SecurityShadowEvaluationDocument,
    pub shadow_evaluation_path: String,
}

#[derive(Debug, Error)]
pub enum SecurityShadowEvaluationError {
    #[error("security shadow evaluation build failed: {0}")]
    Build(String),
    #[error("security shadow evaluation persist failed: {0}")]
    Persist(String),
    #[error("security shadow evaluation history expansion failed: {0}")]
    HistoryExpansion(#[from] SecurityHistoryExpansionError),
}

// 2026-04-11 CST: Persist one governed shadow-evaluation document, because P5
// promotion decisions need a durable assessment layer that summarizes readiness
// and coverage rather than inspecting raw registry JSON every time.
// Purpose: give approval and future audits a stable intermediate governance artifact.
pub fn security_shadow_evaluation(
    request: &SecurityShadowEvaluationRequest,
) -> Result<SecurityShadowEvaluationResult, SecurityShadowEvaluationError> {
    validate_request(request)?;

    let registry = load_model_registry(&request.model_registry_path)?;
    let history_expansions = request
        .history_expansion_paths
        .iter()
        .map(|path| load_security_history_expansion_document(path))
        .collect::<Result<Vec<_>, _>>()?;
    let prior_shadow_evaluations = request
        .prior_shadow_evaluation_paths
        .iter()
        .map(|path| load_shadow_evaluation_document(path))
        .collect::<Result<Vec<_>, _>>()?;
    let comparison_registries = request
        .comparison_model_registry_paths
        .iter()
        .map(|path| load_model_registry(path))
        .collect::<Result<Vec<_>, _>>()?;
    let document = build_shadow_evaluation_document(
        request,
        &registry,
        &comparison_registries,
        &history_expansions,
        &prior_shadow_evaluations,
    );

    let runtime_root = resolve_runtime_root(request);
    let path = runtime_root.join("shadow_evaluations").join(format!(
        "{}.json",
        sanitize_identifier(&document.shadow_evaluation_id)
    ));
    persist_json(&path, &document)?;

    Ok(SecurityShadowEvaluationResult {
        shadow_evaluation: document,
        shadow_evaluation_path: path.to_string_lossy().to_string(),
    })
}

fn build_shadow_evaluation_document(
    request: &SecurityShadowEvaluationRequest,
    registry: &SecurityScorecardModelRegistry,
    comparison_registries: &[SecurityScorecardModelRegistry],
    history_expansions: &[SecurityHistoryExpansionDocument],
    prior_shadow_evaluations: &[SecurityShadowEvaluationDocument],
) -> SecurityShadowEvaluationDocument {
    // 2026-04-11 CST: Read readiness fields from the existing registry metrics,
    // because P5 should extend the current governed training vocabulary instead of
    // inventing a second readiness dialect.
    // Purpose: keep promotion rules consistent with the training readiness panel.
    let sample_readiness_status = metric_string(
        &registry.metrics_summary_json,
        &["readiness_assessment", "minimum_sample_status"],
        "unknown",
    );
    let class_balance_status = metric_string(
        &registry.metrics_summary_json,
        &["readiness_assessment", "class_balance_status"],
        "unknown",
    );
    let path_event_coverage_status = metric_string(
        &registry.metrics_summary_json,
        &["readiness_assessment", "path_event_coverage_status"],
        "unknown",
    );
    let production_readiness = metric_string(
        &registry.metrics_summary_json,
        &["readiness_assessment", "production_readiness"],
        "research_candidate_only",
    );
    let proxy_coverage_status = derive_proxy_coverage_status(
        history_expansions,
        request.market_scope.trim(),
        request.instrument_scope.trim(),
        request.instrument_subscope.as_deref(),
    );
    let shadow_observation_count = 1 + prior_shadow_evaluations
        .iter()
        .filter(|document| {
            shadow_matches(
                document,
                request.market_scope.trim(),
                request.instrument_scope.trim(),
                request.instrument_subscope.as_deref(),
            )
        })
        .count();
    let shadow_consistency_status =
        derive_shadow_consistency_status(prior_shadow_evaluations, shadow_observation_count);
    let shadow_window_count = derive_shadow_window_count(
        registry,
        comparison_registries,
        request.instrument_subscope.as_deref(),
    );
    let oot_stability_status = derive_oot_stability_status(
        registry,
        comparison_registries,
        request.instrument_subscope.as_deref(),
    );
    let window_consistency_status = derive_window_consistency_status(
        registry,
        comparison_registries,
        request.instrument_subscope.as_deref(),
    );
    let promotion_evidence_notes = derive_promotion_evidence_notes(
        shadow_window_count,
        &oot_stability_status,
        &window_consistency_status,
    );
    let promotion_blockers = derive_promotion_blockers(
        &sample_readiness_status,
        &class_balance_status,
        &path_event_coverage_status,
        &proxy_coverage_status,
        &production_readiness,
        shadow_observation_count,
        &shadow_consistency_status,
    );
    let recommended_model_grade = derive_recommended_model_grade(
        &production_readiness,
        &proxy_coverage_status,
        shadow_observation_count,
        &shadow_consistency_status,
        shadow_window_count,
        &oot_stability_status,
        &window_consistency_status,
        &promotion_blockers,
        &promotion_evidence_notes,
    );

    SecurityShadowEvaluationDocument {
        shadow_evaluation_id: format!(
            "shadow-evaluation:{}:{}:{}:{}:v1",
            request.market_scope.trim(),
            request.instrument_scope.trim(),
            request
                .instrument_subscope
                .as_deref()
                .map(sanitize_identifier)
                .unwrap_or_else(|| "none".to_string()),
            request.created_at.trim()
        ),
        contract_version: "security_shadow_evaluation.v1".to_string(),
        document_type: "security_shadow_evaluation".to_string(),
        created_at: request.created_at.trim().to_string(),
        market_scope: request.market_scope.trim().to_string(),
        instrument_scope: request.instrument_scope.trim().to_string(),
        instrument_subscope: request
            .instrument_subscope
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        model_registry_ref: registry.registry_id.clone(),
        sample_readiness_status,
        class_balance_status,
        path_event_coverage_status,
        proxy_coverage_status,
        production_readiness,
        recommended_model_grade,
        shadow_observation_count,
        shadow_consistency_status,
        shadow_window_count,
        oot_stability_status,
        window_consistency_status,
        promotion_blockers,
        promotion_evidence_notes,
        evaluation_notes: request.evaluation_notes.clone(),
    }
}

fn derive_recommended_model_grade(
    production_readiness: &str,
    proxy_coverage_status: &str,
    shadow_observation_count: usize,
    shadow_consistency_status: &str,
    shadow_window_count: usize,
    oot_stability_status: &str,
    window_consistency_status: &str,
    promotion_blockers: &[String],
    promotion_evidence_notes: &[String],
) -> String {
    if promotion_blockers.is_empty()
        && proxy_coverage_status == "history_coverage_ready"
        && matches!(
            production_readiness,
            "champion_ready" | "champion_candidate_ready"
        )
        && shadow_observation_count >= 3
        && shadow_consistency_status == "shadow_consistent"
        && shadow_window_count >= 3
        && oot_stability_status == "oot_stable"
        && window_consistency_status == "window_consistent"
        && promotion_evidence_notes.is_empty()
    {
        return "champion".to_string();
    }
    if proxy_coverage_status == "history_coverage_ready"
        && matches!(
            production_readiness,
            "shadow_ready" | "shadow_candidate_ready" | "champion_candidate_ready"
        )
    {
        return "shadow".to_string();
    }
    "candidate".to_string()
}

fn derive_shadow_window_count(
    registry: &SecurityScorecardModelRegistry,
    comparison_registries: &[SecurityScorecardModelRegistry],
    instrument_subscope: Option<&str>,
) -> usize {
    1 + comparison_registries
        .iter()
        .filter(|comparison| registry_scope_matches(registry, comparison, instrument_subscope))
        .count()
}

fn derive_oot_stability_status(
    registry: &SecurityScorecardModelRegistry,
    comparison_registries: &[SecurityScorecardModelRegistry],
    instrument_subscope: Option<&str>,
) -> String {
    let windows =
        collect_window_metric_snapshots(registry, comparison_registries, instrument_subscope);
    if windows.len() < 3 {
        return "oot_thin".to_string();
    }
    if windows.iter().all(|snapshot| snapshot.is_stable) {
        "oot_stable".to_string()
    } else {
        "oot_unstable".to_string()
    }
}

fn derive_window_consistency_status(
    registry: &SecurityScorecardModelRegistry,
    comparison_registries: &[SecurityScorecardModelRegistry],
    instrument_subscope: Option<&str>,
) -> String {
    let windows =
        collect_window_metric_snapshots(registry, comparison_registries, instrument_subscope);
    if windows.len() < 3 {
        return "window_observation_thin".to_string();
    }
    if windows.iter().all(|snapshot| snapshot.is_stable) {
        "window_consistent".to_string()
    } else {
        "window_inconsistent".to_string()
    }
}

fn derive_promotion_evidence_notes(
    shadow_window_count: usize,
    oot_stability_status: &str,
    window_consistency_status: &str,
) -> Vec<String> {
    let mut notes = Vec::new();
    if shadow_window_count < 3 {
        notes.push("champion gate requires at least two stable comparison windows".to_string());
    }
    if oot_stability_status == "oot_unstable" {
        notes.push("comparison windows did not keep OOT metrics stable enough".to_string());
    }
    if window_consistency_status == "window_inconsistent" {
        notes.push(
            "comparison windows are not consistent enough for champion promotion".to_string(),
        );
    }
    notes
}

fn history_matches(
    history: &SecurityHistoryExpansionDocument,
    market_scope: &str,
    instrument_scope: &str,
    instrument_subscope: Option<&str>,
) -> bool {
    if history.market_scope != market_scope || history.instrument_scope != instrument_scope {
        return false;
    }
    match (history.instrument_subscope.as_deref(), instrument_subscope) {
        (Some(left), Some(right)) => left == right,
        (None, None) => true,
        _ => false,
    }
}

fn shadow_matches(
    shadow: &SecurityShadowEvaluationDocument,
    market_scope: &str,
    instrument_scope: &str,
    instrument_subscope: Option<&str>,
) -> bool {
    if shadow.market_scope != market_scope || shadow.instrument_scope != instrument_scope {
        return false;
    }
    match (shadow.instrument_subscope.as_deref(), instrument_subscope) {
        (Some(left), Some(right)) => left == right,
        (None, None) => true,
        _ => false,
    }
}

fn derive_proxy_coverage_status(
    history_expansions: &[SecurityHistoryExpansionDocument],
    market_scope: &str,
    instrument_scope: &str,
    instrument_subscope: Option<&str>,
) -> String {
    if history_expansions.iter().any(|document| {
        history_matches(
            document,
            market_scope,
            instrument_scope,
            instrument_subscope,
        ) && document.coverage_summary.shadow_readiness_hint == "shadow_coverage_ready"
    }) {
        "history_coverage_ready".to_string()
    } else {
        "history_coverage_missing".to_string()
    }
}

// 2026-04-11 CST: Derive a stable shadow-consistency status, because P6 needs
// repeated governed observations to mean something stronger than a raw count.
// Purpose: let champion promotion distinguish repeated stable shadow behavior from thin history.
fn derive_shadow_consistency_status(
    prior_shadow_evaluations: &[SecurityShadowEvaluationDocument],
    shadow_observation_count: usize,
) -> String {
    if shadow_observation_count < 3 {
        return "shadow_observation_thin".to_string();
    }
    if prior_shadow_evaluations
        .iter()
        .all(|document| document.shadow_consistency_status == "shadow_consistent")
    {
        "shadow_consistent".to_string()
    } else {
        "shadow_inconsistent".to_string()
    }
}

fn derive_promotion_blockers(
    sample_readiness_status: &str,
    class_balance_status: &str,
    path_event_coverage_status: &str,
    proxy_coverage_status: &str,
    production_readiness: &str,
    shadow_observation_count: usize,
    shadow_consistency_status: &str,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if sample_readiness_status != "sample_ready" {
        blockers.push("sample readiness is not strong enough for promotion".to_string());
    }
    if class_balance_status != "class_balance_ready" {
        blockers.push("class balance is not stable enough for promotion".to_string());
    }
    if path_event_coverage_status != "path_event_ready" {
        blockers.push("path-event coverage is not ready for promotion".to_string());
    }
    if proxy_coverage_status != "history_coverage_ready" {
        blockers.push("history proxy coverage is missing for promotion".to_string());
    }
    if matches!(
        production_readiness,
        "champion_ready" | "champion_candidate_ready"
    ) && shadow_observation_count < 3
    {
        blockers
            .push("champion gate requires at least three governed shadow observations".to_string());
    }
    if matches!(
        production_readiness,
        "champion_ready" | "champion_candidate_ready"
    ) && shadow_consistency_status != "shadow_consistent"
    {
        blockers.push("shadow consistency is not stable enough for champion promotion".to_string());
    }
    blockers
}

#[derive(Debug, Clone, Copy)]
struct WindowMetricSnapshot {
    is_stable: bool,
}

fn collect_window_metric_snapshots(
    registry: &SecurityScorecardModelRegistry,
    comparison_registries: &[SecurityScorecardModelRegistry],
    instrument_subscope: Option<&str>,
) -> Vec<WindowMetricSnapshot> {
    let mut windows = vec![build_window_metric_snapshot(registry)];
    windows.extend(
        comparison_registries
            .iter()
            .filter(|comparison| registry_scope_matches(registry, comparison, instrument_subscope))
            .map(build_window_metric_snapshot),
    );
    windows
}

fn build_window_metric_snapshot(registry: &SecurityScorecardModelRegistry) -> WindowMetricSnapshot {
    let auc = metric_f64(&registry.metrics_summary_json, &["test", "auc"]).unwrap_or(0.0);
    let accuracy = metric_f64(&registry.metrics_summary_json, &["test", "accuracy"]).unwrap_or(0.0);
    WindowMetricSnapshot {
        is_stable: auc >= 0.75 && accuracy >= 0.70,
    }
}

fn registry_scope_matches(
    registry: &SecurityScorecardModelRegistry,
    comparison: &SecurityScorecardModelRegistry,
    instrument_subscope: Option<&str>,
) -> bool {
    registry.market_scope == comparison.market_scope
        && registry.instrument_scope == comparison.instrument_scope
        && registry.horizon_days == comparison.horizon_days
        && registry.target_head == comparison.target_head
        && match (
            comparison.instrument_subscope.as_deref(),
            instrument_subscope,
        ) {
            (Some(left), Some(right)) => left == right,
            (None, None) => true,
            _ => false,
        }
}

fn metric_string(root: &serde_json::Value, path: &[&str], fallback: &str) -> String {
    let mut current = root;
    for segment in path {
        let Some(next) = current.get(*segment) else {
            return fallback.to_string();
        };
        current = next;
    }
    current.as_str().unwrap_or(fallback).to_string()
}

fn metric_f64(root: &serde_json::Value, path: &[&str]) -> Option<f64> {
    let mut current = root;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_f64()
}

fn validate_request(
    request: &SecurityShadowEvaluationRequest,
) -> Result<(), SecurityShadowEvaluationError> {
    for (field_name, field_value) in [
        ("created_at", request.created_at.trim()),
        ("market_scope", request.market_scope.trim()),
        ("instrument_scope", request.instrument_scope.trim()),
        ("model_registry_path", request.model_registry_path.trim()),
    ] {
        if field_value.is_empty() {
            return Err(SecurityShadowEvaluationError::Build(format!(
                "{field_name} cannot be empty"
            )));
        }
    }
    Ok(())
}

fn load_model_registry(
    path: &str,
) -> Result<SecurityScorecardModelRegistry, SecurityShadowEvaluationError> {
    let payload = fs::read(path).map_err(|error| {
        SecurityShadowEvaluationError::Persist(format!(
            "failed to read model registry `{path}`: {error}"
        ))
    })?;
    serde_json::from_slice::<SecurityScorecardModelRegistry>(&payload).map_err(|error| {
        SecurityShadowEvaluationError::Build(format!(
            "failed to parse model registry `{path}`: {error}"
        ))
    })
}

fn persist_json<T: Serialize>(path: &Path, value: &T) -> Result<(), SecurityShadowEvaluationError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| SecurityShadowEvaluationError::Persist(error.to_string()))?;
    }
    let payload = serde_json::to_vec_pretty(value)
        .map_err(|error| SecurityShadowEvaluationError::Persist(error.to_string()))?;
    fs::write(path, payload)
        .map_err(|error| SecurityShadowEvaluationError::Persist(error.to_string()))
}

fn resolve_runtime_root(request: &SecurityShadowEvaluationRequest) -> PathBuf {
    request
        .shadow_runtime_root
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

fn load_shadow_evaluation_document(
    path: &str,
) -> Result<SecurityShadowEvaluationDocument, SecurityShadowEvaluationError> {
    let payload = fs::read(path).map_err(|error| {
        SecurityShadowEvaluationError::Persist(format!(
            "failed to read prior shadow evaluation `{path}`: {error}"
        ))
    })?;
    serde_json::from_slice::<SecurityShadowEvaluationDocument>(&payload).map_err(|error| {
        SecurityShadowEvaluationError::Build(format!(
            "failed to parse prior shadow evaluation `{path}`: {error}"
        ))
    })
}

fn default_shadow_consistency_status() -> String {
    "shadow_untracked".to_string()
}

fn default_oot_stability_status() -> String {
    "oot_untracked".to_string()
}

fn default_window_consistency_status() -> String {
    "window_untracked".to_string()
}

fn sanitize_identifier(raw: &str) -> String {
    raw.chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch,
            _ => '_',
        })
        .collect()
}
