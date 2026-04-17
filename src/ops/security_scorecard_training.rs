use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

use crate::ops::stock::security_forward_outcome::{
    SecurityForwardOutcomeError, SecurityForwardOutcomeRequest, security_forward_outcome,
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
use crate::runtime::stock_history_store::{StockHistoryStore, StockHistoryStoreError};

// 2026-04-09 CST: 这里新增正式训练入口请求合同，原因是 Task 5 需要把离线训练从临时脚本提升为可治理的一等 Tool；
// 目的：集中冻结市场范围、样本范围、目标头与运行时路径，避免训练参数散落在 Skill 或 CLI 外层。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityScorecardTrainingRequest {
    #[serde(default = "default_created_at")]
    pub created_at: String,
    #[serde(default)]
    pub training_runtime_root: Option<String>,
    pub market_scope: String,
    pub instrument_scope: String,
    pub symbol_list: Vec<String>,
    #[serde(default)]
    pub market_symbol: Option<String>,
    #[serde(default)]
    pub sector_symbol: Option<String>,
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
}

#[derive(Debug, Clone, PartialEq)]
struct TrainedLogisticModel {
    intercept: f64,
    coefficients: Vec<f64>,
}

const UNSEEN_CATEGORICAL_BIN_LABEL: &str = "__unseen__";

