use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_account_open_position_snapshot::SecurityAccountOpenPositionSnapshotDocument;
use crate::ops::stock::security_per_position_evaluation::SecurityPerPositionEvaluation;
use crate::ops::stock::security_position_contract::SecurityPositionContract;
use crate::ops::stock::security_position_plan::SecurityPositionPlanDocument;

// 2026-04-09 CST: 这里新增账户当前持仓输入合同，原因是账户级仓位管理必须先知道“现有仓位占了多少”；
// 目的：把已有持仓、行业暴露和已投入资金收成正式输入，而不是让外层继续临时口算。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PortfolioHoldingInput {
    pub symbol: String,
    pub market_value: f64,
    #[serde(default)]
    pub sector_tag: Option<String>,
}

// 2026-04-10 CST: 这里新增未平仓快照输入合同，原因是上一轮 execution_record 已经能沉淀 open snapshot，
// 目的：把“当前仍在持仓的正式快照”直接映射成账户层 holdings 输入，减少外层手工回填。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PortfolioOpenPositionSnapshotInput {
    pub symbol: String,
    pub position_state: String,
    pub current_position_pct: f64,
    // 2026-04-16 CST: Added because P0-1 starts carrying formal holding-economics fields
    // out of the runtime-rebuilt open-position snapshot.
    // Purpose: let downstream layers reuse resolved price and breakeven facts without
    // re-querying runtime stores or recomputing dividend math ad hoc.
    #[serde(default)]
    pub price_as_of_date: Option<String>,
    #[serde(default)]
    pub resolved_trade_date: Option<String>,
    #[serde(default)]
    pub current_price: Option<f64>,
    #[serde(default)]
    pub share_adjustment_factor: Option<f64>,
    #[serde(default)]
    pub cumulative_cash_dividend_per_share: Option<f64>,
    #[serde(default)]
    pub dividend_adjusted_cost_basis: Option<f64>,
    #[serde(default)]
    pub holding_total_return_pct: Option<f64>,
    #[serde(default)]
    pub breakeven_price: Option<f64>,
    #[serde(default)]
    pub corporate_action_summary: Option<String>,
    #[serde(default)]
    pub sector_tag: Option<String>,
    #[serde(default)]
    pub source_execution_record_ref: Option<String>,
}

// 2026-04-09 CST: 这里新增候选标的的输入合同，原因是账户级仓位建议需要消费单标的 position_plan 结果；
// 目的：复用现有正式 `security_position_plan` 文档，避免在账户层重复造第二套单标的规则。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PortfolioCandidateInput {
    pub symbol: String,
    #[serde(default)]
    pub sector_tag: Option<String>,
    pub position_plan_document: SecurityPositionPlanDocument,
}

// 2026-04-09 CST: 这里新增账户级仓位规划请求合同，原因是方案A要求把“增量资金怎么投”正式对象化；
// 目的：让 CLI / Skill / 后续界面都围绕同一份账户输入边界生成统一建议。
// 2026-04-09 CST: 这里补入账户风险预算字段，原因是 Task 12 之后的第一优先级就是“账户还能承受多少新增风险”；
// 目的：把现金门禁继续升级成“现金 + 风险预算”双重门禁，避免只有仓位没有风险约束。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioPositionPlanRequest {
    pub account_id: String,
    pub total_equity: f64,
    pub available_cash: f64,
    #[serde(default = "default_min_cash_reserve_pct")]
    pub min_cash_reserve_pct: f64,
    #[serde(default = "default_max_single_position_pct")]
    pub max_single_position_pct: f64,
    #[serde(default = "default_max_sector_exposure_pct")]
    pub max_sector_exposure_pct: f64,
    #[serde(default = "default_max_portfolio_risk_budget_pct")]
    pub max_portfolio_risk_budget_pct: f64,
    #[serde(default)]
    pub current_portfolio_risk_budget_pct: f64,
    #[serde(default = "default_max_single_trade_risk_budget_pct")]
    pub max_single_trade_risk_budget_pct: f64,
    #[serde(default)]
    pub holdings: Vec<PortfolioHoldingInput>,
    #[serde(default)]
    pub account_open_position_snapshot_document:
        Option<SecurityAccountOpenPositionSnapshotDocument>,
    // 2026-04-10 CST: 这里补账户层 open snapshot 入口，原因是方案A当前要补的是“连续状态正式回接”，
    // 目的：允许下一轮账户计划直接消费未平仓快照，而不是要求调用方先人工折算 market_value。
    #[serde(default)]
    pub open_position_snapshots: Vec<PortfolioOpenPositionSnapshotInput>,
    pub candidates: Vec<PortfolioCandidateInput>,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-09 CST: 这里固化单标的的账户级建议项，原因是账户层输出不能只给一行摘要；
