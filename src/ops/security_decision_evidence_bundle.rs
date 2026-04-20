use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use thiserror::Error;

use crate::ops::stock::security_analysis_contextual::SecurityAnalysisContextualResult;
use crate::ops::stock::security_analysis_fullstack::{
    CrossBorderEtfContext, DisclosureContext, EtfContext, FundamentalContext, IndustryContext,
    IntegratedConclusion, SecurityAnalysisFullstackError, SecurityAnalysisFullstackRequest,
    SecurityAnalysisFullstackResult, disclosure_has_abnormal_volatility_notice,
    disclosure_has_annual_report_notice, disclosure_has_buyback_or_increase_notice,
    disclosure_has_dividend_notice, disclosure_has_fund_occupation_notice,
    disclosure_has_inquiry_notice, disclosure_has_litigation_notice,
    disclosure_has_preloss_or_loss_notice, disclosure_has_reduction_notice,
    disclosure_has_refinancing_notice, disclosure_has_risk_warning_notice,
    disclosure_has_termination_notice, disclosure_positive_keyword_count,
    disclosure_risk_keyword_count, security_analysis_fullstack,
};
use crate::ops::stock::security_external_proxy_backfill::resolve_effective_external_proxy_inputs;
use crate::runtime::security_corporate_action_store::SecurityCorporateActionStore;
use crate::runtime::security_disclosure_history_store::SecurityDisclosureHistoryStore;

// 2026-04-14 CST: 这里补回外部代理输入正式合同，原因是 ETF/跨市场代理数据链已经在 backfill/runtime 中落盘，
// 目的：让 committee/snapshot/training 继续消费统一结构，而不是各模块分别自定义一套散乱字段。
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct SecurityExternalProxyInputs {
    #[serde(default)]
    pub yield_curve_proxy_status: Option<String>,
    #[serde(default)]
    pub yield_curve_slope_delta_bp_5d: Option<f64>,
    #[serde(default)]
    pub funding_liquidity_proxy_status: Option<String>,
    #[serde(default)]
    pub funding_liquidity_spread_delta_bp_5d: Option<f64>,
    #[serde(default)]
    pub gold_spot_proxy_status: Option<String>,
    #[serde(default)]
    pub gold_spot_proxy_return_5d: Option<f64>,
    #[serde(default)]
    pub usd_index_proxy_status: Option<String>,
    #[serde(default)]
    pub usd_index_proxy_return_5d: Option<f64>,
    #[serde(default)]
    pub real_rate_proxy_status: Option<String>,
    #[serde(default)]
    pub real_rate_proxy_delta_bp_5d: Option<f64>,
    #[serde(default)]
    pub fx_proxy_status: Option<String>,
    #[serde(default)]
    pub fx_return_5d: Option<f64>,
    #[serde(default)]
    pub overseas_market_proxy_status: Option<String>,
    #[serde(default)]
    pub overseas_market_return_5d: Option<f64>,
    #[serde(default)]
    pub market_session_gap_status: Option<String>,
    #[serde(default)]
    pub market_session_gap_days: Option<f64>,
    #[serde(default)]
    pub etf_fund_flow_proxy_status: Option<String>,
    #[serde(default)]
    pub etf_fund_flow_5d: Option<f64>,
    #[serde(default)]
    pub premium_discount_proxy_status: Option<String>,
    #[serde(default)]
    pub premium_discount_pct: Option<f64>,
    #[serde(default)]
    pub benchmark_relative_strength_status: Option<String>,
    #[serde(default)]
    pub benchmark_relative_return_5d: Option<f64>,
}

// 2026-04-14 CST: 这里补回 ETF 分箱差异化特征族常量，原因是 scorecard runtime 仍要用它判断 ETF 模型是否具备必要特征；
// 目的：先让 ETF 运行时门禁恢复到单一事实源，避免 scorecard 与 training 各自维护不同的 ETF 特征名单。
pub const ETF_DIFFERENTIATING_FEATURES: &[&str] = &[
    "etf_context_status",
    "etf_benchmark_available",
    "etf_asset_scope",
    "etf_scale_available",
    "etf_share_available",
    "etf_premium_discount_rate_pct",
    "etf_structure_risk_count",
    "etf_research_gap_count",
    "premium_discount_proxy_status",
    "premium_discount_pct",
    "etf_fund_flow_proxy_status",
    "etf_fund_flow_5d",
    "benchmark_relative_strength_status",
    "benchmark_relative_return_5d",
    "fx_proxy_status",
    "fx_return_5d",
    "overseas_market_proxy_status",
    "overseas_market_return_5d",
];

// 2026-04-09 CST: 这里新增正式证据包请求合同，原因是 Task 1-2 的 committee 与 snapshot 都不能再直接读取 fullstack 临时结果；
// 目的：把证券研究链冻结成稳定中间层，后续 chair / snapshot / training 都围绕这一层取数，避免语义漂移。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityDecisionEvidenceBundleRequest {
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
    #[serde(default)]
    pub underlying_symbol: Option<String>,
    #[serde(default)]
    pub fx_symbol: Option<String>,
    #[serde(default = "default_lookback_days")]
    pub lookback_days: usize,
    #[serde(default = "default_disclosure_limit")]
    pub disclosure_limit: usize,
    // 2026-04-14 CST: 这里补回可选 external proxy 输入，原因是 ETF/跨市场链路当前会把 dated proxy backfill 与手工覆盖一起合并进证据层；
    // 目的：先保持证据冻结层的向后兼容，避免下游 committee/snapshot 初始化直接因字段缺失而中断。
    #[serde(default)]
    pub external_proxy_inputs: Option<SecurityExternalProxyInputs>,
}

// 2026-04-09 CST: 这里定义证据质量摘要，原因是 committee / snapshot 都需要先判断“证据是否完整”，
// 目的：把多源研究结果压缩成稳定可复用的质量刻度，而不是在每个 Tool 里重复写完整度判断。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityEvidenceQuality {
    pub technical_status: String,
    pub fundamental_status: String,
    pub disclosure_status: String,
    pub overall_status: String,
    pub risk_flags: Vec<String>,
}

// 2026-04-09 CST: 这里定义正式证据包结果，原因是 committee 与 feature_snapshot 都需要共享同一份冻结研究对象，
// 目的：把 analysis_date / data_gaps / evidence_hash 固化下来，避免同一轮分析出现“不同 Tool 看到不同事实”的问题。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityDecisionEvidenceBundleResult {
    pub symbol: String,
    pub analysis_date: String,
    pub technical_context: SecurityAnalysisContextualResult,
    pub fundamental_context: FundamentalContext,
    pub disclosure_context: DisclosureContext,
    // 2026-04-13 CST: 这里把 ETF 专项事实层冻结进正式证据包，原因是 ETF 相关事实不能只存在 fullstack 临时对象里。
    // 目的：让快照、训练和后续投中/投后链条消费同一份 ETF 证据合同。
    pub etf_context: EtfContext,
    // 2026-04-15 CST: Added because cross-border ETF evidence must preserve the
    // underlying-first chain after leaving fullstack.
    // Reason: dropping this object here would make downstream committee/snapshot consumers
    // regress to the old ETF-local interpretation order.
    // Purpose: freeze one reusable cross-border ETF evidence contract for the governed chain.
    pub cross_border_context: CrossBorderEtfContext,
    pub industry_context: IndustryContext,
    pub integrated_conclusion: IntegratedConclusion,
    pub evidence_quality: SecurityEvidenceQuality,
    pub risk_notes: Vec<String>,
    pub data_gaps: Vec<String>,
    pub evidence_hash: String,
    // 2026-04-14 CST: 这里补回证据层持有的有效 external proxy 输入，原因是 ETF 评分和训练需要同一份代理事实进入证据哈希与原子种子；
    // 目的：让 dated proxy/backfill/manual override 在证据冻结后只存在一份有效视图。
    #[serde(default)]
    pub external_proxy_inputs: Option<SecurityExternalProxyInputs>,
}

// 2026-04-17 CST: Added because P0-2 needs one governed disclosure replay summary that can
// override the thinner live disclosure summary when store-backed history exists.
// Reason: the first 40-name P0 rerun showed several event-side features were still collapsing
// because only truncated disclosure context was entering the evidence seed.
// Purpose: centralize the formal disclosure-thickening read path before snapshot and training.
#[derive(Debug, Clone, PartialEq)]
struct GovernedDisclosureSignalSummary {
    // 2026-04-17 CST: Adjusted because the real-data rerun showed raw notice count was a poor
    // event-density proxy once the governed store could return multiple same-day notices.
    // Reason: training only needs one stable cadence feature here, and distinct notice days are
    // materially more informative than a capped pile of same-date rows.
    // Purpose: keep the existing feature name stable while making the underlying signal useful.
    announcement_count: usize,
    disclosure_positive_keyword_count: usize,
    disclosure_risk_keyword_count: usize,
    has_annual_report_notice: bool,
    has_dividend_notice: bool,
    has_buyback_or_increase_notice: bool,
    has_reduction_notice: bool,
    has_refinancing_notice: bool,
    has_inquiry_notice: bool,
    has_litigation_notice: bool,
    has_termination_notice: bool,
    has_abnormal_volatility_notice: bool,
    has_risk_warning_notice: bool,
    has_preloss_or_loss_notice: bool,
    has_fund_occupation_notice: bool,
}

