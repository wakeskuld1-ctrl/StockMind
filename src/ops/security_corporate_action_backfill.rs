use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::runtime::security_corporate_action_store::{
    SecurityCorporateActionRow, SecurityCorporateActionStore, SecurityCorporateActionStoreError,
};

// 2026-04-18 CST: Added because scheme C2 needs one governed dated corporate-action
// backfill contract before training-data completion can stop depending on manual
// side notes or direct SQLite edits.
// Purpose: expose cash-dividend and bonus/split facts through one formal stock tool.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCorporateActionBackfillRequest {
    pub batch_id: String,
    pub created_at: String,
    pub records: Vec<SecurityCorporateActionBackfillRecordInput>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCorporateActionBackfillRecordInput {
    pub symbol: String,
    pub effective_date: String,
    pub action_type: String,
    #[serde(default)]
    pub cash_dividend_per_share: f64,
    #[serde(default = "default_split_ratio")]
    pub split_ratio: f64,
    #[serde(default)]
    pub bonus_ratio: f64,
    pub source: String,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCorporateActionBackfillPersistedRecord {
    pub record_ref: String,
    pub symbol: String,
    pub effective_date: String,
    pub action_type: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCorporateActionBackfillResult {
    pub contract_version: String,
    pub document_type: String,
    pub batch_ref: String,
    pub imported_record_count: usize,
    pub covered_symbol_count: usize,
    pub covered_dates: Vec<String>,
    pub storage_path: String,
    pub backfill_result_path: String,
    pub records: Vec<SecurityCorporateActionBackfillPersistedRecord>,
}

#[derive(Debug, Error)]
pub enum SecurityCorporateActionBackfillError {
    #[error("security corporate action backfill build failed: {0}")]
    Build(String),
    #[error("security corporate action backfill storage failed: {0}")]
    Storage(#[from] SecurityCorporateActionStoreError),
}

// 2026-04-18 CST: Added because governed training and evidence layers need one
// audited import path for dated corporate-action rows instead of writing the
// runtime store ad hoc from tests or manual scripts.
// Purpose: persist idempotent symbol/date/action facts and emit one audit result file.
pub fn security_corporate_action_backfill(
    request: &SecurityCorporateActionBackfillRequest,
) -> Result<SecurityCorporateActionBackfillResult, SecurityCorporateActionBackfillError> {
    validate_request(request)?;

    let store = SecurityCorporateActionStore::workspace_default()?;
    let rows = request
        .records
        .iter()
        .map(|record| {
            let payload_json = serde_json::to_string(&record.payload).map_err(|error| {
                SecurityCorporateActionBackfillError::Build(format!(
                    "failed to serialize corporate action payload: {error}"
                ))
            })?;
            Ok::<SecurityCorporateActionRow, SecurityCorporateActionBackfillError>(
                SecurityCorporateActionRow {
                    symbol: record.symbol.trim().to_string(),
                    effective_date: record.effective_date.trim().to_string(),
                    action_type: record.action_type.trim().to_string(),
                    cash_dividend_per_share: record.cash_dividend_per_share,
                    split_ratio: record.split_ratio,
                    bonus_ratio: record.bonus_ratio,
                    source: record.source.trim().to_string(),
                    payload_json,
                },
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    store.upsert_rows(&rows)?;

    let runtime_root = resolve_backfill_runtime_root(&store);
    let batch_ref = format!("corporate-action-backfill:{}", request.batch_id.trim());
    let result_path = runtime_root
        .join("corporate_action_backfill_results")
        .join(format!("{}.json", sanitize_identifier(&batch_ref)));
    let result = SecurityCorporateActionBackfillResult {
        contract_version: "security_corporate_action_backfill.v1".to_string(),
        document_type: "security_corporate_action_backfill_result".to_string(),
        batch_ref,
        imported_record_count: rows.len(),
        covered_symbol_count: collect_unique_symbol_count(&rows),
        covered_dates: collect_unique_dates(&rows),
        storage_path: store.db_path().to_string_lossy().to_string(),
        backfill_result_path: result_path.to_string_lossy().to_string(),
        records: rows
            .iter()
            .map(|row| SecurityCorporateActionBackfillPersistedRecord {
                record_ref: build_record_ref(&row.symbol, &row.effective_date, &row.action_type),
                symbol: row.symbol.clone(),
                effective_date: row.effective_date.clone(),
                action_type: row.action_type.clone(),
            })
            .collect(),
    };
    persist_json(&result_path, &result)?;
    Ok(result)
}

fn validate_request(
    request: &SecurityCorporateActionBackfillRequest,
) -> Result<(), SecurityCorporateActionBackfillError> {
    if request.batch_id.trim().is_empty() {
        return Err(SecurityCorporateActionBackfillError::Build(
            "batch_id cannot be empty".to_string(),
        ));
    }
    if request.created_at.trim().is_empty() {
        return Err(SecurityCorporateActionBackfillError::Build(
            "created_at cannot be empty".to_string(),
        ));
    }
    if request.records.is_empty() {
        return Err(SecurityCorporateActionBackfillError::Build(
            "records cannot be empty".to_string(),
        ));
    }

    for record in &request.records {
        if record.symbol.trim().is_empty() {
            return Err(SecurityCorporateActionBackfillError::Build(
                "record symbol cannot be empty".to_string(),
            ));
        }
        if record.effective_date.trim().is_empty() {
            return Err(SecurityCorporateActionBackfillError::Build(
                "record effective_date cannot be empty".to_string(),
            ));
        }
        if record.action_type.trim().is_empty() {
            return Err(SecurityCorporateActionBackfillError::Build(
                "record action_type cannot be empty".to_string(),
            ));
        }
        if record.source.trim().is_empty() {
            return Err(SecurityCorporateActionBackfillError::Build(
                "record source cannot be empty".to_string(),
            ));
        }
        if record.split_ratio <= 0.0 {
            return Err(SecurityCorporateActionBackfillError::Build(
                "record split_ratio must be greater than 0".to_string(),
            ));
        }
    }
    Ok(())
}

fn collect_unique_dates(rows: &[SecurityCorporateActionRow]) -> Vec<String> {
    let mut covered_dates = rows
        .iter()
        .map(|row| row.effective_date.trim().to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    covered_dates.sort();
    covered_dates
}

fn collect_unique_symbol_count(rows: &[SecurityCorporateActionRow]) -> usize {
    rows.iter()
        .map(|row| row.symbol.trim().to_string())
        .collect::<BTreeSet<_>>()
        .len()
}

fn resolve_backfill_runtime_root(store: &SecurityCorporateActionStore) -> PathBuf {
    store
        .db_path()
        .parent()
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".stockmind_runtime"))
}

fn persist_json<T: Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), SecurityCorporateActionBackfillError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            SecurityCorporateActionBackfillError::Build(format!(
                "failed to create corporate action backfill result dir: {error}"
            ))
        })?;
    }
    let payload = serde_json::to_vec_pretty(value).map_err(|error| {
        SecurityCorporateActionBackfillError::Build(format!(
            "failed to serialize corporate action backfill result: {error}"
        ))
    })?;
    fs::write(path, payload).map_err(|error| {
        SecurityCorporateActionBackfillError::Build(format!(
            "failed to persist corporate action backfill result `{}`: {error}",
            path.display()
        ))
    })
}

fn build_record_ref(symbol: &str, effective_date: &str, action_type: &str) -> String {
    format!(
        "corporate-action:{}:{}:{}:v1",
        symbol.trim(),
        effective_date.trim(),
        action_type.trim()
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

fn default_split_ratio() -> f64 {
    1.0
}