// 目的：明确写出每只标的的当前占比、目标占比、建议交易额和约束命中情况，方便后续实用决策。
// 2026-04-09 CST: 这里新增逐标的风险预算占用，原因是账户级风控不应只在汇总层存在；
// 目的：让每条建议都能解释“这笔动作会吃掉多少风险预算”，方便投前解释和投后复盘。
// 2026-04-09 CST: 这里新增分层执行建议，原因是方案A-1要把“当前建议走哪一层”正式暴露给账户层调用方；
// 目的：让上层不用再从 starter/max 和当前持仓自行推断当前是试仓、加仓还是等待。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PortfolioAllocationRecommendation {
    pub symbol: String,
    pub action: String,
    pub sector_tag: String,
    pub current_position_pct: f64,
    pub target_position_pct: f64,
    pub incremental_position_pct: f64,
    pub recommended_trade_amount: f64,
    pub estimated_risk_budget_pct: f64,
    pub suggested_tranche_action: String,
    pub suggested_tranche_pct: f64,
    pub remaining_tranche_count: usize,
    pub priority_score: i32,
    pub constraint_flags: Vec<String>,
    pub rationale: Vec<String>,
}

// 2026-04-09 CST: 这里固化账户级仓位规划文档，原因是方案A要补的是正式 Tool，而不是临时 helper；
// 目的：把现金底线、组合集中度和逐标的建议收成可留痕、可交接、可复盘的正式对象。
// 2026-04-09 CST: 这里补入风险预算汇总字段，原因是账户层现在不仅要回答“投给谁”，还要回答“风险预算还能不能投”；
// 目的：让账户层输出直接暴露总预算、当前占用、剩余预算和新增预算占用。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioPositionPlanDocument {
    pub portfolio_position_plan_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub account_id: String,
    pub total_equity: f64,
    pub available_cash: f64,
    pub current_cash_pct: f64,
    pub min_cash_reserve_pct: f64,
    pub deployable_cash_amount: f64,
    pub deployable_cash_pct: f64,
    pub current_invested_pct: f64,
    pub max_portfolio_risk_budget_pct: f64,
    pub current_portfolio_risk_budget_pct: f64,
    pub remaining_portfolio_risk_budget_pct: f64,
    pub max_single_trade_risk_budget_pct: f64,
    pub estimated_new_risk_budget_pct: f64,
    pub total_portfolio_risk_budget_pct: f64,
    pub concentration_warnings: Vec<String>,
    pub risk_budget_warnings: Vec<String>,
    pub allocations: Vec<PortfolioAllocationRecommendation>,
    pub portfolio_summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityPortfolioPositionPlanResult {
    pub portfolio_position_plan: SecurityPortfolioPositionPlanDocument,
}

// 2026-04-18 CST: Added because Task 5 needs one explicit account-level
// monitoring aggregation object built from per-position evaluations.
// Reason: the monitoring evidence package should reuse account math from this
// file instead of duplicating weight, return, and warning calculations.
// Purpose: define the reusable account-aggregation shape for Task 5 and later tasks.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityMonitoringAccountAggregation {
    pub active_position_count: usize,
    pub total_active_weight_pct: f64,
    pub weighted_expected_return_pct: f64,
    pub weighted_expected_drawdown_pct: f64,
    pub total_risk_budget_pct: f64,
    pub concentration_warnings: Vec<String>,
    pub correlation_warnings: Vec<String>,
    pub risk_budget_warnings: Vec<String>,
    pub aggregation_summary: String,
}

// 2026-04-18 CST: Added because Task 5 also needs compact ranked action
// candidates derived from the per-position evaluation layer.
// Reason: future committee review should read one normalized candidate shape
// for add/trim/replace/exit instead of interpreting score maps manually.
// Purpose: define the reusable candidate summary object for monitoring packages.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityMonitoringActionCandidate {
    pub symbol: String,
    pub score: f64,
    pub recommended_action: String,
    pub current_weight_pct: f64,
    pub target_weight_pct: f64,
    pub current_vs_target_gap_pct: f64,
    pub per_position_evaluation_ref: String,
}

// 2026-04-18 CST: Added because the monitoring evidence package should carry a
// normalized action-candidate section produced by reusable portfolio helpers.
// Reason: later tasks should extend this object instead of rebuilding four ranked lists ad hoc.
// Purpose: define the reusable action-simulation payload for Task 5.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityAdjustmentSimulationData {
    pub top_add_candidates: Vec<SecurityMonitoringActionCandidate>,
    pub top_trim_candidates: Vec<SecurityMonitoringActionCandidate>,
    pub top_replace_candidates: Vec<SecurityMonitoringActionCandidate>,
    pub top_exit_candidates: Vec<SecurityMonitoringActionCandidate>,
}

