use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_analysis_fullstack::{DisclosureAnnouncement, DisclosureContext};
use crate::runtime::security_disclosure_history_store::{
    SecurityDisclosureHistoryRecordRow, SecurityDisclosureHistoryStore,
    SecurityDisclosureHistoryStoreError,
};

// 2026-04-12 CST: Add a governed stock disclosure-history backfill request,
// because Historical Data Phase 1 needs replayable announcement history to enter
// the formal stock tool chain instead of remaining live-fetch only.
// Purpose: let one tool import dated stock announcement batches into governed runtime storage.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityDisclosureHistoryBackfillRequest {
    pub batch_id: String,
    pub created_at: String,
    #[serde(default)]
    pub history_runtime_root: Option<String>,
    pub records: Vec<SecurityDisclosureHistoryBackfillRecordInput>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityDisclosureHistoryBackfillRecordInput {
    pub symbol: String,
    pub published_at: String,
    pub title: String,
    #[serde(default)]
    pub article_code: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityDisclosureHistoryBackfillPersistedRecord {
    pub record_ref: String,
    pub symbol: String,
    pub published_at: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityDisclosureHistoryBackfillResult {
    pub contract_version: String,
    pub document_type: String,
    pub batch_ref: String,
    pub imported_record_count: usize,
    pub covered_symbol_count: usize,
    pub covered_published_dates: Vec<String>,
    pub storage_path: String,
    pub backfill_result_path: String,
    pub records: Vec<SecurityDisclosureHistoryBackfillPersistedRecord>,
}

#[derive(Debug, Error)]
pub enum SecurityDisclosureHistoryBackfillError {
    #[error("security disclosure history backfill build failed: {0}")]
    Build(String),
    #[error("security disclosure history backfill storage failed: {0}")]
    Storage(#[from] SecurityDisclosureHistoryStoreError),
}

// 2026-04-12 CST: Persist governed announcement history through one formal stock
// operation, because validation and replay should share the same disclosure rows
// instead of rebuilding event context from one-off live fetches every time.
// Purpose: create an idempotent import path for stock disclosure history.
pub fn security_disclosure_history_backfill(
    request: &SecurityDisclosureHistoryBackfillRequest,
) -> Result<SecurityDisclosureHistoryBackfillResult, SecurityDisclosureHistoryBackfillError> {
    validate_request(request)?;

    let store = resolve_store(request)?;
    let rows = request
        .records
        .iter()
        .map(|record| {
            let record_ref = record
                .article_code
                .clone()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| {
                    build_record_ref(&record.symbol, &record.published_at, &record.title)
                });
            Ok::<SecurityDisclosureHistoryRecordRow, SecurityDisclosureHistoryBackfillError>(
                SecurityDisclosureHistoryRecordRow {
                    symbol: record.symbol.trim().to_string(),
                    published_at: record.published_at.trim().to_string(),
                    title: record.title.trim().to_string(),
                    article_code: record
                        .article_code
                        .clone()
                        .map(|value| value.trim().to_string()),
                    category: record
                        .category
                        .clone()
                        .map(|value| value.trim().to_string()),
                    source: record.source.trim().to_string(),
                    batch_id: request.batch_id.trim().to_string(),
                    record_ref,
                    created_at: request.created_at.trim().to_string(),
                },
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    store.upsert_rows(&rows)?;

    let runtime_root = resolve_runtime_root(&store);
    let batch_ref = format!("disclosure-history-backfill:{}", request.batch_id.trim());
    let result_path = runtime_root
        .join("disclosure_history_backfill_results")
        .join(format!("{}.json", sanitize_identifier(&batch_ref)));
    let result = SecurityDisclosureHistoryBackfillResult {
        contract_version: "security_disclosure_history_backfill.v1".to_string(),
        document_type: "security_disclosure_history_backfill_result".to_string(),
        batch_ref,
        imported_record_count: rows.len(),
        covered_symbol_count: collect_unique_symbol_count(&rows),
        covered_published_dates: collect_covered_published_dates(&rows),
        storage_path: store.db_path().to_string_lossy().to_string(),
        backfill_result_path: result_path.to_string_lossy().to_string(),
        records: rows
            .iter()
            .map(|row| SecurityDisclosureHistoryBackfillPersistedRecord {
                record_ref: row.record_ref.clone(),
                symbol: row.symbol.clone(),
                published_at: row.published_at.clone(),
                title: row.title.clone(),
            })
            .collect(),
    };
    persist_json(&result_path, &result)?;
    Ok(result)
}

// 2026-04-12 CST: Resolve recent governed announcements for a symbol/date pair,
// because fullstack replay should prefer persisted disclosure history when it exists.
// Purpose: centralize stock-disclosure history decoding and summary rebuilding.
pub fn load_historical_disclosure_context(
    symbol: &str,
    as_of_date: Option<&str>,
    limit: usize,
) -> Result<Option<DisclosureContext>, SecurityDisclosureHistoryBackfillError> {
    let store = SecurityDisclosureHistoryStore::workspace_default()?;
    let rows = store.load_recent_records(symbol, as_of_date, limit.max(1))?;
    if rows.is_empty() {
        return Ok(None);
    }
    let recent_announcements = rows
        .into_iter()
        .map(|row| DisclosureAnnouncement {
            published_at: row.published_at,
            title: row.title,
            article_code: row.article_code,
            category: row.category,
        })
        .collect::<Vec<_>>();
    let keyword_summary = build_disclosure_keyword_summary(&recent_announcements);
    let risk_flags = build_disclosure_risk_flags(&recent_announcements);
    let headline = build_disclosure_headline(&recent_announcements, &risk_flags);

    Ok(Some(DisclosureContext {
        status: "available".to_string(),
        source: "governed_disclosure_history".to_string(),
        announcement_count: recent_announcements.len(),
        headline,
        keyword_summary,
        recent_announcements,
        risk_flags,
    }))
}

fn build_disclosure_keyword_summary(notices: &[DisclosureAnnouncement]) -> Vec<String> {
    let mut summary = Vec::new();
    if notices
        .iter()
        .any(|notice| contains_any(&notice.title, &["年度报告", "年报"]))
    {
        summary.push("最近公告包含年度报告".to_string());
    }
    if notices
        .iter()
        .any(|notice| contains_any(&notice.title, &["利润分配", "分红"]))
    {
        summary.push("最近公告包含利润分配或分红信息".to_string());
    }
    if notices
        .iter()
        .any(|notice| contains_any(&notice.title, &["回购", "增持"]))
    {
        summary.push("最近公告包含回购或增持类事项".to_string());
    }
    if summary.is_empty() {
        summary.push("最近公告暂未识别出高频正向事件关键词".to_string());
    }
    summary
}

fn build_disclosure_risk_flags(notices: &[DisclosureAnnouncement]) -> Vec<String> {
    let risk_keywords = [
        ("减持", "最近公告含减持事项，需要留意筹码压力"),
        ("问询", "最近公告含问询事项，需要留意监管关注点"),
        ("诉讼", "最近公告含诉讼事项，需要留意经营不确定性"),
        ("终止", "最近公告含终止事项，需要留意原有催化是否失效"),
        (
            "风险提示",
            "最近公告含风险提示，需要关注公司主动披露的不确定性",
        ),
        ("预亏", "最近公告含预亏信息，需要重新评估盈利预期"),
        ("亏损", "最近公告含亏损相关信息，需要警惕业绩压力"),
    ];
    let mut flags = Vec::new();
    for notice in notices {
        for (keyword, message) in risk_keywords {
            if notice.title.contains(keyword) && !flags.iter().any(|flag| flag == message) {
                flags.push(message.to_string());
            }
        }
    }
    flags
}

fn build_disclosure_headline(notices: &[DisclosureAnnouncement], risk_flags: &[String]) -> String {
    if !risk_flags.is_empty() {
        return "最近公告中已出现需要重点复核的风险关键词，信息面不宜按纯正向理解。".to_string();
    }
    if notices
        .iter()
        .any(|notice| contains_any(&notice.title, &["年度报告", "年报"]))
    {
        return "最近公告以定期披露为主，信息面暂未看到明显负向事件。".to_string();
    }
    if notices
        .iter()
        .any(|notice| contains_any(&notice.title, &["回购", "增持"]))
    {
        return "最近公告含回购或增持类事项，事件层对情绪存在一定支撑。".to_string();
    }
    "最近公告以常规定期披露和公司事项为主，暂未识别到强风险事件。".to_string()
}

fn contains_any(title: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| title.contains(keyword))
}

fn validate_request(
    request: &SecurityDisclosureHistoryBackfillRequest,
) -> Result<(), SecurityDisclosureHistoryBackfillError> {
    if request.batch_id.trim().is_empty() {
        return Err(SecurityDisclosureHistoryBackfillError::Build(
            "batch_id cannot be empty".to_string(),
        ));
    }
    if request.created_at.trim().is_empty() {
        return Err(SecurityDisclosureHistoryBackfillError::Build(
            "created_at cannot be empty".to_string(),
        ));
    }
    if request.records.is_empty() {
        return Err(SecurityDisclosureHistoryBackfillError::Build(
            "records cannot be empty".to_string(),
        ));
    }
    for record in &request.records {
        if record.symbol.trim().is_empty() {
            return Err(SecurityDisclosureHistoryBackfillError::Build(
                "record symbol cannot be empty".to_string(),
            ));
        }
        if record.published_at.trim().is_empty() {
            return Err(SecurityDisclosureHistoryBackfillError::Build(
                "record published_at cannot be empty".to_string(),
            ));
        }
        if record.title.trim().is_empty() {
            return Err(SecurityDisclosureHistoryBackfillError::Build(
                "record title cannot be empty".to_string(),
            ));
        }
        if record.source.trim().is_empty() {
            return Err(SecurityDisclosureHistoryBackfillError::Build(
                "record source cannot be empty".to_string(),
            ));
        }
    }
    Ok(())
}

fn resolve_store(
    request: &SecurityDisclosureHistoryBackfillRequest,
) -> Result<SecurityDisclosureHistoryStore, SecurityDisclosureHistoryBackfillError> {
    if let Some(root) = request
        .history_runtime_root
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return Ok(SecurityDisclosureHistoryStore::new(
            PathBuf::from(root).join("security_disclosure_history.db"),
        ));
    }
    Ok(SecurityDisclosureHistoryStore::workspace_default()?)
}

fn collect_covered_published_dates(rows: &[SecurityDisclosureHistoryRecordRow]) -> Vec<String> {
    rows.iter()
        .map(|row| row.published_at.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn collect_unique_symbol_count(rows: &[SecurityDisclosureHistoryRecordRow]) -> usize {
    rows.iter()
        .map(|row| row.symbol.clone())
        .collect::<BTreeSet<_>>()
        .len()
}

fn resolve_runtime_root(store: &SecurityDisclosureHistoryStore) -> PathBuf {
    store
        .db_path()
        .parent()
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".excel_skill_runtime"))
}

fn build_record_ref(symbol: &str, published_at: &str, title: &str) -> String {
    format!(
        "disclosure-history:{}:{}:{}:v1",
        symbol.trim(),
        published_at.trim(),
        sanitize_identifier(title.trim())
    )
}

fn persist_json(
    path: &Path,
    value: &impl Serialize,
) -> Result<(), SecurityDisclosureHistoryBackfillError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            SecurityDisclosureHistoryBackfillError::Build(format!(
                "failed to create disclosure history result dir: {error}"
            ))
        })?;
    }
    let payload = serde_json::to_vec_pretty(value).map_err(|error| {
        SecurityDisclosureHistoryBackfillError::Build(format!(
            "failed to serialize disclosure history result: {error}"
        ))
    })?;
    fs::write(path, payload).map_err(|error| {
        SecurityDisclosureHistoryBackfillError::Build(format!(
            "failed to persist disclosure history result `{}`: {error}",
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
