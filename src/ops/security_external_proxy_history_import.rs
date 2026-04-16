use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::ops::stock::security_decision_evidence_bundle::SecurityExternalProxyInputs;
use crate::ops::stock::security_external_proxy_backfill::{
    SecurityExternalProxyBackfillError, SecurityExternalProxyBackfillRecordInput,
    SecurityExternalProxyBackfillRequest, security_external_proxy_backfill,
};

// 2026-04-12 CST: Add a file-based proxy-history import request, because Historical Data
// Phase 1 needs one governed bridge for real ETF proxy batches before dedicated live
// crawlers are hardened.
// Purpose: let operators import dated proxy history from CSV/JSON through the stock tool chain.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityExternalProxyHistoryImportRequest {
    pub batch_id: String,
    pub created_at: String,
    pub file_path: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityExternalProxyHistoryImportResult {
    pub contract_version: String,
    pub document_type: String,
    pub source_file_path: String,
    pub imported_record_count: usize,
    pub covered_symbol_count: usize,
    pub coverage_tier: String,
    pub covered_dates: Vec<String>,
    pub covered_proxy_fields: Vec<String>,
    pub storage_path: String,
    pub backfill_result_path: String,
}

#[derive(Debug, Error)]
pub enum SecurityExternalProxyHistoryImportError {
    #[error("security external proxy history import build failed: {0}")]
    Build(String),
    #[error("security external proxy history import persist failed: {0}")]
    Persist(#[from] SecurityExternalProxyBackfillError),
}

// 2026-04-12 CST: Import governed dated proxy rows from a real file, because ETF external
// proxy history needs one auditable ingestion path before stronger live providers land.
// Purpose: create a formal file-to-governed bridge for treasury/gold/cross-border/equity proxy history.
pub fn security_external_proxy_history_import(
    request: &SecurityExternalProxyHistoryImportRequest,
) -> Result<SecurityExternalProxyHistoryImportResult, SecurityExternalProxyHistoryImportError> {
    validate_request(request)?;

    let records = parse_proxy_history_file(Path::new(request.file_path.trim()))?;
    let persisted = security_external_proxy_backfill(&SecurityExternalProxyBackfillRequest {
        batch_id: request.batch_id.trim().to_string(),
        created_at: request.created_at.trim().to_string(),
        records,
    })?;

    Ok(SecurityExternalProxyHistoryImportResult {
        contract_version: "security_external_proxy_history_import.v1".to_string(),
        document_type: "security_external_proxy_history_import_result".to_string(),
        source_file_path: request.file_path.trim().to_string(),
        imported_record_count: persisted.imported_record_count,
        covered_symbol_count: persisted.covered_symbol_count,
        coverage_tier: persisted.coverage_tier,
        covered_dates: persisted.covered_dates,
        covered_proxy_fields: persisted.covered_proxy_fields,
        storage_path: persisted.storage_path,
        backfill_result_path: persisted.backfill_result_path,
    })
}

// 2026-04-12 CST: Keep request validation local, because real-file imports should fail
// before any storage side effect when file coordinates or batch identifiers are incomplete.
// Purpose: preserve deterministic operator semantics for governed proxy-history imports.
fn validate_request(
    request: &SecurityExternalProxyHistoryImportRequest,
) -> Result<(), SecurityExternalProxyHistoryImportError> {
    if request.batch_id.trim().is_empty() {
        return Err(SecurityExternalProxyHistoryImportError::Build(
            "batch_id cannot be empty".to_string(),
        ));
    }
    if request.created_at.trim().is_empty() {
        return Err(SecurityExternalProxyHistoryImportError::Build(
            "created_at cannot be empty".to_string(),
        ));
    }
    if request.file_path.trim().is_empty() {
        return Err(SecurityExternalProxyHistoryImportError::Build(
            "file_path cannot be empty".to_string(),
        ));
    }

    Ok(())
}

// 2026-04-12 CST: Accept both JSON and CSV for proxy-history import, because early real-data
// batches may come from ad-hoc extracts before dedicated crawlers are stabilized.
// Purpose: keep the governed import bridge practical without inventing multiple tools.
fn parse_proxy_history_file(
    path: &Path,
) -> Result<Vec<SecurityExternalProxyBackfillRecordInput>, SecurityExternalProxyHistoryImportError>
{
    let payload = fs::read_to_string(path).map_err(|error| {
        SecurityExternalProxyHistoryImportError::Build(format!(
            "failed to read proxy history file `{}`: {error}",
            path.display()
        ))
    })?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match extension.as_str() {
        "json" => parse_proxy_history_json(&payload),
        "csv" => parse_proxy_history_csv(&payload),
        _ => Err(SecurityExternalProxyHistoryImportError::Build(format!(
            "unsupported proxy history file extension: {}",
            path.display()
        ))),
    }
}

fn parse_proxy_history_json(
    payload: &str,
) -> Result<Vec<SecurityExternalProxyBackfillRecordInput>, SecurityExternalProxyHistoryImportError>
{
    let value = serde_json::from_str::<Value>(payload).map_err(|error| {
        SecurityExternalProxyHistoryImportError::Build(format!(
            "failed to parse proxy history json: {error}"
        ))
    })?;

    if value.is_array() {
        return serde_json::from_value::<Vec<SecurityExternalProxyBackfillRecordInput>>(value)
            .map_err(|error| {
                SecurityExternalProxyHistoryImportError::Build(format!(
                    "failed to decode proxy history records: {error}"
                ))
            });
    }

    if let Some(records) = value.get("records") {
        return serde_json::from_value::<Vec<SecurityExternalProxyBackfillRecordInput>>(
            records.clone(),
        )
        .map_err(|error| {
            SecurityExternalProxyHistoryImportError::Build(format!(
                "failed to decode proxy history records: {error}"
            ))
        });
    }

    Err(SecurityExternalProxyHistoryImportError::Build(
        "proxy history json must be an array or contain a `records` field".to_string(),
    ))
}

fn parse_proxy_history_csv(
    payload: &str,
) -> Result<Vec<SecurityExternalProxyBackfillRecordInput>, SecurityExternalProxyHistoryImportError>
{
    let mut lines = payload.lines().filter(|line| !line.trim().is_empty());
    let Some(header_line) = lines.next() else {
        return Err(SecurityExternalProxyHistoryImportError::Build(
            "proxy history csv cannot be empty".to_string(),
        ));
    };
    // 2026-04-12 CST: Normalize BOM-bearing headers before field lookup, because
    // real CSV exports from Windows tooling may prepend UTF-8 BOM on the first column.
    // Purpose: keep governed proxy-history import tolerant to common operator exports.
    let headers = header_line
        .trim_start_matches('\u{feff}')
        .split(',')
        .map(|value| value.trim().to_string())
        .collect::<Vec<_>>();
    let header_indexes = headers
        .iter()
        .enumerate()
        .map(|(index, name)| (name.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let mut records = Vec::new();

    for line in lines {
        let mut columns = line
            .split(',')
            .map(|value| value.trim().to_string())
            .collect::<Vec<_>>();
        // 2026-04-12 CST: Pad short rows with empty trailing columns, because
        // manual CSV batches often omit trailing blanks while still representing
        // a valid fixed-width header contract.
        // Purpose: avoid false-negative import failures for optional trailing fields.
        while columns.len() < headers.len() {
            columns.push(String::new());
        }
        let record = SecurityExternalProxyBackfillRecordInput {
            symbol: read_required_csv_value(&columns, &header_indexes, "symbol")?,
            as_of_date: read_required_csv_value(&columns, &header_indexes, "as_of_date")?,
            instrument_subscope: read_required_csv_value(
                &columns,
                &header_indexes,
                "instrument_subscope",
            )?,
            external_proxy_inputs: SecurityExternalProxyInputs {
                yield_curve_proxy_status: read_optional_csv_value(
                    &columns,
                    &header_indexes,
                    "yield_curve_proxy_status",
                ),
                yield_curve_slope_delta_bp_5d: read_optional_csv_f64(
                    &columns,
                    &header_indexes,
                    "yield_curve_slope_delta_bp_5d",
                )?,
                funding_liquidity_proxy_status: read_optional_csv_value(
                    &columns,
                    &header_indexes,
                    "funding_liquidity_proxy_status",
                ),
                funding_liquidity_spread_delta_bp_5d: read_optional_csv_f64(
                    &columns,
                    &header_indexes,
                    "funding_liquidity_spread_delta_bp_5d",
                )?,
                gold_spot_proxy_status: read_optional_csv_value(
                    &columns,
                    &header_indexes,
                    "gold_spot_proxy_status",
                ),
                gold_spot_proxy_return_5d: read_optional_csv_f64(
                    &columns,
                    &header_indexes,
                    "gold_spot_proxy_return_5d",
                )?,
                usd_index_proxy_status: read_optional_csv_value(
                    &columns,
                    &header_indexes,
                    "usd_index_proxy_status",
                ),
                usd_index_proxy_return_5d: read_optional_csv_f64(
                    &columns,
                    &header_indexes,
                    "usd_index_proxy_return_5d",
                )?,
                real_rate_proxy_status: read_optional_csv_value(
                    &columns,
                    &header_indexes,
                    "real_rate_proxy_status",
                ),
                real_rate_proxy_delta_bp_5d: read_optional_csv_f64(
                    &columns,
                    &header_indexes,
                    "real_rate_proxy_delta_bp_5d",
                )?,
                fx_proxy_status: read_optional_csv_value(
                    &columns,
                    &header_indexes,
                    "fx_proxy_status",
                ),
                fx_return_5d: read_optional_csv_f64(&columns, &header_indexes, "fx_return_5d")?,
                overseas_market_proxy_status: read_optional_csv_value(
                    &columns,
                    &header_indexes,
                    "overseas_market_proxy_status",
                ),
                overseas_market_return_5d: read_optional_csv_f64(
                    &columns,
                    &header_indexes,
                    "overseas_market_return_5d",
                )?,
                market_session_gap_status: read_optional_csv_value(
                    &columns,
                    &header_indexes,
                    "market_session_gap_status",
                ),
                market_session_gap_days: read_optional_csv_f64(
                    &columns,
                    &header_indexes,
                    "market_session_gap_days",
                )?,
                etf_fund_flow_proxy_status: read_optional_csv_value(
                    &columns,
                    &header_indexes,
                    "etf_fund_flow_proxy_status",
                ),
                etf_fund_flow_5d: read_optional_csv_f64(
                    &columns,
                    &header_indexes,
                    "etf_fund_flow_5d",
                )?,
                premium_discount_proxy_status: read_optional_csv_value(
                    &columns,
                    &header_indexes,
                    "premium_discount_proxy_status",
                ),
                premium_discount_pct: read_optional_csv_f64(
                    &columns,
                    &header_indexes,
                    "premium_discount_pct",
                )?,
                benchmark_relative_strength_status: read_optional_csv_value(
                    &columns,
                    &header_indexes,
                    "benchmark_relative_strength_status",
                ),
                benchmark_relative_return_5d: read_optional_csv_f64(
                    &columns,
                    &header_indexes,
                    "benchmark_relative_return_5d",
                )?,
            },
        };
        records.push(record);
    }

    if records.is_empty() {
        return Err(SecurityExternalProxyHistoryImportError::Build(
            "proxy history csv contained no data rows".to_string(),
        ));
    }

    Ok(records)
}

fn read_required_csv_value(
    columns: &[String],
    header_indexes: &BTreeMap<String, usize>,
    field: &str,
) -> Result<String, SecurityExternalProxyHistoryImportError> {
    let Some(index) = header_indexes.get(field).copied() else {
        return Err(SecurityExternalProxyHistoryImportError::Build(format!(
            "proxy history csv missing required field `{field}`"
        )));
    };
    let value = columns.get(index).cloned().unwrap_or_default();
    if value.trim().is_empty() {
        return Err(SecurityExternalProxyHistoryImportError::Build(format!(
            "proxy history csv field `{field}` cannot be empty"
        )));
    }
    Ok(value)
}

fn read_optional_csv_value(
    columns: &[String],
    header_indexes: &BTreeMap<String, usize>,
    field: &str,
) -> Option<String> {
    header_indexes
        .get(field)
        .and_then(|index| columns.get(*index))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn read_optional_csv_f64(
    columns: &[String],
    header_indexes: &BTreeMap<String, usize>,
    field: &str,
) -> Result<Option<f64>, SecurityExternalProxyHistoryImportError> {
    let Some(value) = read_optional_csv_value(columns, header_indexes, field) else {
        return Ok(None);
    };
    value.parse::<f64>().map(Some).map_err(|error| {
        SecurityExternalProxyHistoryImportError::Build(format!(
            "failed to parse proxy history numeric field `{field}`: {error}"
        ))
    })
}
