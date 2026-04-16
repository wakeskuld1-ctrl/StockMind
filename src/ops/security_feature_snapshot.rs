use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::ops::stock::security_decision_evidence_bundle::{
    SecurityDecisionEvidenceBundleError, SecurityDecisionEvidenceBundleRequest,
    SecurityExternalProxyInputs, build_evidence_bundle_feature_seed, derive_event_density_bucket,
    derive_flow_status, derive_industry_bucket, derive_instrument_subscope, derive_market_regime,
    derive_valuation_status, security_decision_evidence_bundle,
};

// 2026-04-09 CST: 这里新增特征快照请求合同，原因是 Task 2 要把“分析时点可见特征冻结”独立成正式 Tool，
// 目的：让后续训练 / 回算 / 主席线都能从统一入口拿到稳定快照，而不是每次临时拼字段。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityFeatureSnapshotRequest {
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
    #[serde(default = "default_stop_loss_pct")]
    pub stop_loss_pct: f64,
    #[serde(default = "default_target_return_pct")]
    pub target_return_pct: f64,
    // 2026-04-14 CST: 这里补回 external proxy 输入透传，原因是 forward_outcome/训练链当前已经把该字段纳入请求合同；
    // 目的：让 snapshot 继续作为统一中间层消费同一份 ETF/跨市场代理输入，不再在更下游单独分叉。
    #[serde(default)]
    pub external_proxy_inputs: Option<SecurityExternalProxyInputs>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityFeatureSnapshot {
    pub snapshot_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub symbol: String,
    pub market: String,
    pub instrument_type: String,
    pub as_of_date: String,
    pub data_cutoff_at: String,
    pub feature_set_version: String,
    pub raw_features_json: BTreeMap<String, Value>,
    pub group_features_json: BTreeMap<String, Value>,
    pub data_quality_flags: Vec<String>,
    pub snapshot_hash: String,
}

