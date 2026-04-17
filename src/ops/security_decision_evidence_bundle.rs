use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use thiserror::Error;

use crate::ops::stock::security_analysis_contextual::SecurityAnalysisContextualResult;
use crate::ops::stock::security_analysis_fullstack::{
    CrossBorderEtfContext, DisclosureContext, EtfContext, FundamentalContext, IndustryContext,
    FundamentalMetrics, IntegratedConclusion, SecurityAnalysisFullstackError,
    SecurityAnalysisFullstackRequest, SecurityAnalysisFullstackResult,
    disclosure_has_annual_report_notice,
    disclosure_has_buyback_or_increase_notice, disclosure_has_dividend_notice,
    disclosure_has_inquiry_notice, disclosure_has_litigation_notice,
    disclosure_has_preloss_or_loss_notice, disclosure_has_reduction_notice,
    disclosure_has_risk_warning_notice, disclosure_has_termination_notice,
    disclosure_positive_keyword_count, disclosure_risk_keyword_count, security_analysis_fullstack,
};
use crate::ops::stock::security_external_proxy_backfill::{
    load_historical_external_proxy_snapshot, load_latest_external_proxy_snapshot,
    resolve_effective_external_proxy_inputs,
};

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
    let effective_proxy_snapshot = resolve_effective_proxy_snapshot(request)?;
    // 2026-04-17 CST: Added because no-date ETF requests should inherit the latest
    // governed proxy date before the full analysis chain is built.
    // Reason: resolving proxy payloads without resolving the effective analysis date
    // still lets contextual/fullstack drift to live "today" semantics.
    // Purpose: keep technical, evidence, committee, scorecard, and chair aligned on
    // the same governed ETF anchor date.
    let effective_as_of_date = request
        .as_of_date
        .clone()
        .or_else(|| {
            if is_etf_symbol(&request.symbol) {
                effective_proxy_snapshot
                    .as_ref()
                    .map(|(resolved_date, _)| resolved_date.clone())
            } else {
                None
            }
        });
    // 2026-04-14 CST: 这里先把 dated proxy backfill 与请求级 override 合并成有效代理输入，原因是当前 ETF 兼容修补要优先恢复统一事实口径；
    // 目的：即便后续 fullstack 还没显式消费这些字段，证据层也先保留同源可追溯输入，避免继续在更上层散落处理。
    let effective_external_proxy_inputs = resolve_effective_external_proxy_inputs(
        request.symbol.trim(),
        effective_as_of_date.as_deref(),
        request.external_proxy_inputs.clone(),
    )
    .map_err(|error| SecurityDecisionEvidenceBundleError::ExternalProxy(error.to_string()))?;
    let fullstack_request = SecurityAnalysisFullstackRequest {
        symbol: request.symbol.clone(),
        market_symbol: request.market_symbol.clone(),
        sector_symbol: request.sector_symbol.clone(),
        market_profile: request.market_profile.clone(),
        sector_profile: request.sector_profile.clone(),
        as_of_date: effective_as_of_date,
        underlying_symbol: request.underlying_symbol.clone(),
        fx_symbol: request.fx_symbol.clone(),
        lookback_days: request.lookback_days,
        disclosure_limit: request.disclosure_limit,
    };
    let mut analysis = security_analysis_fullstack(&fullstack_request)?;
    hydrate_governed_etf_proxy_information(
        request,
        &mut analysis,
        effective_external_proxy_inputs.as_ref(),
    );
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
    let report_metrics = &bundle.fundamental_context.report_metrics;
    let recent_announcements = &bundle.disclosure_context.recent_announcements;
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
    // 2026-04-17 CST: Added because ETF snapshot/training consumers now require
    // the canonical technical numeric layer on the shared evidence seed.
    // Reason: the previous seed only froze textual signals, which left formal raw
    // snapshots without the numeric ETF factors the regression suite expects.
    // Purpose: keep snapshot, scorecard, and future training readers aligned on
    // one governed technical-factor contract.
    insert_optional_numeric_feature(
        &mut features,
        "close_vs_sma50",
        ratio_delta(
            stock_analysis.indicator_snapshot.close,
            stock_analysis.indicator_snapshot.sma_50,
        ),
    );
    insert_optional_numeric_feature(
        &mut features,
        "close_vs_sma200",
        ratio_delta(
            stock_analysis.indicator_snapshot.close,
            stock_analysis.indicator_snapshot.sma_200,
        ),
    );
    insert_optional_numeric_feature(
        &mut features,
        "volume_ratio_20",
        Some(stock_analysis.indicator_snapshot.volume_ratio_20),
    );
    insert_optional_numeric_feature(
        &mut features,
        "mfi_14",
        Some(stock_analysis.indicator_snapshot.mfi_14),
    );
    insert_optional_numeric_feature(
        &mut features,
        "cci_20",
        Some(stock_analysis.indicator_snapshot.cci_20),
    );
    insert_optional_numeric_feature(
        &mut features,
        "williams_r_14",
        Some(stock_analysis.indicator_snapshot.williams_r_14),
    );
    insert_optional_numeric_feature(
        &mut features,
        "boll_width_ratio_20",
        Some(stock_analysis.indicator_snapshot.boll_width_ratio_20),
    );
    insert_optional_numeric_feature(
        &mut features,
        "rsrs_zscore_18_60",
        Some(stock_analysis.indicator_snapshot.rsrs_zscore_18_60),
    );
    insert_optional_numeric_feature(
        &mut features,
        "atr_14",
        Some(stock_analysis.indicator_snapshot.atr_14),
    );
    insert_optional_numeric_feature(
        &mut features,
        "support_gap_pct_20",
        gap_to_level_pct(
            stock_analysis.indicator_snapshot.support_level_20,
            stock_analysis.indicator_snapshot.close,
        ),
    );
    insert_optional_numeric_feature(
        &mut features,
        "resistance_gap_pct_20",
        gap_to_level_pct(
            stock_analysis.indicator_snapshot.resistance_level_20,
            stock_analysis.indicator_snapshot.close,
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
        json!(bundle.disclosure_context.announcement_count),
    );
    features.insert(
        "disclosure_positive_keyword_count".to_string(),
        json!(disclosure_positive_keyword_count(recent_announcements)),
    );
    features.insert(
        "disclosure_risk_keyword_count".to_string(),
        json!(disclosure_risk_keyword_count(recent_announcements)),
    );
    features.insert(
        "has_annual_report_notice".to_string(),
        json!(disclosure_has_annual_report_notice(recent_announcements)),
    );
    features.insert(
        "has_dividend_notice".to_string(),
        json!(disclosure_has_dividend_notice(recent_announcements)),
    );
    features.insert(
        "has_buyback_or_increase_notice".to_string(),
        json!(disclosure_has_buyback_or_increase_notice(
            recent_announcements
        )),
    );
    features.insert(
        "has_reduction_notice".to_string(),
        json!(disclosure_has_reduction_notice(recent_announcements)),
    );
    features.insert(
        "has_inquiry_notice".to_string(),
        json!(disclosure_has_inquiry_notice(recent_announcements)),
    );
    features.insert(
        "has_litigation_notice".to_string(),
        json!(disclosure_has_litigation_notice(recent_announcements)),
    );
    features.insert(
        "has_termination_notice".to_string(),
        json!(disclosure_has_termination_notice(recent_announcements)),
    );
    features.insert(
        "has_risk_warning_notice".to_string(),
        json!(disclosure_has_risk_warning_notice(recent_announcements)),
    );
    features.insert(
        "has_preloss_or_loss_notice".to_string(),
        json!(disclosure_has_preloss_or_loss_notice(recent_announcements)),
    );
    features
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
        technical_context,
        fundamental_context,
        disclosure_context,
        etf_context,
        cross_border_context,
        industry_context,
        integrated_conclusion,
        ..
    } = analysis;

    let analysis_date = technical_context.analysis_date.clone();
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

