use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_decision_evidence_bundle::SecurityExternalProxyInputs;
use crate::runtime::security_external_proxy_store::{
    SecurityExternalProxyRecordRow, SecurityExternalProxyStore, SecurityExternalProxyStoreError,
};
use crate::runtime::stock_history_store::{
    StockHistoryRow, StockHistoryStore, StockHistoryStoreError,
};

const DEFAULT_BUY_PREMIUM_CEILING_PCT: f64 = 0.01;
const DEFAULT_AVOID_PREMIUM_CEILING_PCT: f64 = 0.02;
const DEFAULT_BENCHMARK_RELATIVE_RETURN_FLOOR_PCT: f64 = 0.01;
const DEFAULT_FX_RETURN_FLOOR_PCT: f64 = 0.005;
const DEFAULT_OVERSEAS_MARKET_RETURN_FLOOR_PCT: f64 = 0.01;
const DEFAULT_LATEST_CASE_LIMIT: usize = 5;
const REPLAY_FORWARD_HORIZON_5D: usize = 5;
const REPLAY_FORWARD_HORIZON_10D: usize = 10;

// 2026-04-15 CST: Added because ETF trust needs a formal request contract instead of another
// ad-hoc narrative layer.
// Reason: the user explicitly asked for one object that can explain the current ETF verdict and
// replay the same rule on historical dates.
// Purpose: freeze the minimal auditable ETF trust-pack boundary on official runtime stores.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityEtfResonanceTrustPackRequest {
    pub symbol: String,
    #[serde(default)]
    pub as_of_date: Option<String>,
    pub start_date: String,
    pub end_date: String,
    #[serde(default = "default_buy_premium_ceiling_pct")]
    pub buy_premium_ceiling_pct: f64,
    #[serde(default = "default_avoid_premium_ceiling_pct")]
    pub avoid_premium_ceiling_pct: f64,
    #[serde(default = "default_benchmark_relative_return_floor_pct")]
    pub benchmark_relative_return_floor_pct: f64,
    #[serde(default = "default_fx_return_floor_pct")]
    pub fx_return_floor_pct: f64,
    #[serde(default = "default_overseas_market_return_floor_pct")]
    pub overseas_market_return_floor_pct: f64,
    #[serde(default = "default_latest_case_limit")]
    pub latest_case_limit: usize,
}

// 2026-04-15 CST: Added because the trust-pack must return one stable ETF evidence object that
// clients can display without post-processing opaque runtime rows.
// Reason: the current trust gap is not storage, it is missing product-shaped evidence.
// Purpose: surface the effective proxy snapshot that drove the current ETF resonance verdict.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct EtfResonanceCurrentProxySnapshot {
    pub symbol: String,
    pub analysis_date: String,
    pub proxy_as_of_date: String,
    pub instrument_subscope: String,
    pub premium_discount_pct: Option<f64>,
    pub benchmark_relative_return_5d: Option<f64>,
    pub fx_return_5d: Option<f64>,
    pub overseas_market_return_5d: Option<f64>,
    pub premium_discount_proxy_status: Option<String>,
    pub benchmark_relative_strength_status: Option<String>,
    pub fx_proxy_status: Option<String>,
    pub overseas_market_proxy_status: Option<String>,
}

// 2026-04-15 CST: Added because current ETF verdicts need to expose gate state and reasons in a
// customer-facing shape, not only raw proxy values.
// Reason: "why should I trust this" requires explicit pass/fail reasons.
// Purpose: make the resonance gate explainable and auditable.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct EtfResonanceVerdict {
    pub status: String,
    pub gate_passed: bool,
    pub classification: String,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct EtfResonanceAssessment {
    pub status: String,
    pub verdict: String,
    pub headline: String,
    pub reason_codes: Vec<String>,
}

// 2026-04-15 CST: Added because the trust-pack replay needs one compact summary object instead of
// forcing callers to recompute rates and averages from raw cases.
// Reason: the user asked for evidence that can support real ETF decisions, not intermediate math.
// Purpose: expose the minimum replay KPIs for "did this rule work historically".
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct EtfResonanceReplaySummary {
    pub sample_count: usize,
    pub eligible_sample_count: usize,
    pub triggered_sample_count: usize,
    pub win_rate_5d: f64,
    pub win_rate_10d: f64,
    pub avg_return_5d: f64,
    pub avg_return_10d: f64,
    pub avg_max_drawdown_10d: f64,
    pub payoff_ratio_10d: Option<f64>,
    pub expectancy_10d: f64,
}

