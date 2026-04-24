use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_decision_evidence_bundle::SecurityExternalProxyInputs;
use crate::runtime::security_external_proxy_store::{
    SecurityExternalProxyRecordRow, SecurityExternalProxyStore, SecurityExternalProxyStoreError,
};

// 2026-04-11 CST: Add a governed dated proxy backfill request, because P4 needs
// historical ETF and macro proxy records to enter the formal stock tool chain
// instead of remaining ad-hoc notes beside a training run.
// Purpose: let one tool import dated proxy batches that feature snapshot and
// training can later resolve by symbol and as-of date.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityExternalProxyBackfillRequest {
    pub batch_id: String,
    pub created_at: String,
    pub records: Vec<SecurityExternalProxyBackfillRecordInput>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityExternalProxyBackfillRecordInput {
    pub symbol: String,
    pub as_of_date: String,
    pub instrument_subscope: String,
    pub external_proxy_inputs: SecurityExternalProxyInputs,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityExternalProxyBackfillPersistedRecord {
    pub record_ref: String,
    pub symbol: String,
    pub as_of_date: String,
    pub instrument_subscope: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityExternalProxyBackfillResult {
    pub contract_version: String,
    pub document_type: String,
    pub batch_ref: String,
    pub imported_record_count: usize,
    #[serde(default)]
    pub covered_symbol_count: usize,
    #[serde(default)]
    pub coverage_tier: String,
    #[serde(default)]
    pub covered_dates: Vec<String>,
    #[serde(default)]
    pub covered_proxy_fields: Vec<String>,
    pub storage_path: String,
    #[serde(default)]
    pub backfill_result_path: String,
    pub records: Vec<SecurityExternalProxyBackfillPersistedRecord>,
}

#[derive(Debug, Error)]
pub enum SecurityExternalProxyBackfillError {
    #[error("security external proxy backfill build failed: {0}")]
    Build(String),
    #[error("security external proxy backfill storage failed: {0}")]
    Storage(#[from] SecurityExternalProxyStoreError),
}

// 2026-04-11 CST: Persist governed historical proxy rows through one formal stock
// operation, because P4 needs dated proxy history to become part of the auditable
// runtime rather than a sidecar spreadsheet or temporary manual input.
// Purpose: create an idempotent import path that later feature snapshots can join.
pub fn security_external_proxy_backfill(
    request: &SecurityExternalProxyBackfillRequest,
) -> Result<SecurityExternalProxyBackfillResult, SecurityExternalProxyBackfillError> {
    validate_backfill_request(request)?;

    let store = SecurityExternalProxyStore::workspace_default()?;
    let rows = request
        .records
        .iter()
        .map(|record| {
            let record_ref = build_record_ref(
                &record.symbol,
                &record.as_of_date,
                &record.instrument_subscope,
            );
            let external_proxy_inputs_json = serde_json::to_string(&record.external_proxy_inputs)
                .map_err(|error| {
                SecurityExternalProxyBackfillError::Build(format!(
                    "failed to serialize external proxy inputs: {error}"
                ))
            })?;
            Ok::<SecurityExternalProxyRecordRow, SecurityExternalProxyBackfillError>(
                SecurityExternalProxyRecordRow {
                    symbol: record.symbol.clone(),
                    as_of_date: record.as_of_date.clone(),
                    instrument_subscope: record.instrument_subscope.clone(),
                    external_proxy_inputs_json,
                    batch_id: request.batch_id.clone(),
                    record_ref,
                    created_at: request.created_at.clone(),
                },
            )
        })
        .collect::<Result<Vec<_>, SecurityExternalProxyBackfillError>>()?;
    store.upsert_rows(&rows)?;
    let covered_dates = collect_unique_dates(&rows);
    let covered_proxy_fields = collect_covered_proxy_fields(request)?;
    let covered_symbol_count = collect_unique_symbol_count(&rows);
    let runtime_root = resolve_backfill_runtime_root(&store);
    let batch_ref = format!("external-proxy-backfill:{}", request.batch_id.trim());
    let result_path = runtime_root
        .join("external_proxy_backfill_results")
        .join(format!("{}.json", sanitize_identifier(&batch_ref)));

    let result = SecurityExternalProxyBackfillResult {
        contract_version: "security_external_proxy_backfill.v1".to_string(),
        document_type: "security_external_proxy_backfill_result".to_string(),
        batch_ref,
        imported_record_count: rows.len(),
        covered_symbol_count,
        coverage_tier: "governed_backfill_ready".to_string(),
        covered_dates,
        covered_proxy_fields,
        storage_path: store.db_path().to_string_lossy().to_string(),
        backfill_result_path: result_path.to_string_lossy().to_string(),
        records: rows
            .iter()
            .map(|row| SecurityExternalProxyBackfillPersistedRecord {
                record_ref: row.record_ref.clone(),
                symbol: row.symbol.clone(),
                as_of_date: row.as_of_date.clone(),
                instrument_subscope: row.instrument_subscope.clone(),
            })
            .collect(),
    };
    persist_json(&result_path, &result)?;

    Ok(result)
}

// 2026-04-11 CST: Resolve one dated proxy snapshot for a symbol/date pair, because
// feature snapshot and training need one helper that decodes the runtime JSON row
// back into the governed external proxy contract.
// Purpose: centralize historical proxy loading so later consumers do not duplicate
// SQLite and serde glue code.
pub fn load_historical_external_proxy_inputs(
    symbol: &str,
    as_of_date: &str,
) -> Result<Option<SecurityExternalProxyInputs>, SecurityExternalProxyBackfillError> {
    let Some((_, inputs)) = load_historical_external_proxy_snapshot(symbol, as_of_date)? else {
        return Ok(None);
    };
    Ok(Some(inputs))
}

// 2026-04-17 CST: Added because the ETF latest-proxy path now needs both the resolved
// proxy payload and the effective proxy date when evidence requests omit as_of_date.
// Reason: returning inputs alone is not enough to realign the full analysis chain onto
// the governed proxy anchor date.
// Purpose: expose one shared dated snapshot helper so evidence/chair consumers do not
// split into separate date-resolution behaviors.
pub fn load_historical_external_proxy_snapshot(
    symbol: &str,
    as_of_date: &str,
) -> Result<Option<(String, SecurityExternalProxyInputs)>, SecurityExternalProxyBackfillError> {
    let store = SecurityExternalProxyStore::workspace_default()?;
    // 2026-04-12 UTC+08: Fall back to the nearest prior dated proxy snapshot,
    // because future-looking runs often freeze on the latest trading day even when
    // the operator asked on a weekend or holiday.
    // Purpose: keep ETF governed information available at final decision time without
    // requiring callers to manually translate non-trading dates back to trading dates.
    let Some(row) = store
        .load_record(symbol, as_of_date)?
        .or(store.load_latest_record_on_or_before(symbol, as_of_date)?)
    else {
        return Ok(None);
    };
    Ok(Some(parse_proxy_snapshot_row(row)?))
}

// 2026-04-17 CST: Added because no-date ETF requests should still consume the latest
// governed proxy history instead of behaving as if no proxy data exists.
// Reason: the previous helper set only supported explicit dates, which broke the
// latest-run chair path when as_of_date was intentionally omitted.
// Purpose: give the evidence layer one canonical latest snapshot loader that also
// reports the effective proxy date it resolved.
pub fn load_latest_external_proxy_snapshot(
    symbol: &str,
) -> Result<Option<(String, SecurityExternalProxyInputs)>, SecurityExternalProxyBackfillError> {
    let store = SecurityExternalProxyStore::workspace_default()?;
    let Some(row) = store.load_latest_record(symbol)? else {
        return Ok(None);
    };
    Ok(Some(parse_proxy_snapshot_row(row)?))
}

// 2026-04-12 CST: Centralize governed ETF proxy hydration here, because the
// snapshot path and the committee->scorecard->chair path must resolve the same
// historical proxy payload instead of drifting into separate merge rules.
// Purpose: make dated proxy backfill a single formal source of truth before the
// evidence bundle is frozen for scoring, package building, and final resolution.
pub fn resolve_effective_external_proxy_inputs(
    symbol: &str,
    as_of_date: Option<&str>,
    overrides: Option<SecurityExternalProxyInputs>,
) -> Result<Option<SecurityExternalProxyInputs>, SecurityExternalProxyBackfillError> {
    let historical_proxy_inputs = if let Some(effective_as_of_date) = as_of_date {
        load_historical_external_proxy_inputs(symbol, effective_as_of_date)?
    } else {
        load_latest_external_proxy_snapshot(symbol)?.map(|(_, inputs)| inputs)
    };
    Ok(merge_external_proxy_inputs(
        historical_proxy_inputs,
        overrides,
    ))
}

// 2026-04-17 CST: Added because all governed snapshot loaders should decode the same
// stored JSON row shape before they merge overrides or propagate effective dates.
// Reason: keeping row parsing duplicated across exact-date/latest helpers would make
// future proxy contract changes drift silently.
// Purpose: centralize row-to-contract decoding for historical proxy snapshot consumers.
fn parse_proxy_snapshot_row(
    row: SecurityExternalProxyRecordRow,
) -> Result<(String, SecurityExternalProxyInputs), SecurityExternalProxyBackfillError> {
    let inputs =
        serde_json::from_str::<SecurityExternalProxyInputs>(&row.external_proxy_inputs_json)
            .map_err(|error| {
                SecurityExternalProxyBackfillError::Build(format!(
                    "failed to parse historical external proxy inputs: {error}"
                ))
            })?;
    Ok((row.as_of_date, inputs))
}

// 2026-04-11 CST: Add a governed result-document loader, because P7 history
// expansion now consumes auditable backfill batches instead of relying only on
// free-form notes.
// Purpose: keep backfill-to-expansion linkage on one stable JSON contract.
pub fn load_security_external_proxy_backfill_result(
    path: &str,
) -> Result<SecurityExternalProxyBackfillResult, SecurityExternalProxyBackfillError> {
    let payload = fs::read(path).map_err(|error| {
        SecurityExternalProxyBackfillError::Build(format!(
            "failed to read security external proxy backfill result `{path}`: {error}"
        ))
    })?;
    serde_json::from_slice::<SecurityExternalProxyBackfillResult>(&payload).map_err(|error| {
        SecurityExternalProxyBackfillError::Build(format!(
            "failed to parse security external proxy backfill result `{path}`: {error}"
        ))
    })
}

// 2026-04-11 CST: Merge dated historical proxy inputs with live/manual overrides,
// because current-day decision flows still need request-level manual bindings while
// training snapshots should prefer dated backfill when present.
// Purpose: keep one effective proxy payload contract for evidence hashing and raw
// snapshot seeding regardless of where the data originated.
pub fn merge_external_proxy_inputs(
    historical: Option<SecurityExternalProxyInputs>,
    overrides: Option<SecurityExternalProxyInputs>,
) -> Option<SecurityExternalProxyInputs> {
    let mut merged = historical.unwrap_or_default();
    if let Some(overrides) = overrides {
        merged.yield_curve_proxy_status = overrides
            .yield_curve_proxy_status
            .or(merged.yield_curve_proxy_status);
        merged.yield_curve_slope_delta_bp_5d = overrides
            .yield_curve_slope_delta_bp_5d
            .or(merged.yield_curve_slope_delta_bp_5d);
        merged.funding_liquidity_proxy_status = overrides
            .funding_liquidity_proxy_status
            .or(merged.funding_liquidity_proxy_status);
        merged.funding_liquidity_spread_delta_bp_5d = overrides
            .funding_liquidity_spread_delta_bp_5d
            .or(merged.funding_liquidity_spread_delta_bp_5d);
        merged.gold_spot_proxy_status = overrides
            .gold_spot_proxy_status
            .or(merged.gold_spot_proxy_status);
        merged.gold_spot_proxy_return_5d = overrides
            .gold_spot_proxy_return_5d
            .or(merged.gold_spot_proxy_return_5d);
        merged.usd_index_proxy_status = overrides
            .usd_index_proxy_status
            .or(merged.usd_index_proxy_status);
        merged.usd_index_proxy_return_5d = overrides
            .usd_index_proxy_return_5d
            .or(merged.usd_index_proxy_return_5d);
        merged.real_rate_proxy_status = overrides
            .real_rate_proxy_status
            .or(merged.real_rate_proxy_status);
        merged.real_rate_proxy_delta_bp_5d = overrides
            .real_rate_proxy_delta_bp_5d
            .or(merged.real_rate_proxy_delta_bp_5d);
        merged.fx_proxy_status = overrides.fx_proxy_status.or(merged.fx_proxy_status);
        merged.fx_return_5d = overrides.fx_return_5d.or(merged.fx_return_5d);
        merged.overseas_market_proxy_status = overrides
            .overseas_market_proxy_status
            .or(merged.overseas_market_proxy_status);
        merged.overseas_market_return_5d = overrides
            .overseas_market_return_5d
            .or(merged.overseas_market_return_5d);
        merged.market_session_gap_status = overrides
            .market_session_gap_status
            .or(merged.market_session_gap_status);
        merged.market_session_gap_days = overrides
            .market_session_gap_days
            .or(merged.market_session_gap_days);
        merged.etf_fund_flow_proxy_status = overrides
            .etf_fund_flow_proxy_status
            .or(merged.etf_fund_flow_proxy_status);
        merged.etf_fund_flow_5d = overrides.etf_fund_flow_5d.or(merged.etf_fund_flow_5d);
        merged.premium_discount_proxy_status = overrides
            .premium_discount_proxy_status
            .or(merged.premium_discount_proxy_status);
        merged.premium_discount_pct = overrides
            .premium_discount_pct
            .or(merged.premium_discount_pct);
        merged.benchmark_relative_strength_status = overrides
            .benchmark_relative_strength_status
            .or(merged.benchmark_relative_strength_status);
        merged.benchmark_relative_return_5d = overrides
            .benchmark_relative_return_5d
            .or(merged.benchmark_relative_return_5d);
    }

    if merged == SecurityExternalProxyInputs::default() {
        None
    } else {
        Some(merged)
    }
}

fn validate_backfill_request(
    request: &SecurityExternalProxyBackfillRequest,
) -> Result<(), SecurityExternalProxyBackfillError> {
    if request.batch_id.trim().is_empty() {
        return Err(SecurityExternalProxyBackfillError::Build(
            "batch_id cannot be empty".to_string(),
        ));
    }
    if request.created_at.trim().is_empty() {
        return Err(SecurityExternalProxyBackfillError::Build(
            "created_at cannot be empty".to_string(),
        ));
    }
    if request.records.is_empty() {
        return Err(SecurityExternalProxyBackfillError::Build(
            "records cannot be empty".to_string(),
        ));
    }
    for record in &request.records {
        if record.symbol.trim().is_empty() {
            return Err(SecurityExternalProxyBackfillError::Build(
                "record symbol cannot be empty".to_string(),
            ));
        }
        if record.as_of_date.trim().is_empty() {
            return Err(SecurityExternalProxyBackfillError::Build(
                "record as_of_date cannot be empty".to_string(),
            ));
        }
        if record.instrument_subscope.trim().is_empty() {
            return Err(SecurityExternalProxyBackfillError::Build(
                "record instrument_subscope cannot be empty".to_string(),
            ));
        }
    }
    Ok(())
}

// 2026-04-11 CST: Collect covered proxy fields from the governed backfill batch,
// because P7 history-expansion governance needs explicit field coverage instead
// of re-inferring it from raw payload blobs later.
// Purpose: freeze one canonical covered-field list beside the backfill result.
fn collect_covered_proxy_fields(
    request: &SecurityExternalProxyBackfillRequest,
) -> Result<Vec<String>, SecurityExternalProxyBackfillError> {
    let mut covered = BTreeSet::new();
    for record in &request.records {
        let value = serde_json::to_value(&record.external_proxy_inputs).map_err(|error| {
            SecurityExternalProxyBackfillError::Build(format!(
                "failed to inspect external proxy inputs for covered fields: {error}"
            ))
        })?;
        let object = value.as_object().ok_or_else(|| {
            SecurityExternalProxyBackfillError::Build(
                "external proxy inputs should serialize into an object".to_string(),
            )
        })?;
        for (field_name, field_value) in object {
            if !field_value.is_null() {
                covered.insert(field_name.clone());
            }
        }
    }
    Ok(covered.into_iter().collect())
}

fn collect_unique_dates(rows: &[SecurityExternalProxyRecordRow]) -> Vec<String> {
    let mut covered_dates = rows
        .iter()
        .map(|row| row.as_of_date.trim().to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    covered_dates.sort();
    covered_dates
}

fn collect_unique_symbol_count(rows: &[SecurityExternalProxyRecordRow]) -> usize {
    rows.iter()
        .map(|row| row.symbol.trim().to_string())
        .collect::<BTreeSet<_>>()
        .len()
}

fn persist_json<T: Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), SecurityExternalProxyBackfillError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            SecurityExternalProxyBackfillError::Build(format!(
                "failed to create external proxy backfill result dir: {error}"
            ))
        })?;
    }
    let payload = serde_json::to_vec_pretty(value).map_err(|error| {
        SecurityExternalProxyBackfillError::Build(format!(
            "failed to serialize external proxy backfill result: {error}"
        ))
    })?;
    fs::write(path, payload).map_err(|error| {
        SecurityExternalProxyBackfillError::Build(format!(
            "failed to persist external proxy backfill result `{}`: {error}",
            path.display()
        ))
    })
}

fn resolve_backfill_runtime_root(store: &SecurityExternalProxyStore) -> PathBuf {
    store
        .db_path()
        .parent()
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".excel_skill_runtime"))
}

fn build_record_ref(symbol: &str, as_of_date: &str, instrument_subscope: &str) -> String {
    format!(
        "external-proxy:{}:{}:{}:v1",
        symbol.trim(),
        as_of_date.trim(),
        instrument_subscope.trim()
    )
}

fn sanitize_identifier(raw: &str) -> String {
    raw.chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch,
            _ => '_',
        })
        .collect()
}