#[derive(Debug, Error)]
pub enum SecurityFeatureSnapshotError {
    #[error("security feature snapshot evidence preparation failed: {0}")]
    Evidence(#[from] SecurityDecisionEvidenceBundleError),
    #[error("security feature snapshot build failed: {0}")]
    Build(String),
}

pub fn security_feature_snapshot(
    request: &SecurityFeatureSnapshotRequest,
) -> Result<SecurityFeatureSnapshot, SecurityFeatureSnapshotError> {
    let evidence_request = SecurityDecisionEvidenceBundleRequest {
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
        external_proxy_inputs: request.external_proxy_inputs.clone(),
    };
    let evidence_bundle = security_decision_evidence_bundle(&evidence_request)?;
    let raw_features_json = enrich_raw_features_json(
        request,
        build_evidence_bundle_feature_seed(&evidence_bundle),
    );
    let group_features_json = build_group_features(request, &raw_features_json);
    let data_quality_flags = build_data_quality_flags(&evidence_bundle);
    let snapshot_hash = build_snapshot_hash(
        &request.symbol,
        &evidence_bundle.analysis_date,
        &raw_features_json,
        &group_features_json,
        &data_quality_flags,
    )?;

    Ok(SecurityFeatureSnapshot {
        snapshot_id: format!(
            "snapshot-{}-{}",
            request.symbol, evidence_bundle.analysis_date
        ),
        contract_version: "security_feature_snapshot.v1".to_string(),
        document_type: "security_feature_snapshot".to_string(),
        symbol: request.symbol.clone(),
        market: derive_market(&request.symbol),
        instrument_type: derive_instrument_type(&request.symbol),
        as_of_date: evidence_bundle.analysis_date.clone(),
        data_cutoff_at: evidence_bundle.analysis_date.clone(),
        feature_set_version: "security_feature_snapshot.v1".to_string(),
        raw_features_json,
        group_features_json,
        data_quality_flags,
        snapshot_hash,
    })
}

fn build_group_features(
    request: &SecurityFeatureSnapshotRequest,
    raw_features_json: &BTreeMap<String, Value>,
) -> BTreeMap<String, Value> {
    let mut groups = BTreeMap::new();
    groups.insert("M".to_string(), json!({
        "market_profile": request.market_profile.clone().unwrap_or_else(|| "unknown".to_string()),
        "market_regime": raw_features_json.get("market_regime").cloned().unwrap_or(Value::Null),
        "industry_bucket": raw_features_json.get("industry_bucket").cloned().unwrap_or(Value::Null),
        "instrument_subscope": raw_features_json.get("instrument_subscope").cloned().unwrap_or(Value::Null),
        "integrated_stance": raw_features_json.get("integrated_stance").cloned().unwrap_or(Value::Null),
        "technical_alignment": raw_features_json.get("technical_alignment").cloned().unwrap_or(Value::Null),
        "subject_asset_class": raw_features_json.get("subject_asset_class").cloned().unwrap_or(Value::Null),
    }));
    groups.insert("F".to_string(), json!({
        "fundamental_status": raw_features_json.get("fundamental_status").cloned().unwrap_or(Value::Null),
        "fundamental_available": raw_features_json.get("fundamental_available").cloned().unwrap_or(Value::Null),
    }));
    groups.insert(
        "V".to_string(),
        json!({
            "valuation_status": raw_features_json.get("valuation_status").cloned().unwrap_or(Value::Null),
        }),
    );
    groups.insert("T".to_string(), json!({
        "technical_alignment": raw_features_json.get("technical_alignment").cloned().unwrap_or(Value::Null),
        "technical_status": raw_features_json.get("technical_status").cloned().unwrap_or(Value::Null),
    }));
    groups.insert(
        "Q".to_string(),
        json!({
            "flow_status": raw_features_json.get("flow_status").cloned().unwrap_or(Value::Null),
            "event_density_bucket": raw_features_json.get("event_density_bucket").cloned().unwrap_or(Value::Null),
        }),
    );
    groups.insert("E".to_string(), json!({
        "disclosure_status": raw_features_json.get("disclosure_status").cloned().unwrap_or(Value::Null),
        "disclosure_available": raw_features_json.get("disclosure_available").cloned().unwrap_or(Value::Null),
    }));
    groups.insert("R".to_string(), json!({
        "overall_evidence_status": raw_features_json.get("overall_evidence_status").cloned().unwrap_or(Value::Null),
        "data_gap_count": raw_features_json.get("data_gap_count").cloned().unwrap_or(Value::Null),
        "risk_note_count": raw_features_json.get("risk_note_count").cloned().unwrap_or(Value::Null),
    }));
    groups.insert(
        "X".to_string(),
        json!({
            "trading_structure_status": "etf_facts_seeded_v1",
            "etf_context_status": raw_features_json.get("etf_context_status").cloned().unwrap_or(Value::Null),
            "etf_benchmark_available": raw_features_json.get("etf_benchmark_available").cloned().unwrap_or(Value::Null),
            "etf_scale_available": raw_features_json.get("etf_scale_available").cloned().unwrap_or(Value::Null),
            "etf_structure_risk_count": raw_features_json.get("etf_structure_risk_count").cloned().unwrap_or(Value::Null),
            // 2026-04-15 CST: Added because historical proxy hydration must stay visible on the
            // formal ETF-specific group surface, not only inside raw feature storage.
            // Reason: treasury/gold/cross-border ETF regressions were reading X-group proxy fields and saw null.
            // Purpose: keep ETF family proxy evidence consistent between raw snapshot and grouped snapshot views.
            "yield_curve_proxy_status": raw_features_json.get("yield_curve_proxy_status").cloned().unwrap_or(Value::Null),
            "yield_curve_slope_delta_bp_5d": raw_features_json.get("yield_curve_slope_delta_bp_5d").cloned().unwrap_or(Value::Null),
            "funding_liquidity_proxy_status": raw_features_json.get("funding_liquidity_proxy_status").cloned().unwrap_or(Value::Null),
            "funding_liquidity_spread_delta_bp_5d": raw_features_json.get("funding_liquidity_spread_delta_bp_5d").cloned().unwrap_or(Value::Null),
            "gold_spot_proxy_status": raw_features_json.get("gold_spot_proxy_status").cloned().unwrap_or(Value::Null),
            "gold_spot_proxy_return_5d": raw_features_json.get("gold_spot_proxy_return_5d").cloned().unwrap_or(Value::Null),
            "usd_index_proxy_status": raw_features_json.get("usd_index_proxy_status").cloned().unwrap_or(Value::Null),
            "usd_index_proxy_return_5d": raw_features_json.get("usd_index_proxy_return_5d").cloned().unwrap_or(Value::Null),
            "real_rate_proxy_status": raw_features_json.get("real_rate_proxy_status").cloned().unwrap_or(Value::Null),
            "real_rate_proxy_delta_bp_5d": raw_features_json.get("real_rate_proxy_delta_bp_5d").cloned().unwrap_or(Value::Null),
            "premium_discount_proxy_status": raw_features_json.get("premium_discount_proxy_status").cloned().unwrap_or(Value::Null),
            "premium_discount_pct": raw_features_json.get("premium_discount_pct").cloned().unwrap_or(Value::Null),
            "etf_fund_flow_proxy_status": raw_features_json.get("etf_fund_flow_proxy_status").cloned().unwrap_or(Value::Null),
            "etf_fund_flow_5d": raw_features_json.get("etf_fund_flow_5d").cloned().unwrap_or(Value::Null),
            "benchmark_relative_strength_status": raw_features_json.get("benchmark_relative_strength_status").cloned().unwrap_or(Value::Null),
            "benchmark_relative_return_5d": raw_features_json.get("benchmark_relative_return_5d").cloned().unwrap_or(Value::Null),
            "cross_border_context_status": raw_features_json.get("cross_border_context_status").cloned().unwrap_or(Value::Null),
            "cross_border_analysis_method": raw_features_json.get("cross_border_analysis_method").cloned().unwrap_or(Value::Null),
            "cross_border_underlying_symbol": raw_features_json.get("cross_border_underlying_symbol").cloned().unwrap_or(Value::Null),
            "cross_border_underlying_bias": raw_features_json.get("cross_border_underlying_bias").cloned().unwrap_or(Value::Null),
            "cross_border_underlying_confidence": raw_features_json.get("cross_border_underlying_confidence").cloned().unwrap_or(Value::Null),
            "cross_border_fx_symbol": raw_features_json.get("cross_border_fx_symbol").cloned().unwrap_or(Value::Null),
            "cross_border_fx_bias": raw_features_json.get("cross_border_fx_bias").cloned().unwrap_or(Value::Null),
            "cross_border_fx_confidence": raw_features_json.get("cross_border_fx_confidence").cloned().unwrap_or(Value::Null),
            "cross_border_premium_verdict": raw_features_json.get("cross_border_premium_verdict").cloned().unwrap_or(Value::Null),
            "cross_border_resonance_verdict": raw_features_json.get("cross_border_resonance_verdict").cloned().unwrap_or(Value::Null),
            "fx_proxy_status": raw_features_json.get("fx_proxy_status").cloned().unwrap_or(Value::Null),
            "fx_return_5d": raw_features_json.get("fx_return_5d").cloned().unwrap_or(Value::Null),
            "overseas_market_proxy_status": raw_features_json.get("overseas_market_proxy_status").cloned().unwrap_or(Value::Null),
            "overseas_market_return_5d": raw_features_json.get("overseas_market_return_5d").cloned().unwrap_or(Value::Null),
            // 2026-04-15 CST: Added because cross-border ETF snapshot consumers need the
            // session-gap proxy on the formal X-group surface as well.
            // Reason: the raw snapshot already carries this governed proxy family and tests read it from X.
            // Purpose: keep grouped ETF features aligned with the canonical raw feature seed.
            "market_session_gap_status": raw_features_json.get("market_session_gap_status").cloned().unwrap_or(Value::Null),
            "market_session_gap_days": raw_features_json.get("market_session_gap_days").cloned().unwrap_or(Value::Null),
        }),
    );
    groups
}

fn build_data_quality_flags(
    evidence_bundle: &crate::ops::stock::security_decision_evidence_bundle::SecurityDecisionEvidenceBundleResult,
) -> Vec<String> {
    let mut flags = Vec::new();
    flags.push(format!(
        "overall_status:{}",
        evidence_bundle.evidence_quality.overall_status
    ));
    flags.extend(
        evidence_bundle
            .data_gaps
            .iter()
            .map(|gap| format!("data_gap:{gap}")),
    );
    flags.extend(
        evidence_bundle
            .evidence_quality
            .risk_flags
            .iter()
            .take(4)
            .map(|flag| format!("risk_flag:{flag}")),
    );
    dedupe_strings(&mut flags);
    flags
}

fn enrich_raw_features_json(
    request: &SecurityFeatureSnapshotRequest,
    mut raw_features_json: BTreeMap<String, Value>,
) -> BTreeMap<String, Value> {
    let market_profile = request
        .market_profile
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let sector_profile = request
        .sector_profile
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let subject_asset_class = raw_features_json
        .get("subject_asset_class")
        .and_then(Value::as_str);
    let instrument_subscope = derive_instrument_subscope(
        &request.symbol,
        request.market_profile.as_deref(),
        subject_asset_class,
    );
    let industry_bucket = derive_industry_bucket(
        request.sector_profile.as_deref(),
        Some(&instrument_subscope),
        subject_asset_class,
    );
    let market_regime =
        derive_market_regime(request.market_profile.as_deref(), subject_asset_class);
    let announcement_count = raw_features_json
        .get("announcement_count")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let disclosure_risk_keyword_count = raw_features_json
        .get("disclosure_risk_keyword_count")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let event_density_bucket =
        derive_event_density_bucket(announcement_count, disclosure_risk_keyword_count);
    let flow_status = derive_flow_status(
        raw_features_json
            .get("money_flow_signal")
            .and_then(Value::as_str),
        raw_features_json
            .get("volume_confirmation")
            .and_then(Value::as_str),
    );
    let valuation_status = derive_valuation_status(
        raw_features_json
            .get("range_position_signal")
            .and_then(Value::as_str),
        raw_features_json
            .get("bollinger_position_signal")
            .and_then(Value::as_str),
        raw_features_json
            .get("mean_reversion_signal")
            .and_then(Value::as_str),
    );

    // 2026-04-16 CST: Added because A-1a promotes request-side regime and industry routing into
    // the canonical raw snapshot rather than leaving them stranded in grouped views only.
    // Reason: training consumes `raw_features_json`, so keeping these fields out of the raw layer
    // would preserve the old thin-sample problem.
    // Purpose: freeze one stable source of truth for downstream training and replay consumers.
    raw_features_json.insert("market_profile".to_string(), Value::String(market_profile));
    raw_features_json.insert("sector_profile".to_string(), Value::String(sector_profile));
    raw_features_json.insert("market_regime".to_string(), Value::String(market_regime));
    raw_features_json.insert(
        "industry_bucket".to_string(),
        Value::String(industry_bucket),
    );
    raw_features_json.insert(
        "instrument_subscope".to_string(),
        Value::String(instrument_subscope),
    );
    raw_features_json.insert(
        "event_density_bucket".to_string(),
        Value::String(event_density_bucket),
    );
    raw_features_json.insert("flow_status".to_string(), Value::String(flow_status));
    raw_features_json.insert(
        "valuation_status".to_string(),
        Value::String(valuation_status),
    );
    raw_features_json
}

fn build_snapshot_hash(
    symbol: &str,
    as_of_date: &str,
    raw_features_json: &BTreeMap<String, Value>,
    group_features_json: &BTreeMap<String, Value>,
    data_quality_flags: &[String],
) -> Result<String, SecurityFeatureSnapshotError> {
    let payload = json!({
        "symbol": symbol,
        "as_of_date": as_of_date,
        "raw_features_json": raw_features_json,
        "group_features_json": group_features_json,
        "data_quality_flags": data_quality_flags,
    });
    let bytes = serde_json::to_vec(&payload)
        .map_err(|error| SecurityFeatureSnapshotError::Build(error.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("snapshot-{:x}", hasher.finalize()))
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

fn derive_market(symbol: &str) -> String {
    if symbol.ends_with(".SH") || symbol.ends_with(".SZ") {
        "A_SHARE".to_string()
    } else {
        "UNKNOWN".to_string()
    }
}

fn derive_instrument_type(symbol: &str) -> String {
    let code = symbol.split('.').next().unwrap_or_default();
    if code.starts_with('5') || code.starts_with('1') {
        "ETF".to_string()
    } else {
        "EQUITY".to_string()
    }
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
