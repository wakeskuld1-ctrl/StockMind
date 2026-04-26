use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

use crate::ops::stock::security_capital_source_factor_snapshot::{
    SecurityCapitalSourceFactorSnapshotError, SecurityCapitalSourceFactorSnapshotRequest,
    security_capital_source_factor_snapshot,
};
use crate::ops::stock::security_decision_evidence_bundle::{
    derive_atr_ratio_14, derive_mean_reversion_deviation_bucket_20d,
    derive_mean_reversion_normalized_distance_20d, derive_ratio_delta,
};
use crate::ops::stock::security_forward_outcome::{
    SecurityForwardOutcomeDocument, SecurityForwardOutcomeError, SecurityForwardOutcomeRequest,
    security_forward_outcome,
};
use crate::ops::stock::security_scorecard::{
    SecurityScorecardModelArtifact, SecurityScorecardModelBin, SecurityScorecardModelFeatureSpec,
};
use crate::ops::stock::security_scorecard_model_registry::{
    SecurityScorecardCandidateArtifactInput, SecurityScorecardModelRegistry, sanitize_identifier,
};
use crate::ops::stock::security_scorecard_refit_run::{
    SecurityScorecardRefitError, SecurityScorecardRefitRequest, SecurityScorecardRefitRun,
    security_scorecard_refit,
};
use crate::ops::stock::security_symbol_taxonomy::resolve_effective_security_routing;
use crate::runtime::security_capital_flow_store::SecurityCapitalFlowStore;
use crate::runtime::stock_history_store::{
    StockHistoryRow, StockHistoryStore, StockHistoryStoreError,
};

// 2026-04-09 CST: 这里新增正式训练入口请求合同，原因是 Task 5 需要把离线训练从临时脚本提升为可治理的一等 Tool；
// 目的：集中冻结市场范围、样本范围、目标头与运行时路径，避免训练参数散落在 Skill 或 CLI 外层。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityScorecardTrainingRequest {
    #[serde(default = "default_created_at")]
    pub created_at: String,
    // 2026-04-22 CST: Added because scheme 2 separates artifact persistence from
    // upstream data loading after the Nikkei capital-source path confusion surfaced.
    // Reason: one root field must not silently mean both "write outputs" and "read sources".
    // Purpose: make the artifact output boundary explicit on the training request.
    #[serde(default)]
    pub artifact_runtime_root: Option<String>,
    #[serde(default)]
    pub training_runtime_root: Option<String>,
    pub market_scope: String,
    pub instrument_scope: String,
    // 2026-04-20 CST: Added because Task 1 freezes non-equity training identity before
    // any real Nikkei or gold slice is allowed to enter the governed trainer.
    // Purpose: preserve the caller-approved instrument subscope all the way into artifact/registry outputs.
    #[serde(default)]
    pub instrument_subscope: Option<String>,
    pub symbol_list: Vec<String>,
    #[serde(default)]
    pub market_symbol: Option<String>,
    #[serde(default)]
    pub sector_symbol: Option<String>,
    // 2026-04-21 CST: Added because the approved Nikkei phase now trains on a
    // dedicated spot/futures contract instead of broad proxy reuse.
    // Reason: the user explicitly asked to add spot and index-futures factors for Nikkei.
    // Purpose: freeze one optional futures identity on the governed training request.
    #[serde(default)]
    pub futures_symbol: Option<String>,
    // 2026-04-25 CST: Added because Nikkei FRED spot history has no traded volume.
    // Reason: weekly training needs a separate volume-only proxy without treating it as a futures price factor.
    // Purpose: keep price source and volume source explicit so volume features can vary without contaminating spot labels.
    #[serde(default)]
    pub volume_proxy_symbol: Option<String>,
    // 2026-04-22 CST: Added because scheme C freezes one explicit training-contract switch
    // before preview A/B retraining compares baseline against JPX/MOF-enhanced Nikkei runs.
    // Reason: the user approved capital-source features as an isolated delta instead of another mixed retrain.
    // Purpose: keep the new factor pack opt-in, Nikkei-only, and traceable in request payloads.
    #[serde(default)]
    pub capital_source_feature_mode: Option<String>,
    // 2026-04-22 CST: Added because scheme 2 requires an explicit source-data root
    // for capital-flow factors instead of guessing from artifact output directories.
    // Reason: the previous wiring accidentally pointed the factor loader at an empty artifact-side database.
    // Purpose: bind Nikkei capital-source features to one explicit input-data boundary.
    #[serde(default)]
    pub capital_flow_runtime_root: Option<String>,
    #[serde(default)]
    pub market_profile: Option<String>,
    #[serde(default)]
    pub sector_profile: Option<String>,
    pub horizon_days: usize,
    pub target_head: String,
    pub train_range: String,
    pub valid_range: String,
    pub test_range: String,
    pub feature_set_version: String,
    pub label_definition_version: String,
    #[serde(default = "default_lookback_days")]
    pub lookback_days: usize,
    #[serde(default = "default_disclosure_limit")]
    pub disclosure_limit: usize,
    #[serde(default = "default_stop_loss_pct")]
    pub stop_loss_pct: f64,
    #[serde(default = "default_target_return_pct")]
    pub target_return_pct: f64,
}

// 2026-04-09 CST: 这里定义训练入口聚合返回对象，原因是调用方不仅要拿到 artifact，还要拿到 refit_run 和 registry；
// 目的：让后续 package、回算和审计链可以在一次调用后继续消费正式治理输出，而不是重新拼接路径。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityScorecardTrainingResult {
    pub artifact: SecurityScorecardModelArtifact,
    pub artifact_path: String,
    pub training_diagnostic_report_path: String,
    pub refit_run: SecurityScorecardRefitRun,
    pub model_registry: SecurityScorecardModelRegistry,
    pub refit_run_path: String,
    pub model_registry_path: String,
    pub metrics_summary_json: Value,
}