// 2026-04-09 CST: 这里单独定义证据包错误边界，原因是上层 Tool 不应该直接暴露 fullstack 内部错误实现，
// 目的：给 dispatcher 和后续治理层统一错误口径，避免错误文本跟着底层结构变化。
#[derive(Debug, Error)]
pub enum SecurityDecisionEvidenceBundleError {
    #[error("证券投决证据冻结失败: {0}")]
    Fullstack(#[from] SecurityAnalysisFullstackError),
    #[error("证券投决证据代理输入解析失败: {0}")]
    ExternalProxy(String),
}

// 2026-04-09 CST: 这里实现正式证据冻结入口，原因是 Task 1-4 的所有新对象都要基于同一份中间证据层，
// 目的：先把 fullstack 结果冻结成单一正式对象，再往上生长 committee、snapshot 与 training 底座。
pub fn security_decision_evidence_bundle(
    request: &SecurityDecisionEvidenceBundleRequest,
) -> Result<SecurityDecisionEvidenceBundleResult, SecurityDecisionEvidenceBundleError> {
    // 2026-04-14 CST: 这里先把 dated proxy backfill 与请求级 override 合并成有效代理输入，原因是当前 ETF 兼容修补要优先恢复统一事实口径；
    // 目的：即便后续 fullstack 还没显式消费这些字段，证据层也先保留同源可追溯输入，避免继续在更上层散落处理。
    let effective_external_proxy_inputs = resolve_effective_external_proxy_inputs(
        request.symbol.trim(),
        request.as_of_date.as_deref(),
        request.external_proxy_inputs.clone(),
    )
    .map_err(|error| SecurityDecisionEvidenceBundleError::ExternalProxy(error.to_string()))?;
    let fullstack_request = SecurityAnalysisFullstackRequest {
        symbol: request.symbol.clone(),
        market_symbol: request.market_symbol.clone(),
        sector_symbol: request.sector_symbol.clone(),
        market_profile: request.market_profile.clone(),
        sector_profile: request.sector_profile.clone(),
        as_of_date: request.as_of_date.clone(),
        underlying_symbol: request.underlying_symbol.clone(),
        fx_symbol: request.fx_symbol.clone(),
        lookback_days: request.lookback_days,
        disclosure_limit: request.disclosure_limit,
    };
    let analysis = security_analysis_fullstack(&fullstack_request)?;
    Ok(build_evidence_bundle(
        request,
        analysis,
        effective_external_proxy_inputs,
    ))
}

// 2026-04-09 CST: 这里新增证据包到原子特征种子的统一映射，原因是 Task 2 的 snapshot 与后续 scorecard 都需要稳定原子特征，
// 目的：把特征抽取口径收口在证据层，避免 snapshot、scorecard、training 各自重复拼字段。
pub fn build_evidence_bundle_feature_seed(
    bundle: &SecurityDecisionEvidenceBundleResult,
) -> BTreeMap<String, Value> {
    let stock_analysis = &bundle.technical_context.stock_analysis;
    let indicator_snapshot = &stock_analysis.indicator_snapshot;
    let report_metrics = &bundle.fundamental_context.report_metrics;
    let disclosure_signals = build_governed_disclosure_signal_summary(bundle);
    let mut features = BTreeMap::new();
    features.insert(
        "integrated_stance".to_string(),
        Value::String(bundle.integrated_conclusion.stance.clone()),
    );
    features.insert(
        "technical_alignment".to_string(),
        Value::String(
            bundle
                .technical_context
                .contextual_conclusion
                .alignment
                .clone(),
        ),
    );
    features.insert(
        "technical_status".to_string(),
        Value::String(bundle.evidence_quality.technical_status.clone()),
    );
    features.insert(
        "fundamental_status".to_string(),
        Value::String(bundle.fundamental_context.status.clone()),
    );
    features.insert(
        "fundamental_available".to_string(),
        json!(bundle.fundamental_context.status == "available"),
    );
    features.insert(
        "disclosure_status".to_string(),
        Value::String(bundle.disclosure_context.status.clone()),
    );
    features.insert(
        "disclosure_available".to_string(),
        json!(bundle.disclosure_context.status == "available"),
    );
    features.insert(
        "overall_evidence_status".to_string(),
        Value::String(bundle.evidence_quality.overall_status.clone()),
    );
    features.insert(
        "subject_asset_class".to_string(),
        Value::String(classify_asset_class(bundle).to_string()),
    );
    features.insert("data_gap_count".to_string(), json!(bundle.data_gaps.len()));
    features.insert(
        "risk_note_count".to_string(),
        json!(bundle.risk_notes.len()),
    );
    features.insert(
        "analysis_date".to_string(),
        Value::String(bundle.analysis_date.clone()),
    );
    // 2026-04-15 CST: Added because cross-border ETF downstream consumers now need
    // the underlying-first governed object on the canonical feature seed.
    // Reason: only freezing generic ETF facts leaves snapshot and scorecard blind to
    // whether underlying, FX, and premium were aligned.
    // Purpose: expose the minimum cross-border ETF chain signals on one stable raw surface.
    features.insert(
        "cross_border_context_status".to_string(),
        Value::String(bundle.cross_border_context.status.clone()),
    );
    features.insert(
        "cross_border_analysis_method".to_string(),
        Value::String(bundle.cross_border_context.analysis_method.clone()),
    );
    insert_optional_string_feature(
        &mut features,
        "cross_border_underlying_symbol",
        bundle.cross_border_context.underlying_market.symbol.clone(),
    );
    insert_optional_string_feature(
        &mut features,
        "cross_border_underlying_bias",
        bundle.cross_border_context.underlying_market.bias.clone(),
    );
    insert_optional_string_feature(
        &mut features,
        "cross_border_underlying_confidence",
        bundle
            .cross_border_context
            .underlying_market
            .confidence
            .clone(),
    );
    insert_optional_string_feature(
        &mut features,
        "cross_border_fx_symbol",
        bundle.cross_border_context.fx_market.symbol.clone(),
    );
    insert_optional_string_feature(
        &mut features,
        "cross_border_fx_bias",
        bundle.cross_border_context.fx_market.bias.clone(),
    );
    insert_optional_string_feature(
        &mut features,
        "cross_border_fx_confidence",
        bundle.cross_border_context.fx_market.confidence.clone(),
    );
    features.insert(
        "cross_border_premium_verdict".to_string(),
        Value::String(
            bundle
                .cross_border_context
                .premium_assessment
                .verdict
                .clone(),
        ),
    );
    features.insert(
        "cross_border_resonance_verdict".to_string(),
        Value::String(bundle.cross_border_context.resonance_verdict.clone()),
    );
    // 2026-04-14 CST: 这里把 external proxy 核心状态继续冻结进种子，原因是 ETF/跨市场训练与运行时打分都要识别这些代理特征是否存在；
    // 目的：先补最小可用特征入口，让 ETF scorecard 门禁重新有统一输入，而不是继续引用缺失字段。
    insert_optional_string_feature(
        &mut features,
        "yield_curve_proxy_status",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.yield_curve_proxy_status.clone()),
    );
    insert_optional_numeric_feature(
        &mut features,
        "yield_curve_slope_delta_bp_5d",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.yield_curve_slope_delta_bp_5d),
    );
    // 2026-04-15 CST: Added because historical treasury/gold ETF proxy hydration was still
    // disappearing before snapshot/scorecard consumers could read it.
    // Reason: only a subset of external proxy fields had been projected into the unified feature seed.
    // Purpose: make all governed proxy families share one canonical raw feature surface.
    insert_optional_string_feature(
        &mut features,
        "funding_liquidity_proxy_status",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.funding_liquidity_proxy_status.clone()),
    );
    insert_optional_numeric_feature(
        &mut features,
        "funding_liquidity_spread_delta_bp_5d",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.funding_liquidity_spread_delta_bp_5d),
    );
    insert_optional_string_feature(
        &mut features,
        "gold_spot_proxy_status",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.gold_spot_proxy_status.clone()),
    );
    insert_optional_numeric_feature(
        &mut features,
        "gold_spot_proxy_return_5d",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.gold_spot_proxy_return_5d),
    );
    insert_optional_string_feature(
        &mut features,
        "usd_index_proxy_status",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.usd_index_proxy_status.clone()),
    );
    insert_optional_numeric_feature(
        &mut features,
        "usd_index_proxy_return_5d",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.usd_index_proxy_return_5d),
    );
    insert_optional_string_feature(
        &mut features,
        "real_rate_proxy_status",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.real_rate_proxy_status.clone()),
    );
    insert_optional_numeric_feature(
        &mut features,
        "real_rate_proxy_delta_bp_5d",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.real_rate_proxy_delta_bp_5d),
    );
    insert_optional_string_feature(
        &mut features,
        "premium_discount_proxy_status",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.premium_discount_proxy_status.clone()),
    );
    insert_optional_numeric_feature(
        &mut features,
        "premium_discount_pct",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.premium_discount_pct),
    );
    insert_optional_string_feature(
        &mut features,
        "etf_fund_flow_proxy_status",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.etf_fund_flow_proxy_status.clone()),
    );
    insert_optional_numeric_feature(
        &mut features,
        "etf_fund_flow_5d",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.etf_fund_flow_5d),
    );
    insert_optional_string_feature(
        &mut features,
        "benchmark_relative_strength_status",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.benchmark_relative_strength_status.clone()),
    );
    insert_optional_numeric_feature(
        &mut features,
        "benchmark_relative_return_5d",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.benchmark_relative_return_5d),
    );
    insert_optional_string_feature(
        &mut features,
        "fx_proxy_status",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.fx_proxy_status.clone()),
    );
    insert_optional_numeric_feature(
        &mut features,
        "fx_return_5d",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.fx_return_5d),
    );
    insert_optional_string_feature(
        &mut features,
        "overseas_market_proxy_status",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.overseas_market_proxy_status.clone()),
    );
    insert_optional_numeric_feature(
        &mut features,
        "overseas_market_return_5d",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.overseas_market_return_5d),
    );
    // 2026-04-15 CST: Added because cross-border ETF proxy hydration still dropped
    // session-gap evidence before grouped snapshot and approval consumers could read it.
    // Reason: market-session-gap fields were present in the governed proxy contract but
    // absent from the canonical feature-seed projection.
    // Purpose: keep cross-market timing evidence on the same raw feature surface as other ETF proxies.
    insert_optional_string_feature(
        &mut features,
        "market_session_gap_status",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.market_session_gap_status.clone()),
    );
    insert_optional_numeric_feature(
        &mut features,
        "market_session_gap_days",
        bundle
            .external_proxy_inputs
            .as_ref()
            .and_then(|inputs| inputs.market_session_gap_days),
    );
    // 2026-04-13 CST: 这里把 ETF 专项事实冻结进证据种子，原因是 ETF 训练与回放后续必须消费结构化基金特征，而不是只看 symbol 前缀。
    // 目的：先收口 ETF 的状态、基准、规模与折溢价口径，为 snapshot / training 预留稳定字段。
    features.insert(
        "etf_context_status".to_string(),
        Value::String(bundle.etf_context.status.clone()),
    );
    features.insert(
        "etf_benchmark_available".to_string(),
        json!(bundle.etf_context.benchmark.is_some()),
    );
    features.insert(
        "etf_asset_scope".to_string(),
        bundle
            .etf_context
            .asset_scope
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    features.insert(
        "etf_scale_available".to_string(),
        json!(bundle.etf_context.latest_scale.is_some()),
    );
    features.insert(
        "etf_share_available".to_string(),
        json!(bundle.etf_context.latest_share.is_some()),
    );
    features.insert(
        "etf_premium_discount_rate_pct".to_string(),
        normalized_numeric_feature(bundle.etf_context.premium_discount_rate_pct),
    );
    features.insert(
        "etf_structure_risk_count".to_string(),
        json!(bundle.etf_context.structure_risk_flags.len()),
    );
    features.insert(
        "etf_research_gap_count".to_string(),
        json!(bundle.etf_context.research_gaps.len()),
    );
    // 2026-04-10 CST: 这里把技术面原子信号正式冻结进证据种子，原因是第一阶段统一评分版已经确认要直接消费技术结构化字段，而不是再解析文案；
    // 目的：让 snapshot、training、scorecard 三条链共用同一批技术事实，避免后续各自拼字段造成口径漂移。
    features.insert(
        "trend_bias".to_string(),
        Value::String(stock_analysis.trend_bias.clone()),
    );
    features.insert(
        "trend_strength".to_string(),
        Value::String(stock_analysis.trend_strength.clone()),
    );
    features.insert(
        "volume_confirmation".to_string(),
        Value::String(stock_analysis.volume_confirmation.clone()),
    );
    features.insert(
        "breakout_signal".to_string(),
        Value::String(stock_analysis.breakout_signal.clone()),
    );
    features.insert(
        "momentum_signal".to_string(),
        Value::String(stock_analysis.momentum_signal.clone()),
    );
    features.insert(
        "divergence_signal".to_string(),
        Value::String(stock_analysis.divergence_signal.clone()),
    );
    features.insert(
        "timing_signal".to_string(),
        Value::String(stock_analysis.timing_signal.clone()),
    );
    features.insert(
        "money_flow_signal".to_string(),
        Value::String(stock_analysis.money_flow_signal.clone()),
    );
    features.insert(
        "mean_reversion_signal".to_string(),
        Value::String(stock_analysis.mean_reversion_signal.clone()),
    );
    features.insert(
        "range_position_signal".to_string(),
        Value::String(stock_analysis.range_position_signal.clone()),
    );
    features.insert(
        "bollinger_position_signal".to_string(),
        Value::String(stock_analysis.bollinger_position_signal.clone()),
    );
    features.insert(
        "bollinger_midline_signal".to_string(),
        Value::String(stock_analysis.bollinger_midline_signal.clone()),
    );
    features.insert(
        "bollinger_bandwidth_signal".to_string(),
        Value::String(stock_analysis.bollinger_bandwidth_signal.clone()),
    );
    features.insert(
        "rsrs_signal".to_string(),
        Value::String(stock_analysis.rsrs_signal.clone()),
    );
    features.insert(
        "volatility_state".to_string(),
        Value::String(stock_analysis.volatility_state.clone()),
    );
    // 2026-04-20 CST: Added because the user explicitly requested a full directional factor
    // inventory before the next Nikkei retraining pass.
    // Purpose: freeze one governed up/down/neutral feature surface now, while keeping the
    // current training contract untouched until the direction-separated target design is approved.
    features.insert(
        "trend_direction_state".to_string(),
        Value::String(self::derive_alignment_direction(stock_analysis.trend_bias.as_str()).to_string()),
    );
    features.insert(
        "trend_direction_strength".to_string(),
        Value::String(
            self::derive_trend_direction_strength(
                stock_analysis.trend_bias.as_str(),
                stock_analysis.trend_strength.as_str(),
            )
            .to_string(),
        ),
    );
    features.insert(
        "volume_direction_state".to_string(),
        Value::String(
            self::derive_volume_direction_state(
                stock_analysis.trend_bias.as_str(),
                stock_analysis.volume_confirmation.as_str(),
            )
            .to_string(),
        ),
    );
    features.insert(
        "breakout_direction".to_string(),
        Value::String(self::derive_breakout_direction(stock_analysis.breakout_signal.as_str()).to_string()),
    );
    features.insert(
        "breakout_stage".to_string(),
        Value::String(self::derive_breakout_stage(stock_analysis.breakout_signal.as_str()).to_string()),
    );
    features.insert(
        "alignment_direction".to_string(),
        Value::String(self::derive_alignment_direction(stock_analysis.trend_bias.as_str()).to_string()),
    );
    features.insert(
        "alignment_consistency".to_string(),
        Value::String(
            self::derive_alignment_consistency(
                bundle
                    .technical_context
                    .contextual_conclusion
                    .alignment
                    .as_str(),
            )
            .to_string(),
        ),
    );
    features.insert(
        "market_direction_regime".to_string(),
        Value::String(
            self::derive_market_direction_regime(
                Some(
                    bundle
                        .technical_context
                        .market_analysis
                        .consultation_conclusion
                        .bias
                        .as_str(),
                ),
                Some(bundle.technical_context.market_analysis.breakout_signal.as_str()),
                Some(bundle.technical_context.market_analysis.momentum_signal.as_str()),
            )
            .to_string(),
        ),
    );
    features.insert(
        "market_volatility_regime".to_string(),
        Value::String(
            self::derive_market_volatility_regime(Some(
                bundle
                    .technical_context
                    .market_analysis
                    .volatility_state
                    .as_str(),
            ))
            .to_string(),
        ),
    );
    features.insert(
        "flow_direction_state".to_string(),
        Value::String(
            self::derive_flow_direction_state(stock_analysis.money_flow_signal.as_str()).to_string(),
        ),
    );
    features.insert(
        "mean_reversion_direction_state".to_string(),
        Value::String(
            self::derive_mean_reversion_direction_state(stock_analysis.mean_reversion_signal.as_str())
                .to_string(),
        ),
    );
    features.insert(
        "range_position_direction_state".to_string(),
        Value::String(
            self::derive_range_position_direction_state(stock_analysis.range_position_signal.as_str())
                .to_string(),
        ),
    );
    features.insert(
        "bollinger_position_direction_state".to_string(),
        Value::String(
            self::derive_bollinger_position_direction_state(
                stock_analysis.bollinger_position_signal.as_str(),
            )
            .to_string(),
        ),
    );
    features.insert(
        "bollinger_midline_direction_state".to_string(),
        Value::String(
            self::derive_bollinger_midline_direction_state(
                stock_analysis.bollinger_midline_signal.as_str(),
            )
            .to_string(),
        ),
    );
    features.insert(
        "rsrs_direction_state".to_string(),
        Value::String(self::derive_rsrs_direction_state(stock_analysis.rsrs_signal.as_str()).to_string()),
    );
    features.insert(
        "divergence_direction_state".to_string(),
        Value::String(
            self::derive_divergence_direction_state(stock_analysis.divergence_signal.as_str()).to_string(),
        ),
    );
    features.insert(
        "timing_direction_state".to_string(),
        Value::String(self::derive_timing_direction_state(stock_analysis.timing_signal.as_str()).to_string()),
    );
    // 2026-04-16 CST: Added because P0 data thickening must expose the governed numeric flow and
    // extension snapshot to downstream sample builders.
    // Reason: the technical layer already computes these values, but the evidence seed previously
    // collapsed them into coarse text-only signals.
    // Purpose: keep snapshot, runtime scorecard, and training on one real numeric feature family.
    features.insert(
        "volume_ratio_20".to_string(),
        json!(indicator_snapshot.volume_ratio_20),
    );
    features.insert("mfi_14".to_string(), json!(indicator_snapshot.mfi_14));
    features.insert("cci_20".to_string(), json!(indicator_snapshot.cci_20));
    features.insert(
        "williams_r_14".to_string(),
        json!(indicator_snapshot.williams_r_14),
    );
    features.insert(
        "boll_width_ratio_20".to_string(),
        json!(indicator_snapshot.boll_width_ratio_20),
    );
    // 2026-04-20 CST: Added because the Nikkei mean-reversion redesign now needs one
    // reusable raw numeric surface before snapshot/runtime/training consume the bucket.
    // Reason: the bucket is no longer based on raw percentage alone, so we must freeze
    // both the MA20 percentage gap and the ATR-normalized distance in the seed.
    // Purpose: keep all downstream consumers on one explainable numeric contract.
    let close_vs_sma20 = derive_ratio_delta(indicator_snapshot.close, indicator_snapshot.sma_20);
    let atr_ratio_14 = derive_atr_ratio_14(indicator_snapshot.close, indicator_snapshot.atr_14);
    let mean_reversion_normalized_distance_20d =
        derive_mean_reversion_normalized_distance_20d(close_vs_sma20, atr_ratio_14);
    // 2026-04-17 CST: Added because the thicker governed technical surface must keep the
    // pre-existing raw snapshot contract alive during migration.
    // Reason: ETF snapshot regressions and runtime scorecard guards still read the legacy
    // `close_vs_sma*`, `rsrs_zscore_18_60`, and key-level gap fields directly.
    // Purpose: widen the feature family without silently dropping the stable aliases that the
    // current StockMind runtime already depends on.
    features.insert(
        "close_vs_sma20".to_string(),
        json!(close_vs_sma20),
    );
    features.insert(
        "close_vs_sma50".to_string(),
        json!(derive_ratio_delta(
            indicator_snapshot.close,
            indicator_snapshot.sma_50,
        )),
    );
    features.insert(
        "close_vs_sma200".to_string(),
        json!(derive_ratio_delta(
            indicator_snapshot.close,
            indicator_snapshot.sma_200,
        )),
    );
    features.insert(
        "macd_histogram".to_string(),
        json!(indicator_snapshot.macd_histogram),
    );
    features.insert("rsi_14".to_string(), json!(indicator_snapshot.rsi_14));
    features.insert(
        "rsi_direction_state".to_string(),
        Value::String(self::derive_rsi_direction_state(indicator_snapshot.rsi_14).to_string()),
    );
    features.insert(
        "rsi_extreme_state".to_string(),
        Value::String(self::derive_rsi_extreme_state(indicator_snapshot.rsi_14).to_string()),
    );
    features.insert(
        "macd_histogram_direction".to_string(),
        Value::String(
            self::derive_macd_histogram_direction(indicator_snapshot.macd_histogram).to_string(),
        ),
    );
    features.insert(
        "rsrs_zscore_18_60".to_string(),
        json!(indicator_snapshot.rsrs_zscore_18_60),
    );
    features.insert("atr_14".to_string(), json!(indicator_snapshot.atr_14));
    features.insert(
        "atr_ratio_14".to_string(),
        json!(atr_ratio_14),
    );
    // 2026-04-20 CST: Added because the approved Nikkei route now reviews and trains
    // mean reversion in ATR-normalized units instead of raw percentage alone.
    // Reason: the user explicitly asked to keep the middle bucket small after the
    // post-2025 volatility regime shift.
    // Purpose: expose one raw numeric field that explains how far price sits from MA20 in ATR units.
    features.insert(
        "mean_reversion_normalized_distance_20d".to_string(),
        json!(mean_reversion_normalized_distance_20d),
    );
    features.insert(
        "support_gap_pct_20".to_string(),
        json!(derive_support_gap_pct_20(
            indicator_snapshot.close,
            indicator_snapshot.support_level_20,
        )),
    );
    features.insert(
        "resistance_gap_pct_20".to_string(),
        json!(derive_resistance_gap_pct_20(
            indicator_snapshot.close,
            indicator_snapshot.resistance_level_20,
        )),
    );
    // 2026-04-10 CST: 这里补基本面结构化字段，原因是 fullstack 已经能拿到财报关键同比与 ROE，但之前证据层没有冻结出来；
    // 目的：让训练和回放都能直接复用“盈利信号 + 三个关键指标”，不再反向钻取 fullstack 对象。
    features.insert(
        "profit_signal".to_string(),
        Value::String(bundle.fundamental_context.profit_signal.clone()),
    );
    // 2026-04-10 CST: 这里把基本面 numeric 特征统一标准化，原因是真实训练已经证明 Option/null 直接透传会破坏训练合同；
    // 目的：从证据层开始冻结稳定 numeric 口径，让 snapshot / training / scorecard 不再各自分叉处理缺失值。
    features.insert(
        "revenue_yoy_pct".to_string(),
        normalized_numeric_feature(report_metrics.revenue_yoy_pct),
    );
    features.insert(
        "net_profit_yoy_pct".to_string(),
        normalized_numeric_feature(report_metrics.net_profit_yoy_pct),
    );
    features.insert(
        "roe_pct".to_string(),
        normalized_numeric_feature(report_metrics.roe_pct),
    );
    // 2026-04-10 CST: 这里补消息面结构化因子，原因是用户要求这轮不再停留在公告 headline，而要把“正向/风险事件”沉成可训练字段；
    // 目的：给评分卡和后续顺丰/平安验证提供稳定的公告事件输入，输出更像“结论/问题点”而不是只说公告可用。
    features.insert(
        "announcement_count".to_string(),
        json!(disclosure_signals.announcement_count),
    );
    features.insert(
        "disclosure_positive_keyword_count".to_string(),
        json!(disclosure_signals.disclosure_positive_keyword_count),
    );
    features.insert(
        "disclosure_risk_keyword_count".to_string(),
        json!(disclosure_signals.disclosure_risk_keyword_count),
    );
    features.insert(
        "has_annual_report_notice".to_string(),
        json!(disclosure_signals.has_annual_report_notice),
    );
    features.insert(
        "has_dividend_notice".to_string(),
        json!(disclosure_signals.has_dividend_notice),
    );
    features.insert(
        "has_buyback_or_increase_notice".to_string(),
        json!(disclosure_signals.has_buyback_or_increase_notice),
    );
    features.insert(
        "has_reduction_notice".to_string(),
        json!(disclosure_signals.has_reduction_notice),
    );
    features.insert(
        "has_inquiry_notice".to_string(),
        json!(disclosure_signals.has_inquiry_notice),
    );
    features.insert(
        "has_litigation_notice".to_string(),
        json!(disclosure_signals.has_litigation_notice),
    );
    features.insert(
        "has_termination_notice".to_string(),
        json!(disclosure_signals.has_termination_notice),
    );
    features.insert(
        "has_risk_warning_notice".to_string(),
        json!(disclosure_signals.has_risk_warning_notice),
    );
    features.insert(
        "has_preloss_or_loss_notice".to_string(),
        json!(disclosure_signals.has_preloss_or_loss_notice),
    );
    // 2026-04-17 CST: Added because disclosure events now need one weighted component surface
    // instead of forcing downstream consumers to reinterpret several sparse booleans on their own.
    // Purpose: freeze one explainable event-scoring slice before the first retraining pass uses it.
    features.insert(
        "hard_risk_score".to_string(),
        json!(derive_hard_risk_score(&disclosure_signals)),
    );
    features.insert(
        "negative_attention_score".to_string(),
        json!(derive_negative_attention_score(&disclosure_signals)),
    );
    features.insert(
        "positive_support_score".to_string(),
        json!(derive_positive_support_score(&disclosure_signals)),
    );
    features.insert(
        "event_net_impact_score".to_string(),
        json!(derive_event_net_impact_score(&disclosure_signals)),
    );
    // 2026-04-16 CST: Added because P0 needs one honest shareholder-return slice without claiming
    // total-return relabeling is already finished.
    // Reason: dividend and buyback notices were already frozen individually, but downstream
    // training still lacked one governed combined bucket for corporate-action style signals.
    // Purpose: expose minimum capital-return and fundamental-quality buckets on the canonical seed.
    features.insert(
        "shareholder_return_status".to_string(),
        Value::String(build_governed_shareholder_return_status(
            bundle,
            &disclosure_signals,
        )),
    );
    let fundamental_quality_bucket = derive_fundamental_quality_bucket(
        &bundle.fundamental_context.profit_signal,
        report_metrics.revenue_yoy_pct,
        report_metrics.net_profit_yoy_pct,
        report_metrics.roe_pct,
    );
    features.insert(
        "fundamental_quality_bucket".to_string(),
        Value::String(fundamental_quality_bucket.clone()),
    );
    // 2026-04-20 CST: Added because Task A splits valuation_status into four reviewable
    // sub-factors before the next Nikkei retraining pass.
    // Purpose: publish the normalized buckets on the raw feature seed so snapshot/training/runtime
    // all read the same plain-language position contract.
    // 2026-04-20 CST: Extended because the approved mean-reversion redesign now bins the
    // ATR-normalized MA20 gap instead of the older raw-percentage / CCI hybrid semantics.
    // Reason: keep the normalized distance and its bucket in the same governed feature seed.
    // Purpose: let replay, runtime, and retraining share one volatility-adjusted contract.
    features.insert(
        "bollinger_position_20d".to_string(),
        Value::String(
            derive_bollinger_position_bucket_20d(stock_analysis.bollinger_position_signal.as_str())
                .to_string(),
        ),
    );
    features.insert(
        "range_position_14d".to_string(),
        Value::String(
            derive_range_position_bucket_14d(stock_analysis.range_position_signal.as_str())
                .to_string(),
        ),
    );
    features.insert(
        "mean_reversion_state_20d".to_string(),
        Value::String(
            derive_mean_reversion_bucket_20d(stock_analysis.mean_reversion_signal.as_str())
                .to_string(),
        ),
    );
    features.insert(
        "mean_reversion_deviation_20d".to_string(),
        Value::String(
            derive_mean_reversion_deviation_bucket_20d(mean_reversion_normalized_distance_20d)
                .to_string(),
        ),
    );
    features.insert(
        "quality_bucket".to_string(),
        Value::String(derive_quality_bucket(&fundamental_quality_bucket).to_string()),
    );
    features
}