// 2026-04-17 CST: Added because ETF proxy date resolution now needs to happen before
// fullstack analysis is invoked.
// Reason: otherwise the evidence layer can hydrate the right proxy payload but still
// keep a mismatched live analysis_date.
// Purpose: expose one effective snapshot resolver that works for exact-date, nearest-prior,
// and no-date latest ETF proxy requests.
fn resolve_effective_proxy_snapshot(
    request: &SecurityDecisionEvidenceBundleRequest,
) -> Result<Option<(String, SecurityExternalProxyInputs)>, SecurityDecisionEvidenceBundleError> {
    let snapshot = if let Some(as_of_date) = request.as_of_date.as_deref() {
        load_historical_external_proxy_snapshot(request.symbol.trim(), as_of_date)
    } else {
        load_latest_external_proxy_snapshot(request.symbol.trim())
    }
    .map_err(|error| SecurityDecisionEvidenceBundleError::ExternalProxy(error.to_string()))?;
    Ok(snapshot)
}

// 2026-04-17 CST: Added because ETF runtime/evidence consumers now need one normalized
// subscope vocabulary across old and new artifact labels.
// Reason: treasury/gold/equity ETF fixtures already use the newer names, while legacy
// helpers still emit bond/commodity aliases.
// Purpose: keep evidence substitution and scorecard gating aligned on one canonical label set.
pub fn normalize_etf_instrument_subscope(
    instrument_subscope: Option<&str>,
) -> Option<&'static str> {
    let normalized = instrument_subscope?.trim().to_ascii_lowercase();
    if normalized.contains("cross_border") || normalized.contains("overseas") {
        Some("cross_border_etf")
    } else if normalized.contains("treasury") || normalized.contains("bond") {
        Some("treasury_etf")
    } else if normalized.contains("gold") || normalized.contains("commodity") {
        Some("gold_etf")
    } else if normalized.contains("equity") {
        Some("equity_etf")
    } else {
        Some("equity_etf")
    }
}

