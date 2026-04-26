use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::runtime::stock_history_store::{
    StockHistoryStore, StockHistoryStoreError, StockHistoryVolumeSourceSummary,
};

const DEFAULT_MINIMUM_EFFECTIVE_HISTORY_DAYS: usize = 750;

// 2026-04-25 CST: Added because Nikkei volume-source readiness must be explicit
// before further scorecard tuning uses proxy volume.
// Purpose: keep manifest inputs stable and reject implicit source guessing.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityVolumeSourceManifestRequest {
    pub instrument_symbol: String,
    pub volume_source_symbols: Vec<String>,
    #[serde(default)]
    pub as_of_date: Option<String>,
    #[serde(default)]
    pub minimum_effective_history_days: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityVolumeSourceManifestResult {
    pub contract_version: String,
    pub document_type: String,
    pub instrument_symbol: String,
    pub as_of_date: Option<String>,
    pub readiness_gates: SecurityVolumeSourceReadinessGates,
    pub volume_sources: Vec<SecurityVolumeSourceStatus>,
    pub summary: SecurityVolumeSourceManifestSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityVolumeSourceReadinessGates {
    pub minimum_effective_history_days: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityVolumeSourceStatus {
    pub symbol: String,
    pub source_names: Vec<String>,
    pub first_trade_date: Option<String>,
    pub last_trade_date: Option<String>,
    pub row_count: usize,
    pub nonzero_volume_rows: usize,
    pub zero_volume_rows: usize,
    pub nonzero_volume_ratio: f64,
    pub min_volume: Option<i64>,
    pub max_volume: Option<i64>,
    pub coverage_status: String,
    pub eligible_for_training: bool,
    pub missing_days_to_effective_gate: usize,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityVolumeSourceManifestSummary {
    pub volume_source_count: usize,
    pub train_ready_source_count: usize,
    pub usable_short_proxy_count: usize,
    pub no_volume_source_count: usize,
    pub missing_source_count: usize,
    pub has_any_usable_volume_source: bool,
    pub has_train_ready_volume_source: bool,
}

#[derive(Debug, Error)]
pub enum SecurityVolumeSourceManifestError {
    #[error("security volume source manifest build failed: {0}")]
    Build(String),
    #[error("security volume source manifest history read failed: {0}")]
    History(#[from] StockHistoryStoreError),
}

// 2026-04-25 CST: Added because the approved Scheme B needs a machine-readable
// inventory of volume sources before the model layer makes any further assumptions.
// Purpose: classify stock-history symbols as no-volume, short proxy, or train-ready proxy.
pub fn security_volume_source_manifest(
    request: &SecurityVolumeSourceManifestRequest,
) -> Result<SecurityVolumeSourceManifestResult, SecurityVolumeSourceManifestError> {
    validate_request(request)?;

    let minimum_effective_history_days = request
        .minimum_effective_history_days
        .unwrap_or(DEFAULT_MINIMUM_EFFECTIVE_HISTORY_DAYS);
    let store = StockHistoryStore::workspace_default()?;
    let mut volume_sources = Vec::new();

    for symbol in &request.volume_source_symbols {
        let trimmed_symbol = symbol.trim();
        if trimmed_symbol.is_empty() {
            continue;
        }
        let summary =
            store.load_volume_source_summary(trimmed_symbol, request.as_of_date.as_deref())?;
        volume_sources.push(build_volume_source_status(
            trimmed_symbol,
            summary,
            minimum_effective_history_days,
        ));
    }

    let summary = build_manifest_summary(&volume_sources);

    Ok(SecurityVolumeSourceManifestResult {
        contract_version: "security_volume_source_manifest.v1".to_string(),
        document_type: "security_volume_source_manifest".to_string(),
        instrument_symbol: request.instrument_symbol.trim().to_string(),
        as_of_date: request.as_of_date.clone(),
        readiness_gates: SecurityVolumeSourceReadinessGates {
            minimum_effective_history_days,
        },
        volume_sources,
        summary,
    })
}

fn validate_request(
    request: &SecurityVolumeSourceManifestRequest,
) -> Result<(), SecurityVolumeSourceManifestError> {
    if request.instrument_symbol.trim().is_empty() {
        return Err(SecurityVolumeSourceManifestError::Build(
            "instrument_symbol cannot be empty".to_string(),
        ));
    }
    if request
        .volume_source_symbols
        .iter()
        .all(|symbol| symbol.trim().is_empty())
    {
        return Err(SecurityVolumeSourceManifestError::Build(
            "volume_source_symbols cannot be empty".to_string(),
        ));
    }
    Ok(())
}

fn build_volume_source_status(
    symbol: &str,
    summary: Option<StockHistoryVolumeSourceSummary>,
    minimum_effective_history_days: usize,
) -> SecurityVolumeSourceStatus {
    let Some(summary) = summary else {
        return SecurityVolumeSourceStatus {
            symbol: symbol.to_string(),
            source_names: Vec::new(),
            first_trade_date: None,
            last_trade_date: None,
            row_count: 0,
            nonzero_volume_rows: 0,
            zero_volume_rows: 0,
            nonzero_volume_ratio: 0.0,
            min_volume: None,
            max_volume: None,
            coverage_status: "missing_history".to_string(),
            eligible_for_training: false,
            missing_days_to_effective_gate: minimum_effective_history_days,
            limitations: vec!["missing_history".to_string()],
        };
    };

    let nonzero_volume_ratio = if summary.row_count == 0 {
        0.0
    } else {
        summary.nonzero_volume_rows as f64 / summary.row_count as f64
    };
    let missing_days_to_effective_gate =
        minimum_effective_history_days.saturating_sub(summary.row_count);
    let coverage_status = if summary.nonzero_volume_rows == 0 {
        "no_volume"
    } else if summary.row_count >= minimum_effective_history_days {
        "train_ready_volume_proxy"
    } else {
        "usable_short_proxy"
    };
    let eligible_for_training = coverage_status == "train_ready_volume_proxy";
    let mut limitations = Vec::new();
    if coverage_status == "no_volume" {
        limitations.push("all_volume_values_are_zero".to_string());
    }
    if coverage_status == "usable_short_proxy" {
        limitations.push("coverage_shorter_than_minimum_effective_history_days".to_string());
    }

    SecurityVolumeSourceStatus {
        symbol: symbol.to_string(),
        source_names: summary.source_names,
        first_trade_date: Some(summary.first_trade_date),
        last_trade_date: Some(summary.last_trade_date),
        row_count: summary.row_count,
        nonzero_volume_rows: summary.nonzero_volume_rows,
        zero_volume_rows: summary.zero_volume_rows,
        nonzero_volume_ratio,
        min_volume: Some(summary.min_volume),
        max_volume: Some(summary.max_volume),
        coverage_status: coverage_status.to_string(),
        eligible_for_training,
        missing_days_to_effective_gate,
        limitations,
    }
}

fn build_manifest_summary(
    sources: &[SecurityVolumeSourceStatus],
) -> SecurityVolumeSourceManifestSummary {
    let train_ready_source_count = sources
        .iter()
        .filter(|source| source.coverage_status == "train_ready_volume_proxy")
        .count();
    let usable_short_proxy_count = sources
        .iter()
        .filter(|source| source.coverage_status == "usable_short_proxy")
        .count();
    let no_volume_source_count = sources
        .iter()
        .filter(|source| source.coverage_status == "no_volume")
        .count();
    let missing_source_count = sources
        .iter()
        .filter(|source| source.coverage_status == "missing_history")
        .count();

    SecurityVolumeSourceManifestSummary {
        volume_source_count: sources.len(),
        train_ready_source_count,
        usable_short_proxy_count,
        no_volume_source_count,
        missing_source_count,
        has_any_usable_volume_source: train_ready_source_count > 0 || usable_short_proxy_count > 0,
        has_train_ready_volume_source: train_ready_source_count > 0,
    }
}