// 2026-04-17 CST: Added because the governed disclosure runtime already stores more than the
// shallow live summary that reaches the first P0 artifact.
// Reason: event density and disclosure risk should be replayed from persisted rows when they
// exist, otherwise the model keeps learning from a capped and flatter signal surface.
// Purpose: build one store-backed disclosure summary with a clean fallback to the existing
// disclosure context for empty or unavailable governed stores.
fn build_governed_disclosure_signal_summary(
    bundle: &SecurityDecisionEvidenceBundleResult,
) -> GovernedDisclosureSignalSummary {
    let fallback = build_fallback_disclosure_signal_summary(
        &bundle.disclosure_context,
        bundle.disclosure_context.announcement_count,
    );
    let Ok(store) = SecurityDisclosureHistoryStore::workspace_default() else {
        return fallback;
    };
    let Ok(rows) = store.load_recent_records(&bundle.symbol, Some(&bundle.analysis_date), 64)
    else {
        return fallback;
    };
    if rows.is_empty() {
        return fallback;
    }

    let notices = rows
        .into_iter()
        .map(
            |row| crate::ops::stock::security_analysis_fullstack::DisclosureAnnouncement {
                published_at: row.published_at,
                title: row.title,
                article_code: row.article_code,
                category: row.category,
            },
        )
        .collect::<Vec<_>>();
    let event_window_notices =
        filter_announcements_within_days(&notices, &bundle.analysis_date, 90);
    let risk_window_notices =
        filter_announcements_within_days(&notices, &bundle.analysis_date, 180);
    let shareholder_window_notices =
        filter_announcements_within_days(&notices, &bundle.analysis_date, 365);

    if event_window_notices.is_empty()
        && risk_window_notices.is_empty()
        && shareholder_window_notices.is_empty()
    {
        return fallback;
    }

    GovernedDisclosureSignalSummary {
        // 2026-04-17 CST: Adjusted because governed event density should reflect cadence across
        // days, not just the raw count of headlines that may cluster on one date.
        // Purpose: prevent the 90d announcement feature from collapsing into a near-constant bin.
        announcement_count: count_distinct_announcement_days(&event_window_notices),
        disclosure_positive_keyword_count: disclosure_positive_keyword_count(
            &shareholder_window_notices,
        ),
        disclosure_risk_keyword_count: disclosure_risk_keyword_count(&risk_window_notices),
        has_annual_report_notice: disclosure_has_annual_report_notice(&shareholder_window_notices),
        has_dividend_notice: disclosure_has_dividend_notice(&shareholder_window_notices),
        has_buyback_or_increase_notice: disclosure_has_buyback_or_increase_notice(
            &shareholder_window_notices,
        ),
        has_reduction_notice: disclosure_has_reduction_notice(&risk_window_notices),
        has_refinancing_notice: disclosure_has_refinancing_notice(&risk_window_notices),
        has_inquiry_notice: disclosure_has_inquiry_notice(&risk_window_notices),
        has_litigation_notice: disclosure_has_litigation_notice(&risk_window_notices),
        has_termination_notice: disclosure_has_termination_notice(&risk_window_notices),
        has_abnormal_volatility_notice: disclosure_has_abnormal_volatility_notice(
            &risk_window_notices,
        ),
        has_risk_warning_notice: disclosure_has_risk_warning_notice(&risk_window_notices),
        has_preloss_or_loss_notice: disclosure_has_preloss_or_loss_notice(&risk_window_notices),
        has_fund_occupation_notice: disclosure_has_fund_occupation_notice(&risk_window_notices),
    }
}

