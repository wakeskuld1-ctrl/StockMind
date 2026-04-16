use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_decision_evidence_bundle::SecurityExternalProxyInputs;
use crate::ops::stock::security_feature_snapshot::{
    SecurityFeatureSnapshot, SecurityFeatureSnapshotError, SecurityFeatureSnapshotRequest,
    security_feature_snapshot,
};
use crate::runtime::stock_history_store::{
    StockHistoryRow, StockHistoryStore, StockHistoryStoreError,
};

// 2026-04-09 CST: 这里新增 forward_outcome 请求合同，原因是 Task 3 需要把“未来标签回填”收口成正式 Tool，而不是外层零散传参计算；
// 目的：让 CLI / Skill / 后续训练入口都能通过统一请求，生成绑定 snapshot 的多期限正式标签对象。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityForwardOutcomeRequest {
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
    #[serde(default = "default_disclosure_limit")]
    pub disclosure_limit: usize,
    #[serde(default = "default_horizons")]
    pub horizons: Vec<usize>,
    #[serde(default = "default_stop_loss_pct")]
    pub stop_loss_pct: f64,
    #[serde(default = "default_target_return_pct")]
    pub target_return_pct: f64,
    #[serde(default = "default_label_definition_version")]
    pub label_definition_version: String,
    #[serde(default)]
    pub external_proxy_inputs: Option<SecurityExternalProxyInputs>,
}

// 2026-04-09 CST: 这里新增正式未来标签对象，原因是 Task 3 要把“回看收益/回撤/事件”升级为可训练、可复盘、可审计的正式 artifact；
// 目的：把 snapshot 绑定、期限、收益路径标签和版本信息固定下来，避免后续训练主链再回头改合同。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityForwardOutcomeDocument {
    pub outcome_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub snapshot_id: String,
    pub symbol: String,
    pub market: String,
    pub instrument_type: String,
    pub as_of_date: String,
    pub horizon_days: usize,
    pub forward_return: f64,
    pub max_drawdown: f64,
    pub max_runup: f64,
    pub positive_return: bool,
    pub hit_upside_first: bool,
    pub hit_stop_first: bool,
    pub label_definition_version: String,
}

// 2026-04-09 CST: 这里新增 forward_outcome Tool 的聚合返回对象，原因是方案 B 要把未来标签显式绑定到 feature_snapshot；
// 目的：让后续训练/回算链路可以一次拿到 snapshot 与多期限标签，而不是在外层手工拼接对象关系。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityForwardOutcomeResult {
    pub snapshot: SecurityFeatureSnapshot,
    pub forward_outcomes: Vec<SecurityForwardOutcomeDocument>,
}

