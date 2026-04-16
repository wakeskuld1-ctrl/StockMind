use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::stock_analysis_data_guard::StockAnalysisDateGuard;
use crate::ops::stock::technical_consultation_basic::{
    TechnicalConsultationBasicError, TechnicalConsultationBasicRequest,
    TechnicalConsultationBasicResult, technical_consultation_basic,
};

const DEFAULT_LOOKBACK_DAYS: usize = 260;

// 2026-04-01 CST: 这里定义上层综合证券分析请求，原因是方案 B 要把综合 Tool 推进到“显式 symbol 或 profile 二选一”的可交付 V1。
// 目的：既保留高级用户手填代理 symbol 的灵活性，也给真实调用提供更低门槛的 profile 入口。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityAnalysisContextualRequest {
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
    #[serde(default = "default_lookback_days")]
    pub lookback_days: usize,
}

// 2026-04-01 CST: 这里定义综合证券分析结果，原因是上层 Tool 不能只回三个子结果平铺，还要回传已收口的环境结论。
// 目的：让 CLI / Skill / 后续 GUI 直接消费统一的综合证券分析合同。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityAnalysisContextualResult {
    pub symbol: String,
    pub market_symbol: String,
    pub sector_symbol: String,
    // 2026-04-08 CST: 这里新增统一分析日期字段，原因是方案 C 要把 briefing 已有的公共合同下沉到 contextual 层；
    // 目的：让调用方读取环境分析结果时，直接获得统一的 `analysis_date`，不必再从嵌套技术结果中反推日期。
    pub analysis_date: String,
    // 2026-04-08 CST: 这里新增证据版本字段，原因是 contextual 层也要向上游暴露稳定、可追踪的事实版本；
    // 目的：让 fullstack/briefing/skill 能引用当前环境分析快照，而不是依赖隐式嵌套结构。
    pub evidence_version: String,
    pub analysis_date_guard: StockAnalysisDateGuard,
    pub stock_analysis: TechnicalConsultationBasicResult,
    pub market_analysis: TechnicalConsultationBasicResult,
    pub sector_analysis: TechnicalConsultationBasicResult,
    pub contextual_conclusion: SecurityContextualConclusion,
}

// 2026-04-01 CST: 这里定义综合结论对象，原因是方案 B 需要把顺风、逆风、混合环境稳定沉淀成正式合同。
// 目的：统一 headline、rationale、risk_flags 的返回结构，降低上层重复解释成本。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityContextualConclusion {
    pub alignment: String,
    pub headline: String,
    pub rationale: Vec<String>,
    pub risk_flags: Vec<String>,
}