// 2026-04-17 CST: Added because the evidence seed still needs deterministic values when
// governed disclosure history is absent or not yet bootstrapped for a symbol/date.
// Purpose: preserve the existing contract while letting the new store-backed path override it.
fn build_fallback_disclosure_signal_summary(
    disclosure_context: &DisclosureContext,
    _announcement_count: usize,
) -> GovernedDisclosureSignalSummary {
    let recent_announcements = &disclosure_context.recent_announcements;
    GovernedDisclosureSignalSummary {
        // 2026-04-17 CST: Adjusted because fallback and governed paths must share the same
        // cadence meaning, otherwise snapshot/training behavior depends on storage availability.
        // Purpose: keep event-density semantics consistent when the governed store is absent.
        announcement_count: count_distinct_announcement_days(recent_announcements),
        disclosure_positive_keyword_count: disclosure_positive_keyword_count(recent_announcements),
        disclosure_risk_keyword_count: disclosure_risk_keyword_count(recent_announcements),
        has_annual_report_notice: disclosure_has_annual_report_notice(recent_announcements),
        has_dividend_notice: disclosure_has_dividend_notice(recent_announcements),
        has_buyback_or_increase_notice: disclosure_has_buyback_or_increase_notice(
            recent_announcements,
        ),
        has_reduction_notice: disclosure_has_reduction_notice(recent_announcements),
        has_refinancing_notice: disclosure_has_refinancing_notice(recent_announcements),
        has_inquiry_notice: disclosure_has_inquiry_notice(recent_announcements),
        has_litigation_notice: disclosure_has_litigation_notice(recent_announcements),
        has_termination_notice: disclosure_has_termination_notice(recent_announcements),
        has_abnormal_volatility_notice: disclosure_has_abnormal_volatility_notice(
            recent_announcements,
        ),
        has_risk_warning_notice: disclosure_has_risk_warning_notice(recent_announcements),
        has_preloss_or_loss_notice: disclosure_has_preloss_or_loss_notice(recent_announcements),
        has_fund_occupation_notice: disclosure_has_fund_occupation_notice(recent_announcements),
    }
}

// 2026-04-17 CST: Added because shareholder-return state should now prefer the formal
// corporate-action store while keeping disclosure-derived hints as a bounded fallback.
// Reason: the store may still be sparse in some workspaces, so dropping disclosure hints would
// make the field regress to empty in the very environments we are trying to thicken.
// Purpose: produce one governed capital-return bucket without reopening label or execution logic.
fn build_governed_shareholder_return_status(
    bundle: &SecurityDecisionEvidenceBundleResult,
    disclosure_signals: &GovernedDisclosureSignalSummary,
) -> String {
    let fallback = derive_shareholder_return_status(
        disclosure_signals.has_dividend_notice,
        disclosure_signals.has_buyback_or_increase_notice,
    );
    let Ok(store) = SecurityCorporateActionStore::workspace_default() else {
        return fallback;
    };
    let Ok(rows) = store.load_rows_on_or_before(&bundle.symbol, &bundle.analysis_date) else {
        return fallback;
    };
    if rows.is_empty() {
        return fallback;
    }

    let has_recent_dividend_action = rows.iter().any(|row| {
        row.action_type == "cash_dividend"
            && is_date_within_days(&row.effective_date, &bundle.analysis_date, 365)
    });

    derive_shareholder_return_status(
        has_recent_dividend_action || disclosure_signals.has_dividend_notice,
        disclosure_signals.has_buyback_or_increase_notice,
    )
}

// 2026-04-17 CST: Added because store-backed disclosure rows need a stable rolling-window filter
// before they can feed event-density and risk helpers.
// Purpose: keep the windowing rule local to the evidence layer and avoid scattering date math.
fn filter_announcements_within_days(
    notices: &[crate::ops::stock::security_analysis_fullstack::DisclosureAnnouncement],
    as_of_date: &str,
    window_days: i64,
) -> Vec<crate::ops::stock::security_analysis_fullstack::DisclosureAnnouncement> {
    notices
        .iter()
        .filter(|notice| is_date_within_days(&notice.published_at, as_of_date, window_days))
        .cloned()
        .collect()
}

fn count_distinct_announcement_days(
    notices: &[crate::ops::stock::security_analysis_fullstack::DisclosureAnnouncement],
) -> usize {
    // 2026-04-17 CST: Added because real disclosure history often contains several notices on the
    // same day, and using raw row count made the event-density feature look crowded for almost
    // every symbol in the refreshed 40-name pool.
    // Purpose: convert governed disclosures into a cadence signal that training can actually use.
    notices
        .iter()
        .map(|notice| notice.published_at.clone())
        .collect::<BTreeSet<_>>()
        .len()
}

// 2026-04-17 CST: Added because governed runtime rows and request dates may carry time suffixes
// or plain dates, and the event-thickening path should tolerate both.
// Purpose: normalize the first P0-2 rolling-window implementation without changing upstream
// storage contracts.
fn is_date_within_days(date_text: &str, as_of_date: &str, window_days: i64) -> bool {
    let Some(as_of_date) = parse_date_prefix(as_of_date) else {
        return false;
    };
    let Some(event_date) = parse_date_prefix(date_text) else {
        return false;
    };
    event_date <= as_of_date && event_date >= as_of_date - Duration::days(window_days.max(0))
}

fn parse_date_prefix(value: &str) -> Option<NaiveDate> {
    let prefix = value.chars().take(10).collect::<String>();
    NaiveDate::parse_from_str(prefix.as_str(), "%Y-%m-%d").ok()
}

// 2026-04-09 CST: 这里集中把 fullstack 映射成正式证据包，原因是研究层与治理层虽然复用事实，但对象职责不同，
// 目的：统一补 analysis_date、quality、data_gaps 与 evidence_hash，避免这些逻辑散落到多个上层 Tool。
fn build_evidence_bundle(
    request: &SecurityDecisionEvidenceBundleRequest,
    analysis: SecurityAnalysisFullstackResult,
    effective_external_proxy_inputs: Option<SecurityExternalProxyInputs>,
) -> SecurityDecisionEvidenceBundleResult {
    let SecurityAnalysisFullstackResult {
        symbol,
        analysis_date,
        technical_context,
        fundamental_context,
        disclosure_context,
        etf_context,
        cross_border_context,
        industry_context,
        integrated_conclusion,
        ..
    } = analysis;

    // 2026-04-20 CST: Added because fullstack may now anchor ETF latest runs to the
    // resolved governed proxy date instead of the nested technical-context date.
    // Reason: rebuilding the evidence bundle from technical_context.analysis_date was
    // silently discarding the top-level fullstack contract that chair consumers rely on.
    // Purpose: preserve the frozen top-level analysis date all the way into evidence,
    // scorecard, committee, and chair outputs.
    let data_gaps = collect_data_gaps(
        &symbol,
        &fundamental_context,
        &disclosure_context,
        &etf_context,
        &cross_border_context,
    );
    let risk_notes = collect_risk_notes(
        &technical_context,
        &fundamental_context,
        &disclosure_context,
        &etf_context,
        &cross_border_context,
        &industry_context,
        &integrated_conclusion,
        &data_gaps,
    );
    let evidence_quality =
        build_evidence_quality(&fundamental_context, &disclosure_context, &risk_notes);
    let evidence_hash = build_evidence_hash(
        &symbol,
        &analysis_date,
        &integrated_conclusion.stance,
        &evidence_quality,
        &data_gaps,
        request,
    );

    SecurityDecisionEvidenceBundleResult {
        symbol,
        analysis_date,
        technical_context,
        fundamental_context,
        disclosure_context,
        etf_context,
        cross_border_context,
        industry_context,
        integrated_conclusion,
        evidence_quality,
        risk_notes,
        data_gaps,
        evidence_hash,
        external_proxy_inputs: effective_external_proxy_inputs,
    }
}

