use std::collections::BTreeMap;
use std::fs;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

use crate::ops::stock::security_decision_evidence_bundle::{
    ETF_DIFFERENTIATING_FEATURES, build_evidence_bundle_feature_seed, derive_event_density_bucket,
    derive_flow_status, derive_industry_bucket, derive_instrument_subscope, derive_market_regime,
    derive_valuation_status, is_etf_symbol, required_etf_feature_family, resolve_etf_subscope,
};
use crate::ops::stock::security_legacy_committee_compat::LegacySecurityDecisionCommitteeResult as SecurityDecisionCommitteeResult;

// 2026-04-09 CST: 这里新增正式评分卡对象合同，原因是用户明确要求证券评分卡不能再用主观分析分冒充正式治理对象；
// 目的：把评分结果升级为可落盘、可版本化、可进入 package、可做后续复盘与验真的正式 artifact。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityScorecardDocument {
    pub scorecard_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub symbol: String,
    pub analysis_date: String,
    pub decision_id: String,
    pub decision_ref: String,
    pub approval_ref: String,
    pub score_status: String,
    pub label_definition: String,
    pub model_binding: SecurityScorecardModelBinding,
    pub raw_feature_snapshot: BTreeMap<String, Value>,
    pub feature_contributions: Vec<SecurityScoreFeatureContribution>,
    pub group_breakdown: Vec<SecurityScoreGroupBreakdown>,
    pub base_score: Option<f64>,
    pub total_score: Option<f64>,
    pub success_probability: Option<f64>,
    // 2026-04-09 CST: 这里新增量化信号字段，原因是 Task 1 要把 scorecard 正式语义收敛为量化线输出，
    // 目的：让主席线与后续总卡明确消费 quant_signal，而不是把 recommendation_action 误当成最终正式决议。
    pub quant_signal: String,
    // 2026-04-09 CST: 这里新增量化立场字段，原因是用户要求量化线和主席线彻底分开，
    // 目的：沉淀 scorecard 自身的量化方向语义，避免后续继续复用旧字段造成混线。
    pub quant_stance: String,
    pub recommendation_action: String,
    pub exposure_side: String,
    pub score_summary: String,
    pub limitations: Vec<String>,
}

// 2026-04-09 CST: 这里显式保留模型绑定元数据，原因是评分卡后续必须能追溯到“哪一版分箱/系数/训练窗口”；
// 目的：让 package、verify 和复盘都能基于稳定字段追踪模型来源，而不是退回口头说明。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityScorecardModelBinding {
    pub model_id: Option<String>,
    pub model_version: Option<String>,
    pub training_window: Option<String>,
    pub oot_window: Option<String>,
    pub positive_label_definition: Option<String>,
    pub instrument_subscope: Option<String>,
    pub binning_version: Option<String>,
    pub coefficient_version: Option<String>,
    pub model_sha256: Option<String>,
}

// 2026-04-09 CST: 这里记录单特征贡献明细，原因是用户要求以后必须能解释“这个分数是怎么算出来的”；
// 目的：把原始值、命中的分箱、WOE、points 和归因分组一起落盘，避免评分卡再次退化成黑盒总分。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityScoreFeatureContribution {
    pub feature_name: String,
    pub group_name: String,
    pub raw_value: Value,
    pub bin_label: Option<String>,
    pub matched: bool,
    pub woe: Option<f64>,
    pub logit_contribution: Option<f64>,
    pub points: f64,
}

// 2026-04-09 CST: 这里新增分组归因摘要，原因是正式评分卡后续仍需要保留 T/F/E/V 这类用户可读归因视角；
// 目的：在不手工指定主观权重的前提下，把模型分数按组做聚合展示，方便复盘和后续调参。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityScoreGroupBreakdown {
    pub group_name: String,
    pub feature_count: usize,
    pub point_total: f64,
}

// 2026-04-09 CST: 这里定义评分卡构建输入，原因是评分卡既依赖投决结果，也依赖运行时锚点与可选模型路径；
// 目的：把落盘所需元数据集中收口，避免 submit_approval 继续膨胀成手写 JSON 拼装器。
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityScorecardBuildInput {
    pub generated_at: String,
    pub decision_id: String,
    pub decision_ref: String,
    pub approval_ref: String,
    pub scorecard_model_path: Option<String>,
}