// 2026-04-19 CST: Added because Task 6 needs one reusable per-contract delta
// view when capital rebasing changes account baselines and live contract caps.
// Reason: the capital rebalance evidence package should reuse a stable simulation
// item shape instead of rebuilding before/after deltas inline.
// Purpose: define the reusable capital rebalance simulation row.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCapitalRebalanceSimulationItem {
    pub symbol: String,
    pub target_weight_pct_before: f64,
    pub target_weight_pct_after: f64,
    pub max_weight_pct_before: f64,
    pub max_weight_pct_after: f64,
    pub risk_budget_pct_before: f64,
    pub risk_budget_pct_after: f64,
    pub intended_principal_amount_before: f64,
    pub intended_principal_amount_after: f64,
    // 2026-04-19 CST: Added serde default because Task 7 committee-package
    // consumers must be able to deserialize earlier capital-rebalance evidence
    // samples that were created before the richer simulation payload landed.
    // Reason: the governance handoff should accept the existing evidence shape
    // instead of failing before it can validate committee-level boundaries.
    // Purpose: keep backward-compatible evidence parsing while newer builders
    // still emit the richer fields explicitly.
    #[serde(default)]
    pub principal_delta_amount: f64,
    #[serde(default)]
    pub simulation_action_hint: String,
    #[serde(default)]
    pub position_contract_ref: String,
}

#[derive(Debug, Error)]
pub enum SecurityPortfolioPositionPlanError {
    #[error("security portfolio position plan build failed: {0}")]
    Build(String),
}

pub fn security_portfolio_position_plan(
    request: &SecurityPortfolioPositionPlanRequest,
) -> Result<SecurityPortfolioPositionPlanResult, SecurityPortfolioPositionPlanError> {
    validate_request(request)?;
    let portfolio_position_plan = build_portfolio_position_plan(request)?;
    Ok(SecurityPortfolioPositionPlanResult {
        portfolio_position_plan,
    })
}