// 2026-04-09 CST: 这里实现 Task 5 的最小正式训练入口，原因是我们需要先把训练主链跑通，再继续做回算重估和晋级治理；
// 目的：以最小的“样本采集 -> 分箱 -> WOE -> logistic -> artifact -> refit”闭环承接现有 scorecard 体系。
pub fn security_scorecard_training(
    request: &SecurityScorecardTrainingRequest,
) -> Result<SecurityScorecardTrainingResult, SecurityScorecardTrainingError> {
    validate_request(request)?;

    let train_range = parse_date_range(&request.train_range)?;
    let valid_range = parse_date_range(&request.valid_range)?;
    let test_range = parse_date_range(&request.test_range)?;
    let feature_configs = training_feature_configs();
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

    let metrics_summary_json = build_metrics_summary(&samples, &feature_models, &trained_model);
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
            instrument_subscope: None,
            model_grade: "candidate".to_string(),
            grade_reason: "training pipeline default candidate grade".to_string(),
        },
        comparison_to_champion_json: None,
        promotion_decision: Some("candidate_only".to_string()),
    })?;

    Ok(SecurityScorecardTrainingResult {
        artifact,
        artifact_path: artifact_path.to_string_lossy().to_string(),
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
    if request.target_head != "direction_head" {
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
    Ok(())
}

fn training_feature_configs() -> Vec<TrainingFeatureConfig> {
    vec![
        TrainingFeatureConfig {
            feature_name: "integrated_stance",
            group_name: "M",
            kind: TrainingFeatureKind::Categorical,
        },
        // 2026-04-16 CST: Added because A-1a starts the first formal regime/industry field
        // thickening pass before model-family upgrades.
        // Reason: the prior baseline lacked stable market-state segmentation, which made later
        // accuracy work look like a pure model problem.
        // Purpose: let training learn across market-regime and industry buckets instead of only
        // raw technical/fundamental event fields.
        TrainingFeatureConfig {
            feature_name: "market_regime",
            group_name: "M",
            kind: TrainingFeatureKind::Categorical,
        },
        TrainingFeatureConfig {
            feature_name: "industry_bucket",
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
            feature_name: "profit_signal",
            group_name: "F",
            kind: TrainingFeatureKind::Categorical,
        },
        TrainingFeatureConfig {
            feature_name: "fundamental_status",
            group_name: "F",
            kind: TrainingFeatureKind::Categorical,
        },
        TrainingFeatureConfig {
            feature_name: "disclosure_status",
            group_name: "E",
            kind: TrainingFeatureKind::Categorical,
        },
        TrainingFeatureConfig {
            feature_name: "announcement_count",
            group_name: "E",
            kind: TrainingFeatureKind::Numeric,
        },
        TrainingFeatureConfig {
            feature_name: "event_density_bucket",
            group_name: "Q",
            kind: TrainingFeatureKind::Categorical,
        },
        TrainingFeatureConfig {
            feature_name: "flow_status",
            group_name: "Q",
            kind: TrainingFeatureKind::Categorical,
        },
        TrainingFeatureConfig {
            feature_name: "disclosure_risk_keyword_count",
            group_name: "E",
            kind: TrainingFeatureKind::Numeric,
        },
        TrainingFeatureConfig {
            feature_name: "has_risk_warning_notice",
            group_name: "E",
            kind: TrainingFeatureKind::Categorical,
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
        TrainingFeatureConfig {
            feature_name: "revenue_yoy_pct",
            group_name: "F",
            kind: TrainingFeatureKind::Numeric,
        },
        TrainingFeatureConfig {
            feature_name: "net_profit_yoy_pct",
            group_name: "F",
            kind: TrainingFeatureKind::Numeric,
        },
        TrainingFeatureConfig {
            feature_name: "roe_pct",
            group_name: "F",
            kind: TrainingFeatureKind::Numeric,
        },
        TrainingFeatureConfig {
            feature_name: "valuation_status",
            group_name: "V",
            kind: TrainingFeatureKind::Categorical,
        },
    ]
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
    let store = StockHistoryStore::workspace_default()?;
    let mut samples = Vec::new();

    for symbol in &request.symbol_list {
        for (split_name, range, target_count) in [
            ("train", train_range, 2_usize),
            ("valid", valid_range, 1_usize),
            ("test", test_range, 1_usize),
        ] {
            let candidate_dates = load_dates_in_range(&store, symbol, range, 200)?;
            let selected_dates = select_evenly_spaced_dates(&candidate_dates, target_count);
            for as_of_date in selected_dates {
                let outcome_result = security_forward_outcome(&SecurityForwardOutcomeRequest {
                    symbol: symbol.clone(),
                    market_symbol: request.market_symbol.clone(),
                    sector_symbol: request.sector_symbol.clone(),
                    market_profile: request.market_profile.clone(),
                    sector_profile: request.sector_profile.clone(),
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
                let feature_values = extract_feature_values(
                    &outcome_result.snapshot.raw_features_json,
                    feature_configs,
                )?;
                samples.push(TrainingSample {
                    split_name: split_name.to_string(),
                    label: if outcome.positive_return { 1.0 } else { 0.0 },
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

fn load_dates_in_range(
    store: &StockHistoryStore,
    symbol: &str,
    range: &TrainingDateRange,
    min_history_rows: usize,
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
            qualified_dates.push(row.trade_date);
        }
    }

    Ok(qualified_dates)
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
            },
        )
        .collect::<Vec<_>>();
    // 2026-04-17 CST: Added because the sibling-repo training fix established one governed
    // unseen-category contract that StockMind should preserve as well.
    // Reason: train-only categorical bins are too thin once later evaluation touches values that
    // never appeared in the fit split.
    // Purpose: append one explicit neutral fallback bin instead of silently leaving the artifact
    // without a stable landing point for unseen categorical values.
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
        });
        lower = Some(*threshold);
    }
    bins.push(FeatureBinModel {
        bin_label: format!("bin_{}", thresholds.len() + 1),
        match_values: Vec::new(),
        min_inclusive: lower,
        max_exclusive: None,
        woe: 0.0,
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

fn build_unseen_categorical_bin() -> FeatureBinModel {
    FeatureBinModel {
        bin_label: UNSEEN_CATEGORICAL_BIN_LABEL.to_string(),
        match_values: vec![],
        min_inclusive: None,
        max_exclusive: None,
        // 2026-04-17 CST: Added because the fallback must keep inference numerically stable
        // without inventing unsupported direction for categories never observed in train.
        // Purpose: use a neutral WOE so later scoring can continue while keeping the artifact
        // contract explicit and inspectable.
        woe: 0.0,
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

fn build_artifact(
    request: &SecurityScorecardTrainingRequest,
    feature_models: &[FeatureModel],
    trained_model: &TrainedLogisticModel,
) -> SecurityScorecardModelArtifact {
    let model_id = format!(
        "{}_{}_{}d_{}",
        request.market_scope.to_lowercase(),
        request.instrument_scope.to_lowercase(),
        request.horizon_days,
        request.target_head
    );
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
        prediction_mode: Some("direction_probability".to_string()),
        prediction_baseline: None,
        training_window: Some(request.train_range.clone()),
        oot_window: Some(request.test_range.clone()),
        positive_label_definition: Some(format!("positive_return_{}d", request.horizon_days)),
        instrument_subscope: None,
        binning_version: Some("woe_binning.v1".to_string()),
        coefficient_version: Some("woe_logistic.v1".to_string()),
        model_sha256: None,
        intercept: Some(trained_model.intercept),
        base_score: 600.0,
        features,
    }
}

fn build_metrics_summary(
    samples: &[TrainingSample],
    feature_models: &[FeatureModel],
    trained_model: &TrainedLogisticModel,
) -> Value {
    let train_metrics = evaluate_split(samples, "train", feature_models, trained_model);
    let valid_metrics = evaluate_split(samples, "valid", feature_models, trained_model);
    let test_metrics = evaluate_split(samples, "test", feature_models, trained_model);

    json!({
        "train": train_metrics,
        "valid": valid_metrics,
        "test": test_metrics,
        "feature_count": feature_models.len(),
        "sample_count": samples.len(),
    })
}

fn evaluate_split(
    samples: &[TrainingSample],
    split_name: &str,
    feature_models: &[FeatureModel],
    trained_model: &TrainedLogisticModel,
) -> Value {
    let split_samples = samples_for_split(samples, split_name);
    if split_samples.is_empty() {
        return json!({
            "sample_count": 0,
            "accuracy": Value::Null,
            "positive_rate": Value::Null,
        });
    }

    let mut correct_count = 0_usize;
    let mut positive_count = 0_usize;
    for sample in &split_samples {
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
    request
        .training_runtime_root
        .as_ref()
        .map(|path| PathBuf::from(path.trim()))
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| {
            PathBuf::from(".worktrees")
                .join("SheetMind-Scenes-inspect")
                .join(".sheetmind_scenes_runtime")
        })
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