// 2026-04-09 CST: 这里集中定义证据缺口规则，原因是 snapshot 与 committee 都需要显式知道“缺了什么”，
// 目的：把上游 unavailable 状态翻译成稳定 data_gap 语义，方便回放、训练和投决解释复用。
fn collect_data_gaps(
    symbol: &str,
    fundamental_context: &FundamentalContext,
    disclosure_context: &DisclosureContext,
    etf_context: &EtfContext,
    cross_border_context: &CrossBorderEtfContext,
) -> Vec<String> {
    let mut data_gaps = Vec::new();

    if fundamental_context.status != "available" {
        data_gaps.push(format!(
            "基本面上下文当前不可用：{}",
            fundamental_context.headline
        ));
    }
    if disclosure_context.status != "available" {
        data_gaps.push(format!(
            "公告上下文当前不可用：{}",
            disclosure_context.headline
        ));
    }
    if classify_symbol_asset_class(symbol) == "etf" && etf_context.status != "available" {
        data_gaps.push(format!(
            "ETF 专项事实上下文当前不可用：{}",
            etf_context.headline
        ));
    }

    if cross_border_context.status == "incomplete" {
        data_gaps.push(format!(
            "跨境 ETF 穿透链未补齐：{}",
            cross_border_context.headline
        ));
    }

    data_gaps
}

// 2026-04-09 CST: 这里统一收集证据层风险提示，原因是 committee、chair、snapshot 都需要复用同一组风险摘要，
// 目的：避免不同对象各自挑选风险字段，最终导致上层治理链口径不一致。
fn collect_risk_notes(
    technical_context: &SecurityAnalysisContextualResult,
    fundamental_context: &FundamentalContext,
    disclosure_context: &DisclosureContext,
    etf_context: &EtfContext,
    cross_border_context: &CrossBorderEtfContext,
    industry_context: &IndustryContext,
    integrated_conclusion: &IntegratedConclusion,
    data_gaps: &[String],
) -> Vec<String> {
    let mut risk_notes = Vec::new();
    risk_notes.extend(technical_context.contextual_conclusion.risk_flags.clone());
    risk_notes.extend(fundamental_context.risk_flags.clone());
    risk_notes.extend(disclosure_context.risk_flags.clone());
    risk_notes.extend(etf_context.structure_risk_flags.clone());
    risk_notes.extend(etf_context.research_gaps.clone());
    risk_notes.extend(cross_border_context.risk_flags.clone());
    risk_notes.extend(industry_context.risk_flags.clone());
    risk_notes.extend(integrated_conclusion.risk_flags.clone());
    risk_notes.extend(data_gaps.iter().cloned());
    dedupe_strings(&mut risk_notes);
    risk_notes
}

// 2026-04-09 CST: 这里把多源可用性收敛成质量摘要，原因是 committee/snapshot 只需要稳定状态，而不是重复解释所有子对象，
// 目的：为风险闸门、数据质量标记和后续训练过滤提供统一输入。
fn build_evidence_quality(
    fundamental_context: &FundamentalContext,
    disclosure_context: &DisclosureContext,
    risk_notes: &[String],
) -> SecurityEvidenceQuality {
    let technical_status = "available".to_string();
    let fundamental_status = fundamental_context.status.clone();
    let disclosure_status = disclosure_context.status.clone();
    let overall_status = if fundamental_status == "available" && disclosure_status == "available" {
        "complete".to_string()
    } else {
        "degraded".to_string()
    };

    SecurityEvidenceQuality {
        technical_status,
        fundamental_status,
        disclosure_status,
        overall_status,
        risk_flags: risk_notes.to_vec(),
    }
}

// 2026-04-09 CST: 这里生成证据哈希，原因是新治理链要求 committee / snapshot / chair 都围绕同一份冻结证据演进，
// 目的：给后续回放、审计和对齐校验提供稳定证据版本锚点。
fn build_evidence_hash(
    symbol: &str,
    analysis_date: &str,
    stance: &str,
    evidence_quality: &SecurityEvidenceQuality,
    data_gaps: &[String],
    request: &SecurityDecisionEvidenceBundleRequest,
) -> String {
    let mut hasher = DefaultHasher::new();
    symbol.hash(&mut hasher);
    analysis_date.hash(&mut hasher);
    stance.hash(&mut hasher);
    evidence_quality.overall_status.hash(&mut hasher);
    evidence_quality.fundamental_status.hash(&mut hasher);
    evidence_quality.disclosure_status.hash(&mut hasher);
    data_gaps.hash(&mut hasher);
    request.market_symbol.hash(&mut hasher);
    request.sector_symbol.hash(&mut hasher);
    request.market_profile.hash(&mut hasher);
    request.sector_profile.hash(&mut hasher);
    request.underlying_symbol.hash(&mut hasher);
    request.fx_symbol.hash(&mut hasher);
    request.lookback_days.hash(&mut hasher);
    request.disclosure_limit.hash(&mut hasher);
    // 2026-04-14 CST: 这里改为序列化后摘要 external proxy 输入，原因是 Option<f64> 无法直接 derive Hash；
    // 目的：继续把 external proxy 输入纳入 evidence hash，同时避免为止血引入更重的自定义 Hash 实现。
    if let Ok(serialized_proxy_inputs) = serde_json::to_string(&request.external_proxy_inputs) {
        serialized_proxy_inputs.hash(&mut hasher);
    }
    format!("sec-{:016x}", hasher.finish())
}

// 2026-04-14 CST: 这里补 ETF/股票统一的 ETF 判断助手，原因是 scorecard runtime 仍通过证据层公共函数识别 ETF；
// 目的：避免 ETF 判断逻辑继续散落在 scorecard/training 多处，先收敛到证据层单一口径。
pub fn is_etf_symbol(symbol: &str) -> bool {
    classify_symbol_asset_class(symbol) == "etf"
}

// 2026-04-14 CST: 这里补 ETF 子池识别助手，原因是训练 artifact 和运行时 scorecard 现在都需要同一份 ETF 子池归类；
// 目的：先用最小规则收口 instrument_subscope，后续再在重构阶段升级为更细颗粒度分类。
pub fn resolve_etf_subscope(
    symbol: &str,
    market_profile: Option<&str>,
    asset_scope: Option<&str>,
) -> Option<&'static str> {
    if !is_etf_symbol(symbol) {
        return None;
    }
    let market_profile = market_profile.unwrap_or_default().to_ascii_lowercase();
    let asset_scope = asset_scope.unwrap_or_default().to_ascii_lowercase();
    if asset_scope.contains("gold")
        || market_profile.contains("gold")
        || asset_scope.contains("commodity")
        || market_profile.contains("commodity")
    {
        Some("commodity_etf")
    } else if asset_scope.contains("treasury")
        || market_profile.contains("treasury")
        || asset_scope.contains("bond")
        || market_profile.contains("bond")
    {
        Some("bond_etf")
    } else if asset_scope.contains("cross_border")
        || asset_scope.contains("overseas")
        || market_profile.contains("overseas")
        || market_profile.contains("cross_border")
    {
        Some("cross_border_etf")
    } else {
        Some("equity_etf")
    }
}

// 2026-04-16 CST: Added because A-1a approved formal training-sample thickening on the
// canonical securities chain.
// Reason: training was still missing one governed place to derive coarse market / industry /
// flow / valuation proxy buckets from already-frozen evidence.
// Purpose: keep snapshot and runtime scorecard aligned on the same minimum segmentation logic
// before later sessions add deeper real-data factor families.
pub fn derive_market_regime(
    market_profile: Option<&str>,
    subject_asset_class: Option<&str>,
    market_bias: Option<&str>,
    market_breakout_signal: Option<&str>,
    market_volatility_state: Option<&str>,
    market_momentum_signal: Option<&str>,
) -> String {
    let market_profile = market_profile.unwrap_or_default().to_ascii_lowercase();
    let subject_asset_class = subject_asset_class.unwrap_or_default().to_ascii_lowercase();
    let market_bias = market_bias.unwrap_or_default().to_ascii_lowercase();
    let market_breakout_signal = market_breakout_signal
        .unwrap_or_default()
        .to_ascii_lowercase();
    let market_volatility_state = market_volatility_state
        .unwrap_or_default()
        .to_ascii_lowercase();
    let market_momentum_signal = market_momentum_signal
        .unwrap_or_default()
        .to_ascii_lowercase();

    if market_bias.contains("bullish") && market_breakout_signal == "confirmed" {
        return "bull_breakout".to_string();
    }
    if market_bias.contains("bullish")
        && (market_momentum_signal == "positive" || market_breakout_signal.contains("retest"))
    {
        return "bull_trend".to_string();
    }
    if market_bias.contains("bearish")
        && (market_momentum_signal == "negative"
            || market_breakout_signal.contains("failed")
            || market_breakout_signal.contains("range"))
    {
        return "bear_pressure".to_string();
    }
    if market_volatility_state.contains("high") || market_volatility_state.contains("wide") {
        return "range_high_vol".to_string();
    }
    if market_volatility_state.contains("low") || market_volatility_state.contains("contract") {
        return "range_low_vol".to_string();
    }

    if market_profile.contains("cross_border") || market_profile.contains("overseas") {
        "cross_border".to_string()
    } else if market_profile.contains("bond") {
        "bond_domestic".to_string()
    } else if market_profile.contains("commodity") {
        "commodity".to_string()
    } else if market_profile.contains("a_share") {
        "a_share".to_string()
    } else if subject_asset_class == "etf" {
        "etf_generic".to_string()
    } else if subject_asset_class == "equity" {
        "equity_generic".to_string()
    } else {
        "unknown".to_string()
    }
}

// 2026-04-16 CST: Added because the approved A-1a phase needs a stable industry bucket
// even when the current repo only has coarse `sector_profile` routing information.
// Reason: using raw `sector_profile` strings directly in every downstream consumer would keep
// the training chain coupled to request naming details.
// Purpose: provide one minimal normalized industry bucket now, with room for later refinement.
pub fn derive_industry_bucket(
    sector_profile: Option<&str>,
    symbol_level_bucket: Option<&str>,
    instrument_subscope: Option<&str>,
    subject_asset_class: Option<&str>,
) -> String {
    if let Some(symbol_level_bucket) = symbol_level_bucket.filter(|value| !value.trim().is_empty())
    {
        return symbol_level_bucket.trim().to_string();
    }
    let sector_profile = sector_profile.unwrap_or_default().to_ascii_lowercase();
    if sector_profile.contains("bank") {
        return "bank".to_string();
    }
    if sector_profile.contains("broker") || sector_profile.contains("securities") {
        return "broker".to_string();
    }
    if sector_profile.contains("insurance") {
        return "insurance".to_string();
    }
    if !sector_profile.is_empty() {
        return sector_profile
            .trim_start_matches("a_share_")
            .trim_end_matches("_peer")
            .trim_end_matches("_cross_border")
            .to_string();
    }
    if let Some(instrument_subscope) = instrument_subscope {
        return instrument_subscope.to_string();
    }
    subject_asset_class.unwrap_or("unknown").to_string()
}

// 2026-04-16 CST: Added because both training and runtime gates need one shared instrument
// subscope vocabulary instead of re-inferring it ad hoc.
// Reason: ETF already has a governed helper, but equities still fell back to missing.
// Purpose: freeze one minimum cross-asset subscope field for sample segmentation.
pub fn derive_instrument_subscope(
    symbol: &str,
    market_profile: Option<&str>,
    subject_asset_class: Option<&str>,
) -> String {
    if let Some(etf_subscope) = resolve_etf_subscope(symbol, market_profile, subject_asset_class) {
        return etf_subscope.to_string();
    }
    let market_profile = market_profile.unwrap_or_default().to_ascii_lowercase();
    if market_profile.contains("hong_kong") || market_profile.contains("h_share") {
        "hk_equity".to_string()
    } else {
        "equity".to_string()
    }
}