// 2026-04-09 CST: 这里集中账户级仓位规则，原因是这轮先做“最实用”的增量配置建议；
// 目的：用现金底线、单票上限、行业上限和风险预算门禁，回答“这笔钱现在该先投给谁、投多少”。
pub fn build_portfolio_position_plan(
    request: &SecurityPortfolioPositionPlanRequest,
) -> Result<SecurityPortfolioPositionPlanDocument, SecurityPortfolioPositionPlanError> {
    let current_cash_pct = request.available_cash / request.total_equity;
    let deployable_cash_amount =
        (request.available_cash - request.total_equity * request.min_cash_reserve_pct).max(0.0);
    let mut remaining_deployable_pct = deployable_cash_amount / request.total_equity;
    // 2026-04-09 CST: 这里新增账户剩余风险预算计算，原因是方案A当前要把风险预算门禁并入现有账户级分配逻辑；
    // 目的：让每只候选同时受现金预算和风险预算双重约束，而不是只看现金和行业上限。
    let initial_remaining_portfolio_risk_budget_pct = (request.max_portfolio_risk_budget_pct
        - request.current_portfolio_risk_budget_pct)
        .max(0.0);
    let mut remaining_portfolio_risk_budget_pct = initial_remaining_portfolio_risk_budget_pct;
    let mut estimated_new_risk_budget_pct = 0.0;

    let mut current_sector_exposure = std::collections::BTreeMap::<String, f64>::new();
    let mut current_symbol_exposure = std::collections::BTreeMap::<String, f64>::new();
    for holding in effective_holdings(request) {
        let exposure_pct = holding.market_value / request.total_equity;
        *current_symbol_exposure.entry(holding.symbol).or_insert(0.0) += exposure_pct;
        let sector_tag = normalized_sector_tag(holding.sector_tag.as_deref());
        *current_sector_exposure.entry(sector_tag).or_insert(0.0) += exposure_pct;
    }

    let mut candidates = request.candidates.clone();
    candidates.sort_by(|left, right| {
        candidate_priority_score(right).cmp(&candidate_priority_score(left))
    });

    let mut concentration_warnings = Vec::new();
    let mut risk_budget_warnings = Vec::new();
    let current_invested_pct =
        ((request.total_equity - request.available_cash) / request.total_equity).max(0.0);
    if current_cash_pct < request.min_cash_reserve_pct {
        concentration_warnings
            .push("当前现金占比已经低于安全底线，新开仓需要更严格控制。".to_string());
    }
    if initial_remaining_portfolio_risk_budget_pct <= 1e-9 {
        risk_budget_warnings
            .push("当前账户风险预算已用满，新增仓位建议应等待预算释放。".to_string());
    }

    let mut allocations = Vec::new();
    for candidate in candidates {
        let plan = &candidate.position_plan_document;
        let symbol = candidate.symbol.clone();
        let sector_tag = normalized_sector_tag(candidate.sector_tag.as_deref());
        let current_position_pct = current_symbol_exposure.get(&symbol).copied().unwrap_or(0.0);
        let current_sector_pct = current_sector_exposure
            .get(&sector_tag)
            .copied()
            .unwrap_or(0.0);
        let risk_cap = risk_cap_pct(&plan.position_risk_grade);
        let base_target_pct = plan
            .max_position_pct
            .min(request.max_single_position_pct)
            .min(risk_cap);
        let sector_headroom_pct = (request.max_sector_exposure_pct - current_sector_pct).max(0.0);
        let position_headroom_pct = (base_target_pct - current_position_pct).max(0.0);
        let deployable_position_pct = remaining_deployable_pct.max(0.0);
        let risk_budget_unit_cost_pct = risk_budget_unit_cost_pct(&plan.position_risk_grade);
        let risk_headroom_from_account_pct =
            (remaining_portfolio_risk_budget_pct / risk_budget_unit_cost_pct).max(0.0);
        let risk_headroom_from_single_trade_pct =
            (request.max_single_trade_risk_budget_pct / risk_budget_unit_cost_pct).max(0.0);
        let risk_budget_limited_position_pct =
            risk_headroom_from_account_pct.min(risk_headroom_from_single_trade_pct);

        let mut constraint_flags = Vec::new();
        if plan.position_action == "wait" || base_target_pct <= 0.0 {
            constraint_flags.push("position_plan_requires_wait".to_string());
        }
        if sector_headroom_pct <= 0.0 {
            constraint_flags.push("sector_limit_reached".to_string());
        }
        if deployable_position_pct <= 0.0 {
            constraint_flags.push("cash_floor_reached".to_string());
        }
        if remaining_portfolio_risk_budget_pct <= 1e-9 {
            constraint_flags.push("portfolio_risk_budget_reached".to_string());
        }
        if request.max_single_trade_risk_budget_pct <= 1e-9 {
            constraint_flags.push("single_trade_risk_budget_reached".to_string());
        }
        if current_position_pct > base_target_pct + 1e-9 {
            constraint_flags.push("single_name_overweight".to_string());
        }

        let desired_increment_pct = recommended_add_tranche_pct(plan).min(position_headroom_pct);
        let allowed_increment_pct = desired_increment_pct
            .min(sector_headroom_pct)
            .min(deployable_position_pct)
            .min(risk_budget_limited_position_pct);
        let target_position_pct = if current_position_pct > base_target_pct + 1e-9 {
            base_target_pct
        } else {
            current_position_pct + allowed_increment_pct
        };
        let incremental_position_pct = target_position_pct - current_position_pct;
        let recommended_trade_amount = request.total_equity * incremental_position_pct;
        let estimated_risk_budget_pct =
            incremental_position_pct.max(0.0) * risk_budget_unit_cost_pct;
        let action = if incremental_position_pct > 1e-9 {
            if current_position_pct > 0.0 {
                "add"
            } else {
                "buy"
            }
        } else if incremental_position_pct < -1e-9 {
            "trim"
        } else {
            "hold"
        };
        let suggested_tranche_action = if incremental_position_pct > 1e-9 {
            if current_position_pct > 1e-9 {
                "add_tranche".to_string()
            } else {
                "entry_tranche".to_string()
            }
        } else if incremental_position_pct < -1e-9 {
            "reduce_tranche".to_string()
        } else {
            "hold".to_string()
        };
        let suggested_tranche_pct = incremental_position_pct.max(0.0);
        let tranche_capacity_after_trade = tranche_count_after_trade(target_position_pct, plan);
        let remaining_tranche_count =
            derived_max_tranche_count(plan).saturating_sub(tranche_capacity_after_trade);

        let mut rationale = vec![
            format!(
                "单票目标上限取 min(max_position_pct {:.2}%, 全局单票上限 {:.2}%, 风险等级上限 {:.2}%)。",
                plan.max_position_pct * 100.0,
                request.max_single_position_pct * 100.0,
                risk_cap * 100.0
            ),
            format!(
                "当前行业暴露 {:.2}%，行业上限 {:.2}%。",
                current_sector_pct * 100.0,
                request.max_sector_exposure_pct * 100.0
            ),
            format!(
                "当前账户剩余风险预算 {:.2}%，单笔风险预算上限 {:.2}%。",
                remaining_portfolio_risk_budget_pct * 100.0,
                request.max_single_trade_risk_budget_pct * 100.0
            ),
            format!(
                "当前分层模板为 `{}`，剩余可用层数 {}。",
                normalized_tranche_template(plan),
                remaining_tranche_count
            ),
        ];
        if incremental_position_pct > 1e-9 {
            rationale.push(format!(
                "当前可部署现金 {:.2}%，本次建议新增 {:.2}%。",
                deployable_position_pct * 100.0,
                incremental_position_pct * 100.0
            ));
            rationale.push(format!(
                "按风险等级折算后，本次预计占用风险预算 {:.2}%。",
                estimated_risk_budget_pct * 100.0
            ));
        } else if action == "trim" {
            rationale.push("当前持仓已经超过账户级单票约束，应优先降回目标上限。".to_string());
        } else {
            rationale.push("当前约束条件下不建议继续增加该标的的暴露。".to_string());
        }

        if incremental_position_pct > 0.0 {
            remaining_deployable_pct =
                (remaining_deployable_pct - incremental_position_pct).max(0.0);
            remaining_portfolio_risk_budget_pct =
                (remaining_portfolio_risk_budget_pct - estimated_risk_budget_pct).max(0.0);
            estimated_new_risk_budget_pct += estimated_risk_budget_pct;
            *current_sector_exposure
                .entry(sector_tag.clone())
                .or_insert(0.0) += incremental_position_pct;
            current_symbol_exposure.insert(symbol.clone(), target_position_pct);
        }

        allocations.push(PortfolioAllocationRecommendation {
            symbol,
            action: action.to_string(),
            sector_tag,
            current_position_pct,
            target_position_pct,
            incremental_position_pct,
            recommended_trade_amount,
            estimated_risk_budget_pct,
            suggested_tranche_action,
            suggested_tranche_pct,
            remaining_tranche_count,
            priority_score: candidate_priority_score(&candidate),
            constraint_flags,
            rationale,
        });
    }

    let buy_count = allocations
        .iter()
        .filter(|item| item.action == "buy" || item.action == "add")
        .count();
    let trim_count = allocations
        .iter()
        .filter(|item| item.action == "trim")
        .count();
    Ok(SecurityPortfolioPositionPlanDocument {
        portfolio_position_plan_id: format!(
            "portfolio-position-plan-{}-{}",
            request.account_id,
            request.created_at.replace(':', "-")
        ),
        contract_version: "security_portfolio_position_plan.v1".to_string(),
        document_type: "security_portfolio_position_plan".to_string(),
        generated_at: normalize_created_at(&request.created_at),
        account_id: request.account_id.clone(),
        total_equity: request.total_equity,
        available_cash: request.available_cash,
        current_cash_pct,
        min_cash_reserve_pct: request.min_cash_reserve_pct,
        deployable_cash_amount,
        deployable_cash_pct: deployable_cash_amount / request.total_equity,
        current_invested_pct,
        max_portfolio_risk_budget_pct: request.max_portfolio_risk_budget_pct,
        current_portfolio_risk_budget_pct: request.current_portfolio_risk_budget_pct,
        remaining_portfolio_risk_budget_pct: initial_remaining_portfolio_risk_budget_pct,
        max_single_trade_risk_budget_pct: request.max_single_trade_risk_budget_pct,
        estimated_new_risk_budget_pct,
        total_portfolio_risk_budget_pct: request.current_portfolio_risk_budget_pct
            + estimated_new_risk_budget_pct,
        concentration_warnings,
        risk_budget_warnings,
        allocations,
        portfolio_summary: format!(
            "账户当前现金占比 {:.2}%，可部署现金 {:.2}%，风险预算总占用 {:.2}%，本轮建议新增/加仓 {} 个标的，降仓 {} 个标的。",
            current_cash_pct * 100.0,
            deployable_cash_amount / request.total_equity * 100.0,
            (request.current_portfolio_risk_budget_pct + estimated_new_risk_budget_pct) * 100.0,
            buy_count,
            trim_count
        ),
    })
}