// 2026-04-15 CST: Added because trust requires concrete historical examples, not only aggregate
// ratios.
// Reason: recent triggered cases let users inspect whether the gate fired in sensible contexts.
// Purpose: preserve the most recent replay samples that passed the ETF resonance gate.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct EtfResonanceTriggeredCase {
    pub trade_date: String,
    pub proxy_as_of_date: String,
    pub premium_discount_pct: f64,
    pub benchmark_relative_return_5d: f64,
    pub fx_return_5d: f64,
    pub overseas_market_return_5d: f64,
    pub forward_return_5d: f64,
    pub forward_return_10d: f64,
    pub max_drawdown_10d: f64,
}

// 2026-04-15 CST: Added because the new ETF trust-pack is a formal result object, not a helper
// function.
// Reason: the output must be versioned and discoverable on the public securities surface.
// Purpose: provide a stable contract for current evidence plus replay validation.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityEtfResonanceTrustPackResult {
    pub contract_version: String,
    pub document_type: String,
    pub symbol: String,
    pub current_analysis_date: String,
    pub current_proxy_snapshot: EtfResonanceCurrentProxySnapshot,
    // 2026-04-15 CST: Added because cross-border ETF trust should expose the same
    // underlying-first order that the user expects in live decision reviews.
    // Reason: a single aggregate verdict hides whether the issue came from the
    // underlying market, FX, or ETF premium layer.
    // Purpose: make the current ETF trust verdict auditable by layer.
    pub underlying_market_assessment: EtfResonanceAssessment,
    pub fx_assessment: EtfResonanceAssessment,
    pub premium_assessment: EtfResonanceAssessment,
    pub current_resonance_verdict: EtfResonanceVerdict,
    pub replay_summary: EtfResonanceReplaySummary,
    pub latest_triggered_cases: Vec<EtfResonanceTriggeredCase>,
}

