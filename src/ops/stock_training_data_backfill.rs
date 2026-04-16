use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_disclosure_history_live_backfill::{
    SecurityDisclosureHistoryLiveBackfillRequest, SecurityDisclosureHistoryLiveBackfillResult,
    security_disclosure_history_live_backfill,
};
use crate::ops::stock::security_fundamental_history_live_backfill::{
    SecurityFundamentalHistoryLiveBackfillRequest, SecurityFundamentalHistoryLiveBackfillResult,
    security_fundamental_history_live_backfill,
};
use crate::ops::stock::sync_stock_price_history::{
    SyncStockPriceHistoryRequest, SyncStockPriceHistoryResult, sync_stock_price_history,
};

// 2026-04-14 CST: Added because plan A+ needs one formal batch contract that thickens stock
// training data before scorecard retraining, instead of forcing operators to chain three tools by hand.
// Purpose: orchestrate price history plus governed financial/disclosure history in one stock-domain entrypoint.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StockTrainingDataBackfillRequest {
    pub equity_symbols: Vec<String>,
    #[serde(default)]
    pub market_symbols: Vec<String>,
    #[serde(default)]
    pub sector_symbols: Vec<String>,
    pub start_date: String,
    pub end_date: String,
    #[serde(default = "default_adjustment")]
    pub adjustment: String,
    #[serde(default = "default_sync_providers")]
    pub providers: Vec<String>,
    pub batch_id: String,
    pub created_at: String,
    #[serde(default)]
    pub history_runtime_root: Option<String>,
    #[serde(default = "default_disclosure_page_size")]
    pub disclosure_page_size: usize,
    #[serde(default = "default_disclosure_max_pages")]
    pub disclosure_max_pages: usize,
    #[serde(default = "default_true")]
    pub backfill_fundamentals: bool,
    #[serde(default = "default_true")]
    pub backfill_disclosures: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StockTrainingDataBackfillResult {
    pub contract_version: String,
    pub document_type: String,
    pub batch_ref: String,
    pub price_sync_results: Vec<SyncStockPriceHistoryResult>,
    pub fundamental_backfill_results: Vec<SecurityFundamentalHistoryLiveBackfillResult>,
    pub disclosure_backfill_results: Vec<SecurityDisclosureHistoryLiveBackfillResult>,
    pub covered_price_symbols: Vec<String>,
    pub covered_equity_symbols: Vec<String>,
    pub known_gaps: Vec<String>,
}

#[derive(Debug, Error)]
pub enum StockTrainingDataBackfillError {
    #[error("stock training data backfill build failed: {0}")]
    Build(String),
    #[error("stock training data backfill price sync failed for `{symbol}`: {message}")]
    PriceSync { symbol: String, message: String },
    #[error("stock training data backfill fundamental import failed for `{symbol}`: {message}")]
    Fundamental { symbol: String, message: String },
    #[error("stock training data backfill disclosure import failed for `{symbol}`: {message}")]
    Disclosure { symbol: String, message: String },
}

// 2026-04-14 CST: Added because scorecard retraining preparation needs one durable batch op,
// but the existing tool family already contains the required fetch/persist building blocks.
// Purpose: compose current price, financial-history, and disclosure-history tools without creating new data paths.
pub fn stock_training_data_backfill(
    request: &StockTrainingDataBackfillRequest,
) -> Result<StockTrainingDataBackfillResult, StockTrainingDataBackfillError> {
    validate_request(request)?;

    let covered_price_symbols = collect_price_symbols(request);
    let covered_equity_symbols = collect_equity_symbols(request);

    let mut price_sync_results = Vec::new();
    for symbol in &covered_price_symbols {
        let price_result = sync_stock_price_history(&SyncStockPriceHistoryRequest {
            symbol: symbol.clone(),
            start_date: request.start_date.trim().to_string(),
            end_date: request.end_date.trim().to_string(),
            adjustment: request.adjustment.trim().to_string(),
            providers: request.providers.clone(),
        })
        .map_err(|error| StockTrainingDataBackfillError::PriceSync {
            symbol: symbol.clone(),
            message: error.to_string(),
        })?;
        price_sync_results.push(price_result);
    }

    let mut fundamental_backfill_results = Vec::new();
    if request.backfill_fundamentals {
        for symbol in &covered_equity_symbols {
            let result = security_fundamental_history_live_backfill(
                &SecurityFundamentalHistoryLiveBackfillRequest {
                    symbol: symbol.clone(),
                    batch_id: build_symbol_batch_id(&request.batch_id, "fundamental", symbol),
                    created_at: request.created_at.trim().to_string(),
                    history_runtime_root: request.history_runtime_root.clone(),
                },
            )
            .map_err(|error| StockTrainingDataBackfillError::Fundamental {
                symbol: symbol.clone(),
                message: error.to_string(),
            })?;
            fundamental_backfill_results.push(result);
        }
    }

    let mut disclosure_backfill_results = Vec::new();
    if request.backfill_disclosures {
        for symbol in &covered_equity_symbols {
            let result = security_disclosure_history_live_backfill(
                &SecurityDisclosureHistoryLiveBackfillRequest {
                    symbol: symbol.clone(),
                    batch_id: build_symbol_batch_id(&request.batch_id, "disclosure", symbol),
                    created_at: request.created_at.trim().to_string(),
                    history_runtime_root: request.history_runtime_root.clone(),
                    page_size: request.disclosure_page_size,
                    max_pages: request.disclosure_max_pages,
                },
            )
            .map_err(|error| StockTrainingDataBackfillError::Disclosure {
                symbol: symbol.clone(),
                message: error.to_string(),
            })?;
            disclosure_backfill_results.push(result);
        }
    }

    Ok(StockTrainingDataBackfillResult {
        contract_version: "stock_training_data_backfill.v1".to_string(),
        document_type: "stock_training_data_backfill_result".to_string(),
        batch_ref: format!("stock-training-data-backfill:{}", request.batch_id.trim()),
        price_sync_results,
        fundamental_backfill_results,
        disclosure_backfill_results,
        covered_price_symbols,
        covered_equity_symbols,
        known_gaps: vec!["corporate_action_history_not_implemented".to_string()],
    })
}