fn validate_request(
    request: &SecurityPortfolioPositionPlanRequest,
) -> Result<(), SecurityPortfolioPositionPlanError> {
    if request.account_id.trim().is_empty() {
        return Err(SecurityPortfolioPositionPlanError::Build(
            "account_id must not be empty".to_string(),
        ));
    }
    if request.total_equity <= 0.0 {
        return Err(SecurityPortfolioPositionPlanError::Build(
            "total_equity must be positive".to_string(),
        ));
    }
    if request.available_cash < 0.0 || request.available_cash > request.total_equity {
        return Err(SecurityPortfolioPositionPlanError::Build(
            "available_cash must be within [0, total_equity]".to_string(),
        ));
    }
    if request.max_portfolio_risk_budget_pct < 0.0 || request.max_portfolio_risk_budget_pct > 1.0 {
        return Err(SecurityPortfolioPositionPlanError::Build(
            "max_portfolio_risk_budget_pct must be within [0, 1]".to_string(),
        ));
    }
    if request.current_portfolio_risk_budget_pct < 0.0 {
        return Err(SecurityPortfolioPositionPlanError::Build(
            "current_portfolio_risk_budget_pct must be non-negative".to_string(),
        ));
    }
    if request.max_single_trade_risk_budget_pct < 0.0
        || request.max_single_trade_risk_budget_pct > 1.0
    {
        return Err(SecurityPortfolioPositionPlanError::Build(
            "max_single_trade_risk_budget_pct must be within [0, 1]".to_string(),
        ));
    }
    if request.candidates.is_empty() {
        return Err(SecurityPortfolioPositionPlanError::Build(
            "candidates must not be empty".to_string(),
        ));
    }
    // 2026-04-10 CST: 这里校验 open snapshot 百分比，原因是它现在会直接进入账户暴露计算，
    // 目的：避免非法快照把当前仓位、行业暴露和后续 trade amount 全部带歪。
    for (index, snapshot) in request.open_position_snapshots.iter().enumerate() {
        if snapshot.current_position_pct < 0.0 || snapshot.current_position_pct > 1.0 {
            return Err(SecurityPortfolioPositionPlanError::Build(format!(
                "open_position_snapshots[{index}].current_position_pct must be within [0, 1]"
            )));
        }
    }
    Ok(())
}

