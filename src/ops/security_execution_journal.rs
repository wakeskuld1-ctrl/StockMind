use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_forward_outcome::{
    SecurityForwardOutcomeDocument, SecurityForwardOutcomeError, SecurityForwardOutcomeRequest,
    security_forward_outcome,
};
use crate::ops::stock::security_position_plan::{
    SecurityPositionPlanError, SecurityPositionPlanRequest, SecurityPositionPlanResult,
    security_position_plan,
};

// 2026-04-09 CST: 这里新增多笔成交输入合同，原因是 P1 要把“单次进出记录”升级成“正式成交 journal”；
// 目的：让后续 execution_record、review 和 package 都能基于结构化成交序列聚合，而不是继续把分批成交塞进备注文本。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityExecutionTradeInput {
    pub trade_date: String,
    pub side: String,
    pub price: f64,
    pub position_pct_delta: f64,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

// 2026-04-09 CST: 这里新增 execution journal 请求合同，原因是 P1 需要把多笔成交正式对象化为独立 Tool；
// 目的：让 CLI / Skill / review / package 可以直接消费统一 journal，而不是靠外层自己拼成交序列。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityExecutionJournalRequest {
    pub symbol: String,
    #[serde(default)]
    pub market_symbol: Option<String>,
    #[serde(default)]
    pub sector_symbol: Option<String>,
    pub market_regime: String,
    pub sector_template: String,
    #[serde(default)]
    pub market_profile: Option<String>,
    #[serde(default)]
    pub sector_profile: Option<String>,
    #[serde(default)]
    pub as_of_date: Option<String>,
    #[serde(default = "default_review_horizon_days")]
    pub review_horizon_days: usize,
    #[serde(default = "default_lookback_days")]
    pub lookback_days: usize,
    #[serde(default = "default_factor_lookback_days")]
    pub factor_lookback_days: usize,
    #[serde(default = "default_disclosure_limit")]
    pub disclosure_limit: usize,
    #[serde(default = "default_stop_loss_pct")]
    pub stop_loss_pct: f64,
    #[serde(default = "default_target_return_pct")]
    pub target_return_pct: f64,
    pub execution_trades: Vec<SecurityExecutionTradeInput>,
    #[serde(default)]
    pub execution_journal_notes: Vec<String>,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-09 CST: 这里固化 journal 内部成交条目，原因是多笔成交不能只留输入态；
// 目的：把每笔成交执行后的位置变化一并写入正式文档，方便后续审计、复盘和引用一致性校验。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityExecutionJournalTrade {
    pub trade_id: String,
    pub trade_date: String,
    pub side: String,
    pub price: f64,
    pub position_pct_delta: f64,
    pub resulting_position_pct: f64,
    pub reason: String,
    pub notes: Vec<String>,
}

