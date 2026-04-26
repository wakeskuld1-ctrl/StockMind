use std::collections::BTreeMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::runtime::formal_security_runtime_registry::FormalSecurityRuntimeRegistry;
use crate::runtime::security_capital_flow_store::{
    SecurityCapitalFlowRecord, SecurityCapitalFlowStore, SecurityCapitalFlowStoreError,
};

// 2026-04-25 CST: Added because training imports this standalone factor
// snapshot contract while the implementation file is absent after merge.
// Reason: capital-source observations must remain separate from the trainer until explicitly approved.
// Purpose: provide a conservative empty snapshot that preserves type and error contracts.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityCapitalSourceFactorSnapshotRequest {
    pub symbol: String,
    pub as_of_date: String,
    #[serde(default)]
    pub capital_flow_runtime_root: Option<String>,
    #[serde(default)]
    pub price_history_runtime_root: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityCapitalSourceFactorValue {
    pub value: Option<f64>,
    pub status: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityCapitalSourceFactorSnapshotResult {
    pub document_type: String,
    pub generated_at: String,
    pub symbol: String,
    pub as_of_date: String,
    pub observation_dates: BTreeMap<String, String>,
    pub factors: BTreeMap<String, SecurityCapitalSourceFactorValue>,
    pub status: String,
    pub summary: String,
}

#[derive(Debug, Error)]
pub enum SecurityCapitalSourceFactorSnapshotError {
    #[error("security capital-source factor snapshot failed: runtime path resolution failed: {0}")]
    RuntimePath(String),
    #[error("security capital-source factor snapshot failed: {0}")]
    Store(#[from] SecurityCapitalFlowStoreError),
}

pub fn security_capital_source_factor_snapshot(
    request: &SecurityCapitalSourceFactorSnapshotRequest,
) -> Result<SecurityCapitalSourceFactorSnapshotResult, SecurityCapitalSourceFactorSnapshotError> {
    let db_path = match &request.capital_flow_runtime_root {
        Some(root) => std::path::PathBuf::from(root).join("security_capital_flow.db"),
        None => FormalSecurityRuntimeRegistry::capital_flow_db_path()
            .map_err(SecurityCapitalSourceFactorSnapshotError::RuntimePath)?,
    };
    let records = SecurityCapitalFlowStore::new(db_path).load_records_until(&request.as_of_date)?;
    let observation_dates = latest_observation_dates(&records);
    let factors = build_factor_values(&records);

    Ok(SecurityCapitalSourceFactorSnapshotResult {
        document_type: "security_capital_source_factor_snapshot".to_string(),
        generated_at: Utc::now().to_rfc3339(),
        symbol: request.symbol.trim().to_string(),
        as_of_date: request.as_of_date.trim().to_string(),
        observation_dates,
        factors,
        status: "computed".to_string(),
        summary: "capital-source factor snapshot computed from governed raw weekly rows"
            .to_string(),
    })
}

fn latest_observation_dates(records: &[SecurityCapitalFlowRecord]) -> BTreeMap<String, String> {
    let mut dates = BTreeMap::new();
    for record in records {
        dates.insert(record.dataset_id.clone(), record.metric_date.clone());
    }
    dates
}

fn build_factor_values(
    records: &[SecurityCapitalFlowRecord],
) -> BTreeMap<String, SecurityCapitalSourceFactorValue> {
    let mut factors = BTreeMap::new();
    for (name, value) in [
        (
            "foreign_net_buy_ratio_1w",
            latest_series_value(records, "foreign_net_buy"),
        ),
        (
            "foreign_net_buy_ratio_wow_1w",
            series_delta(records, "foreign_net_buy", 1),
        ),
        (
            "foreign_net_buy_ratio_ma2_vs_prev2",
            moving_average_delta(records, "foreign_net_buy", 2),
        ),
        (
            "investment_trust_net_buy_ratio_wow_1w",
            series_delta(records, "investment_trust_net_buy", 1),
        ),
        (
            "mof_foreign_japan_equity_net_4w",
            trailing_sum(records, "foreign_japan_equity_net", 4),
        ),
        (
            "mof_foreign_japan_equity_net_wow_1w",
            series_delta(records, "foreign_japan_equity_net", 1),
        ),
        (
            "mof_foreign_japan_equity_net_ma2_vs_prev2",
            moving_average_delta(records, "foreign_japan_equity_net", 2),
        ),
        (
            "recent_up_move_foreign_inflow_share",
            positive_share(records, "foreign_net_buy", 4),
        ),
        (
            "recent_up_move_domestic_inflow_share",
            positive_share(records, "investment_trust_net_buy", 4),
        ),
        (
            "recent_down_move_foreign_outflow_share",
            negative_share(records, "foreign_net_buy", 4),
        ),
        (
            "recent_down_move_domestic_outflow_share",
            negative_share(records, "individual_net_buy", 4),
        ),
        (
            "overseas_flow_persistence_4w",
            positive_share(records, "foreign_japan_equity_net", 4),
        ),
        ("total_net_flow_ratio_4w", total_flow_ratio(records, 4)),
        ("total_net_flow_ratio_13w", total_flow_ratio(records, 13)),
        ("total_net_flow_ratio_26w", total_flow_ratio(records, 26)),
        ("total_net_flow_ratio_52w", total_flow_ratio(records, 52)),
        (
            "total_positive_flow_share_13w",
            total_positive_flow_share(records, 13),
        ),
        (
            "total_positive_flow_share_26w",
            total_positive_flow_share(records, 26),
        ),
        (
            "total_positive_flow_share_52w",
            total_positive_flow_share(records, 52),
        ),
        (
            "total_net_flow_ratio_13w_vs_prev13w",
            total_flow_ratio_delta(records, 13),
        ),
        (
            "total_net_flow_ratio_26w_vs_prev26w",
            total_flow_ratio_delta(records, 26),
        ),
    ] {
        factors.insert(
            name.to_string(),
            SecurityCapitalSourceFactorValue {
                value,
                status: if value.is_some() {
                    "available".to_string()
                } else {
                    "missing".to_string()
                },
                summary: format!("{name} derived from governed capital-flow rows"),
            },
        );
    }
    factors
}

fn series_values(records: &[SecurityCapitalFlowRecord], series_key: &str) -> Vec<f64> {
    records
        .iter()
        .filter(|record| record.series_key == series_key)
        .map(|record| record.value)
        .collect()
}

fn latest_series_value(records: &[SecurityCapitalFlowRecord], series_key: &str) -> Option<f64> {
    series_values(records, series_key).last().copied()
}

fn series_delta(
    records: &[SecurityCapitalFlowRecord],
    series_key: &str,
    lag: usize,
) -> Option<f64> {
    let values = series_values(records, series_key);
    if values.len() <= lag {
        return None;
    }
    Some(values[values.len() - 1] - values[values.len() - 1 - lag])
}

fn moving_average_delta(
    records: &[SecurityCapitalFlowRecord],
    series_key: &str,
    window: usize,
) -> Option<f64> {
    let values = series_values(records, series_key);
    if values.len() < window * 2 {
        return None;
    }
    let recent = average(&values[values.len() - window..]);
    let previous = average(&values[values.len() - window * 2..values.len() - window]);
    Some(recent - previous)
}

fn trailing_sum(
    records: &[SecurityCapitalFlowRecord],
    series_key: &str,
    window: usize,
) -> Option<f64> {
    let values = series_values(records, series_key);
    if values.len() < window {
        return None;
    }
    Some(values[values.len() - window..].iter().sum())
}

fn positive_share(
    records: &[SecurityCapitalFlowRecord],
    series_key: &str,
    window: usize,
) -> Option<f64> {
    let values = series_values(records, series_key);
    if values.len() < window {
        return None;
    }
    let recent = &values[values.len() - window..];
    Some(recent.iter().filter(|value| **value > 0.0).count() as f64 / window as f64)
}

fn negative_share(
    records: &[SecurityCapitalFlowRecord],
    series_key: &str,
    window: usize,
) -> Option<f64> {
    let values = series_values(records, series_key);
    if values.len() < window {
        return None;
    }
    let recent = &values[values.len() - window..];
    Some(recent.iter().filter(|value| **value < 0.0).count() as f64 / window as f64)
}

fn total_flow_ratio(records: &[SecurityCapitalFlowRecord], window: usize) -> Option<f64> {
    total_flow_ratio_for_values(&total_flow_values(records), window)
}

fn total_positive_flow_share(records: &[SecurityCapitalFlowRecord], window: usize) -> Option<f64> {
    let values = total_flow_values(records);
    if values.len() < window {
        return None;
    }
    let recent = &values[values.len() - window..];
    Some(recent.iter().filter(|value| **value > 0.0).count() as f64 / window as f64)
}

fn total_flow_ratio_delta(records: &[SecurityCapitalFlowRecord], window: usize) -> Option<f64> {
    let values = total_flow_values(records);
    if values.len() < window * 2 {
        return None;
    }
    let recent = total_flow_ratio_for_values(&values[values.len() - window..], window)?;
    let previous = total_flow_ratio_for_values(
        &values[values.len() - window * 2..values.len() - window],
        window,
    )?;
    Some(recent - previous)
}

fn total_flow_values(records: &[SecurityCapitalFlowRecord]) -> Vec<f64> {
    let mut by_date = BTreeMap::<String, f64>::new();
    for record in records {
        *by_date.entry(record.metric_date.clone()).or_insert(0.0) += record.value;
    }
    by_date.values().copied().collect::<Vec<_>>()
}

fn total_flow_ratio_for_values(values: &[f64], window: usize) -> Option<f64> {
    if values.len() < window {
        return None;
    }
    let recent = &values[values.len() - window..];
    let recent_sum: f64 = recent.iter().sum();
    let scale: f64 = recent.iter().map(|value| value.abs()).sum::<f64>().max(1.0);
    Some(recent_sum / scale)
}

fn average(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}