#[derive(Debug, Error)]
pub enum SecurityEtfResonanceTrustPackError {
    #[error("security etf resonance trust pack history loading failed: {0}")]
    History(#[from] StockHistoryStoreError),
    #[error("security etf resonance trust pack proxy loading failed: {0}")]
    Proxy(#[from] SecurityExternalProxyStoreError),
    #[error("security etf resonance trust pack build failed: {0}")]
    Build(String),
}

#[derive(Debug, Clone)]
struct ResonanceThresholds {
    buy_premium_ceiling_pct: f64,
    avoid_premium_ceiling_pct: f64,
    benchmark_relative_return_floor_pct: f64,
    fx_return_floor_pct: f64,
    overseas_market_return_floor_pct: f64,
}

#[derive(Debug, Clone)]
struct ReplayAccumulation {
    sample_count: usize,
    eligible_sample_count: usize,
    triggered_cases: Vec<EtfResonanceTriggeredCase>,
}

// 2026-04-15 CST: Added because the user approved building the minimal credible ETF validation
// layer now, without replaying the full chair chain.
// Reason: the trust problem is "prove the rule", not "make the narrative longer".
// Purpose: compute one current ETF resonance verdict and one historical replay summary from the
// official price and proxy stores.
pub fn security_etf_resonance_trust_pack(
    request: &SecurityEtfResonanceTrustPackRequest,
) -> Result<SecurityEtfResonanceTrustPackResult, SecurityEtfResonanceTrustPackError> {
    validate_request(request)?;

    let thresholds = ResonanceThresholds {
        buy_premium_ceiling_pct: request.buy_premium_ceiling_pct,
        avoid_premium_ceiling_pct: request.avoid_premium_ceiling_pct,
        benchmark_relative_return_floor_pct: request.benchmark_relative_return_floor_pct,
        fx_return_floor_pct: request.fx_return_floor_pct,
        overseas_market_return_floor_pct: request.overseas_market_return_floor_pct,
    };

    let stock_store = StockHistoryStore::workspace_default()?;
    let proxy_store = SecurityExternalProxyStore::workspace_default()?;

    let current_analysis_date = resolve_effective_analysis_date(
        &stock_store,
        request.symbol.trim(),
        request.as_of_date.as_deref(),
    )?;
    let (
        current_proxy_snapshot,
        underlying_market_assessment,
        fx_assessment,
        premium_assessment,
        current_resonance_verdict,
    ) = build_current_snapshot_and_verdict(
        &proxy_store,
        request.symbol.trim(),
        &current_analysis_date,
        &thresholds,
    )?;
    let replay = build_replay_summary_and_cases(
        &stock_store,
        &proxy_store,
        request.symbol.trim(),
        request,
        &thresholds,
    )?;

    Ok(SecurityEtfResonanceTrustPackResult {
        contract_version: "security_etf_resonance_trust_pack.v1".to_string(),
        document_type: "security_etf_resonance_trust_pack".to_string(),
        symbol: request.symbol.trim().to_string(),
        current_analysis_date,
        current_proxy_snapshot,
        underlying_market_assessment,
        fx_assessment,
        premium_assessment,
        current_resonance_verdict,
        replay_summary: summarize_replay(&replay),
        latest_triggered_cases: replay
            .triggered_cases
            .into_iter()
            .take(request.latest_case_limit.max(1))
            .collect(),
    })
}

fn validate_request(
    request: &SecurityEtfResonanceTrustPackRequest,
) -> Result<(), SecurityEtfResonanceTrustPackError> {
    if request.symbol.trim().is_empty() {
        return Err(SecurityEtfResonanceTrustPackError::Build(
            "symbol cannot be empty".to_string(),
        ));
    }
    if request.start_date.trim().is_empty() || request.end_date.trim().is_empty() {
        return Err(SecurityEtfResonanceTrustPackError::Build(
            "start_date and end_date cannot be empty".to_string(),
        ));
    }
    if request.start_date > request.end_date {
        return Err(SecurityEtfResonanceTrustPackError::Build(
            "start_date cannot be later than end_date".to_string(),
        ));
    }
    Ok(())
}

fn resolve_effective_analysis_date(
    stock_store: &StockHistoryStore,
    symbol: &str,
    as_of_date: Option<&str>,
) -> Result<String, SecurityEtfResonanceTrustPackError> {
    let resolved = if let Some(as_of_date) = as_of_date {
        stock_store.latest_trade_date_on_or_before(symbol, as_of_date)?
    } else {
        stock_store.latest_trade_date(symbol)?
    };

    resolved.ok_or_else(|| {
        SecurityEtfResonanceTrustPackError::Build(format!("missing stock history for {symbol}"))
    })
}

fn build_current_snapshot_and_verdict(
    proxy_store: &SecurityExternalProxyStore,
    symbol: &str,
    analysis_date: &str,
    thresholds: &ResonanceThresholds,
) -> Result<
    (
        EtfResonanceCurrentProxySnapshot,
        EtfResonanceAssessment,
        EtfResonanceAssessment,
        EtfResonanceAssessment,
        EtfResonanceVerdict,
    ),
    SecurityEtfResonanceTrustPackError,
> {
    let row = proxy_store
        .load_latest_record_on_or_before(symbol, analysis_date)?
        .ok_or_else(|| {
            SecurityEtfResonanceTrustPackError::Build(format!(
                "missing dated ETF proxy snapshot for {symbol} on or before {analysis_date}"
            ))
        })?;
    let inputs = parse_external_proxy_inputs(&row)?;
    let underlying_market_assessment = build_underlying_market_assessment(&inputs, thresholds);
    let fx_assessment = build_fx_assessment(&inputs, thresholds);
    let premium_assessment = build_premium_assessment(&inputs, thresholds);
    let verdict = evaluate_resonance(&inputs, thresholds);

    Ok((
        EtfResonanceCurrentProxySnapshot {
            symbol: symbol.to_string(),
            analysis_date: analysis_date.to_string(),
            proxy_as_of_date: row.as_of_date.clone(),
            instrument_subscope: row.instrument_subscope,
            premium_discount_pct: inputs.premium_discount_pct,
            benchmark_relative_return_5d: inputs.benchmark_relative_return_5d,
            fx_return_5d: inputs.fx_return_5d,
            overseas_market_return_5d: inputs.overseas_market_return_5d,
            premium_discount_proxy_status: inputs.premium_discount_proxy_status,
            benchmark_relative_strength_status: inputs.benchmark_relative_strength_status,
            fx_proxy_status: inputs.fx_proxy_status,
            overseas_market_proxy_status: inputs.overseas_market_proxy_status,
        },
        underlying_market_assessment,
        fx_assessment,
        premium_assessment,
        verdict,
    ))
}

fn build_replay_summary_and_cases(
    stock_store: &StockHistoryStore,
    proxy_store: &SecurityExternalProxyStore,
    symbol: &str,
    request: &SecurityEtfResonanceTrustPackRequest,
    thresholds: &ResonanceThresholds,
) -> Result<ReplayAccumulation, SecurityEtfResonanceTrustPackError> {
    let rows = stock_store.load_rows_in_range(symbol, &request.start_date, &request.end_date)?;
    let mut replay = ReplayAccumulation {
        sample_count: rows.len(),
        eligible_sample_count: 0,
        triggered_cases: Vec::new(),
    };

    for row in rows {
        let Some(proxy_row) =
            proxy_store.load_latest_record_on_or_before(symbol, &row.trade_date)?
        else {
            continue;
        };
        let inputs = parse_external_proxy_inputs(&proxy_row)?;
        let future_rows =
            stock_store.load_forward_rows(symbol, &row.trade_date, REPLAY_FORWARD_HORIZON_10D)?;
        if future_rows.len() < REPLAY_FORWARD_HORIZON_10D {
            continue;
        }

        replay.eligible_sample_count += 1;
        let verdict = evaluate_resonance(&inputs, thresholds);
        if !verdict.gate_passed {
            continue;
        }

        replay.triggered_cases.push(build_triggered_case(
            &row,
            &proxy_row,
            &inputs,
            &future_rows,
        )?);
    }

    replay
        .triggered_cases
        .sort_by(|left, right| right.trade_date.cmp(&left.trade_date));
    Ok(replay)
}

fn parse_external_proxy_inputs(
    row: &SecurityExternalProxyRecordRow,
) -> Result<SecurityExternalProxyInputs, SecurityEtfResonanceTrustPackError> {
    serde_json::from_str::<SecurityExternalProxyInputs>(&row.external_proxy_inputs_json).map_err(
        |error| {
            SecurityEtfResonanceTrustPackError::Build(format!(
                "failed to parse external proxy inputs for {} at {}: {error}",
                row.symbol, row.as_of_date
            ))
        },
    )
}

// 2026-04-15 CST: Added because the user wants the trust result to state whether the
// underlying market itself is supportive, instead of only showing one aggregate ETF verdict.
// Reason: cross-border ETF decisions should be auditable in underlying-first order.
// Purpose: expose the underlying market gate as a first-class assessment object.
fn build_underlying_market_assessment(
    inputs: &SecurityExternalProxyInputs,
    thresholds: &ResonanceThresholds,
) -> EtfResonanceAssessment {
    match (
        inputs.benchmark_relative_return_5d,
        inputs.overseas_market_return_5d,
    ) {
        (Some(benchmark_relative_return_5d), Some(overseas_market_return_5d))
            if benchmark_relative_return_5d >= thresholds.benchmark_relative_return_floor_pct
                && overseas_market_return_5d >= thresholds.overseas_market_return_floor_pct =>
        {
            EtfResonanceAssessment {
                status: "available".to_string(),
                verdict: "supportive".to_string(),
                headline: "穿透标的与海外市场代理同时达到支撑阈值，当前 underlying layer 偏正向。"
                    .to_string(),
                reason_codes: vec!["underlying_market_supportive".to_string()],
            }
        }
        (Some(_), Some(_)) => EtfResonanceAssessment {
            status: "available".to_string(),
            verdict: "watch".to_string(),
            headline: "穿透标的或海外市场代理尚未同时达标，当前更适合作为观察层而非直接放行层。"
                .to_string(),
            reason_codes: vec![
                "benchmark_or_overseas_market_below_floor".to_string(),
            ],
        },
        _ => EtfResonanceAssessment {
            status: "incomplete".to_string(),
            verdict: "incomplete".to_string(),
            headline: "缺少 benchmark_relative_return_5d 或 overseas_market_return_5d，underlying layer 不完整。"
                .to_string(),
            reason_codes: vec!["missing_underlying_market_inputs".to_string()],
        },
    }
}

fn build_fx_assessment(
    inputs: &SecurityExternalProxyInputs,
    thresholds: &ResonanceThresholds,
) -> EtfResonanceAssessment {
    match inputs.fx_return_5d {
        Some(fx_return_5d) if fx_return_5d >= thresholds.fx_return_floor_pct => {
            EtfResonanceAssessment {
                status: "available".to_string(),
                verdict: "supportive".to_string(),
                headline: "FX layer 达到支撑阈值，当前汇率层没有拖累跨境 ETF 共振判断。"
                    .to_string(),
                reason_codes: vec!["fx_supportive".to_string()],
            }
        }
        Some(_) => EtfResonanceAssessment {
            status: "available".to_string(),
            verdict: "watch".to_string(),
            headline: "FX layer 尚未达到支撑阈值，当前跨境 ETF 不宜把汇率层当成明确顺风。"
                .to_string(),
            reason_codes: vec!["fx_support_below_floor".to_string()],
        },
        None => EtfResonanceAssessment {
            status: "incomplete".to_string(),
            verdict: "incomplete".to_string(),
            headline: "缺少 fx_return_5d，当前无法完成跨境 ETF 的汇率层判断。".to_string(),
            reason_codes: vec!["missing_fx_return_5d".to_string()],
        },
    }
}

fn build_premium_assessment(
    inputs: &SecurityExternalProxyInputs,
    thresholds: &ResonanceThresholds,
) -> EtfResonanceAssessment {
    match inputs.premium_discount_pct {
        Some(premium_discount_pct)
            if premium_discount_pct > thresholds.avoid_premium_ceiling_pct =>
        {
            EtfResonanceAssessment {
                status: "available".to_string(),
                verdict: "overheated".to_string(),
                headline: "ETF 折溢价已高于 avoid ceiling，规则上不支持直接追价。".to_string(),
                reason_codes: vec!["premium_above_avoid_ceiling".to_string()],
            }
        }
        Some(premium_discount_pct) if premium_discount_pct > thresholds.buy_premium_ceiling_pct => {
            EtfResonanceAssessment {
                status: "available".to_string(),
                verdict: "watch".to_string(),
                headline: "ETF 折溢价已高于 buy ceiling，当前更适合作为回踩确认后的执行对象。"
                    .to_string(),
                reason_codes: vec!["premium_above_buy_ceiling".to_string()],
            }
        }
        Some(_) => EtfResonanceAssessment {
            status: "available".to_string(),
            verdict: "favorable".to_string(),
            headline: "ETF 折溢价仍处可接受区间，premium layer 没有阻断当前共振判断。".to_string(),
            reason_codes: vec!["premium_within_buy_zone".to_string()],
        },
        None => EtfResonanceAssessment {
            status: "incomplete".to_string(),
            verdict: "incomplete".to_string(),
            headline: "缺少 premium_discount_pct，当前无法完成 ETF 映射层判断。".to_string(),
            reason_codes: vec!["missing_premium_discount_pct".to_string()],
        },
    }
}

fn evaluate_resonance(
    inputs: &SecurityExternalProxyInputs,
    thresholds: &ResonanceThresholds,
) -> EtfResonanceVerdict {
    let mut reason_codes = Vec::new();
    let premium_discount_pct = match inputs.premium_discount_pct {
        Some(value) => value,
        None => {
            reason_codes.push("missing_premium_discount_pct".to_string());
            return EtfResonanceVerdict {
                status: "insufficient_proxy_data".to_string(),
                gate_passed: false,
                classification: "incomplete".to_string(),
                reason_codes,
            };
        }
    };
    let benchmark_relative_return_5d = match inputs.benchmark_relative_return_5d {
        Some(value) => value,
        None => {
            reason_codes.push("missing_benchmark_relative_return_5d".to_string());
            return EtfResonanceVerdict {
                status: "insufficient_proxy_data".to_string(),
                gate_passed: false,
                classification: "incomplete".to_string(),
                reason_codes,
            };
        }
    };
    let fx_return_5d = match inputs.fx_return_5d {
        Some(value) => value,
        None => {
            reason_codes.push("missing_fx_return_5d".to_string());
            return EtfResonanceVerdict {
                status: "insufficient_proxy_data".to_string(),
                gate_passed: false,
                classification: "incomplete".to_string(),
                reason_codes,
            };
        }
    };
    let overseas_market_return_5d = match inputs.overseas_market_return_5d {
        Some(value) => value,
        None => {
            reason_codes.push("missing_overseas_market_return_5d".to_string());
            return EtfResonanceVerdict {
                status: "insufficient_proxy_data".to_string(),
                gate_passed: false,
                classification: "incomplete".to_string(),
                reason_codes,
            };
        }
    };

    if premium_discount_pct > thresholds.avoid_premium_ceiling_pct {
        reason_codes.push("premium_above_avoid_ceiling".to_string());
        return EtfResonanceVerdict {
            status: "blocked".to_string(),
            gate_passed: false,
            classification: "avoid_high_premium".to_string(),
            reason_codes,
        };
    }
    if premium_discount_pct > thresholds.buy_premium_ceiling_pct {
        reason_codes.push("premium_above_buy_ceiling".to_string());
    }
    if benchmark_relative_return_5d < thresholds.benchmark_relative_return_floor_pct {
        reason_codes.push("benchmark_relative_strength_below_floor".to_string());
    }
    if fx_return_5d < thresholds.fx_return_floor_pct {
        reason_codes.push("fx_support_below_floor".to_string());
    }
    if overseas_market_return_5d < thresholds.overseas_market_return_floor_pct {
        reason_codes.push("overseas_market_support_below_floor".to_string());
    }

    if reason_codes.is_empty() {
        EtfResonanceVerdict {
            status: "triggered".to_string(),
            gate_passed: true,
            classification: "buy_zone_resonance".to_string(),
            reason_codes: vec!["all_resonance_gates_passed".to_string()],
        }
    } else {
        EtfResonanceVerdict {
            status: "not_triggered".to_string(),
            gate_passed: false,
            classification: "watch_not_ready".to_string(),
            reason_codes,
        }
    }
}

fn build_triggered_case(
    entry_row: &StockHistoryRow,
    proxy_row: &SecurityExternalProxyRecordRow,
    inputs: &SecurityExternalProxyInputs,
    future_rows: &[StockHistoryRow],
) -> Result<EtfResonanceTriggeredCase, SecurityEtfResonanceTrustPackError> {
    let entry_price = entry_row.adj_close;
    if entry_price <= 0.0 {
        return Err(SecurityEtfResonanceTrustPackError::Build(format!(
            "entry price must be positive for {} at {}",
            proxy_row.symbol, entry_row.trade_date
        )));
    }

    let return_5d = compute_forward_return(entry_price, &future_rows[..REPLAY_FORWARD_HORIZON_5D])?;
    let return_10d =
        compute_forward_return(entry_price, &future_rows[..REPLAY_FORWARD_HORIZON_10D])?;
    let max_drawdown_10d =
        compute_max_drawdown(entry_price, &future_rows[..REPLAY_FORWARD_HORIZON_10D]);

    Ok(EtfResonanceTriggeredCase {
        trade_date: entry_row.trade_date.clone(),
        proxy_as_of_date: proxy_row.as_of_date.clone(),
        premium_discount_pct: inputs.premium_discount_pct.unwrap_or_default(),
        benchmark_relative_return_5d: inputs.benchmark_relative_return_5d.unwrap_or_default(),
        fx_return_5d: inputs.fx_return_5d.unwrap_or_default(),
        overseas_market_return_5d: inputs.overseas_market_return_5d.unwrap_or_default(),
        forward_return_5d: return_5d,
        forward_return_10d: return_10d,
        max_drawdown_10d,
    })
}

fn compute_forward_return(
    entry_price: f64,
    future_rows: &[StockHistoryRow],
) -> Result<f64, SecurityEtfResonanceTrustPackError> {
    let final_row = future_rows.last().ok_or_else(|| {
        SecurityEtfResonanceTrustPackError::Build(
            "missing future rows for ETF trust-pack replay".to_string(),
        )
    })?;
    Ok(final_row.adj_close / entry_price - 1.0)
}

fn compute_max_drawdown(entry_price: f64, future_rows: &[StockHistoryRow]) -> f64 {
    let mut running_peak = entry_price;
    let mut max_drawdown = 0.0_f64;

    for row in future_rows {
        if row.adj_close > running_peak {
            running_peak = row.adj_close;
        }
        let drawdown = 1.0 - row.adj_close / running_peak;
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
        }
    }

    max_drawdown
}

fn summarize_replay(replay: &ReplayAccumulation) -> EtfResonanceReplaySummary {
    let triggered_sample_count = replay.triggered_cases.len();
    let win_count_5d = replay
        .triggered_cases
        .iter()
        .filter(|case_item| case_item.forward_return_5d > 0.0)
        .count();
    let win_count_10d = replay
        .triggered_cases
        .iter()
        .filter(|case_item| case_item.forward_return_10d > 0.0)
        .count();
    let avg_return_5d = average(
        replay
            .triggered_cases
            .iter()
            .map(|case_item| case_item.forward_return_5d)
            .collect(),
    );
    let avg_return_10d = average(
        replay
            .triggered_cases
            .iter()
            .map(|case_item| case_item.forward_return_10d)
            .collect(),
    );
    let avg_max_drawdown_10d = average(
        replay
            .triggered_cases
            .iter()
            .map(|case_item| case_item.max_drawdown_10d)
            .collect(),
    );
    let wins_10d: Vec<f64> = replay
        .triggered_cases
        .iter()
        .filter(|case_item| case_item.forward_return_10d > 0.0)
        .map(|case_item| case_item.forward_return_10d)
        .collect();
    let losses_10d: Vec<f64> = replay
        .triggered_cases
        .iter()
        .filter(|case_item| case_item.forward_return_10d <= 0.0)
        .map(|case_item| case_item.forward_return_10d.abs())
        .collect();
    let avg_win_10d = average(wins_10d.clone());
    let avg_loss_10d = average(losses_10d.clone());
    let win_rate_10d = ratio(win_count_10d, triggered_sample_count);
    let loss_rate_10d = 1.0 - win_rate_10d;

    EtfResonanceReplaySummary {
        sample_count: replay.sample_count,
        eligible_sample_count: replay.eligible_sample_count,
        triggered_sample_count,
        win_rate_5d: ratio(win_count_5d, triggered_sample_count),
        win_rate_10d,
        avg_return_5d,
        avg_return_10d,
        avg_max_drawdown_10d,
        payoff_ratio_10d: if wins_10d.is_empty() || losses_10d.is_empty() {
            None
        } else {
            Some(avg_win_10d / avg_loss_10d)
        },
        expectancy_10d: win_rate_10d * avg_win_10d - loss_rate_10d * avg_loss_10d,
    }
}

fn average(values: Vec<f64>) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn default_buy_premium_ceiling_pct() -> f64 {
    DEFAULT_BUY_PREMIUM_CEILING_PCT
}

fn default_avoid_premium_ceiling_pct() -> f64 {
    DEFAULT_AVOID_PREMIUM_CEILING_PCT
}

fn default_benchmark_relative_return_floor_pct() -> f64 {
    DEFAULT_BENCHMARK_RELATIVE_RETURN_FLOOR_PCT
}

fn default_fx_return_floor_pct() -> f64 {
    DEFAULT_FX_RETURN_FLOOR_PCT
}

fn default_overseas_market_return_floor_pct() -> f64 {
    DEFAULT_OVERSEAS_MARKET_RETURN_FLOOR_PCT
}

fn default_latest_case_limit() -> usize {
    DEFAULT_LATEST_CASE_LIMIT
}
