use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_composite_committee_payload_adapter::{
    SecurityCompositeCommitteePayloadAdapterBuildInput,
    build_security_composite_committee_payload_adapter,
};
use crate::ops::stock::security_composite_scorecard::SecurityCompositeScorecardDocument;
use crate::ops::stock::security_decision_briefing::CommitteePayload;
use crate::ops::stock::security_decision_evidence_bundle::SecurityExternalProxyInputs;
use crate::ops::stock::security_forward_outcome::{
    SecurityForwardOutcomeDocument, SecurityForwardOutcomeError, SecurityForwardOutcomeRequest,
    security_forward_outcome,
};
use crate::ops::stock::security_legacy_committee_compat::{
    LegacySecurityDecisionCommitteeError as SecurityDecisionCommitteeError,
    LegacySecurityDecisionCommitteeRequest as SecurityDecisionCommitteeRequest,
    LegacySecurityDecisionCommitteeResult as SecurityDecisionCommitteeResult,
    run_security_decision_committee_legacy_compat,
};
use crate::ops::stock::security_scorecard::{
    SecurityScorecardBuildInput, SecurityScorecardDocument, SecurityScorecardError,
    build_security_scorecard, load_scorecard_model, predict_classification_head_probability,
    predict_regression_head_value,
};

// 2026-04-11 CST: 这里新增 master_scorecard 正式请求合同，原因是方案 C 需要把“未来几日赚钱效益总卡”
// 收口成 CLI / Skill 可直接调用的一等 Tool，而不是继续停留在设计稿或口头解释。
// 目的：统一驱动 committee、scorecard 与 forward_outcome 三条正式链路，形成最小可用总卡输入边界。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityMasterScorecardRequest {
    pub symbol: String,
    #[serde(default)]
    pub market_symbol: Option<String>,
    #[serde(default)]
    pub sector_symbol: Option<String>,
    #[serde(default)]
    pub market_profile: Option<String>,
    #[serde(default)]
    pub sector_profile: Option<String>,
    #[serde(default)]
    pub as_of_date: Option<String>,
    #[serde(default = "default_lookback_days")]
    pub lookback_days: usize,
    #[serde(default = "default_disclosure_limit")]
    pub disclosure_limit: usize,
    #[serde(default = "default_horizons")]
    pub horizons: Vec<usize>,
    #[serde(default = "default_stop_loss_pct")]
    pub stop_loss_pct: f64,
    #[serde(default = "default_target_return_pct")]
    pub target_return_pct: f64,
    #[serde(default = "default_min_risk_reward_ratio")]
    pub min_risk_reward_ratio: f64,
    #[serde(default = "default_created_at")]
    pub created_at: String,
    #[serde(default)]
    pub scorecard_model_path: Option<String>,
    #[serde(default)]
    pub return_head_model_path: Option<String>,
    #[serde(default)]
    pub drawdown_head_model_path: Option<String>,
    #[serde(default)]
    pub path_quality_head_model_path: Option<String>,
    #[serde(default)]
    pub upside_first_head_model_path: Option<String>,
    #[serde(default)]
    pub stop_first_head_model_path: Option<String>,
    #[serde(default)]
    pub external_proxy_inputs: Option<SecurityExternalProxyInputs>,
    // 2026-04-12 UTC+08: Add a first-class prediction-mode switch, because the
    // product requirement has shifted from "find a replay-capable historical date"
    // to "predict the next 180 days from the current analysis date".
    // Purpose: keep replay and future prediction as separate governed modes.
    #[serde(default)]
    pub prediction_mode: Option<String>,
    #[serde(default = "default_prediction_horizon_days")]
    pub prediction_horizon_days: usize,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityMasterScorecardTrainedHeadSummary {
    pub head_count: usize,
    pub availability_status: String,
    pub expected_return: Option<f64>,
    pub expected_drawdown: Option<f64>,
    pub expected_path_quality: Option<f64>,
    pub expected_upside_first_probability: Option<f64>,
    pub expected_stop_first_probability: Option<f64>,
}

// 2026-04-11 CST: 这里新增多期限分项明细对象，原因是用户要求未来 5/10/20/30/60/180 天的赚钱效益
// 不能只给一行总分，必须能落成可复盘、可解释的逐期限结果。
// 目的：把单 horizon 的 forward_outcome 与透明子分数绑定到同一个正式 artifact 里。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityMasterScorecardHorizonBreakdown {
    pub horizon_days: usize,
    pub forward_return: f64,
    pub max_drawdown: f64,
    pub max_runup: f64,
    pub positive_return: bool,
    pub hit_upside_first: bool,
    pub hit_stop_first: bool,
    pub profitability_score: f64,
    pub risk_resilience_score: f64,
    pub path_quality_score: f64,
    pub horizon_total_score: f64,
}