// 2026-04-01 CST: 这里定义综合 Tool 错误，原因是 V1 阶段需要把“缺代理配置”“profile 错误”“子链路失败”分开表达。
// 目的：让真实调用时的失败原因足够明确，减少排查成本。
#[derive(Debug, Error)]
pub enum SecurityAnalysisContextualError {
    #[error("缺少大盘代理配置：请传入 market_symbol 或 market_profile")]
    MissingMarketProxy,
    #[error("缺少板块代理配置：请传入 sector_symbol 或 sector_profile")]
    MissingSectorProxy,
    #[error("不支持的大盘代理 profile: {profile}")]
    UnsupportedMarketProfile { profile: String },
    #[error("不支持的板块代理 profile: {profile}")]
    UnsupportedSectorProfile { profile: String },
    #[error("个股技术面分析失败: {0}")]
    Stock(#[source] TechnicalConsultationBasicError),
    #[error("大盘环境分析失败: {0}")]
    Market(#[source] TechnicalConsultationBasicError),
    #[error("板块环境分析失败: {0}")]
    Sector(#[source] TechnicalConsultationBasicError),
}

// 2026-04-01 CST: 这里实现综合证券分析主入口，原因是用户已确认在 `technical_consultation_basic` 上层继续推进，而不是回塞到底层。
// 目的：复用 3 次既有技术面分析，先交付一个可调用、可测试、可扩展的综合证券分析 V1。
pub fn security_analysis_contextual(
    request: &SecurityAnalysisContextualRequest,
) -> Result<SecurityAnalysisContextualResult, SecurityAnalysisContextualError> {
    let market_symbol = resolve_market_symbol(request)?;
    let sector_symbol = resolve_sector_symbol(request)?;

    let stock_request = build_basic_request(
        &request.symbol,
        request.as_of_date.clone(),
        request.lookback_days,
    );
    let market_request = build_basic_request(
        &market_symbol,
        request.as_of_date.clone(),
        request.lookback_days,
    );
    let sector_request = build_basic_request(
        &sector_symbol,
        request.as_of_date.clone(),
        request.lookback_days,
    );

    let stock_analysis = technical_consultation_basic(&stock_request)
        .map_err(SecurityAnalysisContextualError::Stock)?;
    let market_analysis = technical_consultation_basic(&market_request)
        .map_err(SecurityAnalysisContextualError::Market)?;
    let sector_analysis = technical_consultation_basic(&sector_request)
        .map_err(SecurityAnalysisContextualError::Sector)?;

    let contextual_conclusion =
        build_contextual_conclusion(&stock_analysis, &market_analysis, &sector_analysis);
    // 2026-04-08 CST: 这里沿用个股技术层的统一日期生成 contextual 顶层合同字段，原因是环境层建立在同一批个股快照之上；
    // 目的：保证 contextual 顶层 `analysis_date / evidence_version` 与底层技术分析保持同日、同版本语义。
    let analysis_date = stock_analysis.analysis_date.clone();
    let evidence_version = format!(
        "security-analysis-contextual:{}:{}:v1",
        request.symbol, analysis_date
    );

    Ok(SecurityAnalysisContextualResult {
        symbol: request.symbol.clone(),
        market_symbol,
        sector_symbol,
        analysis_date,
        evidence_version,
        analysis_date_guard: stock_analysis_date_guard(&stock_analysis),
        stock_analysis,
        market_analysis,
        sector_analysis,
        contextual_conclusion,
    })
}

fn stock_analysis_date_guard(
    stock_analysis: &TechnicalConsultationBasicResult,
) -> StockAnalysisDateGuard {
    StockAnalysisDateGuard {
        requested_as_of_date: stock_analysis.requested_as_of_date.clone(),
        effective_analysis_date: stock_analysis.effective_analysis_date.clone(),
        effective_trade_date: stock_analysis.effective_trade_date.clone(),
        local_data_last_date: stock_analysis.local_data_last_date.clone(),
        data_freshness_status: stock_analysis.data_freshness_status.clone(),
        sync_attempted: stock_analysis.sync_attempted,
        sync_result: stock_analysis.sync_result.clone(),
        date_fallback_reason: stock_analysis.date_fallback_reason.clone(),
    }
}

// 2026-04-01 CST: 这里统一下钻到底层技术面请求对象，原因是三层分析在当前阶段只差 symbol，其余窗口参数一致。
// 目的：让上层聚合逻辑保持单一职责，不在主流程重复拼 request。
fn build_basic_request(
    symbol: &str,
    as_of_date: Option<String>,
    lookback_days: usize,
) -> TechnicalConsultationBasicRequest {
    TechnicalConsultationBasicRequest {
        symbol: symbol.to_string(),
        as_of_date,
        lookback_days: lookback_days.max(DEFAULT_LOOKBACK_DAYS),
    }
}

// 2026-04-01 CST: 这里收口大盘代理解析，原因是方案 B 要把调用入口从“每次纯手填”推进到“symbol 或 profile 二选一”。
// 目的：降低调用门槛，同时在缺参或配置错误时返回清晰的业务层报错。
fn resolve_market_symbol(
    request: &SecurityAnalysisContextualRequest,
) -> Result<String, SecurityAnalysisContextualError> {
    if let Some(symbol) = request.market_symbol.as_ref() {
        return Ok(symbol.clone());
    }

    match request.market_profile.as_deref() {
        Some("a_share_core") => Ok("510300.SH".to_string()),
        Some(profile) => Err(SecurityAnalysisContextualError::UnsupportedMarketProfile {
            profile: profile.to_string(),
        }),
        None => Err(SecurityAnalysisContextualError::MissingMarketProxy),
    }
}

// 2026-04-01 CST: 这里收口板块代理解析，原因是方案 B 需要最小可维护的板块代理配置入口，而不是继续依赖外层重复传完整 symbol。
// 目的：先稳定支持 A 股银行板块 profile，后续再按范围扩展更多行业代理。
fn resolve_sector_symbol(
    request: &SecurityAnalysisContextualRequest,
) -> Result<String, SecurityAnalysisContextualError> {
    if let Some(symbol) = request.sector_symbol.as_ref() {
        return Ok(symbol.clone());
    }

    match request.sector_profile.as_deref() {
        Some("a_share_bank") => Ok("512800.SH".to_string()),
        Some(profile) => Err(SecurityAnalysisContextualError::UnsupportedSectorProfile {
            profile: profile.to_string(),
        }),
        None => Err(SecurityAnalysisContextualError::MissingSectorProxy),
    }
}

// 2026-04-01 CST: 这里聚合三层分析结论，原因是综合 Tool 的价值不在重算指标，而在把个股与环境方向关系收成稳定语义。
// 目的：先把顺风、逆风、混合环境做成正式合同，再视后续范围继续增加信息面等维度。
fn build_contextual_conclusion(
    stock_analysis: &TechnicalConsultationBasicResult,
    market_analysis: &TechnicalConsultationBasicResult,
    sector_analysis: &TechnicalConsultationBasicResult,
) -> SecurityContextualConclusion {
    let stock_direction = map_bias_direction(&stock_analysis.consultation_conclusion.bias);
    let market_direction = map_bias_direction(&market_analysis.consultation_conclusion.bias);
    let sector_direction = map_bias_direction(&sector_analysis.consultation_conclusion.bias);

    if stock_direction == BiasDirection::Neutral {
        return SecurityContextualConclusion {
            alignment: "mixed".to_string(),
            headline: "个股仍处等待态，环境偏强但暂不构成顺风确认".to_string(),
            rationale: vec![
                format!(
                    "大盘代理 {} 与板块代理 {} 当前方向同向，但个股 {} 仍未完成方向选择",
                    market_analysis.symbol, sector_analysis.symbol, stock_analysis.symbol
                ),
                "当前更适合把环境强弱视为观察背景，而不是直接替代个股确认信号".to_string(),
            ],
            risk_flags: vec![
                "个股自身尚未完成方向确认，贸然追随环境可能放大假突破风险".to_string(),
            ],
        };
    }

    if stock_direction == market_direction && stock_direction == sector_direction {
        let direction_text = match stock_direction {
            BiasDirection::Bullish => "多头",
            BiasDirection::Bearish => "空头",
            BiasDirection::Neutral => "中性",
        };
        return SecurityContextualConclusion {
            alignment: "tailwind".to_string(),
            headline: format!("个股与大盘、板块同向，当前属于{direction_text}顺风环境"),
            rationale: vec![
                format!(
                    "个股 {}、大盘代理 {}、板块代理 {} 当前技术面结论同向",
                    stock_analysis.symbol, market_analysis.symbol, sector_analysis.symbol
                ),
                "大盘与板块同向共振，说明当前环境对个股方向形成外部支持".to_string(),
            ],
            risk_flags: vec!["即使处于顺风环境，仍需跟踪个股关键位与量能确认是否持续".to_string()],
        };
    }

    let is_headwind = stock_direction != market_direction && stock_direction != sector_direction;

    SecurityContextualConclusion {
        alignment: if is_headwind {
            "headwind".to_string()
        } else {
            "mixed".to_string()
        },
        headline: if is_headwind {
            "个股方向与大盘、板块明显相反，当前属于逆风环境".to_string()
        } else {
            "个股方向与环境并未完全共振，当前更适合按混合环境处理".to_string()
        },
        rationale: vec![
            format!(
                "个股 {} 与大盘代理 {}、板块代理 {} 的技术面结论未形成完全同向",
                stock_analysis.symbol, market_analysis.symbol, sector_analysis.symbol
            ),
            if is_headwind {
                "大盘与板块同时逆向时，个股单独走强或走弱更容易受到环境压制".to_string()
            } else {
                "环境支持不足时，个股单独走强或走弱的持续性通常需要更严格验证".to_string()
            },
        ],
        risk_flags: vec![if is_headwind {
            "个股与环境逆向时，趋势延续概率通常低于顺风共振阶段".to_string()
        } else {
            "环境与个股不同步时，趋势延续概率通常低于完全同向阶段".to_string()
        }],
    }
}

// 2026-04-01 CST: 这里把底层 bias 映射成上层方向枚举，原因是综合 Tool 只需要判断方向是否同向，不需要重新解释全部技术面细节。
// 目的：隔离底层文案和上层聚合规则，减少未来扩展时的耦合。
fn map_bias_direction(bias: &str) -> BiasDirection {
    match bias {
        "bullish_continuation" | "bull_trap_risk" => BiasDirection::Bullish,
        "bearish_continuation" | "bear_trap_risk" => BiasDirection::Bearish,
        _ => BiasDirection::Neutral,
    }
}

// 2026-04-01 CST: 这里用内部枚举表达方向，原因是字符串比较过于脆弱，不适合作为聚合规则内部语义。
// 目的：让综合结论判断更稳定、更清晰。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BiasDirection {
    Bullish,
    Bearish,
    Neutral,
}

fn default_lookback_days() -> usize {
    DEFAULT_LOOKBACK_DAYS
}