// 2026-04-17 CST: Added because governed ETF proxy families should formally substitute
// for stock-style financial/disclosure contexts when those contexts are not meaningful.
// Reason: treasury and gold ETF requests currently remain degraded even when the proxy
// family is complete and auditable.
// Purpose: project complete ETF proxy evidence into the formal evidence bundle instead of
// forcing single-stock information requirements onto ETF chains.
fn hydrate_governed_etf_proxy_information(
    request: &SecurityDecisionEvidenceBundleRequest,
    analysis: &mut SecurityAnalysisFullstackResult,
    external_proxy_inputs: Option<&SecurityExternalProxyInputs>,
) {
    if !is_etf_symbol(&request.symbol) {
        return;
    }
    let Some(external_proxy_inputs) = external_proxy_inputs else {
        return;
    };
    let instrument_subscope = normalize_etf_instrument_subscope(
        resolve_etf_subscope(
            &request.symbol,
            request
                .sector_profile
                .as_deref()
                .or(request.market_profile.as_deref()),
            analysis
                .etf_context
                .asset_scope
                .as_deref()
                .or(request.sector_profile.as_deref()),
        ),
    );
    if !governed_etf_proxy_family_complete(instrument_subscope, external_proxy_inputs) {
        return;
    }

    if analysis.etf_context.status != "available" {
        analysis.etf_context = build_governed_etf_proxy_etf_context(instrument_subscope);
    }
    if analysis.fundamental_context.status != "available" {
        analysis.fundamental_context =
            build_governed_etf_proxy_fundamental_context(instrument_subscope, &analysis.analysis_date);
    }
    if analysis.disclosure_context.status != "available" {
        analysis.disclosure_context =
            build_governed_etf_proxy_disclosure_context(instrument_subscope, &analysis.analysis_date);
    }
}