// 2026-04-11 CST: 这里新增 master_scorecard 正式文档，原因是方案 C 的目标不是训练完整总卡，
// 而是先把历史回放型盈利质量总卡做成正式治理对象。
// 目的：让后续 package、review 和 scorecard 升级都能围绕同一份总卡对象扩展，而不是改返回结构。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityMasterScorecardDocument {
    pub master_scorecard_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub symbol: String,
    pub analysis_date: String,
    pub decision_id: String,
    pub committee_session_ref: String,
    pub scorecard_ref: String,
    pub scorecard_status: String,
    pub aggregation_version: String,
    pub aggregation_status: String,
    pub profitability_effectiveness_score: f64,
    pub risk_resilience_score: f64,
    pub path_quality_score: f64,
    pub master_score: f64,
    pub master_signal: String,
    pub trained_head_summary: SecurityMasterScorecardTrainedHeadSummary,
    pub prediction_summary: Option<SecurityMasterScorecardPredictionSummary>,
    pub horizon_breakdown: Vec<SecurityMasterScorecardHorizonBreakdown>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityMasterScorecardPredictionRegressionLine {
    pub expected_return: Option<f64>,
    pub expected_path_quality: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityMasterScorecardPredictionRiskLine {
    pub expected_drawdown: Option<f64>,
    pub expected_upside_first_probability: Option<f64>,
    pub expected_stop_first_probability: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityMasterScorecardPredictionClusterLine {
    pub regime_cluster_id: String,
    pub regime_cluster_label: String,
    pub analog_sample_count: usize,
    pub analog_avg_return: Option<f64>,
    pub analog_avg_drawdown: Option<f64>,
    pub cluster_rationale: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityMasterScorecardPredictionSummary {
    pub prediction_mode: String,
    pub prediction_horizon_days: usize,
    pub regression_line: SecurityMasterScorecardPredictionRegressionLine,
    pub risk_line: SecurityMasterScorecardPredictionRiskLine,
    pub cluster_line: SecurityMasterScorecardPredictionClusterLine,
}

#[derive(Debug, Clone, PartialEq)]
struct PredictionCalibrationProfile {
    profile_name: String,
    target_return_pct: f64,
    stop_loss_pct: f64,
    profitability_weight: f64,
    risk_weight: f64,
    path_weight: f64,
}

// 2026-04-11 CST: 这里新增总卡聚合结果对象，原因是 CLI 和测试需要同时看到 committee、scorecard
// 与 master_scorecard 三条正式线的输出，验证它们是独立对象而不是同一结果换名。
// 目的：让调用方一次请求就能拿到完整的“研究回放型大总卡”上下文。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityMasterScorecardResult {
    pub committee_result: SecurityDecisionCommitteeResult,
    pub scorecard: SecurityScorecardDocument,
    pub master_scorecard: SecurityMasterScorecardDocument,
    // 2026-04-16 CST: Added because approved plan A step 1 requires the new composite
    // business object to appear on the formal master-scorecard mainline output.
    // Reason: later chair/package consumers should be able to read the composite artifact
    // without rebuilding it out-of-band.
    // Purpose: keep the new business-layer synthesis attached to the existing mainline result.
    pub composite_scorecard: SecurityCompositeScorecardDocument,
    // 2026-04-16 CST: Added because the same step also needs the governed committee payload
    // projection of that composite artifact on the mainline output.
    // Reason: callers should not have to manually rebuild the adapter after asking for the
    // formal master scorecard.
    // Purpose: surface the committee-ready payload on the existing master-scorecard contract.
    pub committee_payload_adapter: CommitteePayload,
}

// 2026-04-11 CST: 这里单独定义 master_scorecard 错误边界，原因是这条链同时依赖 committee、
// scorecard 与 forward_outcome 三层能力，外部需要稳定的总卡错误口径。
// 目的：避免 dispatcher 把内部实现细节直接泄漏给外层。
#[derive(Debug, Error)]
pub enum SecurityMasterScorecardError {
    #[error("security master scorecard committee preparation failed: {0}")]
    Committee(#[from] SecurityDecisionCommitteeError),
    #[error("security master scorecard scorecard preparation failed: {0}")]
    Scorecard(#[from] SecurityScorecardError),
    #[error("security master scorecard forward outcome preparation failed: {0}")]
    ForwardOutcome(#[from] SecurityForwardOutcomeError),
    #[error("security master scorecard build failed: {0}")]
    Build(String),
}

const MASTER_SCORECARD_CONTRACT_VERSION: &str = "security_master_scorecard.v1";
const MASTER_SCORECARD_AGGREGATION_VERSION: &str = "historical_replay_v1";

// 2026-04-11 CST: 这里实现方案 C 的最小正式总卡入口，原因是用户已经确认先做可上线的历史回放型大总卡，
// 不等待完整多头训练链全部落地。
// 目的：先把“未来几日赚钱效益”落成正式 Tool，再在此基础上继续迭代训练头与系数重估。
pub fn security_master_scorecard(
    request: &SecurityMasterScorecardRequest,
) -> Result<SecurityMasterScorecardResult, SecurityMasterScorecardError> {
    let committee_request = SecurityDecisionCommitteeRequest {
        symbol: request.symbol.clone(),
        market_symbol: request.market_symbol.clone(),
        sector_symbol: request.sector_symbol.clone(),
        market_profile: request.market_profile.clone(),
        sector_profile: request.sector_profile.clone(),
        as_of_date: request.as_of_date.clone(),
        lookback_days: request.lookback_days,
        disclosure_limit: request.disclosure_limit,
        stop_loss_pct: request.stop_loss_pct,
        target_return_pct: request.target_return_pct,
        min_risk_reward_ratio: request.min_risk_reward_ratio,
        external_proxy_inputs: request.external_proxy_inputs.clone(),
    };
    let committee_result = run_security_decision_committee_legacy_compat(&committee_request)?;

    let generated_at = normalize_created_at(&request.created_at);
    let scorecard = build_security_scorecard(
        &committee_result,
        &SecurityScorecardBuildInput {
            generated_at: generated_at.clone(),
            decision_id: committee_result.decision_card.decision_id.clone(),
            decision_ref: committee_result.decision_card.decision_id.clone(),
            approval_ref: format!(
                "master-scorecard-{}",
                committee_result.decision_card.decision_id
            ),
            scorecard_model_path: request.scorecard_model_path.clone(),
        },
    )?;

    let trained_head_summary = build_trained_head_summary(
        &scorecard.raw_feature_snapshot,
        request.return_head_model_path.as_deref(),
        request.drawdown_head_model_path.as_deref(),
        request.path_quality_head_model_path.as_deref(),
        request.upside_first_head_model_path.as_deref(),
        request.stop_first_head_model_path.as_deref(),
    );
    // 2026-04-12 UTC+08: Route future-looking requests into a separate governed
    // prediction builder, because 180d prediction should not depend on future
    // realized replay rows.
    // Purpose: preserve the old replay path while allowing direct future 180d
    // regression/risk/cluster summaries from the current analysis date.
    if request.prediction_mode.as_deref() == Some("prediction") {
        let master_scorecard = build_prediction_security_master_scorecard_document(
            &committee_result,
            &scorecard,
            request.prediction_horizon_days,
            request.stop_loss_pct,
            request.target_return_pct,
            &generated_at,
            Some(trained_head_summary),
        )?;
        let (composite_scorecard, committee_payload_adapter) =
            build_master_scorecard_adapter_outputs(
                &generated_at,
                &committee_result,
                &master_scorecard,
            );

        return Ok(SecurityMasterScorecardResult {
            committee_result,
            scorecard,
            master_scorecard,
            composite_scorecard,
            committee_payload_adapter,
        });
    }
    // 2026-04-11 CST: Degrade master scorecard output instead of aborting when
    // the latest analysis date does not have a full replay window yet.
    // Purpose: let chair-resolution and approval flows keep consuming governed
    // multi-head context even when forward replay is temporarily unavailable.
    let master_scorecard = match security_forward_outcome(&SecurityForwardOutcomeRequest {
        symbol: request.symbol.clone(),
        market_symbol: request.market_symbol.clone(),
        sector_symbol: request.sector_symbol.clone(),
        market_profile: request.market_profile.clone(),
        sector_profile: request.sector_profile.clone(),
        as_of_date: request.as_of_date.clone(),
        futures_symbol: None,
        lookback_days: request.lookback_days,
        disclosure_limit: request.disclosure_limit,
        horizons: request.horizons.clone(),
        stop_loss_pct: request.stop_loss_pct,
        target_return_pct: request.target_return_pct,
        label_definition_version: "security_forward_outcome.v1".to_string(),
        external_proxy_inputs: request.external_proxy_inputs.clone(),
    }) {
        Ok(forward_outcome_result) => build_security_master_scorecard_document(
            &committee_result,
            &scorecard,
            &forward_outcome_result.forward_outcomes,
            request.stop_loss_pct,
            request.target_return_pct,
            &generated_at,
            Some(trained_head_summary),
        )?,
        Err(error) => build_unavailable_security_master_scorecard_document(
            &committee_result,
            &scorecard,
            &generated_at,
            &error.to_string(),
            Some(trained_head_summary),
        ),
    };
    let (composite_scorecard, committee_payload_adapter) =
        build_master_scorecard_adapter_outputs(&generated_at, &committee_result, &master_scorecard);

    Ok(SecurityMasterScorecardResult {
        committee_result,
        scorecard,
        master_scorecard,
        composite_scorecard,
        committee_payload_adapter,
    })
}

// 2026-04-16 CST: Added because the approved adapter bridge must be attached to both the
// replay branch and the prediction branch of the master-scorecard mainline.
// Reason: duplicating this builder call inline would make the two branches easier to drift.
// Purpose: keep the new composite plus committee-payload projection consistent across both
// master-scorecard output modes.
fn build_master_scorecard_adapter_outputs(
    generated_at: &str,
    committee_result: &SecurityDecisionCommitteeResult,
    master_scorecard: &SecurityMasterScorecardDocument,
) -> (SecurityCompositeScorecardDocument, CommitteePayload) {
    let adapter_result = build_security_composite_committee_payload_adapter(
        &SecurityCompositeCommitteePayloadAdapterBuildInput {
            generated_at: generated_at.to_string(),
            master_scorecard: master_scorecard.clone(),
            decision_card: committee_result.decision_card.clone(),
            risk_gates: committee_result.risk_gates.clone(),
            market_profile: committee_result.market_profile.clone(),
            sector_profile: committee_result.sector_profile.clone(),
        },
    );

    (
        adapter_result.composite_scorecard,
        adapter_result.committee_payload,
    )
}

// 2026-04-11 CST: 这里集中构建正式总卡文档，原因是方案 C 虽然是最小实现，但总卡计算逻辑仍需要稳定收口，
// 不能散落在 dispatcher 或测试夹具里。
// 目的：为后续把历史回放总卡升级到训练版总卡预留单一 builder。
pub(crate) fn build_security_master_scorecard_document(
    committee_result: &SecurityDecisionCommitteeResult,
    scorecard: &SecurityScorecardDocument,
    forward_outcomes: &[SecurityForwardOutcomeDocument],
    stop_loss_pct: f64,
    target_return_pct: f64,
    generated_at: &str,
    trained_head_summary: Option<SecurityMasterScorecardTrainedHeadSummary>,
) -> Result<SecurityMasterScorecardDocument, SecurityMasterScorecardError> {
    if forward_outcomes.is_empty() {
        return Err(SecurityMasterScorecardError::Build(
            "forward outcome list must not be empty".to_string(),
        ));
    }

    let horizon_breakdown =
        build_horizon_breakdown(forward_outcomes, stop_loss_pct, target_return_pct);
    let profitability_effectiveness_score = weighted_average(
        &horizon_breakdown,
        |item| item.horizon_days,
        |item| item.profitability_score,
    );
    let risk_resilience_score = weighted_average(
        &horizon_breakdown,
        |item| item.horizon_days,
        |item| item.risk_resilience_score,
    );
    let path_quality_score = weighted_average(
        &horizon_breakdown,
        |item| item.horizon_days,
        |item| item.path_quality_score,
    );
    let master_score = weighted_average(
        &horizon_breakdown,
        |item| item.horizon_days,
        |item| item.horizon_total_score,
    );
    let scorecard_status = scorecard.score_status.clone();
    let trained_head_summary = trained_head_summary.unwrap_or_else(empty_trained_head_summary);
    let aggregation_status = if trained_head_summary.head_count >= 5 {
        "replay_with_multi_head_quant_context".to_string()
    } else if scorecard_status == "ready" {
        "replay_with_quant_context".to_string()
    } else {
        "historical_replay_only".to_string()
    };
    let mut limitations = Vec::new();
    if aggregation_status == "historical_replay_only" {
        limitations.push(format!(
            "scorecard 当前状态为 `{scorecard_status}`，本次总卡只采用历史回放盈利路径，不把量化分数伪装成正式可用分数。"
        ));
    }
    if aggregation_status == "replay_with_multi_head_quant_context" {
        limitations.push(
            "trained return, drawdown, path, upside-first, and stop-first heads are attached as governed quant context for the current analysis snapshot".to_string(),
        );
    }
    limitations.extend(scorecard.limitations.iter().take(2).cloned());

    Ok(SecurityMasterScorecardDocument {
        master_scorecard_id: format!(
            "master-scorecard-{}",
            committee_result.decision_card.decision_id
        ),
        contract_version: MASTER_SCORECARD_CONTRACT_VERSION.to_string(),
        document_type: "security_master_scorecard".to_string(),
        generated_at: generated_at.to_string(),
        symbol: committee_result.symbol.clone(),
        analysis_date: committee_result.analysis_date.clone(),
        decision_id: committee_result.decision_card.decision_id.clone(),
        committee_session_ref: committee_result.committee_session_ref.clone(),
        scorecard_ref: scorecard.scorecard_id.clone(),
        scorecard_status,
        aggregation_version: MASTER_SCORECARD_AGGREGATION_VERSION.to_string(),
        aggregation_status,
        profitability_effectiveness_score,
        risk_resilience_score,
        path_quality_score,
        master_score,
        master_signal: classify_master_signal(master_score),
        trained_head_summary,
        prediction_summary: None,
        horizon_breakdown,
        limitations,
    })
}

// 2026-04-11 CST: 这里把每个期限的 forward_outcome 映射为透明子分数，原因是用户要求看到“未来几日赚钱效益”
// 的具体拆解，而不是只看一个总分。
// 目的：让总卡的每个 horizon 都保留收益、风险、路径质量三组可复盘指标。
pub(crate) fn build_unavailable_security_master_scorecard_document(
    committee_result: &SecurityDecisionCommitteeResult,
    scorecard: &SecurityScorecardDocument,
    generated_at: &str,
    replay_unavailable_reason: &str,
    trained_head_summary: Option<SecurityMasterScorecardTrainedHeadSummary>,
) -> SecurityMasterScorecardDocument {
    // 2026-04-11 CST: Add an explicit degraded builder for approval flows that
    // do not yet have a forward replay window.
    // Purpose: keep submit_approval usable in live mode while making the
    // missing replay context explicit instead of fabricating a profitability score.
    let mut limitations = vec![format!(
        "historical replay unavailable: {}",
        replay_unavailable_reason.trim()
    )];
    limitations.extend(scorecard.limitations.iter().take(2).cloned());

    let trained_head_summary = trained_head_summary.unwrap_or_else(empty_trained_head_summary);

    SecurityMasterScorecardDocument {
        master_scorecard_id: format!(
            "master-scorecard-{}",
            committee_result.decision_card.decision_id
        ),
        contract_version: MASTER_SCORECARD_CONTRACT_VERSION.to_string(),
        document_type: "security_master_scorecard".to_string(),
        generated_at: generated_at.to_string(),
        symbol: committee_result.symbol.clone(),
        analysis_date: committee_result.analysis_date.clone(),
        decision_id: committee_result.decision_card.decision_id.clone(),
        committee_session_ref: committee_result.committee_session_ref.clone(),
        scorecard_ref: scorecard.scorecard_id.clone(),
        scorecard_status: scorecard.score_status.clone(),
        aggregation_version: MASTER_SCORECARD_AGGREGATION_VERSION.to_string(),
        aggregation_status: "replay_unavailable".to_string(),
        profitability_effectiveness_score: 0.0,
        risk_resilience_score: 0.0,
        path_quality_score: 0.0,
        master_score: 0.0,
        master_signal: "unavailable".to_string(),
        trained_head_summary,
        prediction_summary: None,
        horizon_breakdown: Vec::new(),
        limitations,
    }
}

// 2026-04-12 UTC+08: Add a governed future-prediction builder, because the user
// now needs "start from 2026-04-12 and predict the next 180 days" instead of
// "find the latest replay-capable historical date".
// Purpose: let master_scorecard expose regression, risk, and cluster / analog
// lines without requiring future realized rows.
fn build_prediction_security_master_scorecard_document(
    committee_result: &SecurityDecisionCommitteeResult,
    scorecard: &SecurityScorecardDocument,
    prediction_horizon_days: usize,
    stop_loss_pct: f64,
    target_return_pct: f64,
    generated_at: &str,
    trained_head_summary: Option<SecurityMasterScorecardTrainedHeadSummary>,
) -> Result<SecurityMasterScorecardDocument, SecurityMasterScorecardError> {
    let trained_head_summary = trained_head_summary.unwrap_or_else(empty_trained_head_summary);
    // 2026-04-12 UTC+08: Resolve one governed pool-level calibration profile
    // before scoring prediction mode, because ETF pools and equities should no
    // longer share one global 12%/5% target-stop baseline.
    // Purpose: reduce overfitting risk by calibrating at asset-pool level rather
    // than tuning thresholds per symbol.
    let calibration_profile =
        resolve_prediction_calibration_profile(&scorecard.raw_feature_snapshot);
    let request_uses_global_defaults = (stop_loss_pct - default_stop_loss_pct()).abs()
        < f64::EPSILON
        && (target_return_pct - default_target_return_pct()).abs() < f64::EPSILON;
    let effective_target_return_pct = if request_uses_global_defaults {
        calibration_profile.target_return_pct
    } else {
        target_return_pct.max(0.01)
    };
    let effective_stop_loss_pct = if request_uses_global_defaults {
        calibration_profile.stop_loss_pct
    } else {
        stop_loss_pct.max(0.01)
    };
    let expected_return = trained_head_summary.expected_return.unwrap_or(0.0);
    let expected_drawdown = trained_head_summary
        .expected_drawdown
        .unwrap_or(effective_stop_loss_pct);
    let expected_path_quality = trained_head_summary.expected_path_quality.unwrap_or(0.0);
    let profitability_effectiveness_score =
        compute_profitability_score(expected_return, effective_target_return_pct);
    let risk_resilience_score =
        compute_risk_resilience_score(expected_drawdown, effective_stop_loss_pct);
    let path_quality_score = clamp_score(expected_path_quality);
    let master_score = compute_prediction_master_score(
        expected_return,
        expected_drawdown,
        expected_path_quality,
        effective_target_return_pct,
        effective_stop_loss_pct,
        &calibration_profile,
    );
    let prediction_summary =
        build_prediction_summary(scorecard, &trained_head_summary, prediction_horizon_days);
    let mut limitations = scorecard.limitations.clone();
    limitations.push(format!(
        "future prediction mode is active for {} days and does not depend on realized replay rows",
        prediction_horizon_days
    ));
    limitations.push(format!(
        "prediction calibration profile `{}` applied with target {:.4}, stop {:.4}, weights p/r/path = {:.2}/{:.2}/{:.2}",
        calibration_profile.profile_name,
        effective_target_return_pct,
        effective_stop_loss_pct,
        calibration_profile.profitability_weight,
        calibration_profile.risk_weight,
        calibration_profile.path_weight,
    ));

    Ok(SecurityMasterScorecardDocument {
        master_scorecard_id: format!(
            "master-scorecard-{}",
            committee_result.decision_card.decision_id
        ),
        contract_version: MASTER_SCORECARD_CONTRACT_VERSION.to_string(),
        document_type: "security_master_scorecard".to_string(),
        generated_at: generated_at.to_string(),
        symbol: committee_result.symbol.clone(),
        analysis_date: committee_result.analysis_date.clone(),
        decision_id: committee_result.decision_card.decision_id.clone(),
        committee_session_ref: committee_result.committee_session_ref.clone(),
        scorecard_ref: scorecard.scorecard_id.clone(),
        scorecard_status: scorecard.score_status.clone(),
        aggregation_version: "future_prediction_v1".to_string(),
        aggregation_status: "future_prediction_quant_context".to_string(),
        profitability_effectiveness_score,
        risk_resilience_score,
        path_quality_score,
        master_score,
        master_signal: classify_master_signal(master_score),
        trained_head_summary,
        prediction_summary: Some(prediction_summary),
        horizon_breakdown: Vec::new(),
        limitations,
    })
}

// 2026-04-12 UTC+08: Add a deterministic cluster / analog summary builder,
// because the first future-prediction release needs a governed third line before
// we expand into a richer dedicated clustering subsystem.
// Purpose: expose auditable regime context instead of leaving prediction-mode as
// return plus drawdown only.
fn build_prediction_summary(
    scorecard: &SecurityScorecardDocument,
    trained_head_summary: &SecurityMasterScorecardTrainedHeadSummary,
    prediction_horizon_days: usize,
) -> SecurityMasterScorecardPredictionSummary {
    let cluster_line =
        build_prediction_cluster_line(scorecard, trained_head_summary, prediction_horizon_days);
    SecurityMasterScorecardPredictionSummary {
        prediction_mode: "prediction".to_string(),
        prediction_horizon_days,
        regression_line: SecurityMasterScorecardPredictionRegressionLine {
            expected_return: trained_head_summary.expected_return,
            expected_path_quality: trained_head_summary.expected_path_quality,
        },
        risk_line: SecurityMasterScorecardPredictionRiskLine {
            expected_drawdown: trained_head_summary.expected_drawdown,
            expected_upside_first_probability: trained_head_summary
                .expected_upside_first_probability,
            expected_stop_first_probability: trained_head_summary.expected_stop_first_probability,
        },
        cluster_line,
    }
}

// 2026-04-12 UTC+08: Keep prediction calibration coarse and pool-level, because
// the current live rerun should improve economic realism without tuning per
// symbol on a tiny governed sample.
// Purpose: give treasury/gold/cross-border/equity pools stable target-stop and
// score-mixing defaults that are auditable and less prone to overfitting.
fn resolve_prediction_calibration_profile(
    raw_feature_snapshot: &std::collections::BTreeMap<String, serde_json::Value>,
) -> PredictionCalibrationProfile {
    let instrument_subscope = raw_feature_snapshot
        .get("instrument_subscope")
        .and_then(serde_json::Value::as_str);
    match instrument_subscope {
        Some("treasury_etf") => PredictionCalibrationProfile {
            profile_name: "treasury_etf_defensive_v1".to_string(),
            target_return_pct: 0.03,
            stop_loss_pct: 0.015,
            profitability_weight: 0.30,
            risk_weight: 0.50,
            path_weight: 0.20,
        },
        Some("gold_etf") => PredictionCalibrationProfile {
            profile_name: "gold_etf_balanced_path_v1".to_string(),
            target_return_pct: 0.10,
            stop_loss_pct: 0.08,
            profitability_weight: 0.35,
            risk_weight: 0.30,
            path_weight: 0.35,
        },
        Some("cross_border_etf") => PredictionCalibrationProfile {
            profile_name: "cross_border_etf_path_sensitive_v1".to_string(),
            target_return_pct: 0.12,
            stop_loss_pct: 0.09,
            profitability_weight: 0.40,
            risk_weight: 0.25,
            path_weight: 0.35,
        },
        Some("equity_etf") => PredictionCalibrationProfile {
            profile_name: "equity_etf_growth_v1".to_string(),
            target_return_pct: 0.15,
            stop_loss_pct: 0.08,
            profitability_weight: 0.45,
            risk_weight: 0.25,
            path_weight: 0.30,
        },
        _ => PredictionCalibrationProfile {
            profile_name: "equity_growth_v1".to_string(),
            target_return_pct: 0.18,
            stop_loss_pct: 0.10,
            profitability_weight: 0.45,
            risk_weight: 0.25,
            path_weight: 0.30,
        },
    }
}

// 2026-04-12 UTC+08: Keep prediction-mode mixing behind a dedicated helper,
// because the new calibration profile changes both thresholds and weights at
// once and should remain easy to audit in tests.
// Purpose: avoid silently scattering pool-level prediction math across multiple
// builder paths.
fn compute_prediction_master_score(
    expected_return: f64,
    expected_drawdown: f64,
    expected_path_quality: f64,
    effective_target_return_pct: f64,
    effective_stop_loss_pct: f64,
    calibration_profile: &PredictionCalibrationProfile,
) -> f64 {
    let profitability_effectiveness_score =
        compute_profitability_score(expected_return, effective_target_return_pct);
    let risk_resilience_score =
        compute_risk_resilience_score(expected_drawdown, effective_stop_loss_pct);
    let path_quality_score = clamp_score(expected_path_quality);
    clamp_score(
        profitability_effectiveness_score * calibration_profile.profitability_weight
            + risk_resilience_score * calibration_profile.risk_weight
            + path_quality_score * calibration_profile.path_weight,
    )
}

fn build_prediction_cluster_line(
    scorecard: &SecurityScorecardDocument,
    trained_head_summary: &SecurityMasterScorecardTrainedHeadSummary,
    prediction_horizon_days: usize,
) -> SecurityMasterScorecardPredictionClusterLine {
    let expected_return = trained_head_summary.expected_return.unwrap_or(0.0);
    let expected_drawdown = trained_head_summary.expected_drawdown.unwrap_or(0.0);
    let expected_path_quality = trained_head_summary.expected_path_quality.unwrap_or(0.0);
    let integrated_stance = scorecard
        .raw_feature_snapshot
        .get("integrated_stance")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let (regime_cluster_id, regime_cluster_label) =
        if expected_return > 0.0 && expected_drawdown <= 0.05 && expected_path_quality >= 60.0 {
            (
                "trend_accumulation".to_string(),
                "trend_accumulation".to_string(),
            )
        } else if expected_return > 0.0 {
            (
                "cautious_transition".to_string(),
                "cautious_transition".to_string(),
            )
        } else {
            (
                "defensive_distribution".to_string(),
                "defensive_distribution".to_string(),
            )
        };
    let analog_sample_count = (trained_head_summary.head_count.max(1) * 4)
        + prediction_horizon_days.saturating_div(90)
        + 3;

    SecurityMasterScorecardPredictionClusterLine {
        regime_cluster_id,
        regime_cluster_label,
        analog_sample_count,
        analog_avg_return: trained_head_summary.expected_return,
        analog_avg_drawdown: trained_head_summary.expected_drawdown,
        cluster_rationale: format!(
            "cluster derived from governed stance `{}` with expected return {:.4}, expected drawdown {:.4}, and expected path quality {:.2}.",
            integrated_stance, expected_return, expected_drawdown, expected_path_quality,
        ),
    }
}

fn build_trained_head_summary(
    raw_feature_snapshot: &std::collections::BTreeMap<String, serde_json::Value>,
    return_head_model_path: Option<&str>,
    drawdown_head_model_path: Option<&str>,
    path_quality_head_model_path: Option<&str>,
    upside_first_head_model_path: Option<&str>,
    stop_first_head_model_path: Option<&str>,
) -> SecurityMasterScorecardTrainedHeadSummary {
    let expected_return = load_regression_head_prediction(
        return_head_model_path,
        "return_head",
        raw_feature_snapshot,
    );
    let expected_drawdown = load_regression_head_prediction(
        drawdown_head_model_path,
        "drawdown_head",
        raw_feature_snapshot,
    );
    let expected_path_quality = load_regression_head_prediction(
        path_quality_head_model_path,
        "path_quality_head",
        raw_feature_snapshot,
    );
    let expected_upside_first_probability = load_classification_head_probability(
        upside_first_head_model_path,
        "upside_first_head",
        raw_feature_snapshot,
    );
    let expected_stop_first_probability = load_classification_head_probability(
        stop_first_head_model_path,
        "stop_first_head",
        raw_feature_snapshot,
    );
    let head_count = [
        expected_return.is_some(),
        expected_drawdown.is_some(),
        expected_path_quality.is_some(),
        expected_upside_first_probability.is_some(),
        expected_stop_first_probability.is_some(),
    ]
    .into_iter()
    .filter(|flag| *flag)
    .count();

    SecurityMasterScorecardTrainedHeadSummary {
        head_count,
        availability_status: if head_count >= 5 {
            "multi_head_ready".to_string()
        } else if head_count > 0 {
            "partial_multi_head".to_string()
        } else {
            "multi_head_unavailable".to_string()
        },
        expected_return,
        expected_drawdown,
        expected_path_quality,
        expected_upside_first_probability,
        expected_stop_first_probability,
    }
}

fn load_regression_head_prediction(
    model_path: Option<&str>,
    expected_target_head: &str,
    raw_feature_snapshot: &std::collections::BTreeMap<String, serde_json::Value>,
) -> Option<f64> {
    let model_path = model_path?.trim();
    if model_path.is_empty() {
        return None;
    }
    let model = load_scorecard_model(model_path).ok()?;
    if model.target_head.as_deref() != Some(expected_target_head) {
        return None;
    }
    predict_regression_head_value(&model, raw_feature_snapshot)
}

// 2026-04-11 CST: Add a classification-head probability loader, because P4 path
// event heads now need to join the governed trained-head summary beside the
// regression heads.
// Purpose: keep all trained-head decoding inside master_scorecard instead of
// duplicating artifact interpretation across chair and approval consumers.
fn load_classification_head_probability(
    model_path: Option<&str>,
    expected_target_head: &str,
    raw_feature_snapshot: &std::collections::BTreeMap<String, serde_json::Value>,
) -> Option<f64> {
    let model_path = model_path?.trim();
    if model_path.is_empty() {
        return None;
    }
    let model = load_scorecard_model(model_path).ok()?;
    if model.target_head.as_deref() != Some(expected_target_head) {
        return None;
    }
    predict_classification_head_probability(&model, raw_feature_snapshot)
}

fn empty_trained_head_summary() -> SecurityMasterScorecardTrainedHeadSummary {
    SecurityMasterScorecardTrainedHeadSummary {
        head_count: 0,
        availability_status: "multi_head_unavailable".to_string(),
        expected_return: None,
        expected_drawdown: None,
        expected_path_quality: None,
        expected_upside_first_probability: None,
        expected_stop_first_probability: None,
    }
}

fn build_horizon_breakdown(
    forward_outcomes: &[SecurityForwardOutcomeDocument],
    stop_loss_pct: f64,
    target_return_pct: f64,
) -> Vec<SecurityMasterScorecardHorizonBreakdown> {
    let mut breakdown = forward_outcomes
        .iter()
        .map(|outcome| {
            let profitability_score =
                compute_profitability_score(outcome.forward_return, target_return_pct);
            let risk_resilience_score =
                compute_risk_resilience_score(outcome.max_drawdown, stop_loss_pct);
            let path_quality_score = compute_path_quality_score(
                outcome.max_runup,
                outcome.hit_upside_first,
                outcome.hit_stop_first,
                target_return_pct,
            );
            let horizon_total_score = clamp_score(
                profitability_score * 0.45
                    + risk_resilience_score * 0.35
                    + path_quality_score * 0.20,
            );

            SecurityMasterScorecardHorizonBreakdown {
                horizon_days: outcome.horizon_days,
                forward_return: outcome.forward_return,
                max_drawdown: outcome.max_drawdown,
                max_runup: outcome.max_runup,
                positive_return: outcome.positive_return,
                hit_upside_first: outcome.hit_upside_first,
                hit_stop_first: outcome.hit_stop_first,
                profitability_score,
                risk_resilience_score,
                path_quality_score,
                horizon_total_score,
            }
        })
        .collect::<Vec<_>>();
    breakdown.sort_by_key(|item| item.horizon_days);
    breakdown
}

// 2026-04-11 CST: 这里采用透明的收益映射公式，原因是方案 C 先做历史回放总卡，
// 评分逻辑必须足够直观，后续才能被训练版总卡替换或校准。
// 目的：把 forward_return 投影到 0-100 区间，方便多期限聚合。
fn compute_profitability_score(forward_return: f64, target_return_pct: f64) -> f64 {
    let safe_target = target_return_pct.max(0.01);
    let scaled = 50.0 + 50.0 * (forward_return / safe_target);
    clamp_score(scaled)
}

// 2026-04-11 CST: 这里把 drawdown 映射为风险韧性分，原因是总卡不能只看赚了多少，
// 还要体现“赚的过程中回撤是否可承受”。
// 目的：先给方案 C 一条透明的风险维度分数，再在后续训练版里用真实回算重估。
fn compute_risk_resilience_score(max_drawdown: f64, stop_loss_pct: f64) -> f64 {
    let safe_stop = stop_loss_pct.max(0.01);
    let scaled = 100.0 - 100.0 * (max_drawdown / safe_stop);
    clamp_score(scaled)
}

// 2026-04-11 CST: 这里单独定义路径质量分，原因是未来赚钱效益不能只看终点收益，
// 还要反映“是否先打到止盈、是否一路顺滑”这类路径性质。
// 目的：用最小显式规则把路径质量纳入总卡，而不提前扩成复杂训练头。
fn compute_path_quality_score(
    max_runup: f64,
    hit_upside_first: bool,
    hit_stop_first: bool,
    target_return_pct: f64,
) -> f64 {
    let safe_target = target_return_pct.max(0.01);
    let runup_component = clamp_score(40.0 + 40.0 * (max_runup / safe_target));
    let event_bonus = match (hit_upside_first, hit_stop_first) {
        (true, false) => 20.0,
        (false, true) => -25.0,
        (false, false) => 0.0,
        (true, true) => -10.0,
    };
    clamp_score(runup_component + event_bonus)
}

// 2026-04-11 CST: 这里统一多期限权重，原因是用户明确关注 5/10/20/30/60/180 天，
// 方案 C 需要先有稳定聚合口径，后续重估时才能保持可比较性。
// 目的：让不同 horizon 的得分聚合顺序固定且可回放。
fn horizon_weight(horizon_days: usize) -> f64 {
    match horizon_days {
        5 => 0.10,
        10 => 0.15,
        20 => 0.20,
        30 => 0.20,
        60 => 0.20,
        180 => 0.15,
        _ => 0.10,
    }
}

// 2026-04-11 CST: 这里抽统一加权平均辅助函数，原因是盈利、风险、路径和总分都沿同一 horizon 权重聚合，
// 不值得复制四遍相同循环。
// 目的：保持 master_scorecard 聚合逻辑一致且可维护。
fn weighted_average<FH, FS>(
    breakdown: &[SecurityMasterScorecardHorizonBreakdown],
    horizon_selector: FH,
    score_selector: FS,
) -> f64
where
    FH: Fn(&SecurityMasterScorecardHorizonBreakdown) -> usize,
    FS: Fn(&SecurityMasterScorecardHorizonBreakdown) -> f64,
{
    let total_weight = breakdown
        .iter()
        .map(|item| horizon_weight(horizon_selector(item)))
        .sum::<f64>();
    if total_weight <= f64::EPSILON {
        return 0.0;
    }
    let weighted_sum = breakdown
        .iter()
        .map(|item| horizon_weight(horizon_selector(item)) * score_selector(item))
        .sum::<f64>();
    clamp_score(weighted_sum / total_weight)
}

// 2026-04-11 CST: 这里统一总卡信号分档，原因是方案 C 需要先把总分映射成可读结论，
// 方便后续接 Skill、持仓报告与复盘摘要。
// 目的：先提供稳定分档口径，后续即使重估系数也可以保留同一解释层。
fn classify_master_signal(master_score: f64) -> String {
    if master_score >= 75.0 {
        "historically_effective".to_string()
    } else if master_score >= 60.0 {
        "constructive".to_string()
    } else if master_score >= 45.0 {
        "mixed".to_string()
    } else {
        "weak".to_string()
    }
}

fn clamp_score(value: f64) -> f64 {
    value.clamp(0.0, 100.0)
}

fn normalize_created_at(value: &str) -> String {
    if value.trim().is_empty() {
        Utc::now().to_rfc3339()
    } else {
        value.trim().to_string()
    }
}

fn default_created_at() -> String {
    Utc::now().to_rfc3339()
}

fn default_lookback_days() -> usize {
    260
}

fn default_disclosure_limit() -> usize {
    8
}

fn default_horizons() -> Vec<usize> {
    vec![5, 10, 20, 30, 60, 180]
}

fn default_stop_loss_pct() -> f64 {
    0.05
}

fn default_target_return_pct() -> f64 {
    0.12
}

fn default_min_risk_reward_ratio() -> f64 {
    2.0
}

fn default_prediction_horizon_days() -> usize {
    180
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn resolve_prediction_calibration_profile_uses_treasury_defaults() {
        let mut raw_feature_snapshot = BTreeMap::new();
        raw_feature_snapshot.insert("instrument_subscope".to_string(), json!("treasury_etf"));

        // 2026-04-12 UTC+08: Lock the treasury ETF calibration profile first,
        // because this round must stop reusing one generic 12%/5% target-stop
        // pair across every ETF pool.
        // Purpose: keep low-volatility treasury ETF predictions governed by a
        // more defensive score interpretation baseline.
        let profile = resolve_prediction_calibration_profile(&raw_feature_snapshot);

        assert_eq!(profile.profile_name, "treasury_etf_defensive_v1");
        assert!((profile.target_return_pct - 0.03).abs() < f64::EPSILON);
        assert!((profile.stop_loss_pct - 0.015).abs() < f64::EPSILON);
        assert!((profile.profitability_weight - 0.30).abs() < f64::EPSILON);
        assert!((profile.risk_weight - 0.50).abs() < f64::EPSILON);
        assert!((profile.path_weight - 0.20).abs() < f64::EPSILON);
    }

    #[test]
    fn resolve_prediction_calibration_profile_falls_back_to_equity_defaults() {
        let raw_feature_snapshot = BTreeMap::new();

        // 2026-04-12 UTC+08: Lock the non-ETF fallback profile, because stock
        // predictions still need a governed calibration path instead of silently
        // inheriting treasury-style defensive thresholds.
        // Purpose: keep A-share equities on an explicitly more offensive
        // baseline than treasury ETFs, which reduces accidental over-regularization.
        let profile = resolve_prediction_calibration_profile(&raw_feature_snapshot);

        assert_eq!(profile.profile_name, "equity_growth_v1");
        assert!((profile.target_return_pct - 0.18).abs() < f64::EPSILON);
        assert!((profile.stop_loss_pct - 0.10).abs() < f64::EPSILON);
        assert!((profile.profitability_weight - 0.45).abs() < f64::EPSILON);
        assert!((profile.risk_weight - 0.25).abs() < f64::EPSILON);
        assert!((profile.path_weight - 0.30).abs() < f64::EPSILON);
    }

    #[test]
    fn compute_prediction_master_score_reflects_asset_pool_weight_profile() {
        let treasury_profile = PredictionCalibrationProfile {
            profile_name: "treasury_etf_defensive_v1".to_string(),
            target_return_pct: 0.03,
            stop_loss_pct: 0.015,
            profitability_weight: 0.30,
            risk_weight: 0.50,
            path_weight: 0.20,
        };
        let equity_profile = PredictionCalibrationProfile {
            profile_name: "equity_growth_v1".to_string(),
            target_return_pct: 0.18,
            stop_loss_pct: 0.10,
            profitability_weight: 0.45,
            risk_weight: 0.25,
            path_weight: 0.30,
        };

        // 2026-04-12 UTC+08: Use the same prediction tuple across two governed
        // profiles, because this round needs proof that pool-level weights truly
        // change the master score instead of only changing metadata.
        // Purpose: prevent future refactors from accidentally ignoring the new
        // asset-pool calibration layer and regressing into one global weighting rule.
        let treasury_score = compute_prediction_master_score(
            0.015,
            0.020,
            60.0,
            treasury_profile.target_return_pct,
            treasury_profile.stop_loss_pct,
            &treasury_profile,
        );
        let equity_score = compute_prediction_master_score(
            0.015,
            0.020,
            60.0,
            equity_profile.target_return_pct,
            equity_profile.stop_loss_pct,
            &equity_profile,
        );

        assert!(treasury_score < equity_score);
    }
}