// 2026-04-10 CST: 这里集中把 open snapshot 折算成账户 holdings，原因是账户层不应该把“手工 holdings”和“正式快照 holdings”分两套口径；
// 目的：让后续所有暴露计算都统一消费同一份 effective holdings，减少漏算和重复算。
fn effective_holdings(
    request: &SecurityPortfolioPositionPlanRequest,
) -> Vec<PortfolioHoldingInput> {
    let mut holdings = request.holdings.clone();
    let snapshot_inputs = if request.open_position_snapshots.is_empty() {
        request
            .account_open_position_snapshot_document
            .as_ref()
            .map(|document| document.open_position_snapshots.clone())
            .unwrap_or_default()
    } else {
        request.open_position_snapshots.clone()
    };
    holdings.extend(
        snapshot_inputs
            .iter()
            .filter(|snapshot| snapshot.position_state.trim() == "open")
            .filter(|snapshot| snapshot.current_position_pct > 1e-9)
            .map(|snapshot| PortfolioHoldingInput {
                symbol: snapshot.symbol.clone(),
                market_value: request.total_equity * snapshot.current_position_pct,
                sector_tag: snapshot.sector_tag.clone(),
            }),
    );
    holdings
}

fn candidate_priority_score(candidate: &PortfolioCandidateInput) -> i32 {
    let confidence_score = confidence_score(&candidate.position_plan_document.confidence);
    let odds_score = odds_score(&candidate.position_plan_document.odds_grade);
    let risk_penalty = match candidate
        .position_plan_document
        .position_risk_grade
        .as_str()
    {
        "low" => 0,
        "medium" => 5,
        "high" => 10,
        _ => 8,
    };
    confidence_score + odds_score - risk_penalty
}

fn confidence_score(value: &str) -> i32 {
    match value {
        "high" => 30,
        "medium" => 20,
        "low" => 10,
        _ => 5,
    }
}

fn odds_score(value: &str) -> i32 {
    match value {
        "favorable" => 30,
        "balanced" => 20,
        "thin_edge" => 12,
        "pending_research" => 6,
        _ => 8,
    }
}

fn risk_cap_pct(risk_grade: &str) -> f64 {
    match risk_grade {
        "low" => 0.20,
        "medium" => 0.15,
        "high" => 0.10,
        _ => 0.12,
    }
}

// 2026-04-09 CST: 这里新增风险预算折算系数，原因是当前账户层还没有真实波动率和相关性矩阵；
// 目的：先用风险等级做最小可用折算，把“新增仓位”映射成“新增风险预算占用”，为后续精细化预算留接口。
fn risk_budget_unit_cost_pct(risk_grade: &str) -> f64 {
    match risk_grade {
        "low" => 0.25,
        "medium" => 0.50,
        "high" => 0.75,
        _ => 0.60,
    }
}

fn normalized_sector_tag(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .unwrap_or("unknown")
        .to_string()
}

fn normalized_tranche_template(plan: &SecurityPositionPlanDocument) -> String {
    if !plan.tranche_template.trim().is_empty() {
        plan.tranche_template.clone()
    } else {
        "starter_plus_adds".to_string()
    }
}

fn recommended_add_tranche_pct(plan: &SecurityPositionPlanDocument) -> f64 {
    if plan.add_tranche_pct > 0.0 {
        plan.add_tranche_pct
    } else if plan.starter_position_pct > 0.0 {
        (plan.max_position_pct - plan.starter_position_pct)
            .min(plan.starter_position_pct)
            .max(0.0)
    } else {
        0.0
    }
}