// 2026-04-09 CST: 这里固化 execution journal 正式文档，原因是 P1 目标是“先有正式成交对象，再由它聚合 record”；
// 目的：让平台既保留明细级成交事实，也保留聚合后的执行摘要，而不是两者互相替代。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityExecutionJournalDocument {
    pub execution_journal_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub symbol: String,
    pub analysis_date: String,
    // 2026-04-10 CST: 这里新增持仓状态字段，原因是方案A要把 execution_journal 从“只支持已平仓”补到“支持连续持仓快照”；
    // 目的：让下游 execution_record / review 能显式识别当前是已平仓结果还是仍在持仓中的阶段性快照。
    pub position_state: String,
    pub position_plan_ref: String,
    pub snapshot_ref: String,
    pub outcome_ref: String,
    pub trades: Vec<SecurityExecutionJournalTrade>,
    pub trade_count: usize,
    pub entry_trade_count: usize,
    pub exit_trade_count: usize,
    pub holding_start_date: String,
    pub holding_end_date: String,
    pub peak_position_pct: f64,
    pub final_position_pct: f64,
    pub weighted_entry_price: f64,
    pub weighted_exit_price: f64,
    pub realized_return: f64,
    pub execution_journal_notes: Vec<String>,
    pub aggregation_summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityExecutionJournalResult {
    pub position_plan_result: SecurityPositionPlanResult,
    pub forward_outcome_result: SecurityExecutionJournalOutcomeBinding,
    pub execution_journal: SecurityExecutionJournalDocument,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityExecutionJournalOutcomeBinding {
    pub snapshot: crate::ops::stock::security_feature_snapshot::SecurityFeatureSnapshot,
    pub selected_outcome: SecurityForwardOutcomeDocument,
    pub all_outcomes: Vec<SecurityForwardOutcomeDocument>,
}

#[derive(Debug, Error)]
pub enum SecurityExecutionJournalError {
    #[error("security execution journal position preparation failed: {0}")]
    PositionPlan(#[from] SecurityPositionPlanError),
    #[error("security execution journal forward outcome preparation failed: {0}")]
    ForwardOutcome(#[from] SecurityForwardOutcomeError),
    #[error("security execution journal build failed: {0}")]
    Build(String),
}

pub fn security_execution_journal(
    request: &SecurityExecutionJournalRequest,
) -> Result<SecurityExecutionJournalResult, SecurityExecutionJournalError> {
    let position_plan_result = security_position_plan(&SecurityPositionPlanRequest {
        symbol: request.symbol.clone(),
        market_symbol: request.market_symbol.clone(),
        sector_symbol: request.sector_symbol.clone(),
        market_regime: request.market_regime.clone(),
        sector_template: request.sector_template.clone(),
        as_of_date: request.as_of_date.clone(),
        lookback_days: request.lookback_days,
        factor_lookback_days: request.factor_lookback_days,
        disclosure_limit: request.disclosure_limit,
        created_at: request.created_at.clone(),
    })?;
    let forward_outcome_result = security_forward_outcome(&SecurityForwardOutcomeRequest {
        symbol: request.symbol.clone(),
        market_symbol: request.market_symbol.clone(),
        sector_symbol: request.sector_symbol.clone(),
        market_profile: request.market_profile.clone(),
        sector_profile: request.sector_profile.clone(),
        as_of_date: request.as_of_date.clone(),
        lookback_days: request.lookback_days,
        disclosure_limit: request.disclosure_limit,
        horizons: vec![request.review_horizon_days],
        stop_loss_pct: request.stop_loss_pct,
        target_return_pct: request.target_return_pct,
        label_definition_version: "security_forward_outcome.v1".to_string(),
        // 2026-04-14 CST: 这里补齐新合同字段默认值，原因是 execution_journal 当前并不直接处理 external proxy；
        // 目的：先保持执行链与 forward_outcome 合同兼容，不在这一轮扩大执行请求面。
        external_proxy_inputs: None,
    })?;
    let selected_outcome = forward_outcome_result
        .forward_outcomes
        .iter()
        .find(|item| item.horizon_days == request.review_horizon_days)
        .cloned()
        .ok_or_else(|| {
            SecurityExecutionJournalError::Build(format!(
                "missing selected outcome for horizon {}",
                request.review_horizon_days
            ))
        })?;
    let outcome_binding = SecurityExecutionJournalOutcomeBinding {
        snapshot: forward_outcome_result.snapshot.clone(),
        selected_outcome,
        all_outcomes: forward_outcome_result.forward_outcomes.clone(),
    };
    let execution_journal =
        build_security_execution_journal(&position_plan_result, &outcome_binding, request)?;

    Ok(SecurityExecutionJournalResult {
        position_plan_result,
        forward_outcome_result: outcome_binding,
        execution_journal,
    })
}

// 2026-04-09 CST: 这里集中 journal 聚合规则，原因是后续 execution_record / review / audit 都会复用这份事实对象；
// 目的：把“多笔成交 -> 持仓轨迹 -> 加权价格 -> 已实现收益”的口径固化在一个地方，避免多个 Tool 各自拼账。
pub fn build_security_execution_journal(
    position_plan_result: &SecurityPositionPlanResult,
    outcome_binding: &SecurityExecutionJournalOutcomeBinding,
    request: &SecurityExecutionJournalRequest,
) -> Result<SecurityExecutionJournalDocument, SecurityExecutionJournalError> {
    if request.execution_trades.is_empty() {
        return Err(SecurityExecutionJournalError::Build(
            "execution_trades must not be empty".to_string(),
        ));
    }

    let planned_date = parse_date(&outcome_binding.snapshot.as_of_date, "planned_entry_date")?;
    let mut trades = Vec::new();
    let mut resulting_position_pct = 0.0_f64;
    let mut peak_position_pct = 0.0_f64;
    let mut buy_notional = 0.0_f64;
    let mut buy_qty = 0.0_f64;
    let mut sell_notional = 0.0_f64;
    let mut sell_qty = 0.0_f64;
    let mut previous_trade_date: Option<NaiveDate> = None;

    for (index, trade) in request.execution_trades.iter().enumerate() {
        if trade.price <= 0.0 {
            return Err(SecurityExecutionJournalError::Build(format!(
                "execution_trades[{index}].price must be positive"
            )));
        }
        if trade.position_pct_delta <= 0.0 || trade.position_pct_delta > 1.0 {
            return Err(SecurityExecutionJournalError::Build(format!(
                "execution_trades[{index}].position_pct_delta must be within (0, 1]"
            )));
        }

        let trade_date = parse_date(&trade.trade_date, "trade_date")?;
        if trade_date < planned_date {
            return Err(SecurityExecutionJournalError::Build(format!(
                "execution_trades[{index}].trade_date {} must be on or after planned_entry_date {}",
                trade.trade_date, outcome_binding.snapshot.as_of_date
            )));
        }
        if let Some(previous_trade_date) = previous_trade_date {
            if trade_date < previous_trade_date {
                return Err(SecurityExecutionJournalError::Build(
                    "execution_trades must be sorted by trade_date ascending".to_string(),
                ));
            }
        }
        previous_trade_date = Some(trade_date);

        match trade.side.trim() {
            "buy" => {
                buy_notional += trade.price * trade.position_pct_delta;
                buy_qty += trade.position_pct_delta;
                resulting_position_pct += trade.position_pct_delta;
                peak_position_pct = peak_position_pct.max(resulting_position_pct);
            }
            "sell" => {
                sell_notional += trade.price * trade.position_pct_delta;
                sell_qty += trade.position_pct_delta;
                resulting_position_pct -= trade.position_pct_delta;
                if resulting_position_pct < -1e-9 {
                    return Err(SecurityExecutionJournalError::Build(format!(
                        "execution_trades[{index}] would make position negative"
                    )));
                }
            }
            other => {
                return Err(SecurityExecutionJournalError::Build(format!(
                    "execution_trades[{index}].side must be `buy` or `sell`, got `{other}`"
                )));
            }
        }

        if resulting_position_pct > 1.0 + 1e-9 {
            return Err(SecurityExecutionJournalError::Build(format!(
                "execution_trades[{index}] would make position exceed 1.0"
            )));
        }

        trades.push(SecurityExecutionJournalTrade {
            trade_id: format!(
                "execution-trade-{}-{}",
                position_plan_result.position_plan_document.position_plan_id,
                index + 1
            ),
            trade_date: trade.trade_date.trim().to_string(),
            side: trade.side.trim().to_string(),
            price: trade.price,
            position_pct_delta: trade.position_pct_delta,
            resulting_position_pct: rounded_pct(resulting_position_pct),
            reason: trade.reason.clone().unwrap_or_default().trim().to_string(),
            notes: normalize_lines(&trade.notes),
        });
    }

    if buy_qty <= 0.0 {
        return Err(SecurityExecutionJournalError::Build(
            "execution_trades must include at least one buy".to_string(),
        ));
    }

    let holding_start_date = trades
        .first()
        .map(|item| item.trade_date.clone())
        .unwrap_or_default();
    let holding_end_date = trades
        .last()
        .map(|item| item.trade_date.clone())
        .unwrap_or_default();
    let weighted_entry_price = buy_notional / buy_qty;
    // 2026-04-10 CST: 这里把卖出均价改成按“是否已有退出成交”分支计算，原因是未平仓快照不再保证存在卖出腿；
    // 目的：让持仓中阶段也能形成正式 journal，而不是因为缺少退出价就无法落对象。
    let weighted_exit_price = if sell_qty > 1e-9 {
        sell_notional / sell_qty
    } else {
        0.0
    };
    // 2026-04-10 CST: 这里把已实现收益限定在存在卖出成交时才计算，原因是未平仓状态下只能先沉淀已实现部分；
    // 目的：避免把“尚未退出”的阶段误写成完整闭环收益。
    let realized_return = if sell_qty > 1e-9 {
        weighted_exit_price / weighted_entry_price - 1.0
    } else {
        0.0
    };
    let final_position_pct = rounded_pct(resulting_position_pct.max(0.0));
    let position_state = if final_position_pct <= 1e-9 {
        "flat".to_string()
    } else {
        "open".to_string()
    };
    let execution_journal_notes = normalize_lines(&request.execution_journal_notes);

    Ok(SecurityExecutionJournalDocument {
        execution_journal_id: format!(
            "execution-journal-{}-{}",
            position_plan_result.position_plan_document.position_plan_id, holding_start_date
        ),
        contract_version: "security_execution_journal.v1".to_string(),
        document_type: "security_execution_journal".to_string(),
        generated_at: normalize_created_at(&request.created_at),
        symbol: position_plan_result.position_plan_document.symbol.clone(),
        analysis_date: position_plan_result
            .position_plan_document
            .analysis_date
            .clone(),
        position_state: position_state.clone(),
        position_plan_ref: position_plan_result
            .position_plan_document
            .position_plan_id
            .clone(),
        snapshot_ref: outcome_binding.snapshot.snapshot_id.clone(),
        outcome_ref: outcome_binding.selected_outcome.outcome_id.clone(),
        trades,
        trade_count: request.execution_trades.len(),
        entry_trade_count: request
            .execution_trades
            .iter()
            .filter(|item| item.side == "buy")
            .count(),
        exit_trade_count: request
            .execution_trades
            .iter()
            .filter(|item| item.side == "sell")
            .count(),
        holding_start_date,
        holding_end_date,
        peak_position_pct: rounded_pct(peak_position_pct),
        final_position_pct,
        weighted_entry_price,
        weighted_exit_price,
        realized_return,
        execution_journal_notes,
        aggregation_summary: format!(
            "共记录 {} 笔成交，峰值仓位 {:.2}%，加权买入价 {:.2}，加权卖出价 {:.2}，已实现收益 {:.2}%。",
            request.execution_trades.len(),
            peak_position_pct * 100.0,
            weighted_entry_price,
            weighted_exit_price,
            realized_return * 100.0
        ),
    })
}

fn parse_date(value: &str, field_name: &str) -> Result<NaiveDate, SecurityExecutionJournalError> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").map_err(|error| {
        SecurityExecutionJournalError::Build(format!(
            "{field_name} must be YYYY-MM-DD, got `{value}`: {error}"
        ))
    })
}

fn normalize_lines(items: &[String]) -> Vec<String> {
    items
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(|item| item.to_string())
        .collect()
}

fn rounded_pct(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

fn normalize_created_at(value: &str) -> String {
    if value.trim().is_empty() {
        Utc::now().to_rfc3339()
    } else {
        value.trim().to_string()
    }
}

fn default_created_at() -> String {
    Utc::now().to_rfc3339()
}

fn default_review_horizon_days() -> usize {
    20
}

fn default_lookback_days() -> usize {
    180
}

fn default_factor_lookback_days() -> usize {
    120
}

fn default_disclosure_limit() -> usize {
    6
}

fn default_stop_loss_pct() -> f64 {
    0.05
}

fn default_target_return_pct() -> f64 {
    0.12
}