// 2026-04-17 CST: Added because ETF runtime scoring still needs the ETF-wide
// differentiating family to be non-null even when public ETF facts are unavailable.
// Reason: proxy-complete treasury/gold ETF requests were still degrading to
// feature_incomplete because `etf_asset_scope` stayed null.
// Purpose: project one minimal ETF context from governed proxy completeness so the
// evidence seed exposes the required ETF-wide family consistently.
fn build_governed_etf_proxy_etf_context(instrument_subscope: Option<&str>) -> EtfContext {
    let asset_scope = Some(instrument_subscope.unwrap_or("equity_etf").to_string());
    EtfContext {
        status: "available".to_string(),
        source: "governed_etf_proxy_information".to_string(),
        fund_name: None,
        benchmark: None,
        asset_scope: asset_scope.clone(),
        latest_scale: None,
        latest_share: None,
        premium_discount_rate_pct: None,
        headline: format!(
            "Governed ETF proxy information supplied the minimum ETF context for `{}`.",
            instrument_subscope.unwrap_or("equity_etf")
        ),
        structure_risk_flags: vec![],
        research_gaps: vec![],
    }
}

// 2026-04-17 CST: Added because ETF proxy-backed evidence needs an explicit formal
// source tag once it replaces stock-only financial availability.
// Reason: silently marking the context available without a source change would make
// later audits ambiguous.
// Purpose: produce one auditable fundamental context for complete governed ETF proxy families.
fn build_governed_etf_proxy_fundamental_context(
    instrument_subscope: Option<&str>,
    analysis_date: &str,
) -> FundamentalContext {
    FundamentalContext {
        status: "available".to_string(),
        source: "governed_etf_proxy_information".to_string(),
        latest_report_period: Some(analysis_date.to_string()),
        report_notice_date: Some(analysis_date.to_string()),
        headline: format!(
            "ETF proxy-backed structural evidence is complete for `{}` on {}.",
            instrument_subscope.unwrap_or("equity_etf"),
            analysis_date
        ),
        profit_signal: "proxy_complete".to_string(),
        report_metrics: FundamentalMetrics {
            revenue: None,
            revenue_yoy_pct: None,
            net_profit: None,
            net_profit_yoy_pct: None,
            roe_pct: None,
        },
        narrative: vec![
            "Governed ETF proxy history replaced stock-only financial availability.".to_string(),
        ],
        risk_flags: vec![],
    }
}

// 2026-04-17 CST: Added because ETF proxy-backed completeness must also unblock the
// disclosure-side evidence contract for non-stock instruments.
// Reason: leaving disclosure unavailable would keep fully-governed ETF evidence
// permanently degraded even after proxy hydration succeeds.
// Purpose: expose one minimal formal disclosure context sourced from governed ETF proxies.
fn build_governed_etf_proxy_disclosure_context(
    instrument_subscope: Option<&str>,
    analysis_date: &str,
) -> DisclosureContext {
    DisclosureContext {
        status: "available".to_string(),
        source: "governed_etf_proxy_information".to_string(),
        announcement_count: 0,
        headline: format!(
            "ETF proxy-backed event surface is sufficient for `{}` on {}.",
            instrument_subscope.unwrap_or("equity_etf"),
            analysis_date
        ),
        keyword_summary: vec!["governed_etf_proxy_complete".to_string()],
        recent_announcements: vec![],
        risk_flags: vec![],
    }
}