fn derived_max_tranche_count(plan: &SecurityPositionPlanDocument) -> usize {
    if plan.max_tranche_count > 0 {
        plan.max_tranche_count
    } else if plan.starter_position_pct <= 0.0 || plan.max_position_pct <= 0.0 {
        0
    } else {
        let add_tranche_pct = recommended_add_tranche_pct(plan);
        if plan.max_position_pct <= plan.starter_position_pct || add_tranche_pct <= 1e-9 {
            1
        } else {
            let remaining = (plan.max_position_pct - plan.starter_position_pct).max(0.0);
            1 + (remaining / add_tranche_pct).ceil() as usize
        }
    }
}

fn tranche_count_after_trade(
    target_position_pct: f64,
    plan: &SecurityPositionPlanDocument,
) -> usize {
    let entry_tranche_pct = if plan.entry_tranche_pct > 0.0 {
        plan.entry_tranche_pct
    } else {
        plan.starter_position_pct
    };
    if target_position_pct <= 0.0 || entry_tranche_pct <= 0.0 {
        return 0;
    }
    let add_tranche_pct = recommended_add_tranche_pct(plan);
    if target_position_pct <= entry_tranche_pct + 1e-9 || add_tranche_pct <= 1e-9 {
        return 1;
    }
    let remaining = (target_position_pct - entry_tranche_pct).max(0.0);
    1 + (remaining / add_tranche_pct).ceil() as usize
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

fn default_min_cash_reserve_pct() -> f64 {
    0.20
}

fn default_max_single_position_pct() -> f64 {
    0.20
}

fn default_max_sector_exposure_pct() -> f64 {
    0.35
}

fn default_max_portfolio_risk_budget_pct() -> f64 {
    0.05
}

fn default_max_single_trade_risk_budget_pct() -> f64 {
    0.02
}

// 2026-04-18 CST: Added because Task 5 should reuse account-level weight,
// return, and warning math in the existing portfolio helper owner file.
// Reason: this avoids cloning portfolio semantics into the monitoring package module.
// Purpose: build the first reusable monitoring account aggregation from evaluated holdings.
pub fn build_monitoring_account_aggregation(
    per_position_evaluations: &[SecurityPerPositionEvaluation],
    position_contracts: &[SecurityPositionContract],
) -> SecurityMonitoringAccountAggregation {
    let active_position_count = per_position_evaluations.len();
    let total_active_weight_pct = round_pct(
        per_position_evaluations
            .iter()
            .map(|evaluation| evaluation.current_weight_pct)
            .sum(),
    );

    let weighted_expected_return_pct = if total_active_weight_pct <= f64::EPSILON {
        0.0
    } else {
        round_pct(
            per_position_evaluations
                .iter()
                .map(|evaluation| {
                    evaluation.current_weight_pct * evaluation.updated_expected_return_pct
                })
                .sum::<f64>()
                / total_active_weight_pct,
        )
    };
    let weighted_expected_drawdown_pct = if total_active_weight_pct <= f64::EPSILON {
        0.0
    } else {
        round_pct(
            per_position_evaluations
                .iter()
                .map(|evaluation| {
                    evaluation.current_weight_pct * evaluation.updated_expected_drawdown_pct
                })
                .sum::<f64>()
                / total_active_weight_pct,
        )
    };
    let total_risk_budget_pct = round_pct(
        position_contracts
            .iter()
            .filter(|contract| {
                per_position_evaluations
                    .iter()
                    .any(|evaluation| evaluation.symbol == contract.symbol)
            })
            .map(|contract| contract.risk_budget_pct)
            .sum(),
    );

    let mut concentration_warnings = Vec::new();
    for evaluation in per_position_evaluations {
        if evaluation.current_weight_pct > evaluation.target_weight_pct + 1e-9 {
            concentration_warnings.push(format!("single_name_over_target:{}", evaluation.symbol));
        }
        if evaluation.current_weight_pct > evaluation.max_weight_pct + 1e-9 {
            concentration_warnings.push(format!("single_name_over_max:{}", evaluation.symbol));
        }
    }
    concentration_warnings.sort();
    concentration_warnings.dedup();

    let correlation_warnings = Vec::new();
    let mut risk_budget_warnings = Vec::new();
    if total_risk_budget_pct > 0.10 {
        risk_budget_warnings.push("risk_budget_pressure_high".to_string());
    }

    SecurityMonitoringAccountAggregation {
        active_position_count,
        total_active_weight_pct,
        weighted_expected_return_pct,
        weighted_expected_drawdown_pct,
        total_risk_budget_pct,
        concentration_warnings,
        correlation_warnings,
        risk_budget_warnings,
        aggregation_summary: format!(
            "monitoring aggregation covers {} active positions with {:.2}% total active weight",
            active_position_count,
            total_active_weight_pct * 100.0
        ),
    }
}

// 2026-04-18 CST: Added because Task 5 should keep action-candidate ranking
// logic in the existing portfolio math owner file.
// Reason: later monitoring, rebalance, and adjustment tasks will all need one
// reusable ranked action section derived from per-position evaluations.
// Purpose: build the first reusable ranked action-candidate payload.
pub fn build_adjustment_simulation_data(
    per_position_evaluations: &[SecurityPerPositionEvaluation],
) -> SecurityAdjustmentSimulationData {
    let mut add_candidates = build_ranked_candidates(per_position_evaluations, |evaluation| {
        evaluation.action_scores.add_score
    });
    let mut trim_candidates = build_ranked_candidates(per_position_evaluations, |evaluation| {
        evaluation.action_scores.trim_score
    });
    let mut replace_candidates = build_ranked_candidates(per_position_evaluations, |evaluation| {
        evaluation.action_scores.replace_score
    });
    let mut exit_candidates = build_ranked_candidates(per_position_evaluations, |evaluation| {
        evaluation.action_scores.exit_score
    });

    add_candidates.truncate(3);
    trim_candidates.truncate(3);
    replace_candidates.truncate(3);
    exit_candidates.truncate(3);

    SecurityAdjustmentSimulationData {
        top_add_candidates: add_candidates,
        top_trim_candidates: trim_candidates,
        top_replace_candidates: replace_candidates,
        top_exit_candidates: exit_candidates,
    }
}

fn build_ranked_candidates<F>(
    per_position_evaluations: &[SecurityPerPositionEvaluation],
    score_selector: F,
) -> Vec<SecurityMonitoringActionCandidate>
where
    F: Fn(&SecurityPerPositionEvaluation) -> f64,
{
    let mut candidates = per_position_evaluations
        .iter()
        .map(|evaluation| SecurityMonitoringActionCandidate {
            symbol: evaluation.symbol.clone(),
            score: score_selector(evaluation),
            recommended_action: evaluation.recommended_action.clone(),
            current_weight_pct: evaluation.current_weight_pct,
            target_weight_pct: evaluation.target_weight_pct,
            current_vs_target_gap_pct: evaluation.current_vs_target_gap_pct,
            per_position_evaluation_ref: evaluation.per_position_evaluation_id.clone(),
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.symbol.cmp(&right.symbol))
    });
    candidates
}