// 2026-04-09 CST: 这里集中定义训练入口错误边界，原因是 Task 5 同时覆盖样本采集、分箱建模、落盘与 refit 接线；
// 目的：向 dispatcher 暴露稳定、可定位的错误语义，避免把底层失败原样泄漏到外层。
#[derive(Debug, Error)]
pub enum SecurityScorecardTrainingError {
    #[error("security scorecard training build failed: {0}")]
    Build(String),
    #[error("security scorecard training history loading failed: {0}")]
    History(#[from] StockHistoryStoreError),
    #[error("security scorecard training outcome loading failed: {0}")]
    Outcome(#[from] SecurityForwardOutcomeError),
    #[error("security scorecard training capital-source loading failed: {0}")]
    CapitalSource(#[from] SecurityCapitalSourceFactorSnapshotError),
    #[error("security scorecard training persist failed: {0}")]
    Persist(String),
    #[error("security scorecard training refit failed: {0}")]
    Refit(#[from] SecurityScorecardRefitError),
}

#[derive(Debug, Clone, PartialEq)]
struct TrainingDateRange {
    start: NaiveDate,
    end: NaiveDate,
}

#[derive(Debug, Clone, PartialEq)]
struct TrainingSample {
    // 2026-04-17 CST: Added because the new diagnostic layer needs chronological folds and
    // per-symbol slice visibility without changing the prediction contract.
    // Reason: split-only samples were enough for the old minimal trainer but not for walk-forward
    // and drift inspection.
    // Purpose: keep diagnostics grounded in real symbol/date lineage.
    symbol: String,
    as_of_date: NaiveDate,
    split_name: String,
    label: f64,
    feature_values: BTreeMap<String, TrainingFeatureValue>,
}

#[derive(Debug, Clone, PartialEq)]
enum TrainingFeatureKind {
    Numeric,
    Categorical,
}

#[derive(Debug, Clone, PartialEq)]
enum TrainingFeatureValue {
    Numeric(f64),
    Category(String),
}

#[derive(Debug, Clone, PartialEq)]
struct TrainingFeatureConfig {
    feature_name: &'static str,
    group_name: &'static str,
    kind: TrainingFeatureKind,
}

#[derive(Debug, Clone, PartialEq)]
struct FeatureModel {
    feature_name: String,
    group_name: String,
    kind: TrainingFeatureKind,
    bins: Vec<FeatureBinModel>,
}

#[derive(Debug, Clone, PartialEq)]
struct FeatureBinModel {
    bin_label: String,
    match_values: Vec<String>,
    min_inclusive: Option<f64>,
    max_exclusive: Option<f64>,
    woe: f64,
    // 2026-04-17 CST: Added because the governed diagnostic report must explain support, IV,
    // and semantically backward bins instead of only publishing final WOE values.
    // Reason: the previous bin contract was too thin to explain overfitting.
    // Purpose: retain enough training-time evidence for diagnostics while preserving prediction behavior.
    positive_count: f64,
    negative_count: f64,
    sample_count: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct TrainedLogisticModel {
    intercept: f64,
    coefficients: Vec<f64>,
}

const UNSEEN_CATEGORICAL_BIN_LABEL: &str = "__unseen__";
const NIKKEI_JPX_MOF_CAPITAL_SOURCE_MODE: &str = "nikkei_jpx_mof_v1";

#[derive(Debug, Clone, PartialEq)]
struct EncodedDiagnosticSample {
    symbol: String,
    as_of_date: NaiveDate,
    split_name: String,
    label: f64,
    predicted_probability: f64,
    encoded_features: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WeeklyPriceFeatureRow {
    pub week_start_date: String,
    pub week_end_date: String,
    pub feature_values: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WeeklyRollingWindowPlan {
    pub train_anchor_dates: Vec<String>,
    pub valid_anchor_dates: Vec<String>,
    pub test_anchor_dates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct WeeklyRollingWindowSamples {
    window_id: usize,
    train_samples: Vec<TrainingSample>,
    valid_samples: Vec<TrainingSample>,
    test_samples: Vec<TrainingSample>,
}

#[derive(Debug, Clone, PartialEq)]
struct WeeklyTrainingContext {
    artifact_samples: Vec<TrainingSample>,
    rolling_windows: Vec<WeeklyRollingWindowSamples>,
    capital_source_observation_rows: Vec<CapitalSourceObservationRow>,
}

#[derive(Debug, Clone, PartialEq)]
struct CapitalSourceObservationRow {
    symbol: String,
    as_of_date: NaiveDate,
    observation_dates: BTreeMap<String, String>,
    factor_values: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq)]
struct WindowEvaluationRecord {
    window_id: usize,
    split_name: String,
    as_of_date: NaiveDate,
    label: f64,
    predicted_probability: f64,
}

// 2026-04-09 CST: 这里实现 Task 5 的最小正式训练入口，原因是我们需要先把训练主链跑通，再继续做回算重估和晋级治理；
// 目的：以最小的“样本采集 -> 分箱 -> WOE -> logistic -> artifact -> refit”闭环承接现有 scorecard 体系。
pub fn security_scorecard_training(
    request: &SecurityScorecardTrainingRequest,
) -> Result<SecurityScorecardTrainingResult, SecurityScorecardTrainingError> {
    validate_request(request)?;

    let train_range = parse_date_range(&request.train_range)?;
    let valid_range = parse_date_range(&request.valid_range)?;
    let test_range = parse_date_range(&request.test_range)?;
    let feature_configs = training_feature_configs(request);
    if uses_nikkei_weekly_training_contract(request) {
        return security_scorecard_training_weekly_contract(
            request,
            &train_range,
            &valid_range,
            &test_range,
            &feature_configs,
        );
    }
    let samples = collect_samples(
        request,
        &train_range,
        &valid_range,
        &test_range,
        &feature_configs,
    )?;
    let train_samples = samples_for_split(&samples, "train");
    if train_samples.len() < 2 {
        return Err(SecurityScorecardTrainingError::Build(
            "train split does not contain enough samples".to_string(),
        ));
    }
    let positive_count = train_samples
        .iter()
        .filter(|sample| sample.label >= 0.5)
        .count();
    let negative_count = train_samples.len().saturating_sub(positive_count);
    if positive_count == 0 || negative_count == 0 {
        return Err(SecurityScorecardTrainingError::Build(
            "train split must contain both positive and negative labels".to_string(),
        ));
    }

    let feature_models = build_feature_models(&train_samples, &feature_configs)?;
    let train_matrix = encode_samples(&train_samples, &feature_models)?;
    let trained_model = train_logistic_model(&train_matrix);
    let artifact = build_artifact(request, &feature_models, &trained_model);

    let runtime_root = resolve_runtime_root(request);
    let artifact_path = runtime_root.join("scorecard_artifacts").join(format!(
        "{}__{}.json",
        sanitize_identifier(&artifact.model_id),
        sanitize_identifier(&artifact.model_version)
    ));
    persist_json(&artifact_path, &artifact)?;

    // 2026-04-17 CST: Added because the user explicitly asked to inspect process metrics instead
    // of only split accuracy before discussing further model work.
    // Reason: training output must now explain drift, correlation, fold stability, and feature dominance.
    // Purpose: persist one governed diagnostic report alongside the artifact without changing model semantics.
    let training_diagnostic_report_json = build_training_diagnostic_report(
        request,
        &samples,
        &feature_configs,
        &feature_models,
        &trained_model,
    )?;
    let training_diagnostic_report_path =
        runtime_root
            .join("scorecard_training_diagnostics")
            .join(format!(
                "{}__{}.json",
                sanitize_identifier(&artifact.model_id),
                sanitize_identifier(&artifact.model_version)
            ));
    persist_json(
        &training_diagnostic_report_path,
        &training_diagnostic_report_json,
    )?;

    let metrics_summary_json = build_metrics_summary(
        &samples,
        &feature_models,
        &trained_model,
        training_diagnostic_report_json
            .get("summary")
            .cloned()
            .unwrap_or_else(|| json!({})),
        valid_range.start,
    );
    let refit_result = security_scorecard_refit(&SecurityScorecardRefitRequest {
        created_at: request.created_at.clone(),
        refit_runtime_root: Some(runtime_root.to_string_lossy().to_string()),
        market_scope: request.market_scope.clone(),
        instrument_scope: request.instrument_scope.clone(),
        feature_set_version: request.feature_set_version.clone(),
        label_definition_version: request.label_definition_version.clone(),
        train_range: request.train_range.clone(),
        valid_range: request.valid_range.clone(),
        test_range: request.test_range.clone(),
        candidate_artifact: SecurityScorecardCandidateArtifactInput {
            model_id: artifact.model_id.clone(),
            model_version: artifact.model_version.clone(),
            horizon_days: request.horizon_days,
            target_head: request.target_head.clone(),
            status: "candidate".to_string(),
            artifact_path: artifact_path.to_string_lossy().to_string(),
            metrics_summary_json: metrics_summary_json.clone(),
            published_at: Some(request.created_at.clone()),
            // 2026-04-14 CST: 这里补齐 registry 新增的候选模型字段，原因是 training 仍按旧合同构造
            // candidate artifact，导致 refit 链无法消费当前正式 registry 输入。
            // 目的：先用最小默认值恢复训练产物登记能力，不在本轮扩散到更大范围的模型分级重构。
            // 2026-04-20 CST: Added because Task 1 requires the formal trainer to stop
            // dropping the approved non-equity identity contract before registry ingestion.
            // Purpose: keep artifact and registry scope aligned for downstream selection gates.
            instrument_subscope: request.instrument_subscope.clone(),
            model_grade: "candidate".to_string(),
            grade_reason: "training pipeline default candidate grade".to_string(),
        },
        comparison_to_champion_json: None,
        promotion_decision: Some("candidate_only".to_string()),
    })?;

    Ok(SecurityScorecardTrainingResult {
        artifact,
        artifact_path: artifact_path.to_string_lossy().to_string(),
        training_diagnostic_report_path: training_diagnostic_report_path
            .to_string_lossy()
            .to_string(),
        refit_run: refit_result.refit_run,
        model_registry: refit_result.model_registry,
        refit_run_path: refit_result.refit_run_path,
        model_registry_path: refit_result.model_registry_path,
        metrics_summary_json,
    })
}

fn security_scorecard_training_weekly_contract(
    request: &SecurityScorecardTrainingRequest,
    train_range: &TrainingDateRange,
    valid_range: &TrainingDateRange,
    test_range: &TrainingDateRange,
    feature_configs: &[TrainingFeatureConfig],
) -> Result<SecurityScorecardTrainingResult, SecurityScorecardTrainingError> {
    // 2026-04-23 CST: Added because the approved Nikkei weekly route must keep
    // artifact fitting and rolling-window evaluation as separate governed surfaces.
    // Reason: flattening all weekly windows into one global split silently polluted valid/test counts.
    // Purpose: train the persisted artifact once, while evaluating weekly accuracy on independent windows.
    let weekly_context = build_weekly_training_context(
        request,
        train_range,
        valid_range,
        test_range,
        feature_configs,
    )?;
    let samples = weekly_context.artifact_samples;
    let train_samples = samples_for_split(&samples, "train");
    if train_samples.len() < 2 {
        return Err(SecurityScorecardTrainingError::Build(
            "train split does not contain enough samples".to_string(),
        ));
    }
    let positive_count = train_samples
        .iter()
        .filter(|sample| sample.label >= 0.5)
        .count();
    let negative_count = train_samples.len().saturating_sub(positive_count);
    if positive_count == 0 || negative_count == 0 {
        return Err(SecurityScorecardTrainingError::Build(
            "train split must contain both positive and negative labels".to_string(),
        ));
    }

    let feature_models = build_feature_models(&train_samples, feature_configs)?;
    let train_matrix = encode_samples(&train_samples, &feature_models)?;
    let trained_model = train_logistic_model(&train_matrix);
    let artifact = build_artifact(request, &feature_models, &trained_model);

    let runtime_root = resolve_runtime_root(request);
    let artifact_path = runtime_root.join("scorecard_artifacts").join(format!(
        "{}__{}.json",
        sanitize_identifier(&artifact.model_id),
        sanitize_identifier(&artifact.model_version)
    ));
    persist_json(&artifact_path, &artifact)?;

    let training_diagnostic_report_json = build_training_diagnostic_report(
        request,
        &samples,
        feature_configs,
        &feature_models,
        &trained_model,
    )?;
    let training_diagnostic_report_path =
        runtime_root
            .join("scorecard_training_diagnostics")
            .join(format!(
                "{}__{}.json",
                sanitize_identifier(&artifact.model_id),
                sanitize_identifier(&artifact.model_version)
            ));
    persist_json(
        &training_diagnostic_report_path,
        &training_diagnostic_report_json,
    )?;

    let metrics_summary_json = build_weekly_metrics_summary(
        &samples,
        &weekly_context.rolling_windows,
        &weekly_context.capital_source_observation_rows,
        feature_configs,
        &feature_models,
        &trained_model,
        training_diagnostic_report_json
            .get("summary")
            .cloned()
            .unwrap_or_else(|| json!({})),
        valid_range.start,
    )?;
    let refit_result = security_scorecard_refit(&SecurityScorecardRefitRequest {
        created_at: request.created_at.clone(),
        refit_runtime_root: Some(runtime_root.to_string_lossy().to_string()),
        market_scope: request.market_scope.clone(),
        instrument_scope: request.instrument_scope.clone(),
        feature_set_version: request.feature_set_version.clone(),
        label_definition_version: request.label_definition_version.clone(),
        train_range: request.train_range.clone(),
        valid_range: request.valid_range.clone(),
        test_range: request.test_range.clone(),
        candidate_artifact: SecurityScorecardCandidateArtifactInput {
            model_id: artifact.model_id.clone(),
            model_version: artifact.model_version.clone(),
            horizon_days: request.horizon_days,
            target_head: request.target_head.clone(),
            status: "candidate".to_string(),
            artifact_path: artifact_path.to_string_lossy().to_string(),
            metrics_summary_json: metrics_summary_json.clone(),
            published_at: Some(request.created_at.clone()),
            instrument_subscope: request.instrument_subscope.clone(),
            model_grade: "candidate".to_string(),
            grade_reason: "training pipeline default candidate grade".to_string(),
        },
        comparison_to_champion_json: None,
        promotion_decision: Some("candidate_only".to_string()),
    })?;

    Ok(SecurityScorecardTrainingResult {
        artifact,
        artifact_path: artifact_path.to_string_lossy().to_string(),
        training_diagnostic_report_path: training_diagnostic_report_path
            .to_string_lossy()
            .to_string(),
        refit_run: refit_result.refit_run,
        model_registry: refit_result.model_registry,
        refit_run_path: refit_result.refit_run_path,
        model_registry_path: refit_result.model_registry_path,
        metrics_summary_json,
    })
}

fn validate_request(
    request: &SecurityScorecardTrainingRequest,
) -> Result<(), SecurityScorecardTrainingError> {
    for (field_name, field_value) in [
        ("market_scope", request.market_scope.trim()),
        ("instrument_scope", request.instrument_scope.trim()),
        ("target_head", request.target_head.trim()),
        ("train_range", request.train_range.trim()),
        ("valid_range", request.valid_range.trim()),
        ("test_range", request.test_range.trim()),
        ("feature_set_version", request.feature_set_version.trim()),
        (
            "label_definition_version",
            request.label_definition_version.trim(),
        ),
    ] {
        if field_value.is_empty() {
            return Err(SecurityScorecardTrainingError::Build(format!(
                "{field_name} cannot be empty"
            )));
        }
    }
    if request.horizon_days == 0 {
        return Err(SecurityScorecardTrainingError::Build(
            "horizon_days must be greater than 0".to_string(),
        ));
    }
    if !is_supported_target_head(&request.target_head) {
        return Err(SecurityScorecardTrainingError::Build(format!(
            "unsupported target_head `{}`",
            request.target_head
        )));
    }
    if request.symbol_list.is_empty() {
        return Err(SecurityScorecardTrainingError::Build(
            "symbol_list cannot be empty".to_string(),
        ));
    }
    // 2026-04-22 CST: Added because scheme 2 introduces an explicit artifact root
    // while retaining the legacy alias during migration.
    // Purpose: fail closed when callers provide two different output-root meanings.
    if let (Some(artifact_root), Some(legacy_root)) = (
        normalized_artifact_runtime_root(request),
        normalized_legacy_training_runtime_root(request),
    ) {
        if artifact_root != legacy_root {
            return Err(SecurityScorecardTrainingError::Build(
                "artifact_runtime_root conflicts with legacy training_runtime_root".to_string(),
            ));
        }
    }
    if let Some(mode) = normalized_capital_source_feature_mode(request) {
        if mode != NIKKEI_JPX_MOF_CAPITAL_SOURCE_MODE {
            return Err(SecurityScorecardTrainingError::Build(format!(
                "unsupported capital_source_feature_mode `{mode}`"
            )));
        }
        if !uses_nikkei_index_feature_contract(request) {
            return Err(SecurityScorecardTrainingError::Build(
                "capital_source_feature_mode is only supported for instrument_subscope `nikkei_index`"
                    .to_string(),
            ));
        }
        // 2026-04-22 CST: Added because enhanced Nikkei training must bind capital-source
        // factors to one explicit input-data root instead of guessing from output paths.
        // Purpose: reject source-boundary ambiguity before sample collection starts.
        if normalized_capital_flow_runtime_root(request).is_none() {
            return Err(SecurityScorecardTrainingError::Build(
                "capital_flow_runtime_root is required when capital_source_feature_mode is enabled"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

// 2026-04-20 CST: Added because the approved contract refactor must separate
// supported target heads from legacy positive-only assumptions before retraining.
// Reason: the old validator and label builder only understood direction_head.
// Purpose: centralize governed head semantics for validation, labeling, and artifact metadata.
// 2026-04-21 CST: Extended because the approved Nikkei route now adds a repair-stable
// head without replacing the existing direction heads.
// Purpose: keep head validation and artifact semantics frozen in one governed switch.
fn is_supported_target_head(target_head: &str) -> bool {
    matches!(
        target_head.trim(),
        "direction_head" | "direction_up_head" | "direction_down_head" | "repair_stable_head"
    )
}

fn resolve_target_label_definition(target_head: &str, horizon_days: usize) -> String {
    if target_head.trim() == "direction_head" {
        return "positive_return_1w".to_string();
    }
    match target_head.trim() {
        "direction_up_head" => format!("positive_return_{}d", horizon_days),
        "direction_down_head" => format!("negative_return_{}d", horizon_days),
        "repair_stable_head" => format!("repair_stable_{}d", horizon_days),
        _ => format!("unsupported_target_head_{}d", horizon_days),
    }
}

fn resolve_positive_label_definition(target_head: &str, horizon_days: usize) -> Option<String> {
    match target_head.trim() {
        "direction_head" => Some("positive_return_1w".to_string()),
        "direction_up_head" => Some(format!("positive_return_{}d", horizon_days)),
        "direction_down_head" => None,
        "repair_stable_head" => Some(format!("repair_stable_{}d", horizon_days)),
        _ => None,
    }
}

fn resolve_training_label(outcome: &SecurityForwardOutcomeDocument, target_head: &str) -> f64 {
    match target_head.trim() {
        "direction_head" | "direction_up_head" => {
            if outcome.positive_return {
                1.0
            } else {
                0.0
            }
        }
        "direction_down_head" => {
            if outcome.forward_return < 0.0 {
                1.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

fn resolve_weekly_direction_label(current_close: f64, next_close: f64) -> Option<f64> {
    if current_close.abs() <= f64::EPSILON {
        return None;
    }
    Some(if (next_close / current_close) - 1.0 > 0.0 {
        1.0
    } else {
        0.0
    })
}

// 2026-04-21 CST: Added because the approved Nikkei retraining route now targets
// "oversold can repair and stay repaired" instead of plain 10-day direction.
// Reason: the user explicitly moved this workstream onto a path-based repair contract.
// Purpose: freeze one binary label rule before sample collection and artifact output diverge.
fn resolve_repair_stable_label_from_buckets(
    current_bucket: &str,
    future_buckets: &[String],
) -> Option<f64> {
    if !is_mean_reversion_lower_half_bucket(current_bucket) || future_buckets.is_empty() {
        return None;
    }

    let hit_upper_half = future_buckets
        .iter()
        .any(|bucket| is_mean_reversion_upper_half_bucket(bucket));
    let final_bucket = future_buckets.last()?;
    let stable_finish = is_mean_reversion_non_lower_half_bucket(final_bucket);

    Some(if hit_upper_half && stable_finish {
        1.0
    } else {
        0.0
    })
}

// 2026-04-21 CST: Added because repair training must only start from current lower-half states.
// Reason: mixing neutral or upper-half starts back into the sample pool would violate the approved contract.
// Purpose: keep the repair head focused on "oversold entry -> future repair stability" only.
fn is_mean_reversion_lower_half_bucket(bucket: &str) -> bool {
    matches!(bucket.trim(), "strong_down" | "weak_down")
}

// 2026-04-21 CST: Added because the repair path contract defines "repair hit" by first reaching
// the upper half, not merely escaping into neutral.
// Reason: the user explicitly separated "touched upper half" from "just bounced a little".
// Purpose: preserve one auditable upper-half trigger for repair-path labels.
fn is_mean_reversion_upper_half_bucket(bucket: &str) -> bool {
    matches!(bucket.trim(), "weak_up" | "strong_up")
}

// 2026-04-21 CST: Added because repair_stable ends on day 10 as long as the path did not fall
// back into the lower half, including neutral finishes.
// Reason: the approved contract defines stability as "not back below neutral" after an upper-half hit.
// Purpose: keep repair_stable and repair_hit separated by the final-day landing zone.
fn is_mean_reversion_non_lower_half_bucket(bucket: &str) -> bool {
    matches!(bucket.trim(), "neutral" | "weak_up" | "strong_up")
}

// 2026-04-21 CST: Added because the new repair head must sample from oversold dates rather than
// from the full calendar and then accidentally skip almost everything.
// Reason: the approved route is about lower-half repair candidates, not generic direction dates.
// Purpose: pre-filter dates so monthly-like spacing happens inside the eligible repair population.
fn filter_repair_stable_candidate_dates(
    store: &StockHistoryStore,
    symbol: &str,
    dates: &[String],
    lookback_days: usize,
    horizon_days: usize,
) -> Result<Vec<String>, SecurityScorecardTrainingError> {
    let mut eligible_dates = Vec::new();
    for date in dates {
        let current_bucket =
            derive_mean_reversion_bucket_for_date(store, symbol, date, lookback_days)?;
        if !is_mean_reversion_lower_half_bucket(&current_bucket) {
            continue;
        }
        let future_buckets =
            derive_future_mean_reversion_buckets(store, symbol, date, lookback_days, horizon_days)?;
        if future_buckets.len() == horizon_days {
            eligible_dates.push(date.clone());
        }
    }
    Ok(eligible_dates)
}

// 2026-04-21 CST: Added because both candidate filtering and future-path labeling need the
// same local mean-reversion bucket semantics without reusing external network-dependent tools.
// Reason: repair_stable_head is a price-path label, so local history must be enough to reproduce it.
// Purpose: compute the governed ATR-normalized MA20 bucket directly from stored OHLC history.
fn derive_mean_reversion_bucket_for_date(
    store: &StockHistoryStore,
    symbol: &str,
    as_of_date: &str,
    lookback_days: usize,
) -> Result<String, SecurityScorecardTrainingError> {
    let recent_rows = store.load_recent_rows(symbol, Some(as_of_date), lookback_days.max(20))?;
    derive_mean_reversion_bucket_from_rows(&recent_rows)
}

// 2026-04-21 CST: Added because the repair head needs the whole forward path converted into
// the same five buckets as the current snapshot.
// Reason: the label depends on whether price ever reaches the upper half and where it ends on day 10.
// Purpose: keep future-path repair classification on the same bucket vocabulary as the current state.
fn derive_future_mean_reversion_buckets(
    store: &StockHistoryStore,
    symbol: &str,
    as_of_date: &str,
    lookback_days: usize,
    horizon_days: usize,
) -> Result<Vec<String>, SecurityScorecardTrainingError> {
    let future_rows = store.load_forward_rows(symbol, as_of_date, horizon_days)?;
    let mut future_buckets = Vec::with_capacity(future_rows.len());
    for future_row in future_rows {
        let recent_rows =
            store.load_recent_rows(symbol, Some(&future_row.trade_date), lookback_days.max(20))?;
        future_buckets.push(derive_mean_reversion_bucket_from_rows(&recent_rows)?);
    }
    Ok(future_buckets)
}

// 2026-04-21 CST: Added because repair_stable_head cannot depend on a precomputed snapshot for
// every intermediate forward date.
// Reason: the user approved a local path label, so training needs one internal bucket calculator.
// Purpose: reproduce the ATR-normalized MA20 bucket using only stored rows and the shared formulas.
fn derive_mean_reversion_bucket_from_rows(
    recent_rows: &[StockHistoryRow],
) -> Result<String, SecurityScorecardTrainingError> {
    let close = recent_rows
        .last()
        .map(|row| row.close)
        .ok_or_else(|| SecurityScorecardTrainingError::Build("missing latest close".to_string()))?;
    let sma_20 = sma_last_from_rows(recent_rows, 20)?;
    let atr_14 = atr_last_from_rows(recent_rows, 14)?;
    let close_vs_sma20 = derive_ratio_delta(close, sma_20);
    let atr_ratio_14 = derive_atr_ratio_14(close, atr_14);
    let normalized_distance =
        derive_mean_reversion_normalized_distance_20d(close_vs_sma20, atr_ratio_14);
    Ok(derive_mean_reversion_deviation_bucket_20d(normalized_distance).to_string())
}

// 2026-04-21 CST: Added because the local repair-path label still needs the same SMA20 denominator
// that the shared snapshot contract uses.
// Reason: reusing only the last close would make the bucket drift away from the governed MA20 view.
// Purpose: provide one minimal in-file SMA helper for repair labeling without widening module exposure.
fn sma_last_from_rows(
    rows: &[StockHistoryRow],
    period: usize,
) -> Result<f64, SecurityScorecardTrainingError> {
    if rows.len() < period {
        return Err(SecurityScorecardTrainingError::Build(format!(
            "insufficient rows for sma_{period}: actual={}",
            rows.len()
        )));
    }
    let window = &rows[rows.len() - period..];
    Ok(window.iter().map(|row| row.close).sum::<f64>() / period as f64)
}

// 2026-04-21 CST: Added because the ATR-normalized repair bucket must keep the same Wilder-style
// volatility denominator used by the shared technical indicators.
// Reason: using a plain average range here would make repair labels drift from the approved bucket semantics.
// Purpose: reproduce atr_14 locally so forward-path dates can be labeled without new snapshot documents.
fn atr_last_from_rows(
    rows: &[StockHistoryRow],
    period: usize,
) -> Result<f64, SecurityScorecardTrainingError> {
    if rows.len() < period + 1 {
        return Err(SecurityScorecardTrainingError::Build(format!(
            "insufficient rows for atr_{period}: actual={}",
            rows.len()
        )));
    }

    let true_ranges = true_ranges_from_rows(rows);
    let mut atr = true_ranges[..period].iter().sum::<f64>() / period as f64;
    for true_range in true_ranges.iter().skip(period) {
        atr = ((atr * (period as f64 - 1.0)) + true_range) / period as f64;
    }
    Ok(atr)
}

// 2026-04-21 CST: Added because ATR needs one shared true-range sequence inside the repair label path.
// Reason: duplicating the three-leg true-range formula inline would make later reviews harder to audit.
// Purpose: keep the local repair-label volatility math compact and traceable.
fn true_ranges_from_rows(rows: &[StockHistoryRow]) -> Vec<f64> {
    let mut true_ranges = Vec::with_capacity(rows.len().saturating_sub(1));
    for index in 1..rows.len() {
        let current_row = &rows[index];
        let previous_row = &rows[index - 1];
        let high_low = current_row.high - current_row.low;
        let high_close = (current_row.high - previous_row.close).abs();
        let low_close = (current_row.low - previous_row.close).abs();
        true_ranges.push(high_low.max(high_close).max(low_close));
    }
    true_ranges
}

fn training_feature_configs(
    request: &SecurityScorecardTrainingRequest,
) -> Vec<TrainingFeatureConfig> {
    if uses_nikkei_weekly_training_contract(request) {
        let mut feature_configs = vec![
            TrainingFeatureConfig {
                feature_name: "weekly_spot_return_min",
                group_name: "W",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "weekly_spot_return_p10",
                group_name: "W",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "weekly_spot_return_p25",
                group_name: "W",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "weekly_spot_return_p50",
                group_name: "W",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "weekly_spot_return_p75",
                group_name: "W",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "weekly_spot_return_p90",
                group_name: "W",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "weekly_spot_return_max",
                group_name: "W",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "weekly_spot_close_position",
                group_name: "W",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "weekly_spot_drawdown",
                group_name: "W",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "weekly_spot_rebound",
                group_name: "W",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "weekly_volume_ratio_4w",
                group_name: "V",
                kind: TrainingFeatureKind::Numeric,
            },
            // 2026-04-26 CST: Added because Nikkei index accumulation can be a
            // half-year or yearly behavior instead of a 4-week volume burst.
            // Purpose: let weekly training inspect slow institutional positioning.
            TrainingFeatureConfig {
                feature_name: "weekly_volume_ratio_13w",
                group_name: "V",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "weekly_volume_ratio_26w",
                group_name: "V",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "weekly_volume_ratio_52w",
                group_name: "V",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "weekly_price_position_52w",
                group_name: "V",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "weekly_volume_accumulation_26w",
                group_name: "V",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "weekly_volume_accumulation_52w",
                group_name: "V",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "weekly_high_volume_low_price_signal",
                group_name: "V",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "weekly_high_volume_breakout_signal",
                group_name: "V",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "weekly_up_day_volume_share",
                group_name: "V",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "weekly_down_day_volume_share",
                group_name: "V",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "weekly_volume_price_confirmation",
                group_name: "V",
                kind: TrainingFeatureKind::Numeric,
            },
        ];
        if uses_nikkei_futures_feature_contract(request) {
            feature_configs.extend([
                TrainingFeatureConfig {
                    feature_name: "weekly_futures_return_p50",
                    group_name: "X",
                    kind: TrainingFeatureKind::Numeric,
                },
                TrainingFeatureConfig {
                    feature_name: "weekly_basis_pct_p50",
                    group_name: "X",
                    kind: TrainingFeatureKind::Numeric,
                },
                TrainingFeatureConfig {
                    feature_name: "weekly_futures_relative_strength_p50",
                    group_name: "X",
                    kind: TrainingFeatureKind::Numeric,
                },
            ]);
        }
        return feature_configs;
    }

    let mut feature_configs = vec![
        // 2026-04-16 CST: Added because A-1a starts the first formal regime/industry field
        // thickening pass before model-family upgrades.
        // Reason: the prior baseline lacked stable market-state segmentation, which made later
        // accuracy work look like a pure model problem.
        // Purpose: let training learn across market-regime and industry buckets instead of only
        // raw technical/fundamental event fields.
        // 2026-04-21 CST: Updated because the user explicitly rejected the integrated summary
        // label for Nikkei training after confirming the real risk sits in weight-driven structure.
        // Purpose: keep only atomic market-state inputs in the governed Phase-B index contract.
        TrainingFeatureConfig {
            feature_name: "market_regime",
            group_name: "M",
            kind: TrainingFeatureKind::Categorical,
        },
        TrainingFeatureConfig {
            feature_name: "instrument_subscope",
            group_name: "M",
            kind: TrainingFeatureKind::Categorical,
        },
        TrainingFeatureConfig {
            feature_name: "technical_alignment",
            group_name: "T",
            kind: TrainingFeatureKind::Categorical,
        },
        // 2026-04-10 CST: 这里扩第一阶段统一评分版入模特征，原因是当前训练只吃 4 个占位字段，无法表达技术面、基本面、消息面的结构化差异；
        // 目的：先把“明显更有信息量、但仍可稳定跑通”的第一批特征纳入 artifact，为顺丰/平安验证输出更像样的问题点。
        TrainingFeatureConfig {
            feature_name: "trend_bias",
            group_name: "T",
            kind: TrainingFeatureKind::Categorical,
        },
        TrainingFeatureConfig {
            feature_name: "trend_strength",
            group_name: "T",
            kind: TrainingFeatureKind::Categorical,
        },
        TrainingFeatureConfig {
            feature_name: "volume_confirmation",
            group_name: "T",
            kind: TrainingFeatureKind::Categorical,
        },
        TrainingFeatureConfig {
            feature_name: "breakout_signal",
            group_name: "T",
            kind: TrainingFeatureKind::Categorical,
        },
        TrainingFeatureConfig {
            feature_name: "momentum_signal",
            group_name: "T",
            kind: TrainingFeatureKind::Categorical,
        },
        TrainingFeatureConfig {
            feature_name: "flow_status",
            group_name: "Q",
            kind: TrainingFeatureKind::Categorical,
        },
        TrainingFeatureConfig {
            feature_name: "volume_ratio_20",
            group_name: "Q",
            kind: TrainingFeatureKind::Numeric,
        },
        TrainingFeatureConfig {
            feature_name: "mfi_14",
            group_name: "Q",
            kind: TrainingFeatureKind::Numeric,
        },
        TrainingFeatureConfig {
            feature_name: "macd_histogram",
            group_name: "Q",
            kind: TrainingFeatureKind::Numeric,
        },
        TrainingFeatureConfig {
            feature_name: "data_gap_count",
            group_name: "R",
            kind: TrainingFeatureKind::Numeric,
        },
        TrainingFeatureConfig {
            feature_name: "risk_note_count",
            group_name: "R",
            kind: TrainingFeatureKind::Numeric,
        },
        // 2026-04-20 CST: Added because Task A replaces the mixed valuation_status factor with
        // plain position/quality buckets that users can inspect independently.
        // Purpose: let training answer which 14d range, 20d band, 20d mean-reversion, and
        // quality state actually helps instead of hiding them inside one composite label.
        // 2026-04-21 CST: Extended because the approved Nikkei retraining route now reads
        // mean-reversion from MA20 deviation bands instead of the older CCI-only enum.
        // Reason: keep the governed training contract aligned with the new five-level bucket.
        // Purpose: ensure retraining evaluates percentage deviation strength, not just CCI labels.
        TrainingFeatureConfig {
            feature_name: "bollinger_position_20d",
            group_name: "V",
            kind: TrainingFeatureKind::Categorical,
        },
        TrainingFeatureConfig {
            feature_name: "range_position_14d",
            group_name: "V",
            kind: TrainingFeatureKind::Categorical,
        },
        TrainingFeatureConfig {
            feature_name: "mean_reversion_deviation_20d",
            group_name: "V",
            kind: TrainingFeatureKind::Categorical,
        },
        TrainingFeatureConfig {
            feature_name: "rsi_14",
            group_name: "V",
            kind: TrainingFeatureKind::Numeric,
        },
        TrainingFeatureConfig {
            feature_name: "atr_ratio_14",
            group_name: "V",
            kind: TrainingFeatureKind::Numeric,
        },
    ];

    if uses_nikkei_index_feature_contract(request) {
        // 2026-04-22 CST: Added because the user explicitly rejected risk_note_count as
        // a meaningful Nikkei index risk factor after the first futures-proxy retrain.
        // Reason: the field kept surfacing as a high-coefficient, low-information noise term.
        // Purpose: keep the Nikkei-only contract focused on market structure rather than note-count metadata.
        feature_configs.retain(|config| config.feature_name != "risk_note_count");
    }

    if uses_nikkei_futures_feature_contract(request) {
        // 2026-04-21 CST: Added because the approved Nikkei route must stop
        // training on zero-variance placeholders once futures factors are present.
        // Reason: the last governed retrain showed these six fields carried no information.
        // Purpose: keep the Nikkei-only contract lean while avoiding global feature deletion.
        feature_configs.retain(|config| {
            !matches!(
                config.feature_name,
                "instrument_subscope"
                    | "volume_confirmation"
                    | "flow_status"
                    | "volume_ratio_20"
                    | "mfi_14"
                    | "data_gap_count"
            )
        });
        // 2026-04-22 CST: Added because the user approved splitting the old mixed
        // lead-strength factor into three atomic 3d futures/spot signals.
        // Reason: the old delta compressed direction and relative outperformance into one opaque value.
        // Purpose: let Nikkei retraining inspect futures direction, spot direction, and relative strength separately.
        feature_configs.extend([
            TrainingFeatureConfig {
                feature_name: "futures_return_1d",
                group_name: "X",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "futures_spot_basis_pct",
                group_name: "X",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "futures_return_3d",
                group_name: "X",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "spot_return_3d",
                group_name: "X",
                kind: TrainingFeatureKind::Numeric,
            },
            TrainingFeatureConfig {
                feature_name: "futures_relative_strength_3d",
                group_name: "X",
                kind: TrainingFeatureKind::Numeric,
            },
        ]);
    }

    feature_configs
}

fn uses_nikkei_index_feature_contract(request: &SecurityScorecardTrainingRequest) -> bool {
    matches!(
        request.instrument_subscope.as_deref().map(str::trim),
        Some("nikkei_index")
    )
}

fn uses_nikkei_weekly_training_contract(request: &SecurityScorecardTrainingRequest) -> bool {
    uses_nikkei_index_feature_contract(request) && request.target_head.trim() == "direction_head"
}

fn uses_nikkei_futures_feature_contract(request: &SecurityScorecardTrainingRequest) -> bool {
    uses_nikkei_index_feature_contract(request)
        && request
            .futures_symbol
            .as_deref()
            .map(str::trim)
            .is_some_and(|symbol| !symbol.is_empty())
}

fn normalized_volume_proxy_symbol(request: &SecurityScorecardTrainingRequest) -> Option<&str> {
    request
        .volume_proxy_symbol
        .as_deref()
        .map(str::trim)
        .filter(|symbol| !symbol.is_empty())
}

fn normalized_capital_source_feature_mode(
    request: &SecurityScorecardTrainingRequest,
) -> Option<&str> {
    request
        .capital_source_feature_mode
        .as_deref()
        .map(str::trim)
        .filter(|mode| !mode.is_empty())
}

fn normalized_artifact_runtime_root(request: &SecurityScorecardTrainingRequest) -> Option<String> {
    request
        .artifact_runtime_root
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalized_legacy_training_runtime_root(
    request: &SecurityScorecardTrainingRequest,
) -> Option<String> {
    request
        .training_runtime_root
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalized_capital_flow_runtime_root(
    request: &SecurityScorecardTrainingRequest,
) -> Option<String> {
    request
        .capital_flow_runtime_root
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn uses_nikkei_capital_source_feature_contract(request: &SecurityScorecardTrainingRequest) -> bool {
    uses_nikkei_index_feature_contract(request)
        && matches!(
            normalized_capital_source_feature_mode(request),
            Some(NIKKEI_JPX_MOF_CAPITAL_SOURCE_MODE)
        )
}

fn parse_date_range(raw: &str) -> Result<TrainingDateRange, SecurityScorecardTrainingError> {
    let Some((start_raw, end_raw)) = raw.split_once("..") else {
        return Err(SecurityScorecardTrainingError::Build(format!(
            "invalid date range `{raw}`"
        )));
    };
    let start = NaiveDate::parse_from_str(start_raw.trim(), "%Y-%m-%d").map_err(|error| {
        SecurityScorecardTrainingError::Build(format!(
            "invalid range start `{}`: {error}",
            start_raw.trim()
        ))
    })?;
    let end = NaiveDate::parse_from_str(end_raw.trim(), "%Y-%m-%d").map_err(|error| {
        SecurityScorecardTrainingError::Build(format!(
            "invalid range end `{}`: {error}",
            end_raw.trim()
        ))
    })?;
    if end < start {
        return Err(SecurityScorecardTrainingError::Build(format!(
            "invalid date range `{raw}`: end is earlier than start"
        )));
    }
    Ok(TrainingDateRange { start, end })
}

fn collect_samples(
    request: &SecurityScorecardTrainingRequest,
    train_range: &TrainingDateRange,
    valid_range: &TrainingDateRange,
    test_range: &TrainingDateRange,
    feature_configs: &[TrainingFeatureConfig],
) -> Result<Vec<TrainingSample>, SecurityScorecardTrainingError> {
    if uses_nikkei_weekly_training_contract(request) {
        return collect_weekly_samples(
            request,
            train_range,
            valid_range,
            test_range,
            feature_configs,
        );
    }
    let store = StockHistoryStore::workspace_default()?;
    let mut samples = Vec::new();

    for symbol in &request.symbol_list {
        let effective_routing = resolve_effective_security_routing(
            symbol,
            request.market_symbol.as_deref(),
            request.sector_symbol.as_deref(),
            request.market_profile.as_deref(),
            request.sector_profile.as_deref(),
        );
        for (split_name, range) in [
            ("train", train_range),
            ("valid", valid_range),
            ("test", test_range),
        ] {
            let mut candidate_dates =
                load_dates_in_range(&store, symbol, range, 200, request.horizon_days)?;
            // 2026-04-21 CST: Added because repair_stable_head must sample only from oversold
            // starting points before spacing dates across the split.
            // Reason: selecting monthly dates first and filtering later would collapse the repair sample density.
            // Purpose: keep the approved lower-half repair population intact before cadence sampling.
            if request.target_head.trim() == "repair_stable_head" {
                candidate_dates = filter_repair_stable_candidate_dates(
                    &store,
                    symbol,
                    &candidate_dates,
                    request.lookback_days,
                    request.horizon_days,
                )?;
            }
            let target_count = build_split_target_count(split_name, range);
            let selected_dates = select_evenly_spaced_dates(&candidate_dates, target_count);
            for as_of_date in selected_dates {
                let outcome_result = security_forward_outcome(&SecurityForwardOutcomeRequest {
                    symbol: symbol.clone(),
                    market_symbol: effective_routing.market_symbol.clone(),
                    sector_symbol: effective_routing.sector_symbol.clone(),
                    futures_symbol: request.futures_symbol.clone(),
                    market_profile: effective_routing.market_profile.clone(),
                    sector_profile: effective_routing.sector_profile.clone(),
                    as_of_date: Some(as_of_date.clone()),
                    lookback_days: request.lookback_days,
                    disclosure_limit: request.disclosure_limit,
                    horizons: vec![request.horizon_days],
                    stop_loss_pct: request.stop_loss_pct,
                    target_return_pct: request.target_return_pct,
                    label_definition_version: request.label_definition_version.clone(),
                    // 2026-04-14 CST: 这里显式补空 external_proxy_inputs，原因是 future outcome
                    // 正式合同已支持外部代理输入，但训练链这轮仍只消费本地特征快照。
                    // 目的：先保证纯本地训练路径可编译、可运行，后续再接入更厚的信息面输入。
                    external_proxy_inputs: None,
                })?;
                let outcome = outcome_result
                    .forward_outcomes
                    .first()
                    .cloned()
                    .ok_or_else(|| {
                        SecurityScorecardTrainingError::Build(format!(
                            "missing forward outcome for {symbol} at {as_of_date}"
                        ))
                    })?;
                let merged_raw_features_json = outcome_result.snapshot.raw_features_json.clone();
                let _capital_source_observation =
                    load_capital_source_observation_row(request, symbol, &as_of_date)?;
                let feature_values =
                    extract_feature_values(&merged_raw_features_json, feature_configs)?;
                // 2026-04-21 CST: Added because repair_stable_head uses a future-path label that
                // cannot be read from the existing one-point forward outcome document.
                // Reason: the approved contract now asks whether oversold states repair and stay repaired.
                // Purpose: branch label resolution without changing legacy direction head semantics.
                let label = if request.target_head.trim() == "repair_stable_head" {
                    let current_bucket = merged_raw_features_json
                        .get("mean_reversion_deviation_20d")
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
                            SecurityScorecardTrainingError::Build(format!(
                                "snapshot missing mean_reversion_deviation_20d for {symbol} at {as_of_date}"
                            ))
                        })?;
                    let future_buckets = derive_future_mean_reversion_buckets(
                        &store,
                        symbol,
                        &as_of_date,
                        request.lookback_days,
                        request.horizon_days,
                    )?;
                    resolve_repair_stable_label_from_buckets(current_bucket, &future_buckets)
                        .ok_or_else(|| {
                            SecurityScorecardTrainingError::Build(format!(
                                "repair_stable_head requires lower-half start for {symbol} at {as_of_date}"
                            ))
                        })?
                } else {
                    resolve_training_label(&outcome, &request.target_head)
                };
                samples.push(TrainingSample {
                    symbol: symbol.clone(),
                    as_of_date: NaiveDate::parse_from_str(&as_of_date, "%Y-%m-%d").map_err(
                        |error| {
                            SecurityScorecardTrainingError::Build(format!(
                                "invalid as_of_date `{as_of_date}` for {symbol}: {error}"
                            ))
                        },
                    )?,
                    split_name: split_name.to_string(),
                    label,
                    feature_values,
                });
            }
        }
    }

    if samples.is_empty() {
        return Err(SecurityScorecardTrainingError::Build(
            "no samples were collected for the requested ranges".to_string(),
        ));
    }

    Ok(samples)
}

fn collect_weekly_samples(
    request: &SecurityScorecardTrainingRequest,
    train_range: &TrainingDateRange,
    _valid_range: &TrainingDateRange,
    test_range: &TrainingDateRange,
    feature_configs: &[TrainingFeatureConfig],
) -> Result<Vec<TrainingSample>, SecurityScorecardTrainingError> {
    let store = StockHistoryStore::workspace_default()?;
    let mut samples = Vec::new();
    let window_start = train_range.start.format("%Y-%m-%d").to_string();
    let window_end = test_range.end.format("%Y-%m-%d").to_string();

    for symbol in &request.symbol_list {
        let effective_routing = resolve_effective_security_routing(
            symbol,
            request.market_symbol.as_deref(),
            request.sector_symbol.as_deref(),
            request.market_profile.as_deref(),
            request.sector_profile.as_deref(),
        );
        let spot_rows = load_rows_for_weekly_training(&store, symbol, &window_start, &window_end)?;
        let futures_rows = if let Some(futures_symbol) = request
            .futures_symbol
            .as_deref()
            .map(str::trim)
            .filter(|symbol| !symbol.is_empty())
        {
            Some(load_rows_for_weekly_training(
                &store,
                futures_symbol,
                &window_start,
                &window_end,
            )?)
        } else {
            None
        };
        let volume_proxy_rows =
            if let Some(volume_proxy_symbol) = normalized_volume_proxy_symbol(request) {
                Some(load_rows_for_weekly_training(
                    &store,
                    volume_proxy_symbol,
                    &window_start,
                    &window_end,
                )?)
            } else {
                None
            };

        let weekly_feature_rows = build_weekly_price_feature_rows(
            &spot_rows,
            futures_rows.as_deref(),
            volume_proxy_rows.as_deref(),
        )?;
        let weekly_feature_map = weekly_feature_rows
            .into_iter()
            .map(|row| {
                let week_end = NaiveDate::parse_from_str(&row.week_end_date, "%Y-%m-%d")
                    .expect("weekly feature row week_end_date should parse");
                (week_end.iso_week(), row)
            })
            .collect::<BTreeMap<_, _>>();

        let anchor_dates = build_training_anchor_dates(
            request,
            &store,
            symbol,
            &spot_rows,
            train_range,
            test_range,
        )?;
        let rolling_windows = build_weekly_rolling_split_plan(&anchor_dates, 24, 1, 1, 1)?;

        let mut test_anchor_dates = std::collections::BTreeSet::<String>::new();
        let mut valid_anchor_dates = std::collections::BTreeSet::<String>::new();
        let mut train_anchor_dates = std::collections::BTreeSet::<String>::new();
        for rolling_window in rolling_windows {
            test_anchor_dates.extend(rolling_window.test_anchor_dates);
            valid_anchor_dates.extend(
                rolling_window
                    .valid_anchor_dates
                    .into_iter()
                    .filter(|date| !test_anchor_dates.contains(date)),
            );
            train_anchor_dates.extend(rolling_window.train_anchor_dates.into_iter().filter(
                |date| !valid_anchor_dates.contains(date) && !test_anchor_dates.contains(date),
            ));
        }

        for (split_name, anchor_dates) in [
            ("train".to_string(), train_anchor_dates),
            ("valid".to_string(), valid_anchor_dates),
            ("test".to_string(), test_anchor_dates),
        ] {
            for as_of_date in anchor_dates {
                let as_of_week = NaiveDate::parse_from_str(&as_of_date, "%Y-%m-%d")
                    .map_err(|error| {
                        SecurityScorecardTrainingError::Build(format!(
                            "invalid weekly anchor date `{as_of_date}` for {symbol}: {error}"
                        ))
                    })?
                    .iso_week();
                let weekly_features = weekly_feature_map.get(&as_of_week).ok_or_else(|| {
                    SecurityScorecardTrainingError::Build(format!(
                        "weekly feature row is missing for {symbol} at {as_of_date}"
                    ))
                })?;
                let merged_raw_features_json = weekly_features
                    .feature_values
                    .iter()
                    .map(|(feature_name, value)| (feature_name.clone(), Value::from(*value)))
                    .collect::<BTreeMap<_, _>>();
                match load_capital_source_observation_row(request, symbol, &as_of_date) {
                    Ok(Some(_observation_row)) => {}
                    Ok(None) => {}
                    Err(error) if should_skip_weekly_anchor_due_to_capital_history(&error) => {
                        continue;
                    }
                    Err(error) => return Err(error),
                }
                let outcome_result = security_forward_outcome(&SecurityForwardOutcomeRequest {
                    symbol: symbol.clone(),
                    market_symbol: effective_routing.market_symbol.clone(),
                    sector_symbol: effective_routing.sector_symbol.clone(),
                    futures_symbol: request.futures_symbol.clone(),
                    market_profile: effective_routing.market_profile.clone(),
                    sector_profile: effective_routing.sector_profile.clone(),
                    as_of_date: Some(as_of_date.clone()),
                    lookback_days: request.lookback_days,
                    disclosure_limit: request.disclosure_limit,
                    horizons: vec![request.horizon_days],
                    stop_loss_pct: request.stop_loss_pct,
                    target_return_pct: request.target_return_pct,
                    label_definition_version: request.label_definition_version.clone(),
                    external_proxy_inputs: None,
                })?;
                let outcome = outcome_result
                    .forward_outcomes
                    .first()
                    .cloned()
                    .ok_or_else(|| {
                        SecurityScorecardTrainingError::Build(format!(
                            "missing forward outcome for {symbol} at {as_of_date}"
                        ))
                    })?;
                let feature_values =
                    extract_feature_values(&merged_raw_features_json, feature_configs)?;
                let label = resolve_training_label(&outcome, &request.target_head);
                samples.push(TrainingSample {
                    symbol: symbol.clone(),
                    as_of_date: NaiveDate::parse_from_str(&as_of_date, "%Y-%m-%d").map_err(
                        |error| {
                            SecurityScorecardTrainingError::Build(format!(
                                "invalid weekly as_of_date `{as_of_date}` for {symbol}: {error}"
                            ))
                        },
                    )?,
                    split_name: split_name.clone(),
                    label,
                    feature_values,
                });
            }
        }
    }

    if samples.is_empty() {
        return Err(SecurityScorecardTrainingError::Build(
            "no weekly samples were collected for the requested ranges".to_string(),
        ));
    }
    Ok(samples)
}

fn build_weekly_training_context(
    request: &SecurityScorecardTrainingRequest,
    train_range: &TrainingDateRange,
    valid_range: &TrainingDateRange,
    test_range: &TrainingDateRange,
    feature_configs: &[TrainingFeatureConfig],
) -> Result<WeeklyTrainingContext, SecurityScorecardTrainingError> {
    let store = StockHistoryStore::workspace_default()?;
    let mut artifact_samples = Vec::new();
    let mut rolling_windows = Vec::new();
    let mut capital_source_observation_rows = Vec::new();
    let window_start = train_range.start.format("%Y-%m-%d").to_string();
    let window_end = test_range.end.format("%Y-%m-%d").to_string();

    for symbol in &request.symbol_list {
        let spot_rows = load_rows_for_weekly_training(&store, symbol, &window_start, &window_end)?;
        let futures_rows = if let Some(futures_symbol) = request
            .futures_symbol
            .as_deref()
            .map(str::trim)
            .filter(|symbol| !symbol.is_empty())
        {
            Some(load_rows_for_weekly_training(
                &store,
                futures_symbol,
                &window_start,
                &window_end,
            )?)
        } else {
            None
        };
        let volume_proxy_rows =
            if let Some(volume_proxy_symbol) = normalized_volume_proxy_symbol(request) {
                Some(load_rows_for_weekly_training(
                    &store,
                    volume_proxy_symbol,
                    &window_start,
                    &window_end,
                )?)
            } else {
                None
            };

        let weekly_feature_rows = build_weekly_price_feature_rows(
            &spot_rows,
            futures_rows.as_deref(),
            volume_proxy_rows.as_deref(),
        )?;
        let weekly_feature_map = weekly_feature_rows
            .into_iter()
            .map(|row| {
                let week_end = NaiveDate::parse_from_str(&row.week_end_date, "%Y-%m-%d")
                    .expect("weekly feature row week_end_date should parse");
                (week_end.iso_week(), row)
            })
            .collect::<BTreeMap<_, _>>();

        let anchor_dates = build_training_anchor_dates(
            request,
            &store,
            symbol,
            &spot_rows,
            train_range,
            test_range,
        )?;
        let rolling_plans = build_weekly_rolling_split_plan(&anchor_dates, 24, 1, 1, 1)?;
        let mut anchor_sample_map = BTreeMap::<String, TrainingSample>::new();
        let spot_close_by_date = spot_rows
            .iter()
            .map(|row| (row.trade_date.clone(), row.close))
            .collect::<BTreeMap<_, _>>();

        for (anchor_index, as_of_date) in anchor_dates.iter().enumerate() {
            let Some(next_anchor_date) = anchor_dates.get(anchor_index + 1) else {
                continue;
            };
            let as_of_week = NaiveDate::parse_from_str(as_of_date, "%Y-%m-%d")
                .map_err(|error| {
                    SecurityScorecardTrainingError::Build(format!(
                        "invalid weekly anchor date `{as_of_date}` for {symbol}: {error}"
                    ))
                })?
                .iso_week();
            let weekly_features = match weekly_feature_map.get(&as_of_week) {
                Some(features) => features,
                None => continue,
            };
            let merged_raw_features_json = weekly_features
                .feature_values
                .iter()
                .map(|(feature_name, value)| (feature_name.clone(), Value::from(*value)))
                .collect::<BTreeMap<_, _>>();
            match load_capital_source_observation_row(request, symbol, as_of_date) {
                Ok(Some(observation_row)) => {
                    capital_source_observation_rows.push(observation_row);
                }
                Ok(None) => {}
                Err(error) if should_skip_weekly_anchor_due_to_capital_history(&error) => {
                    continue;
                }
                Err(error) => return Err(error),
            }
            let feature_values =
                extract_feature_values(&merged_raw_features_json, feature_configs)?;
            let current_close = spot_close_by_date.get(as_of_date).copied().ok_or_else(|| {
                SecurityScorecardTrainingError::Build(format!(
                    "weekly spot close is missing for {symbol} at {as_of_date}"
                ))
            })?;
            let next_close = spot_close_by_date
                .get(next_anchor_date)
                .copied()
                .ok_or_else(|| {
                    SecurityScorecardTrainingError::Build(format!(
                        "weekly spot close is missing for {symbol} at next anchor {next_anchor_date}"
                    ))
                })?;
            let label =
                resolve_weekly_direction_label(current_close, next_close).ok_or_else(|| {
                    SecurityScorecardTrainingError::Build(format!(
                        "weekly direction label is unavailable for {symbol} at {as_of_date}"
                    ))
                })?;
            let parsed_date =
                NaiveDate::parse_from_str(as_of_date, "%Y-%m-%d").map_err(|error| {
                    SecurityScorecardTrainingError::Build(format!(
                        "invalid weekly as_of_date `{as_of_date}` for {symbol}: {error}"
                    ))
                })?;
            let split_name = classify_weekly_artifact_split(parsed_date, valid_range, test_range);
            let sample = TrainingSample {
                symbol: symbol.clone(),
                as_of_date: parsed_date,
                split_name: split_name.to_string(),
                label,
                feature_values,
            };
            anchor_sample_map.insert(as_of_date.clone(), sample.clone());
            artifact_samples.push(sample);
        }

        for (window_index, plan) in rolling_plans.into_iter().enumerate() {
            let Some(train_samples) =
                collect_weekly_window_samples(&anchor_sample_map, &plan.train_anchor_dates)
            else {
                continue;
            };
            let Some(valid_samples) =
                collect_weekly_window_samples(&anchor_sample_map, &plan.valid_anchor_dates)
            else {
                continue;
            };
            let Some(test_samples) =
                collect_weekly_window_samples(&anchor_sample_map, &plan.test_anchor_dates)
            else {
                continue;
            };
            rolling_windows.push(WeeklyRollingWindowSamples {
                window_id: window_index + 1,
                train_samples,
                valid_samples,
                test_samples,
            });
        }
    }

    if artifact_samples.is_empty() {
        return Err(SecurityScorecardTrainingError::Build(
            "no weekly samples were collected for the requested ranges".to_string(),
        ));
    }
    if rolling_windows.is_empty() {
        return Err(SecurityScorecardTrainingError::Build(
            "weekly rolling split plan did not yield any complete evaluation windows".to_string(),
        ));
    }

    artifact_samples.sort_by(|left, right| {
        left.as_of_date
            .cmp(&right.as_of_date)
            .then_with(|| left.symbol.cmp(&right.symbol))
    });

    Ok(WeeklyTrainingContext {
        artifact_samples,
        rolling_windows,
        capital_source_observation_rows,
    })
}

fn collect_weekly_window_samples(
    anchor_sample_map: &BTreeMap<String, TrainingSample>,
    anchor_dates: &[String],
) -> Option<Vec<TrainingSample>> {
    anchor_dates
        .iter()
        .map(|anchor_date| anchor_sample_map.get(anchor_date).cloned())
        .collect()
}

fn classify_weekly_artifact_split(
    as_of_date: NaiveDate,
    valid_range: &TrainingDateRange,
    test_range: &TrainingDateRange,
) -> &'static str {
    if as_of_date >= test_range.start && as_of_date <= test_range.end {
        "test"
    } else if as_of_date >= valid_range.start && as_of_date <= valid_range.end {
        "valid"
    } else {
        "train"
    }
}

fn should_skip_weekly_anchor_due_to_capital_history(
    error: &SecurityScorecardTrainingError,
) -> bool {
    match error {
        SecurityScorecardTrainingError::Build(message) => {
            message.contains("source history is insufficient")
                || message.contains("has no governed observations")
        }
        _ => false,
    }
}

fn load_rows_for_weekly_training(
    store: &StockHistoryStore,
    symbol: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<StockHistoryRow>, SecurityScorecardTrainingError> {
    let start = NaiveDate::parse_from_str(start_date, "%Y-%m-%d").map_err(|error| {
        SecurityScorecardTrainingError::Build(format!(
            "invalid weekly training start_date `{start_date}`: {error}"
        ))
    })?;
    let end = NaiveDate::parse_from_str(end_date, "%Y-%m-%d").map_err(|error| {
        SecurityScorecardTrainingError::Build(format!(
            "invalid weekly training end_date `{end_date}`: {error}"
        ))
    })?;
    let lookback_days = (end - start).num_days().unsigned_abs() as usize + 64;
    let rows = store.load_recent_rows(symbol, Some(end_date), lookback_days.max(64))?;
    Ok(rows
        .into_iter()
        .filter(|row| {
            NaiveDate::parse_from_str(&row.trade_date, "%Y-%m-%d")
                .map(|trade_date| trade_date >= start && trade_date <= end)
                .unwrap_or(false)
        })
        .collect())
}

fn build_training_anchor_dates(
    request: &SecurityScorecardTrainingRequest,
    store: &StockHistoryStore,
    symbol: &str,
    spot_rows: &[StockHistoryRow],
    train_range: &TrainingDateRange,
    test_range: &TrainingDateRange,
) -> Result<Vec<String>, SecurityScorecardTrainingError> {
    let start_date = train_range.start.format("%Y-%m-%d").to_string();
    let end_date = test_range.end.format("%Y-%m-%d").to_string();
    if let Some(capital_flow_runtime_root) = normalized_capital_flow_runtime_root(request) {
        let governed_weekly_dates = load_governed_weekly_observation_dates(
            &capital_flow_runtime_root,
            &start_date,
            &end_date,
        )?;
        if !governed_weekly_dates.is_empty() {
            return filter_weekly_training_anchor_dates(
                store,
                symbol,
                request,
                build_weekly_anchor_dates(
                    spot_rows,
                    &governed_weekly_dates,
                    &start_date,
                    &end_date,
                )?,
            );
        }
    }
    filter_weekly_training_anchor_dates(
        store,
        symbol,
        request,
        build_weekly_price_buckets(spot_rows)?
            .into_iter()
            .map(|bucket| bucket.week_end.format("%Y-%m-%d").to_string())
            .filter(|date| date >= &start_date && date <= &end_date)
            .collect(),
    )
}

fn filter_weekly_training_anchor_dates(
    store: &StockHistoryStore,
    symbol: &str,
    request: &SecurityScorecardTrainingRequest,
    anchor_dates: Vec<String>,
) -> Result<Vec<String>, SecurityScorecardTrainingError> {
    let mut filtered_anchor_dates = Vec::new();
    for (index, anchor_date) in anchor_dates.iter().enumerate() {
        let history_rows =
            store.load_recent_rows(symbol, Some(anchor_date), request.lookback_days.max(200))?;
        if history_rows.len() < 200 {
            continue;
        }
        if uses_nikkei_weekly_training_contract(request) {
            if index + 1 >= anchor_dates.len() {
                continue;
            }
        } else {
            let future_rows = store.load_forward_rows(symbol, anchor_date, request.horizon_days)?;
            if future_rows.len() < request.horizon_days {
                continue;
            }
        }
        filtered_anchor_dates.push(anchor_date.clone());
    }
    Ok(filtered_anchor_dates)
}

fn load_capital_source_observation_row(
    request: &SecurityScorecardTrainingRequest,
    symbol: &str,
    as_of_date: &str,
) -> Result<Option<CapitalSourceObservationRow>, SecurityScorecardTrainingError> {
    if !uses_nikkei_capital_source_feature_contract(request) {
        return Ok(None);
    }

    // 2026-04-24 CST: Updated because the approved route demotes capital-source
    // metrics from training features to observation-only output.
    // Reason: the user wants to review funding metrics manually before letting
    // them influence Nikkei weekly model fitting.
    // Purpose: keep one governed observation fetch path while removing model input coupling.
    let capital_flow_runtime_root =
        normalized_capital_flow_runtime_root(request).ok_or_else(|| {
            SecurityScorecardTrainingError::Build(
                "capital_flow_runtime_root is required when capital_source_feature_mode is enabled"
                    .to_string(),
            )
        })?;
    let snapshot =
        security_capital_source_factor_snapshot(&SecurityCapitalSourceFactorSnapshotRequest {
            symbol: symbol.to_string(),
            as_of_date: as_of_date.to_string(),
            capital_flow_runtime_root: Some(capital_flow_runtime_root.clone()),
            price_history_runtime_root: None,
        })?;
    if snapshot.observation_dates.is_empty() {
        return Err(SecurityScorecardTrainingError::Build(format!(
            "capital-flow runtime root `{capital_flow_runtime_root}` has no governed observations for {symbol} at {as_of_date}"
        )));
    }

    let parsed_date = NaiveDate::parse_from_str(as_of_date, "%Y-%m-%d").map_err(|error| {
        SecurityScorecardTrainingError::Build(format!(
            "invalid capital-source observation date `{as_of_date}` for {symbol}: {error}"
        ))
    })?;
    let factor_values = snapshot
        .factors
        .iter()
        .filter_map(|(feature_name, factor)| {
            factor.value.map(|value| (feature_name.clone(), value))
        })
        .collect::<BTreeMap<_, _>>();

    // 2026-04-24 CST: Updated because capital-source metrics now live in the
    // observation-only layer instead of the training feature contract.
    // Reason: requiring every observation metric to be available would wrongly
    // drop otherwise valid weekly samples when one exploratory metric is sparse.
    // Purpose: keep training sample eligibility independent from observation sparsity.
    for (feature_name, numeric_value) in &factor_values {
        if !numeric_value.is_finite() {
            return Err(SecurityScorecardTrainingError::Build(format!(
                "capital-source feature `{feature_name}` is invalid for {symbol} at {as_of_date}"
            )));
        }
    }

    Ok(Some(CapitalSourceObservationRow {
        symbol: symbol.to_string(),
        as_of_date: parsed_date,
        observation_dates: snapshot.observation_dates,
        factor_values,
    }))
}

fn load_dates_in_range(
    store: &StockHistoryStore,
    symbol: &str,
    range: &TrainingDateRange,
    min_history_rows: usize,
    min_future_rows: usize,
) -> Result<Vec<String>, SecurityScorecardTrainingError> {
    let end_text = range.end.format("%Y-%m-%d").to_string();
    let lookback_days = (range.end - range.start).num_days().unsigned_abs() as usize + 32;
    let rows = store.load_recent_rows(symbol, Some(&end_text), lookback_days.max(32))?;

    let mut qualified_dates = Vec::new();
    for row in rows {
        let is_in_range = NaiveDate::parse_from_str(&row.trade_date, "%Y-%m-%d")
            .map(|trade_date| trade_date >= range.start && trade_date <= range.end)
            .unwrap_or(false);
        if !is_in_range {
            continue;
        }
        let history_rows =
            store.load_recent_rows(symbol, Some(&row.trade_date), min_history_rows)?;
        if history_rows.len() >= min_history_rows {
            let future_rows = store.load_forward_rows(symbol, &row.trade_date, min_future_rows)?;
            if future_rows.len() >= min_future_rows {
                qualified_dates.push(row.trade_date);
            }
        }
    }

    Ok(qualified_dates)
}

fn build_split_target_count(split_name: &str, range: &TrainingDateRange) -> usize {
    // 2026-04-20 CST: Added because the user approved decade-scale Nikkei training
    // and the old fixed 2/1/1 split counts left almost all of the history unused.
    // Purpose: densify sampling with one stable cadence rule while keeping short-window requests valid.
    let range_days = (range.end - range.start).num_days().unsigned_abs() as usize + 1;
    let cadence_days = match split_name {
        "train" => 28,
        "valid" | "test" => 21,
        _ => 28,
    };
    let minimum = match split_name {
        "train" => 2,
        _ => 1,
    };
    ((range_days + cadence_days - 1) / cadence_days).max(minimum)
}

fn select_evenly_spaced_dates(dates: &[String], target_count: usize) -> Vec<String> {
    if target_count == 0 || dates.is_empty() {
        return Vec::new();
    }
    if dates.len() <= target_count {
        return dates.to_vec();
    }

    let mut selected = Vec::new();
    for index in 0..target_count {
        let position = if target_count == 1 {
            dates.len() - 1
        } else {
            index * (dates.len() - 1) / (target_count - 1)
        };
        let candidate = dates[position].clone();
        if !selected.contains(&candidate) {
            selected.push(candidate);
        }
    }
    selected
}

fn extract_feature_values(
    raw_features_json: &BTreeMap<String, Value>,
    feature_configs: &[TrainingFeatureConfig],
) -> Result<BTreeMap<String, TrainingFeatureValue>, SecurityScorecardTrainingError> {
    let mut feature_values = BTreeMap::new();
    for config in feature_configs {
        let value = raw_features_json.get(config.feature_name).ok_or_else(|| {
            SecurityScorecardTrainingError::Build(format!(
                "snapshot missing feature `{}`",
                config.feature_name
            ))
        })?;
        let feature_value = match config.kind {
            TrainingFeatureKind::Numeric => value
                .as_f64()
                .or_else(|| value.as_i64().map(|number| number as f64))
                .map(TrainingFeatureValue::Numeric)
                .ok_or_else(|| {
                    SecurityScorecardTrainingError::Build(format!(
                        "feature `{}` is not numeric",
                        config.feature_name
                    ))
                })?,
            TrainingFeatureKind::Categorical => match value {
                Value::String(text) => TrainingFeatureValue::Category(text.clone()),
                Value::Bool(flag) => TrainingFeatureValue::Category(flag.to_string()),
                Value::Null => TrainingFeatureValue::Category("__missing__".to_string()),
                _ => TrainingFeatureValue::Category(value.to_string()),
            },
        };
        feature_values.insert(config.feature_name.to_string(), feature_value);
    }
    Ok(feature_values)
}

fn samples_for_split<'a>(
    samples: &'a [TrainingSample],
    split_name: &str,
) -> Vec<&'a TrainingSample> {
    samples
        .iter()
        .filter(|sample| sample.split_name == split_name)
        .collect()
}

fn build_feature_models(
    train_samples: &[&TrainingSample],
    feature_configs: &[TrainingFeatureConfig],
) -> Result<Vec<FeatureModel>, SecurityScorecardTrainingError> {
    let total_positive = train_samples
        .iter()
        .filter(|sample| sample.label >= 0.5)
        .count() as f64;
    let total_negative = train_samples.len() as f64 - total_positive;

    feature_configs
        .iter()
        .map(|config| {
            let bins = match config.kind {
                TrainingFeatureKind::Categorical => build_categorical_bins(
                    train_samples,
                    config.feature_name,
                    total_positive,
                    total_negative,
                )?,
                TrainingFeatureKind::Numeric => build_numeric_bins(
                    train_samples,
                    config.feature_name,
                    total_positive,
                    total_negative,
                )?,
            };
            Ok(FeatureModel {
                feature_name: config.feature_name.to_string(),
                group_name: config.group_name.to_string(),
                kind: config.kind.clone(),
                bins,
            })
        })
        .collect()
}

fn build_categorical_bins(
    train_samples: &[&TrainingSample],
    feature_name: &str,
    total_positive: f64,
    total_negative: f64,
) -> Result<Vec<FeatureBinModel>, SecurityScorecardTrainingError> {
    let mut bucket_counts: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    for sample in train_samples {
        let TrainingFeatureValue::Category(value) = sample
            .feature_values
            .get(feature_name)
            .cloned()
            .ok_or_else(|| {
                SecurityScorecardTrainingError::Build(format!(
                    "sample missing categorical feature `{feature_name}`"
                ))
            })?
        else {
            return Err(SecurityScorecardTrainingError::Build(format!(
                "feature `{feature_name}` expected categorical value"
            )));
        };
        let entry = bucket_counts.entry(value).or_insert((0.0, 0.0));
        if sample.label >= 0.5 {
            entry.0 += 1.0;
        } else {
            entry.1 += 1.0;
        }
    }

    let mut bins = bucket_counts
        .into_iter()
        .map(
            |(value, (positive_count, negative_count))| FeatureBinModel {
                bin_label: value.clone(),
                match_values: vec![value],
                min_inclusive: None,
                max_exclusive: None,
                woe: compute_woe(
                    positive_count,
                    negative_count,
                    total_positive,
                    total_negative,
                ),
                positive_count,
                negative_count,
                sample_count: positive_count + negative_count,
            },
        )
        .collect::<Vec<_>>();
    // 2026-04-17 CST: Added because diagnostic encoding now touches valid/test rows after
    // train-only categorical binning has completed.
    // Reason: real reruns can introduce train-unseen categorical values and previously crashed
    // after artifact persistence.
    // Purpose: append one explicit neutral fallback bin so unseen categorical values stay
    // inspectable without aborting the governed training run.
    bins.push(build_unseen_categorical_bin());
    Ok(bins)
}

fn build_numeric_bins(
    train_samples: &[&TrainingSample],
    feature_name: &str,
    total_positive: f64,
    total_negative: f64,
) -> Result<Vec<FeatureBinModel>, SecurityScorecardTrainingError> {
    let numeric_values = train_samples
        .iter()
        .map(|sample| {
            let TrainingFeatureValue::Numeric(value) = sample
                .feature_values
                .get(feature_name)
                .cloned()
                .ok_or_else(|| {
                    SecurityScorecardTrainingError::Build(format!(
                        "sample missing numeric feature `{feature_name}`"
                    ))
                })?
            else {
                return Err(SecurityScorecardTrainingError::Build(format!(
                    "feature `{feature_name}` expected numeric value"
                )));
            };
            Ok(value)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let thresholds = build_numeric_thresholds(&numeric_values);
    let template_bins = build_numeric_intervals(&thresholds);
    let mut bucket_counts = vec![(0.0_f64, 0.0_f64); template_bins.len()];

    for sample in train_samples {
        let TrainingFeatureValue::Numeric(value) = sample
            .feature_values
            .get(feature_name)
            .cloned()
            .ok_or_else(|| {
                SecurityScorecardTrainingError::Build(format!(
                    "sample missing numeric feature `{feature_name}`"
                ))
            })?
        else {
            return Err(SecurityScorecardTrainingError::Build(format!(
                "feature `{feature_name}` expected numeric value"
            )));
        };
        let Some((index, _)) = template_bins
            .iter()
            .enumerate()
            .find(|(_, bin)| numeric_bin_matches(bin, value))
        else {
            return Err(SecurityScorecardTrainingError::Build(format!(
                "no numeric bin matched feature `{feature_name}` value {value}"
            )));
        };
        if sample.label >= 0.5 {
            bucket_counts[index].0 += 1.0;
        } else {
            bucket_counts[index].1 += 1.0;
        }
    }

    Ok(template_bins
        .into_iter()
        .enumerate()
        .map(|(index, bin)| FeatureBinModel {
            bin_label: bin.bin_label,
            match_values: Vec::new(),
            min_inclusive: bin.min_inclusive,
            max_exclusive: bin.max_exclusive,
            woe: compute_woe(
                bucket_counts[index].0,
                bucket_counts[index].1,
                total_positive,
                total_negative,
            ),
            positive_count: bucket_counts[index].0,
            negative_count: bucket_counts[index].1,
            sample_count: bucket_counts[index].0 + bucket_counts[index].1,
        })
        .collect())
}

fn build_numeric_thresholds(values: &[f64]) -> Vec<f64> {
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    sorted.dedup_by(|left, right| (*left - *right).abs() <= 1e-9);
    if sorted.len() <= 1 {
        return Vec::new();
    }

    let mut thresholds = vec![sorted[sorted.len() / 3], sorted[(sorted.len() * 2) / 3]];
    thresholds.sort_by(|left, right| left.total_cmp(right));
    thresholds.dedup_by(|left, right| (*left - *right).abs() <= 1e-9);
    thresholds
}

fn build_numeric_intervals(thresholds: &[f64]) -> Vec<FeatureBinModel> {
    if thresholds.is_empty() {
        return vec![FeatureBinModel {
            bin_label: "all".to_string(),
            match_values: Vec::new(),
            min_inclusive: None,
            max_exclusive: None,
            woe: 0.0,
            positive_count: 0.0,
            negative_count: 0.0,
            sample_count: 0.0,
        }];
    }

    let mut bins = Vec::new();
    let mut lower = None;
    for (index, threshold) in thresholds.iter().enumerate() {
        bins.push(FeatureBinModel {
            bin_label: format!("bin_{}", index + 1),
            match_values: Vec::new(),
            min_inclusive: lower,
            max_exclusive: Some(*threshold),
            woe: 0.0,
            positive_count: 0.0,
            negative_count: 0.0,
            sample_count: 0.0,
        });
        lower = Some(*threshold);
    }
    bins.push(FeatureBinModel {
        bin_label: format!("bin_{}", thresholds.len() + 1),
        match_values: Vec::new(),
        min_inclusive: lower,
        max_exclusive: None,
        woe: 0.0,
        positive_count: 0.0,
        negative_count: 0.0,
        sample_count: 0.0,
    });
    bins
}

fn numeric_bin_matches(bin: &FeatureBinModel, value: f64) -> bool {
    let lower_match = bin.min_inclusive.map(|min| value >= min).unwrap_or(true);
    let upper_match = bin.max_exclusive.map(|max| value < max).unwrap_or(true);
    lower_match && upper_match
}

fn compute_woe(
    positive_count: f64,
    negative_count: f64,
    total_positive: f64,
    total_negative: f64,
) -> f64 {
    let smooth = 0.5;
    let positive_rate = (positive_count + smooth) / (total_positive + smooth * 2.0);
    let negative_rate = (negative_count + smooth) / (total_negative + smooth * 2.0);
    (positive_rate / negative_rate).ln()
}

fn build_unseen_categorical_bin() -> FeatureBinModel {
    FeatureBinModel {
        bin_label: UNSEEN_CATEGORICAL_BIN_LABEL.to_string(),
        match_values: Vec::new(),
        min_inclusive: None,
        max_exclusive: None,
        // 2026-04-17 CST: Added because unseen categorical values should not inherit directional
        // bias from class imbalance when the trainer has never observed that category in train.
        // Reason: a smoothed pseudo-count WOE would still fabricate signal for unknown states.
        // Purpose: keep the fallback bin neutral while making the contract explicit in artifacts
        // and diagnostics.
        woe: 0.0,
        positive_count: 0.0,
        negative_count: 0.0,
        sample_count: 0.0,
    }
}

fn encode_samples(
    samples: &[&TrainingSample],
    feature_models: &[FeatureModel],
) -> Result<Vec<(Vec<f64>, f64)>, SecurityScorecardTrainingError> {
    samples
        .iter()
        .map(|sample| {
            let mut row = vec![1.0_f64];
            for feature_model in feature_models {
                row.push(resolve_feature_woe(feature_model, sample)?);
            }
            Ok((row, sample.label))
        })
        .collect()
}

fn resolve_feature_woe(
    feature_model: &FeatureModel,
    sample: &TrainingSample,
) -> Result<f64, SecurityScorecardTrainingError> {
    let value = sample
        .feature_values
        .get(&feature_model.feature_name)
        .ok_or_else(|| {
            SecurityScorecardTrainingError::Build(format!(
                "sample missing feature `{}`",
                feature_model.feature_name
            ))
        })?;

    match (&feature_model.kind, value) {
        (TrainingFeatureKind::Categorical, TrainingFeatureValue::Category(category)) => {
            feature_model
                .bins
                .iter()
                .find(|bin| {
                    bin.match_values
                        .iter()
                        .any(|candidate| candidate == category)
                })
                .map(|bin| bin.woe)
                .or_else(|| {
                    feature_model
                        .bins
                        .iter()
                        .find(|bin| bin.bin_label == UNSEEN_CATEGORICAL_BIN_LABEL)
                        .map(|bin| bin.woe)
                })
                .ok_or_else(|| {
                    SecurityScorecardTrainingError::Build(format!(
                        "no categorical bin matched feature `{}` value `{category}`",
                        feature_model.feature_name
                    ))
                })
        }
        (TrainingFeatureKind::Numeric, TrainingFeatureValue::Numeric(number)) => feature_model
            .bins
            .iter()
            .find(|bin| numeric_bin_matches(bin, *number))
            .map(|bin| bin.woe)
            .ok_or_else(|| {
                SecurityScorecardTrainingError::Build(format!(
                    "no numeric bin matched feature `{}` value {}",
                    feature_model.feature_name, number
                ))
            }),
        _ => Err(SecurityScorecardTrainingError::Build(format!(
            "feature `{}` kind mismatch",
            feature_model.feature_name
        ))),
    }
}

// 2026-04-09 CST: 这里使用最小批量梯度下降拟合 logistic，原因是 Task 5 首版只要求纯 Rust 的轻量闭环，不提前引入额外训练框架；
// 目的：先稳定产出可回放的 coefficient artifact，为后续更复杂的 walk-forward 和晋级治理打底。
fn train_logistic_model(encoded_train_rows: &[(Vec<f64>, f64)]) -> TrainedLogisticModel {
    let parameter_count = encoded_train_rows
        .first()
        .map(|(row, _)| row.len())
        .unwrap_or(1);
    let mut beta = vec![0.0_f64; parameter_count];
    let average_norm = encoded_train_rows
        .iter()
        .map(|(row, _)| row.iter().map(|value| value * value).sum::<f64>())
        .sum::<f64>()
        / encoded_train_rows.len() as f64;
    let learning_rate = 1.0 / average_norm.max(1.0);

    for _ in 0..10_000 {
        let mut gradient = vec![0.0_f64; parameter_count];
        for (row, label) in encoded_train_rows {
            let prediction = logistic(dot(row, &beta));
            let error = prediction - *label;
            for (index, value) in row.iter().enumerate() {
                gradient[index] += error * value;
            }
        }
        let mut max_change = 0.0_f64;
        for index in 0..parameter_count {
            let step = learning_rate * gradient[index] / encoded_train_rows.len() as f64;
            beta[index] -= step;
            max_change = max_change.max(step.abs());
        }
        if max_change <= 1e-8 {
            break;
        }
    }

    TrainedLogisticModel {
        intercept: beta[0],
        coefficients: beta.into_iter().skip(1).collect(),
    }
}

// 2026-04-17 CST: Added because the current training output must explain process metrics before
// any model-family upgrade is considered.
// Reason: the user explicitly asked for feature correlation, coefficient exposure, drift, and
// walk-forward stability instead of one aggregate accuracy number.
// Purpose: build one governed diagnostic report that stays local to the training chain.
fn build_training_diagnostic_report(
    request: &SecurityScorecardTrainingRequest,
    samples: &[TrainingSample],
    feature_configs: &[TrainingFeatureConfig],
    feature_models: &[FeatureModel],
    trained_model: &TrainedLogisticModel,
) -> Result<Value, SecurityScorecardTrainingError> {
    let sample_refs = samples.iter().collect::<Vec<_>>();
    let encoded_samples = encode_diagnostic_samples(&sample_refs, feature_models, trained_model)?;
    let feature_coverage_summary =
        build_feature_coverage_summary(samples, feature_models, trained_model);
    let feature_influence_summary = build_feature_influence_summary(feature_models, trained_model);
    let correlation_summary = build_correlation_summary(feature_models, &encoded_samples);
    let drift_summary = build_drift_summary(feature_models, &encoded_samples);
    let walk_forward_summary = build_walk_forward_summary(feature_configs, samples)?;
    let segment_slice_summary =
        build_segment_slice_summary(samples, feature_models, trained_model)?;
    let readiness_assessment = build_readiness_assessment(
        samples,
        feature_models,
        &correlation_summary,
        &feature_influence_summary,
        &walk_forward_summary,
    );
    let summary = json!({
        "feature_coverage_summary": feature_coverage_summary.clone(),
        "feature_influence_summary": feature_influence_summary.clone(),
        "correlation_summary": correlation_summary.clone(),
        "drift_summary": drift_summary.clone(),
        "walk_forward_summary": walk_forward_summary.clone(),
        "segment_slice_summary": segment_slice_summary.clone(),
        "readiness_assessment": readiness_assessment.clone(),
    });

    Ok(json!({
        "contract_version": "security_scorecard_training_diagnostic_report.v1",
        "document_type": "security_scorecard_training_diagnostic_report",
        "created_at": request.created_at,
        "model_id": build_governed_model_id(request),
        "model_version": format!("candidate_{}", sanitize_identifier(&request.created_at)),
        "training_window": request.train_range,
        "validation_window": request.valid_range,
        "oot_window": request.test_range,
        "feature_coverage_summary": feature_coverage_summary,
        "feature_influence_summary": feature_influence_summary,
        "correlation_summary": correlation_summary,
        "drift_summary": drift_summary,
        "walk_forward_summary": walk_forward_summary,
        "segment_slice_summary": segment_slice_summary,
        "readiness_assessment": readiness_assessment,
        "summary": summary,
    }))
}

// 2026-04-17 CST: Added because correlation, drift, and walk-forward all need one consistent
// encoded feature surface instead of recomputing ad hoc per diagnostic.
// Reason: the old trainer only exposed encoded rows during fitting.
// Purpose: retain one inspectable encoded sample view without changing predictions.
fn encode_diagnostic_samples(
    samples: &[&TrainingSample],
    feature_models: &[FeatureModel],
    trained_model: &TrainedLogisticModel,
) -> Result<Vec<EncodedDiagnosticSample>, SecurityScorecardTrainingError> {
    samples
        .iter()
        .map(|sample| {
            let mut encoded_features = Vec::with_capacity(feature_models.len());
            for feature_model in feature_models {
                encoded_features.push(resolve_feature_woe(feature_model, sample)?);
            }
            Ok(EncodedDiagnosticSample {
                symbol: sample.symbol.clone(),
                as_of_date: sample.as_of_date,
                split_name: sample.split_name.clone(),
                label: sample.label,
                predicted_probability: predict_probability(sample, feature_models, trained_model)?,
                encoded_features,
            })
        })
        .collect()
}

fn build_feature_coverage_summary(
    samples: &[TrainingSample],
    feature_models: &[FeatureModel],
    trained_model: &TrainedLogisticModel,
) -> Value {
    let train_sample_count = samples_for_split(samples, "train").len() as f64;
    let train_positive = feature_models
        .first()
        .map(|feature_model| {
            feature_model
                .bins
                .iter()
                .map(|bin| bin.positive_count)
                .sum::<f64>()
        })
        .unwrap_or(0.0);
    let train_negative = feature_models
        .first()
        .map(|feature_model| {
            feature_model
                .bins
                .iter()
                .map(|bin| bin.negative_count)
                .sum::<f64>()
        })
        .unwrap_or(0.0);

    let features = feature_models
        .iter()
        .enumerate()
        .map(|(index, feature_model)| {
            let coefficient = trained_model
                .coefficients
                .get(index)
                .copied()
                .unwrap_or(0.0);
            let missing_count = samples
                .iter()
                .filter(|sample| {
                    matches!(
                        sample.feature_values.get(&feature_model.feature_name),
                        Some(TrainingFeatureValue::Category(value)) if value == "__missing__"
                    )
                })
                .count();
            let distinct_observed_values = samples
                .iter()
                .filter_map(|sample| sample.feature_values.get(&feature_model.feature_name))
                .map(feature_value_as_key)
                .map(|value| (value, true))
                .collect::<BTreeMap<_, _>>()
                .len();
            let dominant_bin_share_train = feature_model
                .bins
                .iter()
                .map(|bin| bin.sample_count)
                .fold(0.0_f64, f64::max)
                / train_sample_count.max(1.0);
            let information_value = feature_model
                .bins
                .iter()
                .map(|bin| compute_information_value(bin, train_positive, train_negative))
                .sum::<f64>();

            json!({
                "feature_name": feature_model.feature_name,
                "group_name": feature_model.group_name,
                "kind": feature_kind_name(&feature_model.kind),
                "coefficient": coefficient,
                "absolute_coefficient": coefficient.abs(),
                "distinct_observed_values": distinct_observed_values,
                "missing_count": missing_count,
                "missing_rate": missing_count as f64 / samples.len().max(1) as f64,
                "train_bin_count": feature_model.bins.len(),
                "dominant_bin_share_train": dominant_bin_share_train,
                "information_value": information_value,
            })
        })
        .collect::<Vec<_>>();

    json!({
        "feature_count": feature_models.len(),
        "train_sample_count": train_sample_count,
        "features": features,
    })
}

fn build_feature_influence_summary(
    feature_models: &[FeatureModel],
    trained_model: &TrainedLogisticModel,
) -> Value {
    let feature_coefficients = feature_models
        .iter()
        .enumerate()
        .map(|(index, feature_model)| {
            let coefficient = trained_model
                .coefficients
                .get(index)
                .copied()
                .unwrap_or(0.0);
            json!({
                "feature_name": feature_model.feature_name,
                "group_name": feature_model.group_name,
                "coefficient": coefficient,
                "absolute_coefficient": coefficient.abs(),
            })
        })
        .collect::<Vec<_>>();

    let mut bin_contributions = Vec::new();
    for (index, feature_model) in feature_models.iter().enumerate() {
        let coefficient = trained_model
            .coefficients
            .get(index)
            .copied()
            .unwrap_or(0.0);
        let total_support = feature_model
            .bins
            .iter()
            .map(|bin| bin.sample_count)
            .sum::<f64>()
            .max(1.0);
        for bin in &feature_model.bins {
            bin_contributions.push(json!({
                "feature_name": feature_model.feature_name,
                "group_name": feature_model.group_name,
                "bin_label": bin.bin_label,
                "coefficient": coefficient,
                "woe": bin.woe,
                "logit_contribution": coefficient * bin.woe,
                "support_share_train": bin.sample_count / total_support,
                "sample_count_train": bin.sample_count,
            }));
        }
    }
    let mut top_positive_bins = bin_contributions.clone();
    top_positive_bins.sort_by(|left, right| {
        value_f64(right, "logit_contribution").total_cmp(&value_f64(left, "logit_contribution"))
    });
    top_positive_bins.truncate(12);

    let mut top_negative_bins = bin_contributions;
    top_negative_bins.sort_by(|left, right| {
        value_f64(left, "logit_contribution").total_cmp(&value_f64(right, "logit_contribution"))
    });
    top_negative_bins.truncate(12);

    let counterintuitive_bins = build_counterintuitive_bin_warnings(feature_models, trained_model);

    json!({
        "feature_coefficients": feature_coefficients,
        "top_positive_bins": top_positive_bins,
        "top_negative_bins": top_negative_bins,
        "counterintuitive_bins": counterintuitive_bins,
    })
}

fn build_counterintuitive_bin_warnings(
    feature_models: &[FeatureModel],
    trained_model: &TrainedLogisticModel,
) -> Vec<Value> {
    let mut warnings = Vec::new();
    for (index, feature_model) in feature_models.iter().enumerate() {
        let coefficient = trained_model
            .coefficients
            .get(index)
            .copied()
            .unwrap_or(0.0);
        let bins = &feature_model.bins;
        if bins.is_empty() {
            continue;
        }
        match feature_model.feature_name.as_str() {
            "hard_risk_score"
            | "negative_attention_score"
            | "disclosure_risk_keyword_count"
            | "data_gap_count"
            | "risk_note_count" => {
                if let Some(bin) = bins.last() {
                    let contribution = coefficient * bin.woe;
                    if contribution > 0.0 {
                        warnings.push(json!({
                            "feature_name": feature_model.feature_name,
                            "bin_label": bin.bin_label,
                            "expected_direction": "highest_bin_should_reduce_logit",
                            "observed_logit_contribution": contribution,
                        }));
                    }
                }
            }
            "positive_support_score" => {
                if let Some(bin) = bins.last() {
                    let contribution = coefficient * bin.woe;
                    if contribution < 0.0 {
                        warnings.push(json!({
                            "feature_name": feature_model.feature_name,
                            "bin_label": bin.bin_label,
                            "expected_direction": "highest_bin_should_raise_logit",
                            "observed_logit_contribution": contribution,
                        }));
                    }
                }
            }
            "event_net_impact_score" => {
                if let Some(lowest_bin) = bins.first() {
                    let contribution = coefficient * lowest_bin.woe;
                    if contribution > 0.0 {
                        warnings.push(json!({
                            "feature_name": feature_model.feature_name,
                            "bin_label": lowest_bin.bin_label,
                            "expected_direction": "lowest_bin_should_reduce_logit",
                            "observed_logit_contribution": contribution,
                        }));
                    }
                }
                if let Some(highest_bin) = bins.last() {
                    let contribution = coefficient * highest_bin.woe;
                    if contribution < 0.0 {
                        warnings.push(json!({
                            "feature_name": feature_model.feature_name,
                            "bin_label": highest_bin.bin_label,
                            "expected_direction": "highest_bin_should_raise_logit",
                            "observed_logit_contribution": contribution,
                        }));
                    }
                }
            }
            _ => {}
        }
    }
    warnings
}

fn build_correlation_summary(
    feature_models: &[FeatureModel],
    encoded_samples: &[EncodedDiagnosticSample],
) -> Value {
    let mut high_pairs = Vec::new();
    let mut max_abs_correlation = 0.0_f64;
    let mut zero_variance_features = Vec::new();

    let feature_columns = feature_models
        .iter()
        .enumerate()
        .map(|(index, feature_model)| {
            let values = encoded_samples
                .iter()
                .filter_map(|sample| sample.encoded_features.get(index).copied())
                .collect::<Vec<_>>();
            (feature_model.feature_name.clone(), values)
        })
        .collect::<Vec<_>>();

    let variances = feature_columns
        .iter()
        .map(|(feature_name, values)| {
            let variance = compute_variance(values);
            if variance <= 1e-12 {
                zero_variance_features.push(feature_name.clone());
            }
            variance
        })
        .collect::<Vec<_>>();

    for left_index in 0..feature_columns.len() {
        for right_index in (left_index + 1)..feature_columns.len() {
            if variances[left_index] <= 1e-12 || variances[right_index] <= 1e-12 {
                continue;
            }
            let correlation = compute_pearson_correlation(
                &feature_columns[left_index].1,
                &feature_columns[right_index].1,
            );
            let absolute = correlation.abs();
            max_abs_correlation = max_abs_correlation.max(absolute);
            if absolute >= 0.85 {
                high_pairs.push(json!({
                    "left_feature": feature_columns[left_index].0,
                    "right_feature": feature_columns[right_index].0,
                    "correlation": correlation,
                    "absolute_correlation": absolute,
                }));
            }
        }
    }
    high_pairs.sort_by(|left, right| {
        value_f64(right, "absolute_correlation").total_cmp(&value_f64(left, "absolute_correlation"))
    });
    high_pairs.truncate(15);

    json!({
        "encoding_basis": "woe_encoded_all_samples",
        "sample_count": encoded_samples.len(),
        "pair_count": feature_models.len() * feature_models.len().saturating_sub(1) / 2,
        "high_correlation_pair_count": high_pairs.len(),
        "max_absolute_correlation": max_abs_correlation,
        "zero_variance_features": zero_variance_features,
        "high_correlation_pairs": high_pairs,
    })
}

fn build_drift_summary(
    feature_models: &[FeatureModel],
    encoded_samples: &[EncodedDiagnosticSample],
) -> Value {
    json!({
        "encoding_basis": "mean_abs_woe_shift",
        "train_valid": build_split_shift_summary("train", "valid", feature_models, encoded_samples),
        "valid_test": build_split_shift_summary("valid", "test", feature_models, encoded_samples),
    })
}

fn build_split_shift_summary(
    left_split: &str,
    right_split: &str,
    feature_models: &[FeatureModel],
    encoded_samples: &[EncodedDiagnosticSample],
) -> Value {
    let left_samples = encoded_samples
        .iter()
        .filter(|sample| sample.split_name == left_split)
        .collect::<Vec<_>>();
    let right_samples = encoded_samples
        .iter()
        .filter(|sample| sample.split_name == right_split)
        .collect::<Vec<_>>();
    if left_samples.is_empty() || right_samples.is_empty() {
        return json!({
            "left_split": left_split,
            "right_split": right_split,
            "status": "insufficient_samples",
            "feature_shifts": [],
        });
    }

    let mut feature_shifts = feature_models
        .iter()
        .enumerate()
        .map(|(index, feature_model)| {
            let left_mean = mean_of(
                &left_samples
                    .iter()
                    .filter_map(|sample| sample.encoded_features.get(index).copied())
                    .collect::<Vec<_>>(),
            );
            let right_mean = mean_of(
                &right_samples
                    .iter()
                    .filter_map(|sample| sample.encoded_features.get(index).copied())
                    .collect::<Vec<_>>(),
            );
            json!({
                "feature_name": feature_model.feature_name,
                "group_name": feature_model.group_name,
                "left_mean": left_mean,
                "right_mean": right_mean,
                "absolute_mean_shift": (left_mean - right_mean).abs(),
            })
        })
        .collect::<Vec<_>>();

    feature_shifts.sort_by(|left, right| {
        value_f64(right, "absolute_mean_shift").total_cmp(&value_f64(left, "absolute_mean_shift"))
    });

    let average_shift = feature_shifts
        .iter()
        .map(|entry| value_f64(entry, "absolute_mean_shift"))
        .sum::<f64>()
        / feature_shifts.len().max(1) as f64;
    let max_shift = feature_shifts
        .iter()
        .map(|entry| value_f64(entry, "absolute_mean_shift"))
        .fold(0.0_f64, f64::max);

    feature_shifts.truncate(12);
    json!({
        "left_split": left_split,
        "right_split": right_split,
        "status": "ok",
        "average_absolute_mean_shift": average_shift,
        "max_absolute_mean_shift": max_shift,
        "feature_shifts": feature_shifts,
    })
}

fn build_walk_forward_summary(
    feature_configs: &[TrainingFeatureConfig],
    samples: &[TrainingSample],
) -> Result<Value, SecurityScorecardTrainingError> {
    let mut ordered_samples = samples.iter().collect::<Vec<_>>();
    ordered_samples.sort_by(|left, right| {
        left.as_of_date
            .cmp(&right.as_of_date)
            .then_with(|| left.symbol.cmp(&right.symbol))
    });

    if ordered_samples.len() < 6 {
        return Ok(json!({
            "status": "insufficient_samples",
            "sample_count": ordered_samples.len(),
            "fold_count": 0,
            "executed_folds": [],
            "skipped_folds": [],
        }));
    }

    let validation_window_size = (ordered_samples.len() / 4).max(1);
    let minimum_train_count = (ordered_samples.len() / 2)
        .max(4)
        .min(ordered_samples.len().saturating_sub(validation_window_size));
    let mut folds = Vec::new();
    let mut skipped_folds = Vec::new();
    let mut train_end = minimum_train_count;
    let mut fold_index = 1_usize;

    while train_end < ordered_samples.len() {
        let valid_end = (train_end + validation_window_size).min(ordered_samples.len());
        let train_fold = ordered_samples[..train_end].to_vec();
        let valid_fold = ordered_samples[train_end..valid_end].to_vec();
        if valid_fold.is_empty() {
            break;
        }
        let train_positive_count = train_fold
            .iter()
            .filter(|sample| sample.label >= 0.5)
            .count();
        let train_negative_count = train_fold.len().saturating_sub(train_positive_count);
        if train_positive_count == 0 || train_negative_count == 0 {
            skipped_folds.push(json!({
                "fold_index": fold_index,
                "reason": "train_fold_missing_class_balance",
                "train_sample_count": train_fold.len(),
                "validation_sample_count": valid_fold.len(),
            }));
            train_end = valid_end;
            fold_index += 1;
            continue;
        }

        let fold_feature_models = build_feature_models(&train_fold, feature_configs)?;
        let fold_train_matrix = encode_samples(&train_fold, &fold_feature_models)?;
        let fold_model = train_logistic_model(&fold_train_matrix);
        let validation_metrics =
            evaluate_sample_refs(&valid_fold, &fold_feature_models, &fold_model);
        folds.push(json!({
            "fold_index": fold_index,
            "train_sample_count": train_fold.len(),
            "validation_sample_count": valid_fold.len(),
            "train_end_date": train_fold
                .last()
                .map(|sample| sample.as_of_date.to_string())
                .unwrap_or_default(),
            "validation_start_date": valid_fold
                .first()
                .map(|sample| sample.as_of_date.to_string())
                .unwrap_or_default(),
            "validation_end_date": valid_fold
                .last()
                .map(|sample| sample.as_of_date.to_string())
                .unwrap_or_default(),
            "validation_accuracy": validation_metrics["accuracy"],
            "validation_positive_rate": validation_metrics["positive_rate"],
        }));
        train_end = valid_end;
        fold_index += 1;
    }

    let fold_accuracies = folds
        .iter()
        .filter_map(|fold| fold.get("validation_accuracy").and_then(Value::as_f64))
        .collect::<Vec<_>>();
    Ok(json!({
        "status": if folds.is_empty() { "insufficient_executed_folds" } else { "ok" },
        "sample_count": ordered_samples.len(),
        "minimum_train_count": minimum_train_count,
        "validation_window_size": validation_window_size,
        "fold_count": folds.len(),
        "executed_folds": folds,
        "skipped_folds": skipped_folds,
        "mean_validation_accuracy": mean_of(&fold_accuracies),
        "min_validation_accuracy": fold_accuracies.iter().copied().fold(1.0_f64, f64::min),
        "max_validation_accuracy": fold_accuracies.iter().copied().fold(0.0_f64, f64::max),
    }))
}

fn build_segment_slice_summary(
    samples: &[TrainingSample],
    feature_models: &[FeatureModel],
    trained_model: &TrainedLogisticModel,
) -> Result<Value, SecurityScorecardTrainingError> {
    let test_samples = samples_for_split(samples, "test");
    // 2026-04-21 CST: Updated because the approved follow-up cleanup also removes
    // integrated_stance, so diagnostics must slice on retained atomic dimensions only.
    // Purpose: keep segment summaries aligned with the live training contract after summary-label removal.
    let dimensions = ["market_regime", "trend_bias", "breakout_signal"]
        .iter()
        .map(|feature_name| {
            Ok(json!({
                "feature_name": feature_name,
                "slices": build_segment_slices_for_feature(
                    *feature_name,
                    &test_samples,
                    feature_models,
                    trained_model,
                )?,
            }))
        })
        .collect::<Result<Vec<_>, SecurityScorecardTrainingError>>()?;

    Ok(json!({
        "split_name": "test",
        "dimension_count": dimensions.len(),
        "dimensions": dimensions,
    }))
}

fn build_segment_slices_for_feature(
    feature_name: &str,
    samples: &[&TrainingSample],
    feature_models: &[FeatureModel],
    trained_model: &TrainedLogisticModel,
) -> Result<Vec<Value>, SecurityScorecardTrainingError> {
    let mut grouped = BTreeMap::<String, Vec<&TrainingSample>>::new();
    for sample in samples {
        let slice_value = sample
            .feature_values
            .get(feature_name)
            .map(feature_value_as_key)
            .unwrap_or_else(|| "__missing__".to_string());
        grouped.entry(slice_value).or_default().push(*sample);
    }

    grouped
        .into_iter()
        .map(|(slice_name, slice_samples)| {
            let predicted_probabilities = slice_samples
                .iter()
                .map(|sample| predict_probability(sample, feature_models, trained_model))
                .collect::<Result<Vec<_>, _>>()?;
            let metrics = evaluate_sample_refs(&slice_samples, feature_models, trained_model);
            Ok(json!({
                "slice_name": slice_name,
                "sample_count": slice_samples.len(),
                "accuracy": metrics["accuracy"],
                "positive_rate": metrics["positive_rate"],
                "average_predicted_probability": mean_of(&predicted_probabilities),
            }))
        })
        .collect()
}

fn build_readiness_assessment(
    samples: &[TrainingSample],
    feature_models: &[FeatureModel],
    correlation_summary: &Value,
    feature_influence_summary: &Value,
    walk_forward_summary: &Value,
) -> Value {
    let train_samples = samples_for_split(samples, "train");
    let valid_samples = samples_for_split(samples, "valid");
    let test_samples = samples_for_split(samples, "test");
    let high_correlation_pair_count =
        value_u64(correlation_summary, "high_correlation_pair_count") as usize;
    let counterintuitive_bin_count = feature_influence_summary
        .get("counterintuitive_bins")
        .and_then(Value::as_array)
        .map(|entries| entries.len())
        .unwrap_or(0);
    let walk_forward_fold_count = value_u64(walk_forward_summary, "fold_count") as usize;
    let mean_walk_forward_accuracy = walk_forward_summary
        .get("mean_validation_accuracy")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let sample_per_feature = samples.len() as f64 / feature_models.len().max(1) as f64;

    let train_positive_rate = train_samples
        .iter()
        .filter(|sample| sample.label >= 0.5)
        .count() as f64
        / train_samples.len().max(1) as f64;
    let valid_positive_rate = valid_samples
        .iter()
        .filter(|sample| sample.label >= 0.5)
        .count() as f64
        / valid_samples.len().max(1) as f64;
    let test_positive_rate = test_samples
        .iter()
        .filter(|sample| sample.label >= 0.5)
        .count() as f64
        / test_samples.len().max(1) as f64;

    let mut warnings = Vec::new();
    if sample_per_feature < 5.0 {
        warnings.push("sample_per_feature_is_below_minimum".to_string());
    }
    if high_correlation_pair_count > 0 {
        warnings.push("high_correlation_pairs_detected".to_string());
    }
    if counterintuitive_bin_count > 0 {
        warnings.push("counterintuitive_bins_detected".to_string());
    }
    if walk_forward_fold_count == 0 {
        warnings.push("walk_forward_folds_not_executed".to_string());
    }
    if walk_forward_fold_count > 0 && mean_walk_forward_accuracy < 0.55 {
        warnings.push("walk_forward_accuracy_is_weak".to_string());
    }
    if (train_positive_rate - valid_positive_rate).abs() > 0.25
        || (valid_positive_rate - test_positive_rate).abs() > 0.25
    {
        warnings.push("label_distribution_shift_is_large".to_string());
    }

    let production_readiness = if counterintuitive_bin_count > 0 || sample_per_feature < 3.0 {
        "blocked"
    } else if warnings.is_empty() {
        "candidate"
    } else {
        "caution"
    };

    json!({
        "production_readiness": production_readiness,
        "sample_per_feature": sample_per_feature,
        "high_correlation_pair_count": high_correlation_pair_count,
        "counterintuitive_bin_count": counterintuitive_bin_count,
        "walk_forward_fold_count": walk_forward_fold_count,
        "mean_walk_forward_accuracy": mean_walk_forward_accuracy,
        "warnings": warnings,
    })
}

fn compute_information_value(
    bin: &FeatureBinModel,
    total_positive: f64,
    total_negative: f64,
) -> f64 {
    let smooth = 0.5;
    let positive_rate = (bin.positive_count + smooth) / (total_positive + smooth * 2.0);
    let negative_rate = (bin.negative_count + smooth) / (total_negative + smooth * 2.0);
    (positive_rate - negative_rate) * bin.woe
}

fn feature_kind_name(kind: &TrainingFeatureKind) -> &'static str {
    match kind {
        TrainingFeatureKind::Numeric => "numeric",
        TrainingFeatureKind::Categorical => "categorical",
    }
}

fn feature_value_as_key(value: &TrainingFeatureValue) -> String {
    match value {
        TrainingFeatureValue::Numeric(number) => format!("{number:.6}"),
        TrainingFeatureValue::Category(category) => category.clone(),
    }
}

fn evaluate_sample_refs(
    split_samples: &[&TrainingSample],
    feature_models: &[FeatureModel],
    trained_model: &TrainedLogisticModel,
) -> Value {
    if split_samples.is_empty() {
        return json!({
            "sample_count": 0,
            "accuracy": Value::Null,
            "positive_rate": Value::Null,
        });
    }

    let mut correct_count = 0_usize;
    let mut positive_count = 0_usize;
    for sample in split_samples {
        let probability = predict_probability(sample, feature_models, trained_model).unwrap_or(0.5);
        let predicted = if probability >= 0.5 { 1.0 } else { 0.0 };
        if (predicted - sample.label).abs() <= 1e-9 {
            correct_count += 1;
        }
        if sample.label >= 0.5 {
            positive_count += 1;
        }
    }

    json!({
        "sample_count": split_samples.len(),
        "accuracy": correct_count as f64 / split_samples.len() as f64,
        "positive_rate": positive_count as f64 / split_samples.len() as f64,
    })
}

fn mean_of(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn compute_variance(values: &[f64]) -> f64 {
    if values.len() <= 1 {
        return 0.0;
    }
    let mean = mean_of(values);
    values
        .iter()
        .map(|value| {
            let diff = *value - mean;
            diff * diff
        })
        .sum::<f64>()
        / values.len() as f64
}

fn compute_pearson_correlation(left: &[f64], right: &[f64]) -> f64 {
    if left.len() != right.len() || left.len() <= 1 {
        return 0.0;
    }
    let left_mean = mean_of(left);
    let right_mean = mean_of(right);
    let numerator = left
        .iter()
        .zip(right.iter())
        .map(|(left_value, right_value)| (*left_value - left_mean) * (*right_value - right_mean))
        .sum::<f64>();
    let left_variance = compute_variance(left);
    let right_variance = compute_variance(right);
    if left_variance <= 1e-12 || right_variance <= 1e-12 {
        return 0.0;
    }
    numerator / ((left.len() as f64) * left_variance.sqrt() * right_variance.sqrt())
}

fn value_f64(value: &Value, field_name: &str) -> f64 {
    value.get(field_name).and_then(Value::as_f64).unwrap_or(0.0)
}

fn value_u64(value: &Value, field_name: &str) -> u64 {
    value.get(field_name).and_then(Value::as_u64).unwrap_or(0)
}

fn build_artifact(
    request: &SecurityScorecardTrainingRequest,
    feature_models: &[FeatureModel],
    trained_model: &TrainedLogisticModel,
) -> SecurityScorecardModelArtifact {
    let model_id = build_governed_model_id(request);
    let model_version = format!("candidate_{}", sanitize_identifier(&request.created_at));

    let features = feature_models
        .iter()
        .enumerate()
        .map(|(index, feature_model)| {
            let coefficient = trained_model
                .coefficients
                .get(index)
                .copied()
                .unwrap_or(0.0);
            SecurityScorecardModelFeatureSpec {
                feature_name: feature_model.feature_name.clone(),
                group_name: feature_model.group_name.clone(),
                bins: feature_model
                    .bins
                    .iter()
                    .map(|bin| SecurityScorecardModelBin {
                        bin_label: bin.bin_label.clone(),
                        match_values: bin.match_values.clone(),
                        min_inclusive: bin.min_inclusive,
                        max_exclusive: bin.max_exclusive,
                        woe: Some(bin.woe),
                        logit_contribution: Some(coefficient * bin.woe),
                        points: coefficient * bin.woe * 100.0,
                        // 2026-04-14 CST: 这里给分箱补 predicted_value 默认位，原因是 scorecard
                        // 模型合同已经允许额外预测值挂接，但当前训练首版还没输出该维度。
                        // 目的：先维持 artifact 合同稳定，不伪造不存在的预测值。
                        predicted_value: None,
                    })
                    .collect(),
            }
        })
        .collect();

    SecurityScorecardModelArtifact {
        model_id,
        model_version,
        label_definition: request.label_definition_version.clone(),
        // 2026-04-14 CST: 这里补齐模型 artifact 新增的预测/子范围字段，原因是训练器仍在按旧合同
        // 构造落盘对象，已经无法被当前 scorecard 正式消费者反序列化。
        // 目的：先恢复 artifact 合同兼容，后续再把真实 baseline/mode 训练逻辑补完整。
        target_head: Some(request.target_head.clone()),
        target_label_definition: Some(resolve_target_label_definition(
            &request.target_head,
            request.horizon_days,
        )),
        prediction_mode: Some("direction_probability".to_string()),
        prediction_baseline: None,
        training_window: Some(request.train_range.clone()),
        oot_window: Some(request.test_range.clone()),
        positive_label_definition: resolve_positive_label_definition(
            &request.target_head,
            request.horizon_days,
        ),
        // 2026-04-20 CST: Added because Task 1 must persist the explicit non-equity
        // training identity on the canonical artifact, not only in transient request state.
        // Purpose: make downstream runtime and registry consumers read the same governed scope.
        instrument_subscope: request.instrument_subscope.clone(),
        binning_version: Some("woe_binning.v1".to_string()),
        coefficient_version: Some("woe_logistic.v1".to_string()),
        model_sha256: None,
        intercept: Some(trained_model.intercept),
        base_score: 600.0,
        features,
    }
}

fn build_governed_model_id(request: &SecurityScorecardTrainingRequest) -> String {
    // 2026-04-20 CST: Added because Task 2 freezes Nikkei as a governed non-equity
    // training family and the old identity rule collapsed all index candidates together.
    // Purpose: keep artifact, diagnostics, and registry selection keyed by the approved instrument subscope.
    let horizon_token = if uses_nikkei_weekly_training_contract(request) {
        "1w".to_string()
    } else {
        format!("{}d", request.horizon_days)
    };
    match request.instrument_subscope.as_deref().map(str::trim) {
        Some(subscope) if !subscope.is_empty() => format!(
            "{}_{}_{}_{}_{}",
            request.market_scope.to_lowercase(),
            request.instrument_scope.to_lowercase(),
            sanitize_identifier(subscope),
            horizon_token,
            request.target_head
        ),
        _ => format!(
            "{}_{}_{}_{}",
            request.market_scope.to_lowercase(),
            request.instrument_scope.to_lowercase(),
            horizon_token,
            request.target_head
        ),
    }
}

fn build_metrics_summary(
    samples: &[TrainingSample],
    feature_models: &[FeatureModel],
    trained_model: &TrainedLogisticModel,
    diagnostics_summary: Value,
    validation_start: NaiveDate,
) -> Value {
    let train_metrics = evaluate_split(samples, "train", feature_models, trained_model);
    let valid_metrics = evaluate_split(samples, "valid", feature_models, trained_model);
    let test_metrics = evaluate_split(samples, "test", feature_models, trained_model);
    let post_validation_holdout =
        evaluate_samples_after_cutoff(samples, validation_start, feature_models, trained_model);

    json!({
        "train": train_metrics,
        "valid": valid_metrics,
        "test": test_metrics,
        "post_validation_holdout": post_validation_holdout,
        "feature_count": feature_models.len(),
        "sample_count": samples.len(),
        "diagnostics": diagnostics_summary,
    })
}

fn build_weekly_metrics_summary(
    artifact_samples: &[TrainingSample],
    rolling_windows: &[WeeklyRollingWindowSamples],
    capital_source_observation_rows: &[CapitalSourceObservationRow],
    feature_configs: &[TrainingFeatureConfig],
    artifact_feature_models: &[FeatureModel],
    artifact_trained_model: &TrainedLogisticModel,
    diagnostics_summary: Value,
    validation_start: NaiveDate,
) -> Result<Value, SecurityScorecardTrainingError> {
    let train_metrics = evaluate_split(
        artifact_samples,
        "train",
        artifact_feature_models,
        artifact_trained_model,
    );
    let mut valid_records = Vec::new();
    let mut test_records = Vec::new();
    let mut skipped_windows = Vec::new();

    for window in rolling_windows {
        let train_refs = window.train_samples.iter().collect::<Vec<_>>();
        let positive_count = train_refs
            .iter()
            .filter(|sample| sample.label >= 0.5)
            .count();
        let negative_count = train_refs.len().saturating_sub(positive_count);
        if train_refs.len() < 2 || positive_count == 0 || negative_count == 0 {
            skipped_windows.push(json!({
                "window_id": window.window_id,
                "reason": "train_window_missing_class_balance",
                "train_sample_count": train_refs.len(),
                "valid_sample_count": window.valid_samples.len(),
                "test_sample_count": window.test_samples.len(),
            }));
            continue;
        }
        let fold_feature_models = build_feature_models(&train_refs, feature_configs)?;
        let fold_train_matrix = encode_samples(&train_refs, &fold_feature_models)?;
        let fold_model = train_logistic_model(&fold_train_matrix);
        valid_records.extend(collect_window_evaluation_records(
            window.window_id,
            "valid",
            &window.valid_samples,
            &fold_feature_models,
            &fold_model,
        )?);
        test_records.extend(collect_window_evaluation_records(
            window.window_id,
            "test",
            &window.test_samples,
            &fold_feature_models,
            &fold_model,
        )?);
    }

    let post_validation_holdout_records = test_records
        .iter()
        .filter(|record| record.as_of_date >= validation_start)
        .cloned()
        .collect::<Vec<_>>();

    Ok(json!({
        "train": train_metrics,
        "valid": summarize_window_evaluation_records(&valid_records),
        "test": summarize_window_evaluation_records(&test_records),
        "post_validation_holdout": summarize_window_holdout_records(
            &post_validation_holdout_records,
            validation_start,
        ),
        "feature_count": artifact_feature_models.len(),
        "sample_count": artifact_samples.len(),
        "rolling_window_count": valid_records
            .iter()
            .map(|record| record.window_id)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        "capital_source_observation": build_capital_source_observation_summary(
            capital_source_observation_rows,
        ),
        "skipped_window_count": skipped_windows.len(),
        "skipped_windows": skipped_windows,
        "diagnostics": diagnostics_summary,
    }))
}

fn build_capital_source_observation_summary(
    observation_rows: &[CapitalSourceObservationRow],
) -> Value {
    if observation_rows.is_empty() {
        return Value::Null;
    }

    let mut latest_row = &observation_rows[0];
    let mut factor_series = BTreeMap::<String, Vec<f64>>::new();
    for row in observation_rows {
        if row.as_of_date > latest_row.as_of_date {
            latest_row = row;
        }
        for (factor_name, value) in &row.factor_values {
            factor_series
                .entry(factor_name.clone())
                .or_default()
                .push(*value);
        }
    }

    let factor_stats = factor_series
        .iter()
        .map(|(factor_name, values)| {
            let mut sorted_values = values.clone();
            sorted_values.sort_by(|left, right| left.total_cmp(right));
            let mean = if sorted_values.is_empty() {
                0.0
            } else {
                sorted_values.iter().sum::<f64>() / sorted_values.len() as f64
            };
            (
                factor_name.clone(),
                json!({
                    "sample_count": sorted_values.len(),
                    "min": sorted_values.first().copied().unwrap_or(0.0),
                    "p50": percentile(&sorted_values, 0.5),
                    "max": sorted_values.last().copied().unwrap_or(0.0),
                    "mean": round_metric(mean),
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();

    json!({
        "mode": "observation_only",
        "sample_count": observation_rows.len(),
        "factor_count": factor_series.len(),
        "latest_as_of_date": latest_row.as_of_date.format("%Y-%m-%d").to_string(),
        "latest_values": latest_row.factor_values,
        "latest_observation_dates": latest_row.observation_dates,
        "factor_stats": factor_stats,
    })
}

fn percentile(sorted_values: &[f64], quantile: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    let clamped_quantile = quantile.clamp(0.0, 1.0);
    let index = ((sorted_values.len() - 1) as f64 * clamped_quantile).round() as usize;
    round_metric(sorted_values[index])
}

fn round_metric(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

fn evaluate_samples_after_cutoff(
    samples: &[TrainingSample],
    cutoff_date: NaiveDate,
    feature_models: &[FeatureModel],
    trained_model: &TrainedLogisticModel,
) -> Value {
    // 2026-04-20 CST: Added because the approved decade Nikkei route must report
    // how the model performs after the training cutoff instead of only split-local metrics.
    // Purpose: expose one direct holdout view for all samples from the validation start onward.
    let holdout_samples = samples
        .iter()
        .filter(|sample| sample.as_of_date >= cutoff_date)
        .collect::<Vec<_>>();
    let metrics = evaluate_sample_refs(&holdout_samples, feature_models, trained_model);
    json!({
        "cutoff_date": cutoff_date.to_string(),
        "sample_count": metrics["sample_count"],
        "accuracy": metrics["accuracy"],
        "positive_rate": metrics["positive_rate"],
        "start_date": holdout_samples
            .first()
            .map(|sample| sample.as_of_date.to_string())
            .unwrap_or_default(),
        "end_date": holdout_samples
            .last()
            .map(|sample| sample.as_of_date.to_string())
            .unwrap_or_default(),
    })
}

fn evaluate_split(
    samples: &[TrainingSample],
    split_name: &str,
    feature_models: &[FeatureModel],
    trained_model: &TrainedLogisticModel,
) -> Value {
    let split_samples = samples_for_split(samples, split_name);
    evaluate_sample_refs(&split_samples, feature_models, trained_model)
}

fn collect_window_evaluation_records(
    window_id: usize,
    split_name: &str,
    samples: &[TrainingSample],
    feature_models: &[FeatureModel],
    trained_model: &TrainedLogisticModel,
) -> Result<Vec<WindowEvaluationRecord>, SecurityScorecardTrainingError> {
    samples
        .iter()
        .map(|sample| {
            Ok(WindowEvaluationRecord {
                window_id,
                split_name: split_name.to_string(),
                as_of_date: sample.as_of_date,
                label: sample.label,
                predicted_probability: predict_probability(sample, feature_models, trained_model)?,
            })
        })
        .collect()
}

fn summarize_window_evaluation_records(records: &[WindowEvaluationRecord]) -> Value {
    let sample_count = records.len() as f64;
    let accuracy = if sample_count <= 0.0 {
        0.0
    } else {
        records
            .iter()
            .filter(|record| {
                let predicted_label = if record.predicted_probability >= 0.5 {
                    1.0
                } else {
                    0.0
                };
                (predicted_label - record.label).abs() <= f64::EPSILON
            })
            .count() as f64
            / sample_count
    };
    let positive_rate = if sample_count <= 0.0 {
        0.0
    } else {
        records.iter().filter(|record| record.label >= 0.5).count() as f64 / sample_count
    };
    json!({
        "split_name": records
            .first()
            .map(|record| record.split_name.clone())
            .unwrap_or_default(),
        "sample_count": records.len(),
        "accuracy": accuracy,
        "positive_rate": positive_rate,
        "start_date": records
            .first()
            .map(|record| record.as_of_date.to_string())
            .unwrap_or_default(),
        "end_date": records
            .last()
            .map(|record| record.as_of_date.to_string())
            .unwrap_or_default(),
    })
}

fn summarize_window_holdout_records(
    records: &[WindowEvaluationRecord],
    cutoff_date: NaiveDate,
) -> Value {
    let metrics = summarize_window_evaluation_records(records);
    json!({
        "cutoff_date": cutoff_date.to_string(),
        "sample_count": metrics["sample_count"],
        "accuracy": metrics["accuracy"],
        "positive_rate": metrics["positive_rate"],
        "start_date": metrics["start_date"],
        "end_date": metrics["end_date"],
    })
}

fn predict_probability(
    sample: &TrainingSample,
    feature_models: &[FeatureModel],
    trained_model: &TrainedLogisticModel,
) -> Result<f64, SecurityScorecardTrainingError> {
    let mut logit = trained_model.intercept;
    for (index, feature_model) in feature_models.iter().enumerate() {
        let coefficient = trained_model
            .coefficients
            .get(index)
            .copied()
            .unwrap_or(0.0);
        logit += coefficient * resolve_feature_woe(feature_model, sample)?;
    }
    Ok(logistic(logit))
}

fn resolve_runtime_root(request: &SecurityScorecardTrainingRequest) -> PathBuf {
    // 2026-04-22 CST: Updated because scheme 2 separates artifact persistence
    // from source-data loading after the Nikkei capital-source miswiring.
    // Reason: the old training_runtime_root remains as a legacy alias, but artifact output now has its own explicit boundary.
    // Purpose: keep backward compatibility while preferring the explicit artifact root.
    normalized_artifact_runtime_root(request)
        .or_else(|| normalized_legacy_training_runtime_root(request))
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(".worktrees")
                .join("SheetMind-Scenes-inspect")
                .join(".sheetmind_scenes_runtime")
        })
}

pub fn debug_build_weekly_price_feature_rows(
    spot_rows: &[StockHistoryRow],
    futures_rows: Option<&[StockHistoryRow]>,
) -> Result<Vec<WeeklyPriceFeatureRow>, SecurityScorecardTrainingError> {
    build_weekly_price_feature_rows(spot_rows, futures_rows, None)
}

pub fn debug_load_governed_weekly_observation_dates(
    capital_flow_runtime_root: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<String>, SecurityScorecardTrainingError> {
    load_governed_weekly_observation_dates(capital_flow_runtime_root, start_date, end_date)
}

pub fn debug_build_weekly_anchor_dates(
    spot_rows: &[StockHistoryRow],
    governed_weekly_dates: &[String],
    start_date: &str,
    end_date: &str,
) -> Result<Vec<String>, SecurityScorecardTrainingError> {
    build_weekly_anchor_dates(spot_rows, governed_weekly_dates, start_date, end_date)
}

pub fn debug_build_weekly_rolling_split_plan(
    anchors: &[String],
    train_weeks: usize,
    valid_weeks: usize,
    test_weeks: usize,
    stride_weeks: usize,
) -> Result<Vec<WeeklyRollingWindowPlan>, SecurityScorecardTrainingError> {
    build_weekly_rolling_split_plan(anchors, train_weeks, valid_weeks, test_weeks, stride_weeks)
}

fn build_weekly_price_feature_rows(
    spot_rows: &[StockHistoryRow],
    futures_rows: Option<&[StockHistoryRow]>,
    volume_proxy_rows: Option<&[StockHistoryRow]>,
) -> Result<Vec<WeeklyPriceFeatureRow>, SecurityScorecardTrainingError> {
    let weekly_spot_rows = build_weekly_price_buckets(spot_rows)?;
    let weekly_futures_rows = futures_rows
        .map(build_weekly_price_buckets)
        .transpose()?
        .unwrap_or_default();
    let futures_by_week = weekly_futures_rows
        .into_iter()
        .map(|bucket| (bucket.week_start, bucket))
        .collect::<BTreeMap<_, _>>();
    let weekly_volume_proxy_rows = volume_proxy_rows
        .map(build_weekly_price_buckets)
        .transpose()?
        .unwrap_or_default();
    let volume_proxy_by_week = weekly_volume_proxy_rows
        .into_iter()
        .map(|bucket| (bucket.week_start, bucket))
        .collect::<BTreeMap<_, _>>();

    let mut feature_rows = Vec::new();
    for (index, spot_bucket) in weekly_spot_rows.iter().enumerate() {
        let mut feature_values = BTreeMap::new();
        append_distribution_features(
            &mut feature_values,
            "weekly_spot_return",
            &spot_bucket.daily_returns,
        );
        feature_values.insert(
            "weekly_spot_close_position".to_string(),
            spot_bucket.close_position,
        );
        feature_values.insert("weekly_spot_drawdown".to_string(), spot_bucket.drawdown);
        feature_values.insert("weekly_spot_rebound".to_string(), spot_bucket.rebound);
        let volume_source_bucket = volume_proxy_by_week
            .get(&spot_bucket.week_start)
            .filter(|bucket| bucket.total_volume > f64::EPSILON)
            .or_else(|| {
                futures_by_week
                    .get(&spot_bucket.week_start)
                    .filter(|bucket| bucket.total_volume > f64::EPSILON)
            })
            .unwrap_or(spot_bucket);
        let prior_volume_buckets_4w = prior_volume_buckets_for_window(
            &weekly_spot_rows,
            &futures_by_week,
            &volume_proxy_by_week,
            index,
            4,
        );
        let prior_volume_mean = mean_weekly_volume(&prior_volume_buckets_4w);
        feature_values.insert(
            "weekly_volume_ratio_4w".to_string(),
            prior_volume_mean
                .map(|mean| {
                    if mean <= f64::EPSILON {
                        1.0
                    } else {
                        volume_source_bucket.total_volume / mean
                    }
                })
                .unwrap_or(1.0),
        );
        // 2026-04-26 CST: Added because current-week volume must be judged
        // against quarterly, half-year, and yearly baselines for index-scale
        // accumulation behavior. Purpose: avoid treating slow positioning as
        // short-term volume noise.
        let weekly_volume_ratio_13w = weekly_volume_ratio_for_window(
            volume_source_bucket,
            &weekly_spot_rows,
            &futures_by_week,
            &volume_proxy_by_week,
            index,
            13,
        );
        let weekly_volume_ratio_26w = weekly_volume_ratio_for_window(
            volume_source_bucket,
            &weekly_spot_rows,
            &futures_by_week,
            &volume_proxy_by_week,
            index,
            26,
        );
        let weekly_volume_ratio_52w = weekly_volume_ratio_for_window(
            volume_source_bucket,
            &weekly_spot_rows,
            &futures_by_week,
            &volume_proxy_by_week,
            index,
            52,
        );
        let weekly_price_position_52w =
            price_position_in_prior_weeks(spot_bucket, &weekly_spot_rows, index, 52);
        let weekly_volume_accumulation_26w =
            (weekly_volume_ratio_26w - 1.0).max(0.0) * (1.0 - weekly_price_position_52w);
        let weekly_volume_accumulation_52w =
            (weekly_volume_ratio_52w - 1.0).max(0.0) * (1.0 - weekly_price_position_52w);
        feature_values.insert(
            "weekly_volume_ratio_13w".to_string(),
            weekly_volume_ratio_13w,
        );
        feature_values.insert(
            "weekly_volume_ratio_26w".to_string(),
            weekly_volume_ratio_26w,
        );
        feature_values.insert(
            "weekly_volume_ratio_52w".to_string(),
            weekly_volume_ratio_52w,
        );
        feature_values.insert(
            "weekly_price_position_52w".to_string(),
            weekly_price_position_52w,
        );
        feature_values.insert(
            "weekly_volume_accumulation_26w".to_string(),
            weekly_volume_accumulation_26w,
        );
        feature_values.insert(
            "weekly_volume_accumulation_52w".to_string(),
            weekly_volume_accumulation_52w,
        );
        feature_values.insert(
            "weekly_high_volume_low_price_signal".to_string(),
            if weekly_volume_ratio_52w >= 1.10 && weekly_price_position_52w <= 0.40 {
                1.0
            } else {
                0.0
            },
        );
        feature_values.insert(
            "weekly_high_volume_breakout_signal".to_string(),
            if weekly_volume_ratio_52w >= 1.10 && weekly_price_position_52w >= 0.80 {
                1.0
            } else {
                0.0
            },
        );
        feature_values.insert(
            "weekly_up_day_volume_share".to_string(),
            volume_source_bucket.up_day_volume_share,
        );
        feature_values.insert(
            "weekly_down_day_volume_share".to_string(),
            volume_source_bucket.down_day_volume_share,
        );
        let weekly_return = match (spot_bucket.rows.first(), spot_bucket.rows.last()) {
            (Some(first), Some(last)) if first.close.abs() > f64::EPSILON => {
                (last.close / first.close) - 1.0
            }
            _ => 0.0,
        };
        let volume_ratio = feature_values
            .get("weekly_volume_ratio_4w")
            .copied()
            .unwrap_or(1.0);
        feature_values.insert(
            "weekly_volume_price_confirmation".to_string(),
            if weekly_return > 0.0 && volume_ratio >= 1.05 {
                1.0
            } else if weekly_return < 0.0 && volume_ratio >= 1.05 {
                -1.0
            } else {
                0.0
            },
        );

        if let Some(futures_bucket) = futures_by_week.get(&spot_bucket.week_start) {
            append_distribution_features(
                &mut feature_values,
                "weekly_futures_return",
                &futures_bucket.daily_returns,
            );
            append_distribution_features(
                &mut feature_values,
                "weekly_basis_pct",
                &aligned_basis_series(&spot_bucket.rows, &futures_bucket.rows),
            );
            append_distribution_features(
                &mut feature_values,
                "weekly_futures_relative_strength",
                &aligned_relative_strength_series(&spot_bucket.rows, &futures_bucket.rows),
            );
        }

        feature_rows.push(WeeklyPriceFeatureRow {
            week_start_date: spot_bucket.week_start.format("%Y-%m-%d").to_string(),
            week_end_date: spot_bucket.week_end.format("%Y-%m-%d").to_string(),
            feature_values,
        });
    }

    Ok(feature_rows)
}

fn weekly_volume_ratio_for_window(
    current_bucket: &WeeklyPriceBucket,
    weekly_spot_rows: &[WeeklyPriceBucket],
    futures_by_week: &BTreeMap<NaiveDate, WeeklyPriceBucket>,
    volume_proxy_by_week: &BTreeMap<NaiveDate, WeeklyPriceBucket>,
    index: usize,
    prior_week_count: usize,
) -> f64 {
    let prior_volume_buckets = prior_volume_buckets_for_window(
        weekly_spot_rows,
        futures_by_week,
        volume_proxy_by_week,
        index,
        prior_week_count,
    );
    let Some(prior_mean) = mean_weekly_volume(&prior_volume_buckets) else {
        return 1.0;
    };
    if prior_mean <= f64::EPSILON {
        1.0
    } else {
        current_bucket.total_volume / prior_mean
    }
}

fn prior_volume_buckets_for_window<'a>(
    weekly_spot_rows: &'a [WeeklyPriceBucket],
    futures_by_week: &'a BTreeMap<NaiveDate, WeeklyPriceBucket>,
    volume_proxy_by_week: &'a BTreeMap<NaiveDate, WeeklyPriceBucket>,
    index: usize,
    prior_week_count: usize,
) -> Vec<&'a WeeklyPriceBucket> {
    if index == 0 {
        return Vec::new();
    }
    let start_index = index.saturating_sub(prior_week_count);
    weekly_spot_rows[start_index..index]
        .iter()
        .map(|bucket| weekly_volume_source_bucket(bucket, futures_by_week, volume_proxy_by_week))
        .collect()
}

fn weekly_volume_source_bucket<'a>(
    spot_bucket: &'a WeeklyPriceBucket,
    futures_by_week: &'a BTreeMap<NaiveDate, WeeklyPriceBucket>,
    volume_proxy_by_week: &'a BTreeMap<NaiveDate, WeeklyPriceBucket>,
) -> &'a WeeklyPriceBucket {
    volume_proxy_by_week
        .get(&spot_bucket.week_start)
        .filter(|bucket| bucket.total_volume > f64::EPSILON)
        .or_else(|| {
            futures_by_week
                .get(&spot_bucket.week_start)
                .filter(|bucket| bucket.total_volume > f64::EPSILON)
        })
        .unwrap_or(spot_bucket)
}

fn mean_weekly_volume(prior_volume_buckets: &[&WeeklyPriceBucket]) -> Option<f64> {
    if prior_volume_buckets.is_empty() {
        return None;
    }
    Some(
        prior_volume_buckets
            .iter()
            .map(|bucket| bucket.total_volume)
            .sum::<f64>()
            / prior_volume_buckets.len() as f64,
    )
}

fn price_position_in_prior_weeks(
    current_bucket: &WeeklyPriceBucket,
    weekly_spot_rows: &[WeeklyPriceBucket],
    index: usize,
    prior_week_count: usize,
) -> f64 {
    if index == 0 {
        return 0.5;
    }
    let start_index = index.saturating_sub(prior_week_count);
    let prior_buckets = &weekly_spot_rows[start_index..index];
    if prior_buckets.is_empty() || prior_buckets.len() < prior_week_count {
        return 0.5;
    }
    let prior_closes = prior_buckets
        .iter()
        .filter_map(weekly_bucket_last_close)
        .collect::<Vec<_>>();
    if prior_closes.is_empty() {
        return 0.5;
    }
    let prior_low = prior_closes.iter().copied().fold(f64::INFINITY, f64::min);
    let prior_high = prior_closes
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let range = prior_high - prior_low;
    if range.abs() <= f64::EPSILON {
        return 0.5;
    }
    let current_close = weekly_bucket_last_close(current_bucket).unwrap_or(prior_low);
    ((current_close - prior_low) / range).clamp(0.0, 1.0)
}

fn weekly_bucket_last_close(bucket: &WeeklyPriceBucket) -> Option<f64> {
    bucket.rows.last().map(|row| row.close)
}

fn load_governed_weekly_observation_dates(
    capital_flow_runtime_root: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<String>, SecurityScorecardTrainingError> {
    let store = SecurityCapitalFlowStore::new(
        PathBuf::from(capital_flow_runtime_root).join("security_capital_flow.db"),
    );
    let mut dates = store
        .load_metric_dates_in_range("jpx_weekly_investor_type", "weekly", start_date, end_date)
        .map_err(|error| SecurityScorecardTrainingError::Build(error.to_string()))?;
    if dates.is_empty() {
        dates = store
            .load_metric_dates_in_range("mof_weekly_cross_border", "weekly", start_date, end_date)
            .map_err(|error| SecurityScorecardTrainingError::Build(error.to_string()))?;
    }
    Ok(dates)
}

fn build_weekly_anchor_dates(
    spot_rows: &[StockHistoryRow],
    governed_weekly_dates: &[String],
    start_date: &str,
    end_date: &str,
) -> Result<Vec<String>, SecurityScorecardTrainingError> {
    let start = NaiveDate::parse_from_str(start_date, "%Y-%m-%d").map_err(|error| {
        SecurityScorecardTrainingError::Build(format!(
            "invalid weekly anchor start_date `{start_date}`: {error}"
        ))
    })?;
    let end = NaiveDate::parse_from_str(end_date, "%Y-%m-%d").map_err(|error| {
        SecurityScorecardTrainingError::Build(format!(
            "invalid weekly anchor end_date `{end_date}`: {error}"
        ))
    })?;
    let price_weeks = build_weekly_price_buckets(spot_rows)?
        .into_iter()
        .map(|bucket| bucket.week_end.iso_week())
        .collect::<std::collections::BTreeSet<_>>();

    let mut anchors = Vec::new();
    for metric_date in governed_weekly_dates {
        let parsed_date = NaiveDate::parse_from_str(metric_date, "%Y-%m-%d").map_err(|error| {
            SecurityScorecardTrainingError::Build(format!(
                "invalid governed weekly date `{metric_date}`: {error}"
            ))
        })?;
        if parsed_date < start || parsed_date > end {
            continue;
        }
        if price_weeks.contains(&parsed_date.iso_week()) {
            anchors.push(metric_date.clone());
        }
    }
    Ok(anchors)
}

fn build_weekly_rolling_split_plan(
    anchors: &[String],
    train_weeks: usize,
    valid_weeks: usize,
    test_weeks: usize,
    stride_weeks: usize,
) -> Result<Vec<WeeklyRollingWindowPlan>, SecurityScorecardTrainingError> {
    if train_weeks == 0 || valid_weeks == 0 || test_weeks == 0 || stride_weeks == 0 {
        return Err(SecurityScorecardTrainingError::Build(
            "weekly rolling plan requires all window sizes to be greater than 0".to_string(),
        ));
    }
    let total_window = train_weeks + valid_weeks + test_weeks;
    if anchors.len() < total_window {
        return Ok(Vec::new());
    }

    let mut windows = Vec::new();
    let mut start_index = 0;
    while start_index + total_window <= anchors.len() {
        let train_end = start_index + train_weeks;
        let valid_end = train_end + valid_weeks;
        let test_end = valid_end + test_weeks;
        windows.push(WeeklyRollingWindowPlan {
            train_anchor_dates: anchors[start_index..train_end].to_vec(),
            valid_anchor_dates: anchors[train_end..valid_end].to_vec(),
            test_anchor_dates: anchors[valid_end..test_end].to_vec(),
        });
        start_index += stride_weeks;
    }
    Ok(windows)
}

#[derive(Debug, Clone)]
struct WeeklyPriceBucket {
    week_start: NaiveDate,
    week_end: NaiveDate,
    rows: Vec<StockHistoryRow>,
    daily_returns: Vec<f64>,
    total_volume: f64,
    up_day_volume_share: f64,
    down_day_volume_share: f64,
    close_position: f64,
    drawdown: f64,
    rebound: f64,
}

fn build_weekly_price_buckets(
    rows: &[StockHistoryRow],
) -> Result<Vec<WeeklyPriceBucket>, SecurityScorecardTrainingError> {
    let mut parsed_rows = rows
        .iter()
        .cloned()
        .map(|row| {
            let trade_date =
                NaiveDate::parse_from_str(&row.trade_date, "%Y-%m-%d").map_err(|error| {
                    SecurityScorecardTrainingError::Build(format!(
                        "invalid weekly aggregation trade_date `{}`: {error}",
                        row.trade_date
                    ))
                })?;
            Ok((trade_date, row))
        })
        .collect::<Result<Vec<_>, SecurityScorecardTrainingError>>()?;
    parsed_rows.sort_by_key(|(trade_date, _)| *trade_date);

    let mut grouped_rows = BTreeMap::<NaiveDate, Vec<StockHistoryRow>>::new();
    for (trade_date, row) in parsed_rows {
        grouped_rows
            .entry(week_start_date(trade_date))
            .or_default()
            .push(row);
    }

    let mut buckets = Vec::new();
    for (week_start, week_rows) in grouped_rows {
        if week_rows.len() < 2 {
            continue;
        }
        let week_end = week_rows
            .last()
            .and_then(|row| NaiveDate::parse_from_str(&row.trade_date, "%Y-%m-%d").ok())
            .unwrap_or(week_start);
        let close_values = week_rows.iter().map(|row| row.close).collect::<Vec<_>>();
        let total_volume = week_rows
            .iter()
            .map(|row| row.volume.max(0) as f64)
            .sum::<f64>();
        let high_max = week_rows
            .iter()
            .map(|row| row.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let low_min = week_rows
            .iter()
            .map(|row| row.low)
            .fold(f64::INFINITY, f64::min);
        let last_close = *close_values.last().unwrap_or(&0.0);
        let range = (high_max - low_min).abs();
        let close_position = if range <= f64::EPSILON {
            0.5
        } else {
            ((last_close - low_min) / range).clamp(0.0, 1.0)
        };
        let peak_close = close_values
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let trough_close = close_values.iter().copied().fold(f64::INFINITY, f64::min);
        let drawdown = if peak_close <= f64::EPSILON {
            0.0
        } else {
            ((trough_close / peak_close) - 1.0).abs()
        };
        let rebound_range = (peak_close - trough_close).abs();
        let rebound = if rebound_range <= f64::EPSILON {
            0.0
        } else {
            ((last_close - trough_close) / rebound_range).clamp(0.0, 1.0)
        };
        let mut up_day_volume = 0.0;
        let mut down_day_volume = 0.0;
        for window in week_rows.windows(2) {
            let previous = &window[0];
            let current = &window[1];
            if current.close > previous.close {
                up_day_volume += current.volume.max(0) as f64;
            } else if current.close < previous.close {
                down_day_volume += current.volume.max(0) as f64;
            }
        }
        buckets.push(WeeklyPriceBucket {
            week_start,
            week_end,
            rows: week_rows,
            daily_returns: pairwise_returns(&close_values),
            total_volume,
            up_day_volume_share: if total_volume <= f64::EPSILON {
                0.0
            } else {
                up_day_volume / total_volume
            },
            down_day_volume_share: if total_volume <= f64::EPSILON {
                0.0
            } else {
                down_day_volume / total_volume
            },
            close_position,
            drawdown,
            rebound,
        });
    }

    Ok(buckets)
}

fn week_start_date(trade_date: NaiveDate) -> NaiveDate {
    trade_date - Duration::days(trade_date.weekday().num_days_from_monday() as i64)
}

fn pairwise_returns(values: &[f64]) -> Vec<f64> {
    values
        .windows(2)
        .filter_map(|window| {
            let previous = window[0];
            let current = window[1];
            if previous.abs() <= f64::EPSILON {
                None
            } else {
                Some((current / previous) - 1.0)
            }
        })
        .collect()
}

fn aligned_basis_series(
    spot_rows: &[StockHistoryRow],
    futures_rows: &[StockHistoryRow],
) -> Vec<f64> {
    let spot_by_date = spot_rows
        .iter()
        .map(|row| (row.trade_date.as_str(), row.close))
        .collect::<BTreeMap<_, _>>();
    futures_rows
        .iter()
        .filter_map(|row| {
            let spot_close = spot_by_date.get(row.trade_date.as_str())?;
            if spot_close.abs() <= f64::EPSILON {
                None
            } else {
                Some((row.close / *spot_close) - 1.0)
            }
        })
        .collect()
}

fn aligned_relative_strength_series(
    spot_rows: &[StockHistoryRow],
    futures_rows: &[StockHistoryRow],
) -> Vec<f64> {
    let spot_returns = pairwise_returns(&spot_rows.iter().map(|row| row.close).collect::<Vec<_>>());
    let futures_returns =
        pairwise_returns(&futures_rows.iter().map(|row| row.close).collect::<Vec<_>>());
    spot_returns
        .into_iter()
        .zip(futures_returns)
        .map(|(spot_return, futures_return)| futures_return - spot_return)
        .collect()
}

fn append_distribution_features(
    feature_values: &mut BTreeMap<String, f64>,
    prefix: &str,
    values: &[f64],
) {
    if values.is_empty() {
        return;
    }
    for (suffix, quantile) in [
        ("min", 0.0),
        ("p10", 0.10),
        ("p25", 0.25),
        ("p50", 0.50),
        ("p75", 0.75),
        ("p90", 0.90),
        ("max", 1.0),
    ] {
        feature_values.insert(
            format!("{prefix}_{suffix}"),
            numeric_quantile(values, quantile),
        );
    }
}

fn numeric_quantile(values: &[f64], quantile: f64) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    if sorted.len() == 1 {
        return sorted[0];
    }
    let position = (sorted.len() - 1) as f64 * quantile.clamp(0.0, 1.0);
    let lower_index = position.floor() as usize;
    let upper_index = position.ceil() as usize;
    if lower_index == upper_index {
        sorted[lower_index]
    } else {
        let lower = sorted[lower_index];
        let upper = sorted[upper_index];
        lower + (upper - lower) * (position - lower_index as f64)
    }
}

fn persist_json<T: Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), SecurityScorecardTrainingError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| SecurityScorecardTrainingError::Persist(error.to_string()))?;
    }
    let payload = serde_json::to_vec_pretty(value)
        .map_err(|error| SecurityScorecardTrainingError::Persist(error.to_string()))?;
    fs::write(path, &payload)
        .map_err(|error| SecurityScorecardTrainingError::Persist(error.to_string()))?;
    Ok(())
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right.iter()).map(|(x, y)| x * y).sum()
}

fn logistic(value: f64) -> f64 {
    1.0 / (1.0 + (-value).exp())
}

fn default_created_at() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn default_lookback_days() -> usize {
    260
}

fn default_disclosure_limit() -> usize {
    8
}

fn default_stop_loss_pct() -> f64 {
    0.05
}

fn default_target_return_pct() -> f64 {
    0.12
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::stock::security_forward_outcome::SecurityForwardOutcomeDocument;
    use serde_json::Value;

    // 2026-04-20 CST: Added because the approved contract change must first lock
    // the new target-head surface in red tests before trainer behavior is updated.
    // Reason: the old trainer only accepted direction_head and kept label fields mixed.
    // Purpose: make contract drift visible before implementation.
    fn build_training_request(target_head: &str) -> SecurityScorecardTrainingRequest {
        SecurityScorecardTrainingRequest {
            created_at: "2026-04-20T21:30:00+08:00".to_string(),
            artifact_runtime_root: None,
            training_runtime_root: None,
            market_scope: "GLOBAL".to_string(),
            instrument_scope: "INDEX".to_string(),
            instrument_subscope: Some("nikkei_index".to_string()),
            symbol_list: vec!["NK225.IDX".to_string()],
            market_symbol: Some("NK225.IDX".to_string()),
            sector_symbol: Some("NK225.IDX".to_string()),
            futures_symbol: None,
            volume_proxy_symbol: None,
            capital_source_feature_mode: None,
            capital_flow_runtime_root: None,
            market_profile: None,
            sector_profile: None,
            horizon_days: 10,
            target_head: target_head.to_string(),
            train_range: "2025-01-01..2025-06-30".to_string(),
            valid_range: "2025-07-01..2025-09-30".to_string(),
            test_range: "2025-10-01..2025-12-31".to_string(),
            feature_set_version: "security_feature_snapshot.v1".to_string(),
            label_definition_version: "security_forward_outcome.v1".to_string(),
            lookback_days: 260,
            disclosure_limit: 8,
            stop_loss_pct: 0.05,
            target_return_pct: 0.12,
        }
    }

    #[test]
    fn validate_request_accepts_direction_up_and_down_heads() {
        assert!(validate_request(&build_training_request("direction_up_head")).is_ok());
        assert!(validate_request(&build_training_request("direction_down_head")).is_ok());
        assert!(validate_request(&build_training_request("repair_stable_head")).is_ok());
    }

    #[test]
    fn validate_request_accepts_nikkei_capital_source_feature_mode() {
        let mut request = build_training_request("direction_head");
        request.instrument_subscope = Some("nikkei_index".to_string());
        request.capital_source_feature_mode = Some("nikkei_jpx_mof_v1".to_string());
        request.capital_flow_runtime_root = Some("E:/SM/tests/runtime".to_string());
        assert!(validate_request(&request).is_ok());
    }

    #[test]
    fn validate_request_rejects_unknown_capital_source_feature_mode() {
        let mut request = build_training_request("direction_head");
        request.instrument_subscope = Some("nikkei_index".to_string());
        request.capital_source_feature_mode = Some("unsupported_mode".to_string());
        let error = validate_request(&request).expect_err("unsupported mode should be rejected");
        assert!(
            error
                .to_string()
                .contains("unsupported capital_source_feature_mode"),
            "error={error}"
        );
    }

    #[test]
    fn validate_request_rejects_missing_capital_flow_runtime_root_for_enhanced_mode() {
        let mut request = build_training_request("direction_head");
        request.instrument_subscope = Some("nikkei_index".to_string());
        request.capital_source_feature_mode = Some("nikkei_jpx_mof_v1".to_string());
        let error =
            validate_request(&request).expect_err("enhanced mode should require capital-flow root");
        assert!(
            error
                .to_string()
                .contains("capital_flow_runtime_root is required"),
            "error={error}"
        );
    }

    #[test]
    fn validate_request_rejects_conflicting_artifact_runtime_roots() {
        let mut request = build_training_request("direction_head");
        request.training_runtime_root = Some("E:/SM/legacy_artifacts".to_string());
        request.artifact_runtime_root = Some("E:/SM/new_artifacts".to_string());
        let error = validate_request(&request)
            .expect_err("conflicting artifact root aliases should be rejected");
        assert!(
            error
                .to_string()
                .contains("artifact_runtime_root conflicts"),
            "error={error}"
        );
    }

    #[test]
    fn build_artifact_serializes_target_label_definition_for_up_and_down_heads() {
        let trained_model = TrainedLogisticModel {
            intercept: 0.0,
            coefficients: Vec::new(),
        };

        let weekly_artifact_json = serde_json::to_value(build_artifact(
            &build_training_request("direction_head"),
            &[],
            &trained_model,
        ))
        .expect("weekly artifact should serialize");
        assert_eq!(
            weekly_artifact_json["target_label_definition"],
            Value::String("positive_return_1w".to_string())
        );
        assert_eq!(
            weekly_artifact_json["positive_label_definition"],
            Value::String("positive_return_1w".to_string())
        );

        let up_artifact_json = serde_json::to_value(build_artifact(
            &build_training_request("direction_up_head"),
            &[],
            &trained_model,
        ))
        .expect("up artifact should serialize");
        assert_eq!(
            up_artifact_json["target_label_definition"],
            Value::String("positive_return_10d".to_string())
        );
        assert_eq!(
            up_artifact_json["positive_label_definition"],
            Value::String("positive_return_10d".to_string())
        );

        let down_artifact_json = serde_json::to_value(build_artifact(
            &build_training_request("direction_down_head"),
            &[],
            &trained_model,
        ))
        .expect("down artifact should serialize");
        assert_eq!(
            down_artifact_json["target_label_definition"],
            Value::String("negative_return_10d".to_string())
        );
        assert_eq!(down_artifact_json["positive_label_definition"], Value::Null);

        let repair_artifact_json = serde_json::to_value(build_artifact(
            &build_training_request("repair_stable_head"),
            &[],
            &trained_model,
        ))
        .expect("repair artifact should serialize");
        assert_eq!(
            repair_artifact_json["target_label_definition"],
            Value::String("repair_stable_10d".to_string())
        );
        assert_eq!(
            repair_artifact_json["positive_label_definition"],
            Value::String("repair_stable_10d".to_string())
        );
    }

    #[test]
    fn resolve_training_label_uses_forward_return_direction_for_down_head() {
        let negative_outcome = SecurityForwardOutcomeDocument {
            outcome_id: "outcome-neg".to_string(),
            contract_version: "security_forward_outcome.v1".to_string(),
            document_type: "security_forward_outcome".to_string(),
            snapshot_id: "snapshot-neg".to_string(),
            symbol: "NK225.IDX".to_string(),
            market: "GLOBAL".to_string(),
            instrument_type: "INDEX".to_string(),
            as_of_date: "2025-10-08".to_string(),
            horizon_days: 10,
            forward_return: -0.031,
            max_drawdown: 0.042,
            max_runup: 0.005,
            positive_return: false,
            hit_upside_first: false,
            hit_stop_first: false,
            label_definition_version: "security_forward_outcome.v1".to_string(),
        };
        let positive_outcome = SecurityForwardOutcomeDocument {
            outcome_id: "outcome-pos".to_string(),
            contract_version: "security_forward_outcome.v1".to_string(),
            document_type: "security_forward_outcome".to_string(),
            snapshot_id: "snapshot-pos".to_string(),
            symbol: "NK225.IDX".to_string(),
            market: "GLOBAL".to_string(),
            instrument_type: "INDEX".to_string(),
            as_of_date: "2025-10-09".to_string(),
            horizon_days: 10,
            forward_return: 0.024,
            max_drawdown: 0.011,
            max_runup: 0.029,
            positive_return: true,
            hit_upside_first: false,
            hit_stop_first: false,
            label_definition_version: "security_forward_outcome.v1".to_string(),
        };

        assert_eq!(
            resolve_training_label(&negative_outcome, "direction_down_head"),
            1.0
        );
        assert_eq!(
            resolve_training_label(&positive_outcome, "direction_down_head"),
            0.0
        );
        assert_eq!(
            resolve_training_label(&positive_outcome, "direction_up_head"),
            1.0
        );
    }

    #[test]
    fn resolve_weekly_direction_label_uses_next_anchor_close_direction() {
        assert_eq!(resolve_weekly_direction_label(38000.0, 38250.0), Some(1.0));
        assert_eq!(resolve_weekly_direction_label(38000.0, 37950.0), Some(0.0));
        assert_eq!(resolve_weekly_direction_label(38000.0, 38000.0), Some(0.0));
        assert_eq!(resolve_weekly_direction_label(0.0, 38000.0), None);
    }

    #[test]
    fn resolve_repair_stable_label_separates_stable_repair_from_hit_and_fail() {
        assert_eq!(
            resolve_repair_stable_label_from_buckets(
                "weak_down",
                &[
                    "weak_down".to_string(),
                    "neutral".to_string(),
                    "weak_up".to_string(),
                ],
            ),
            Some(1.0)
        );
        assert_eq!(
            resolve_repair_stable_label_from_buckets(
                "strong_down",
                &[
                    "neutral".to_string(),
                    "weak_up".to_string(),
                    "weak_down".to_string(),
                ],
            ),
            Some(0.0)
        );
        assert_eq!(
            resolve_repair_stable_label_from_buckets(
                "strong_down",
                &[
                    "weak_down".to_string(),
                    "neutral".to_string(),
                    "neutral".to_string(),
                ],
            ),
            Some(0.0)
        );
        assert_eq!(
            resolve_repair_stable_label_from_buckets(
                "neutral",
                &["weak_up".to_string(), "weak_up".to_string()],
            ),
            None
        );
    }
}
