use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::runtime::stock_history_store::{StockHistoryCoverageSummary, StockHistoryStore};

// 2026-04-14 CST: Added because real-trading stock readiness now needs one formal audit contract
// that says which symbols are actually train-ready after backfill, instead of only saying data was fetched.
// Purpose: read the governed stock pool config plus official runtime history store and emit one coverage verdict.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StockTrainingDataCoverageAuditRequest {
    pub pool_config_path: String,
    #[serde(default)]
    pub as_of_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StockTrainingDataCoverageAuditResult {
    pub contract_version: String,
    pub document_type: String,
    pub pool_version: String,
    pub market_scope: String,
    pub instrument_scope: String,
    pub as_of_date: Option<String>,
    pub readiness_gates: StockTrainingCoverageGates,
    pub symbol_coverage: Vec<StockSymbolCoverageStatus>,
    pub summary: StockTrainingCoverageSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StockTrainingCoverageGates {
    pub minimum_symbol_count: usize,
    pub minimum_industry_bucket_count: usize,
    pub minimum_effective_history_days_per_symbol: usize,
    pub hard_floor_history_days_per_symbol: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StockSymbolCoverageStatus {
    pub symbol: String,
    pub pool_id: String,
    pub sector_proxy_symbol: String,
    pub first_trade_date: Option<String>,
    pub last_trade_date: Option<String>,
    pub history_days: usize,
    pub meets_effective_history_gate: bool,
    pub meets_hard_floor_history_gate: bool,
    pub eligible_for_training: bool,
    pub missing_days_to_effective_gate: usize,
    pub missing_days_to_hard_floor_gate: usize,
    pub coverage_status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StockTrainingCoverageSummary {
    pub total_symbols: usize,
    pub symbols_with_any_history: usize,
    pub training_ready_symbols: usize,
    pub hard_floor_pass_symbols: usize,
    pub missing_history_symbols: usize,
    pub effective_gate_passed: bool,
    pub industry_bucket_count: usize,
    pub industry_gate_passed: bool,
    pub training_pool_ready: bool,
}

#[derive(Debug, Error)]
pub enum StockTrainingDataCoverageAuditError {
    #[error("stock training data coverage audit build failed: {0}")]
    Build(String),
    #[error("stock training data coverage audit config read failed: {0}")]
    ConfigRead(String),
    #[error("stock training data coverage audit config parse failed: {0}")]
    ConfigParse(String),
    #[error("stock training data coverage audit history read failed: {0}")]
    HistoryRead(String),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct StockPoolConfig {
    meta: StockPoolMeta,
    market_scope: String,
    instrument_scope: String,
    readiness_gates: StockPoolReadinessGates,
    pools: Vec<StockPoolDefinition>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct StockPoolMeta {
    version: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct StockPoolReadinessGates {
    minimum_symbol_count: usize,
    minimum_industry_bucket_count: usize,
    minimum_effective_history_days_per_symbol: usize,
    hard_floor_history_days_per_symbol: usize,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct StockPoolDefinition {
    pool_id: String,
    sector_proxy_symbol: String,
    equity_symbols: Vec<String>,
}

// 2026-04-14 CST: Added because the user explicitly asked to stop reporting "engineering-ready"
// and instead answer whether the current stock pool is usable for the next real training round.
// Purpose: convert frozen pool config plus official history coverage into one formal readiness verdict.
pub fn stock_training_data_coverage_audit(
    request: &StockTrainingDataCoverageAuditRequest,
) -> Result<StockTrainingDataCoverageAuditResult, StockTrainingDataCoverageAuditError> {
    validate_request(request)?;

    let config = load_pool_config(&request.pool_config_path)?;
    let store = StockHistoryStore::workspace_default()
        .map_err(|error| StockTrainingDataCoverageAuditError::HistoryRead(error.to_string()))?;

    let mut symbol_coverage = Vec::new();
    for pool in &config.pools {
        for symbol in &pool.equity_symbols {
            let trimmed_symbol = symbol.trim();
            if trimmed_symbol.is_empty() {
                continue;
            }

            let coverage = store
                .load_coverage_summary(trimmed_symbol, request.as_of_date.as_deref())
                .map_err(|error| {
                    StockTrainingDataCoverageAuditError::HistoryRead(error.to_string())
                })?;

            symbol_coverage.push(build_symbol_coverage_status(
                trimmed_symbol,
                pool,
                &config.readiness_gates,
                coverage,
            ));
        }
    }

    let summary = build_coverage_summary(&symbol_coverage, &config.readiness_gates);

    Ok(StockTrainingDataCoverageAuditResult {
        contract_version: "stock_training_data_coverage_audit.v1".to_string(),
        document_type: "stock_training_data_coverage_audit_result".to_string(),
        pool_version: config.meta.version,
        market_scope: config.market_scope,
        instrument_scope: config.instrument_scope,
        as_of_date: request.as_of_date.clone(),
        readiness_gates: StockTrainingCoverageGates {
            minimum_symbol_count: config.readiness_gates.minimum_symbol_count,
            minimum_industry_bucket_count: config.readiness_gates.minimum_industry_bucket_count,
            minimum_effective_history_days_per_symbol: config
                .readiness_gates
                .minimum_effective_history_days_per_symbol,
            hard_floor_history_days_per_symbol: config
                .readiness_gates
                .hard_floor_history_days_per_symbol,
        },
        symbol_coverage,
        summary,
    })
}

fn validate_request(
    request: &StockTrainingDataCoverageAuditRequest,
) -> Result<(), StockTrainingDataCoverageAuditError> {
    if request.pool_config_path.trim().is_empty() {
        return Err(StockTrainingDataCoverageAuditError::Build(
            "pool_config_path cannot be empty".to_string(),
        ));
    }
    Ok(())
}

fn load_pool_config(
    pool_config_path: &str,
) -> Result<StockPoolConfig, StockTrainingDataCoverageAuditError> {
    let config_path = Path::new(pool_config_path);
    let config_text = fs::read_to_string(config_path)
        .map_err(|error| StockTrainingDataCoverageAuditError::ConfigRead(error.to_string()))?;
    serde_json::from_str::<StockPoolConfig>(&config_text)
        .map_err(|error| StockTrainingDataCoverageAuditError::ConfigParse(error.to_string()))
}

fn build_symbol_coverage_status(
    symbol: &str,
    pool: &StockPoolDefinition,
    readiness_gates: &StockPoolReadinessGates,
    coverage: Option<StockHistoryCoverageSummary>,
) -> StockSymbolCoverageStatus {
    let (first_trade_date, last_trade_date, history_days) = match coverage {
        Some(coverage) => (
            Some(coverage.first_trade_date),
            Some(coverage.last_trade_date),
            coverage.history_days,
        ),
        None => (None, None, 0),
    };
    let meets_effective_history_gate =
        history_days >= readiness_gates.minimum_effective_history_days_per_symbol;
    let meets_hard_floor_history_gate =
        history_days >= readiness_gates.hard_floor_history_days_per_symbol;
    let eligible_for_training = meets_effective_history_gate;
    let missing_days_to_effective_gate = readiness_gates
        .minimum_effective_history_days_per_symbol
        .saturating_sub(history_days);
    let missing_days_to_hard_floor_gate = readiness_gates
        .hard_floor_history_days_per_symbol
        .saturating_sub(history_days);

    let coverage_status = if history_days == 0 {
        "missing_history"
    } else if eligible_for_training {
        "train_ready"
    } else if meets_hard_floor_history_gate {
        "backfill_needed"
    } else {
        "blocked_below_hard_floor"
    };

    StockSymbolCoverageStatus {
        symbol: symbol.to_string(),
        pool_id: pool.pool_id.clone(),
        sector_proxy_symbol: pool.sector_proxy_symbol.clone(),
        first_trade_date,
        last_trade_date,
        history_days,
        meets_effective_history_gate,
        meets_hard_floor_history_gate,
        eligible_for_training,
        missing_days_to_effective_gate,
        missing_days_to_hard_floor_gate,
        coverage_status: coverage_status.to_string(),
    }
}

fn build_coverage_summary(
    symbol_coverage: &[StockSymbolCoverageStatus],
    readiness_gates: &StockPoolReadinessGates,
) -> StockTrainingCoverageSummary {
    let training_ready_symbols = symbol_coverage
        .iter()
        .filter(|item| item.eligible_for_training)
        .count();
    let hard_floor_pass_symbols = symbol_coverage
        .iter()
        .filter(|item| item.meets_hard_floor_history_gate)
        .count();
    let symbols_with_any_history = symbol_coverage
        .iter()
        .filter(|item| item.history_days > 0)
        .count();
    let missing_history_symbols = symbol_coverage
        .iter()
        .filter(|item| item.history_days == 0)
        .count();
    let industry_bucket_count = symbol_coverage
        .iter()
        .filter(|item| item.eligible_for_training)
        .map(|item| item.pool_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let effective_gate_passed = training_ready_symbols >= readiness_gates.minimum_symbol_count;
    let industry_gate_passed =
        industry_bucket_count >= readiness_gates.minimum_industry_bucket_count;

    StockTrainingCoverageSummary {
        total_symbols: symbol_coverage.len(),
        symbols_with_any_history,
        training_ready_symbols,
        hard_floor_pass_symbols,
        missing_history_symbols,
        effective_gate_passed,
        industry_bucket_count,
        industry_gate_passed,
        training_pool_ready: effective_gate_passed && industry_gate_passed,
    }
}
