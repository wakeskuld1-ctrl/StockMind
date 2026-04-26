use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::runtime::formal_security_runtime_registry::FormalSecurityRuntimeRegistry;
use crate::runtime::security_capital_flow_store::{
    SecurityCapitalFlowRecord, SecurityCapitalFlowStore, SecurityCapitalFlowStoreError,
};

// 2026-04-25 CST: Added because the stock boundary references the governed
// capital-flow backfill contract but the module was missing after consolidation.
// Reason: compile health requires the public contract to exist even before source adapters are restored.
// Purpose: keep raw-flow ingestion explicit and side-effect-free in this recovery pass.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityCapitalFlowBackfillRequest {
    #[serde(default = "default_created_at")]
    pub created_at: String,
    #[serde(default)]
    pub batch_id: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub frequency: String,
    #[serde(default)]
    pub rows: Vec<Value>,
    #[serde(default)]
    pub records: Vec<SecurityCapitalFlowBackfillRecord>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCapitalFlowBackfillRecord {
    pub dataset_id: String,
    pub frequency: String,
    pub metric_date: String,
    pub series_key: String,
    pub value: f64,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub payload_json: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityCapitalFlowBackfillResult {
    pub document_type: String,
    pub generated_at: String,
    pub source: String,
    pub frequency: String,
    pub accepted_row_count: usize,
    pub status: String,
    pub summary: String,
}

#[derive(Debug, Error)]
pub enum SecurityCapitalFlowBackfillError {
    #[error("security capital flow backfill failed: runtime path resolution failed: {0}")]
    RuntimePath(String),
    #[error("security capital flow backfill failed: {0}")]
    Store(#[from] SecurityCapitalFlowStoreError),
}

pub fn security_capital_flow_backfill(
    request: &SecurityCapitalFlowBackfillRequest,
) -> Result<SecurityCapitalFlowBackfillResult, SecurityCapitalFlowBackfillError> {
    let source = request.source.trim().to_string();
    let frequency = request.frequency.trim().to_string();
    let records = normalize_records(request);
    let db_path = FormalSecurityRuntimeRegistry::capital_flow_db_path()
        .map_err(SecurityCapitalFlowBackfillError::RuntimePath)?;
    let store = SecurityCapitalFlowStore::new(db_path);
    let accepted_row_count = store.insert_records(&records)?;

    Ok(SecurityCapitalFlowBackfillResult {
        document_type: "security_capital_flow_backfill".to_string(),
        generated_at: normalize_created_at(&request.created_at),
        source,
        frequency,
        accepted_row_count,
        status: "persisted".to_string(),
        summary: format!(
            "capital-flow backfill persisted {} governed raw records for batch {}",
            accepted_row_count,
            request.batch_id.trim()
        ),
    })
}

fn normalize_records(
    request: &SecurityCapitalFlowBackfillRequest,
) -> Vec<SecurityCapitalFlowRecord> {
    let mut records = request
        .records
        .iter()
        .map(|record| SecurityCapitalFlowRecord {
            dataset_id: record.dataset_id.trim().to_string(),
            frequency: record.frequency.trim().to_string(),
            metric_date: record.metric_date.trim().to_string(),
            series_key: record.series_key.trim().to_string(),
            value: record.value,
            source: record.source.trim().to_string(),
            payload_json: record.payload_json.clone(),
        })
        .collect::<Vec<_>>();

    records.extend(request.rows.iter().filter_map(record_from_value));
    records
}

fn record_from_value(value: &Value) -> Option<SecurityCapitalFlowRecord> {
    Some(SecurityCapitalFlowRecord {
        dataset_id: value.get("dataset_id")?.as_str()?.trim().to_string(),
        frequency: value.get("frequency")?.as_str()?.trim().to_string(),
        metric_date: value.get("metric_date")?.as_str()?.trim().to_string(),
        series_key: value.get("series_key")?.as_str()?.trim().to_string(),
        value: value.get("value")?.as_f64()?,
        source: value
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string(),
        payload_json: value.get("payload_json").cloned().unwrap_or(Value::Null),
    })
}

fn normalize_created_at(created_at: &str) -> String {
    let trimmed = created_at.trim();
    if trimmed.is_empty() {
        Utc::now().to_rfc3339()
    } else {
        trimmed.to_string()
    }
}

fn default_created_at() -> String {
    Utc::now().to_rfc3339()
}
