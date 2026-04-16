use chrono::{Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::sync_stock_price_history::{
    SyncStockPriceHistoryRequest, SyncStockPriceHistoryResult, sync_stock_price_history,
};
use crate::runtime::stock_history_store::{StockHistoryStore, StockHistoryStoreError};

// 2026-04-09 CST: 这里统一定义分析日期门禁输入，原因是用户要求把“本地优先、自动补数、最近交易日回退”沉到 Tool/Contract 层。
// 目的：让 technical/fullstack/briefing/position_plan 复用同一份日期与补数决策，而不是各层各写一套 if/else。
#[derive(Debug, Clone, PartialEq)]
pub struct StockAnalysisDataGuardRequest {
    pub symbol: String,
    pub requested_as_of_date: Option<String>,
    pub lookback_days: usize,
}

// 2026-04-09 CST: 这里定义标准化日期门禁元数据，原因是用户明确要求输出 requested/effective/local/sync/fallback 等显式字段。
// 目的：把“为什么这次分析落到了哪个交易日”变成稳定合同，供 Skill、Tool、文档和后续 AI 一致消费。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StockAnalysisDateGuard {
    pub requested_as_of_date: String,
    pub effective_analysis_date: String,
    pub effective_trade_date: String,
    #[serde(default)]
    pub local_data_last_date: Option<String>,
    pub data_freshness_status: String,
    pub sync_attempted: bool,
    #[serde(default)]
    pub sync_result: Option<StockAnalysisSyncResult>,
    #[serde(default)]
    pub date_fallback_reason: Option<String>,
}

// 2026-04-09 CST: 这里单独定义补数回执，原因是上层不仅要知道“有没有补”，还要知道“补到了哪里、用了什么 provider、是否失败”。
// 目的：让补数结果具备可审计性，避免继续用含糊的布尔值掩盖关键事实。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StockAnalysisSyncResult {
    pub status: String,
    #[serde(default)]
    pub provider_used: Option<String>,
    #[serde(default)]
    pub imported_row_count: Option<usize>,
    #[serde(default)]
    pub synced_start_date: Option<String>,
    #[serde(default)]
    pub synced_end_date: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Error)]
pub enum StockAnalysisDataGuardError {
    #[error("{0}")]
    Store(#[from] StockHistoryStoreError),
    #[error("分析日期 `{value}` 不是合法的 YYYY-MM-DD")]
    InvalidRequestedDate { value: String },
    #[error(
        "股票 `{symbol}` 在请求日期 `{requested_as_of_date}` 之前没有可用历史数据；补数结果：{message}"
    )]
    NoUsableTradeDate {
        symbol: String,
        requested_as_of_date: String,
        message: String,
    },
}

// 2026-04-09 CST: 这里提供统一日期门禁入口，原因是技术面与上层证券链路都需要同一套“本地优先 -> 补数 -> 回退”决策。
// 目的：先把真实的交易日选择做成单一职责函数，再让各 Tool 只消费标准化结果。
pub fn ensure_analysis_date_guard(
    store: &StockHistoryStore,
    request: &StockAnalysisDataGuardRequest,
) -> Result<StockAnalysisDateGuard, StockAnalysisDataGuardError> {
    let requested_as_of_date = normalize_requested_date(request.requested_as_of_date.as_deref())?;
    let local_data_last_date = store.latest_trade_date(&request.symbol)?;
    let mut sync_attempted = false;
    let mut sync_result = None;

    let exact_local_date =
        store.latest_trade_date_on_or_before(&request.symbol, &requested_as_of_date)?;
    let has_exact_requested_date = exact_local_date
        .as_deref()
        .map(|date| date == requested_as_of_date)
        .unwrap_or(false);

    // 2026-04-09 CST: 这里严格执行“先查本地，再决定是否补数”，原因是用户明确纠正过调用顺序。
    // 目的：避免还没看本地库存就直接去抓站外数据，破坏标准化交付口径。
    if !has_exact_requested_date {
        sync_attempted = true;
        sync_result = Some(run_sync_attempt(
            store,
            request,
            &requested_as_of_date,
            local_data_last_date.as_deref(),
        )?);
    }

    let effective_trade_date = store
        .latest_trade_date_on_or_before(&request.symbol, &requested_as_of_date)?
        .ok_or_else(|| StockAnalysisDataGuardError::NoUsableTradeDate {
            symbol: request.symbol.clone(),
            requested_as_of_date: requested_as_of_date.clone(),
            message: sync_result
                .as_ref()
                .and_then(|result| result.message.clone())
                .unwrap_or_else(|| "本地与补数后都没有可用数据".to_string()),
        })?;

    let (data_freshness_status, date_fallback_reason) =
        if effective_trade_date == requested_as_of_date {
            if sync_attempted {
                ("synced_exact_requested_date".to_string(), None)
            } else {
                ("local_exact_requested_date".to_string(), None)
            }
        } else if sync_attempted {
            (
                "synced_then_fallback_to_latest_trade_date".to_string(),
                Some("requested_date_has_no_valid_close_after_sync".to_string()),
            )
        } else {
            (
                "local_fallback_to_latest_trade_date".to_string(),
                Some("requested_date_has_no_valid_local_close".to_string()),
            )
        };

    Ok(StockAnalysisDateGuard {
        requested_as_of_date,
        effective_analysis_date: effective_trade_date.clone(),
        effective_trade_date,
        local_data_last_date,
        data_freshness_status,
        sync_attempted,
        sync_result,
        date_fallback_reason,
    })
}

