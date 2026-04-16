use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_analysis_fullstack::{FundamentalContext, FundamentalMetrics};
use crate::runtime::security_fundamental_history_store::{
    SecurityFundamentalHistoryRecordRow, SecurityFundamentalHistoryStore,
    SecurityFundamentalHistoryStoreError,
};

// 2026-04-12 CST: Add a governed stock fundamental-history backfill request,
// because Historical Data Phase 1 needs replayable financial snapshots to enter
// the formal stock tool chain instead of remaining one-off live fetches.
// Purpose: let one tool import dated stock financial batches into governed runtime storage.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityFundamentalHistoryBackfillRequest {
    pub batch_id: String,
    pub created_at: String,
    #[serde(default)]
    pub history_runtime_root: Option<String>,
    pub records: Vec<SecurityFundamentalHistoryBackfillRecordInput>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityFundamentalHistoryBackfillRecordInput {
    pub symbol: String,
    pub report_period: String,
    #[serde(default)]
    pub notice_date: Option<String>,
    pub source: String,
    pub report_metrics: FundamentalMetrics,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityFundamentalHistoryBackfillPersistedRecord {
    pub record_ref: String,
    pub symbol: String,
    pub report_period: String,
    #[serde(default)]
    pub notice_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityFundamentalHistoryBackfillResult {
    pub contract_version: String,
    pub document_type: String,
    pub batch_ref: String,
    pub imported_record_count: usize,
    pub covered_symbol_count: usize,
    pub covered_report_periods: Vec<String>,
    pub storage_path: String,
    pub backfill_result_path: String,
    pub records: Vec<SecurityFundamentalHistoryBackfillPersistedRecord>,
}

#[derive(Debug, Error)]
pub enum SecurityFundamentalHistoryBackfillError {
    #[error("security fundamental history backfill build failed: {0}")]
    Build(String),
    #[error("security fundamental history backfill storage failed: {0}")]
    Storage(#[from] SecurityFundamentalHistoryStoreError),
}

// 2026-04-12 CST: Persist governed financial snapshots through one formal stock
// operation, because validation and replay should share the same historical
// financial rows instead of re-fetching live fundamentals every time.
// Purpose: create an idempotent import path for stock fundamental history.
pub fn security_fundamental_history_backfill(
    request: &SecurityFundamentalHistoryBackfillRequest,
) -> Result<SecurityFundamentalHistoryBackfillResult, SecurityFundamentalHistoryBackfillError> {
    validate_request(request)?;

    let store = resolve_store(request)?;
    let rows = request
        .records
        .iter()
        .map(|record| {
            let record_ref = build_record_ref(&record.symbol, &record.report_period);
            let report_metrics_json =
                serde_json::to_string(&record.report_metrics).map_err(|error| {
                    SecurityFundamentalHistoryBackfillError::Build(format!(
                        "failed to serialize report metrics: {error}"
                    ))
                })?;
            Ok::<SecurityFundamentalHistoryRecordRow, SecurityFundamentalHistoryBackfillError>(
                SecurityFundamentalHistoryRecordRow {
                    symbol: record.symbol.trim().to_string(),
                    report_period: record.report_period.trim().to_string(),
                    notice_date: record
                        .notice_date
                        .clone()
                        .map(|value| value.trim().to_string()),
                    source: record.source.trim().to_string(),
                    report_metrics_json,
                    batch_id: request.batch_id.trim().to_string(),
                    record_ref,
                    created_at: request.created_at.trim().to_string(),
                },
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    store.upsert_rows(&rows)?;

    let runtime_root = resolve_runtime_root(&store);
    let batch_ref = format!("fundamental-history-backfill:{}", request.batch_id.trim());
    let result_path = runtime_root
        .join("fundamental_history_backfill_results")
        .join(format!("{}.json", sanitize_identifier(&batch_ref)));
    let result = SecurityFundamentalHistoryBackfillResult {
        contract_version: "security_fundamental_history_backfill.v1".to_string(),
        document_type: "security_fundamental_history_backfill_result".to_string(),
        batch_ref,
        imported_record_count: rows.len(),
        covered_symbol_count: collect_unique_symbol_count(&rows),
        covered_report_periods: collect_covered_report_periods(&rows),
        storage_path: store.db_path().to_string_lossy().to_string(),
        backfill_result_path: result_path.to_string_lossy().to_string(),
        records: rows
            .iter()
            .map(|row| SecurityFundamentalHistoryBackfillPersistedRecord {
                record_ref: row.record_ref.clone(),
                symbol: row.symbol.clone(),
                report_period: row.report_period.clone(),
                notice_date: row.notice_date.clone(),
            })
            .collect(),
    };
    persist_json(&result_path, &result)?;
    Ok(result)
}

// 2026-04-12 CST: Resolve one governed financial snapshot for a symbol/date pair,
// because fullstack replay should prefer persisted financial history when it exists.
// Purpose: centralize stock-fundamental history decoding and narrative rebuilding.
pub fn load_historical_fundamental_context(
    symbol: &str,
    as_of_date: Option<&str>,
) -> Result<Option<FundamentalContext>, SecurityFundamentalHistoryBackfillError> {
    let store = SecurityFundamentalHistoryStore::workspace_default()?;
    let Some(row) = store.load_latest_record(symbol, as_of_date)? else {
        return Ok(None);
    };
    let metrics =
        serde_json::from_str::<FundamentalMetrics>(&row.report_metrics_json).map_err(|error| {
            SecurityFundamentalHistoryBackfillError::Build(format!(
                "failed to parse governed report metrics: {error}"
            ))
        })?;
    Ok(Some(build_context_from_metrics(
        "governed_fundamental_history".to_string(),
        Some(row.report_period),
        row.notice_date,
        metrics,
    )))
}

fn build_context_from_metrics(
    source: String,
    latest_report_period: Option<String>,
    report_notice_date: Option<String>,
    metrics: FundamentalMetrics,
) -> FundamentalContext {
    let profit_signal = classify_fundamental_signal(&metrics);
    let (headline, narrative, risk_flags) = build_fundamental_narrative(&metrics, &profit_signal);

    FundamentalContext {
        status: "available".to_string(),
        source,
        latest_report_period,
        report_notice_date,
        headline,
        profit_signal,
        report_metrics: metrics,
        narrative,
        risk_flags,
    }
}

fn classify_fundamental_signal(metrics: &FundamentalMetrics) -> String {
    match (metrics.revenue_yoy_pct, metrics.net_profit_yoy_pct) {
        (Some(revenue), Some(profit)) if revenue >= 0.0 && profit >= 0.0 => "positive".to_string(),
        (Some(revenue), Some(profit)) if revenue < 0.0 && profit < 0.0 => "negative".to_string(),
        (Some(_), Some(_)) => "mixed".to_string(),
        _ => "unknown".to_string(),
    }
}

fn build_fundamental_narrative(
    metrics: &FundamentalMetrics,
    profit_signal: &str,
) -> (String, Vec<String>, Vec<String>) {
    let revenue_text = metrics
        .revenue_yoy_pct
        .map(|value| format!("营收同比 {:.2}%", value))
        .unwrap_or_else(|| "营收同比暂缺".to_string());
    let profit_text = metrics
        .net_profit_yoy_pct
        .map(|value| format!("归母净利润同比 {:.2}%", value))
        .unwrap_or_else(|| "归母净利润同比暂缺".to_string());
    let roe_text = metrics
        .roe_pct
        .map(|value| format!("ROE {:.2}%", value))
        .unwrap_or_else(|| "ROE 暂缺".to_string());

    let headline = match profit_signal {
        "positive" => "最新财报显示营收和归母净利润保持同比增长。".to_string(),
        "negative" => "最新财报显示营收和归母净利润同步承压。".to_string(),
        "mixed" => "最新财报的收入与利润表现分化，需要继续确认经营趋势。".to_string(),
        _ => "最新财报仅返回了部分指标，当前更适合作为辅助观察。".to_string(),
    };

    let narrative = vec![
        headline.clone(),
        format!("{revenue_text}，{profit_text}。"),
        format!("盈利质量仍需结合 {roe_text} 与后续现金流披露继续核验。"),
    ];

    let mut risk_flags = Vec::new();
    if metrics.net_profit_yoy_pct.is_some_and(|value| value < 0.0) {
        risk_flags.push("归母净利润同比为负，后续估值修复弹性可能受限".to_string());
    }
    if metrics.revenue_yoy_pct.is_some_and(|value| value < 0.0) {
        risk_flags.push("营收同比为负，需要警惕需求或价格压力继续传导".to_string());
    }
    if metrics.roe_pct.is_some_and(|value| value < 8.0) {
        risk_flags.push("ROE 偏低，盈利效率仍需后续报告进一步验证".to_string());
    }
    if metrics.revenue_yoy_pct.is_none() || metrics.net_profit_yoy_pct.is_none() {
        risk_flags.push("财报关键同比指标不完整，当前解读存在缺口".to_string());
    }

    (headline, narrative, risk_flags)
}

fn validate_request(
    request: &SecurityFundamentalHistoryBackfillRequest,
) -> Result<(), SecurityFundamentalHistoryBackfillError> {
    if request.batch_id.trim().is_empty() {
        return Err(SecurityFundamentalHistoryBackfillError::Build(
            "batch_id cannot be empty".to_string(),
        ));
    }
    if request.created_at.trim().is_empty() {
        return Err(SecurityFundamentalHistoryBackfillError::Build(
            "created_at cannot be empty".to_string(),
        ));
    }
    if request.records.is_empty() {
        return Err(SecurityFundamentalHistoryBackfillError::Build(
            "records cannot be empty".to_string(),
        ));
    }
    for record in &request.records {
        if record.symbol.trim().is_empty() {
            return Err(SecurityFundamentalHistoryBackfillError::Build(
                "record symbol cannot be empty".to_string(),
            ));
        }
        if record.report_period.trim().is_empty() {
            return Err(SecurityFundamentalHistoryBackfillError::Build(
                "record report_period cannot be empty".to_string(),
            ));
        }
        if record.source.trim().is_empty() {
            return Err(SecurityFundamentalHistoryBackfillError::Build(
                "record source cannot be empty".to_string(),
            ));
        }
    }
    Ok(())
}

fn resolve_store(
    request: &SecurityFundamentalHistoryBackfillRequest,
) -> Result<SecurityFundamentalHistoryStore, SecurityFundamentalHistoryBackfillError> {
    if let Some(root) = request
        .history_runtime_root
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return Ok(SecurityFundamentalHistoryStore::new(
            PathBuf::from(root).join("security_fundamental_history.db"),
        ));
    }
    Ok(SecurityFundamentalHistoryStore::workspace_default()?)
}

fn collect_covered_report_periods(rows: &[SecurityFundamentalHistoryRecordRow]) -> Vec<String> {
    rows.iter()
        .map(|row| row.report_period.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn collect_unique_symbol_count(rows: &[SecurityFundamentalHistoryRecordRow]) -> usize {
    rows.iter()
        .map(|row| row.symbol.clone())
        .collect::<BTreeSet<_>>()
        .len()
}

fn resolve_runtime_root(store: &SecurityFundamentalHistoryStore) -> PathBuf {
    store
        .db_path()
        .parent()
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".excel_skill_runtime"))
}

fn build_record_ref(symbol: &str, report_period: &str) -> String {
    format!(
        "fundamental-history:{}:{}:v1",
        symbol.trim(),
        report_period.trim()
    )
}

fn persist_json(
    path: &Path,
    value: &impl Serialize,
) -> Result<(), SecurityFundamentalHistoryBackfillError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            SecurityFundamentalHistoryBackfillError::Build(format!(
                "failed to create fundamental history result dir: {error}"
            ))
        })?;
    }
    let payload = serde_json::to_vec_pretty(value).map_err(|error| {
        SecurityFundamentalHistoryBackfillError::Build(format!(
            "failed to serialize fundamental history result: {error}"
        ))
    })?;
    fs::write(path, payload).map_err(|error| {
        SecurityFundamentalHistoryBackfillError::Build(format!(
            "failed to persist fundamental history result `{}`: {error}",
            path.display()
        ))
    })
}

fn sanitize_identifier(raw: &str) -> String {
    raw.chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch,
            _ => '_',
        })
        .collect()
}
