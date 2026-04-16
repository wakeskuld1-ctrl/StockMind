use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

use crate::ops::stock::security_scorecard_model_registry::{
    SecurityScorecardModelRegistry, sanitize_identifier,
};
use crate::ops::stock::security_scorecard_training::{
    SecurityScorecardTrainingError, SecurityScorecardTrainingRequest,
    security_scorecard_training,
};

// 2026-04-12 CST: Add a governed direction-first orchestration request, because
// the seven-hour training round now needs one stable tool contract instead of
// shell-only loops that are hard to resume or hand off.
// Purpose: freeze the candidate list, runtime root, and survivor budget for staged ranking.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityDirectionFirstTrainingRunRequest {
    pub created_at: String,
    #[serde(default)]
    pub direction_first_runtime_root: Option<String>,
    #[serde(default = "default_survivor_count")]
    pub survivor_count: usize,
    pub candidate_pairs: Vec<SecurityDirectionFirstTrainingCandidatePair>,
}

// 2026-04-12 CST: Add per-candidate pair inputs, because the thin orchestration
// layer must support both reranking existing registries and launching fresh
// training without rewriting the training mainline.
// Purpose: let one request mix governed registry reuse with minimal training bridges.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityDirectionFirstTrainingCandidatePair {
    pub candidate_id: String,
    pub market_pool: String,
    pub horizon_days: usize,
    #[serde(default)]
    pub direction_model_registry_path: Option<String>,
    #[serde(default)]
    pub return_model_registry_path: Option<String>,
    #[serde(default)]
    pub direction_training_request: Option<SecurityScorecardTrainingRequest>,
    #[serde(default)]
    pub return_training_request: Option<SecurityScorecardTrainingRequest>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityDirectionFirstTrainingRunResult {
    pub stage_summary: SecurityDirectionFirstTrainingStageSummary,
    pub stage_summary_path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityDirectionFirstTrainingStageSummary {
    pub run_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub created_at: String,
    pub survivor_count: usize,
    pub total_candidates: usize,
    pub ranking_policy: SecurityDirectionFirstRankingPolicy,
    pub survivors: Vec<SecurityDirectionFirstRankedCandidate>,
    pub eliminated: Vec<SecurityDirectionFirstRankedCandidate>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityDirectionFirstRankingPolicy {
    pub selection_strategy: String,
    pub primary_metric: String,
    pub secondary_metric: String,
    pub tertiary_metric: String,
    pub quaternary_metric: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityDirectionFirstRankedCandidate {
    pub candidate_id: String,
    pub market_pool: String,
    pub horizon_days: usize,
    pub direction_model_registry_path: String,
    pub return_model_registry_path: String,
    pub direction_metrics: Value,
    pub return_metrics: Value,
    pub direction_production_readiness: String,
    pub return_production_readiness: String,
}

#[derive(Debug, Error)]
pub enum SecurityDirectionFirstTrainingRunError {
    #[error("security direction-first training run build failed: {0}")]
    Build(String),
    #[error("security direction-first training run persist failed: {0}")]
    Persist(String),
    #[error("security direction-first training run training failed: {0}")]
    Training(#[from] SecurityScorecardTrainingError),
}

#[derive(Debug, Clone)]
struct ResolvedCandidatePair {
    summary: SecurityDirectionFirstRankedCandidate,
    score: DirectionFirstRankingScore,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct DirectionFirstRankingScore {
    direction_test_accuracy: f64,
    direction_test_auc: f64,
    return_test_directional_hit_rate: f64,
    return_test_rmse_improvement_vs_baseline: f64,
}

// 2026-04-12 CST: Add the thin direction-first orchestration entry point, because
// the user wants long-running governed training progress without replacing the
// existing training -> refit -> shadow -> promotion chain.
// Purpose: reuse governed registries, rank by direction first, and persist one resumable stage summary.
pub fn security_direction_first_training_run(
    request: &SecurityDirectionFirstTrainingRunRequest,
) -> Result<SecurityDirectionFirstTrainingRunResult, SecurityDirectionFirstTrainingRunError> {
    validate_request(request)?;

    let resolved_candidates = request
        .candidate_pairs
        .iter()
        .map(resolve_candidate_pair)
        .collect::<Result<Vec<_>, _>>()?;
    let ranking_policy = default_ranking_policy();
    let mut ranked_candidates = resolved_candidates;
    ranked_candidates.sort_by(|left, right| compare_scores(&right.score, &left.score));

    let survivor_count = request.survivor_count.min(ranked_candidates.len());
    let survivors = ranked_candidates
        .iter()
        .take(survivor_count)
        .map(|entry| entry.summary.clone())
        .collect::<Vec<_>>();
    let eliminated = ranked_candidates
        .iter()
        .skip(survivor_count)
        .map(|entry| entry.summary.clone())
        .collect::<Vec<_>>();

    let stage_summary = SecurityDirectionFirstTrainingStageSummary {
        run_id: format!(
            "direction-first-training-run:{}:{}:v1",
            sanitize_identifier(request.created_at.trim()),
            ranked_candidates.len()
        ),
        contract_version: "security_direction_first_training_run.v1".to_string(),
        document_type: "security_direction_first_training_run".to_string(),
        created_at: request.created_at.trim().to_string(),
        survivor_count,
        total_candidates: ranked_candidates.len(),
        ranking_policy,
        survivors,
        eliminated,
    };

    let runtime_root = resolve_runtime_root(request);
    let stage_summary_path = runtime_root.join("direction_first_training_runs").join(format!(
        "{}.json",
        sanitize_identifier(&stage_summary.run_id)
    ));
    persist_json(&stage_summary_path, &stage_summary)?;

    Ok(SecurityDirectionFirstTrainingRunResult {
        stage_summary,
        stage_summary_path: stage_summary_path.to_string_lossy().to_string(),
    })
}

fn validate_request(
    request: &SecurityDirectionFirstTrainingRunRequest,
) -> Result<(), SecurityDirectionFirstTrainingRunError> {
    for (field_name, field_value) in [("created_at", request.created_at.trim())] {
        if field_value.is_empty() {
            return Err(SecurityDirectionFirstTrainingRunError::Build(format!(
                "{field_name} cannot be empty"
            )));
        }
    }
    if request.survivor_count == 0 {
        return Err(SecurityDirectionFirstTrainingRunError::Build(
            "survivor_count must be greater than 0".to_string(),
        ));
    }
    if request.candidate_pairs.is_empty() {
        return Err(SecurityDirectionFirstTrainingRunError::Build(
            "candidate_pairs cannot be empty".to_string(),
        ));
    }
    for candidate in &request.candidate_pairs {
        validate_candidate_source(
            &candidate.candidate_id,
            "direction",
            candidate.direction_model_registry_path.as_deref(),
            candidate.direction_training_request.as_ref(),
        )?;
        validate_candidate_source(
            &candidate.candidate_id,
            "return",
            candidate.return_model_registry_path.as_deref(),
            candidate.return_training_request.as_ref(),
        )?;
        if candidate.market_pool.trim().is_empty() {
            return Err(SecurityDirectionFirstTrainingRunError::Build(format!(
                "candidate `{}` market_pool cannot be empty",
                candidate.candidate_id
            )));
        }
        if candidate.horizon_days == 0 {
            return Err(SecurityDirectionFirstTrainingRunError::Build(format!(
                "candidate `{}` horizon_days must be greater than 0",
                candidate.candidate_id
            )));
        }
    }
    Ok(())
}

fn validate_candidate_source(
    candidate_id: &str,
    head_kind: &str,
    registry_path: Option<&str>,
    training_request: Option<&SecurityScorecardTrainingRequest>,
) -> Result<(), SecurityDirectionFirstTrainingRunError> {
    let has_registry_path = registry_path
        .map(str::trim)
        .map(|value| !value.is_empty())
        .unwrap_or(false);
    let has_training_request = training_request.is_some();
    match (has_registry_path, has_training_request) {
        (true, false) | (false, true) => Ok(()),
        (false, false) => Err(SecurityDirectionFirstTrainingRunError::Build(format!(
            "candidate `{candidate_id}` must provide either {head_kind}_model_registry_path or {head_kind}_training_request"
        ))),
        (true, true) => Err(SecurityDirectionFirstTrainingRunError::Build(format!(
            "candidate `{candidate_id}` cannot provide both {head_kind}_model_registry_path and {head_kind}_training_request"
        ))),
    }
}

fn resolve_candidate_pair(
    candidate: &SecurityDirectionFirstTrainingCandidatePair,
) -> Result<ResolvedCandidatePair, SecurityDirectionFirstTrainingRunError> {
    let (direction_registry, direction_registry_path) = resolve_registry_source(
        candidate.direction_model_registry_path.as_deref(),
        candidate.direction_training_request.as_ref(),
        "direction",
    )?;
    let (return_registry, return_registry_path) = resolve_registry_source(
        candidate.return_model_registry_path.as_deref(),
        candidate.return_training_request.as_ref(),
        "return",
    )?;

    let direction_metrics = direction_registry
        .metrics_summary_json
        .get("test")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let return_metrics = return_registry
        .metrics_summary_json
        .get("test")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let score = DirectionFirstRankingScore {
        direction_test_accuracy: metric_f64(&direction_metrics, &["accuracy"]),
        direction_test_auc: metric_f64(&direction_metrics, &["auc"]),
        return_test_directional_hit_rate: metric_f64(&return_metrics, &["directional_hit_rate"]),
        return_test_rmse_improvement_vs_baseline: metric_f64(
            &return_metrics,
            &["rmse_improvement_vs_baseline"],
        ),
    };

    Ok(ResolvedCandidatePair {
        summary: SecurityDirectionFirstRankedCandidate {
            candidate_id: candidate.candidate_id.trim().to_string(),
            market_pool: candidate.market_pool.trim().to_string(),
            horizon_days: candidate.horizon_days,
            direction_model_registry_path: direction_registry_path,
            return_model_registry_path: return_registry_path,
            direction_metrics,
            return_metrics,
            direction_production_readiness: metric_string(
                &direction_registry.metrics_summary_json,
                &["readiness_assessment", "production_readiness"],
                "unknown",
            ),
            return_production_readiness: metric_string(
                &return_registry.metrics_summary_json,
                &["readiness_assessment", "production_readiness"],
                "unknown",
            ),
        },
        score,
    })
}

fn resolve_registry_source(
    registry_path: Option<&str>,
    training_request: Option<&SecurityScorecardTrainingRequest>,
    head_kind: &str,
) -> Result<(SecurityScorecardModelRegistry, String), SecurityDirectionFirstTrainingRunError> {
    if let Some(path) = registry_path
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let registry = load_model_registry(path)?;
        return Ok((registry, path.to_string()));
    }
    let Some(training_request) = training_request else {
        return Err(SecurityDirectionFirstTrainingRunError::Build(format!(
            "{head_kind} source is missing"
        )));
    };
    let training_result = security_scorecard_training(training_request)?;
    Ok((
        training_result.model_registry,
        training_result.model_registry_path,
    ))
}

fn load_model_registry(
    path: &str,
) -> Result<SecurityScorecardModelRegistry, SecurityDirectionFirstTrainingRunError> {
    let payload = fs::read(path).map_err(|error| {
        SecurityDirectionFirstTrainingRunError::Persist(format!(
            "failed to read model registry `{path}`: {error}"
        ))
    })?;
    serde_json::from_slice::<SecurityScorecardModelRegistry>(&payload).map_err(|error| {
        SecurityDirectionFirstTrainingRunError::Build(format!(
            "failed to parse model registry `{path}`: {error}"
        ))
    })
}

fn compare_scores(
    left: &DirectionFirstRankingScore,
    right: &DirectionFirstRankingScore,
) -> Ordering {
    compare_desc(left.direction_test_accuracy, right.direction_test_accuracy)
        .then_with(|| compare_desc(left.direction_test_auc, right.direction_test_auc))
        .then_with(|| {
            compare_desc(
                left.return_test_directional_hit_rate,
                right.return_test_directional_hit_rate,
            )
        })
        .then_with(|| {
            compare_desc(
                left.return_test_rmse_improvement_vs_baseline,
                right.return_test_rmse_improvement_vs_baseline,
            )
        })
}

fn compare_desc(left: f64, right: f64) -> Ordering {
    left.total_cmp(&right)
}

fn default_ranking_policy() -> SecurityDirectionFirstRankingPolicy {
    SecurityDirectionFirstRankingPolicy {
        selection_strategy: "direction_first_then_regression_tiebreak".to_string(),
        primary_metric: "direction_test_accuracy".to_string(),
        secondary_metric: "direction_test_auc".to_string(),
        tertiary_metric: "return_test_directional_hit_rate".to_string(),
        quaternary_metric: "return_test_rmse_improvement_vs_baseline".to_string(),
    }
}

fn metric_f64(metric_root: &Value, path: &[&str]) -> f64 {
    path.iter()
        .try_fold(metric_root, |current, key| current.get(*key))
        .and_then(Value::as_f64)
        .unwrap_or(f64::NEG_INFINITY)
}

fn metric_string(metric_root: &Value, path: &[&str], fallback: &str) -> String {
    path.iter()
        .try_fold(metric_root, |current, key| current.get(*key))
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_string()
}

fn persist_json<T: Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), SecurityDirectionFirstTrainingRunError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| SecurityDirectionFirstTrainingRunError::Persist(error.to_string()))?;
    }
    let payload = serde_json::to_vec_pretty(value)
        .map_err(|error| SecurityDirectionFirstTrainingRunError::Persist(error.to_string()))?;
    fs::write(path, payload)
        .map_err(|error| SecurityDirectionFirstTrainingRunError::Persist(error.to_string()))
}

