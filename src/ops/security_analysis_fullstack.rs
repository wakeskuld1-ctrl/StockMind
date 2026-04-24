use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use thiserror::Error;

use crate::ops::stock::security_analysis_contextual::{
    SecurityAnalysisContextualError, SecurityAnalysisContextualRequest,
    SecurityAnalysisContextualResult, security_analysis_contextual,
};
use crate::ops::stock::security_decision_evidence_bundle::SecurityExternalProxyInputs;
use crate::ops::stock::security_disclosure_history_backfill::load_historical_disclosure_context;
use crate::ops::stock::security_external_proxy_backfill::{
    load_historical_external_proxy_snapshot, load_latest_external_proxy_snapshot,
};
use crate::ops::stock::security_fundamental_history_backfill::load_historical_fundamental_context;
use crate::ops::stock::stock_analysis_data_guard::StockAnalysisDateGuard;
use crate::ops::stock::technical_consultation_basic::{
    TechnicalConsultationBasicRequest, TechnicalConsultationBasicResult,
    technical_consultation_basic,
};

const DEFAULT_DISCLOSURE_LIMIT: usize = 8;
const DEFAULT_CROSS_BORDER_LOOKBACK_DAYS: usize = 260;
const DEFAULT_SINA_FINANCIAL_URL_BASE: &str =
    "https://vip.stock.finance.sina.com.cn/corp/go.php/vFD_FinancialGuideLine";
const DEFAULT_SINA_ANNOUNCEMENT_URL_BASE: &str =
    "https://vip.stock.finance.sina.com.cn/corp/go.php/vCB_AllBulletin";

