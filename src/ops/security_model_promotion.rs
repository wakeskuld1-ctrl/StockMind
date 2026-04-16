use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_scorecard_model_registry::SecurityScorecardModelRegistry;
use crate::ops::stock::security_shadow_evaluation::SecurityShadowEvaluationDocument;

// 2026-04-11 CST: Add a governed model-promotion request contract, because P5
// needs grade transitions to become explicit persisted decisions instead of
// remaining hidden in registry mutations.
// Purpose: separate promotion judgment from raw training/refit mechanics.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityModelPromotionRequest {
    pub created_at: String,
    #[serde(default)]
    pub promotion_runtime_root: Option<String>,
    pub model_registry_path: String,
    pub shadow_evaluation_path: String,
    pub requested_model_grade: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityModelPromotionDocument {
    pub promotion_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub created_at: String,
    pub model_registry_ref: String,
    pub shadow_evaluation_ref: String,
    pub requested_model_grade: String,
    pub approved_model_grade: String,
    pub promotion_decision: String,
    pub promotion_reason: String,
    #[serde(default)]
    pub shadow_observation_count: usize,
    #[serde(default)]
    pub shadow_consistency_status: String,
    #[serde(default)]
    pub shadow_window_count: usize,
    #[serde(default)]
    pub oot_stability_status: String,
    #[serde(default)]
    pub window_consistency_status: String,
    #[serde(default)]
    pub promotion_blockers: Vec<String>,
    #[serde(default)]
    pub promotion_evidence_notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityModelPromotionResult {
    pub promotion: SecurityModelPromotionDocument,
    pub promotion_path: String,
}

#[derive(Debug, Error)]
pub enum SecurityModelPromotionError {
    #[error("security model promotion build failed: {0}")]
    Build(String),
    #[error("security model promotion persist failed: {0}")]
    Persist(String),
}

// 2026-04-11 CST: Persist one governed promotion decision, because P5 needs
// approval and package consumers to read an auditable grade decision rather than
// infer promotion state from loose text notes.
// Purpose: make candidate/shadow/champion transitions explicit and replayable.
pub fn security_model_promotion(
    request: &SecurityModelPromotionRequest,
) -> Result<SecurityModelPromotionResult, SecurityModelPromotionError> {
    validate_request(request)?;

    let registry = load_model_registry(&request.model_registry_path)?;
    let shadow_evaluation = load_shadow_evaluation(&request.shadow_evaluation_path)?;
    let document = build_promotion_document(request, &registry, &shadow_evaluation);

    let runtime_root = resolve_runtime_root(request);
    let path = runtime_root.join("model_promotions").join(format!(
        "{}.json",
        sanitize_identifier(&document.promotion_id)
    ));
    persist_json(&path, &document)?;

    Ok(SecurityModelPromotionResult {
        promotion: document,
        promotion_path: path.to_string_lossy().to_string(),
    })
}

fn build_promotion_document(
    request: &SecurityModelPromotionRequest,
    registry: &SecurityScorecardModelRegistry,
    shadow_evaluation: &SecurityShadowEvaluationDocument,
) -> SecurityModelPromotionDocument {
    // 2026-04-11 CST: Keep promotion judgment rule-based and explicit, because
    // P5 governance should remain reviewable instead of turning grade transitions
    // into another hidden heuristic.
    // Purpose: make every promotion outcome explainable in one stable field pair.
    let (approved_model_grade, promotion_decision, promotion_reason) =
        match request.requested_model_grade.trim() {
            "champion"
                if shadow_evaluation.recommended_model_grade == "champion"
                    && shadow_evaluation.shadow_observation_count >= 3
                    && shadow_evaluation.shadow_consistency_status == "shadow_consistent"
                    && shadow_evaluation.shadow_window_count >= 3
                    && shadow_evaluation.oot_stability_status == "oot_stable"
                    && shadow_evaluation.window_consistency_status == "window_consistent"
                    && shadow_evaluation.promotion_blockers.is_empty()
                    && shadow_evaluation.promotion_evidence_notes.is_empty() =>
            {
                (
                    "champion".to_string(),
                    "promote_to_champion".to_string(),
                    "shadow evaluation satisfied champion threshold".to_string(),
                )
            }
            "shadow"
                if matches!(
                    shadow_evaluation.recommended_model_grade.as_str(),
                    "shadow" | "champion"
                ) =>
            {
                (
                    "shadow".to_string(),
                    "promote_to_shadow".to_string(),
                    "shadow evaluation approved shadow-grade usage".to_string(),
                )
            }
            _ => (
                registry.model_grade.clone(),
                "retain_current_grade".to_string(),
                "shadow evaluation did not justify the requested promotion".to_string(),
            ),
        };

    SecurityModelPromotionDocument {
        promotion_id: format!(
            "model-promotion:{}:{}:{}:v1",
            registry.registry_id,
            request.requested_model_grade.trim(),
            request.created_at.trim()
        ),
        contract_version: "security_model_promotion.v1".to_string(),
        document_type: "security_model_promotion".to_string(),
        created_at: request.created_at.trim().to_string(),
        model_registry_ref: registry.registry_id.clone(),
        shadow_evaluation_ref: shadow_evaluation.shadow_evaluation_id.clone(),
        requested_model_grade: request.requested_model_grade.trim().to_string(),
        approved_model_grade,
        promotion_decision,
        promotion_reason,
        shadow_observation_count: shadow_evaluation.shadow_observation_count,
        shadow_consistency_status: shadow_evaluation.shadow_consistency_status.clone(),
        shadow_window_count: shadow_evaluation.shadow_window_count,
        oot_stability_status: shadow_evaluation.oot_stability_status.clone(),
        window_consistency_status: shadow_evaluation.window_consistency_status.clone(),
        promotion_blockers: shadow_evaluation.promotion_blockers.clone(),
        promotion_evidence_notes: shadow_evaluation.promotion_evidence_notes.clone(),
    }
}

fn validate_request(
    request: &SecurityModelPromotionRequest,
) -> Result<(), SecurityModelPromotionError> {
    for (field_name, field_value) in [
        ("created_at", request.created_at.trim()),
        ("model_registry_path", request.model_registry_path.trim()),
        (
            "shadow_evaluation_path",
            request.shadow_evaluation_path.trim(),
        ),
        (
            "requested_model_grade",
            request.requested_model_grade.trim(),
        ),
    ] {
        if field_value.is_empty() {
            return Err(SecurityModelPromotionError::Build(format!(
                "{field_name} cannot be empty"
            )));
        }
    }
    Ok(())
}

fn load_model_registry(
    path: &str,
) -> Result<SecurityScorecardModelRegistry, SecurityModelPromotionError> {
    let payload = fs::read(path).map_err(|error| {
        SecurityModelPromotionError::Persist(format!(
            "failed to read model registry `{path}`: {error}"
        ))
    })?;
    serde_json::from_slice::<SecurityScorecardModelRegistry>(&payload).map_err(|error| {
        SecurityModelPromotionError::Build(format!(
            "failed to parse model registry `{path}`: {error}"
        ))
    })
}

fn load_shadow_evaluation(
    path: &str,
) -> Result<SecurityShadowEvaluationDocument, SecurityModelPromotionError> {
    let payload = fs::read(path).map_err(|error| {
        SecurityModelPromotionError::Persist(format!(
            "failed to read shadow evaluation `{path}`: {error}"
        ))
    })?;
    serde_json::from_slice::<SecurityShadowEvaluationDocument>(&payload).map_err(|error| {
        SecurityModelPromotionError::Build(format!(
            "failed to parse shadow evaluation `{path}`: {error}"
        ))
    })
}

fn persist_json<T: Serialize>(path: &Path, value: &T) -> Result<(), SecurityModelPromotionError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| SecurityModelPromotionError::Persist(error.to_string()))?;
    }
    let payload = serde_json::to_vec_pretty(value)
        .map_err(|error| SecurityModelPromotionError::Persist(error.to_string()))?;
    fs::write(path, payload)
        .map_err(|error| SecurityModelPromotionError::Persist(error.to_string()))
}

fn resolve_runtime_root(request: &SecurityModelPromotionRequest) -> PathBuf {
    request
        .promotion_runtime_root
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

fn sanitize_identifier(raw: &str) -> String {
    raw.chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch,
            _ => '_',
        })
        .collect()
}