fn resolve_runtime_root(request: &SecurityDirectionFirstTrainingRunRequest) -> PathBuf {
    request
        .direction_first_runtime_root
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

fn default_survivor_count() -> usize {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_first_ranking_prefers_higher_accuracy_before_regression() {
        let stronger_direction = DirectionFirstRankingScore {
            direction_test_accuracy: 0.78,
            direction_test_auc: 0.72,
            return_test_directional_hit_rate: 0.52,
            return_test_rmse_improvement_vs_baseline: 0.001,
        };
        let stronger_regression = DirectionFirstRankingScore {
            direction_test_accuracy: 0.71,
            direction_test_auc: 0.79,
            return_test_directional_hit_rate: 0.66,
            return_test_rmse_improvement_vs_baseline: 0.014,
        };

        assert_eq!(
            compare_scores(&stronger_direction, &stronger_regression),
            Ordering::Greater
        );
    }

    #[test]
    fn direction_first_ranking_uses_auc_then_regression_as_tiebreak() {
        let stronger_auc = DirectionFirstRankingScore {
            direction_test_accuracy: 0.75,
            direction_test_auc: 0.78,
            return_test_directional_hit_rate: 0.51,
            return_test_rmse_improvement_vs_baseline: 0.003,
        };
        let weaker_auc_better_regression = DirectionFirstRankingScore {
            direction_test_accuracy: 0.75,
            direction_test_auc: 0.74,
            return_test_directional_hit_rate: 0.67,
            return_test_rmse_improvement_vs_baseline: 0.016,
        };

        assert_eq!(
            compare_scores(&stronger_auc, &weaker_auc_better_regression),
            Ordering::Greater
        );
    }
}