// 2026-04-16 CST: Added because A-1a explicitly promotes event-density segmentation into the
// formal training sample, but we still only have announcement-level public evidence today.
// Reason: later models need a coarse but honest event bucket without pretending we already have
// full information-flow coverage.
// Purpose: expose one stable sparse/moderate/dense event field from current disclosure evidence.
pub fn derive_event_density_bucket(
    announcement_count: usize,
    disclosure_risk_keyword_count: usize,
) -> String {
    // 2026-04-17 CST: Adjusted because the earlier threshold turned a 90d cadence of only a few
    // announcement days into the same bucket as truly crowded disclosure windows.
    // Purpose: widen the moderate band so real governed symbols can leave the single dense bucket.
    if announcement_count >= 6 || disclosure_risk_keyword_count >= 3 {
        "dense".to_string()
    } else if announcement_count >= 3 || disclosure_risk_keyword_count >= 1 {
        "moderate".to_string()
    } else if announcement_count >= 1 {
        "light".to_string()
    } else {
        "quiet".to_string()
    }
}

// 2026-04-16 CST: Added because Q-group can no longer stay on a placeholder once training starts
// consuming flow-aware segmentation.
// Reason: the repo already freezes `money_flow_signal` and `volume_confirmation`, but downstream
// users had no governed way to read them as one coarse flow verdict.
// Purpose: provide a minimum supportive/mixed/pressured bucket until richer flow data lands.
pub fn derive_flow_status(
    money_flow_signal: Option<&str>,
    volume_confirmation: Option<&str>,
    volume_ratio_20: Option<f64>,
    mfi_14: Option<f64>,
    macd_histogram: Option<f64>,
) -> String {
    let money_flow_signal = money_flow_signal.unwrap_or_default().to_ascii_lowercase();
    let volume_confirmation = volume_confirmation.unwrap_or_default().to_ascii_lowercase();
    let volume_ratio_20 = volume_ratio_20.unwrap_or(1.0);
    let mfi_14 = mfi_14.unwrap_or(50.0);
    let macd_histogram = macd_histogram.unwrap_or(0.0);
    let positive_flow = money_flow_signal.contains("positive")
        || money_flow_signal.contains("support")
        || money_flow_signal.contains("inflow")
        || money_flow_signal.contains("accumulation");
    let negative_flow = money_flow_signal.contains("negative")
        || money_flow_signal.contains("pressure")
        || money_flow_signal.contains("outflow")
        || money_flow_signal.contains("distribution");
    let confirmed_volume = volume_confirmation.contains("confirm")
        || volume_confirmation.contains("support")
        || volume_confirmation.contains("positive")
        || volume_ratio_20 >= 1.15;
    let weak_volume = volume_confirmation.contains("weak")
        || volume_confirmation.contains("absent")
        || volume_confirmation.contains("negative")
        || volume_ratio_20 <= 0.90;
    let supportive_money = mfi_14 >= 35.0 && mfi_14 <= 75.0 && macd_histogram >= -0.05;
    let pressured_money = mfi_14 >= 78.0 || macd_histogram <= -0.05;

    if positive_flow && confirmed_volume && supportive_money {
        "supportive".to_string()
    } else if negative_flow && weak_volume && pressured_money {
        "pressured".to_string()
    } else {
        "mixed".to_string()
    }
}

// 2026-04-20 CST: Added because money-flow semantics were already available upstream but were
// not projected into a simple directional state for factor review.
// Purpose: expose a governed inflow/outflow/neutral direction field before training migration.
pub fn derive_flow_direction_state(money_flow_signal: &str) -> &'static str {
    let money_flow_signal = money_flow_signal.to_ascii_lowercase();
    if money_flow_signal.contains("accumulation")
        || money_flow_signal.contains("positive")
        || money_flow_signal.contains("support")
        || money_flow_signal.contains("inflow")
    {
        "up"
    } else if money_flow_signal.contains("distribution")
        || money_flow_signal.contains("negative")
        || money_flow_signal.contains("pressure")
        || money_flow_signal.contains("outflow")
    {
        "down"
    } else {
        "neutral"
    }
}

// 2026-04-16 CST: Added because V-group still had no formal output even though the current
// evidence layer already carries position and mean-reversion proxies.
// Reason: we do not yet have full PE/PB coverage, but training still needs a governed way to
// separate compressed vs extended price-state buckets.
// Purpose: keep the current field honest as a position/extension proxy while removing the placeholder.
pub fn derive_valuation_status(
    range_position_signal: Option<&str>,
    bollinger_position_signal: Option<&str>,
    mean_reversion_signal: Option<&str>,
    profit_signal: Option<&str>,
    revenue_yoy_pct: Option<f64>,
    net_profit_yoy_pct: Option<f64>,
    roe_pct: Option<f64>,
) -> String {
    let range_position_signal = range_position_signal
        .unwrap_or_default()
        .to_ascii_lowercase();
    let bollinger_position_signal = bollinger_position_signal
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mean_reversion_signal = mean_reversion_signal
        .unwrap_or_default()
        .to_ascii_lowercase();
    let quality_bucket = derive_fundamental_quality_bucket(
        profit_signal.unwrap_or_default(),
        revenue_yoy_pct,
        net_profit_yoy_pct,
        roe_pct,
    );

    let extended = range_position_signal.contains("high")
        || range_position_signal.contains("upper")
        || bollinger_position_signal.contains("upper")
        || mean_reversion_signal.contains("overbought");
    let compressed = range_position_signal.contains("low")
        || range_position_signal.contains("lower")
        || bollinger_position_signal.contains("lower")
        || mean_reversion_signal.contains("oversold");

    if compressed && quality_bucket == "strong" {
        "undervalued_candidate".to_string()
    } else if extended && quality_bucket == "fragile" {
        "overvalued_risk".to_string()
    } else if extended {
        "extended".to_string()
    } else if compressed {
        "compressed".to_string()
    } else {
        "balanced".to_string()
    }
}

// 2026-04-20 CST: Added because Task A needs position-factor review buckets that the user can
// read directly before the next Nikkei retraining pass.
// Purpose: freeze one plain upper/middle/lower mapping instead of forcing review to parse the
// longer consultation wording each time.
pub fn derive_bollinger_position_bucket_20d(bollinger_position_signal: &str) -> &'static str {
    let bollinger_position_signal = bollinger_position_signal.to_ascii_lowercase();
    if bollinger_position_signal.contains("upper") {
        "upper"
    } else if bollinger_position_signal.contains("lower") {
        "lower"
    } else {
        "middle"
    }
}

// 2026-04-20 CST: Added because Task A needs the old range-position wording normalized into a
// high/middle/low bucket that users can review against 14d window semantics.
// Purpose: expose one stable 14d range-position bucket for training and factor diagnostics.
pub fn derive_range_position_bucket_14d(range_position_signal: &str) -> &'static str {
    let range_position_signal = range_position_signal.to_ascii_lowercase();
    if range_position_signal.contains("overbought") {
        "high"
    } else if range_position_signal.contains("oversold") {
        "low"
    } else {
        "middle"
    }
}

// 2026-04-20 CST: Added because Task A needs the mean-reversion slice kept separate from the
// bundled valuation_status field.
// Purpose: freeze an overbought/neutral/oversold 20d bucket that can be audited independently.
pub fn derive_mean_reversion_bucket_20d(mean_reversion_signal: &str) -> &'static str {
    let mean_reversion_signal = mean_reversion_signal.to_ascii_lowercase();
    if mean_reversion_signal.contains("overbought") {
        "overbought"
    } else if mean_reversion_signal.contains("oversold") {
        "oversold"
    } else {
        "neutral"
    }
}

// 2026-04-20 CST: Added because the approved Nikkei retraining route now normalizes
// MA20 deviation by ATR14 before assigning the five review buckets.
// Reason: the 2025 regime shift made raw percentage bands drift, especially in the middle bucket.
// Purpose: freeze one volatility-adjusted distance measure for snapshot, runtime, and training.
pub fn derive_mean_reversion_normalized_distance_20d(
    close_vs_sma20: f64,
    atr_ratio_14: f64,
) -> f64 {
    if atr_ratio_14.abs() <= f64::EPSILON {
        0.0
    } else {
        close_vs_sma20 / atr_ratio_14
    }
}

// 2026-04-20 CST: Updated because the approved Nikkei retraining route now bins ATR-normalized
// distance instead of raw percentage distance from MA20.
// Reason: the user wants the neutral bucket compressed while keeping weak-direction buckets dense.
// Purpose: freeze one five-level normalized mean-reversion contract across replay and training.
pub fn derive_mean_reversion_deviation_bucket_20d(
    mean_reversion_normalized_distance_20d: f64,
) -> &'static str {
    if mean_reversion_normalized_distance_20d < -2.6 {
        "strong_down"
    } else if mean_reversion_normalized_distance_20d < -0.15 {
        "weak_down"
    } else if mean_reversion_normalized_distance_20d <= 0.15 {
        "neutral"
    } else if mean_reversion_normalized_distance_20d <= 2.6 {
        "weak_up"
    } else {
        "strong_up"
    }
}

// 2026-04-20 CST: Added because Task A must review the quality slice with a plain user-facing
// name instead of the longer legacy training alias.
// Purpose: keep quality semantics stable while letting the new training contract read
// strong/balanced/fragile directly.
pub fn derive_quality_bucket(fundamental_quality_bucket: &str) -> &'static str {
    let fundamental_quality_bucket = fundamental_quality_bucket.to_ascii_lowercase();
    if fundamental_quality_bucket == "strong" {
        "strong"
    } else if fundamental_quality_bucket == "fragile" {
        "fragile"
    } else {
        "balanced"
    }
}

// 2026-04-20 CST: Added because several technical oscillation families already carried explicit
// rebound vs pullback meaning, but that meaning was not frozen into directional helper fields.
// Purpose: standardize directional helper output across mean-reversion, range, band, and timing signals.
pub fn derive_mean_reversion_direction_state(mean_reversion_signal: &str) -> &'static str {
    let mean_reversion_signal = mean_reversion_signal.to_ascii_lowercase();
    if mean_reversion_signal.contains("oversold") {
        "up"
    } else if mean_reversion_signal.contains("overbought") {
        "down"
    } else {
        "neutral"
    }
}

pub fn derive_range_position_direction_state(range_position_signal: &str) -> &'static str {
    let range_position_signal = range_position_signal.to_ascii_lowercase();
    if range_position_signal.contains("oversold") {
        "up"
    } else if range_position_signal.contains("overbought") {
        "down"
    } else {
        "neutral"
    }
}

pub fn derive_bollinger_position_direction_state(
    bollinger_position_signal: &str,
) -> &'static str {
    let bollinger_position_signal = bollinger_position_signal.to_ascii_lowercase();
    if bollinger_position_signal.contains("lower") {
        "up"
    } else if bollinger_position_signal.contains("upper") {
        "down"
    } else {
        "neutral"
    }
}

pub fn derive_bollinger_midline_direction_state(
    bollinger_midline_signal: &str,
) -> &'static str {
    let bollinger_midline_signal = bollinger_midline_signal.to_ascii_lowercase();
    if bollinger_midline_signal.contains("support") {
        "up"
    } else if bollinger_midline_signal.contains("resistance") {
        "down"
    } else {
        "neutral"
    }
}

pub fn derive_rsrs_direction_state(rsrs_signal: &str) -> &'static str {
    let rsrs_signal = rsrs_signal.to_ascii_lowercase();
    if rsrs_signal.contains("bullish") {
        "up"
    } else if rsrs_signal.contains("bearish") {
        "down"
    } else {
        "neutral"
    }
}

pub fn derive_divergence_direction_state(divergence_signal: &str) -> &'static str {
    let divergence_signal = divergence_signal.to_ascii_lowercase();
    if divergence_signal.contains("bullish") {
        "up"
    } else if divergence_signal.contains("bearish") {
        "down"
    } else {
        "neutral"
    }
}

pub fn derive_timing_direction_state(timing_signal: &str) -> &'static str {
    let timing_signal = timing_signal.to_ascii_lowercase();
    if timing_signal.contains("oversold") {
        "up"
    } else if timing_signal.contains("overbought") {
        "down"
    } else {
        "neutral"
    }
}