// 2026-04-09 CST: 这里新增模型 artifact 合同，原因是本轮正式边界是“消费离线训练产物”，不是运行时手工拍权重；
// 目的：为后续真实分箱/WOE/贡献回归结果预留稳定输入格式，同时本轮先支持无模型时的正式退化语义。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityScorecardModelArtifact {
    pub model_id: String,
    pub model_version: String,
    pub label_definition: String,
    #[serde(default)]
    pub target_head: Option<String>,
    #[serde(default)]
    pub prediction_mode: Option<String>,
    #[serde(default)]
    pub prediction_baseline: Option<f64>,
    #[serde(default)]
    pub training_window: Option<String>,
    #[serde(default)]
    pub oot_window: Option<String>,
    #[serde(default)]
    pub positive_label_definition: Option<String>,
    #[serde(default)]
    pub instrument_subscope: Option<String>,
    #[serde(default)]
    pub binning_version: Option<String>,
    #[serde(default)]
    pub coefficient_version: Option<String>,
    #[serde(default)]
    pub model_sha256: Option<String>,
    #[serde(default)]
    pub intercept: Option<f64>,
    pub base_score: f64,
    #[serde(default)]
    pub features: Vec<SecurityScorecardModelFeatureSpec>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityScorecardModelFeatureSpec {
    pub feature_name: String,
    pub group_name: String,
    #[serde(default)]
    pub bins: Vec<SecurityScorecardModelBin>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityScorecardModelBin {
    pub bin_label: String,
    #[serde(default)]
    pub match_values: Vec<String>,
    #[serde(default)]
    pub min_inclusive: Option<f64>,
    #[serde(default)]
    pub max_exclusive: Option<f64>,
    #[serde(default)]
    pub woe: Option<f64>,
    #[serde(default)]
    pub logit_contribution: Option<f64>,
    #[serde(default)]
    pub points: f64,
    #[serde(default)]
    pub predicted_value: Option<f64>,
}

#[derive(Debug, Error)]
pub enum SecurityScorecardError {
    #[error("证券评分卡构建失败: {0}")]
    Build(String),
}

pub fn build_security_scorecard(
    committee: &SecurityDecisionCommitteeResult,
    input: &SecurityScorecardBuildInput,
) -> Result<SecurityScorecardDocument, SecurityScorecardError> {
    let raw_feature_snapshot = build_raw_feature_snapshot(committee);
    let scorecard_id = format!("scorecard-{}", input.decision_id);
    let recommendation_action = committee.decision_card.recommendation_action.clone();
    let exposure_side = committee.decision_card.exposure_side.clone();
    // 2026-04-09 CST: 这里先生成量化线自身字段，原因是 Task 1 要明确 scorecard 是量化线，不是主席线；
    // 目的：即便当前仍保留 recommendation_action / exposure_side 兼容字段，也要同时落正式 quant_signal / quant_stance。
    let fallback_quant_signal = derive_quant_signal(None, &recommendation_action);
    let fallback_quant_stance = derive_quant_stance(None, &exposure_side);

    let Some(model_path) = input
        .scorecard_model_path
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    else {
        return Ok(SecurityScorecardDocument {
            scorecard_id,
            contract_version: "security_scorecard.v1".to_string(),
            document_type: "security_scorecard".to_string(),
            generated_at: input.generated_at.clone(),
            symbol: committee.symbol.clone(),
            analysis_date: committee.analysis_date.clone(),
            decision_id: input.decision_id.clone(),
            decision_ref: input.decision_ref.clone(),
            approval_ref: input.approval_ref.clone(),
            score_status: "model_unavailable".to_string(),
            label_definition: "horizon_10d_stop_5pct_target_10pct".to_string(),
            model_binding: SecurityScorecardModelBinding {
                model_id: None,
                model_version: None,
                training_window: None,
                oot_window: None,
                positive_label_definition: None,
                instrument_subscope: None,
                binning_version: None,
                coefficient_version: None,
                model_sha256: None,
            },
            raw_feature_snapshot,
            feature_contributions: Vec::new(),
            group_breakdown: Vec::new(),
            base_score: None,
            total_score: None,
            success_probability: None,
            quant_signal: fallback_quant_signal,
            quant_stance: fallback_quant_stance,
            recommendation_action,
            exposure_side,
            score_summary:
                "未提供评分卡模型 artifact，系统已落正式 scorecard 对象，但不会伪造主观分数。"
                    .to_string(),
            limitations: vec![
                "当前未提供评分卡模型 artifact，无法执行分箱、WOE 与点数累加。".to_string(),
                "本轮只保留正式对象、原始特征快照与治理链挂接，不输出伪造分数。".to_string(),
            ],
        });
    };

    let model = load_scorecard_model(model_path)?;
    if let Some(score_status) =
        etf_cross_section_guard_status(&committee.symbol, &raw_feature_snapshot, &model)
    {
        return Ok(SecurityScorecardDocument {
            scorecard_id,
            contract_version: "security_scorecard.v1".to_string(),
            document_type: "security_scorecard".to_string(),
            generated_at: input.generated_at.clone(),
            symbol: committee.symbol.clone(),
            analysis_date: committee.analysis_date.clone(),
            decision_id: input.decision_id.clone(),
            decision_ref: input.decision_ref.clone(),
            approval_ref: input.approval_ref.clone(),
            score_status: score_status.clone(),
            label_definition: model.label_definition.clone(),
            model_binding: SecurityScorecardModelBinding {
                model_id: Some(model.model_id.clone()),
                model_version: Some(model.model_version.clone()),
                training_window: model.training_window.clone(),
                oot_window: model.oot_window.clone(),
                positive_label_definition: model.positive_label_definition.clone(),
                instrument_subscope: model.instrument_subscope.clone(),
                binning_version: model.binning_version.clone(),
                coefficient_version: model.coefficient_version.clone(),
                model_sha256: model.model_sha256.clone(),
            },
            raw_feature_snapshot,
            feature_contributions: Vec::new(),
            group_breakdown: Vec::new(),
            base_score: None,
            total_score: None,
            success_probability: None,
            quant_signal: derive_quant_signal(Some(&score_status), &recommendation_action),
            quant_stance: derive_quant_stance(Some(&score_status), &exposure_side),
            recommendation_action,
            exposure_side,
            score_summary:
                "ETF scorecard runtime rejected the current model binding because the model lacks the required ETF feature family for this sub-pool or the ETF sub-pool does not match."
                    .to_string(),
            limitations: vec![
                "当前 ETF 评分卡绑定的模型缺少 ETF 专用特征族，或其 ETF 子池与当前标的不一致，不能用于横截面对比或可执行概率判断。".to_string(),
                "请为当前 ETF 子池使用单独训练出的 ETF scorecard artifact，再进入主席裁决与审批主链。".to_string(),
            ],
        });
    }
    let contributions = score_features(&model, &raw_feature_snapshot);
    let total_points = contributions.iter().map(|item| item.points).sum::<f64>();
    let total_score = model.base_score + total_points;
    let group_breakdown = build_group_breakdown(&contributions);
    let score_status = if contributions.iter().all(|item| item.matched) {
        "ready"
    } else {
        "feature_incomplete"
    };
    let limitations = if score_status == "ready" {
        Vec::new()
    } else {
        vec!["部分特征未命中模型分箱，当前评分结果仅可用于治理留痕与复核。".to_string()]
    };
    let quant_signal = derive_quant_signal(Some(score_status), &recommendation_action);
    let quant_stance = derive_quant_stance(Some(score_status), &exposure_side);
    let success_probability = model.intercept.map(|intercept| {
        logistic(
            intercept
                + contributions
                    .iter()
                    .filter_map(|item| item.logit_contribution)
                    .sum::<f64>(),
        )
    });

    Ok(SecurityScorecardDocument {
        scorecard_id,
        contract_version: "security_scorecard.v1".to_string(),
        document_type: "security_scorecard".to_string(),
        generated_at: input.generated_at.clone(),
        symbol: committee.symbol.clone(),
        analysis_date: committee.analysis_date.clone(),
        decision_id: input.decision_id.clone(),
        decision_ref: input.decision_ref.clone(),
        approval_ref: input.approval_ref.clone(),
        score_status: score_status.to_string(),
        label_definition: model.label_definition.clone(),
        model_binding: SecurityScorecardModelBinding {
            model_id: Some(model.model_id.clone()),
            model_version: Some(model.model_version.clone()),
            training_window: model.training_window.clone(),
            oot_window: model.oot_window.clone(),
            positive_label_definition: model.positive_label_definition.clone(),
            instrument_subscope: model.instrument_subscope.clone(),
            binning_version: model.binning_version.clone(),
            coefficient_version: model.coefficient_version.clone(),
            model_sha256: model.model_sha256.clone(),
        },
        raw_feature_snapshot,
        feature_contributions: contributions,
        group_breakdown,
        base_score: Some(model.base_score),
        total_score: Some(total_score),
        success_probability,
        quant_signal,
        quant_stance,
        recommendation_action,
        exposure_side,
        score_summary: format!(
            "评分卡已基于模型 {}:{} 完成打分。",
            model.model_id, model.model_version
        ),
        limitations,
    })
}

pub(crate) fn load_scorecard_model(
    path: &str,
) -> Result<SecurityScorecardModelArtifact, SecurityScorecardError> {
    let payload = fs::read(path).map_err(|error| {
        SecurityScorecardError::Build(format!("failed to read scorecard model: {error}"))
    })?;
    serde_json::from_slice(&payload).map_err(|error| {
        SecurityScorecardError::Build(format!("failed to parse scorecard model: {error}"))
    })
}

fn build_raw_feature_snapshot(
    committee: &SecurityDecisionCommitteeResult,
) -> BTreeMap<String, Value> {
    let warn_count = committee
        .risk_gates
        .iter()
        .filter(|gate| gate.result == "warn")
        .count();
    // 2026-04-11 CST: Reuse the unified evidence seed as the runtime scorecard base,
    // because ETF and equity model families now need one canonical raw snapshot instead
    // of separate hand-built field lists that can drift apart.
    // Purpose: make runtime scorecard and training consume the same ETF/equity raw
    // feature universe before vote-only fields are appended.
    let mut snapshot = build_evidence_bundle_feature_seed(&committee.evidence_bundle);
    // 2026-04-12 UTC+08: Normalize ETF integrated stance before runtime scoring,
    // because ETF information-layer upgrades now emit richer stance labels such as
    // `mixed_watch` and `watchful_positive`, while the ETF scorecard should consume
    // one governed modeling bucket instead of drifting with presentation wording.
    // Purpose: keep ETF runtime scoring aligned with ETF retraining so final scorecards
    // stop falling into `feature_incomplete` only because the stance label got richer.
    if is_etf_symbol(&committee.symbol) {
        if let Some(Value::String(stance)) = snapshot.get("integrated_stance") {
            snapshot.insert(
                "integrated_stance".to_string(),
                Value::String(normalize_integrated_stance_for_modeling(stance)),
            );
        }
    }
    // 2026-04-15 CST: Added because ETF runtime subscope resolution in scorecard
    // needs the same market and sector context that committee requests already carried.
    // Reason: cross-border ETF approval could see complete proxy inputs but still miss
    // its ETF subtype when raw snapshot omitted market_profile and sector_profile.
    // Purpose: keep scorecard gating aligned with request-time ETF context without
    // widening the evidence contract during this stop-gap repair round.
    if let Some(market_profile) = committee.market_profile.as_ref() {
        snapshot.insert(
            "market_profile".to_string(),
            Value::String(market_profile.clone()),
        );
    }
    if let Some(sector_profile) = committee.sector_profile.as_ref() {
        snapshot.insert(
            "sector_profile".to_string(),
            Value::String(sector_profile.clone()),
        );
    }
    let subject_asset_class = snapshot.get("subject_asset_class").and_then(Value::as_str);
    let instrument_subscope = derive_instrument_subscope(
        &committee.symbol,
        committee.market_profile.as_deref(),
        subject_asset_class,
    );
    let industry_bucket = derive_industry_bucket(
        committee.sector_profile.as_deref(),
        Some(&instrument_subscope),
        subject_asset_class,
    );
    let market_regime =
        derive_market_regime(committee.market_profile.as_deref(), subject_asset_class);
    let announcement_count = snapshot
        .get("announcement_count")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let disclosure_risk_keyword_count = snapshot
        .get("disclosure_risk_keyword_count")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let event_density_bucket =
        derive_event_density_bucket(announcement_count, disclosure_risk_keyword_count);
    let flow_status = derive_flow_status(
        snapshot.get("money_flow_signal").and_then(Value::as_str),
        snapshot.get("volume_confirmation").and_then(Value::as_str),
    );
    let valuation_status = derive_valuation_status(
        snapshot
            .get("range_position_signal")
            .and_then(Value::as_str),
        snapshot
            .get("bollinger_position_signal")
            .and_then(Value::as_str),
        snapshot
            .get("mean_reversion_signal")
            .and_then(Value::as_str),
    );
    // 2026-04-16 CST: Added because runtime scorecard must consume the same thickened
    // segmentation vocabulary that snapshot/training now freeze.
    // Reason: otherwise replay/training/runtime would drift on market regime and Q/V proxy fields.
    // Purpose: keep the main securities chain on one canonical raw feature contract.
    snapshot.insert("market_regime".to_string(), Value::String(market_regime));
    snapshot.insert(
        "industry_bucket".to_string(),
        Value::String(industry_bucket),
    );
    snapshot.insert(
        "instrument_subscope".to_string(),
        Value::String(instrument_subscope),
    );
    snapshot.insert(
        "event_density_bucket".to_string(),
        Value::String(event_density_bucket),
    );
    snapshot.insert("flow_status".to_string(), Value::String(flow_status));
    snapshot.insert(
        "valuation_status".to_string(),
        Value::String(valuation_status),
    );
    snapshot.insert("warn_count".to_string(), json!(warn_count));
    snapshot.insert(
        "majority_vote".to_string(),
        Value::String(committee.vote_tally.majority_vote.clone()),
    );
    snapshot.insert(
        "majority_count".to_string(),
        json!(committee.vote_tally.majority_count),
    );
    snapshot.insert(
        "buy_count".to_string(),
        json!(committee.vote_tally.buy_count),
    );
    snapshot.insert(
        "hold_count".to_string(),
        json!(committee.vote_tally.hold_count),
    );
    snapshot.insert(
        "reduce_count".to_string(),
        json!(committee.vote_tally.reduce_count),
    );
    snapshot.insert(
        "avoid_count".to_string(),
        json!(committee.vote_tally.avoid_count),
    );
    snapshot.insert(
        "abstain_count".to_string(),
        json!(committee.vote_tally.abstain_count),
    );
    snapshot.insert(
        "risk_veto_status".to_string(),
        Value::String(committee.risk_veto.status.clone()),
    );
    snapshot.insert(
        "committee_confidence_score".to_string(),
        json!(committee.decision_card.confidence_score),
    );
    snapshot.insert(
        "recommendation_action".to_string(),
        Value::String(committee.decision_card.recommendation_action.clone()),
    );
    snapshot.insert(
        "exposure_side".to_string(),
        Value::String(committee.decision_card.exposure_side.clone()),
    );
    snapshot
}

// 2026-04-12 UTC+08: Centralize ETF integrated-stance modeling buckets, because
// ETF information synthesis now uses richer user-facing labels than the original
// scorecard artifacts were trained on.
// Purpose: let training and runtime share one auditable ETF stance vocabulary
// without flattening the richer analysis wording shown to users.
pub(crate) fn normalize_integrated_stance_for_modeling(raw: &str) -> String {
    match raw.trim() {
        "technical_only" | "mixed_watch" | "watchful_positive" | "neutral" => {
            "watchful_context".to_string()
        }
        "constructive" | "positive" => "constructive".to_string(),
        "cautious" | "watchful_negative" | "negative" => "cautious".to_string(),
        other => other.to_string(),
    }
}

// 2026-04-11 CST: Guard ETF runtime against wrong model-family bindings, because an
// ETF should not surface actionable probabilities when the bound model lacks the ETF
// differentiating feature family.
// Purpose: downgrade invalid ETF outputs before chair/approval consumers mistake a
// coarse shared model for a trustworthy ETF comparison signal.
fn etf_cross_section_guard_status(
    symbol: &str,
    raw_feature_snapshot: &BTreeMap<String, Value>,
    model: &SecurityScorecardModelArtifact,
) -> Option<String> {
    if !is_etf_symbol(symbol) {
        return None;
    }

    let model_features = model
        .features
        .iter()
        .map(|feature| feature.feature_name.as_str())
        .collect::<Vec<_>>();
    let model_has_etf_family = ETF_DIFFERENTIATING_FEATURES.iter().any(|feature_name| {
        model_features
            .iter()
            .any(|candidate| candidate == feature_name)
    });
    let snapshot_has_etf_family = ETF_DIFFERENTIATING_FEATURES
        .iter()
        .any(|feature_name| matches!(raw_feature_snapshot.get(*feature_name), Some(value) if !value.is_null()));
    let runtime_subscope = resolve_etf_subscope(
        symbol,
        raw_feature_snapshot
            .get("market_profile")
            .and_then(Value::as_str),
        raw_feature_snapshot
            .get("sector_profile")
            .and_then(Value::as_str),
    );
    let required_feature_family =
        required_etf_feature_family(runtime_subscope.or(model.instrument_subscope.as_deref()));
    let model_has_required_feature_family = required_feature_family.iter().all(|feature_name| {
        model_features
            .iter()
            .any(|candidate| candidate == feature_name)
    });

    if snapshot_has_etf_family && (!model_has_etf_family || !model_has_required_feature_family) {
        Some("cross_section_invalid".to_string())
    } else if let (Some(runtime_subscope), Some(model_subscope)) =
        (runtime_subscope, model.instrument_subscope.as_deref())
    {
        if runtime_subscope != model_subscope {
            Some("cross_section_invalid".to_string())
        } else {
            None
        }
    } else if snapshot_has_etf_family && model.instrument_subscope.is_none() {
        Some("cross_section_invalid".to_string())
    } else {
        None
    }
}

fn score_features(
    model: &SecurityScorecardModelArtifact,
    raw_feature_snapshot: &BTreeMap<String, Value>,
) -> Vec<SecurityScoreFeatureContribution> {
    model
        .features
        .iter()
        .map(|feature| {
            let raw_value = raw_feature_snapshot
                .get(&feature.feature_name)
                .cloned()
                .unwrap_or(Value::Null);
            let matched_bin = feature
                .bins
                .iter()
                .find(|bin| value_matches_bin(&raw_value, bin));
            match matched_bin {
                Some(bin) => SecurityScoreFeatureContribution {
                    feature_name: feature.feature_name.clone(),
                    group_name: feature.group_name.clone(),
                    raw_value,
                    bin_label: Some(bin.bin_label.clone()),
                    matched: true,
                    woe: bin.woe,
                    logit_contribution: bin.logit_contribution,
                    points: bin.points,
                },
                None => SecurityScoreFeatureContribution {
                    feature_name: feature.feature_name.clone(),
                    group_name: feature.group_name.clone(),
                    raw_value,
                    bin_label: None,
                    matched: false,
                    woe: None,
                    logit_contribution: None,
                    points: 0.0,
                },
            }
        })
        .collect()
}

pub(crate) fn value_matches_bin(value: &Value, bin: &SecurityScorecardModelBin) -> bool {
    if !bin.match_values.is_empty() {
        // 2026-04-12 UTC+08: Accept a governed categorical fallback bucket here,
        // because pooled ETF validation now needs unseen holdout categories to stay
        // scorable instead of downgrading the whole scorecard to incomplete.
        // Purpose: let training publish `__other__` as the final categorical bin and
        // have runtime matching consume it only after explicit labels miss.
        let wildcard_match = bin.match_values.iter().any(|item| item == "__other__");
        // 2026-04-16 CST: Normalize primitive categorical runtime values before
        // string-bin matching, because governed scorecard artifacts currently store
        // boolean categories like `false` as strings while raw snapshots can still
        // surface them as JSON booleans.
        // Purpose: fix the real categorical contract gap at the matcher layer so
        // ready scorecards do not degrade to `feature_incomplete` for bool features.
        let Some(raw) = categorical_match_key(value) else {
            return wildcard_match;
        };
        return bin.match_values.iter().any(|item| item == raw) || wildcard_match;
    }

    let Some(number) = value.as_f64() else {
        return false;
    };
    let lower_ok = bin
        .min_inclusive
        .map(|lower| number >= lower)
        .unwrap_or(true);
    let upper_ok = bin
        .max_exclusive
        .map(|upper| number < upper)
        .unwrap_or(true);
    lower_ok && upper_ok
}

fn categorical_match_key(value: &Value) -> Option<&str> {
    match value {
        Value::String(raw) => Some(raw.as_str()),
        Value::Bool(true) => Some("true"),
        Value::Bool(false) => Some("false"),
        _ => None,
    }
}

// 2026-04-11 CST: Add a shared regression-head reader, because P3 introduces
// return/drawdown/path artifacts that must be consumed by master_scorecard and
// chair_resolution without inventing a second artifact format.
// Purpose: keep multi-head prediction loading governed by the same model schema
// while preserving backward compatibility for the legacy direction scorecard.
pub(crate) fn predict_regression_head_value(
    model: &SecurityScorecardModelArtifact,
    raw_feature_snapshot: &BTreeMap<String, Value>,
) -> Option<f64> {
    if model.prediction_mode.as_deref() != Some("regression") {
        return None;
    }

    let matched_values = model
        .features
        .iter()
        .filter_map(|feature| {
            let raw_value = raw_feature_snapshot.get(&feature.feature_name)?;
            feature
                .bins
                .iter()
                .find(|bin| value_matches_bin(raw_value, bin))
                .and_then(|bin| bin.predicted_value)
        })
        .collect::<Vec<_>>();

    if matched_values.is_empty() {
        model.prediction_baseline
    } else {
        Some(matched_values.iter().sum::<f64>() / matched_values.len() as f64)
    }
}

// 2026-04-11 CST: Add a classification-head probability helper, because P4
// master_scorecard now needs upside-first and stop-first probabilities to travel
// beside the regression heads as governed path-event context.
// Purpose: let downstream consumers reuse the scorecard artifact contract instead of
// inventing a second probability decoder for classification heads.
pub(crate) fn predict_classification_head_probability(
    model: &SecurityScorecardModelArtifact,
    raw_feature_snapshot: &BTreeMap<String, Value>,
) -> Option<f64> {
    if model.prediction_mode.as_deref() != Some("classification") {
        return None;
    }

    let base_intercept = model.intercept.unwrap_or(0.0);
    let logit_sum = model
        .features
        .iter()
        .filter_map(|feature| {
            let raw_value = raw_feature_snapshot.get(&feature.feature_name)?;
            feature
                .bins
                .iter()
                .find(|bin| value_matches_bin(raw_value, bin))
                .and_then(|bin| bin.logit_contribution)
        })
        .sum::<f64>();
    Some(logistic(base_intercept + logit_sum))
}

fn build_group_breakdown(
    contributions: &[SecurityScoreFeatureContribution],
) -> Vec<SecurityScoreGroupBreakdown> {
    let mut grouped = BTreeMap::<String, (usize, f64)>::new();
    for contribution in contributions {
        let entry = grouped
            .entry(contribution.group_name.clone())
            .or_insert((0_usize, 0.0_f64));
        entry.0 += 1;
        entry.1 += contribution.points;
    }
    grouped
        .into_iter()
        .map(
            |(group_name, (feature_count, point_total))| SecurityScoreGroupBreakdown {
                group_name,
                feature_count,
                point_total,
            },
        )
        .collect()
}

fn logistic(value: f64) -> f64 {
    1.0 / (1.0 + (-value).exp())
}

// 2026-04-09 CST: 这里集中维护量化信号映射，原因是 scorecard 已被明确为量化线正式输出，
// 目的：让 chair_resolution / master_scorecard / replay 都消费同一套 quant_signal 口径，避免各处手写导致语义漂移。
fn derive_quant_signal(score_status: Option<&str>, fallback_action: &str) -> String {
    match score_status {
        Some("ready") => format!("quant_{fallback_action}"),
        Some("feature_incomplete") => "quant_incomplete".to_string(),
        Some(_) => "quant_unavailable".to_string(),
        None => "quant_unavailable".to_string(),
    }
}

// 2026-04-09 CST: 这里集中维护量化立场映射，原因是 scorecard 与主席线都会读取这一层方向语义，
// 目的：把“量化怎么想”和“主席最终怎么裁决”明确拆开，同时保持回放和复盘口径稳定。
fn derive_quant_stance(score_status: Option<&str>, fallback_exposure_side: &str) -> String {
    match score_status {
        Some("ready") => fallback_exposure_side.to_string(),
        Some("feature_incomplete") => "guarded".to_string(),
        Some(_) => "unavailable".to_string(),
        None => "unavailable".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::{
        SecurityScorecardModelArtifact, SecurityScorecardModelFeatureSpec,
        etf_cross_section_guard_status, normalize_integrated_stance_for_modeling,
        value_matches_bin,
    };

    #[test]
    fn etf_runtime_guard_rejects_models_without_etf_specific_feature_family() {
        // 2026-04-11 CST: Add an ETF runtime guard red test, reason: different ETF
        // symbols previously produced identical probabilities because the model only
        // consumed a coarse shared feature family.
        // Purpose: require runtime scorecard to downgrade ETF outputs when the bound
        // model does not carry ETF-specific differentiating features.
        let model = SecurityScorecardModelArtifact {
            model_id: "a_share_etf_10d_direction_head".to_string(),
            model_version: "candidate_20260411".to_string(),
            label_definition: "security_forward_outcome.v1".to_string(),
            target_head: None,
            prediction_mode: None,
            prediction_baseline: None,
            training_window: None,
            oot_window: None,
            positive_label_definition: None,
            instrument_subscope: None,
            binning_version: None,
            coefficient_version: None,
            model_sha256: None,
            intercept: Some(0.0),
            base_score: 600.0,
            features: vec![
                SecurityScorecardModelFeatureSpec {
                    feature_name: "integrated_stance".to_string(),
                    group_name: "M".to_string(),
                    bins: Vec::new(),
                },
                SecurityScorecardModelFeatureSpec {
                    feature_name: "technical_alignment".to_string(),
                    group_name: "T".to_string(),
                    bins: Vec::new(),
                },
                SecurityScorecardModelFeatureSpec {
                    feature_name: "data_gap_count".to_string(),
                    group_name: "R".to_string(),
                    bins: Vec::new(),
                },
                SecurityScorecardModelFeatureSpec {
                    feature_name: "risk_note_count".to_string(),
                    group_name: "R".to_string(),
                    bins: Vec::new(),
                },
            ],
        };
        let raw_snapshot = BTreeMap::from([
            ("close_vs_sma50".to_string(), json!(0.012)),
            ("volume_ratio_20".to_string(), json!(1.18)),
            ("rsrs_zscore_18_60".to_string(), json!(0.76)),
        ]);

        assert_eq!(
            etf_cross_section_guard_status("511010.SH", &raw_snapshot, &model),
            Some("cross_section_invalid".to_string())
        );
    }

    #[test]
    fn etf_runtime_guard_rejects_wrong_etf_subscope_binding() {
        // 2026-04-11 CST: Add a red test for ETF sub-pool mismatch, reason:
        // bond ETF, gold ETF, and cross-border ETF should not be treated as one
        // interchangeable ETF bucket once separate model families are introduced.
        // Purpose: require runtime scorecard to reject an ETF artifact whose declared
        // sub-pool does not match the live ETF request context.
        let model = SecurityScorecardModelArtifact {
            model_id: "a_share_etf_equity_etf_10d_direction_head".to_string(),
            model_version: "candidate_20260411".to_string(),
            label_definition: "security_forward_outcome.v1".to_string(),
            target_head: None,
            prediction_mode: None,
            prediction_baseline: None,
            training_window: None,
            oot_window: None,
            positive_label_definition: None,
            instrument_subscope: Some("equity_etf".to_string()),
            binning_version: None,
            coefficient_version: None,
            model_sha256: None,
            intercept: Some(0.0),
            base_score: 600.0,
            features: vec![
                SecurityScorecardModelFeatureSpec {
                    feature_name: "close_vs_sma50".to_string(),
                    group_name: "T".to_string(),
                    bins: Vec::new(),
                },
                SecurityScorecardModelFeatureSpec {
                    feature_name: "volume_ratio_20".to_string(),
                    group_name: "T".to_string(),
                    bins: Vec::new(),
                },
                SecurityScorecardModelFeatureSpec {
                    feature_name: "rsrs_zscore_18_60".to_string(),
                    group_name: "T".to_string(),
                    bins: Vec::new(),
                },
            ],
        };
        let mut raw_snapshot = BTreeMap::new();
        raw_snapshot.insert("close_vs_sma50".to_string(), json!(0.01));
        raw_snapshot.insert("volume_ratio_20".to_string(), json!(1.02));
        raw_snapshot.insert("rsrs_zscore_18_60".to_string(), json!(0.15));
        raw_snapshot.insert("market_profile".to_string(), json!("a_share_core"));
        raw_snapshot.insert("sector_profile".to_string(), json!("bond_etf_peer"));

        assert_eq!(
            etf_cross_section_guard_status("511010.SH", &raw_snapshot, &model),
            Some("cross_section_invalid".to_string())
        );
    }

    #[test]
    fn etf_runtime_guard_rejects_treasury_binding_without_treasury_feature_family() {
        // 2026-04-11 CST: Add a red test for subscope-specific ETF feature families, reason:
        // a treasury ETF artifact that only carries equity-ETF-style features still remains a
        // structurally wrong quantitative binding even if its declared subscope says treasury.
        // Purpose: force runtime governance to validate the minimum treasury ETF factor family
        // instead of trusting the artifact's subscope label alone.
        let model = SecurityScorecardModelArtifact {
            model_id: "a_share_etf_treasury_etf_10d_direction_head".to_string(),
            model_version: "candidate_20260411".to_string(),
            label_definition: "security_forward_outcome.v1".to_string(),
            target_head: None,
            prediction_mode: None,
            prediction_baseline: None,
            training_window: None,
            oot_window: None,
            positive_label_definition: None,
            instrument_subscope: Some("treasury_etf".to_string()),
            binning_version: None,
            coefficient_version: None,
            model_sha256: None,
            intercept: Some(0.0),
            base_score: 600.0,
            features: vec![
                SecurityScorecardModelFeatureSpec {
                    feature_name: "close_vs_sma50".to_string(),
                    group_name: "T".to_string(),
                    bins: Vec::new(),
                },
                SecurityScorecardModelFeatureSpec {
                    feature_name: "volume_ratio_20".to_string(),
                    group_name: "T".to_string(),
                    bins: Vec::new(),
                },
                SecurityScorecardModelFeatureSpec {
                    feature_name: "support_gap_pct_20".to_string(),
                    group_name: "T".to_string(),
                    bins: Vec::new(),
                },
            ],
        };
        let mut raw_snapshot = BTreeMap::new();
        raw_snapshot.insert("close_vs_sma200".to_string(), json!(0.004));
        raw_snapshot.insert("boll_width_ratio_20".to_string(), json!(0.012));
        raw_snapshot.insert("atr_14".to_string(), json!(0.18));
        raw_snapshot.insert("rsrs_zscore_18_60".to_string(), json!(0.21));
        raw_snapshot.insert("market_profile".to_string(), json!("a_share_core"));
        raw_snapshot.insert("sector_profile".to_string(), json!("bond_etf_peer"));

        assert_eq!(
            etf_cross_section_guard_status("511010.SH", &raw_snapshot, &model),
            Some("cross_section_invalid".to_string())
        );
    }

    #[test]
    fn normalize_integrated_stance_for_modeling_collapses_new_etf_watch_labels() {
        // 2026-04-12 UTC+08: Add a red test for ETF stance normalization, because the
        // live ETF information layer now emits `mixed_watch` and `watchful_positive`
        // while the trained ETF scorecard artifacts still expect one governed watch bucket.
        // Purpose: lock the shared modeling bucket so ETF retraining and runtime scoring
        // do not drift apart when information wording becomes richer.
        assert_eq!(
            normalize_integrated_stance_for_modeling("technical_only"),
            "watchful_context"
        );
        assert_eq!(
            normalize_integrated_stance_for_modeling("mixed_watch"),
            "watchful_context"
        );
        assert_eq!(
            normalize_integrated_stance_for_modeling("watchful_positive"),
            "watchful_context"
        );
        assert_eq!(
            normalize_integrated_stance_for_modeling("cautious"),
            "cautious"
        );
    }

    #[test]
    fn value_matches_bin_accepts_normalized_etf_watch_bucket() {
        // 2026-04-12 UTC+08: Add a red runtime-matching test, because the real ETF
        // failure now happens after governed proxy information reaches the scorecard
        // but the integrated-stance feature still misses the trained bin.
        // Purpose: require the runtime raw snapshot to match the normalized ETF watch
        // bucket instead of downgrading the whole ETF scorecard to feature_incomplete.
        let bin = super::SecurityScorecardModelBin {
            bin_label: "watchful_context".to_string(),
            match_values: vec!["watchful_context".to_string()],
            min_inclusive: None,
            max_exclusive: None,
            woe: Some(0.1),
            logit_contribution: Some(0.2),
            points: 3.0,
            predicted_value: None,
        };

        assert!(value_matches_bin(&json!("watchful_context"), &bin));
        assert!(!value_matches_bin(&json!("mixed_watch"), &bin));
    }

    #[test]
    fn value_matches_bin_accepts_other_bucket_for_unseen_category() {
        // 2026-04-12 UTC+08: Add a red test for categorical fallback matching,
        // because pooled ETF validation still drops to `feature_incomplete` when a
        // holdout sample surfaces a category that the training split never saw.
        // Purpose: require runtime scorecard matching to route unseen categories
        // into a governed `__other__` bucket instead of treating them as missing.
        let bin = super::SecurityScorecardModelBin {
            bin_label: "__other__".to_string(),
            match_values: vec!["__other__".to_string()],
            min_inclusive: None,
            max_exclusive: None,
            woe: Some(0.0),
            logit_contribution: Some(0.0),
            points: 0.0,
            predicted_value: Some(0.15),
        };

        assert!(value_matches_bin(&json!("watchful_context"), &bin));
        assert!(value_matches_bin(&json!("defensive_distribution"), &bin));
    }

    #[test]
    fn value_matches_bin_accepts_boolean_runtime_value_for_string_category_bin() {
        // 2026-04-16 CST: Add a red regression for bool categorical matching,
        // because the governed scorecard model stores `true/false` categories as
        // strings while runtime raw snapshots can still carry JSON booleans.
        // Purpose: prevent approval-ready scorecards from degrading only because
        // one boolean feature crosses the model/runtime boundary in JSON form.
        let bin = super::SecurityScorecardModelBin {
            bin_label: "false".to_string(),
            match_values: vec!["false".to_string()],
            min_inclusive: None,
            max_exclusive: None,
            woe: Some(0.0),
            logit_contribution: Some(0.0),
            points: 0.0,
            predicted_value: None,
        };

        assert!(value_matches_bin(&json!(false), &bin));
        assert!(!value_matches_bin(&json!(true), &bin));
    }
}