fn validate_request(
    request: &StockTrainingDataBackfillRequest,
) -> Result<(), StockTrainingDataBackfillError> {
    if collect_price_symbols(request).is_empty() {
        return Err(StockTrainingDataBackfillError::Build(
            "at least one symbol is required".to_string(),
        ));
    }
    if request.start_date.trim().is_empty() {
        return Err(StockTrainingDataBackfillError::Build(
            "start_date cannot be empty".to_string(),
        ));
    }
    if request.end_date.trim().is_empty() {
        return Err(StockTrainingDataBackfillError::Build(
            "end_date cannot be empty".to_string(),
        ));
    }
    if request.batch_id.trim().is_empty() {
        return Err(StockTrainingDataBackfillError::Build(
            "batch_id cannot be empty".to_string(),
        ));
    }
    if request.created_at.trim().is_empty() {
        return Err(StockTrainingDataBackfillError::Build(
            "created_at cannot be empty".to_string(),
        ));
    }
    if request.providers.is_empty() {
        return Err(StockTrainingDataBackfillError::Build(
            "providers cannot be empty".to_string(),
        ));
    }

    Ok(())
}

fn collect_price_symbols(request: &StockTrainingDataBackfillRequest) -> Vec<String> {
    let mut symbols = BTreeSet::new();
    for symbol in request
        .equity_symbols
        .iter()
        .chain(request.market_symbols.iter())
        .chain(request.sector_symbols.iter())
    {
        let trimmed = symbol.trim();
        if !trimmed.is_empty() {
            symbols.insert(trimmed.to_string());
        }
    }
    symbols.into_iter().collect()
}

fn collect_equity_symbols(request: &StockTrainingDataBackfillRequest) -> Vec<String> {
    request
        .equity_symbols
        .iter()
        .map(|symbol| symbol.trim().to_string())
        .filter(|symbol| !symbol.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn build_symbol_batch_id(batch_id: &str, scope: &str, symbol: &str) -> String {
    format!(
        "{}-{}-{}",
        batch_id.trim(),
        scope,
        symbol.trim().replace('.', "_").to_lowercase()
    )
}

fn default_adjustment() -> String {
    "qfq".to_string()
}

fn default_sync_providers() -> Vec<String> {
    // 2026-04-14 CST: Updated because a real live probe on 600036.SH showed that `sina`
    // returned 1277 rows from 2021-01-04 while `tencent` only returned 640 rows from 2023-08-18.
    // Purpose: prefer the longer-history free source first so stock-first real-trading backfill
    // does not silently degrade into a short-history dataset.
    vec!["sina".to_string(), "tencent".to_string()]
}

fn default_disclosure_page_size() -> usize {
    20
}

fn default_disclosure_max_pages() -> usize {
    3
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::default_sync_providers;

    #[test]
    fn default_sync_providers_prefers_sina_before_tencent_for_long_history_backfill() {
        // 2026-04-14 CST: Added because the real live probe showed source-order materially changes
        // the recovered history length for A-share price backfill.
        // Purpose: lock the stock batch backfill default onto the longer-history-first ordering.
        assert_eq!(
            default_sync_providers(),
            vec!["sina".to_string(), "tencent".to_string()]
        );
    }
}