// 2026-04-17 CST: Added because ETF proxy substitution must follow the same required
// family contract that runtime scorecard gating uses.
// Reason: otherwise evidence completeness and scoring validity could drift on the same ETF.
// Purpose: declare ETF proxy completeness only when every required family field is present.
fn governed_etf_proxy_family_complete(
    instrument_subscope: Option<&str>,
    external_proxy_inputs: &SecurityExternalProxyInputs,
) -> bool {
    let payload = serde_json::to_value(external_proxy_inputs)
        .ok()
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    required_etf_feature_family(instrument_subscope)
        .iter()
        .all(|feature_name| matches!(payload.get(*feature_name), Some(value) if !value.is_null()))
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
        || asset_scope.contains("commodity")
        || market_profile.contains("gold")
        || market_profile.contains("commodity")
    {
        Some("gold_etf")
    } else if asset_scope.contains("treasury")
        || asset_scope.contains("bond")
        || market_profile.contains("treasury")
        || market_profile.contains("bond")
    {
        Some("treasury_etf")
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
) -> String {
    let market_profile = market_profile.unwrap_or_default().to_ascii_lowercase();
    let subject_asset_class = subject_asset_class.unwrap_or_default().to_ascii_lowercase();

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
    instrument_subscope: Option<&str>,
    subject_asset_class: Option<&str>,
) -> String {
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
    if announcement_count >= 5 || disclosure_risk_keyword_count >= 3 {
        "dense".to_string()
    } else if announcement_count >= 2 || disclosure_risk_keyword_count >= 1 {
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
) -> String {
    let money_flow_signal = money_flow_signal.unwrap_or_default().to_ascii_lowercase();
    let volume_confirmation = volume_confirmation.unwrap_or_default().to_ascii_lowercase();
    let positive_flow = money_flow_signal.contains("positive")
        || money_flow_signal.contains("support")
        || money_flow_signal.contains("inflow");
    let negative_flow = money_flow_signal.contains("negative")
        || money_flow_signal.contains("pressure")
        || money_flow_signal.contains("outflow");
    let confirmed_volume = volume_confirmation.contains("confirm")
        || volume_confirmation.contains("support")
        || volume_confirmation.contains("positive");
    let weak_volume = volume_confirmation.contains("weak")
        || volume_confirmation.contains("absent")
        || volume_confirmation.contains("negative");

    if positive_flow && confirmed_volume {
        "supportive".to_string()
    } else if negative_flow && weak_volume {
        "pressured".to_string()
    } else {
        "mixed".to_string()
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

    let extended = range_position_signal.contains("high")
        || range_position_signal.contains("upper")
        || bollinger_position_signal.contains("upper")
        || mean_reversion_signal.contains("overbought");
    let compressed = range_position_signal.contains("low")
        || range_position_signal.contains("lower")
        || bollinger_position_signal.contains("lower")
        || mean_reversion_signal.contains("oversold");

    if extended {
        "extended".to_string()
    } else if compressed {
        "compressed".to_string()
    } else {
        "balanced".to_string()
    }
}

// 2026-04-14 CST: 这里补 ETF 特征族门禁函数，原因是当前 scorecard runtime 要判断不同 ETF 子池至少具备哪些特征；
// 目的：让 ETF 模型兼容门禁先恢复成显式合同，而不是在编译失败期间完全失去约束。
pub fn required_etf_feature_family(instrument_subscope: Option<&str>) -> &'static [&'static str] {
    match normalize_etf_instrument_subscope(instrument_subscope).unwrap_or("equity_etf") {
        "gold_etf" => &[
            "gold_spot_proxy_status",
            "gold_spot_proxy_return_5d",
            "real_rate_proxy_status",
            "real_rate_proxy_delta_bp_5d",
        ],
        "treasury_etf" => &[
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

// 2026-04-17 CST: Added because the canonical evidence seed now needs a shared
// ratio helper for derived technical numeric factors.
// Reason: repeating ad-hoc percentage math across snapshot and training layers
// would quickly drift once ETF factor families evolve again.
// Purpose: keep close-vs-average style factors zero-safe and contract-stable.
fn ratio_delta(numerator: f64, denominator: f64) -> Option<f64> {
    if denominator.abs() <= f64::EPSILON {
        None
    } else {
        Some((numerator - denominator) / denominator.abs())
    }
}

// 2026-04-17 CST: Added because support/resistance gap factors should share one
// stable normalization baseline across all feature consumers.
// Reason: the feature snapshot regression locks presence today, and future
// training needs the same gap semantics instead of one-off local formulas.
// Purpose: expose key-level distance features as reusable percent gaps from the
// current price anchor.
fn gap_to_level_pct(target_level: f64, anchor_price: f64) -> Option<f64> {
    if anchor_price.abs() <= f64::EPSILON {
        None
    } else {
        Some((target_level - anchor_price) / anchor_price.abs())
    }
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
    if bundle.etf_context.status != "not_applicable" {
        "etf"
    } else {
        "equity"
    }
}

fn classify_symbol_asset_class(symbol: &str) -> &'static str {
    let normalized_symbol = symbol.trim().to_uppercase();
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