// 2026-04-09 CST: 这里集中定义 forward_outcome 错误边界，原因是该 Tool 同时依赖 snapshot 构建、SQLite 历史读取和标签计算；
// 目的：把失败原因翻译成稳定、可定位的正式错误，便于 CLI 与后续训练链路排障。
#[derive(Debug, Error)]
pub enum SecurityForwardOutcomeError {
    #[error("security forward outcome snapshot preparation failed: {0}")]
    Snapshot(#[from] SecurityFeatureSnapshotError),
    #[error("security forward outcome history loading failed: {0}")]
    History(#[from] StockHistoryStoreError),
    #[error("security forward outcome build failed: {0}")]
    Build(String),
}

// 2026-04-09 CST: 这里实现 Task 3 的正式未来标签入口，原因是我们需要把多期限 forward label 从临时分析升级为正式产品能力；
// 目的：生成绑定 snapshot 的 5/10/20/30/60/180 期限收益、回撤、runup 与止盈止损事件标签，供后续训练与复盘复用。
pub fn security_forward_outcome(
    request: &SecurityForwardOutcomeRequest,
) -> Result<SecurityForwardOutcomeResult, SecurityForwardOutcomeError> {
    let snapshot = security_feature_snapshot(&SecurityFeatureSnapshotRequest {
        symbol: request.symbol.clone(),
        market_symbol: request.market_symbol.clone(),
        sector_symbol: request.sector_symbol.clone(),
        market_profile: request.market_profile.clone(),
        sector_profile: request.sector_profile.clone(),
        as_of_date: request.as_of_date.clone(),
        underlying_symbol: None,
        fx_symbol: None,
        lookback_days: request.lookback_days,
        disclosure_limit: request.disclosure_limit,
        stop_loss_pct: request.stop_loss_pct,
        target_return_pct: request.target_return_pct,
        external_proxy_inputs: request.external_proxy_inputs.clone(),
    })?;
    let horizons = normalize_horizons(&request.horizons);
    let max_horizon = horizons.iter().copied().max().ok_or_else(|| {
        SecurityForwardOutcomeError::Build("no valid horizons provided".to_string())
    })?;

    let store = StockHistoryStore::workspace_default()?;
    let entry_row = load_entry_row(&store, &snapshot.symbol, &snapshot.as_of_date)?;
    let future_rows =
        store.load_forward_rows(&snapshot.symbol, &snapshot.as_of_date, max_horizon)?;
    if future_rows.len() < max_horizon {
        return Err(SecurityForwardOutcomeError::Build(format!(
            "insufficient future rows for {} at {}: required {}, available {}",
            snapshot.symbol,
            snapshot.as_of_date,
            max_horizon,
            future_rows.len()
        )));
    }

    let forward_outcomes = horizons
        .into_iter()
        .map(|horizon_days| {
            build_forward_outcome_document(
                &snapshot,
                &entry_row,
                &future_rows[..horizon_days],
                horizon_days,
                &request.label_definition_version,
                request.stop_loss_pct,
                request.target_return_pct,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(SecurityForwardOutcomeResult {
        snapshot,
        forward_outcomes,
    })
}

// 2026-04-09 CST: 这里统一从 snapshot 对应的分析日取入场基准行，原因是 forward_return / drawdown / runup 都必须绑定同一时点基准；
// 目的：确保所有未来标签都以 snapshot 的 as_of_date 为唯一锚点，避免标签与快照错位。
fn load_entry_row(
    store: &StockHistoryStore,
    symbol: &str,
    as_of_date: &str,
) -> Result<StockHistoryRow, SecurityForwardOutcomeError> {
    let recent_rows = store.load_recent_rows(symbol, Some(as_of_date), 1)?;
    let entry_row = recent_rows.last().cloned().ok_or_else(|| {
        SecurityForwardOutcomeError::Build(format!(
            "missing entry row for {} at {}",
            symbol, as_of_date
        ))
    })?;
    if entry_row.trade_date != as_of_date {
        return Err(SecurityForwardOutcomeError::Build(format!(
            "entry row drift for {}: expected {}, got {}",
            symbol, as_of_date, entry_row.trade_date
        )));
    }
    Ok(entry_row)
}

// 2026-04-09 CST: 这里构建单一期限的正式 outcome 文档，原因是后续训练/回算需要逐期限独立消费标签对象；
// 目的：把每个期限的收益、路径风险和事件标签固定为稳定的 artifact 结构。
fn build_forward_outcome_document(
    snapshot: &SecurityFeatureSnapshot,
    entry_row: &StockHistoryRow,
    future_rows: &[StockHistoryRow],
    horizon_days: usize,
    label_definition_version: &str,
    stop_loss_pct: f64,
    target_return_pct: f64,
) -> Result<SecurityForwardOutcomeDocument, SecurityForwardOutcomeError> {
    let final_row = future_rows.last().ok_or_else(|| {
        SecurityForwardOutcomeError::Build(format!(
            "missing final row for horizon {} on {}",
            horizon_days, snapshot.symbol
        ))
    })?;
    let entry_price = entry_row.adj_close;
    if entry_price <= 0.0 {
        return Err(SecurityForwardOutcomeError::Build(format!(
            "entry price must be positive for {} at {}",
            snapshot.symbol, snapshot.as_of_date
        )));
    }

    let forward_return = final_row.adj_close / entry_price - 1.0;
    let max_drawdown = compute_max_drawdown(entry_price, future_rows);
    let max_runup = compute_max_runup(entry_price, future_rows);
    let (hit_upside_first, hit_stop_first) =
        compute_event_hits(entry_price, future_rows, stop_loss_pct, target_return_pct);

    Ok(SecurityForwardOutcomeDocument {
        outcome_id: format!("forward-outcome-{}-{}d", snapshot.snapshot_id, horizon_days),
        contract_version: "security_forward_outcome.v1".to_string(),
        document_type: "security_forward_outcome".to_string(),
        snapshot_id: snapshot.snapshot_id.clone(),
        symbol: snapshot.symbol.clone(),
        market: snapshot.market.clone(),
        instrument_type: snapshot.instrument_type.clone(),
        as_of_date: snapshot.as_of_date.clone(),
        horizon_days,
        forward_return,
        max_drawdown,
        max_runup,
        positive_return: forward_return > 0.0,
        hit_upside_first,
        hit_stop_first,
        label_definition_version: label_definition_version.to_string(),
    })
}

// 2026-04-09 CST: 这里按收盘路径计算最大回撤，原因是 Task 3 第一版需要稳定、可复算的路径风险标签；
// 目的：先用 close-to-close 路径统一口径，避免引入额外盘中顺序假设。
fn compute_max_drawdown(entry_price: f64, future_rows: &[StockHistoryRow]) -> f64 {
    let mut running_peak = entry_price;
    let mut max_drawdown = 0.0_f64;

    for row in future_rows {
        if row.adj_close > running_peak {
            running_peak = row.adj_close;
        }
        let drawdown = 1.0 - row.adj_close / running_peak;
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
        }
    }

    max_drawdown
}

// 2026-04-09 CST: 这里按收盘路径计算最大 runup，原因是 Task 3 需要为后续训练层提供最小可用的路径收益标签；
// 目的：先输出相对入场基准的最大有利变动，后续如需扩展盘中 runup 再单独版本化。
fn compute_max_runup(entry_price: f64, future_rows: &[StockHistoryRow]) -> f64 {
    let mut max_runup = 0.0_f64;

    for row in future_rows {
        let runup = row.adj_close / entry_price - 1.0;
        if runup > max_runup {
            max_runup = runup;
        }
    }

    max_runup
}

// 2026-04-09 CST: 这里统一判断止盈/止损先触发事件，原因是 Task 3 目标包含事件头标签，后续训练需要稳定的一阶事件口径；
// 目的：先按日线 high/low 顺序近似判断“先到目标还是先到止损”，若同日同时触发则保守记为都不成立，避免引入伪确定性。
fn compute_event_hits(
    entry_price: f64,
    future_rows: &[StockHistoryRow],
    stop_loss_pct: f64,
    target_return_pct: f64,
) -> (bool, bool) {
    let upside_price = entry_price * (1.0 + target_return_pct.max(0.0));
    let stop_price = entry_price * (1.0 - stop_loss_pct.max(0.0));

    for row in future_rows {
        let hit_upside = row.high >= upside_price;
        let hit_stop = row.low <= stop_price;

        if hit_upside && hit_stop {
            return (false, false);
        }
        if hit_upside {
            return (true, false);
        }
        if hit_stop {
            return (false, true);
        }
    }

    (false, false)
}

// 2026-04-09 CST: 这里统一规范 horizons，原因是 Tool 允许外层传入自定义期限，但训练与回算主链需要有去重、排序后的稳定顺序；
// 目的：防止重复 horizon 或 0 日窗口污染正式标签输出。
fn normalize_horizons(horizons: &[usize]) -> Vec<usize> {
    let mut normalized = horizons
        .iter()
        .copied()
        .filter(|value| *value > 0)
        .collect::<Vec<_>>();
    normalized.sort_unstable();
    normalized.dedup();
    normalized
}

fn default_lookback_days() -> usize {
    260
}

fn default_disclosure_limit() -> usize {
    8
}

fn default_horizons() -> Vec<usize> {
    vec![5, 10, 20, 30, 60, 180]
}

fn default_stop_loss_pct() -> f64 {
    0.05
}

fn default_target_return_pct() -> f64 {
    0.12
}

fn default_label_definition_version() -> String {
    "security_forward_outcome.v1".to_string()
}