// 2026-04-02 CST: 这里重写 fullstack 请求结构旁的说明，原因是当前证券分析主链已经从“单东财抓取”升级成“多源降级聚合”；
// 目的：让调用方继续沿用原有入参，但底层可以透明切到东财、官方备源和新浪备源，不再把网络可达性暴露给上层。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityAnalysisFullstackRequest {
    pub symbol: String,
    #[serde(default)]
    pub market_symbol: Option<String>,
    #[serde(default)]
    pub sector_symbol: Option<String>,
    #[serde(default)]
    pub market_profile: Option<String>,
    #[serde(default)]
    pub sector_profile: Option<String>,
    #[serde(default)]
    pub as_of_date: Option<String>,
    #[serde(default)]
    pub underlying_symbol: Option<String>,
    #[serde(default)]
    pub fx_symbol: Option<String>,
    #[serde(default = "default_lookback_days")]
    pub lookback_days: usize,
    #[serde(default = "default_disclosure_limit")]
    pub disclosure_limit: usize,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityAnalysisFullstackResult {
    pub symbol: String,
    // 2026-04-08 CST: 这里新增统一分析日期字段，原因是方案 C 要把公共合同从 briefing 下沉到 fullstack 层；
    // 目的：让后续 briefing / committee / agent 可以直接消费 fullstack 顶层日期，而不必回钻 nested technical_context。
    pub analysis_date: String,
    // 2026-04-08 CST: 这里新增证据版本字段，原因是 fullstack 聚合了技术面、财报和公告，需要稳定的证据快照版本号；
    // 目的：为后续链路提供统一的事实版本引用，避免只靠 symbol 或隐式嵌套字段判断版本。
    pub evidence_version: String,
    pub analysis_date_guard: StockAnalysisDateGuard,
    pub technical_context: SecurityAnalysisContextualResult,
    pub fundamental_context: FundamentalContext,
    pub disclosure_context: DisclosureContext,
    // 2026-04-13 CST: 这里把 ETF 专项事实层补进 fullstack 顶层，原因是 ETF 目前只有资产识别，没有正式研究上下文。
    // 目的：让后续证据包、特征快照和训练入口能消费基金结构、基准与折溢价等专项事实。
    pub etf_context: EtfContext,
    // 2026-04-15 CST: Added because cross-border ETF analysis must be modeled as
    // underlying-first instead of ETF-candle-first.
    // Reason: the user explicitly required all cross-border ETFs to follow the
    // "underlying market -> FX -> premium -> ETF mapping" workflow.
    // Purpose: give the formal securities chain one reusable cross-border ETF object.
    pub cross_border_context: CrossBorderEtfContext,
    pub industry_context: IndustryContext,
    pub integrated_conclusion: IntegratedConclusion,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FundamentalContext {
    pub status: String,
    pub source: String,
    pub latest_report_period: Option<String>,
    pub report_notice_date: Option<String>,
    pub headline: String,
    pub profit_signal: String,
    pub report_metrics: FundamentalMetrics,
    pub narrative: Vec<String>,
    pub risk_flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FundamentalMetrics {
    pub revenue: Option<f64>,
    pub revenue_yoy_pct: Option<f64>,
    pub net_profit: Option<f64>,
    pub net_profit_yoy_pct: Option<f64>,
    pub roe_pct: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct DisclosureContext {
    pub status: String,
    pub source: String,
    pub announcement_count: usize,
    pub headline: String,
    pub keyword_summary: Vec<String>,
    pub recent_announcements: Vec<DisclosureAnnouncement>,
    pub risk_flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct DisclosureAnnouncement {
    pub published_at: String,
    pub title: String,
    pub article_code: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct EtfContext {
    pub status: String,
    pub source: String,
    pub fund_name: Option<String>,
    pub benchmark: Option<String>,
    pub asset_scope: Option<String>,
    pub latest_scale: Option<f64>,
    pub latest_share: Option<f64>,
    pub premium_discount_rate_pct: Option<f64>,
    pub headline: String,
    pub structure_risk_flags: Vec<String>,
    pub research_gaps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CrossBorderEtfContext {
    pub status: String,
    pub analysis_method: String,
    pub underlying_market: CrossBorderLegAnalysis,
    pub fx_market: CrossBorderLegAnalysis,
    pub premium_assessment: CrossBorderPremiumAssessment,
    pub resonance_verdict: String,
    pub headline: String,
    pub rationale: Vec<String>,
    pub risk_flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CrossBorderLegAnalysis {
    pub status: String,
    pub symbol: Option<String>,
    pub bias: Option<String>,
    pub confidence: Option<String>,
    pub headline: String,
    pub support_level_20: Option<f64>,
    pub resistance_level_20: Option<f64>,
    pub rationale: Vec<String>,
    pub risk_flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CrossBorderPremiumAssessment {
    pub status: String,
    pub premium_discount_rate_pct: Option<f64>,
    pub verdict: String,
    pub headline: String,
    pub risk_flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct IndustryContext {
    pub sector_symbol: String,
    pub proxy_bias: String,
    pub headline: String,
    pub rationale: Vec<String>,
    pub risk_flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct IntegratedConclusion {
    pub stance: String,
    pub headline: String,
    pub rationale: Vec<String>,
    pub risk_flags: Vec<String>,
}

#[derive(Debug, Error)]
pub enum SecurityAnalysisFullstackError {
    #[error("技术上下文分析失败: {0}")]
    Technical(#[from] SecurityAnalysisContextualError),
}

#[derive(Debug, Error)]
enum FundamentalFetchError {
    #[error("财报源请求失败: {0}")]
    Transport(String),
    #[error("财报源响应解析失败: {0}")]
    Parse(String),
    #[error("财报源没有返回可用数据")]
    Empty,
}

#[derive(Debug, Error)]
enum DisclosureFetchError {
    #[error("公告源请求失败: {0}")]
    Transport(String),
    #[error("公告源响应解析失败: {0}")]
    Parse(String),
    #[error("公告源没有返回可用数据")]
    Empty,
}

#[derive(Debug, Error)]
enum EtfFetchError {
    #[error("ETF 公开事实源请求失败: {0}")]
    Transport(String),
    #[error("ETF 公开事实源解析失败: {0}")]
    Parse(String),
    #[error("ETF 公开事实源没有返回可用数据")]
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
struct RawAnnouncement {
    published_at: Option<String>,
    title: String,
    article_code: Option<String>,
    category: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct ParsedHtmlRow {
    html: String,
    cells: Vec<String>,
}

// 2026-04-14 CST: Added because plan A+ needs the fullstack provider parser to serve governed
// history imports as well as one-shot analysis, and duplicating parser logic would diverge fast.
// Purpose: expose one reusable multi-period financial row contract for stock history thickening.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct GovernedFundamentalHistoryRow {
    pub report_period: String,
    pub notice_date: Option<String>,
    pub source: String,
    pub report_metrics: FundamentalMetrics,
}

// 2026-04-14 CST: Added because governed announcement backfill should reuse the same provider
// parsing semantics as fullstack analysis instead of inventing a second event parser.
// Purpose: expose one reusable disclosure-history row contract for stock history thickening.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct GovernedDisclosureHistoryRow {
    pub published_at: String,
    pub title: String,
    pub article_code: Option<String>,
    pub category: Option<String>,
    pub source: String,
}

// 2026-04-20 CST: Added because the frozen P10/P11 ETF contract now treats a complete
// governed proxy family as first-class research evidence when stock-only info is absent.
// Reason: chair/committee regressions showed the chain still fell back to unavailable
// stock semantics even after governed ETF proxy history was already bound.
// Purpose: keep the proxy payload and its resolved date together while fullstack decides
// whether ETF information can be promoted from degraded to governed-available.
#[derive(Debug, Clone, PartialEq)]
struct GovernedEtfProxySnapshot {
    as_of_date: String,
    inputs: SecurityExternalProxyInputs,
}

pub fn security_analysis_fullstack(
    request: &SecurityAnalysisFullstackRequest,
) -> Result<SecurityAnalysisFullstackResult, SecurityAnalysisFullstackError> {
    let technical_request = SecurityAnalysisContextualRequest {
        symbol: request.symbol.clone(),
        market_symbol: request.market_symbol.clone(),
        sector_symbol: request.sector_symbol.clone(),
        market_profile: request.market_profile.clone(),
        sector_profile: request.sector_profile.clone(),
        as_of_date: request.as_of_date.clone(),
        lookback_days: request.lookback_days,
    };
    let technical_context = security_analysis_contextual(&technical_request)?;
    // 2026-04-13 CST: 这里先在技术分析后同步装配 ETF 专项事实层，原因是 ETF 后续要和技术面共享同一分析日期与证据版本。
    // 目的：把 ETF 信息厚度直接纳入 fullstack 主对象，而不是留在 briefing 层做隐式补充。
    let mut etf_context = fetch_etf_context(&request.symbol);
    // 2026-04-20 CST: Added because ETF governed proxy history can now replace the
    // old "stock-only info missing => fullstack degraded" behavior for P10/P11 closeout.
    // Reason: the governed proxy backfill already resolves exact/latest dates and should
    // promote ETF info layers before committee/chair freeze the evidence bundle.
    // Purpose: centralize the ETF proxy-complete recovery decision in fullstack instead
    // of patching chair output after the research contract is already frozen.
    let governed_etf_proxy = resolve_governed_etf_proxy_snapshot(request);
    let etf_proxy_subscope = derive_etf_proxy_subscope(request, &etf_context);
    let has_complete_governed_etf_proxy = governed_etf_proxy
        .as_ref()
        .map(|snapshot| {
            governed_etf_proxy_family_is_complete(&snapshot.inputs, &etf_proxy_subscope)
        })
        .unwrap_or(false);
    // 2026-04-17 CST: Reason=once governed history exists, validation and replay should
    // read that frozen evidence before touching live providers again. Purpose=restore the
    // governed-history precedence contract for both fundamentals and disclosures.
    let mut fundamental_context =
        match load_historical_fundamental_context(&request.symbol, request.as_of_date.as_deref()) {
            Ok(Some(context)) => context,
            Ok(None) | Err(_) => match fetch_fundamental_context(&request.symbol) {
                Ok(context) => context,
                Err(error) => build_unavailable_fundamental_context(error.to_string()),
            },
        };
    let mut disclosure_context = match load_historical_disclosure_context(
        &request.symbol,
        request.as_of_date.as_deref(),
        request.disclosure_limit.max(1),
    ) {
        Ok(Some(context)) => context,
        Ok(None) | Err(_) => {
            match fetch_disclosure_context(&request.symbol, request.disclosure_limit.max(1)) {
                Ok(context) => context,
                Err(error) => build_unavailable_disclosure_context(error.to_string()),
            }
        }
    };
    // 2026-04-20 CST: Added because ETF proxy-complete runs must stop reporting
    // stock-only missing data once the governed ETF proxy family is fully bound.
    // Reason: the frozen handoff contract explicitly allows proxy-complete ETF evidence
    // to substitute for stock fundamentals/disclosures in committee and chair flows.
    // Purpose: promote the minimum governed ETF research surface in one place without
    // changing the stock path or requiring callers to special-case unavailable contexts.
    if has_complete_governed_etf_proxy {
        let proxy_as_of_date = governed_etf_proxy
            .as_ref()
            .map(|snapshot| snapshot.as_of_date.as_str())
            .unwrap_or_else(|| technical_context.analysis_date.as_str());
        if fundamental_context.status != "available" {
            fundamental_context = build_governed_etf_proxy_fundamental_context(
                &request.symbol,
                proxy_as_of_date,
                &etf_proxy_subscope,
            );
        }
        if disclosure_context.status != "available" {
            disclosure_context = build_governed_etf_proxy_disclosure_context(
                &request.symbol,
                proxy_as_of_date,
                &etf_proxy_subscope,
            );
        }
        if etf_context.status != "available" {
            etf_context =
                build_governed_etf_proxy_etf_context(&request.symbol, &etf_proxy_subscope);
        }
    }
    let cross_border_context =
        build_cross_border_context(request, &technical_context, &etf_context);
    let industry_context = build_industry_context(&technical_context);
    let integrated_conclusion = build_integrated_conclusion(
        &request.symbol,
        &technical_context,
        &fundamental_context,
        &disclosure_context,
        &etf_context,
        &cross_border_context,
        &industry_context,
    );
    // 2026-04-08 CST: 这里沿用技术上下文的统一日期生成 fullstack 顶层合同字段，原因是聚合链路必须对齐同一分析时点；
    // 目的：确保顶层 fullstack 合同能稳定暴露 `analysis_date / evidence_version`，供更高层直接复用。
    // 2026-04-20 CST: Added because latest ETF requests without explicit as_of_date must
    // anchor their top-level analysis date to the resolved governed proxy date.
    // Reason: the technical context may resolve to a later calendar date than the latest
    // governed ETF proxy snapshot, which previously broke chair-level replay semantics.
    // Purpose: freeze the public fullstack date to the ETF proxy anchor when that is the
    // actual governed evidence date consumed by downstream scorecard and chair flows.
    let analysis_date = if request.as_of_date.is_none() && has_complete_governed_etf_proxy {
        governed_etf_proxy
            .as_ref()
            .map(|snapshot| snapshot.as_of_date.clone())
            .unwrap_or_else(|| technical_context.analysis_date.clone())
    } else {
        technical_context.analysis_date.clone()
    };
    let evidence_version = format!(
        "security-analysis-fullstack:{}:{}:v1",
        request.symbol, analysis_date
    );

    Ok(SecurityAnalysisFullstackResult {
        symbol: request.symbol.clone(),
        analysis_date,
        evidence_version,
        analysis_date_guard: technical_context.analysis_date_guard.clone(),
        technical_context,
        fundamental_context,
        disclosure_context,
        etf_context,
        cross_border_context,
        industry_context,
        integrated_conclusion,
    })
}

// 2026-04-02 CST: 这里把财报抓取改成三层 provider 链，原因是用户现场已经确认东财在本机网络下会稳定 TLS 失败；
// 目的：先走东财主源，再尝试可插拔官方源，最后退到新浪公开页，尽量把 technical_only 缩到真正全链路都失效时。
fn fetch_fundamental_context(symbol: &str) -> Result<FundamentalContext, FundamentalFetchError> {
    let mut attempt_errors = Vec::new();

    match fetch_fundamental_from_eastmoney(symbol) {
        Ok(context) => return Ok(context),
        Err(error) => attempt_errors.push(format!("eastmoney_financials: {error}")),
    }

    if let Some(url) = build_optional_official_financial_url(symbol) {
        match fetch_fundamental_from_official_json(&url) {
            Ok(context) => return Ok(context),
            Err(error) => attempt_errors.push(format!("official_financials: {error}")),
        }
    }

    match fetch_fundamental_from_sina_resilient(symbol) {
        Ok(context) => return Ok(context),
        Err(error) => attempt_errors.push(format!("sina_financial_guideline: {error}")),
    }

    Err(FundamentalFetchError::Transport(attempt_errors.join(" | ")))
}

// 2026-04-14 CST: Added because live governed backfill needs all available report periods from
// the current financial provider instead of only the latest row used by interactive analysis.
// Purpose: centralize stock financial-history fetch/parsing for batch data thickening and retraining.
pub fn fetch_live_fundamental_history_rows_for_governed_history(
    symbol: &str,
) -> Result<Vec<GovernedFundamentalHistoryRow>, String> {
    let mut attempt_errors = Vec::new();

    match fetch_live_fundamental_history_rows_from_eastmoney(symbol) {
        Ok(rows) => return Ok(rows),
        Err(error) => attempt_errors.push(format!("eastmoney_financials: {error}")),
    }

    match fetch_live_fundamental_history_rows_from_sina(symbol) {
        Ok(rows) => return Ok(rows),
        Err(error) => attempt_errors.push(format!("sina_financial_guideline: {error}")),
    }

    Err(attempt_errors.join(" | "))
}

// 2026-04-02 CST: 这里把公告抓取也改成三层 provider 链，原因是单一公告源比财报源更容易受 TLS、限流和前端改版影响；
// 目的：把公告摘要稳定在“主源失败仍可继续返回最近公告”的产品语义上，避免上层每次都手动补公告信息。
fn fetch_disclosure_context(
    symbol: &str,
    disclosure_limit: usize,
) -> Result<DisclosureContext, DisclosureFetchError> {
    let mut attempt_errors = Vec::new();

    match fetch_disclosure_from_eastmoney(symbol, disclosure_limit) {
        Ok(context) => return Ok(context),
        Err(error) => attempt_errors.push(format!("eastmoney_announcements: {error}")),
    }

    if let Some(url) = build_optional_official_announcement_url(symbol, disclosure_limit) {
        match fetch_disclosure_from_official_json(&url, disclosure_limit) {
            Ok(context) => return Ok(context),
            Err(error) => attempt_errors.push(format!("official_announcements: {error}")),
        }
    }

    match fetch_disclosure_from_sina(symbol, disclosure_limit) {
        Ok(context) => return Ok(context),
        Err(error) => attempt_errors.push(format!("sina_announcements: {error}")),
    }

    Err(DisclosureFetchError::Transport(attempt_errors.join(" | ")))
}

// 2026-04-14 CST: Added because plan A+ needs paged announcement history for governed storage,
// while fullstack interactive analysis only consumes a short recent slice.
// Purpose: reuse one Eastmoney announcement parser for disclosure-history thickening and retraining prep.
pub fn fetch_live_disclosure_history_rows_for_governed_history(
    symbol: &str,
    page_size: usize,
    max_pages: usize,
) -> Result<Vec<GovernedDisclosureHistoryRow>, String> {
    let mut attempt_errors = Vec::new();

    match fetch_live_disclosure_history_rows_from_eastmoney(symbol, page_size, max_pages) {
        Ok(rows) => return Ok(rows),
        Err(error) => attempt_errors.push(format!("eastmoney_announcements: {error}")),
    }

    match fetch_live_disclosure_history_rows_from_sina(symbol, page_size, max_pages) {
        Ok(rows) => return Ok(rows),
        Err(error) => attempt_errors.push(format!("sina_announcements: {error}")),
    }

    Err(attempt_errors.join(" | "))
}

// 2026-04-14 CST: Added because governed financial-history backfill cannot stay on a single free
// provider if we want practical real-trading preparation under unstable public network conditions.
// Purpose: keep Eastmoney as the first structured source, but allow later fallback branches.
fn fetch_live_fundamental_history_rows_from_eastmoney(
    symbol: &str,
) -> Result<Vec<GovernedFundamentalHistoryRow>, String> {
    let url = build_eastmoney_financial_url(symbol);
    let body = http_get_text(&url, "financials").map_err(|error| error.to_string())?;
    let payload = serde_json::from_str::<Value>(&body).map_err(|error| error.to_string())?;
    let rows = payload
        .as_array()
        .or_else(|| payload.get("data").and_then(Value::as_array))
        .ok_or_else(|| "financial history payload does not contain a row array".to_string())?;

    let parsed_rows = rows
        .iter()
        .filter_map(|row| {
            let report_period = financial_string(
                row,
                &["REPORT_DATE", "REPORTDATE", "REPORT_DATE_NAME", "date"],
            )?;
            Some(GovernedFundamentalHistoryRow {
                report_period: normalize_date_like(report_period),
                notice_date: financial_string(
                    row,
                    &["NOTICE_DATE", "NOTICEDATE", "latestNoticeDate"],
                )
                .map(normalize_date_like),
                source: "eastmoney_financials".to_string(),
                report_metrics: FundamentalMetrics {
                    revenue: financial_number(
                        row,
                        &[
                            "TOTAL_OPERATE_INCOME",
                            "TOTALOPERATEINCOME",
                            "营业总收入",
                            "yyzsr",
                        ],
                    ),
                    revenue_yoy_pct: financial_number(
                        row,
                        &["YSTZ", "YYZSR_GTHR", "营业总收入同比"],
                    ),
                    net_profit: financial_number(
                        row,
                        &["PARENT_NETPROFIT", "PARENTNETPROFIT", "归母净利润", "gsjlr"],
                    ),
                    net_profit_yoy_pct: financial_number(
                        row,
                        &["SJLTZ", "NETPROFIT_GTHR", "归母净利润同比"],
                    ),
                    roe_pct: financial_number(row, &["ROEJQ", "ROE_WEIGHTED", "jqjzcsyl"]),
                },
            })
        })
        .collect::<Vec<_>>();

    if parsed_rows.is_empty() {
        return Err(
            "financial history payload does not contain any usable report rows".to_string(),
        );
    }

    Ok(parsed_rows)
}

// 2026-04-14 CST: Added because Sina already serves as the resilient interactive fallback, but
// the governed history path previously dropped that resilience and blocked the whole batch.
// Purpose: parse multi-period Sina financial rows into governed history records when Eastmoney fails.
fn fetch_live_fundamental_history_rows_from_sina(
    symbol: &str,
) -> Result<Vec<GovernedFundamentalHistoryRow>, String> {
    let url = build_sina_financial_url(symbol);
    let body = http_get_text(&url, "sina_financials").map_err(|error| error.to_string())?;
    let rows = parse_html_rows_with_raw(&body);
    if rows.is_empty() {
        return Err("sina financial page does not contain any table rows".to_string());
    }

    let mut report_periods: Vec<String> = Vec::new();
    let mut revenue_values: Vec<Option<f64>> = Vec::new();
    let mut revenue_yoy_values: Vec<Option<f64>> = Vec::new();
    let mut net_profit_values: Vec<Option<f64>> = Vec::new();
    let mut net_profit_yoy_values: Vec<Option<f64>> = Vec::new();
    let mut roe_values: Vec<Option<f64>> = Vec::new();

    for row in rows {
        let Some(label) = row.cells.first().map(|value| value.trim()) else {
            continue;
        };
        let normalized_label = normalize_sina_financial_label(label);
        let numeric_values = collect_sina_numeric_columns(&row.cells);

        if is_sina_report_period_row(&normalized_label, &row.cells) {
            report_periods = row
                .cells
                .iter()
                .skip(1)
                .map(|value| value.trim())
                .filter(|value| looks_like_report_date(value))
                .map(|value| normalize_date_like(value.to_string()))
                .collect::<Vec<_>>();
            continue;
        }
        if is_sina_financial_metric_row(
            &normalized_label,
            &row.html,
            &[],
            &["营业总收入元", "主营业务收入元", "营业收入元"],
        ) {
            revenue_values = numeric_values;
            continue;
        }
        if is_sina_financial_metric_row(
            &normalized_label,
            &row.html,
            &["financialratios43"],
            &["营业总收入增长率", "主营业务收入增长率", "营业收入增长率"],
        ) {
            revenue_yoy_values = numeric_values;
            continue;
        }
        if is_sina_financial_metric_row(
            &normalized_label,
            &row.html,
            &["financialratios57", "financialratios65"],
            &["归母净利润元", "净利润元", "扣除非经常性损益后的净利润元"],
        ) {
            net_profit_values = numeric_values;
            continue;
        }
        if is_sina_financial_metric_row(
            &normalized_label,
            &row.html,
            &["financialratios44"],
            &["归母净利润增长率", "净利润增长率"],
        ) {
            net_profit_yoy_values = numeric_values;
            continue;
        }
        if is_sina_financial_metric_row(
            &normalized_label,
            &row.html,
            &["financialratios59", "financialratios62"],
            &["净资产收益率", "加权净资产收益率"],
        ) {
            roe_values = numeric_values;
        }
    }

    if report_periods.is_empty() {
        return Err("sina financial page does not expose any report periods".to_string());
    }

    let parsed_rows = report_periods
        .iter()
        .enumerate()
        .map(|(index, report_period)| GovernedFundamentalHistoryRow {
            report_period: report_period.clone(),
            notice_date: None,
            source: "sina_financial_guideline".to_string(),
            report_metrics: FundamentalMetrics {
                revenue: revenue_values.get(index).cloned().flatten(),
                revenue_yoy_pct: revenue_yoy_values.get(index).cloned().flatten(),
                net_profit: net_profit_values.get(index).cloned().flatten(),
                net_profit_yoy_pct: net_profit_yoy_values.get(index).cloned().flatten(),
                roe_pct: roe_values.get(index).cloned().flatten(),
            },
        })
        .collect::<Vec<_>>();

    if parsed_rows.is_empty() {
        return Err("sina financial page does not contain any usable report rows".to_string());
    }

    Ok(parsed_rows)
}

// 2026-04-14 CST: Added because the governed announcement-history path also needs the same
// provider resilience as interactive analysis if it is going to support real-trading prep.
// Purpose: keep the original Eastmoney paging logic as the first structured source.
fn fetch_live_disclosure_history_rows_from_eastmoney(
    symbol: &str,
    page_size: usize,
    max_pages: usize,
) -> Result<Vec<GovernedDisclosureHistoryRow>, String> {
    let normalized_page_size = page_size.max(1).min(200);
    let normalized_max_pages = max_pages.max(1);
    let mut collected = Vec::new();

    for page_index in 1..=normalized_max_pages {
        let url = build_eastmoney_announcement_page_url(symbol, normalized_page_size, page_index);
        let body = http_get_text(&url, "announcements").map_err(|error| error.to_string())?;
        let payload = serde_json::from_str::<Value>(&body).map_err(|error| error.to_string())?;
        let notices = extract_announcement_list(&payload).map_err(|error| error.to_string())?;
        if notices.is_empty() {
            break;
        }

        collected.extend(notices.into_iter().filter_map(|notice| {
            let published_at = notice.published_at.map(normalize_date_like)?;
            Some(GovernedDisclosureHistoryRow {
                published_at,
                title: notice.title,
                article_code: notice.article_code,
                category: notice.category,
                source: "eastmoney_announcements".to_string(),
            })
        }));
    }

    if collected.is_empty() {
        return Err("announcement history payload does not contain any usable rows".to_string());
    }

    Ok(collected)
}

// 2026-04-14 CST: Added because the interactive disclosure chain already knows how to parse the
// Sina bulletin page, but the governed history path previously did not reuse that fallback.
// Purpose: recover recent announcement history from the public Sina page when Eastmoney is unstable.
fn fetch_live_disclosure_history_rows_from_sina(
    symbol: &str,
    page_size: usize,
    max_pages: usize,
) -> Result<Vec<GovernedDisclosureHistoryRow>, String> {
    let url = build_sina_announcement_url(symbol);
    let body = http_get_text(&url, "sina_announcements").map_err(|error| error.to_string())?;
    let regex = Regex::new(
        r#"(?is)(\d{4}-\d{2}-\d{2})\s*&nbsp;\s*<a[^>]*href=['"]([^'"]+)['"][^>]*>(.*?)</a>"#,
    )
    .map_err(|error| error.to_string())?;

    let limit = page_size.max(1) * max_pages.max(1);
    let rows = regex
        .captures_iter(&body)
        .filter_map(|capture| {
            let title = strip_html_tags(capture.get(3)?.as_str());
            if title.trim().is_empty() {
                return None;
            }
            let href = capture.get(2)?.as_str().trim();
            Some(GovernedDisclosureHistoryRow {
                published_at: normalize_date_like(capture.get(1)?.as_str().trim().to_string()),
                title,
                article_code: extract_query_param(href, "id")
                    .or_else(|| Some(to_absolute_sina_url(href))),
                category: Some("公司公告".to_string()),
                source: "sina_announcements".to_string(),
            })
        })
        .take(limit)
        .collect::<Vec<_>>();

    if rows.is_empty() {
        return Err("sina announcement page does not contain any usable rows".to_string());
    }

    Ok(rows)
}

// 2026-04-13 CST: 这里新增 ETF 公开事实层抓取入口，原因是证券主链已经要求 ETF 与股票一起进入正式主链，
// 不能继续只有“识别出 ETF”，却没有 ETF 自己的事实层。
// 目的：先收口基金名称、基准、规模、份额与折溢价这批最小正式字段。
fn fetch_etf_context(symbol: &str) -> EtfContext {
    if !is_etf_symbol(symbol) {
        return build_not_applicable_etf_context();
    }

    match fetch_etf_context_from_public_facts(symbol) {
        Ok(context) => context,
        Err(error) => build_builtin_etf_context(symbol)
            .unwrap_or_else(|| build_unavailable_etf_context(error.to_string())),
    }
}

// 2026-04-15 CST: Added because cross-border ETF analysis should be driven by the
// penetrated underlying market first, then FX, then ETF premium.
// Reason: using only the ETF local candle as the primary object was methodologically wrong
// for products such as Nikkei or other overseas-linked ETFs.
// Purpose: build one formal cross-border ETF context that later governance layers can reuse.
fn build_cross_border_context(
    request: &SecurityAnalysisFullstackRequest,
    technical_context: &SecurityAnalysisContextualResult,
    etf_context: &EtfContext,
) -> CrossBorderEtfContext {
    if !is_cross_border_etf(request, etf_context) {
        return CrossBorderEtfContext {
            status: "not_applicable".to_string(),
            analysis_method: "underlying_first_cross_border_etf_v1".to_string(),
            underlying_market: build_unavailable_cross_border_leg(
                "not_applicable",
                None,
                "当前标的不属于跨境 ETF，不生成穿透标的分析层。",
                Vec::new(),
            ),
            fx_market: build_unavailable_cross_border_leg(
                "not_applicable",
                None,
                "当前标的不属于跨境 ETF，不生成 FX 分析层。",
                Vec::new(),
            ),
            premium_assessment: CrossBorderPremiumAssessment {
                status: "not_applicable".to_string(),
                premium_discount_rate_pct: None,
                verdict: "not_applicable".to_string(),
                headline: "当前标的不属于跨境 ETF，不生成折溢价映射层。".to_string(),
                risk_flags: Vec::new(),
            },
            resonance_verdict: "not_applicable".to_string(),
            headline: "当前标的不属于跨境 ETF，不启用穿透市场优先规则。".to_string(),
            rationale: Vec::new(),
            risk_flags: Vec::new(),
        };
    }

    let resolved_underlying_symbol =
        resolve_cross_border_leg_symbol(request, etf_context, "underlying");
    let resolved_fx_symbol = resolve_cross_border_leg_symbol(request, etf_context, "fx");
    let underlying_market = build_cross_border_leg_analysis(
        resolved_underlying_symbol.as_deref(),
        request,
        "underlying",
    );
    let fx_market = build_cross_border_leg_analysis(resolved_fx_symbol.as_deref(), request, "fx");
    let premium_assessment =
        build_cross_border_premium_assessment(etf_context.premium_discount_rate_pct);

    let underlying_state = classify_cross_border_leg_state(underlying_market.bias.as_deref());
    let fx_state = classify_cross_border_leg_state(fx_market.bias.as_deref());
    let premium_state = premium_assessment.verdict.as_str();

    let resonance_verdict =
        if underlying_market.status != "available" || fx_market.status != "available" {
            "incomplete_underlying_chain".to_string()
        } else if premium_state == "overheated" {
            "avoid_chasing_high_premium".to_string()
        } else if underlying_state == "supportive"
            && fx_state != "adverse"
            && premium_state == "favorable"
        {
            "underlying_first_confirmed".to_string()
        } else if underlying_state == "supportive" && fx_state != "adverse" {
            "buy_on_pullback_only".to_string()
        } else if underlying_state == "neutral" || fx_state == "neutral" {
            "watch_underlying_confirmation".to_string()
        } else {
            "underlying_not_supportive".to_string()
        };

    let mut rationale = vec![format!(
        "本轮跨境 ETF 先看穿透标的 {}，再看汇率层 {}，最后再把结果映射到 ETF 折溢价。",
        underlying_market
            .symbol
            .clone()
            .unwrap_or_else(|| "待补 underlying_symbol".to_string()),
        fx_market
            .symbol
            .clone()
            .unwrap_or_else(|| "待补 fx_symbol".to_string())
    )];
    rationale.extend(underlying_market.rationale.clone());
    rationale.extend(fx_market.rationale.clone());
    rationale.push(premium_assessment.headline.clone());
    rationale.push(format!(
        "ETF 自身本地技术面 {} 只作为映射层参考，不再替代穿透市场判断。",
        technical_context.contextual_conclusion.alignment
    ));

    let mut risk_flags = Vec::new();
    risk_flags.extend(underlying_market.risk_flags.clone());
    risk_flags.extend(fx_market.risk_flags.clone());
    risk_flags.extend(premium_assessment.risk_flags.clone());
    if resonance_verdict == "incomplete_underlying_chain" {
        risk_flags.push(
            "跨境 ETF 缺少穿透标的或汇率分析输入，当前不能把 ETF 本地走势直接当成主判断。"
                .to_string(),
        );
    }

    let headline = match resonance_verdict.as_str() {
        "underlying_first_confirmed" => {
            "穿透标的与汇率层均提供支撑，且折溢价仍可控，当前可按跨境 ETF 正统顺序做积极跟踪。"
                .to_string()
        }
        "buy_on_pullback_only" => {
            "穿透标的仍偏支持，但折溢价已不在最优区，当前更适合回踩确认后执行。".to_string()
        }
        "watch_underlying_confirmation" => {
            "穿透标的或汇率层仍在确认阶段，当前不应只凭 ETF 本地价格动作做激进判断。".to_string()
        }
        "avoid_chasing_high_premium" => {
            "穿透标的即使不弱，当前折溢价层也已过热，规则上不支持追价。".to_string()
        }
        "incomplete_underlying_chain" => {
            "跨境 ETF 穿透分析输入尚不完整，当前只能保留为不完整的观察结论。".to_string()
        }
        _ => "穿透标的当前未形成正向支撑，跨境 ETF 不宜仅凭本地 K 线抬高结论。".to_string(),
    };

    CrossBorderEtfContext {
        status: if resonance_verdict == "incomplete_underlying_chain" {
            "incomplete".to_string()
        } else {
            "available".to_string()
        },
        analysis_method: "underlying_first_cross_border_etf_v1".to_string(),
        underlying_market,
        fx_market,
        premium_assessment,
        resonance_verdict,
        headline,
        rationale,
        risk_flags,
    }
}

// 2026-04-15 CST: Added because the user explicitly asked for the cross-border
// ETF mapping rule to live inside the formal chain rather than operator memory.
// Purpose: keep the current Nikkei-linked ETF family on one canonical default mapping path.
fn resolve_cross_border_leg_symbol(
    request: &SecurityAnalysisFullstackRequest,
    etf_context: &EtfContext,
    leg_kind: &str,
) -> Option<String> {
    let explicit_symbol = if leg_kind == "underlying" {
        request.underlying_symbol.as_ref()
    } else {
        request.fx_symbol.as_ref()
    };
    if let Some(explicit_symbol) = explicit_symbol
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        return Some(explicit_symbol.to_string());
    }

    default_cross_border_mapping(request, etf_context).map(|mapping| {
        if leg_kind == "underlying" {
            mapping.underlying_symbol
        } else {
            mapping.fx_symbol
        }
    })
}

fn default_cross_border_mapping(
    request: &SecurityAnalysisFullstackRequest,
    etf_context: &EtfContext,
) -> Option<CrossBorderDefaultMapping> {
    let normalized_symbol = request.symbol.trim().to_uppercase();
    let sector_profile = request
        .sector_profile
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let benchmark = etf_context
        .benchmark
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();

    if normalized_symbol == "159866.SZ"
        || sector_profile.contains("nikkei_qdii_cross_border_peer")
        || benchmark.contains("nikkei")
        || etf_context
            .benchmark
            .as_deref()
            .unwrap_or_default()
            .contains("日经")
    {
        return Some(CrossBorderDefaultMapping {
            underlying_symbol: "NK225.IDX".to_string(),
            fx_symbol: "JPYCNY.FX".to_string(),
        });
    }

    None
}

struct CrossBorderDefaultMapping {
    underlying_symbol: String,
    fx_symbol: String,
}

fn build_cross_border_leg_analysis(
    symbol: Option<&str>,
    request: &SecurityAnalysisFullstackRequest,
    leg_kind: &str,
) -> CrossBorderLegAnalysis {
    let Some(symbol) = symbol.map(str::trim).filter(|value| !value.is_empty()) else {
        return build_unavailable_cross_border_leg(
            "missing_input",
            None,
            if leg_kind == "underlying" {
                "缺少 underlying_symbol，当前无法先对穿透标的做正式技术判断。"
            } else {
                "缺少 fx_symbol，当前无法补齐跨境 ETF 的汇率判断层。"
            },
            Vec::new(),
        );
    };

    let basic_request = TechnicalConsultationBasicRequest {
        symbol: symbol.to_string(),
        as_of_date: request.as_of_date.clone(),
        lookback_days: request
            .lookback_days
            .max(DEFAULT_CROSS_BORDER_LOOKBACK_DAYS),
    };

    match technical_consultation_basic(&basic_request) {
        Ok(result) => build_available_cross_border_leg(&result),
        Err(error) => build_unavailable_cross_border_leg(
            "analysis_failed",
            Some(symbol.to_string()),
            if leg_kind == "underlying" {
                "穿透标的分析失败，当前不能把 ETF 本地走势直接当成主结论。"
            } else {
                "FX 层分析失败，当前跨境 ETF 结论缺少汇率支撑判断。"
            },
            vec![error.to_string()],
        ),
    }
}

fn build_available_cross_border_leg(
    result: &TechnicalConsultationBasicResult,
) -> CrossBorderLegAnalysis {
    CrossBorderLegAnalysis {
        status: "available".to_string(),
        symbol: Some(result.symbol.clone()),
        bias: Some(result.consultation_conclusion.bias.clone()),
        confidence: Some(result.consultation_conclusion.confidence.clone()),
        headline: result.consultation_conclusion.headline.clone(),
        support_level_20: Some(result.indicator_snapshot.support_level_20),
        resistance_level_20: Some(result.indicator_snapshot.resistance_level_20),
        rationale: result.consultation_conclusion.rationale.clone(),
        risk_flags: result.consultation_conclusion.risk_flags.clone(),
    }
}

fn build_unavailable_cross_border_leg(
    status: &str,
    symbol: Option<String>,
    headline: &str,
    risk_flags: Vec<String>,
) -> CrossBorderLegAnalysis {
    CrossBorderLegAnalysis {
        status: status.to_string(),
        symbol,
        bias: None,
        confidence: None,
        headline: headline.to_string(),
        support_level_20: None,
        resistance_level_20: None,
        rationale: Vec::new(),
        risk_flags,
    }
}

fn build_cross_border_premium_assessment(
    premium_discount_rate_pct: Option<f64>,
) -> CrossBorderPremiumAssessment {
    match premium_discount_rate_pct {
        Some(value) if value <= 1.0 => CrossBorderPremiumAssessment {
            status: "available".to_string(),
            premium_discount_rate_pct: Some(value),
            verdict: "favorable".to_string(),
            headline: format!(
                "当前折溢价 {:.2}% 仍处可接受区间，规则上允许把穿透市场结论映射到 ETF 执行层。",
                value
            ),
            risk_flags: Vec::new(),
        },
        Some(value) if value <= 2.0 => CrossBorderPremiumAssessment {
            status: "available".to_string(),
            premium_discount_rate_pct: Some(value),
            verdict: "watch".to_string(),
            headline: format!(
                "当前折溢价 {:.2}% 已不在最优低位，若执行更适合回踩确认而不是直接追价。",
                value
            ),
            risk_flags: vec![
                "ETF 当前不在最优低溢价区，映射执行应降级为回踩买而不是追高。".to_string(),
            ],
        },
        Some(value) => CrossBorderPremiumAssessment {
            status: "available".to_string(),
            premium_discount_rate_pct: Some(value),
            verdict: "overheated".to_string(),
            headline: format!(
                "当前折溢价 {:.2}% 已过热，即使穿透市场偏强也不支持激进追仓。",
                value
            ),
            risk_flags: vec![
                "折溢价已过热，跨境 ETF 容易把海外强势提前透支到本地成交价格。".to_string(),
            ],
        },
        None => CrossBorderPremiumAssessment {
            status: "unavailable".to_string(),
            premium_discount_rate_pct: None,
            verdict: "incomplete".to_string(),
            headline: "当前缺少 ETF 折溢价字段，穿透市场结论无法完整映射到执行层。".to_string(),
            risk_flags: vec!["缺少正式折溢价输入，跨境 ETF 建议不能视作完整执行结论。".to_string()],
        },
    }
}

fn is_cross_border_etf(
    request: &SecurityAnalysisFullstackRequest,
    etf_context: &EtfContext,
) -> bool {
    request
        .market_profile
        .as_deref()
        .map(is_cross_border_profile)
        .unwrap_or(false)
        || request
            .sector_profile
            .as_deref()
            .map(is_cross_border_profile)
            .unwrap_or(false)
        || etf_context
            .asset_scope
            .as_deref()
            .map(is_cross_border_asset_scope)
            .unwrap_or(false)
        || etf_context
            .benchmark
            .as_deref()
            .map(is_cross_border_benchmark)
            .unwrap_or(false)
}

fn is_cross_border_profile(profile: &str) -> bool {
    let normalized = profile.to_ascii_lowercase();
    normalized.contains("cross_border")
        || normalized.contains("cross-border")
        || normalized.contains("qdii")
        || normalized.contains("overseas")
}

fn is_cross_border_asset_scope(asset_scope: &str) -> bool {
    let normalized = asset_scope.to_ascii_lowercase();
    normalized.contains("cross")
        || normalized.contains("overseas")
        || asset_scope.contains("跨境")
        || asset_scope.contains("海外")
}

fn is_cross_border_benchmark(benchmark: &str) -> bool {
    let normalized = benchmark.to_ascii_lowercase();
    normalized.contains("nikkei")
        || normalized.contains("hang seng")
        || normalized.contains("s&p")
        || benchmark.contains("日经")
        || benchmark.contains("恒生")
        || benchmark.contains("海外")
}

fn classify_cross_border_leg_state(bias: Option<&str>) -> &'static str {
    match bias.unwrap_or_default() {
        "bullish_continuation" => "supportive",
        "bearish_continuation" => "adverse",
        _ => "neutral",
    }
}

fn fetch_etf_context_from_public_facts(symbol: &str) -> Result<EtfContext, EtfFetchError> {
    let url = build_optional_etf_facts_url(symbol)
        .ok_or_else(|| EtfFetchError::Transport("未配置 ETF 公共事实源 URL".to_string()))?;
    let body = http_get_text(&url, "etf_facts").map_err(EtfFetchError::Transport)?;
    let payload = serde_json::from_str::<Value>(&body)
        .map_err(|error| EtfFetchError::Parse(error.to_string()))?;
    let root = extract_first_object(&payload).ok_or(EtfFetchError::Empty)?;

    let fund_name = json_string(root, &["fund_name", "name", "security_name"]);
    let benchmark = json_string(root, &["benchmark", "tracking_index", "index_name"]);
    let asset_scope = json_string(root, &["asset_scope", "fund_type", "investment_scope"]);
    let latest_scale = json_number(root, &["latest_scale", "fund_scale", "scale"]);
    let latest_share = json_number(root, &["latest_share", "fund_share", "share"]);
    let premium_discount_rate_pct = json_number(
        root,
        &[
            "premium_discount_rate_pct",
            "premium_rate_pct",
            "discount_rate_pct",
            "premium_discount_pct",
        ],
    );
    let structure_risk_flags = json_string_array(
        root,
        &[
            "structure_risk_flags",
            "risk_flags",
            "structure_risks",
            "special_risks",
        ],
    );
    let mut research_gaps = json_string_array(
        root,
        &["research_gaps", "data_gaps", "missing_fields", "known_gaps"],
    );

    if benchmark.is_none() {
        research_gaps.push("缺少 ETF 跟踪基准字段。".to_string());
    }
    if premium_discount_rate_pct.is_none() {
        research_gaps.push("缺少 ETF 折溢价字段。".to_string());
    }
    if latest_scale.is_none() {
        research_gaps.push("缺少 ETF 最新规模字段。".to_string());
    }

    dedupe_strings(&mut research_gaps);

    Ok(EtfContext {
        status: "available".to_string(),
        source: "public_etf_facts".to_string(),
        fund_name: fund_name.clone(),
        benchmark: benchmark.clone(),
        asset_scope: asset_scope.clone(),
        latest_scale,
        latest_share,
        premium_discount_rate_pct,
        headline: build_etf_headline(
            fund_name.as_deref(),
            benchmark.as_deref(),
            asset_scope.as_deref(),
            latest_scale,
            premium_discount_rate_pct,
        ),
        structure_risk_flags,
        research_gaps,
    })
}

// 2026-04-15 CST: Added because scheme B-2 must not let one missing URL
// configuration erase the entire ETF facts layer for the currently supported
// cross-border ETF family.
// Purpose: provide a minimal built-in structural facts fallback until a wider live-source registry lands.
fn build_builtin_etf_context(symbol: &str) -> Option<EtfContext> {
    let normalized_symbol = symbol.trim().to_uppercase();
    if normalized_symbol != "159866.SZ" {
        return None;
    }

    let fund_name = Some("日经ETF工银".to_string());
    let benchmark = Some("日经225指数".to_string());
    let asset_scope = Some("跨境股票ETF".to_string());
    let mut research_gaps = vec![
        "当前使用内置 ETF 事实兜底，仍建议后续补 live facts URL。".to_string(),
        "当前缺少 ETF 实时折溢价字段。".to_string(),
        "当前缺少 ETF 最新规模/份额字段。".to_string(),
    ];
    dedupe_strings(&mut research_gaps);

    Some(EtfContext {
        status: "available".to_string(),
        source: "builtin_etf_registry".to_string(),
        fund_name: fund_name.clone(),
        benchmark: benchmark.clone(),
        asset_scope: asset_scope.clone(),
        latest_scale: None,
        latest_share: None,
        premium_discount_rate_pct: None,
        headline: build_etf_headline(
            fund_name.as_deref(),
            benchmark.as_deref(),
            asset_scope.as_deref(),
            None,
            None,
        ),
        structure_risk_flags: vec![],
        research_gaps,
    })
}

fn fetch_fundamental_from_eastmoney(
    symbol: &str,
) -> Result<FundamentalContext, FundamentalFetchError> {
    let url = build_eastmoney_financial_url(symbol);
    let body = http_get_text(&url, "financials").map_err(FundamentalFetchError::Transport)?;
    let payload = serde_json::from_str::<Value>(&body)
        .map_err(|error| FundamentalFetchError::Parse(error.to_string()))?;
    let latest = extract_latest_financial_row(&payload)?;

    let latest_report_period = financial_string(
        latest,
        &["REPORT_DATE", "REPORTDATE", "REPORT_DATE_NAME", "date"],
    )
    .map(normalize_date_like);
    let report_notice_date =
        financial_string(latest, &["NOTICE_DATE", "NOTICEDATE", "latestNoticeDate"])
            .map(normalize_date_like);
    let metrics = FundamentalMetrics {
        revenue: financial_number(
            latest,
            &[
                "TOTAL_OPERATE_INCOME",
                "TOTALOPERATEINCOME",
                "营业总收入",
                "yyzsr",
            ],
        ),
        revenue_yoy_pct: financial_number(latest, &["YSTZ", "YYZSR_GTHR", "营业总收入同比"]),
        net_profit: financial_number(
            latest,
            &["PARENT_NETPROFIT", "PARENTNETPROFIT", "归母净利润", "gsjlr"],
        ),
        net_profit_yoy_pct: financial_number(
            latest,
            &["SJLTZ", "NETPROFIT_GTHR", "归母净利润同比"],
        ),
        roe_pct: financial_number(latest, &["ROEJQ", "ROE_WEIGHTED", "jqjzcsyl"]),
    };

    Ok(build_available_fundamental_context(
        "eastmoney_financials",
        latest_report_period,
        report_notice_date,
        metrics,
    ))
}

fn fetch_fundamental_from_official_json(
    url: &str,
) -> Result<FundamentalContext, FundamentalFetchError> {
    let body =
        http_get_text(url, "official_financials").map_err(FundamentalFetchError::Transport)?;
    let payload = serde_json::from_str::<Value>(&body)
        .map_err(|error| FundamentalFetchError::Parse(error.to_string()))?;
    let root = extract_first_object(&payload).ok_or(FundamentalFetchError::Empty)?;
    let metrics_node = root
        .get("report_metrics")
        .or_else(|| root.get("metrics"))
        .unwrap_or(root);
    let metrics = FundamentalMetrics {
        revenue: json_number(
            metrics_node,
            &["revenue", "operate_income", "total_operate_income"],
        ),
        revenue_yoy_pct: json_number(metrics_node, &["revenue_yoy_pct", "operate_income_yoy_pct"]),
        net_profit: json_number(metrics_node, &["net_profit", "parent_netprofit", "profit"]),
        net_profit_yoy_pct: json_number(metrics_node, &["net_profit_yoy_pct", "profit_yoy_pct"]),
        roe_pct: json_number(metrics_node, &["roe_pct", "roe"]),
    };
    if metrics == empty_fundamental_metrics()
        && json_string(root, &["latest_report_period", "report_period"]).is_none()
    {
        return Err(FundamentalFetchError::Empty);
    }

    Ok(build_available_fundamental_context(
        root.get("source")
            .and_then(Value::as_str)
            .unwrap_or("official_financials"),
        json_string(root, &["latest_report_period", "report_period"]).map(normalize_date_like),
        json_string(root, &["report_notice_date", "notice_date"]).map(normalize_date_like),
        metrics,
    ))
}

#[allow(dead_code)]
fn fetch_fundamental_from_sina(symbol: &str) -> Result<FundamentalContext, FundamentalFetchError> {
    let url = build_sina_financial_url(symbol);
    let body = http_get_text(&url, "sina_financials").map_err(FundamentalFetchError::Transport)?;
    let rows = parse_html_rows_with_raw(&body);
    if rows.is_empty() {
        return Err(FundamentalFetchError::Parse(
            "新浪财务页没有可解析表格".to_string(),
        ));
    }

    let mut latest_report_period = None;
    let mut metrics = FundamentalMetrics {
        revenue: None,
        revenue_yoy_pct: None,
        net_profit: None,
        net_profit_yoy_pct: None,
        roe_pct: None,
    };

    for row in rows {
        let Some(label) = row.cells.first().map(|value| value.trim()) else {
            continue;
        };
        // 2026-04-02 CST: 这里给新浪财报解析补“标签归一化 + typecode 锚点”双保险，原因是线上页面在当前链路里可能出现乱码标签；
        // 目的：只要关键行的结构锚点还在，就继续识别净利增速、ROE 和报告期，避免财报备源被误判为空。
        let _normalized_label = normalize_sina_financial_label(label);
        let first_value = row
            .cells
            .iter()
            .skip(1)
            .find(|value| !value.trim().is_empty() && value.trim() != "--")
            .map(|value| value.trim().to_string());

        if label.contains("报告日期") {
            latest_report_period = first_value.map(normalize_date_like);
            continue;
        }
        if label.contains("营业总收入(元)")
            || label.contains("主营业务收入(元)")
            || label.contains("营业收入(元)")
        {
            metrics.revenue = first_value.as_deref().and_then(parse_number_text);
            continue;
        }
        if label.contains("营业总收入增长率(%)")
            || label.contains("主营业务收入增长率(%)")
            || label.contains("营业收入增长率(%)")
        {
            metrics.revenue_yoy_pct = first_value.as_deref().and_then(parse_number_text);
            continue;
        }
        if label.contains("归母净利润(元)") || label.contains("净利润(元)") {
            metrics.net_profit = first_value.as_deref().and_then(parse_number_text);
            continue;
        }
        if label.contains("归母净利润增长率(%)") || label.contains("净利润增长率(%)")
        {
            metrics.net_profit_yoy_pct = first_value.as_deref().and_then(parse_number_text);
            continue;
        }
        if label.contains("净资产收益率(%)") || label.contains("加权净资产收益率(%)")
        {
            if metrics.roe_pct.is_none() {
                metrics.roe_pct = first_value.as_deref().and_then(parse_number_text);
            }
        }
    }

    if latest_report_period.is_none() && metrics == empty_fundamental_metrics() {
        return Err(FundamentalFetchError::Empty);
    }

    Ok(build_available_fundamental_context(
        "sina_financial_guideline",
        latest_report_period,
        None,
        metrics,
    ))
}

// 2026-04-02 CST: 这里新增更稳的新浪财报解析分支，原因是线上真实页面在当前链路里会出现乱码标签，但 HTML 结构与 typecode 仍稳定；
// 目的：通过“日期行识别 + financialratios typecode + 归一化标签”三层兜底，让财报备源在真实环境中恢复可用。
fn fetch_fundamental_from_sina_resilient(
    symbol: &str,
) -> Result<FundamentalContext, FundamentalFetchError> {
    let url = build_sina_financial_url(symbol);
    let body = http_get_text(&url, "sina_financials").map_err(FundamentalFetchError::Transport)?;
    let rows = parse_html_rows_with_raw(&body);
    if rows.is_empty() {
        return Err(FundamentalFetchError::Parse(
            "新浪财务页没有可解析表格".to_string(),
        ));
    }

    let mut latest_report_period = None;
    let mut metrics = FundamentalMetrics {
        revenue: None,
        revenue_yoy_pct: None,
        net_profit: None,
        net_profit_yoy_pct: None,
        roe_pct: None,
    };

    for row in rows {
        let Some(label) = row.cells.first().map(|value| value.trim()) else {
            continue;
        };
        let normalized_label = normalize_sina_financial_label(label);
        let first_value = row
            .cells
            .iter()
            .skip(1)
            .find(|value| !value.trim().is_empty() && value.trim() != "--")
            .map(|value| value.trim().to_string());

        if is_sina_report_period_row(&normalized_label, &row.cells) {
            latest_report_period = first_value.map(normalize_date_like);
            continue;
        }
        if is_sina_financial_metric_row(
            &normalized_label,
            &row.html,
            &[],
            &["营业总收入元", "主营业务收入元", "营业收入元"],
        ) {
            metrics.revenue = first_value.as_deref().and_then(parse_number_text);
            continue;
        }
        if is_sina_financial_metric_row(
            &normalized_label,
            &row.html,
            &["financialratios43"],
            &["营业总收入增长率", "主营业务收入增长率", "营业收入增长率"],
        ) {
            metrics.revenue_yoy_pct = first_value.as_deref().and_then(parse_number_text);
            continue;
        }
        if is_sina_financial_metric_row(
            &normalized_label,
            &row.html,
            &["financialratios57", "financialratios65"],
            &["归母净利润元", "净利润元", "扣除非经常性损益后的净利润元"],
        ) {
            metrics.net_profit = first_value.as_deref().and_then(parse_number_text);
            continue;
        }
        if is_sina_financial_metric_row(
            &normalized_label,
            &row.html,
            &["financialratios44"],
            &["归母净利润增长率", "净利润增长率"],
        ) {
            metrics.net_profit_yoy_pct = first_value.as_deref().and_then(parse_number_text);
            continue;
        }
        if is_sina_financial_metric_row(
            &normalized_label,
            &row.html,
            &["financialratios59", "financialratios62"],
            &["净资产收益率", "加权净资产收益率"],
        ) {
            if metrics.roe_pct.is_none() {
                metrics.roe_pct = first_value.as_deref().and_then(parse_number_text);
            }
        }
    }

    if latest_report_period.is_none() && metrics == empty_fundamental_metrics() {
        return Err(FundamentalFetchError::Empty);
    }

    Ok(build_available_fundamental_context(
        "sina_financial_guideline",
        latest_report_period,
        None,
        metrics,
    ))
}

fn fetch_disclosure_from_eastmoney(
    symbol: &str,
    disclosure_limit: usize,
) -> Result<DisclosureContext, DisclosureFetchError> {
    let url = build_eastmoney_announcement_url(symbol, disclosure_limit);
    let body = http_get_text(&url, "announcements").map_err(DisclosureFetchError::Transport)?;
    let payload = serde_json::from_str::<Value>(&body)
        .map_err(|error| DisclosureFetchError::Parse(error.to_string()))?;
    let notices = extract_announcement_list(&payload)?;
    if notices.is_empty() {
        return Err(DisclosureFetchError::Empty);
    }

    Ok(build_available_disclosure_context(
        "eastmoney_announcements",
        notices,
        disclosure_limit,
    ))
}

fn fetch_disclosure_from_official_json(
    url: &str,
    disclosure_limit: usize,
) -> Result<DisclosureContext, DisclosureFetchError> {
    let body =
        http_get_text(url, "official_announcements").map_err(DisclosureFetchError::Transport)?;
    let payload = serde_json::from_str::<Value>(&body)
        .map_err(|error| DisclosureFetchError::Parse(error.to_string()))?;
    let root = extract_first_object(&payload).ok_or(DisclosureFetchError::Empty)?;
    let list = root
        .get("recent_announcements")
        .or_else(|| root.get("announcements"))
        .and_then(Value::as_array)
        .ok_or_else(|| {
            DisclosureFetchError::Parse("官方公告备源缺少 announcements 数组".to_string())
        })?;

    let notices = list
        .iter()
        .filter_map(|item| {
            let title = json_string(item, &["title"])?;
            Some(RawAnnouncement {
                published_at: json_string(item, &["published_at", "notice_date", "date"]),
                title,
                article_code: json_string(item, &["article_code", "id", "code", "url"]),
                category: json_string(item, &["category"]),
            })
        })
        .collect::<Vec<_>>();
    if notices.is_empty() {
        return Err(DisclosureFetchError::Empty);
    }

    Ok(build_available_disclosure_context(
        root.get("source")
            .and_then(Value::as_str)
            .unwrap_or("official_announcements"),
        notices,
        disclosure_limit,
    ))
}

fn fetch_disclosure_from_sina(
    symbol: &str,
    disclosure_limit: usize,
) -> Result<DisclosureContext, DisclosureFetchError> {
    let url = build_sina_announcement_url(symbol);
    let body =
        http_get_text(&url, "sina_announcements").map_err(DisclosureFetchError::Transport)?;
    let regex = Regex::new(
        r#"(?is)(\d{4}-\d{2}-\d{2})\s*&nbsp;\s*<a[^>]*href=['"]([^'"]+)['"][^>]*>(.*?)</a>"#,
    )
    .map_err(|error| DisclosureFetchError::Parse(error.to_string()))?;

    let notices = regex
        .captures_iter(&body)
        .filter_map(|capture| {
            let title = strip_html_tags(capture.get(3)?.as_str());
            if title.trim().is_empty() {
                return None;
            }
            let href = capture.get(2)?.as_str().trim();
            Some(RawAnnouncement {
                published_at: Some(capture.get(1)?.as_str().to_string()),
                title,
                article_code: extract_query_param(href, "id")
                    .or_else(|| Some(to_absolute_sina_url(href))),
                category: Some("公司公告".to_string()),
            })
        })
        .collect::<Vec<_>>();
    if notices.is_empty() {
        return Err(DisclosureFetchError::Empty);
    }

    Ok(build_available_disclosure_context(
        "sina_announcements",
        notices,
        disclosure_limit,
    ))
}

fn build_available_fundamental_context(
    source: &str,
    latest_report_period: Option<String>,
    report_notice_date: Option<String>,
    metrics: FundamentalMetrics,
) -> FundamentalContext {
    let profit_signal = classify_fundamental_signal(&metrics);
    let (headline, narrative, risk_flags) = build_fundamental_narrative(&metrics, &profit_signal);

    FundamentalContext {
        status: "available".to_string(),
        source: source.to_string(),
        latest_report_period,
        report_notice_date,
        headline,
        profit_signal,
        report_metrics: metrics,
        narrative,
        risk_flags,
    }
}

fn build_available_disclosure_context(
    source: &str,
    notices: Vec<RawAnnouncement>,
    disclosure_limit: usize,
) -> DisclosureContext {
    let recent_announcements = notices
        .into_iter()
        .take(disclosure_limit)
        .map(|notice| DisclosureAnnouncement {
            published_at: notice
                .published_at
                .map(normalize_date_like)
                .unwrap_or_default(),
            title: notice.title,
            article_code: notice.article_code,
            category: notice.category,
        })
        .collect::<Vec<_>>();
    let keyword_summary = build_disclosure_keyword_summary(&recent_announcements);
    let risk_flags = build_disclosure_risk_flags(&recent_announcements);
    let headline = build_disclosure_headline(&recent_announcements, &risk_flags);

    DisclosureContext {
        status: "available".to_string(),
        source: source.to_string(),
        announcement_count: recent_announcements.len(),
        headline,
        keyword_summary,
        recent_announcements,
        risk_flags,
    }
}

fn build_industry_context(technical_context: &SecurityAnalysisContextualResult) -> IndustryContext {
    IndustryContext {
        sector_symbol: technical_context.sector_symbol.clone(),
        proxy_bias: technical_context
            .sector_analysis
            .consultation_conclusion
            .bias
            .clone(),
        headline: technical_context
            .sector_analysis
            .consultation_conclusion
            .headline
            .clone(),
        rationale: technical_context
            .sector_analysis
            .consultation_conclusion
            .rationale
            .clone(),
        risk_flags: technical_context
            .sector_analysis
            .consultation_conclusion
            .risk_flags
            .clone(),
    }
}

fn build_integrated_conclusion(
    symbol: &str,
    technical_context: &SecurityAnalysisContextualResult,
    fundamental_context: &FundamentalContext,
    disclosure_context: &DisclosureContext,
    etf_context: &EtfContext,
    cross_border_context: &CrossBorderEtfContext,
    industry_context: &IndustryContext,
) -> IntegratedConclusion {
    let technical_alignment = technical_context.contextual_conclusion.alignment.as_str();
    let is_etf = is_etf_symbol(symbol);
    let is_cross_border = cross_border_context.status != "not_applicable";
    let has_info_gap = if is_etf {
        etf_context.status != "available" || disclosure_context.status != "available"
    } else {
        fundamental_context.status != "available" || disclosure_context.status != "available"
    };
    let has_disclosure_risk = !disclosure_context.risk_flags.is_empty();
    let has_fundamental_risk = fundamental_context.profit_signal == "negative";
    let has_etf_structure_risk = !etf_context.structure_risk_flags.is_empty();

    let stance = if is_cross_border && cross_border_context.status == "incomplete" {
        "technical_only".to_string()
    } else if has_info_gap {
        "technical_only".to_string()
    } else if is_cross_border
        && cross_border_context.resonance_verdict == "underlying_first_confirmed"
        && !has_disclosure_risk
        && !has_etf_structure_risk
    {
        "constructive".to_string()
    } else if is_cross_border
        && cross_border_context.resonance_verdict == "buy_on_pullback_only"
        && !has_disclosure_risk
    {
        "watchful_positive".to_string()
    } else if is_cross_border
        && (cross_border_context.resonance_verdict == "watch_underlying_confirmation"
            || cross_border_context.resonance_verdict == "avoid_chasing_high_premium")
    {
        "cautious".to_string()
    } else if is_etf
        && technical_alignment == "tailwind"
        && !has_disclosure_risk
        && !has_etf_structure_risk
    {
        "constructive".to_string()
    } else if is_etf
        && technical_alignment == "mixed"
        && !has_disclosure_risk
        && !has_etf_structure_risk
    {
        "watchful_positive".to_string()
    } else if technical_alignment == "tailwind"
        && fundamental_context.profit_signal == "positive"
        && !has_disclosure_risk
    {
        "constructive".to_string()
    } else if technical_alignment == "mixed"
        && fundamental_context.profit_signal == "positive"
        && !has_disclosure_risk
    {
        "watchful_positive".to_string()
    } else if technical_alignment == "headwind"
        || has_fundamental_risk
        || has_disclosure_risk
        || has_etf_structure_risk
    {
        "cautious".to_string()
    } else {
        "mixed_watch".to_string()
    };

    let headline = match stance.as_str() {
        "constructive" => {
            "技术环境、财报快照和公告节奏形成了同向共振，当前更适合按偏积极的综合结论跟踪。"
                .to_string()
        }
        "watchful_positive" => {
            "财报快照偏正面，但技术环境仍在确认阶段，当前更适合作为边观察边等待确认的结论使用。"
                .to_string()
        }
        "technical_only" => {
            "信息面主链当前不可用，当前结论暂时只能以技术面和行业代理为主。".to_string()
        }
        "cautious" => {
            "技术面、财报面或公告面至少有一层没有形成正向共振，当前更适合保持谨慎。".to_string()
        }
        _ => "当前综合信息尚未形成单边优势，更适合作为观察性结论使用。".to_string(),
    };

    let mut rationale = vec![
        format!(
            "技术层当前为 {}，行业代理 {} 的 headline 为：{}",
            technical_context.contextual_conclusion.alignment,
            industry_context.sector_symbol,
            industry_context.headline
        ),
        format!(
            "财报层状态为 {}，利润信号为 {}。",
            fundamental_context.status, fundamental_context.profit_signal
        ),
        format!(
            "公告层状态为 {}，最近纳入 {} 条公告摘要。",
            disclosure_context.status, disclosure_context.announcement_count
        ),
    ];
    if fundamental_context.status == "available" {
        rationale.push(fundamental_context.headline.clone());
    }
    if etf_context.status == "available" {
        rationale.push(etf_context.headline.clone());
    }
    if is_cross_border {
        rationale.push(cross_border_context.headline.clone());
    }
    if disclosure_context.status == "available" {
        rationale.push(disclosure_context.headline.clone());
    }

    let mut risk_flags = Vec::new();
    if has_info_gap {
        risk_flags.push("财报面或公告面当前缺失，综合判断存在信息盲区".to_string());
    }
    risk_flags.extend(fundamental_context.risk_flags.clone());
    risk_flags.extend(disclosure_context.risk_flags.clone());
    risk_flags.extend(etf_context.structure_risk_flags.clone());
    risk_flags.extend(etf_context.research_gaps.clone());
    risk_flags.extend(cross_border_context.risk_flags.clone());
    if technical_alignment == "headwind" {
        risk_flags.push("技术环境仍处逆风，信息面正向也不能直接替代价格确认".to_string());
    }

    IntegratedConclusion {
        stance,
        headline,
        rationale,
        risk_flags,
    }
}

// 2026-04-20 CST: Added because ETF governed proxy history now needs one canonical
// fullstack loader instead of separate chair-only and snapshot-only interpretations.
// Reason: P10/P11 regressions showed latest/no-date and on-or-before replay were drifting.
// Purpose: resolve the effective governed ETF proxy snapshot without turning store errors
// into top-level fullstack failures for non-ETF or store-missing environments.
fn resolve_governed_etf_proxy_snapshot(
    request: &SecurityAnalysisFullstackRequest,
) -> Option<GovernedEtfProxySnapshot> {
    if !is_etf_symbol(&request.symbol) {
        return None;
    }
    let snapshot = if let Some(as_of_date) = request.as_of_date.as_deref() {
        load_historical_external_proxy_snapshot(&request.symbol, as_of_date).ok()?
    } else {
        load_latest_external_proxy_snapshot(&request.symbol).ok()?
    }?;
    Some(GovernedEtfProxySnapshot {
        as_of_date: snapshot.0,
        inputs: snapshot.1,
    })
}

// 2026-04-20 CST: Added because ETF proxy completeness needs one shared subscope
// vocabulary even when public ETF facts are unavailable.
// Reason: treasury/gold/equity tests route through sector profile aliases while the
// live ETF facts provider may be absent or incomplete.
// Purpose: derive the minimum ETF family needed to validate governed proxy completeness.
fn derive_etf_proxy_subscope(
    request: &SecurityAnalysisFullstackRequest,
    etf_context: &EtfContext,
) -> String {
    let sector_profile = request
        .sector_profile
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let asset_scope = etf_context
        .asset_scope
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if sector_profile.contains("gold")
        || asset_scope.contains("gold")
        || asset_scope.contains("commodity")
    {
        "commodity_etf".to_string()
    } else if sector_profile.contains("treasury")
        || sector_profile.contains("bond")
        || asset_scope.contains("treasury")
        || asset_scope.contains("bond")
    {
        "bond_etf".to_string()
    } else if sector_profile.contains("cross_border")
        || sector_profile.contains("qdii")
        || asset_scope.contains("cross_border")
        || asset_scope.contains("overseas")
    {
        "cross_border_etf".to_string()
    } else {
        "equity_etf".to_string()
    }
}

// 2026-04-20 CST: Added because ETF proxy substitution must only activate when the
// full governed feature family for that ETF pool is present.
// Reason: partial proxy payloads should still surface as degraded instead of silently
// claiming the ETF information contract is complete.
// Purpose: keep ETF governed substitution gated by the same family completeness idea
// that the scorecard runtime already enforces for bound ETF artifacts.
fn governed_etf_proxy_family_is_complete(
    inputs: &SecurityExternalProxyInputs,
    etf_proxy_subscope: &str,
) -> bool {
    match etf_proxy_subscope {
        "commodity_etf" => {
            inputs.gold_spot_proxy_status.is_some()
                && inputs.gold_spot_proxy_return_5d.is_some()
                && inputs.real_rate_proxy_status.is_some()
                && inputs.real_rate_proxy_delta_bp_5d.is_some()
        }
        "bond_etf" => {
            inputs.yield_curve_proxy_status.is_some()
                && inputs.yield_curve_slope_delta_bp_5d.is_some()
                && inputs.funding_liquidity_proxy_status.is_some()
                && inputs.funding_liquidity_spread_delta_bp_5d.is_some()
        }
        "cross_border_etf" => {
            inputs.fx_proxy_status.is_some()
                && inputs.fx_return_5d.is_some()
                && inputs.overseas_market_proxy_status.is_some()
                && inputs.overseas_market_return_5d.is_some()
                && inputs.market_session_gap_status.is_some()
                && inputs.market_session_gap_days.is_some()
        }
        _ => {
            inputs.etf_fund_flow_proxy_status.is_some()
                && inputs.etf_fund_flow_5d.is_some()
                && inputs.premium_discount_proxy_status.is_some()
                && inputs.premium_discount_pct.is_some()
                && inputs.benchmark_relative_strength_status.is_some()
                && inputs.benchmark_relative_return_5d.is_some()
        }
    }
}

// 2026-04-20 CST: Added because ETF proxy-complete runs still need a formal ETF context
// object even when public ETF facts are unavailable.
// Reason: data-gap collection and integrated conclusion logic both read etf_context.status.
// Purpose: synthesize the minimum ETF research context required to keep governed proxy
// evidence from being downgraded back to "ETF facts unavailable" semantics.
fn build_governed_etf_proxy_etf_context(symbol: &str, etf_proxy_subscope: &str) -> EtfContext {
    let asset_scope = match etf_proxy_subscope {
        "commodity_etf" => "commodity",
        "bond_etf" => "bond",
        "cross_border_etf" => "cross_border",
        _ => "equity",
    };
    EtfContext {
        status: "available".to_string(),
        source: "governed_etf_proxy_information".to_string(),
        fund_name: None,
        benchmark: None,
        asset_scope: Some(asset_scope.to_string()),
        latest_scale: None,
        latest_share: None,
        premium_discount_rate_pct: None,
        headline: format!(
            "{symbol} ETF context promoted from governed proxy history because the {etf_proxy_subscope} proxy family is complete."
        ),
        structure_risk_flags: vec![],
        research_gaps: vec![],
    }
}

// 2026-04-20 CST: Added because ETF proxy-complete governance now allows the fullstack
// chain to stop demanding stock-only financial statements for ETF evidence completeness.
// Reason: committee/chair evidence quality only needs one governed available-vs-unavailable
// contract here, not an invented pseudo-financial signal.
// Purpose: synthesize a neutral ETF information layer that keeps evidence complete
// while preserving the fact that no stock-style report metrics were claimed.
fn build_governed_etf_proxy_fundamental_context(
    symbol: &str,
    proxy_as_of_date: &str,
    etf_proxy_subscope: &str,
) -> FundamentalContext {
    FundamentalContext {
        status: "available".to_string(),
        source: "governed_etf_proxy_information".to_string(),
        latest_report_period: Some(proxy_as_of_date.to_string()),
        report_notice_date: Some(proxy_as_of_date.to_string()),
        headline: format!(
            "{symbol} uses governed ETF proxy information dated {proxy_as_of_date} as the formal {etf_proxy_subscope} information layer."
        ),
        profit_signal: "unknown".to_string(),
        report_metrics: empty_fundamental_metrics(),
        narrative: vec![format!(
            "Governed ETF proxy family for {etf_proxy_subscope} is complete on {proxy_as_of_date}, so stock-only financial availability is no longer required for ETF evidence completeness."
        )],
        risk_flags: vec![],
    }
}

// 2026-04-20 CST: Added because ETF proxy-complete governance now allows disclosure
// completeness to be satisfied by the governed ETF proxy layer when stock notices are absent.
// Reason: the previous unavailable disclosure fallback kept chair evidence degraded even
// though the ETF proxy history had already been formally bound for the same symbol/date.
// Purpose: expose a stable governed disclosure placeholder with explicit provenance instead
// of silently inheriting the stock announcement fallback source.
fn build_governed_etf_proxy_disclosure_context(
    symbol: &str,
    proxy_as_of_date: &str,
    etf_proxy_subscope: &str,
) -> DisclosureContext {
    DisclosureContext {
        status: "available".to_string(),
        source: "governed_etf_proxy_information".to_string(),
        announcement_count: 0,
        headline: format!(
            "{symbol} uses governed ETF proxy information dated {proxy_as_of_date} as the formal {etf_proxy_subscope} event layer."
        ),
        keyword_summary: vec!["governed_etf_proxy_information".to_string()],
        recent_announcements: vec![],
        risk_flags: vec![],
    }
}

fn build_unavailable_fundamental_context(message: String) -> FundamentalContext {
    FundamentalContext {
        status: "unavailable".to_string(),
        source: "multi_source_fallback".to_string(),
        latest_report_period: None,
        report_notice_date: None,
        headline: "财报快照当前不可用，综合结论已退化为技术优先。".to_string(),
        profit_signal: "unknown".to_string(),
        report_metrics: empty_fundamental_metrics(),
        narrative: vec!["多源财报抓取均未返回可消费数据，当前已跳过财报层聚合。".to_string()],
        risk_flags: vec![message],
    }
}

fn build_unavailable_disclosure_context(message: String) -> DisclosureContext {
    DisclosureContext {
        status: "unavailable".to_string(),
        source: "multi_source_fallback".to_string(),
        announcement_count: 0,
        headline: "公告摘要当前不可用，综合结论未纳入事件驱动层信息。".to_string(),
        keyword_summary: vec![],
        recent_announcements: vec![],
        risk_flags: vec![message],
    }
}

fn build_not_applicable_etf_context() -> EtfContext {
    EtfContext {
        status: "not_applicable".to_string(),
        source: "not_applicable".to_string(),
        fund_name: None,
        benchmark: None,
        asset_scope: None,
        latest_scale: None,
        latest_share: None,
        premium_discount_rate_pct: None,
        headline: "当前标的不是 ETF，不生成 ETF 专项事实层。".to_string(),
        structure_risk_flags: vec![],
        research_gaps: vec![],
    }
}

fn build_unavailable_etf_context(message: String) -> EtfContext {
    EtfContext {
        status: "unavailable".to_string(),
        source: "public_etf_fallback".to_string(),
        fund_name: None,
        benchmark: None,
        asset_scope: None,
        latest_scale: None,
        latest_share: None,
        premium_discount_rate_pct: None,
        headline: "ETF 专项公开事实层当前不可用，主链暂不输出基金结构与折溢价结论。".to_string(),
        structure_risk_flags: vec![message],
        research_gaps: vec![
            "ETF 专项公开事实源不可用。".to_string(),
            "当前缺少跟踪基准、规模/份额与折溢价等正式输入。".to_string(),
        ],
    }
}

fn build_etf_headline(
    fund_name: Option<&str>,
    benchmark: Option<&str>,
    asset_scope: Option<&str>,
    latest_scale: Option<f64>,
    premium_discount_rate_pct: Option<f64>,
) -> String {
    let name = fund_name.unwrap_or("该 ETF");
    let benchmark_part = benchmark.unwrap_or("跟踪基准待补充");
    let scope_part = asset_scope.unwrap_or("资产范围待补充");
    let scale_part = latest_scale
        .map(|value| format!("最新规模约 {:.2}", value))
        .unwrap_or_else(|| "最新规模待补充".to_string());
    let premium_part = premium_discount_rate_pct
        .map(|value| format!("折溢价 {:.2}%", value))
        .unwrap_or_else(|| "折溢价待补充".to_string());
    format!(
        "{name} 当前按 {benchmark_part} 进行观察，资产范围为 {scope_part}，{scale_part}，{premium_part}。"
    )
}

fn build_eastmoney_financial_url(symbol: &str) -> String {
    let base = std::env::var("EXCEL_SKILL_EASTMONEY_FINANCIAL_URL_BASE").unwrap_or_else(|_| {
        "https://emweb.securities.eastmoney.com/PC_HSF10/NewFinanceAnalysis/MainTargetAjax"
            .to_string()
    });
    append_query_params(
        &base,
        &[
            ("type", "1".to_string()),
            ("code", normalize_eastmoney_code(symbol)),
        ],
    )
}

fn build_eastmoney_announcement_url(symbol: &str, disclosure_limit: usize) -> String {
    let base = std::env::var("EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE")
        .unwrap_or_else(|_| "https://np-anotice-stock.eastmoney.com/api/security/ann".to_string());
    append_query_params(
        &base,
        &[
            ("sr", "-1".to_string()),
            ("page_size", disclosure_limit.min(20).to_string()),
            ("page_index", "1".to_string()),
            ("ann_type", "A".to_string()),
            ("stock_list", normalize_plain_stock_code(symbol)),
        ],
    )
}

// 2026-04-14 CST: Added because governed disclosure backfill must request multiple pages, while
// the interactive fullstack path only builds page 1 from a top-N disclosure limit.
// Purpose: keep paged announcement URL generation colocated with the existing Eastmoney helpers.
fn build_eastmoney_announcement_page_url(
    symbol: &str,
    page_size: usize,
    page_index: usize,
) -> String {
    let base = std::env::var("EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE")
        .unwrap_or_else(|_| "https://np-anotice-stock.eastmoney.com/api/security/ann".to_string());
    append_query_params(
        &base,
        &[
            ("sr", "-1".to_string()),
            ("page_size", page_size.max(1).min(200).to_string()),
            ("page_index", page_index.max(1).to_string()),
            ("ann_type", "A".to_string()),
            ("stock_list", normalize_plain_stock_code(symbol)),
        ],
    )
}

fn build_optional_official_financial_url(symbol: &str) -> Option<String> {
    std::env::var("EXCEL_SKILL_OFFICIAL_FINANCIAL_URL_BASE")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|base| append_query_params(&base, &[("symbol", symbol.to_string())]))
}

fn build_optional_official_announcement_url(
    symbol: &str,
    disclosure_limit: usize,
) -> Option<String> {
    std::env::var("EXCEL_SKILL_OFFICIAL_ANNOUNCEMENT_URL_BASE")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|base| {
            append_query_params(
                &base,
                &[
                    ("symbol", symbol.to_string()),
                    ("limit", disclosure_limit.to_string()),
                ],
            )
        })
}

fn build_optional_etf_facts_url(symbol: &str) -> Option<String> {
    std::env::var("EXCEL_SKILL_ETF_FACTS_URL_BASE")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|base| append_query_params(&base, &[("symbol", symbol.to_string())]))
}

fn build_sina_financial_url(symbol: &str) -> String {
    let plain = normalize_plain_stock_code(symbol);
    match std::env::var("EXCEL_SKILL_SINA_FINANCIAL_URL_BASE") {
        Ok(base) if !base.trim().is_empty() => {
            append_query_params(&base, &[("symbol", symbol.to_string()), ("stockid", plain)])
        }
        _ => format!("{DEFAULT_SINA_FINANCIAL_URL_BASE}/stockid/{plain}/displaytype/4.phtml"),
    }
}

fn build_sina_announcement_url(symbol: &str) -> String {
    let plain = normalize_plain_stock_code(symbol);
    match std::env::var("EXCEL_SKILL_SINA_ANNOUNCEMENT_URL_BASE") {
        Ok(base) if !base.trim().is_empty() => {
            append_query_params(&base, &[("symbol", symbol.to_string()), ("stockid", plain)])
        }
        _ => format!("{DEFAULT_SINA_ANNOUNCEMENT_URL_BASE}/stockid/{plain}.phtml"),
    }
}

fn append_query_params(base: &str, params: &[(&str, String)]) -> String {
    let separator = if base.contains('?') { '&' } else { '?' };
    let query = params
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");
    format!("{base}{separator}{query}")
}

// 2026-04-17 CST: Added because live upstream sockets can stall long enough to
// make full-suite verification look frozen.
// Reason: the previous helper had no explicit timeout bound.
// Purpose: keep analysis/backfill fetch failures short and deterministic.
fn resolve_http_timeout() -> Duration {
    const DEFAULT_HTTP_TIMEOUT_SECS: u64 = 8;
    std::env::var("EXCEL_SKILL_HTTP_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(DEFAULT_HTTP_TIMEOUT_SECS))
}

fn http_get_text(url: &str, source_label: &str) -> Result<String, String> {
    match ureq::get(url)
        .set("Accept", "text/html,application/json;q=0.9,*/*;q=0.8")
        .timeout(resolve_http_timeout())
        .call()
    {
        Ok(response) => response.into_string().map_err(|error| error.to_string()),
        Err(ureq::Error::Status(status, response)) => {
            let body = response.into_string().unwrap_or_default();
            Err(if body.is_empty() {
                format!("{source_label} HTTP {status}")
            } else {
                format!("{source_label} HTTP {status}: {body}")
            })
        }
        Err(ureq::Error::Transport(error)) => Err(error.to_string()),
    }
}

fn extract_latest_financial_row<'a>(
    payload: &'a Value,
) -> Result<&'a Value, FundamentalFetchError> {
    if let Some(rows) = payload.as_array() {
        return rows.first().ok_or(FundamentalFetchError::Empty);
    }
    if let Some(rows) = payload.get("data").and_then(|value| value.as_array()) {
        return rows.first().ok_or(FundamentalFetchError::Empty);
    }
    Err(FundamentalFetchError::Parse(
        "财报源返回结构不符合预期".to_string(),
    ))
}

fn extract_announcement_list(
    payload: &Value,
) -> Result<Vec<RawAnnouncement>, DisclosureFetchError> {
    let list = payload
        .get("data")
        .and_then(|value| value.get("list"))
        .and_then(Value::as_array)
        .ok_or_else(|| DisclosureFetchError::Parse("公告源返回结构不符合预期".to_string()))?;

    Ok(list
        .iter()
        .filter_map(|item| {
            let title = item
                .get("title")
                .and_then(Value::as_str)?
                .trim()
                .to_string();
            if title.is_empty() {
                return None;
            }
            Some(RawAnnouncement {
                published_at: item
                    .get("notice_date")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                title,
                article_code: item
                    .get("art_code")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                category: item
                    .get("columns")
                    .and_then(Value::as_array)
                    .and_then(|columns| columns.first())
                    .and_then(|value| value.get("column_name"))
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
            })
        })
        .collect())
}

fn extract_first_object(payload: &Value) -> Option<&Value> {
    if payload.is_object() {
        return Some(payload);
    }
    payload.as_array().and_then(|rows| rows.first())
}

fn financial_string(row: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        row.get(*key)
            .and_then(Value::as_str)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn financial_number(row: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter()
        .find_map(|key| row.get(*key))
        .and_then(value_as_f64)
}

fn json_string(row: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        row.get(*key)
            .and_then(Value::as_str)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn json_number(row: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter()
        .find_map(|key| row.get(*key))
        .and_then(value_as_f64)
}

fn json_string_array(row: &Value, keys: &[&str]) -> Vec<String> {
    keys.iter()
        .find_map(|key| row.get(*key))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn is_etf_symbol(symbol: &str) -> bool {
    let normalized_symbol = symbol.trim().to_uppercase();
    normalized_symbol
        .strip_suffix(".SZ")
        .map(|code| code.starts_with("15") || code.starts_with("16"))
        .unwrap_or(false)
        || normalized_symbol
            .strip_suffix(".SH")
            .map(|code| code.starts_with("51") || code.starts_with("56") || code.starts_with("58"))
            .unwrap_or(false)
}

fn dedupe_strings(values: &mut Vec<String>) {
    let mut deduped = Vec::new();
    for value in values.drain(..) {
        if !deduped.contains(&value) {
            deduped.push(value);
        }
    }
    *values = deduped;
}

fn value_as_f64(value: &Value) -> Option<f64> {
    if let Some(number) = value.as_f64() {
        return Some(number);
    }
    value.as_str().and_then(parse_number_text)
}

fn parse_number_text(text: &str) -> Option<f64> {
    text.replace(',', "").trim().parse::<f64>().ok()
}

#[allow(dead_code)]
fn parse_html_table_rows(html: &str) -> Vec<Vec<String>> {
    let row_regex = Regex::new(r"(?is)<tr[^>]*>(.*?)</tr>").expect("row regex should compile");
    let cell_regex =
        Regex::new(r"(?is)<t[dh][^>]*>(.*?)</t[dh]>").expect("cell regex should compile");

    row_regex
        .captures_iter(html)
        .filter_map(|row_capture| {
            let row_html = row_capture.get(1)?.as_str();
            let cells = cell_regex
                .captures_iter(row_html)
                .filter_map(|cell_capture| cell_capture.get(1))
                .map(|cell| strip_html_tags(cell.as_str()))
                .collect::<Vec<_>>();
            if cells.is_empty() { None } else { Some(cells) }
        })
        .collect()
}

fn parse_html_rows_with_raw(html: &str) -> Vec<ParsedHtmlRow> {
    let row_regex = Regex::new(r"(?is)<tr[^>]*>(.*?)</tr>").expect("row regex should compile");
    let cell_regex =
        Regex::new(r"(?is)<t[dh][^>]*>(.*?)</t[dh]>").expect("cell regex should compile");

    row_regex
        .captures_iter(html)
        .filter_map(|row_capture| {
            let row_html = row_capture.get(1)?.as_str();
            let cells = cell_regex
                .captures_iter(row_html)
                .filter_map(|cell_capture| cell_capture.get(1))
                .map(|cell| strip_html_tags(cell.as_str()))
                .collect::<Vec<_>>();
            if cells.is_empty() {
                None
            } else {
                Some(ParsedHtmlRow {
                    html: row_html.to_string(),
                    cells,
                })
            }
        })
        .collect()
}

fn collect_sina_numeric_columns(cells: &[String]) -> Vec<Option<f64>> {
    cells
        .iter()
        .skip(1)
        .map(|value| parse_number_text(value.trim()))
        .collect::<Vec<_>>()
}

fn normalize_sina_financial_label(label: &str) -> String {
    label
        .replace("&nbsp;", "")
        .replace(" ", "")
        .replace('\u{a0}', "")
        .replace('\t', "")
        .replace('\r', "")
        .replace('\n', "")
        .replace('（', "(")
        .replace('）', ")")
        .replace('_', "")
        .replace(':', "")
        .replace('：', "")
}

fn is_sina_report_period_row(normalized_label: &str, cells: &[String]) -> bool {
    if normalized_label.contains("报告日期") {
        return true;
    }

    let date_like_count = cells
        .iter()
        .skip(1)
        .take(4)
        .filter(|value| looks_like_report_date(value))
        .count();

    date_like_count >= 2
}

fn looks_like_report_date(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.len() == 10
        && trimmed.chars().nth(4) == Some('-')
        && trimmed.chars().nth(7) == Some('-')
        && trimmed
            .chars()
            .enumerate()
            .all(|(idx, ch)| matches!(idx, 4 | 7) || ch.is_ascii_digit())
}

fn is_sina_financial_metric_row(
    normalized_label: &str,
    row_html: &str,
    typecodes: &[&str],
    label_keywords: &[&str],
) -> bool {
    typecodes.iter().any(|code| row_html.contains(code))
        || label_keywords
            .iter()
            .any(|keyword| normalized_label.contains(keyword))
}

fn strip_html_tags(html: &str) -> String {
    let tag_regex = Regex::new(r"(?is)<[^>]+>").expect("tag regex should compile");
    tag_regex
        .replace_all(html, "")
        .replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .trim()
        .to_string()
}

fn extract_query_param(url: &str, key: &str) -> Option<String> {
    let query = url.split('?').nth(1)?;
    query.split('&').find_map(|segment| {
        let (segment_key, value) = segment.split_once('=')?;
        if segment_key == key {
            Some(value.to_string())
        } else {
            None
        }
    })
}

fn to_absolute_sina_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://vip.stock.finance.sina.com.cn{url}")
    }
}

fn classify_fundamental_signal(metrics: &FundamentalMetrics) -> String {
    let positives = [metrics.revenue_yoy_pct, metrics.net_profit_yoy_pct]
        .into_iter()
        .flatten()
        .filter(|value| *value >= 0.0)
        .count();
    let negatives = [metrics.revenue_yoy_pct, metrics.net_profit_yoy_pct]
        .into_iter()
        .flatten()
        .filter(|value| *value < 0.0)
        .count();

    match (positives, negatives) {
        (0, 0) => "unknown".to_string(),
        (0, _) => "negative".to_string(),
        (_, 0) => "positive".to_string(),
        _ => "mixed".to_string(),
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
        "positive" => "最新财报快照显示核心盈利指标仍保持正向。".to_string(),
        "negative" => "最新财报快照显示核心盈利指标正在承压。".to_string(),
        "mixed" => "最新财报快照的收入与利润表现分化，需避免单一指标误导。".to_string(),
        _ => "最新财报快照只返回了部分指标，当前更适合作为辅助观察。".to_string(),
    };

    let narrative = vec![
        headline.clone(),
        format!("{revenue_text}，{profit_text}。"),
        format!("盈利质量可继续结合 {roe_text} 与后续现金流披露确认。"),
    ];

    let mut risk_flags = Vec::new();
    if metrics.net_profit_yoy_pct.is_some_and(|value| value < 0.0) {
        risk_flags.push("归母净利润同比为负，后续估值修复弹性可能受限".to_string());
    }
    if metrics.revenue_yoy_pct.is_some_and(|value| value < 0.0) {
        risk_flags.push("营收同比为负，需警惕需求或价格压力继续传导".to_string());
    }
    if metrics.roe_pct.is_some_and(|value| value < 8.0) {
        risk_flags.push("ROE 偏低，盈利效率仍需后续报告进一步验证".to_string());
    }
    if metrics.revenue_yoy_pct.is_none() || metrics.net_profit_yoy_pct.is_none() {
        risk_flags.push("财报关键同比指标不完整，当前解读存在缺口".to_string());
    }

    (headline, narrative, risk_flags)
}

fn build_disclosure_keyword_summary(notices: &[DisclosureAnnouncement]) -> Vec<String> {
    let mut summary = Vec::new();
    if disclosure_has_annual_report_notice(notices) {
        summary.push("最近公告包含年度报告或定期报告".to_string());
    }
    if disclosure_has_dividend_notice(notices) {
        summary.push("最近公告包含利润分配或分红信息".to_string());
    }
    if disclosure_has_buyback_or_increase_notice(notices) {
        summary.push("最近公告包含回购或增持类事项".to_string());
    }
    if summary.is_empty() {
        summary.push("最近公告暂未识别出高频正向事件关键词".to_string());
    }
    summary
}

fn build_disclosure_risk_flags(notices: &[DisclosureAnnouncement]) -> Vec<String> {
    let risk_keywords = [
        ("减持", "最近公告含减持事项，需留意筹码压力"),
        ("定增", "最近公告含再融资事项，需留意融资摊薄与预期重定价"),
        (
            "向特定对象发行",
            "最近公告含再融资事项，需留意融资摊薄与预期重定价",
        ),
        (
            "非公开发行",
            "最近公告含再融资事项，需留意融资摊薄与预期重定价",
        ),
        ("配股", "最近公告含再融资事项，需留意融资摊薄与预期重定价"),
        (
            "募集配套资金",
            "最近公告含再融资事项，需留意融资摊薄与预期重定价",
        ),
        ("问询", "最近公告含问询事项，需留意监管关注点"),
        ("诉讼", "最近公告含诉讼事项，需留意经营不确定性"),
        ("终止", "最近公告含终止事项，需留意原有催化是否失效"),
        (
            "异常波动",
            "最近公告含异常波动事项，需警惕短期情绪与监管扰动",
        ),
        (
            "风险提示",
            "最近公告含风险提示，需关注公司主动披露的不确定性",
        ),
        ("预亏", "最近公告含预亏信息，需重新评估盈利预期"),
        ("亏损", "最近公告含亏损相关信息，需警惕业绩压力"),
        (
            "资金占用",
            "最近公告含资金占用事项，需重点复核治理与财务风险",
        ),
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
        return "最近公告中已经出现需要重点复核的风险关键词，信息面不宜按纯正向理解。".to_string();
    }
    if disclosure_has_annual_report_notice(notices) {
        return "最近公告以定期披露为主，信息面暂未看到明显负向事件。".to_string();
    }
    if disclosure_has_buyback_or_increase_notice(notices) {
        return "最近公告含回购或增持类事项，事件层对情绪存在一定支撑。".to_string();
    }
    "最近公告以常规定期披露和公司事项为主，暂未识别到强风险事件。".to_string()
}

// 2026-04-10 CST: 这里把公告关键词识别下沉成可复用 helper，原因是 fullstack 摘要、证据包特征种子和后续评分卡训练都要共用同一套消息面口径；
// 目的：避免“摘要里识别了、特征里没识别”或不同上层对象各自维护关键词表，导致结构化因子与文案结论不一致。
pub(crate) fn disclosure_positive_keyword_count(notices: &[DisclosureAnnouncement]) -> usize {
    [
        disclosure_has_annual_report_notice(notices),
        disclosure_has_dividend_notice(notices),
        disclosure_has_buyback_or_increase_notice(notices),
    ]
    .into_iter()
    .filter(|flag| *flag)
    .count()
}

// 2026-04-10 CST: 这里补风险关键词计数 helper，原因是第一阶段统一评分版需要把消息面从“有无公告”下沉到“风险密度”；
// 目的：给 snapshot / training / scorecard 提供稳定的离散风险强度输入，而不是只在 headline 里体现。
pub(crate) fn disclosure_risk_keyword_count(notices: &[DisclosureAnnouncement]) -> usize {
    // 2026-04-17 CST: Added because the thicker training/snapshot migration now needs the
    // coarse risk-count feature to include refinancing, abnormal-volatility, and fund-occupation
    // event families instead of undercounting the governed disclosure surface.
    // Purpose: keep legacy risk density and newer weighted event scoring aligned on the same set
    // of negative disclosure families.
    [
        disclosure_has_reduction_notice(notices),
        disclosure_has_refinancing_notice(notices),
        disclosure_has_inquiry_notice(notices),
        disclosure_has_litigation_notice(notices),
        disclosure_has_termination_notice(notices),
        disclosure_has_abnormal_volatility_notice(notices),
        disclosure_has_risk_warning_notice(notices),
        disclosure_has_preloss_or_loss_notice(notices),
        disclosure_has_fund_occupation_notice(notices),
    ]
    .into_iter()
    .filter(|flag| *flag)
    .count()
}

pub(crate) fn disclosure_has_annual_report_notice(notices: &[DisclosureAnnouncement]) -> bool {
    disclosure_notice_exists(notices, &["年度报告", "年报", "定期报告"])
}

pub(crate) fn disclosure_has_dividend_notice(notices: &[DisclosureAnnouncement]) -> bool {
    disclosure_notice_exists(notices, &["利润分配", "分红"])
}

pub(crate) fn disclosure_has_buyback_or_increase_notice(
    notices: &[DisclosureAnnouncement],
) -> bool {
    disclosure_notice_exists(notices, &["回购", "增持"])
}

pub(crate) fn disclosure_has_reduction_notice(notices: &[DisclosureAnnouncement]) -> bool {
    disclosure_notice_exists(notices, &["减持"])
}

// 2026-04-17 CST: Added because refinancing headlines should become a first-class negative
// event family instead of staying invisible to the governed disclosure features.
// Purpose: give securities analysis one reusable refinancing detector for snapshot and training.
pub(crate) fn disclosure_has_refinancing_notice(notices: &[DisclosureAnnouncement]) -> bool {
    disclosure_notice_exists(
        notices,
        &[
            "定增",
            "非公开发行",
            "向特定对象发行",
            "配股",
            "募集配套资金",
        ],
    )
}

pub(crate) fn disclosure_has_inquiry_notice(notices: &[DisclosureAnnouncement]) -> bool {
    disclosure_notice_exists(notices, &["问询"])
}

pub(crate) fn disclosure_has_litigation_notice(notices: &[DisclosureAnnouncement]) -> bool {
    disclosure_notice_exists(notices, &["诉讼"])
}

pub(crate) fn disclosure_has_termination_notice(notices: &[DisclosureAnnouncement]) -> bool {
    disclosure_notice_exists(notices, &["终止"])
}

// 2026-04-17 CST: Added because unusual-volatility notices often matter as short-term negative
// attention even when they do not amount to a hard fundamental risk.
// Purpose: separate abnormal trading heat from the older generic risk-warning bucket.
pub(crate) fn disclosure_has_abnormal_volatility_notice(
    notices: &[DisclosureAnnouncement],
) -> bool {
    disclosure_notice_exists(notices, &["异常波动", "交易异常波动"])
}

pub(crate) fn disclosure_has_risk_warning_notice(notices: &[DisclosureAnnouncement]) -> bool {
    disclosure_notice_exists(notices, &["风险提示"])
}

pub(crate) fn disclosure_has_preloss_or_loss_notice(notices: &[DisclosureAnnouncement]) -> bool {
    disclosure_notice_exists(notices, &["预亏", "亏损"])
}

// 2026-04-17 CST: Added because fund occupation is materially more severe than generic negative
// attention and should be promoted into the governed hard-risk surface.
// Purpose: expose one reusable detector for capital-occupation governance events.
pub(crate) fn disclosure_has_fund_occupation_notice(notices: &[DisclosureAnnouncement]) -> bool {
    disclosure_notice_exists(notices, &["资金占用", "非经营性资金占用"])
}

// 2026-04-10 CST: 这里集中封装标题关键词命中，原因是上面多组 helper 都只是“同一批公告标题是否命中不同规则”的变体；
// 目的：保持关键词口径单点维护，减少后续扩展消息面因子时的复制代码。
fn disclosure_notice_exists(notices: &[DisclosureAnnouncement], keywords: &[&str]) -> bool {
    notices
        .iter()
        .any(|notice| contains_any(&notice.title, keywords))
}

fn contains_any(title: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| title.contains(keyword))
}

fn empty_fundamental_metrics() -> FundamentalMetrics {
    FundamentalMetrics {
        revenue: None,
        revenue_yoy_pct: None,
        net_profit: None,
        net_profit_yoy_pct: None,
        roe_pct: None,
    }
}

fn normalize_eastmoney_code(symbol: &str) -> String {
    let normalized = symbol.trim().to_uppercase();
    if let Some((code, exchange)) = normalized.split_once('.') {
        return format!("{exchange}{code}");
    }
    if normalized.len() == 6 {
        let exchange = if normalized.starts_with(['6', '9']) {
            "SH"
        } else {
            "SZ"
        };
        return format!("{exchange}{normalized}");
    }
    normalized
}

fn normalize_plain_stock_code(symbol: &str) -> String {
    symbol
        .trim()
        .split('.')
        .next()
        .unwrap_or(symbol)
        .to_string()
}

fn normalize_date_like(value: String) -> String {
    value.chars().take(10).collect()
}

fn default_lookback_days() -> usize {
    260
}

fn default_disclosure_limit() -> usize {
    DEFAULT_DISCLOSURE_LIMIT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::lock_test_env;
    use std::io::Read;
    use std::net::TcpListener;
    use std::thread;
    use std::time::{Duration, Instant};

    fn spawn_nonresponsive_http_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
        let address = listener
            .local_addr()
            .expect("test server should expose local addr");
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0_u8; 1024];
                let _ = stream.read(&mut buffer);
                thread::sleep(Duration::from_secs(3));
            }
        });
        format!("http://{address}")
    }

    #[test]
    fn http_get_text_times_out_when_server_never_finishes_response() {
        let _env_guard = lock_test_env();
        let original_timeout = std::env::var("EXCEL_SKILL_HTTP_TIMEOUT_SECS").ok();
        unsafe {
            std::env::set_var("EXCEL_SKILL_HTTP_TIMEOUT_SECS", "1");
        }

        let started_at = Instant::now();
        let result = http_get_text(&spawn_nonresponsive_http_server(), "financials");

        match original_timeout {
            Some(value) => unsafe {
                std::env::set_var("EXCEL_SKILL_HTTP_TIMEOUT_SECS", value);
            },
            None => unsafe {
                std::env::remove_var("EXCEL_SKILL_HTTP_TIMEOUT_SECS");
            },
        }

        assert!(
            result.is_err(),
            "nonresponsive upstream should fail instead of hanging"
        );
        assert!(
            started_at.elapsed() < Duration::from_millis(2500),
            "request should time out quickly, got {:?}",
            started_at.elapsed()
        );
    }
}