fn round_pct(value: f64) -> f64 {
    (value * 10000.0).round() / 10000.0
}

// 2026-04-19 CST: Added because Task 6 needs one reusable helper that turns
// before/after contracts into governed rebalance deltas.
// Reason: the capital rebase package should reuse portfolio math ownership in this
// file instead of duplicating before/after contract comparison logic elsewhere.
// Purpose: build the reusable capital rebalance simulation rows.
pub fn build_capital_rebalance_simulation(
    original_contracts: &[SecurityPositionContract],
    rebased_contracts: &[SecurityPositionContract],
) -> Vec<SecurityCapitalRebalanceSimulationItem> {
    let mut simulation = original_contracts
        .iter()
        .filter_map(|original_contract| {
            let rebased_contract = rebased_contracts
                .iter()
                .find(|candidate| candidate.symbol == original_contract.symbol)?;
            let principal_delta_amount = round_amount(
                rebased_contract.intended_principal_amount
                    - original_contract.intended_principal_amount,
            );
            let simulation_action_hint = if principal_delta_amount > 0.0 {
                "capital_scale_up".to_string()
            } else if principal_delta_amount < 0.0 {
                "capital_scale_down".to_string()
            } else if (rebased_contract.max_weight_pct - original_contract.max_weight_pct).abs()
                > f64::EPSILON
                || (rebased_contract.risk_budget_pct - original_contract.risk_budget_pct).abs()
                    > f64::EPSILON
            {
                "constraint_reset".to_string()
            } else {
                "no_change".to_string()
            };

            Some(SecurityCapitalRebalanceSimulationItem {
                symbol: original_contract.symbol.clone(),
                target_weight_pct_before: round_pct(original_contract.target_weight_pct),
                target_weight_pct_after: round_pct(rebased_contract.target_weight_pct),
                max_weight_pct_before: round_pct(original_contract.max_weight_pct),
                max_weight_pct_after: round_pct(rebased_contract.max_weight_pct),
                risk_budget_pct_before: round_pct(original_contract.risk_budget_pct),
                risk_budget_pct_after: round_pct(rebased_contract.risk_budget_pct),
                intended_principal_amount_before: round_amount(
                    original_contract.intended_principal_amount,
                ),
                intended_principal_amount_after: round_amount(
                    rebased_contract.intended_principal_amount,
                ),
                principal_delta_amount,
                simulation_action_hint,
                position_contract_ref: original_contract.position_contract_id.clone(),
            })
        })
        .collect::<Vec<_>>();
    simulation.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    simulation
}

fn round_amount(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
