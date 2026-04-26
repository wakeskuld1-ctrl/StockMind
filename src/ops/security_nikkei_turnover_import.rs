use std::collections::HashMap;
use std::fs;
use std::path::Path;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::runtime::stock_history_store::{
    StockHistoryImportSummary, StockHistoryRow, StockHistoryStore, StockHistoryStoreError,
};

const DEFAULT_TURNOVER_SOURCE: &str = "nikkei_official_total_trading_value";
const TURNOVER_SCALE: f64 = 1_000_000.0;

// 2026-04-25 CST: Added because automated Nikkei official downloads are Cloudflare-blocked
// in this environment, but the project still needs a governed receiver for manual exports.
// Purpose: parse official Total Trading Value files into an explicit turnover proxy symbol.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityNikkeiTurnoverImportRequest {
    pub source_path: String,
    pub price_symbol: String,
    pub turnover_symbol: String,
    #[serde(default = "default_turnover_source")]
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityNikkeiTurnoverImportResult {
    pub contract_version: String,
    pub document_type: String,
    pub price_symbol: String,
    pub turnover_symbol: String,
    pub source: String,
    pub imported_row_count: usize,
    pub skipped_missing_price_count: usize,
    pub database_path: String,
    pub table_name: String,
    pub date_range: SecurityNikkeiTurnoverImportDateRange,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityNikkeiTurnoverImportDateRange {
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Error)]
pub enum SecurityNikkeiTurnoverImportError {
    #[error("security nikkei turnover import build failed: {0}")]
    Build(String),
    #[error("security nikkei turnover import source read failed: {0}")]
    SourceRead(String),
    #[error("security nikkei turnover import parse failed: {0}")]
    Parse(String),
    #[error("{0}")]
    History(#[from] StockHistoryStoreError),
}

#[derive(Debug, Clone, PartialEq)]
struct TurnoverObservation {
    trade_date: String,
    turnover_trillion_yen: f64,
}

pub fn security_nikkei_turnover_import(
    request: &SecurityNikkeiTurnoverImportRequest,
) -> Result<SecurityNikkeiTurnoverImportResult, SecurityNikkeiTurnoverImportError> {
    validate_request(request)?;

    let source_text = fs::read_to_string(Path::new(&request.source_path))
        .map_err(|error| SecurityNikkeiTurnoverImportError::SourceRead(error.to_string()))?;
    let observations = parse_turnover_observations(&source_text)?;
    let store = StockHistoryStore::workspace_default()?;
    let start_date = observations
        .iter()
        .map(|observation| observation.trade_date.as_str())
        .min()
        .expect("turnover observations should not be empty");
    let end_date = observations
        .iter()
        .map(|observation| observation.trade_date.as_str())
        .max()
        .expect("turnover observations should not be empty");
    let price_rows = store.load_rows_in_range(&request.price_symbol, start_date, end_date)?;
    let price_by_date = price_rows
        .into_iter()
        .map(|row| (row.trade_date.clone(), row))
        .collect::<HashMap<_, _>>();

    let mut skipped_missing_price_count = 0usize;
    let mut import_rows = Vec::new();
    for observation in observations {
        let Some(price_row) = price_by_date.get(&observation.trade_date) else {
            skipped_missing_price_count += 1;
            continue;
        };
        import_rows.push(StockHistoryRow {
            trade_date: observation.trade_date,
            open: price_row.open,
            high: price_row.high,
            low: price_row.low,
            close: price_row.close,
            adj_close: price_row.adj_close,
            volume: (observation.turnover_trillion_yen * TURNOVER_SCALE).round() as i64,
        });
    }

    if import_rows.is_empty() {
        return Err(SecurityNikkeiTurnoverImportError::Build(
            "no turnover rows aligned with price history".to_string(),
        ));
    }

    let summary = store.import_rows(&request.turnover_symbol, &request.source, &import_rows)?;
    Ok(build_result(
        request,
        &store,
        &summary,
        skipped_missing_price_count,
    ))
}

fn default_turnover_source() -> String {
    DEFAULT_TURNOVER_SOURCE.to_string()
}

fn validate_request(
    request: &SecurityNikkeiTurnoverImportRequest,
) -> Result<(), SecurityNikkeiTurnoverImportError> {
    if request.source_path.trim().is_empty() {
        return Err(SecurityNikkeiTurnoverImportError::Build(
            "source_path cannot be empty".to_string(),
        ));
    }
    if request.price_symbol.trim().is_empty() {
        return Err(SecurityNikkeiTurnoverImportError::Build(
            "price_symbol cannot be empty".to_string(),
        ));
    }
    if request.turnover_symbol.trim().is_empty() {
        return Err(SecurityNikkeiTurnoverImportError::Build(
            "turnover_symbol cannot be empty".to_string(),
        ));
    }
    Ok(())
}

fn parse_turnover_observations(
    source_text: &str,
) -> Result<Vec<TurnoverObservation>, SecurityNikkeiTurnoverImportError> {
    let lines = source_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let Some(header_line) = lines.first() else {
        return Err(SecurityNikkeiTurnoverImportError::Parse(
            "source file is empty".to_string(),
        ));
    };
    let delimiter = detect_delimiter(header_line);
    let headers = split_delimited_line(header_line, delimiter);
    let date_index = headers
        .iter()
        .position(|header| normalize_header(header) == "date")
        .ok_or_else(|| {
            SecurityNikkeiTurnoverImportError::Parse("missing date column".to_string())
        })?;
    let turnover_index = headers
        .iter()
        .position(|header| {
            let normalized = normalize_header(header);
            normalized.contains("totaltradingvalue") || normalized.contains("tradingvalue")
        })
        .ok_or_else(|| {
            SecurityNikkeiTurnoverImportError::Parse(
                "missing Total Trading Value column".to_string(),
            )
        })?;

    let mut observations = Vec::new();
    for (line_offset, line) in lines.iter().skip(1).enumerate() {
        let line_number = line_offset + 2;
        let fields = split_delimited_line(line, delimiter);
        if fields.len() <= date_index || fields.len() <= turnover_index {
            return Err(SecurityNikkeiTurnoverImportError::Parse(format!(
                "line {line_number} has too few columns"
            )));
        }
        let trade_date = parse_official_date(fields[date_index].trim()).map_err(|message| {
            SecurityNikkeiTurnoverImportError::Parse(format!("line {line_number}: {message}"))
        })?;
        let turnover_trillion_yen =
            parse_turnover_value(fields[turnover_index].trim()).map_err(|message| {
                SecurityNikkeiTurnoverImportError::Parse(format!("line {line_number}: {message}"))
            })?;
        observations.push(TurnoverObservation {
            trade_date,
            turnover_trillion_yen,
        });
    }

    if observations.is_empty() {
        return Err(SecurityNikkeiTurnoverImportError::Parse(
            "no turnover data rows found".to_string(),
        ));
    }
    Ok(observations)
}

fn detect_delimiter(header_line: &str) -> char {
    if header_line.contains('\t') {
        '\t'
    } else {
        ','
    }
}

fn split_delimited_line(line: &str, delimiter: char) -> Vec<String> {
    line.split(delimiter)
        .map(|field| field.trim().trim_matches('"').to_string())
        .collect()
}

fn normalize_header(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

fn parse_official_date(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    for format in ["%Y-%m-%d", "%b/%d/%Y", "%b %d %Y"] {
        if let Ok(date) = NaiveDate::parse_from_str(trimmed, format) {
            return Ok(date.format("%Y-%m-%d").to_string());
        }
    }
    Err(format!("invalid official date `{trimmed}`"))
}

fn parse_turnover_value(raw: &str) -> Result<f64, String> {
    let normalized = raw.replace(',', "");
    normalized
        .parse::<f64>()
        .map_err(|error| format!("invalid turnover value `{raw}`: {error}"))
}

fn build_result(
    request: &SecurityNikkeiTurnoverImportRequest,
    store: &StockHistoryStore,
    summary: &StockHistoryImportSummary,
    skipped_missing_price_count: usize,
) -> SecurityNikkeiTurnoverImportResult {
    SecurityNikkeiTurnoverImportResult {
        contract_version: "security_nikkei_turnover_import.v1".to_string(),
        document_type: "security_nikkei_turnover_import_result".to_string(),
        price_symbol: request.price_symbol.clone(),
        turnover_symbol: request.turnover_symbol.clone(),
        source: request.source.clone(),
        imported_row_count: summary.imported_row_count,
        skipped_missing_price_count,
        database_path: store.db_path().display().to_string(),
        table_name: "stock_price_history".to_string(),
        date_range: SecurityNikkeiTurnoverImportDateRange {
            start_date: summary.start_date.clone(),
            end_date: summary.end_date.clone(),
        },
        unit: "total_trading_value_trillion_yen_scaled_1e6".to_string(),
    }
}