// 2026-04-14 CST: 这里补 ETF 特征族门禁函数，原因是当前 scorecard runtime 要判断不同 ETF 子池至少具备哪些特征；
// 目的：让 ETF 模型兼容门禁先恢复成显式合同，而不是在编译失败期间完全失去约束。
fn derive_hard_risk_score(disclosure_signals: &GovernedDisclosureSignalSummary) -> f64 {
    // 2026-04-17 CST: Added because event-side analysis now needs a weighted hard-risk component
    // instead of collapsing governance and profit warnings into one flat boolean.
    // Purpose: expose one explainable severe-event score for governed snapshot and training.
    let mut score = 0.0;
    if disclosure_signals.has_risk_warning_notice {
        score += 4.0;
    }
    if disclosure_signals.has_inquiry_notice {
        score += 3.0;
    }
    if disclosure_signals.has_litigation_notice {
        score += 4.0;
    }
    if disclosure_signals.has_preloss_or_loss_notice {
        score += 4.0;
    }
    if disclosure_signals.has_fund_occupation_notice {
        score += 5.0;
    }
    score
}

fn derive_negative_attention_score(disclosure_signals: &GovernedDisclosureSignalSummary) -> f64 {
    // 2026-04-17 CST: Added because several negative-but-not-fatal events should still pressure
    // the message surface without being mislabeled as the same severity as hard risks.
    // Purpose: separate financing and attention shocks from the severe-risk bucket.
    let mut score = 0.0;
    if disclosure_signals.has_reduction_notice {
        score += 2.0;
    }
    if disclosure_signals.has_refinancing_notice {
        score += 2.0;
    }
    if disclosure_signals.has_termination_notice {
        score += 2.0;
    }
    if disclosure_signals.has_abnormal_volatility_notice {
        score += 1.0;
    }
    score
}

fn derive_positive_support_score(disclosure_signals: &GovernedDisclosureSignalSummary) -> f64 {
    // 2026-04-17 CST: Added because message-side support should be expressed as its own component
    // before netting against the new negative buckets.
    // Purpose: preserve simple upside-support evidence without claiming a full event-stage model.
    let mut score = 0.0;
    if disclosure_signals.has_buyback_or_increase_notice {
        score += 3.0;
    }
    if disclosure_signals.has_dividend_notice {
        score += 1.0;
    }
    score
}

fn derive_event_net_impact_score(disclosure_signals: &GovernedDisclosureSignalSummary) -> f64 {
    // 2026-04-17 CST: Added because downstream consumers need one net event direction field that
    // remains explainable and decomposable back into positive / negative components.
    // Purpose: provide the first governed disclosure impact score without hiding component details.
    derive_positive_support_score(disclosure_signals)
        - derive_hard_risk_score(disclosure_signals)
        - derive_negative_attention_score(disclosure_signals)
}

pub fn derive_shareholder_return_status(
    has_dividend_notice: bool,
    has_buyback_or_increase_notice: bool,
) -> String {
    if has_dividend_notice && has_buyback_or_increase_notice {
        "capital_return_active".to_string()
    } else if has_dividend_notice {
        "dividend_only".to_string()
    } else if has_buyback_or_increase_notice {
        "buyback_or_increase_only".to_string()
    } else {
        "capital_return_absent".to_string()
    }
}

pub fn derive_fundamental_quality_bucket(
    profit_signal: &str,
    revenue_yoy_pct: Option<f64>,
    net_profit_yoy_pct: Option<f64>,
    roe_pct: Option<f64>,
) -> String {
    let profit_signal = profit_signal.to_ascii_lowercase();
    let revenue_yoy_pct = revenue_yoy_pct.unwrap_or(0.0);
    let net_profit_yoy_pct = net_profit_yoy_pct.unwrap_or(0.0);
    let roe_pct = roe_pct.unwrap_or(0.0);

    if profit_signal == "positive" && net_profit_yoy_pct >= 8.0 && roe_pct >= 12.0 {
        "strong".to_string()
    } else if profit_signal == "negative"
        || net_profit_yoy_pct < 0.0
        || revenue_yoy_pct < 0.0
        || roe_pct < 6.0
    {
        "fragile".to_string()
    } else {
        "balanced".to_string()
    }
}

pub fn derive_atr_ratio_14(close: f64, atr_14: f64) -> f64 {
    if close.abs() <= f64::EPSILON {
        0.0
    } else {
        atr_14 / close.abs()
    }
}

// 2026-04-20 CST: Added because RSI and MACD were entering training only as floating-point bins,
// while the user explicitly asked to review them in up/down semantics first.
// Purpose: expose fixed, sample-independent directional helper labels alongside the raw numbers.
pub fn derive_rsi_direction_state(rsi_14: f64) -> &'static str {
    if rsi_14 >= 50.0 {
        "above_50"
    } else {
        "below_50"
    }
}

pub fn derive_rsi_extreme_state(rsi_14: f64) -> &'static str {
    if rsi_14 >= 70.0 {
        "overbought"
    } else if rsi_14 <= 30.0 {
        "oversold"
    } else {
        "neutral"
    }
}

pub fn derive_macd_histogram_direction(macd_histogram: f64) -> &'static str {
    if macd_histogram > 0.0 {
        "positive"
    } else if macd_histogram < 0.0 {
        "negative"
    } else {
        "flat"
    }
}

pub fn derive_ratio_delta(current_value: f64, baseline_value: f64) -> f64 {
    if baseline_value.abs() <= f64::EPSILON {
        0.0
    } else {
        current_value / baseline_value - 1.0
    }
}

pub fn derive_support_gap_pct_20(close: f64, support_level_20: f64) -> f64 {
    if close.abs() <= f64::EPSILON {
        0.0
    } else {
        (close - support_level_20) / close.abs()
    }
}

pub fn derive_resistance_gap_pct_20(close: f64, resistance_level_20: f64) -> f64 {
    if close.abs() <= f64::EPSILON {
        0.0
    } else {
        (resistance_level_20 - close) / close.abs()
    }
}

pub fn required_etf_feature_family(instrument_subscope: Option<&str>) -> &'static [&'static str] {
    match normalize_etf_subscope_alias(instrument_subscope.unwrap_or("equity_etf")) {
        "commodity_etf" => &[
            "gold_spot_proxy_status",
            "gold_spot_proxy_return_5d",
            "real_rate_proxy_status",
            "real_rate_proxy_delta_bp_5d",
        ],
        "bond_etf" => &[
            "yield_curve_proxy_status",
            "yield_curve_slope_delta_bp_5d",
            "funding_liquidity_proxy_status",
            "funding_liquidity_spread_delta_bp_5d",
        ],
        "cross_border_etf" => &[
            "fx_proxy_status",
            "fx_return_5d",
            "overseas_market_proxy_status",
            "overseas_market_return_5d",
            // 2026-04-15 CST: Added because cross-border ETF gating should treat
            // session-gap evidence as part of the required proxy family.
            // Reason: approval/runtime consumers already use this signal when overseas sessions are offset.
            // Purpose: prevent committee and approval gates from seeing an incomplete cross-border proxy surface.
            "market_session_gap_status",
            "market_session_gap_days",
        ],
        _ => &[
            "benchmark_relative_strength_status",
            "benchmark_relative_return_5d",
            "etf_fund_flow_proxy_status",
            "etf_fund_flow_5d",
        ],
    }
}

// 2026-04-20 CST: Added because the next index-training pass needs direction and consistency
// semantics to be explicit before any feature-selection discussion can be trusted.
// Purpose: expose one governed up/down/neutral vocabulary that later training heads can reuse.
pub fn derive_alignment_direction(trend_bias: &str) -> &'static str {
    let trend_bias = trend_bias.to_ascii_lowercase();
    if trend_bias.contains("bullish") {
        "up"
    } else if trend_bias.contains("bearish") {
        "down"
    } else {
        "neutral"
    }
}

// 2026-04-20 CST: Added because the old `technical_alignment` field mixed direction and
// consistency into one label, which made the user-facing factor review hard to interpret.
// Purpose: split "same side or not" from "up or down" without changing existing alignment labels.
pub fn derive_alignment_consistency(alignment: &str) -> &'static str {
    match alignment {
        "tailwind" => "aligned",
        "headwind" => "conflicted",
        _ => "mixed",
    }
}

// 2026-04-20 CST: Added because trend strength alone cannot answer whether the strong move is
// pointing up or down, which was the user's main factor-direction complaint.
// Purpose: bind trend direction and strength into one stable categorical state.
pub fn derive_trend_direction_strength(trend_bias: &str, trend_strength: &str) -> &'static str {
    match (
        derive_alignment_direction(trend_bias),
        trend_strength.to_ascii_lowercase().as_str(),
    ) {
        ("up", "strong") => "up_strong",
        ("up", "moderate") => "up_moderate",
        ("down", "strong") => "down_strong",
        ("down", "moderate") => "down_moderate",
        (_, "weak") => "range_weak",
        ("up", _) => "up_moderate",
        ("down", _) => "down_moderate",
        _ => "range_weak",
    }
}

// 2026-04-20 CST: Added because the current volume label says whether volume confirms, but not
// which side it confirms.
// Purpose: freeze one directional volume helper before the next factor-selection pass.
pub fn derive_volume_direction_state(
    trend_bias: &str,
    volume_confirmation: &str,
) -> &'static str {
    let volume_confirmation = volume_confirmation.to_ascii_lowercase();
    if volume_confirmation.contains("weak") {
        return "weakening";
    }
    if !volume_confirmation.contains("confirm")
        && !volume_confirmation.contains("support")
        && !volume_confirmation.contains("positive")
    {
        return "neutral";
    }
    match derive_alignment_direction(trend_bias) {
        "up" => "up_confirmed",
        "down" => "down_confirmed",
        _ => "neutral",
    }
}

// 2026-04-20 CST: Added because breakout structure currently carries both up-breaks and
// down-breaks in one family, which blocked clear factor audits.
// Purpose: expose the directional half of breakout structure as a separate governed field.
pub fn derive_breakout_direction(breakout_signal: &str) -> &'static str {
    let breakout_signal = breakout_signal.to_ascii_lowercase();
    if breakout_signal.contains("resistance") {
        "up"
    } else if breakout_signal.contains("support") {
        "down"
    } else {
        "none"
    }
}

// 2026-04-20 CST: Added because breakout direction alone still hides whether the move is only
// being watched, already confirmed, or already failed.
// Purpose: split structural stage from structural direction for later training selection.
pub fn derive_breakout_stage(breakout_signal: &str) -> &'static str {
    let breakout_signal = breakout_signal.to_ascii_lowercase();
    if breakout_signal.contains("failed") {
        "failed"
    } else if breakout_signal.contains("watch") {
        "watch"
    } else if breakout_signal.contains("confirmed") {
        "confirmed"
    } else {
        "range"
    }
}

// 2026-04-20 CST: Added because the existing coarse market regime mixes market direction and
// volatility shape in one label, which made the user's requested up/down audit noisy.
// Purpose: provide one small market-direction helper ahead of the later trainer migration.
pub fn derive_market_direction_regime(
    market_bias: Option<&str>,
    market_breakout_signal: Option<&str>,
    market_momentum_signal: Option<&str>,
) -> &'static str {
    let market_bias = market_bias.unwrap_or_default().to_ascii_lowercase();
    let market_breakout_signal = market_breakout_signal
        .unwrap_or_default()
        .to_ascii_lowercase();
    let market_momentum_signal = market_momentum_signal
        .unwrap_or_default()
        .to_ascii_lowercase();
    if market_bias.contains("bullish")
        || market_breakout_signal.contains("resistance")
        || market_momentum_signal == "positive"
    {
        "up"
    } else if market_bias.contains("bearish")
        || market_breakout_signal.contains("support")
        || market_momentum_signal == "negative"
    {
        "down"
    } else {
        "range"
    }
}

// 2026-04-20 CST: Added because the old market regime field also bundled volatility semantics.
// Purpose: keep high/low/normal volatility as a separate condition field instead of a direction field.
pub fn derive_market_volatility_regime(market_volatility_state: Option<&str>) -> &'static str {
    let market_volatility_state = market_volatility_state
        .unwrap_or_default()
        .to_ascii_lowercase();
    if market_volatility_state.contains("high")
        || market_volatility_state.contains("wide")
        || market_volatility_state.contains("expand")
    {
        "high"
    } else if market_volatility_state.contains("low")
        || market_volatility_state.contains("contract")
        || market_volatility_state.contains("narrow")
    {
        "low"
    } else {
        "normal"
    }
}

