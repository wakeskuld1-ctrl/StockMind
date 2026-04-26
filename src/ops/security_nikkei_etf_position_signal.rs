use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::runtime::stock_history_store::{
    StockHistoryRow, StockHistoryStore, StockHistoryStoreError,
};

const CONTRACT_VERSION: &str = "security_nikkei_etf_position_signal.v1";
const DOCUMENT_TYPE: &str = "security_nikkei_etf_position_signal";
const DEFAULT_MINIMUM_INDEX_HISTORY_DAYS: usize = 220;
const DEFAULT_MODEL_MODE: &str = "rule_only";
const HGB_ADJUSTMENT_CONTRACT_VERSION: &str = "nikkei_v3_hgb_adjustment.v1";

// 2026-04-26 CST: Added because the approved Nikkei ETF daily Tool needs one
// stable input contract before it can be run by operators every day.
// Purpose: keep index anchor, ETF target, model mode, and data gates explicit.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityNikkeiEtfPositionSignalRequest {
    pub as_of_date: String,
    pub instrument_symbol: String,
    pub etf_symbol: String,
    #[serde(default)]
    pub volume_proxy_symbol: Option<String>,
    #[serde(default)]
    pub model_mode: Option<String>,
    #[serde(default)]
    pub model_artifact_path: Option<String>,
    #[serde(default)]
    pub minimum_index_history_days: Option<usize>,
    #[serde(default)]
    pub component_weights_path: Option<String>,
    #[serde(default)]
    pub component_history_dir: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityNikkeiEtfPositionSignalResult {
    pub contract_version: String,
    pub document_type: String,
    pub as_of_date: String,
    pub instrument_symbol: String,
    pub etf_symbol: String,
    pub model_mode: String,
    pub market_regime: String,
    pub position_cap: f64,
    pub v3_base_position: f64,
    pub hgb_adjustment: f64,
    pub target_position: f64,
    pub entry_signal: String,
    pub exit_signal: String,
    pub breadth_signal: String,
    pub volume_signal: String,
    pub volume_metrics: NikkeiEtfPositionVolumeMetrics,
    pub reason_codes: Vec<String>,
    pub risk_flags: Vec<String>,
    pub data_coverage: NikkeiEtfPositionDataCoverage,
    pub decision_trace: NikkeiEtfPositionDecisionTrace,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NikkeiEtfPositionDataCoverage {
    pub index_rows_used: usize,
    pub latest_index_trade_date: String,
    pub component_weight_coverage_ratio: Option<f64>,
    pub component_history_coverage_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NikkeiEtfPositionDecisionTrace {
    pub latest_close: f64,
    pub moving_average_50d: f64,
    pub moving_average_200d: f64,
    pub moving_average_200d_slope: f64,
    pub rule_summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NikkeiEtfPositionVolumeMetrics {
    pub volume_proxy_symbol: Option<String>,
    pub volume_rows_used: usize,
    pub volume_ratio_3d_vs_prev20: Option<f64>,
    pub price_breakout_20d: bool,
}

#[derive(Debug, Error)]
pub enum SecurityNikkeiEtfPositionSignalError {
    #[error("security Nikkei ETF position signal failed: {0}")]
    Build(String),
    #[error("security Nikkei ETF position signal history read failed: {0}")]
    History(#[from] StockHistoryStoreError),
}

// 2026-04-26 CST: Added because the daily ETF signal must be side-effect-free
// and must not use rows after the operator-selected date.
// Purpose: compute the first governed rule-only V3 position signal while rejecting
// incomplete HGB mode instead of guessing model state.
pub fn security_nikkei_etf_position_signal(
    request: &SecurityNikkeiEtfPositionSignalRequest,
) -> Result<SecurityNikkeiEtfPositionSignalResult, SecurityNikkeiEtfPositionSignalError> {
    validate_request(request)?;
    let model_mode = normalized_model_mode(request);
    if model_mode == "v3_hgb" && is_blank_option(&request.model_artifact_path) {
        return Err(SecurityNikkeiEtfPositionSignalError::Build(
            "model_artifact_path is required when model_mode is v3_hgb".to_string(),
        ));
    }
    if model_mode != "rule_only" && model_mode != "v3_hgb" {
        return Err(SecurityNikkeiEtfPositionSignalError::Build(format!(
            "unsupported model_mode: {model_mode}"
        )));
    }

    let minimum_index_history_days = request
        .minimum_index_history_days
        .unwrap_or(DEFAULT_MINIMUM_INDEX_HISTORY_DAYS);
    let store = StockHistoryStore::workspace_default()?;
    let rows = store.load_rows_in_range(
        request.instrument_symbol.trim(),
        "0001-01-01",
        request.as_of_date.trim(),
    )?;
    if rows.len() < minimum_index_history_days {
        return Err(SecurityNikkeiEtfPositionSignalError::Build(format!(
            "insufficient index history: required {minimum_index_history_days}, found {}",
            rows.len()
        )));
    }

    let component_breadth = load_component_breadth(request)?;
    let volume_confirmation = load_volume_confirmation(request, &rows)?;
    let hgb_adjustment = load_hgb_adjustment(request, &model_mode)? as f64;
    let metrics = compute_regime_metrics(&rows)?;
    let regime = classify_regime(&metrics, component_breadth.as_ref());
    let position_cap = position_cap_for_regime(regime);
    let v3_base_position = base_position_for_regime(regime);
    let target_position = (v3_base_position + 0.25_f64 * hgb_adjustment).clamp(0.0, position_cap);
    let mut reason_codes = reason_codes_for_regime(regime);
    reason_codes.push(reason_code_for_hgb_adjustment(hgb_adjustment));
    if let Some(breadth) = &component_breadth {
        reason_codes.push(breadth.reason_code().to_string());
    }
    if let Some(volume) = &volume_confirmation {
        reason_codes.push(volume.reason_code().to_string());
    }

    Ok(SecurityNikkeiEtfPositionSignalResult {
        contract_version: CONTRACT_VERSION.to_string(),
        document_type: DOCUMENT_TYPE.to_string(),
        as_of_date: request.as_of_date.trim().to_string(),
        instrument_symbol: request.instrument_symbol.trim().to_string(),
        etf_symbol: request.etf_symbol.trim().to_string(),
        model_mode,
        market_regime: regime.to_string(),
        position_cap,
        v3_base_position,
        hgb_adjustment,
        target_position,
        entry_signal: entry_signal_for_position(target_position).to_string(),
        exit_signal: exit_signal_for_regime(regime).to_string(),
        breadth_signal: component_breadth
            .as_ref()
            .map(ComponentBreadth::signal)
            .unwrap_or("component_breadth_not_supplied")
            .to_string(),
        volume_signal: volume_confirmation
            .as_ref()
            .map(VolumeConfirmation::signal)
            .unwrap_or("volume_proxy_not_supplied")
            .to_string(),
        volume_metrics: NikkeiEtfPositionVolumeMetrics {
            volume_proxy_symbol: request
                .volume_proxy_symbol
                .as_deref()
                .map(str::trim)
                .filter(|symbol| !symbol.is_empty())
                .map(str::to_string),
            volume_rows_used: volume_confirmation
                .as_ref()
                .map(|volume| volume.volume_rows_used)
                .unwrap_or(0),
            volume_ratio_3d_vs_prev20: volume_confirmation
                .as_ref()
                .map(|volume| volume.volume_ratio_3d_vs_prev20),
            price_breakout_20d: volume_confirmation
                .as_ref()
                .map(|volume| volume.price_breakout_20d)
                .unwrap_or(false),
        },
        reason_codes,
        risk_flags: risk_flags_for_request(request),
        data_coverage: NikkeiEtfPositionDataCoverage {
            index_rows_used: rows.len(),
            latest_index_trade_date: rows
                .last()
                .map(|row| row.trade_date.clone())
                .unwrap_or_default(),
            component_weight_coverage_ratio: component_breadth
                .as_ref()
                .map(|breadth| breadth.weight_coverage_ratio),
            component_history_coverage_count: component_breadth
                .as_ref()
                .map(|breadth| breadth.covered_component_count)
                .unwrap_or(0),
        },
        decision_trace: NikkeiEtfPositionDecisionTrace {
            latest_close: metrics.latest_close,
            moving_average_50d: metrics.ma50,
            moving_average_200d: metrics.ma200,
            moving_average_200d_slope: metrics.ma200_slope,
            rule_summary: format!(
                "regime={regime}; close_vs_200d={:.6}; ma50_vs_ma200={:.6}; ma200_slope={:.6}",
                metrics.latest_close - metrics.ma200,
                metrics.ma50 - metrics.ma200,
                metrics.ma200_slope
            ),
        },
    })
}

#[derive(Debug, Clone, PartialEq)]
struct RegimeMetrics {
    latest_close: f64,
    ma50: f64,
    ma200: f64,
    ma200_slope: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct ComponentWeight {
    symbol: String,
    weight: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct ComponentBreadth {
    weighted_above_200d_ratio: f64,
    weight_coverage_ratio: f64,
    covered_component_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct VolumeConfirmation {
    volume_rows_used: usize,
    volume_ratio_3d_vs_prev20: f64,
    price_breakout_20d: bool,
}

fn validate_request(
    request: &SecurityNikkeiEtfPositionSignalRequest,
) -> Result<(), SecurityNikkeiEtfPositionSignalError> {
    if request.as_of_date.trim().is_empty() {
        return Err(SecurityNikkeiEtfPositionSignalError::Build(
            "as_of_date cannot be empty".to_string(),
        ));
    }
    NaiveDate::parse_from_str(request.as_of_date.trim(), "%Y-%m-%d").map_err(|error| {
        SecurityNikkeiEtfPositionSignalError::Build(format!(
            "as_of_date must use YYYY-MM-DD: {error}"
        ))
    })?;
    if request.instrument_symbol.trim().is_empty() {
        return Err(SecurityNikkeiEtfPositionSignalError::Build(
            "instrument_symbol cannot be empty".to_string(),
        ));
    }
    if request.etf_symbol.trim().is_empty() {
        return Err(SecurityNikkeiEtfPositionSignalError::Build(
            "etf_symbol cannot be empty".to_string(),
        ));
    }
    Ok(())
}

fn normalized_model_mode(request: &SecurityNikkeiEtfPositionSignalRequest) -> String {
    request
        .model_mode
        .as_deref()
        .unwrap_or(DEFAULT_MODEL_MODE)
        .trim()
        .to_ascii_lowercase()
}

fn is_blank_option(value: &Option<String>) -> bool {
    value.as_deref().map(str::trim).unwrap_or("").is_empty()
}

fn compute_regime_metrics(
    rows: &[StockHistoryRow],
) -> Result<RegimeMetrics, SecurityNikkeiEtfPositionSignalError> {
    if rows.len() < DEFAULT_MINIMUM_INDEX_HISTORY_DAYS {
        return Err(SecurityNikkeiEtfPositionSignalError::Build(
            "at least 220 rows are required to compute V3 regime metrics".to_string(),
        ));
    }

    let latest_close = rows.last().map(|row| row.close).ok_or_else(|| {
        SecurityNikkeiEtfPositionSignalError::Build("index history is empty".to_string())
    })?;
    let ma50 = moving_average(rows, rows.len() - 50, rows.len());
    let ma200 = moving_average(rows, rows.len() - 200, rows.len());
    let previous_ma200 = moving_average(rows, rows.len() - 220, rows.len() - 20);

    Ok(RegimeMetrics {
        latest_close,
        ma50,
        ma200,
        ma200_slope: ma200 - previous_ma200,
    })
}

fn moving_average(rows: &[StockHistoryRow], start: usize, end: usize) -> f64 {
    rows[start..end].iter().map(|row| row.close).sum::<f64>() / (end - start) as f64
}

fn classify_regime(
    metrics: &RegimeMetrics,
    component_breadth: Option<&ComponentBreadth>,
) -> &'static str {
    let component_allows_bull = component_breadth
        .map(|breadth| breadth.weighted_above_200d_ratio >= 0.55)
        .unwrap_or(true);
    let component_allows_bear = component_breadth
        .map(|breadth| breadth.weighted_above_200d_ratio <= 0.45)
        .unwrap_or(true);

    if metrics.latest_close > metrics.ma200
        && metrics.ma50 > metrics.ma200
        && metrics.ma200_slope > 0.0
        && component_allows_bull
    {
        "bull"
    } else if metrics.latest_close < metrics.ma200
        && metrics.ma50 < metrics.ma200
        && metrics.ma200_slope < 0.0
        && component_allows_bear
    {
        "bear"
    } else {
        "neutral"
    }
}

fn position_cap_for_regime(regime: &str) -> f64 {
    match regime {
        "bull" => 1.0,
        "bear" => 0.35,
        _ => 0.75,
    }
}

fn base_position_for_regime(regime: &str) -> f64 {
    match regime {
        "bull" => 1.0,
        "bear" => 0.25,
        _ => 0.5,
    }
}

fn reason_codes_for_regime(regime: &str) -> Vec<String> {
    match regime {
        "bull" => vec!["bull_regime_confirmed".to_string()],
        "bear" => vec!["bear_regime_confirmed".to_string()],
        _ => vec!["neutral_regime_confirmed".to_string()],
    }
}

fn entry_signal_for_position(target_position: f64) -> &'static str {
    if target_position >= 0.95 {
        "full_position_allowed"
    } else if target_position >= 0.5 {
        "partial_position_allowed"
    } else {
        "defensive_position_only"
    }
}

fn exit_signal_for_regime(regime: &str) -> &'static str {
    if regime == "bear" {
        "reduce_to_bear_cap"
    } else {
        "no_exit_confirmation"
    }
}

fn risk_flags_for_request(request: &SecurityNikkeiEtfPositionSignalRequest) -> Vec<String> {
    let mut flags = Vec::new();
    if is_blank_option(&request.component_weights_path)
        || is_blank_option(&request.component_history_dir)
    {
        flags.push("component_breadth_not_supplied".to_string());
    }
    if is_blank_option(&request.volume_proxy_symbol) {
        flags.push("volume_proxy_not_supplied".to_string());
    }
    flags
}

fn load_volume_confirmation(
    request: &SecurityNikkeiEtfPositionSignalRequest,
    index_rows: &[StockHistoryRow],
) -> Result<Option<VolumeConfirmation>, SecurityNikkeiEtfPositionSignalError> {
    let Some(volume_symbol) = request.volume_proxy_symbol.as_deref().map(str::trim) else {
        return Ok(None);
    };
    if volume_symbol.is_empty() {
        return Ok(None);
    }

    let store = StockHistoryStore::workspace_default()?;
    let rows = store.load_rows_in_range(volume_symbol, "0001-01-01", request.as_of_date.trim())?;
    if rows.len() < 23 {
        return Err(SecurityNikkeiEtfPositionSignalError::Build(format!(
            "insufficient volume proxy history: required 23, found {}",
            rows.len()
        )));
    }

    let last3_average = average_volume(&rows[rows.len() - 3..]);
    let previous20_average = average_volume(&rows[rows.len() - 23..rows.len() - 3]);
    let volume_ratio_3d_vs_prev20 = if previous20_average > 0.0 {
        last3_average / previous20_average
    } else {
        0.0
    };

    Ok(Some(VolumeConfirmation {
        volume_rows_used: rows.len(),
        volume_ratio_3d_vs_prev20,
        price_breakout_20d: price_breakout_20d(index_rows),
    }))
}

fn load_hgb_adjustment(
    request: &SecurityNikkeiEtfPositionSignalRequest,
    model_mode: &str,
) -> Result<i64, SecurityNikkeiEtfPositionSignalError> {
    if model_mode != "v3_hgb" {
        return Ok(0);
    }

    let artifact_path = request
        .model_artifact_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| {
            SecurityNikkeiEtfPositionSignalError::Build(
                "model_artifact_path is required when model_mode is v3_hgb".to_string(),
            )
        })?;
    let body = fs::read_to_string(artifact_path).map_err(|error| {
        SecurityNikkeiEtfPositionSignalError::Build(format!(
            "HGB adjustment artifact read failed: {error}"
        ))
    })?;
    let value = serde_json::from_str::<Value>(&body).map_err(|error| {
        SecurityNikkeiEtfPositionSignalError::Build(format!(
            "HGB adjustment artifact JSON parse failed: {error}"
        ))
    })?;

    let contract_version = value
        .get("contract_version")
        .and_then(Value::as_str)
        .unwrap_or("");
    if contract_version != HGB_ADJUSTMENT_CONTRACT_VERSION {
        return Err(SecurityNikkeiEtfPositionSignalError::Build(format!(
            "unsupported HGB adjustment artifact contract_version: {contract_version}"
        )));
    }
    let artifact_date = value
        .get("as_of_date")
        .and_then(Value::as_str)
        .unwrap_or("");
    if artifact_date != request.as_of_date.trim() {
        return Err(SecurityNikkeiEtfPositionSignalError::Build(format!(
            "HGB adjustment artifact as_of_date mismatch: expected {}, found {artifact_date}",
            request.as_of_date.trim()
        )));
    }
    let adjustment = value
        .get("adjustment")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            SecurityNikkeiEtfPositionSignalError::Build(
                "HGB adjustment artifact adjustment must be -1, 0, or 1".to_string(),
            )
        })?;
    if ![-1, 0, 1].contains(&adjustment) {
        return Err(SecurityNikkeiEtfPositionSignalError::Build(
            "HGB adjustment artifact adjustment must be -1, 0, or 1".to_string(),
        ));
    }

    Ok(adjustment)
}

fn average_volume(rows: &[StockHistoryRow]) -> f64 {
    rows.iter()
        .map(|row| {
            if row.volume > 0 {
                row.volume as f64
            } else {
                row.close
            }
        })
        .sum::<f64>()
        / rows.len() as f64
}

fn price_breakout_20d(rows: &[StockHistoryRow]) -> bool {
    if rows.len() < 21 {
        return false;
    }
    let latest_close = rows.last().map(|row| row.close).unwrap_or_default();
    let previous_20_high = rows[rows.len() - 21..rows.len() - 1]
        .iter()
        .map(|row| row.high)
        .fold(f64::MIN, f64::max);
    latest_close > previous_20_high
}

fn reason_code_for_hgb_adjustment(adjustment: f64) -> String {
    if adjustment > 0.0 {
        "hgb_adjustment_up".to_string()
    } else if adjustment < 0.0 {
        "hgb_adjustment_down".to_string()
    } else {
        "hgb_adjustment_neutral".to_string()
    }
}

fn load_component_breadth(
    request: &SecurityNikkeiEtfPositionSignalRequest,
) -> Result<Option<ComponentBreadth>, SecurityNikkeiEtfPositionSignalError> {
    let Some(weights_path) = request.component_weights_path.as_deref().map(str::trim) else {
        return Ok(None);
    };
    let Some(history_dir) = request.component_history_dir.as_deref().map(str::trim) else {
        return Ok(None);
    };
    if weights_path.is_empty() || history_dir.is_empty() {
        return Ok(None);
    }

    let weights = read_component_weights(Path::new(weights_path))?;
    if weights.is_empty() {
        return Err(SecurityNikkeiEtfPositionSignalError::Build(
            "component weights cannot be empty".to_string(),
        ));
    }

    let total_weight = weights.iter().map(|weight| weight.weight).sum::<f64>();
    if total_weight <= 0.0 {
        return Err(SecurityNikkeiEtfPositionSignalError::Build(
            "component total weight must be positive".to_string(),
        ));
    }

    let mut covered_weight = 0.0;
    let mut weighted_above_200d = 0.0;
    let mut covered_component_count = 0;
    let history_dir = Path::new(history_dir);
    for component in weights {
        let rows = read_component_history(
            &history_dir.join(format!("{}.csv", component.symbol)),
            request.as_of_date.trim(),
        )?;
        if rows.len() < 200 {
            continue;
        }
        covered_weight += component.weight;
        covered_component_count += 1;
        let ma200 = moving_average(&rows, rows.len() - 200, rows.len());
        if rows.last().map(|row| row.close).unwrap_or_default() > ma200 {
            weighted_above_200d += component.weight;
        }
    }

    if covered_component_count == 0 || covered_weight <= 0.0 {
        return Err(SecurityNikkeiEtfPositionSignalError::Build(
            "no component history has enough rows for 200D breadth".to_string(),
        ));
    }

    Ok(Some(ComponentBreadth {
        weighted_above_200d_ratio: weighted_above_200d / covered_weight,
        weight_coverage_ratio: covered_weight / total_weight,
        covered_component_count,
    }))
}

fn read_component_weights(
    path: &Path,
) -> Result<Vec<ComponentWeight>, SecurityNikkeiEtfPositionSignalError> {
    let body = fs::read_to_string(path).map_err(|error| {
        SecurityNikkeiEtfPositionSignalError::Build(format!(
            "component weights read failed: {error}"
        ))
    })?;
    let mut weights = Vec::new();
    for line in body.lines() {
        let columns = split_csv_line(line);
        if columns.len() < 2 {
            continue;
        }
        let Ok(weight) = columns[1].parse::<f64>() else {
            continue;
        };
        if !columns[0].is_empty() && weight > 0.0 {
            weights.push(ComponentWeight {
                symbol: columns[0].to_string(),
                weight,
            });
        }
    }
    Ok(weights)
}

fn read_component_history(
    path: &PathBuf,
    as_of_date: &str,
) -> Result<Vec<StockHistoryRow>, SecurityNikkeiEtfPositionSignalError> {
    let body = fs::read_to_string(path).map_err(|error| {
        SecurityNikkeiEtfPositionSignalError::Build(format!(
            "component history read failed for {}: {error}",
            path.display()
        ))
    })?;
    let mut rows = Vec::new();
    for line in body.lines() {
        let columns = split_csv_line(line);
        if columns.len() < 7 || columns[0].as_str() > as_of_date {
            continue;
        }
        let parsed = (
            columns[1].parse::<f64>(),
            columns[2].parse::<f64>(),
            columns[3].parse::<f64>(),
            columns[4].parse::<f64>(),
            columns[5].parse::<f64>(),
            columns[6].parse::<i64>(),
        );
        let (Ok(open), Ok(high), Ok(low), Ok(close), Ok(adj_close), Ok(volume)) = parsed else {
            continue;
        };
        rows.push(StockHistoryRow {
            trade_date: columns[0].to_string(),
            open,
            high,
            low,
            close,
            adj_close,
            volume,
        });
    }
    rows.sort_by(|left, right| left.trade_date.cmp(&right.trade_date));
    Ok(rows)
}

fn split_csv_line(line: &str) -> Vec<String> {
    line.split(',')
        .map(|column| column.trim().trim_matches('"').to_string())
        .collect()
}

impl ComponentBreadth {
    fn signal(&self) -> &'static str {
        if self.weighted_above_200d_ratio >= 0.55 {
            "component_bull_breadth_confirmed"
        } else if self.weighted_above_200d_ratio <= 0.45 {
            "component_bear_breadth_confirmed"
        } else {
            "component_neutral_breadth_confirmed"
        }
    }

    fn reason_code(&self) -> &'static str {
        if self.weighted_above_200d_ratio >= 0.55 {
            "component_breadth_bull_confirmed"
        } else if self.weighted_above_200d_ratio <= 0.45 {
            "component_breadth_bear_confirmed"
        } else {
            "component_breadth_neutral_confirmed"
        }
    }
}

impl VolumeConfirmation {
    fn signal(&self) -> &'static str {
        if self.price_breakout_20d && self.volume_ratio_3d_vs_prev20 >= 1.2 {
            "volume_backed_20d_breakout_confirmed"
        } else if self.volume_ratio_3d_vs_prev20 >= 1.2 {
            "volume_expansion_without_20d_breakout"
        } else if self.volume_ratio_3d_vs_prev20 >= 1.05 {
            "mild_volume_expansion"
        } else {
            "volume_not_confirmed"
        }
    }

    fn reason_code(&self) -> &'static str {
        if self.price_breakout_20d && self.volume_ratio_3d_vs_prev20 >= 1.2 {
            "volume_backed_breakout_confirmed"
        } else if self.volume_ratio_3d_vs_prev20 >= 1.2 {
            "volume_expansion_confirmed"
        } else if self.volume_ratio_3d_vs_prev20 >= 1.05 {
            "mild_volume_expansion"
        } else {
            "volume_confirmation_absent"
        }
    }
}
