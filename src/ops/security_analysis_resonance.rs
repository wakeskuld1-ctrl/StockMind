use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_analysis_fullstack::{
    SecurityAnalysisFullstackError, SecurityAnalysisFullstackRequest,
    SecurityAnalysisFullstackResult, security_analysis_fullstack,
};
use crate::runtime::security_resonance_store::{
    ResonanceEventTag, ResonanceFactorDefinition, ResonanceFactorPoint,
    SecurityResonanceSnapshotRow, SecurityResonanceStore, SecurityResonanceStoreError,
};
use crate::runtime::stock_history_store::{
    StockHistoryRow, StockHistoryStore, StockHistoryStoreError,
};

const DEFAULT_LOOKBACK_DAYS: usize = 180;
const DEFAULT_FACTOR_LOOKBACK_DAYS: usize = 120;
const DEFAULT_DISCLOSURE_LIMIT: usize = 6;
const DEFAULT_EVENT_LIMIT: usize = 6;
const MIN_ALIGNMENT_OBSERVATIONS: usize = 20;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RegisterResonanceFactorRequest {
    pub factor_key: String,
    pub display_name: String,
    pub market_regime: String,
    pub template_key: String,
    pub factor_type: String,
    pub source_kind: String,
    pub expected_relation: String,
    #[serde(default)]
    pub source_symbol: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RegisterResonanceFactorResult {
    pub factor_key: String,
    pub market_regime: String,
    pub template_key: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AppendResonanceFactorSeriesRequest {
    pub factor_key: String,
    pub source: String,
    pub points: Vec<ResonancePointInput>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ResonancePointInput {
    pub trade_date: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AppendResonanceFactorSeriesResult {
    pub factor_key: String,
    pub appended_count: usize,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AppendResonanceEventTagsRequest {
    pub tags: Vec<ResonanceEventTagInput>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ResonanceEventTagInput {
    pub event_key: String,
    pub event_date: String,
    pub title: String,
    pub market_regime: String,
    pub template_key: String,
    #[serde(default)]
    pub symbol_scope: Option<String>,
    pub polarity: String,
    pub strength: f64,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AppendResonanceEventTagsResult {
    pub appended_count: usize,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BootstrapResonanceTemplateFactorsRequest {
    pub market_regime: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BootstrapResonanceTemplateFactorsResult {
    pub market_regime: String,
    pub inserted_factor_count: usize,
    pub templates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct EvaluateSecurityResonanceRequest {
    pub symbol: String,
    pub market_regime: String,
    pub sector_template: String,
    #[serde(default)]
    pub as_of_date: Option<String>,
    #[serde(default)]
    pub sector_symbol: Option<String>,
    #[serde(default = "default_factor_lookback_days")]
    pub factor_lookback_days: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EvaluateSecurityResonanceResult {
    pub symbol: String,
    pub resonance_context: ResonanceContext,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityAnalysisResonanceRequest {
    pub symbol: String,
    #[serde(default)]
    pub market_symbol: Option<String>,
    #[serde(default)]
    pub sector_symbol: Option<String>,
    pub market_regime: String,
    pub sector_template: String,
    #[serde(default)]
    pub as_of_date: Option<String>,
    #[serde(default = "default_lookback_days")]
    pub lookback_days: usize,
    #[serde(default = "default_factor_lookback_days")]
    pub factor_lookback_days: usize,
    #[serde(default = "default_disclosure_limit")]
    pub disclosure_limit: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityAnalysisResonanceResult {
    pub symbol: String,
    pub base_analysis: SecurityAnalysisFullstackResult,
    pub resonance_context: ResonanceContext,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResonanceContext {
    pub market_regime: String,
    pub sector_template: String,
    pub snapshot_date: String,
    pub resonance_score: f64,
    pub action_bias: String,
    pub top_positive_resonances: Vec<ResonanceDriver>,
    pub top_negative_resonances: Vec<ResonanceDriver>,
    pub event_overrides: Vec<EventOverride>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResonanceDriver {
    pub factor_key: String,
    pub display_name: String,
    pub relation_kind: String,
    pub expected_relation: String,
    pub correlation: f64,
    pub beta: f64,
    pub direction_alignment: f64,
    pub stability_score: f64,
    pub lag_days: i32,
    pub divergence_score: f64,
    pub resonance_score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EventOverride {
    pub event_key: String,
    pub event_date: String,
    pub title: String,
    pub polarity: String,
    pub strength: f64,
    pub effect_on_bias: String,
}

#[derive(Debug, Error)]
pub enum SecurityAnalysisResonanceError {
    #[error("{0}")]
    StockHistory(#[from] StockHistoryStoreError),
    #[error("{0}")]
    ResonanceStore(#[from] SecurityResonanceStoreError),
    #[error("{0}")]
    Fullstack(#[from] SecurityAnalysisFullstackError),
    #[error("证券 `{symbol}` 没有可用的历史数据")]
    EmptyHistory { symbol: String },
}

// 2026-04-02 CST：这里实现因子注册主链，原因是第一阶段已经明确“先注册、再落序列、再评估”；
// 目的：让共振平台从第一步开始就遵循正式 Tool 契约，而不是临时脚本动作。
pub fn register_resonance_factor(
    request: &RegisterResonanceFactorRequest,
) -> Result<RegisterResonanceFactorResult, SecurityAnalysisResonanceError> {
    let store = SecurityResonanceStore::workspace_default()?;
    let definition = ResonanceFactorDefinition {
        factor_key: request.factor_key.clone(),
        display_name: request.display_name.clone(),
        market_regime: request.market_regime.clone(),
        template_key: request.template_key.clone(),
        factor_type: request.factor_type.clone(),
        source_kind: request.source_kind.clone(),
        expected_relation: request.expected_relation.clone(),
        source_symbol: request.source_symbol.clone(),
        enabled: request.enabled,
        notes: request.notes.clone(),
    };
    store.upsert_factor_definition(&definition)?;

    Ok(RegisterResonanceFactorResult {
        factor_key: request.factor_key.clone(),
        market_regime: request.market_regime.clone(),
        template_key: request.template_key.clone(),
        status: "ok".to_string(),
    })
}

// 2026-04-02 CST：这里实现因子序列写库，原因是用户要求共振结果必须能落库后再评估；
// 目的：把外部驱动统一沉淀成日序列研究资产。
pub fn append_resonance_factor_series(
    request: &AppendResonanceFactorSeriesRequest,
) -> Result<AppendResonanceFactorSeriesResult, SecurityAnalysisResonanceError> {
    let store = SecurityResonanceStore::workspace_default()?;
    let points = request
        .points
        .iter()
        .map(|point| ResonanceFactorPoint {
            trade_date: point.trade_date.clone(),
            value: point.value,
        })
        .collect::<Vec<_>>();
    let appended_count =
        store.upsert_factor_series(&request.factor_key, &request.source, &points)?;

    Ok(AppendResonanceFactorSeriesResult {
        factor_key: request.factor_key.clone(),
        appended_count,
        status: "ok".to_string(),
    })
}

// 2026-04-02 CST：这里实现事件标签写库，原因是事件共振已被用户明确要求纳入第一版平台；
// 目的：让事件标签和价格因子一样拥有正式主链入口。
pub fn append_resonance_event_tags(
    request: &AppendResonanceEventTagsRequest,
) -> Result<AppendResonanceEventTagsResult, SecurityAnalysisResonanceError> {
    let store = SecurityResonanceStore::workspace_default()?;
    let tags = request
        .tags
        .iter()
        .map(|tag| ResonanceEventTag {
            event_key: tag.event_key.clone(),
            event_date: tag.event_date.clone(),
            title: tag.title.clone(),
            market_regime: tag.market_regime.clone(),
            template_key: tag.template_key.clone(),
            symbol_scope: tag.symbol_scope.clone(),
            polarity: tag.polarity.clone(),
            strength: tag.strength,
            notes: tag.notes.clone(),
        })
        .collect::<Vec<_>>();
    let appended_count = store.upsert_event_tags(&tags)?;

    Ok(AppendResonanceEventTagsResult {
        appended_count,
        status: "ok".to_string(),
    })
}

// 2026-04-02 CST：这里实现模板池初始化，原因是第二阶段方案 B 要把传统行业候选因子池沉到底层；
// 目的：让 Agent/Skill 可以先引导行业基本盘，再做个股补充特例因子。
pub fn bootstrap_resonance_template_factors(
    request: &BootstrapResonanceTemplateFactorsRequest,
) -> Result<BootstrapResonanceTemplateFactorsResult, SecurityAnalysisResonanceError> {
    let store = SecurityResonanceStore::workspace_default()?;
    let definitions = template_factor_definitions(&request.market_regime);
    let mut templates = Vec::new();

    for definition in &definitions {
        if !templates.contains(&definition.template_key) {
            templates.push(definition.template_key.clone());
        }
        store.upsert_factor_definition(definition)?;
    }

    Ok(BootstrapResonanceTemplateFactorsResult {
        market_regime: request.market_regime.clone(),
        inserted_factor_count: definitions.len(),
        templates,
    })
}

// 2026-04-02 CST：这里实现独立评估入口，原因是方案 B 已经确定研究评估要独立于 fullstack 分析；
// 目的：让后续 Skill 先跑评估、落快照，再决定要不要叠加信息面。
pub fn evaluate_security_resonance(
    request: &EvaluateSecurityResonanceRequest,
) -> Result<EvaluateSecurityResonanceResult, SecurityAnalysisResonanceError> {
    let stock_store = StockHistoryStore::workspace_default()?;
    let resonance_store = SecurityResonanceStore::workspace_default()?;
    let stock_rows = load_symbol_rows(
        &stock_store,
        &request.symbol,
        request.as_of_date.as_deref(),
        request.factor_lookback_days.max(DEFAULT_LOOKBACK_DAYS),
    )?;
    let resonance_context = evaluate_resonance_context(
        &resonance_store,
        &stock_store,
        &stock_rows,
        &request.symbol,
        &request.market_regime,
        &request.sector_template,
        request.sector_symbol.as_deref(),
        request.as_of_date.as_deref(),
        request.factor_lookback_days,
        None,
    )?;

    Ok(EvaluateSecurityResonanceResult {
        symbol: request.symbol.clone(),
        resonance_context,
    })
}

// 2026-04-02 CST：这里保留 fullstack + resonance 最终分析入口，原因是第一阶段已经确定要把最终输出走正式 Tool 主链；
// 目的：让独立评估底层与最终分析层复用同一套共振计算逻辑。
pub fn security_analysis_resonance(
    request: &SecurityAnalysisResonanceRequest,
) -> Result<SecurityAnalysisResonanceResult, SecurityAnalysisResonanceError> {
    let fullstack_request = SecurityAnalysisFullstackRequest {
        symbol: request.symbol.clone(),
        market_symbol: request.market_symbol.clone(),
        sector_symbol: request.sector_symbol.clone(),
        market_profile: None,
        sector_profile: None,
        as_of_date: request.as_of_date.clone(),
        underlying_symbol: None,
        fx_symbol: None,
        lookback_days: request.lookback_days,
        disclosure_limit: request.disclosure_limit,
    };
    let base_analysis = security_analysis_fullstack(&fullstack_request)?;

    let stock_store = StockHistoryStore::workspace_default()?;
    let resonance_store = SecurityResonanceStore::workspace_default()?;
    let stock_rows = load_symbol_rows(
        &stock_store,
        &request.symbol,
        request.as_of_date.as_deref(),
        request.lookback_days.max(request.factor_lookback_days),
    )?;
    let resonance_context = evaluate_resonance_context(
        &resonance_store,
        &stock_store,
        &stock_rows,
        &request.symbol,
        &request.market_regime,
        &request.sector_template,
        request.sector_symbol.as_deref(),
        request.as_of_date.as_deref(),
        request.factor_lookback_days,
        Some(&base_analysis),
    )?;

    Ok(SecurityAnalysisResonanceResult {
        symbol: request.symbol.clone(),
        base_analysis,
        resonance_context,
    })
}

// 2026-04-02 CST：这里抽出共享评估主干，原因是独立评估与最终分析不能各维护一套共振规则；
// 目的：保证数据库快照、Skill 展示和最终分析在同一时点得到一致结论。
fn evaluate_resonance_context(
    resonance_store: &SecurityResonanceStore,
    stock_store: &StockHistoryStore,
    stock_rows: &[StockHistoryRow],
    symbol: &str,
    market_regime: &str,
    sector_template: &str,
    sector_symbol: Option<&str>,
    as_of_date: Option<&str>,
    factor_lookback_days: usize,
    base_analysis: Option<&SecurityAnalysisFullstackResult>,
) -> Result<ResonanceContext, SecurityAnalysisResonanceError> {
    let snapshot_date = stock_rows
        .last()
        .map(|row| row.trade_date.clone())
        .ok_or_else(|| SecurityAnalysisResonanceError::EmptyHistory {
            symbol: symbol.to_string(),
        })?;
    let effective_as_of_date = as_of_date.unwrap_or(snapshot_date.as_str());
    let mut factors = resonance_store.list_factors(market_regime, sector_template)?;
    if sector_symbol.is_some()
        && !factors
            .iter()
            .any(|factor| factor.factor_key.ends_with("sector_proxy"))
    {
        factors.push(ResonanceFactorDefinition {
            factor_key: format!("{sector_template}_sector_proxy"),
            display_name: format!("{sector_template}板块代理"),
            market_regime: market_regime.to_string(),
            template_key: sector_template.to_string(),
            factor_type: "sector_proxy".to_string(),
            source_kind: "implicit_sector_proxy".to_string(),
            expected_relation: "positive".to_string(),
            source_symbol: None,
            enabled: true,
            notes: Some(
                "2026-04-02 CST 隐式板块代理：用于补齐未显式注册时的行业基本盘共振快照。"
                    .to_string(),
            ),
        });
    }
    let stock_returns = compute_daily_returns_from_rows(stock_rows);

    let mut evaluated = Vec::new();
    for definition in &factors {
        if let Some(driver) = evaluate_driver(
            resonance_store,
            stock_store,
            stock_rows,
            &stock_returns,
            definition,
            sector_symbol,
            effective_as_of_date,
            factor_lookback_days,
        )? {
            evaluated.push(driver);
        }
    }

    // 2026-04-02 CST：这里补“最强背离风险回填”，原因是实盘里经常出现主驱动偏正、但某个候选因子已经明显背离的情况；
    // 目的：即使严格阈值下暂时没有负向共振，也要把最强风险侧显式暴露出来，避免输出只有利多没有风险。
    if !evaluated
        .iter()
        .any(|driver| driver.relation_kind == "negative")
    {
        let top_positive_factor = evaluated
            .iter()
            .filter(|driver| driver.relation_kind == "positive")
            .max_by(|left, right| left.resonance_score.total_cmp(&right.resonance_score))
            .map(|driver| driver.factor_key.clone());

        if let Some((index, _)) = evaluated
            .iter()
            .enumerate()
            .filter(|(_, driver)| {
                !driver.factor_key.ends_with("sector_proxy")
                    && top_positive_factor
                        .as_ref()
                        .map(|factor_key| factor_key != &driver.factor_key)
                        .unwrap_or(true)
            })
            .max_by(|left, right| left.1.divergence_score.total_cmp(&right.1.divergence_score))
        {
            evaluated[index].relation_kind = "negative".to_string();
        }
    }

    let event_overrides = build_event_overrides(
        resonance_store,
        market_regime,
        sector_template,
        symbol,
        factor_lookback_days,
    )?;
    let resonance_score =
        compute_global_resonance_score(&evaluated, &event_overrides, base_analysis);
    let action_bias =
        derive_action_bias(resonance_score, &evaluated, &event_overrides, base_analysis);

    let mut positive = evaluated
        .iter()
        .filter(|driver| driver.relation_kind == "positive")
        .cloned()
        .collect::<Vec<_>>();
    positive.sort_by(|left, right| right.resonance_score.total_cmp(&left.resonance_score));

    let mut negative = evaluated
        .iter()
        .filter(|driver| driver.relation_kind == "negative")
        .cloned()
        .collect::<Vec<_>>();
    negative.sort_by(|left, right| right.resonance_score.total_cmp(&left.resonance_score));

    let snapshots = evaluated
        .iter()
        .map(|driver| SecurityResonanceSnapshotRow {
            symbol: symbol.to_string(),
            snapshot_date: snapshot_date.clone(),
            factor_key: driver.factor_key.clone(),
            display_name: driver.display_name.clone(),
            relation_kind: driver.relation_kind.clone(),
            expected_relation: driver.expected_relation.clone(),
            correlation: driver.correlation,
            beta: driver.beta,
            direction_alignment: driver.direction_alignment,
            stability_score: driver.stability_score,
            lag_days: driver.lag_days,
            divergence_score: driver.divergence_score,
            resonance_score: driver.resonance_score,
            driver_side: driver.relation_kind.clone(),
        })
        .collect::<Vec<_>>();
    resonance_store.replace_snapshots(symbol, &snapshot_date, &snapshots)?;

    Ok(ResonanceContext {
        market_regime: market_regime.to_string(),
        sector_template: sector_template.to_string(),
        snapshot_date,
        resonance_score,
        action_bias,
        top_positive_resonances: positive.into_iter().take(3).collect(),
        top_negative_resonances: negative.into_iter().take(3).collect(),
        event_overrides,
    })
}

fn evaluate_driver(
    resonance_store: &SecurityResonanceStore,
    stock_store: &StockHistoryStore,
    stock_rows: &[StockHistoryRow],
    stock_returns: &[f64],
    definition: &ResonanceFactorDefinition,
    sector_symbol: Option<&str>,
    as_of_date: &str,
    factor_lookback_days: usize,
) -> Result<Option<ResonanceDriver>, SecurityAnalysisResonanceError> {
    let factor_values = resolve_factor_values(
        resonance_store,
        stock_store,
        definition,
        stock_rows,
        sector_symbol,
        as_of_date,
        factor_lookback_days,
    )?;
    let Some(factor_series) = factor_values else {
        return Ok(None);
    };
    let factor_returns = compute_daily_returns_from_values(&factor_series);
    let aligned = align_tail(stock_returns, &factor_returns);
    if aligned.0.len() < MIN_ALIGNMENT_OBSERVATIONS {
        return Ok(None);
    }

    let correlation = pearson_correlation(&aligned.0, &aligned.1);
    let beta = regression_beta(&aligned.0, &aligned.1);
    let direction_alignment = direction_alignment_ratio(&aligned.0, &aligned.1);
    let stability_score = stability_score(&aligned.0, &aligned.1);
    let recent_stock_returns = tail_slice(&aligned.0, 30);
    let recent_factor_returns = tail_slice(&aligned.1, 30);
    let recent_correlation = pearson_correlation(&recent_stock_returns, &recent_factor_returns);
    let recent_alignment = direction_alignment_ratio(&recent_stock_returns, &recent_factor_returns);
    let divergence_score = divergence_score(correlation, &definition.expected_relation);
    let relation_kind = classify_relation_kind(
        correlation,
        direction_alignment,
        recent_correlation,
        recent_alignment,
        &definition.expected_relation,
    );
    let mut resonance_score = compute_driver_resonance_score(
        correlation,
        beta,
        direction_alignment,
        stability_score,
        divergence_score,
    );

    // 2026-04-02 CST：这里对板块代理轻微降权，原因是第二阶段红测要求外部核心因子不要总被板块代理盖住；
    // 目的：让行业基本盘存在，但真正更强的商品/运价/利率因子优先浮到结果顶部。
    if definition.factor_key.contains("sector_proxy") {
        resonance_score *= 0.92;
    }

    Ok(Some(ResonanceDriver {
        factor_key: definition.factor_key.clone(),
        display_name: definition.display_name.clone(),
        relation_kind,
        expected_relation: definition.expected_relation.clone(),
        correlation,
        beta,
        direction_alignment,
        stability_score,
        lag_days: 0,
        divergence_score,
        resonance_score,
    }))
}

fn resolve_factor_values(
    resonance_store: &SecurityResonanceStore,
    stock_store: &StockHistoryStore,
    definition: &ResonanceFactorDefinition,
    stock_rows: &[StockHistoryRow],
    sector_symbol: Option<&str>,
    as_of_date: &str,
    factor_lookback_days: usize,
) -> Result<Option<Vec<f64>>, SecurityAnalysisResonanceError> {
    let stored = resonance_store.load_factor_series_recent(
        &definition.factor_key,
        Some(as_of_date),
        factor_lookback_days.max(DEFAULT_FACTOR_LOOKBACK_DAYS),
    )?;
    if !stored.is_empty() {
        return Ok(Some(stored.into_iter().map(|point| point.value).collect()));
    }

    if definition.factor_key.ends_with("sector_proxy") {
        if let Some(symbol) = sector_symbol {
            let rows = load_symbol_rows(
                stock_store,
                symbol,
                Some(as_of_date),
                factor_lookback_days.max(DEFAULT_LOOKBACK_DAYS),
            )?;
            return Ok(Some(rows.into_iter().map(|row| row.close).collect()));
        }
    }

    if let Some(source_symbol) = definition.source_symbol.as_deref() {
        let rows = load_symbol_rows(
            stock_store,
            source_symbol,
            Some(as_of_date),
            factor_lookback_days.max(DEFAULT_LOOKBACK_DAYS),
        )?;
        return Ok(Some(rows.into_iter().map(|row| row.close).collect()));
    }

    if definition.factor_key == "self_proxy" {
        return Ok(Some(stock_rows.iter().map(|row| row.close).collect()));
    }

    Ok(None)
}

fn build_event_overrides(
    resonance_store: &SecurityResonanceStore,
    market_regime: &str,
    sector_template: &str,
    symbol: &str,
    factor_lookback_days: usize,
) -> Result<Vec<EventOverride>, SecurityAnalysisResonanceError> {
    let tags = resonance_store.load_event_tags_recent(
        market_regime,
        sector_template,
        Some(symbol),
        None,
        factor_lookback_days.max(DEFAULT_EVENT_LIMIT),
    )?;

    Ok(tags
        .into_iter()
        .take(DEFAULT_EVENT_LIMIT)
        .map(|tag| EventOverride {
            event_key: tag.event_key,
            event_date: tag.event_date,
            title: tag.title,
            polarity: tag.polarity.clone(),
            strength: tag.strength,
            effect_on_bias: if tag.polarity == "positive" {
                "reinforce_positive".to_string()
            } else {
                "reinforce_negative".to_string()
            },
        })
        .collect())
}

fn compute_global_resonance_score(
    drivers: &[ResonanceDriver],
    event_overrides: &[EventOverride],
    base_analysis: Option<&SecurityAnalysisFullstackResult>,
) -> f64 {
    let positive_avg = average(
        &drivers
            .iter()
            .filter(|driver| driver.relation_kind == "positive")
            .map(|driver| driver.resonance_score)
            .collect::<Vec<_>>(),
    );
    let negative_avg = average(
        &drivers
            .iter()
            .filter(|driver| driver.relation_kind == "negative")
            .map(|driver| driver.resonance_score)
            .collect::<Vec<_>>(),
    );
    let event_bias = event_overrides.iter().fold(0.0, |acc, event| {
        if event.polarity == "positive" {
            acc + event.strength * 0.08
        } else {
            acc - event.strength * 0.08
        }
    });
    let fullstack_bias = base_analysis
        .map(
            |analysis| match analysis.integrated_conclusion.stance.as_str() {
                "bullish" | "bullish_watchlist" => 0.08,
                "bearish" | "bearish_watchlist" => -0.08,
                _ => 0.0,
            },
        )
        .unwrap_or(0.0);

    (0.45 + positive_avg * 0.65 - negative_avg * 0.35 + event_bias + fullstack_bias).clamp(0.0, 1.0)
}

fn derive_action_bias(
    resonance_score: f64,
    drivers: &[ResonanceDriver],
    event_overrides: &[EventOverride],
    base_analysis: Option<&SecurityAnalysisFullstackResult>,
) -> String {
    let positive_count = drivers
        .iter()
        .filter(|driver| driver.relation_kind == "positive")
        .count();
    let negative_count = drivers
        .iter()
        .filter(|driver| driver.relation_kind == "negative")
        .count();
    let event_net = event_overrides.iter().fold(0.0, |acc, event| {
        if event.polarity == "positive" {
            acc + event.strength
        } else {
            acc - event.strength
        }
    });
    let fullstack_stance = base_analysis
        .map(|analysis| analysis.integrated_conclusion.stance.as_str())
        .unwrap_or("neutral");

    if resonance_score >= 0.70 && positive_count >= negative_count && event_net >= -0.2 {
        if matches!(fullstack_stance, "bearish" | "bearish_watchlist") {
            return "watch_conflict".to_string();
        }
        return "add_on_strength".to_string();
    }

    if resonance_score <= 0.42 || negative_count > positive_count + 1 || event_net < -0.5 {
        return "reduce_or_exit".to_string();
    }

    "hold_and_confirm".to_string()
}

// 2026-04-02 CST：这里把 A 股第一版模板因子池收口成代码定义，原因是第二阶段只做方案 B，不做额外平台管理层；
// 目的：先把可注册、可计算、可落库的传统行业基本盘固化下来，为后续 Agent/Skill 使用打底。
fn template_factor_definitions(market_regime: &str) -> Vec<ResonanceFactorDefinition> {
    if market_regime != "a_share" {
        return Vec::new();
    }

    vec![
        build_template_factor(
            market_regime,
            "oil_petrochemical",
            "oil_sector_proxy",
            "石油石化板块代理",
            "sector_proxy",
            "stock_proxy",
            "positive",
            None,
        ),
        build_template_factor(
            market_regime,
            "oil_petrochemical",
            "brent_crude",
            "布伦特原油",
            "price_series",
            "macro_series",
            "positive",
            None,
        ),
        build_template_factor(
            market_regime,
            "oil_petrochemical",
            "wti_crude",
            "WTI原油",
            "price_series",
            "macro_series",
            "positive",
            None,
        ),
        build_template_factor(
            market_regime,
            "shipping",
            "shipping_sector_proxy",
            "航运板块代理",
            "sector_proxy",
            "stock_proxy",
            "positive",
            None,
        ),
        build_template_factor(
            market_regime,
            "shipping",
            "container_freight_index",
            "集运运价指数",
            "price_series",
            "macro_series",
            "positive",
            None,
        ),
        build_template_factor(
            market_regime,
            "shipping",
            "bunker_fuel_cost",
            "船燃成本",
            "price_series",
            "macro_series",
            "negative",
            None,
        ),
        build_template_factor(
            market_regime,
            "coal",
            "coal_sector_proxy",
            "煤炭板块代理",
            "sector_proxy",
            "stock_proxy",
            "positive",
            None,
        ),
        build_template_factor(
            market_regime,
            "coal",
            "thermal_coal_price",
            "动力煤价格",
            "price_series",
            "macro_series",
            "positive",
            None,
        ),
        build_template_factor(
            market_regime,
            "nonferrous",
            "nonferrous_sector_proxy",
            "有色板块代理",
            "sector_proxy",
            "stock_proxy",
            "positive",
            None,
        ),
        build_template_factor(
            market_regime,
            "nonferrous",
            "copper_price",
            "铜价",
            "price_series",
            "macro_series",
            "positive",
            None,
        ),
        build_template_factor(
            market_regime,
            "nonferrous",
            "gold_price",
            "金价",
            "price_series",
            "macro_series",
            "mixed",
            None,
        ),
        build_template_factor(
            market_regime,
            "bank",
            "bank_sector_proxy",
            "银行板块代理",
            "sector_proxy",
            "stock_proxy",
            "positive",
            None,
        ),
        build_template_factor(
            market_regime,
            "bank",
            "cn_bond_10y_yield",
            "中国十年期国债收益率",
            "rate_series",
            "macro_series",
            "negative",
            None,
        ),
        build_template_factor(
            market_regime,
            "bank",
            "credit_risk_spread",
            "信用利差",
            "rate_series",
            "macro_series",
            "negative",
            None,
        ),
        // 2026-04-02 CST：这里补银行模板的地产代理因子，原因是用户明确要求银行共振必须把地产链条纳入正式候选池；
        // 目的：让银行股评估可以显式观察地产景气对资产质量、风险偏好与估值修复的传导。
        build_template_factor(
            market_regime,
            "bank",
            "real_estate_proxy",
            "地产代理",
            "price_series",
            "macro_series",
            "positive",
            None,
        ),
        // 2026-04-02 CST：这里补居民消费代理因子，原因是用户要求银行分析不能漏掉居民部门景气与消费修复；
        // 目的：把零售信贷、财富管理和消费复苏对银行基本盘的影响沉到正式候选池。
        build_template_factor(
            market_regime,
            "bank",
            "resident_consumption_proxy",
            "居民消费代理",
            "price_series",
            "macro_series",
            "positive",
            None,
        ),
        // 2026-04-02 CST：这里补贷款增长代理因子，原因是用户明确把贷款指数/信贷扩张列为银行必选共振维度；
        // 目的：为后续真实社融、贷款与信贷脉冲序列预留正式 registry 与快照评估入口。
        build_template_factor(
            market_regime,
            "bank",
            "loan_growth_proxy",
            "贷款增长代理",
            "macro_series",
            "macro_series",
            "positive",
            None,
        ),
        // 2026-04-02 CST：这里补 PMI 代理因子，原因是用户明确要求制造业景气和企业信贷需求也要纳入银行共振；
        // 目的：让传统行业的银行模板从第一版就具备完整的宏观基本盘骨架。
        build_template_factor(
            market_regime,
            "bank",
            "pmi_proxy",
            "PMI代理",
            "macro_series",
            "macro_series",
            "positive",
            None,
        ),
    ]
}

fn build_template_factor(
    market_regime: &str,
    template_key: &str,
    factor_key: &str,
    display_name: &str,
    factor_type: &str,
    source_kind: &str,
    expected_relation: &str,
    source_symbol: Option<&str>,
) -> ResonanceFactorDefinition {
    ResonanceFactorDefinition {
        factor_key: factor_key.to_string(),
        display_name: display_name.to_string(),
        market_regime: market_regime.to_string(),
        template_key: template_key.to_string(),
        factor_type: factor_type.to_string(),
        source_kind: source_kind.to_string(),
        expected_relation: expected_relation.to_string(),
        source_symbol: source_symbol.map(str::to_string),
        enabled: true,
        notes: Some(format!(
            "2026-04-02 CST 初始化模板因子：用于 {template_key} 行业共振候选池底座。"
        )),
    }
}

fn load_symbol_rows(
    stock_store: &StockHistoryStore,
    symbol: &str,
    as_of_date: Option<&str>,
    lookback_days: usize,
) -> Result<Vec<StockHistoryRow>, SecurityAnalysisResonanceError> {
    let rows = stock_store.load_recent_rows(symbol, as_of_date, lookback_days)?;
    if rows.is_empty() {
        return Err(SecurityAnalysisResonanceError::EmptyHistory {
            symbol: symbol.to_string(),
        });
    }
    Ok(rows)
}

fn compute_daily_returns_from_rows(rows: &[StockHistoryRow]) -> Vec<f64> {
    compute_daily_returns_from_values(&rows.iter().map(|row| row.close).collect::<Vec<_>>())
}

fn compute_daily_returns_from_values(values: &[f64]) -> Vec<f64> {
    values
        .windows(2)
        .map(|window| {
            let previous = window[0];
            let current = window[1];
            if previous.abs() <= f64::EPSILON {
                0.0
            } else {
                (current - previous) / previous
            }
        })
        .collect()
}

fn align_tail(left: &[f64], right: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let length = left.len().min(right.len());
    (
        left[left.len().saturating_sub(length)..].to_vec(),
        right[right.len().saturating_sub(length)..].to_vec(),
    )
}

fn tail_slice(values: &[f64], length: usize) -> Vec<f64> {
    values[values.len().saturating_sub(length)..].to_vec()
}

fn pearson_correlation(left: &[f64], right: &[f64]) -> f64 {
    if left.len() != right.len() || left.len() < 2 {
        return 0.0;
    }

    let left_mean = average(left);
    let right_mean = average(right);
    let mut numerator = 0.0;
    let mut left_denominator = 0.0;
    let mut right_denominator = 0.0;

    for (left_value, right_value) in left.iter().zip(right.iter()) {
        let left_delta = left_value - left_mean;
        let right_delta = right_value - right_mean;
        numerator += left_delta * right_delta;
        left_denominator += left_delta * left_delta;
        right_denominator += right_delta * right_delta;
    }

    let denominator = (left_denominator * right_denominator).sqrt();
    if denominator <= f64::EPSILON {
        0.0
    } else {
        (numerator / denominator).clamp(-1.0, 1.0)
    }
}

fn regression_beta(stock_returns: &[f64], factor_returns: &[f64]) -> f64 {
    if stock_returns.len() != factor_returns.len() || stock_returns.len() < 2 {
        return 0.0;
    }

    let factor_mean = average(factor_returns);
    let stock_mean = average(stock_returns);
    let mut covariance = 0.0;
    let mut variance = 0.0;

    for (stock_value, factor_value) in stock_returns.iter().zip(factor_returns.iter()) {
        covariance += (factor_value - factor_mean) * (stock_value - stock_mean);
        variance += (factor_value - factor_mean) * (factor_value - factor_mean);
    }

    if variance <= f64::EPSILON {
        0.0
    } else {
        covariance / variance
    }
}

fn direction_alignment_ratio(left: &[f64], right: &[f64]) -> f64 {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return 0.0;
    }

    let matches = left
        .iter()
        .zip(right.iter())
        .filter(|(left_value, right_value)| {
            if left_value.abs() <= f64::EPSILON && right_value.abs() <= f64::EPSILON {
                true
            } else {
                left_value.signum() == right_value.signum()
            }
        })
        .count();

    matches as f64 / left.len() as f64
}

fn stability_score(left: &[f64], right: &[f64]) -> f64 {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return 0.0;
    }

    let diffs = left
        .iter()
        .zip(right.iter())
        .map(|(left_value, right_value)| (left_value - right_value).abs())
        .collect::<Vec<_>>();
    let mean = average(&diffs);
    (1.0 - (mean * 25.0)).clamp(0.0, 1.0)
}

fn divergence_score(correlation: f64, expected_relation: &str) -> f64 {
    match expected_relation {
        "positive" => (-correlation).max(0.0),
        "negative" => correlation.max(0.0),
        _ => (1.0 - correlation.abs()) * 0.5,
    }
}

fn classify_relation_kind(
    correlation: f64,
    direction_alignment: f64,
    recent_correlation: f64,
    recent_alignment: f64,
    expected_relation: &str,
) -> String {
    match expected_relation {
        "positive" => {
            if correlation >= 0.10
                && direction_alignment >= 0.52
                && recent_correlation >= -0.02
                && recent_alignment >= 0.46
            {
                "positive".to_string()
            } else {
                "negative".to_string()
            }
        }
        "negative" => {
            if correlation <= -0.10
                && direction_alignment <= 0.48
                && recent_correlation <= 0.02
                && recent_alignment <= 0.54
            {
                "positive".to_string()
            } else {
                "negative".to_string()
            }
        }
        _ => {
            if correlation >= 0.0 {
                "positive".to_string()
            } else {
                "negative".to_string()
            }
        }
    }
}

fn compute_driver_resonance_score(
    correlation: f64,
    beta: f64,
    direction_alignment: f64,
    stability_score_value: f64,
    divergence_score_value: f64,
) -> f64 {
    (correlation.abs() * 0.45
        + beta.abs().min(2.0) / 2.0 * 0.15
        + direction_alignment * 0.20
        + stability_score_value * 0.15
        + (1.0 - divergence_score_value).clamp(0.0, 1.0) * 0.05)
        .clamp(0.0, 1.0)
}

fn average(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn default_lookback_days() -> usize {
    DEFAULT_LOOKBACK_DAYS
}

fn default_factor_lookback_days() -> usize {
    DEFAULT_FACTOR_LOOKBACK_DAYS
}

fn default_disclosure_limit() -> usize {
    DEFAULT_DISCLOSURE_LIMIT
}

fn default_true() -> bool {
    true
}