// 2026-04-20 CST: Added because ETF runtime and artifact fixtures now carry both
// legacy pool names (`gold_etf`, `treasury_etf`) and normalized family names
// (`commodity_etf`, `bond_etf`).
// Reason: scorecard-family validation must compare equivalent ETF pools instead of
// rejecting valid bindings purely due to alias vocabulary drift.
// Purpose: keep required-feature-family checks and subscope guards aligned with the
// frozen ETF proxy-history contract recorded in handoff.
pub fn normalize_etf_subscope_alias(instrument_subscope: &str) -> &'static str {
    match instrument_subscope {
        "gold_etf" => "commodity_etf",
        "treasury_etf" => "bond_etf",
        "commodity_etf" => "commodity_etf",
        "bond_etf" => "bond_etf",
        "cross_border_etf" => "cross_border_etf",
        _ => "equity_etf",
    }
}

fn dedupe_strings(values: &mut Vec<String>) {
    let mut deduped = Vec::new();
    for value in values.drain(..) {
        if !deduped.contains(&value) {
            deduped.push(value);
        }
    }
    *values = deduped;
}

fn normalized_numeric_feature(value: Option<f64>) -> Value {
    // 2026-04-10 CST: 这里把缺失 numeric 特征回填成稳定数字，原因是用户要求做成统一标准而不是下游各自兜底；
    // 目的：保证对训练和评分暴露的 numeric feature 永远保持 numeric 类型，减少 null 漂移带来的契约断裂。
    json!(value.unwrap_or(0.0))
}

fn insert_optional_string_feature(
    features: &mut BTreeMap<String, Value>,
    key: &str,
    value: Option<String>,
) {
    features.insert(
        key.to_string(),
        value.map(Value::String).unwrap_or(Value::Null),
    );
}

fn insert_optional_numeric_feature(
    features: &mut BTreeMap<String, Value>,
    key: &str,
    value: Option<f64>,
) {
    features.insert(
        key.to_string(),
        value.map_or(Value::Null, |value| json!(value)),
    );
}

fn classify_asset_class(bundle: &SecurityDecisionEvidenceBundleResult) -> &'static str {
    // 2026-04-20 CST: Added because Task 1 freezes non-equity subject identity in the
    // evidence seed before downstream training slices widen beyond plain equities.
    // Purpose: let explicit index/FX symbols bypass the old ETF-vs-equity-only fallback.
    let symbol_asset_class = classify_symbol_asset_class(&bundle.symbol);
    if symbol_asset_class != "equity" {
        symbol_asset_class
    } else if bundle.etf_context.status != "not_applicable" {
        "etf"
    } else {
        "equity"
    }
}

fn classify_symbol_asset_class(symbol: &str) -> &'static str {
    let normalized_symbol = symbol.trim().to_uppercase();
    // 2026-04-20 CST: Added because Task 1 starts with Nikkei index identity governance and
    // must not let explicit index/FX suffixes collapse back into the equity bucket.
    // Purpose: provide one minimal shared asset-class rule for snapshot, evidence, and training.
    if normalized_symbol.ends_with(".IDX") {
        return "index";
    }
    if normalized_symbol.ends_with(".FX") {
        return "fx";
    }
    let is_etf = normalized_symbol
        .strip_suffix(".SZ")
        .map(|code| code.starts_with("15") || code.starts_with("16"))
        .unwrap_or(false)
        || normalized_symbol
            .strip_suffix(".SH")
            .map(|code| code.starts_with("51") || code.starts_with("56") || code.starts_with("58"))
            .unwrap_or(false);
    if is_etf { "etf" } else { "equity" }
}

fn default_lookback_days() -> usize {
    260
}

fn default_disclosure_limit() -> usize {
    8
}

#[cfg(test)]
mod tests {
    use super::{
        derive_alignment_consistency, derive_alignment_direction,
        derive_bollinger_position_bucket_20d, derive_breakout_direction, derive_breakout_stage,
        derive_event_density_bucket, derive_macd_histogram_direction,
        derive_market_direction_regime, derive_market_volatility_regime,
        derive_mean_reversion_bucket_20d, derive_mean_reversion_deviation_bucket_20d,
        derive_mean_reversion_normalized_distance_20d,
        derive_quality_bucket,
        derive_range_position_bucket_14d, derive_rsi_direction_state, derive_rsi_extreme_state,
        derive_trend_direction_strength, derive_volume_direction_state,
    };

    #[test]
    fn derive_event_density_bucket_keeps_mid_cadence_symbols_out_of_dense() {
        // 2026-04-17 CST: Added because the real 40-name rerun showed that the old threshold made
        // almost every governed symbol look dense once the disclosure store could provide enough
        // recent notices.
        // Purpose: keep the bucket honest for symbols with visible activity but without truly
        // crowded announcement cadence.
        assert_eq!(derive_event_density_bucket(5, 0), "moderate");
        assert_eq!(derive_event_density_bucket(3, 1), "moderate");
    }

    #[test]
    fn derive_event_density_bucket_still_marks_high_cadence_or_high_risk_as_dense() {
        // 2026-04-17 CST: Added because relaxing the cadence threshold must not remove the
        // dense bucket for genuinely crowded or risky disclosure windows.
        // Purpose: preserve the upper-tail signal while fixing the former single-bucket collapse.
        assert_eq!(derive_event_density_bucket(6, 0), "dense");
        assert_eq!(derive_event_density_bucket(2, 3), "dense");
    }

    #[test]
    fn derive_breakout_direction_and_stage_split_up_down_and_structure() {
        assert_eq!(
            derive_breakout_direction("confirmed_resistance_breakout"),
            "up"
        );
        assert_eq!(
            derive_breakout_direction("confirmed_support_breakdown"),
            "down"
        );
        assert_eq!(derive_breakout_direction("range_bound"), "none");
        assert_eq!(derive_breakout_stage("confirmed_resistance_breakout"), "confirmed");
        assert_eq!(derive_breakout_stage("support_breakdown_watch"), "watch");
        assert_eq!(derive_breakout_stage("failed_resistance_breakout"), "failed");
        assert_eq!(derive_breakout_stage("range_bound"), "range");
    }

    #[test]
    fn derive_trend_and_volume_directional_states_keep_direction_and_conviction_together() {
        assert_eq!(
            derive_trend_direction_strength("bullish", "strong"),
            "up_strong"
        );
        assert_eq!(
            derive_trend_direction_strength("bearish", "moderate"),
            "down_moderate"
        );
        assert_eq!(
            derive_trend_direction_strength("sideways", "weak"),
            "range_weak"
        );
        assert_eq!(
            derive_volume_direction_state("bullish", "confirmed"),
            "up_confirmed"
        );
        assert_eq!(
            derive_volume_direction_state("bearish", "confirmed"),
            "down_confirmed"
        );
        assert_eq!(
            derive_volume_direction_state("sideways", "weakening"),
            "weakening"
        );
    }

    #[test]
    fn derive_alignment_and_market_regimes_separate_direction_from_consistency() {
        assert_eq!(derive_alignment_direction("bullish"), "up");
        assert_eq!(derive_alignment_direction("bearish"), "down");
        assert_eq!(derive_alignment_direction("sideways"), "neutral");
        assert_eq!(derive_alignment_consistency("tailwind"), "aligned");
        assert_eq!(derive_alignment_consistency("headwind"), "conflicted");
        assert_eq!(derive_alignment_consistency("mixed"), "mixed");
        assert_eq!(
            derive_market_direction_regime(
                Some("bullish_continuation"),
                Some("confirmed_resistance_breakout"),
                Some("positive")
            ),
            "up"
        );
        assert_eq!(
            derive_market_direction_regime(
                Some("bearish_continuation"),
                Some("support_breakdown_watch"),
                Some("negative")
            ),
            "down"
        );
        assert_eq!(
            derive_market_volatility_regime(Some("high_volatility_expansion")),
            "high"
        );
        assert_eq!(
            derive_market_volatility_regime(Some("low_volatility_contraction")),
            "low"
        );
    }

    #[test]
    fn derive_rsi_and_macd_directional_states_keep_fixed_semantics() {
        assert_eq!(derive_rsi_direction_state(58.0), "above_50");
        assert_eq!(derive_rsi_direction_state(42.0), "below_50");
        assert_eq!(derive_rsi_extreme_state(72.0), "overbought");
        assert_eq!(derive_rsi_extreme_state(25.0), "oversold");
        assert_eq!(derive_rsi_extreme_state(53.0), "neutral");
        assert_eq!(derive_macd_histogram_direction(12.5), "positive");
        assert_eq!(derive_macd_histogram_direction(-3.2), "negative");
        assert_eq!(derive_macd_histogram_direction(0.0), "flat");
    }

    #[test]
    fn derive_position_and_quality_buckets_normalize_valuation_inputs_for_training() {
        // 2026-04-20 CST: Added because Task A splits the old valuation_status bundle into
        // independently reviewable training buckets before the next Nikkei retraining pass.
        // Purpose: lock the user-facing upper/middle/lower, high/middle/low, and
        // overbought/neutral/oversold semantics into one stable helper contract.
        assert_eq!(
            derive_bollinger_position_bucket_20d("upper_band_breakout_risk"),
            "upper"
        );
        assert_eq!(
            derive_bollinger_position_bucket_20d("lower_band_rebound_candidate"),
            "lower"
        );
        assert_eq!(derive_bollinger_position_bucket_20d("neutral"), "middle");

        assert_eq!(
            derive_range_position_bucket_14d("overbought_pullback_risk"),
            "high"
        );
        assert_eq!(
            derive_range_position_bucket_14d("oversold_rebound_candidate"),
            "low"
        );
        assert_eq!(derive_range_position_bucket_14d("neutral"), "middle");

        assert_eq!(
            derive_mean_reversion_bucket_20d("overbought_reversal_risk"),
            "overbought"
        );
        assert_eq!(
            derive_mean_reversion_bucket_20d("oversold_rebound_candidate"),
            "oversold"
        );
        assert_eq!(derive_mean_reversion_bucket_20d("neutral"), "neutral");

        assert_eq!(derive_quality_bucket("strong"), "strong");
        assert_eq!(derive_quality_bucket("fragile"), "fragile");
        assert_eq!(derive_quality_bucket("balanced"), "balanced");
    }

    #[test]
    fn derive_mean_reversion_deviation_bucket_20d_uses_atr_normalized_bands() {
        // 2026-04-20 CST: Added because the approved Nikkei route now moves away from raw
        // MA20 percentage bands toward ATR-normalized distance bands.
        // Reason: the user wants the middle bucket compressed and the weak-direction buckets
        // to remain meaningful after the 2025 volatility regime shift.
        // Purpose: lock the 0.15 ATR / 2.6 ATR bucket edges before snapshot and training consume them.
        assert!(
            (derive_mean_reversion_normalized_distance_20d(0.026, 0.01) - 2.6).abs() < 1e-9
        );
        assert!(
            (derive_mean_reversion_normalized_distance_20d(-0.013, 0.01) + 1.3).abs() < 1e-9
        );
        assert_eq!(
            derive_mean_reversion_deviation_bucket_20d(-2.61),
            "strong_down"
        );
        assert_eq!(
            derive_mean_reversion_deviation_bucket_20d(-2.60),
            "weak_down"
        );
        assert_eq!(
            derive_mean_reversion_deviation_bucket_20d(-0.16),
            "weak_down"
        );
        assert_eq!(
            derive_mean_reversion_deviation_bucket_20d(-0.15),
            "neutral"
        );
        assert_eq!(
            derive_mean_reversion_deviation_bucket_20d(0.0),
            "neutral"
        );
        assert_eq!(
            derive_mean_reversion_deviation_bucket_20d(0.15),
            "neutral"
        );
        assert_eq!(
            derive_mean_reversion_deviation_bucket_20d(0.16),
            "weak_up"
        );
        assert_eq!(
            derive_mean_reversion_deviation_bucket_20d(2.60),
            "weak_up"
        );
        assert_eq!(
            derive_mean_reversion_deviation_bucket_20d(2.61),
            "strong_up"
        );
    }
}