fn normalize_requested_date(value: Option<&str>) -> Result<String, StockAnalysisDataGuardError> {
    let normalized = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| Local::now().date_naive().format("%Y-%m-%d").to_string());
    parse_date(&normalized)?;
    Ok(normalized)
}

fn run_sync_attempt(
    _store: &StockHistoryStore,
    request: &StockAnalysisDataGuardRequest,
    requested_as_of_date: &str,
    local_data_last_date: Option<&str>,
) -> Result<StockAnalysisSyncResult, StockAnalysisDataGuardError> {
    let start_date = derive_sync_start_date(
        request.lookback_days,
        requested_as_of_date,
        local_data_last_date,
    )?;
    let sync_request = SyncStockPriceHistoryRequest {
        symbol: request.symbol.clone(),
        start_date: start_date.clone(),
        end_date: requested_as_of_date.to_string(),
        adjustment: "qfq".to_string(),
        providers: vec!["tencent".to_string(), "sina".to_string()],
    };

    Ok(match sync_stock_price_history(&sync_request) {
        Ok(result) => build_sync_success_result(&result),
        Err(error) => StockAnalysisSyncResult {
            status: "failed".to_string(),
            provider_used: None,
            imported_row_count: None,
            synced_start_date: Some(start_date),
            synced_end_date: Some(requested_as_of_date.to_string()),
            message: Some(error.to_string()),
        },
    })
}

fn derive_sync_start_date(
    lookback_days: usize,
    requested_as_of_date: &str,
    local_data_last_date: Option<&str>,
) -> Result<String, StockAnalysisDataGuardError> {
    if let Some(local_last_date) = local_data_last_date {
        let local_last_date = parse_date(local_last_date)?;
        return Ok((local_last_date + Duration::days(1))
            .format("%Y-%m-%d")
            .to_string());
    }

    let requested_date = parse_date(requested_as_of_date)?;
    let bootstrap_days = lookback_days.max(260) as i64 + 30;
    Ok((requested_date - Duration::days(bootstrap_days))
        .format("%Y-%m-%d")
        .to_string())
}

fn build_sync_success_result(result: &SyncStockPriceHistoryResult) -> StockAnalysisSyncResult {
    StockAnalysisSyncResult {
        status: "synced".to_string(),
        provider_used: Some(result.provider_used.clone()),
        imported_row_count: Some(result.imported_row_count),
        synced_start_date: Some(result.date_range.start_date.clone()),
        synced_end_date: Some(result.date_range.end_date.clone()),
        message: None,
    }
}

fn parse_date(value: &str) -> Result<NaiveDate, StockAnalysisDataGuardError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| {
        StockAnalysisDataGuardError::InvalidRequestedDate {
            value: value.to_string(),
        }
    })
}
