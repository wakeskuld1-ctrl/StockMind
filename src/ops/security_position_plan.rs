use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_approved_open_position_packet::SecurityApprovedOpenPositionPacketDocument;
use crate::ops::stock::security_decision_briefing::{
    SecurityDecisionBriefingError, SecurityDecisionBriefingRequest, SecurityDecisionBriefingResult,
    security_decision_briefing,
};
use crate::ops::stock::security_legacy_committee_compat::LegacySecurityDecisionCommitteeResult as SecurityDecisionCommitteeResult;
use crate::ops::stock::security_master_scorecard::SecurityMasterScorecardDocument;
use crate::ops::stock::security_scorecard::SecurityScorecardDocument;
use crate::ops::stock::stock_analysis_data_guard::StockAnalysisDateGuard;

// 2026-04-02 CST: 这里定义证券仓位计划，原因是审批对象需要从“是否可做”继续落到“准备怎么做”；
// 目的：把执行方案独立成正式对象，后续投中管理、复盘和再审批都围绕同一对象演进。
// 2026-04-08 CST: 这里补入合同头、审批绑定和 reduce_plan，原因是 Task 2 要把仓位计划升级成正式可审批对象；
// 目的：让 approval_request、package、verify 和后续执行层都能围绕统一合同消费 position_plan，而不是继续把它当作临时附属输出。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecurityPositionPlan {
    pub contract_version: String,
    pub document_type: String,
    pub plan_id: String,
    pub decision_id: String,
    pub decision_ref: String,
    pub approval_ref: String,
    pub symbol: String,
    pub analysis_date: String,
    pub plan_direction: String,
    pub plan_status: String,
    pub risk_budget_pct: f64,
    pub suggested_gross_pct: f64,
    pub starter_gross_pct: f64,
    pub max_gross_pct: f64,
    #[serde(default = "default_entry_grade")]
    pub entry_grade: String,
    #[serde(default)]
    pub entry_reason: String,
    #[serde(default)]
    pub entry_blockers: Vec<String>,
    #[serde(default)]
    pub target_gross_pct: f64,
    #[serde(default = "default_sizing_grade")]
    pub sizing_grade: String,
    #[serde(default)]
    pub sizing_reason: String,
    #[serde(default)]
    pub sizing_risk_flags: Vec<String>,
    pub entry_plan: PositionEntryPlan,
    pub add_plan: PositionAddPlan,
    pub reduce_plan: PositionReducePlan,
    pub stop_loss_plan: PositionStopLossPlan,
    pub take_profit_plan: PositionTakeProfitPlan,
    pub cancel_conditions: Vec<String>,
    pub sizing_rationale: Vec<String>,
    pub approval_binding: SecurityPositionPlanApprovalBinding,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionEntryPlan {
    pub entry_mode: String,
    pub trigger_condition: String,
    pub starter_gross_pct: f64,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionAddPlan {
    pub allow_add: bool,
    pub trigger_condition: String,
    pub max_gross_pct: f64,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionReducePlan {
    pub allow_reduce: bool,
    pub trigger_condition: String,
    pub target_gross_pct: f64,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionStopLossPlan {
    pub stop_loss_pct: f64,
    pub hard_stop_condition: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionTakeProfitPlan {
    pub first_target_pct: f64,
    pub second_target_pct: f64,
    pub partial_exit_rule: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecurityPositionPlanApprovalBinding {
    pub decision_ref: String,
    pub approval_ref: String,
    pub approval_request_ref: String,
    pub package_scope: String,
    pub binding_status: String,
}

// 2026-04-14 CST: 这里补回 security_position_plan 的正式 Tool 请求合同，原因是当前主链里的
// execution_journal / execution_record / dispatcher 仍按旧的独立 Tool 口径直接消费仓位计划。
// 目的：先在不推翻现有 builder 的前提下恢复旧调用面，降低本轮“稳底盘”收口成本。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPositionPlanRequest {
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
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-14 CST: 这里补回旧版 position plan 正式文档壳，原因是当前投中/投后链仍引用
// starter_position_pct / max_position_pct / committee_payload_ref 等旧字段名。
// 目的：先让旧链路继续可编译可运行，后续重构时再统一成一份正式仓位文档。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPositionPlanDocument {
    pub position_plan_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub symbol: String,
    pub analysis_date: String,
    pub analysis_date_guard: StockAnalysisDateGuard,
    pub evidence_version: String,
    pub briefing_ref: String,
    pub committee_payload_ref: String,
    pub recommended_action: String,
    pub confidence: String,
    pub odds_grade: String,
    pub historical_confidence: String,
    pub confidence_grade: String,
    pub position_action: String,
    pub entry_mode: String,
    pub starter_position_pct: f64,
    pub max_position_pct: f64,
    #[serde(default)]
    pub risk_budget_pct: f64,
    #[serde(default)]
    pub entry_tranche_pct: f64,
    #[serde(default)]
    pub add_tranche_pct: f64,
    #[serde(default)]
    pub reduce_tranche_pct: f64,
    #[serde(default)]
    pub max_tranche_count: usize,
    #[serde(default)]
    pub tranche_template: String,
    #[serde(default)]
    pub tranche_trigger_rules: Vec<String>,
    #[serde(default)]
    pub cooldown_rule: String,
    pub add_on_trigger: String,
    pub reduce_on_trigger: String,
    pub hard_stop_trigger: String,
    pub liquidity_cap: String,
    pub position_risk_grade: String,
    pub regime_adjustment: String,
    pub execution_notes: Vec<String>,
    pub rationale: Vec<String>,
}

// 2026-04-14 CST: 这里补回旧版 Tool 结果壳，原因是 execution_journal / execution_record
// 当前直接依赖 `position_plan_result.position_plan_document`。
// 目的：以最小兼容方式恢复主链，而不是这一轮强推所有调用方一起改名。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityPositionPlanResult {
    pub briefing_core: SecurityDecisionBriefingResult,
    pub position_plan_document: SecurityPositionPlanDocument,
}

// 2026-04-18 CST: Added because Task 2 needs a small stable bridge object
// between the pre-trade plan document and the post-open live contract layer.
// Reason: the user explicitly asked us not to rename the pre-trade plan into
// the live contract, so the mapping itself must become a named seam.
// Purpose: freeze the first machine-readable seed that `PositionContract` consumes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecurityPositionContractSeed {
    pub position_plan_ref: String,
    pub symbol: String,
    pub analysis_date: String,
    pub entry_mode: String,
    pub risk_budget_pct: f64,
    #[serde(default)]
    pub liquidity_guardrail: Option<String>,
    #[serde(default)]
    pub concentration_guardrail: Option<String>,
}

// 2026-04-14 CST: 这里补回旧版 Tool 错误类型，原因是 execution_journal 已经把 position plan
// 作为正式前置步骤，仍需要稳定的错误边界。
// 目的：继续沿用单点错误封装，避免多条调用链自己解释 briefing 构建失败。
#[derive(Debug, Error)]
pub enum SecurityPositionPlanError {
    #[error("security_position_plan briefing assembly failed: {0}")]
    Briefing(#[from] SecurityDecisionBriefingError),
}

// 2026-04-13 CST: Add a reusable entry-layer assessment object here, because
// the user now wants the system to answer "can we enter now" before we move on
// to later sizing automation.
// Purpose: keep position_plan and chair on one governed entry-grade contract.
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityEntryAssessment {
    pub entry_grade: String,
    pub entry_reason: String,
    pub entry_blockers: Vec<String>,
}

// 2026-04-13 CST: Centralize the rule inputs for the first-stage entry layer,
// because later callers should not hand-roll score-status, chair-action, and
// reward-risk parsing in multiple files.
// Purpose: preserve one stable rule surface while we keep architecture changes minimal.
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityEntryAssessmentInput {
    pub plan_direction: String,
    pub committee_status: String,
    pub score_status: String,
    pub chair_action: String,
    pub confidence_score: f64,
    pub expected_return_pct: Option<f64>,
    pub expected_drawdown_pct: Option<f64>,
    pub warn_count: usize,
    pub blocking_gate_names: Vec<String>,
}

// 2026-04-13 CST: Add a second-stage sizing assessment object here, because the
// user now needs the system to answer "how much is appropriate" after the
// first-stage entry decision is available.
// Purpose: keep target sizing, starter sizing, and add/stop guardrails on one governed contract.
#[derive(Debug, Clone, PartialEq)]
pub struct SecuritySizingAssessment {
    pub target_gross_pct: f64,
    pub starter_gross_pct: f64,
    pub max_gross_pct: f64,
    pub risk_budget_pct: f64,
    pub sizing_grade: String,
    pub sizing_reason: String,
    pub sizing_risk_flags: Vec<String>,
    pub allow_add: bool,
    pub allow_reduce: bool,
    pub reduce_target_gross_pct: f64,
}

// 2026-04-13 CST: Keep the sizing rule inputs explicit, because we want the
// second-stage sizing layer to remain auditable and reusable from both submit
// approval and chair output without hidden coupling.
// Purpose: avoid introducing a second implicit ruleset in later stages.
#[derive(Debug, Clone, PartialEq)]
pub struct SecuritySizingAssessmentInput {
    pub entry_grade: String,
    pub plan_direction: String,
    pub score_status: String,
    pub chair_action: String,
    pub confidence_score: f64,
    pub expected_return_pct: Option<f64>,
    pub expected_drawdown_pct: Option<f64>,
    pub warn_count: usize,
}

// 2026-04-14 CST: 这里补回 security_position_plan 正式 Tool 入口，原因是当前 dispatcher、
// execution_journal、execution_record 仍以它为正式编排起点。
// 目的：让我们在保留新 builder 的同时，先恢复旧主链的可编译和可运行状态。
pub fn security_position_plan(
    request: &SecurityPositionPlanRequest,
) -> Result<SecurityPositionPlanResult, SecurityPositionPlanError> {
    let briefing_request = SecurityDecisionBriefingRequest {
        symbol: request.symbol.clone(),
        market_symbol: request.market_symbol.clone(),
        sector_symbol: request.sector_symbol.clone(),
        market_regime: request.market_regime.clone(),
        sector_template: request.sector_template.clone(),
        as_of_date: request.as_of_date.clone(),
        lookback_days: request.lookback_days,
        factor_lookback_days: request.factor_lookback_days,
        disclosure_limit: request.disclosure_limit,
    };
    let briefing_core = security_decision_briefing(&briefing_request)?;
    let position_plan_document = build_security_position_plan_document(&briefing_core, request);

    Ok(SecurityPositionPlanResult {
        briefing_core,
        position_plan_document,
    })
}

// 2026-04-14 CST: 这里保留旧版文档 builder，原因是现有 execution/package 链消费的是
// briefing_core -> position_plan_document 这一层正式投影。
// 目的：把兼容逻辑继续集中在本文件，避免上层多处重复拼字段。
pub fn build_security_position_plan_document(
    briefing_core: &SecurityDecisionBriefingResult,
    request: &SecurityPositionPlanRequest,
) -> SecurityPositionPlanDocument {
    let position_plan = &briefing_core.position_plan;
    let execution_plan = &briefing_core.execution_plan;
    let odds_brief = &briefing_core.odds_brief;
    let committee_payload = &briefing_core.committee_payload;
    let briefing_ref = briefing_core.evidence_version.clone();
    let committee_payload_ref = format!(
        "committee-payload:{}:{}",
        briefing_core.symbol, briefing_core.analysis_date
    );
    let entry_tranche_pct = position_plan.starter_position_pct;
    let add_tranche_pct = execution_plan.add_position_pct;
    let reduce_tranche_pct = execution_plan.reduce_position_pct;
    let max_tranche_count = derive_max_tranche_count(
        position_plan.starter_position_pct,
        position_plan.max_position_pct,
        execution_plan.add_position_pct,
    );

    SecurityPositionPlanDocument {
        position_plan_id: format!(
            "position-plan-{}-{}",
            briefing_core.symbol, briefing_core.analysis_date
        ),
        contract_version: "security_position_plan.v1".to_string(),
        document_type: "security_position_plan".to_string(),
        generated_at: normalize_created_at(&request.created_at),
        symbol: briefing_core.symbol.clone(),
        analysis_date: briefing_core.analysis_date.clone(),
        analysis_date_guard: briefing_core.analysis_date_guard.clone(),
        evidence_version: briefing_core.evidence_version.clone(),
        briefing_ref,
        committee_payload_ref,
        recommended_action: committee_payload.recommended_action.clone(),
        confidence: committee_payload.confidence.clone(),
        odds_grade: odds_brief.odds_grade.clone(),
        historical_confidence: odds_brief.historical_confidence.clone(),
        confidence_grade: odds_brief.confidence_grade.clone(),
        position_action: position_plan.position_action.clone(),
        entry_mode: position_plan.entry_mode.clone(),
        starter_position_pct: position_plan.starter_position_pct,
        max_position_pct: position_plan.max_position_pct,
        risk_budget_pct: derive_default_position_plan_document_risk_budget_pct(
            &position_plan.position_risk_grade,
        ),
        entry_tranche_pct,
        add_tranche_pct,
        reduce_tranche_pct,
        max_tranche_count,
        tranche_template: "starter_plus_adds".to_string(),
        tranche_trigger_rules: vec![
            format!(
                "首层按 {:.0}% 建立试仓，只在 `{}` 对应场景成立后执行。",
                entry_tranche_pct * 100.0,
                position_plan.entry_mode
            ),
            format!(
                "后续每层按 {:.0}% 推进，并以 `{}` 作为加仓确认。",
                add_tranche_pct * 100.0,
                position_plan.add_on_trigger
            ),
            format!(
                "若触发 `{}`，先按 {:.0}% 节奏减仓。",
                position_plan.reduce_on_trigger,
                reduce_tranche_pct * 100.0
            ),
        ],
        cooldown_rule: "同一交易日不连续执行两次同方向加仓，至少等待一个确认周期。".to_string(),
        add_on_trigger: position_plan.add_on_trigger.clone(),
        reduce_on_trigger: position_plan.reduce_on_trigger.clone(),
        hard_stop_trigger: position_plan.hard_stop_trigger.clone(),
        liquidity_cap: position_plan.liquidity_cap.clone(),
        position_risk_grade: position_plan.position_risk_grade.clone(),
        regime_adjustment: position_plan.regime_adjustment.clone(),
        execution_notes: position_plan.execution_notes.clone(),
        rationale: position_plan.rationale.clone(),
    }
}

// 2026-04-18 CST: Added because Task 2 needs one explicit mapping from the
// pre-trade position-plan document into the post-open contract seed layer.
// Reason: this preserves the user's approved boundary: seed formation stays in
// `security_position_plan`, while live contract formation stays elsewhere.
// Purpose: expose the document-to-seed adapter without widening the public plan object.
pub fn build_position_contract_seed_from_position_plan_document(
    position_plan_document: &SecurityPositionPlanDocument,
) -> SecurityPositionContractSeed {
    SecurityPositionContractSeed {
        position_plan_ref: position_plan_document.position_plan_id.clone(),
        symbol: position_plan_document.symbol.clone(),
        analysis_date: position_plan_document.analysis_date.clone(),
        entry_mode: position_plan_document.entry_mode.clone(),
        risk_budget_pct: if position_plan_document.risk_budget_pct > 0.0 {
            position_plan_document.risk_budget_pct
        } else {
            derive_default_position_plan_document_risk_budget_pct(
                &position_plan_document.position_risk_grade,
            )
        },
        liquidity_guardrail: normalize_optional_text(&position_plan_document.liquidity_cap),
        concentration_guardrail: Some(format!(
            "single_position_cap={:.2}%; tranche_template={}",
            position_plan_document.max_position_pct * 100.0,
            position_plan_document.tranche_template
        )),
    }
}

// 2026-04-18 CST: Added because Task 2 also needs the seed layer to merge the
// post-open approved packet limits with the pre-trade plan seed.
// Reason: live contract formation should happen after both approval and plan
// seed are visible, not from either one independently.
// Purpose: centralize the first packet-plus-seed merge rule for Task 2.
pub fn build_position_contract_seed_from_documents(
    approved_open_position_packet: &SecurityApprovedOpenPositionPacketDocument,
    position_plan_document: &SecurityPositionPlanDocument,
) -> SecurityPositionContractSeed {
    let base_seed =
        build_position_contract_seed_from_position_plan_document(position_plan_document);
    let capped_risk_budget_pct = base_seed
        .risk_budget_pct
        .min(approved_open_position_packet.max_single_trade_risk_budget_pct);

    SecurityPositionContractSeed {
        position_plan_ref: base_seed.position_plan_ref,
        symbol: base_seed.symbol,
        analysis_date: base_seed.analysis_date,
        entry_mode: approved_open_position_packet.recommended_entry_mode.clone(),
        risk_budget_pct: capped_risk_budget_pct,
        liquidity_guardrail: base_seed.liquidity_guardrail,
        concentration_guardrail: Some(format!(
            "single_position_cap={:.2}%; sector_cap={:.2}%",
            approved_open_position_packet.max_single_position_pct * 100.0,
            approved_open_position_packet.max_sector_exposure_pct * 100.0
        )),
    }
}

// 2026-04-14 CST: 这里补本地默认窗口，原因是本轮兼容层直接在本模块声明了 serde default，
// 但当前文件原本只保留 builder，没有这些 Tool 级默认函数。
// 目的：先把旧 Tool 合同补齐到可编译，不依赖外部私有默认函数。
fn default_lookback_days() -> usize {
    180
}

// 2026-04-14 CST: 这里补本地因子窗口默认值，原因同上。
// 目的：保持 position_plan 和 execution_journal 当前默认口径一致。
fn default_factor_lookback_days() -> usize {
    120
}

// 2026-04-14 CST: 这里补公告窗口默认值，原因同上。
// 目的：先恢复 Tool 合同的完整性，避免 serde default 找不到本地函数。
fn default_disclosure_limit() -> usize {
    6
}

// 2026-04-14 CST: 这里补 created_at 默认值，原因是兼容文档壳仍需要稳定时间戳。
// 目的：让旧链路可继续生成正式对象，不因字段默认值缺失而编译失败。
fn default_created_at() -> String {
    Utc::now().to_rfc3339()
}

// 2026-04-14 CST: 这里补 created_at 规范化，原因是兼容文档 builder 仍需要和其他正式对象一样
// 在空值时自动落当前时间。
// 目的：保持输出对象的一致性，不在调用方散落时间戳补齐逻辑。
fn normalize_created_at(value: &str) -> String {
    if value.trim().is_empty() {
        Utc::now().to_rfc3339()
    } else {
        value.trim().to_string()
    }
}

// 2026-04-18 CST: Added because Task 2 needs one stable fallback for older
// position-plan documents that did not persist risk budget explicitly.
// Reason: the live contract layer still needs a deterministic risk-budget seed
// even when the source document came from the earlier compatibility builder.
// Purpose: preserve backward compatibility while the new post-open layers land.
fn derive_default_position_plan_document_risk_budget_pct(position_risk_grade: &str) -> f64 {
    match position_risk_grade {
        "low" => 0.003,
        "medium" => 0.006,
        "high" => 0.01,
        _ => 0.006,
    }
}

// 2026-04-18 CST: Added because the new seed layer should avoid keeping empty
// strings as optional guardrail content.
// Reason: downstream contract documents should not need to distinguish between
// blank text and absent optional guardrail values.
// Purpose: normalize optional seed strings in one place.
fn normalize_optional_text(value: &str) -> Option<String> {
    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

// 2026-04-14 CST: 这里补旧版分层数量推导辅助函数，原因是兼容文档壳仍暴露
// `max_tranche_count` 字段。
// 目的：把这类简单推导继续收口在本模块，而不是让调用方自己估算。
fn derive_max_tranche_count(
    starter_position_pct: f64,
    max_position_pct: f64,
    add_tranche_pct: f64,
) -> usize {
    if starter_position_pct <= 0.0 || max_position_pct <= 0.0 {
        return 0;
    }
    if max_position_pct <= starter_position_pct || add_tranche_pct <= 0.0 {
        return 1;
    }
    let remaining = (max_position_pct - starter_position_pct).max(0.0);
    1 + (remaining / add_tranche_pct).ceil() as usize
}

// 2026-04-02 CST: 这里定义仓位计划生成输入，原因是执行计划除了 committee 结果，还必须拿到当前审批锚点；
// 目的：确保 position_plan 从第一版起就正式绑定 decision_ref / approval_ref，而不是游离在审批对象之外。
// 2026-04-08 CST: 这里补入 decision_id，原因是 Task 2 要让仓位计划能被审批链和版本链直接定位；
// 目的：避免后续 approval_request / revision / review 再从外部反推这份仓位计划属于哪次决议。
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityPositionPlanBuildInput {
    pub decision_id: String,
    pub decision_ref: String,
    pub approval_ref: String,
}

// 2026-04-02 CST: 这里实现规则型仓位规划器，原因是当前阶段先要稳定生成可审批执行方案，而不是追求复杂优化；
// 目的：用可解释规则把 `blocked / needs_more_evidence / ready_for_review` 分别落成不同仓位级别。
pub fn build_security_position_plan(
    committee: &SecurityDecisionCommitteeResult,
    input: &SecurityPositionPlanBuildInput,
) -> SecurityPositionPlan {
    let status = committee.decision_card.status.as_str();
    let confidence = committee.decision_card.confidence_score;
    let warn_count = committee
        .risk_gates
        .iter()
        .filter(|gate| gate.result == "warn")
        .count();

    let (plan_status, _suggested, starter, max_gross, _risk_budget, allow_add) = match status {
        "blocked" => ("blocked", 0.0, 0.0, 0.0, 0.0, false),
        "needs_more_evidence" => ("probe_only", 0.05, 0.03, 0.05, 0.005, false),
        _ => {
            let mut suggested = if confidence >= 0.80 { 0.12 } else { 0.10 };
            if warn_count > 0 {
                suggested -= 0.02;
            }
            let starter = if suggested >= 0.10 { 0.06 } else { 0.05 };
            let max_gross = (suggested + 0.03_f64).min(0.15_f64);
            ("reviewable", suggested, starter, max_gross, 0.01, true)
        }
    };

    let stop_loss_pct = parse_percent(&committee.decision_card.downside_risk).unwrap_or(0.05);
    let (first_target_pct, second_target_pct) =
        parse_percent_range(&committee.decision_card.expected_return_range);
    let allow_reduce = plan_status != "blocked";
    let reduce_target_gross_pct = if plan_status == "blocked" {
        0.0
    } else if plan_status == "probe_only" {
        0.0
    } else {
        starter
    };
    let plan_direction = normalize_plan_direction(&committee.decision_card.exposure_side);
    let initial_entry_assessment = build_security_entry_assessment(&SecurityEntryAssessmentInput {
        plan_direction: plan_direction.clone(),
        committee_status: committee.decision_card.status.clone(),
        score_status: "pending_scorecard".to_string(),
        chair_action: committee.decision_card.recommendation_action.clone(),
        confidence_score: confidence,
        expected_return_pct: Some(parse_percent_midpoint(
            &committee.decision_card.expected_return_range,
        )),
        expected_drawdown_pct: parse_percent(&committee.decision_card.downside_risk),
        warn_count,
        blocking_gate_names: collect_blocking_gate_names(committee),
    });
    let initial_sizing_assessment =
        build_security_sizing_assessment(&SecuritySizingAssessmentInput {
            entry_grade: initial_entry_assessment.entry_grade.clone(),
            plan_direction: plan_direction.clone(),
            score_status: "pending_scorecard".to_string(),
            chair_action: committee.decision_card.recommendation_action.clone(),
            confidence_score: confidence,
            expected_return_pct: Some(parse_percent_midpoint(
                &committee.decision_card.expected_return_range,
            )),
            expected_drawdown_pct: parse_percent(&committee.decision_card.downside_risk),
            warn_count,
        });

    let cancel_conditions = if plan_status == "blocked" {
        vec![
            "当前风险闸门未通过，不进入执行。".to_string(),
            committee.decision_card.final_recommendation.clone(),
        ]
    } else if plan_status == "probe_only" {
        vec![
            "补齐证据前不得扩大仓位。".to_string(),
            "若出现新增阻断性风险闸门，取消执行。".to_string(),
        ]
    } else {
        vec![
            "若跌破止损条件则取消后续加仓。".to_string(),
            "若市场或板块环境明显转逆风，则暂停执行。".to_string(),
        ]
    };

    let sizing_rationale = match plan_status {
        "blocked" => vec![
            "当前投决状态为 blocked，因此仓位计划归零。".to_string(),
            "执行计划仅保留取消条件，不生成建仓动作。".to_string(),
        ],
        "probe_only" => vec![
            "当前仅处于 needs_more_evidence，对应试探仓计划。".to_string(),
            "在补证据并重新审批前，不允许扩大仓位。".to_string(),
        ],
        _ => vec![
            format!("当前可进入审阅状态，置信度 {:.2}。", confidence),
            format!("存在 {} 个提醒闸门，已在仓位上做降档处理。", warn_count),
        ],
    };

    let mut position_plan = SecurityPositionPlan {
        contract_version: "security_position_plan.v2".to_string(),
        document_type: "security_position_plan".to_string(),
        plan_id: format!("plan-{}", committee.decision_card.decision_id),
        decision_id: input.decision_id.clone(),
        decision_ref: input.decision_ref.clone(),
        approval_ref: input.approval_ref.clone(),
        symbol: committee.symbol.clone(),
        analysis_date: committee.analysis_date.clone(),
        plan_direction,
        plan_status: plan_status.to_string(),
        risk_budget_pct: initial_sizing_assessment.risk_budget_pct,
        suggested_gross_pct: initial_sizing_assessment.target_gross_pct,
        starter_gross_pct: initial_sizing_assessment.starter_gross_pct,
        max_gross_pct: initial_sizing_assessment.max_gross_pct,
        entry_grade: initial_entry_assessment.entry_grade.clone(),
        entry_reason: initial_entry_assessment.entry_reason.clone(),
        entry_blockers: initial_entry_assessment.entry_blockers.clone(),
        // 2026-04-13 CST: Persist second-stage sizing fields here, because the
        // formal plan must carry target sizing for later approval/chair reuse.
        // Purpose: make the new sizing layer machine-readable across artifacts.
        target_gross_pct: initial_sizing_assessment.target_gross_pct,
        sizing_grade: initial_sizing_assessment.sizing_grade.clone(),
        sizing_reason: initial_sizing_assessment.sizing_reason.clone(),
        sizing_risk_flags: initial_sizing_assessment.sizing_risk_flags.clone(),
        entry_plan: PositionEntryPlan {
            entry_mode: build_entry_mode_from_entry_grade(&initial_entry_assessment.entry_grade),
            trigger_condition: if plan_status == "blocked" {
                "当前不允许建仓".to_string()
            } else {
                format!("首仓 {:.1}% ，满足投决条件后执行", starter * 100.0)
            },
            starter_gross_pct: starter,
            notes: format!("首仓方案依据当前状态 {}", plan_status),
        },
        add_plan: PositionAddPlan {
            allow_add,
            trigger_condition: if allow_add {
                "回踩确认或突破延续后允许加仓".to_string()
            } else {
                "当前不允许加仓".to_string()
            },
            max_gross_pct: max_gross,
            notes: if allow_add {
                "加仓前需继续满足风险闸门要求".to_string()
            } else {
                "补证据或风险解除前禁止加仓".to_string()
            },
        },
        reduce_plan: PositionReducePlan {
            allow_reduce,
            trigger_condition: if allow_reduce {
                "达到第一目标位或市场环境转弱时允许主动减仓".to_string()
            } else {
                "当前无持仓可减".to_string()
            },
            target_gross_pct: reduce_target_gross_pct,
            notes: if allow_reduce {
                "减仓规则用于把仓位降回更稳健区间，避免只定义加仓而不定义收缩。".to_string()
            } else {
                "blocked 状态下不生成减仓动作。".to_string()
            },
        },
        stop_loss_plan: PositionStopLossPlan {
            stop_loss_pct,
            hard_stop_condition: if plan_status == "blocked" {
                "不执行，无止损动作".to_string()
            } else {
                format!("跌破 {:.1}% 风险线则执行硬止损", stop_loss_pct * 100.0)
            },
            notes: "止损线直接继承投决会风险参数".to_string(),
        },
        take_profit_plan: PositionTakeProfitPlan {
            first_target_pct,
            second_target_pct,
            partial_exit_rule: if plan_status == "blocked" {
                "不执行，无止盈动作".to_string()
            } else {
                "第一目标减仓三分之一，第二目标继续兑现".to_string()
            },
            notes: "止盈目标沿用投决卡预期收益区间".to_string(),
        },
        cancel_conditions,
        sizing_rationale,
        approval_binding: SecurityPositionPlanApprovalBinding {
            decision_ref: input.decision_ref.clone(),
            approval_ref: input.approval_ref.clone(),
            approval_request_ref: input.approval_ref.clone(),
            package_scope: "security_decision_submit_approval".to_string(),
            binding_status: "bound_to_approval_request".to_string(),
        },
    };
    // 2026-04-13 CST: Re-apply the shared sizing mapping before returning,
    // because the legacy inline build block above still contains first-stage
    // defaults that must now be overridden by the governed sizing contract.
    // Purpose: keep this change incremental without a larger file rewrite.
    apply_security_sizing_assessment_to_position_plan(
        &mut position_plan,
        &initial_sizing_assessment,
        &committee.decision_card.final_recommendation,
        confidence,
        warn_count,
    );
    position_plan
}

// 2026-04-13 CST: Apply the first-stage entry overlay after scorecard/master
// scorecard become available, because submit_approval currently builds the
// base position_plan before quant readiness is known.
// Purpose: refresh the formal plan in-place instead of introducing another
// parallel object or larger refactor.
pub fn apply_security_entry_layer_to_position_plan(
    position_plan: &mut SecurityPositionPlan,
    committee: &SecurityDecisionCommitteeResult,
    scorecard: &SecurityScorecardDocument,
    master_scorecard: Option<&SecurityMasterScorecardDocument>,
    chair_action: &str,
) {
    let entry_assessment = build_security_entry_assessment_from_documents(
        committee,
        scorecard,
        master_scorecard,
        &position_plan.plan_direction,
        chair_action,
    );
    position_plan.entry_grade = entry_assessment.entry_grade;
    position_plan.entry_reason = entry_assessment.entry_reason.clone();
    position_plan.entry_blockers = entry_assessment.entry_blockers.clone();
    position_plan.entry_plan.entry_mode =
        build_entry_mode_from_entry_grade(&position_plan.entry_grade);
    position_plan.entry_plan.notes = format!(
        "{} | {}",
        position_plan.entry_plan.notes, entry_assessment.entry_reason
    );
}

// 2026-04-13 CST: Add a document-driven sizing builder next to the entry
// builder, because chair and submit_approval must reuse the same second-stage
// sizing semantics instead of each inventing their own target percentages.
// Purpose: keep the new "how much" layer stable after this architecture change.
pub fn build_security_sizing_assessment_from_documents(
    committee: &SecurityDecisionCommitteeResult,
    scorecard: &SecurityScorecardDocument,
    master_scorecard: Option<&SecurityMasterScorecardDocument>,
    plan_direction: &str,
    chair_action: &str,
    entry_grade: &str,
) -> SecuritySizingAssessment {
    let (expected_return_pct, expected_drawdown_pct) =
        resolve_expected_entry_metrics(committee, master_scorecard);
    build_security_sizing_assessment(&SecuritySizingAssessmentInput {
        entry_grade: entry_grade.to_string(),
        plan_direction: plan_direction.to_string(),
        score_status: scorecard.score_status.clone(),
        chair_action: chair_action.to_string(),
        confidence_score: committee.decision_card.confidence_score,
        expected_return_pct,
        expected_drawdown_pct,
        warn_count: committee
            .risk_gates
            .iter()
            .filter(|gate| gate.result == "warn")
            .count(),
    })
}

// 2026-04-13 CST: Refresh the second-stage sizing layer after scorecard/master
// scorecard land, because submit_approval currently builds the base plan before
// quant readiness and chair action are finalized.
// Purpose: update one formal plan object in place instead of creating a sibling object.
pub fn apply_security_sizing_layer_to_position_plan(
    position_plan: &mut SecurityPositionPlan,
    committee: &SecurityDecisionCommitteeResult,
    scorecard: &SecurityScorecardDocument,
    master_scorecard: Option<&SecurityMasterScorecardDocument>,
    chair_action: &str,
) {
    let warn_count = committee
        .risk_gates
        .iter()
        .filter(|gate| gate.result == "warn")
        .count();
    let sizing_assessment = build_security_sizing_assessment_from_documents(
        committee,
        scorecard,
        master_scorecard,
        &position_plan.plan_direction,
        chair_action,
        &position_plan.entry_grade,
    );
    apply_security_sizing_assessment_to_position_plan(
        position_plan,
        &sizing_assessment,
        &committee.decision_card.final_recommendation,
        committee.decision_card.confidence_score,
        warn_count,
    );
}

// 2026-04-13 CST: Build the shared entry-layer assessment from the governed
// committee, scorecard, and optional master_scorecard documents, because both
// the position plan and chair output need the same answer.
// Purpose: prevent the dual-anchor rollout from drifting into two rule sets.
pub fn build_security_entry_assessment_from_documents(
    committee: &SecurityDecisionCommitteeResult,
    scorecard: &SecurityScorecardDocument,
    master_scorecard: Option<&SecurityMasterScorecardDocument>,
    plan_direction: &str,
    chair_action: &str,
) -> SecurityEntryAssessment {
    let (expected_return_pct, expected_drawdown_pct) =
        resolve_expected_entry_metrics(committee, master_scorecard);
    build_security_entry_assessment(&SecurityEntryAssessmentInput {
        plan_direction: plan_direction.to_string(),
        committee_status: committee.decision_card.status.clone(),
        score_status: scorecard.score_status.clone(),
        chair_action: chair_action.to_string(),
        confidence_score: committee.decision_card.confidence_score,
        expected_return_pct,
        expected_drawdown_pct,
        warn_count: committee
            .risk_gates
            .iter()
            .filter(|gate| gate.result == "warn")
            .count(),
        blocking_gate_names: collect_blocking_gate_names(committee),
    })
}

// 2026-04-13 CST: Keep the first-stage entry grading rules small and explicit,
// because the user asked us not to do another broad refactor and only needs a
// reliable answer to "when can we start entering".
// Purpose: ship one auditable ruleset before later sizing and scaling stages.
pub fn build_security_entry_assessment(
    input: &SecurityEntryAssessmentInput,
) -> SecurityEntryAssessment {
    if !input.blocking_gate_names.is_empty() || input.committee_status == "blocked" {
        let mut entry_blockers = input.blocking_gate_names.clone();
        if entry_blockers.is_empty() {
            entry_blockers.push("committee_blocked".to_string());
        }
        return SecurityEntryAssessment {
            entry_grade: "blocked".to_string(),
            entry_reason: format!(
                "entry blocked because committee status is `{}` and blocking gates remain unresolved",
                input.committee_status
            ),
            entry_blockers,
        };
    }

    if input.score_status != "ready" {
        return SecurityEntryAssessment {
            entry_grade: "watch".to_string(),
            entry_reason: format!(
                "watch only because scorecard status `{}` is not ready for governed entry",
                input.score_status
            ),
            entry_blockers: vec![input.score_status.clone()],
        };
    }

    if input.chair_action != "buy" || input.plan_direction != "Long" {
        return SecurityEntryAssessment {
            entry_grade: "watch".to_string(),
            entry_reason: format!(
                "watch only because chair action `{}` with direction `{}` does not authorize a long entry",
                input.chair_action, input.plan_direction
            ),
            entry_blockers: vec![
                format!("chair_action:{}", input.chair_action),
                format!("plan_direction:{}", input.plan_direction),
            ],
        };
    }

    let expected_return_pct = input.expected_return_pct.unwrap_or(0.0);
    let expected_drawdown_pct = input.expected_drawdown_pct.unwrap_or(0.0);
    let reward_risk_ratio = if expected_drawdown_pct <= f64::EPSILON {
        expected_return_pct
    } else {
        expected_return_pct / expected_drawdown_pct
    };

    if input.confidence_score >= 0.78 && input.warn_count == 0 && reward_risk_ratio >= 1.5 {
        return SecurityEntryAssessment {
            entry_grade: "standard_long".to_string(),
            entry_reason: format!(
                "standard long entry cleared with confidence {:.2}, reward-risk {:.2}, and no warning gates",
                input.confidence_score, reward_risk_ratio
            ),
            entry_blockers: Vec::new(),
        };
    }

    SecurityEntryAssessment {
        entry_grade: "pilot_long".to_string(),
        entry_reason: format!(
            "pilot long entry allowed with confidence {:.2}, reward-risk {:.2}, and {} warning gates",
            input.confidence_score, reward_risk_ratio, input.warn_count
        ),
        entry_blockers: Vec::new(),
    }
}

// 2026-04-13 CST: Centralize the second-stage sizing rules here, because the
// user wants one governed answer to "how much" and later modules must not
// drift into their own target sizing heuristics.
// Purpose: keep target/starter/max/add-reduce semantics auditable and reusable.
pub fn build_security_sizing_assessment(
    input: &SecuritySizingAssessmentInput,
) -> SecuritySizingAssessment {
    match input.entry_grade.as_str() {
        "blocked" => SecuritySizingAssessment {
            target_gross_pct: 0.0,
            starter_gross_pct: 0.0,
            max_gross_pct: 0.0,
            risk_budget_pct: 0.0,
            sizing_grade: "blocked_flat".to_string(),
            sizing_reason: format!(
                "sizing forced flat because entry grade `{}` blocks execution",
                input.entry_grade
            ),
            sizing_risk_flags: vec!["entry_blocked".to_string()],
            allow_add: false,
            allow_reduce: false,
            reduce_target_gross_pct: 0.0,
        },
        "watch" => SecuritySizingAssessment {
            target_gross_pct: 0.01,
            starter_gross_pct: 0.01,
            max_gross_pct: 0.01,
            risk_budget_pct: 0.001,
            sizing_grade: "watch_probe".to_string(),
            sizing_reason: format!(
                "watch grade keeps only a tiny probe while score status `{}` / chair `{}` still need confirmation",
                input.score_status, input.chair_action
            ),
            sizing_risk_flags: vec![
                format!("score_status:{}", input.score_status),
                format!("chair_action:{}", input.chair_action),
            ],
            allow_add: false,
            allow_reduce: true,
            reduce_target_gross_pct: 0.0,
        },
        "standard_long" => SecuritySizingAssessment {
            target_gross_pct: 0.12,
            starter_gross_pct: 0.06,
            max_gross_pct: 0.15,
            risk_budget_pct: 0.01,
            sizing_grade: "standard_build".to_string(),
            sizing_reason: format!(
                "standard build cleared with confidence {:.2} and warning gate count {}",
                input.confidence_score, input.warn_count
            ),
            sizing_risk_flags: if input.warn_count == 0 {
                Vec::new()
            } else {
                vec![format!("warn_count:{}", input.warn_count)]
            },
            allow_add: true,
            allow_reduce: true,
            reduce_target_gross_pct: 0.06,
        },
        _ => SecuritySizingAssessment {
            target_gross_pct: 0.06,
            starter_gross_pct: 0.03,
            max_gross_pct: 0.08,
            risk_budget_pct: 0.006,
            sizing_grade: "pilot_build".to_string(),
            sizing_reason: format!(
                "pilot build keeps moderate size because entry grade `{}` is not yet standard conviction",
                input.entry_grade
            ),
            sizing_risk_flags: if input.warn_count == 0 {
                Vec::new()
            } else {
                vec![format!("warn_count:{}", input.warn_count)]
            },
            allow_add: true,
            allow_reduce: true,
            reduce_target_gross_pct: 0.03,
        },
    }
}

// 2026-04-13 CST: Keep all in-place sizing field updates behind one helper,
// because build + submit + chair must stay on the same mapping and text contract.
// Purpose: minimize future touch points and avoid another repeated refactor.
fn apply_security_sizing_assessment_to_position_plan(
    position_plan: &mut SecurityPositionPlan,
    sizing_assessment: &SecuritySizingAssessment,
    final_recommendation: &str,
    confidence_score: f64,
    warn_count: usize,
) {
    let plan_status = position_plan.plan_status.clone();
    position_plan.risk_budget_pct = sizing_assessment.risk_budget_pct;
    position_plan.suggested_gross_pct = sizing_assessment.target_gross_pct;
    position_plan.starter_gross_pct = sizing_assessment.starter_gross_pct;
    position_plan.max_gross_pct = sizing_assessment.max_gross_pct;
    position_plan.target_gross_pct = sizing_assessment.target_gross_pct;
    position_plan.sizing_grade = sizing_assessment.sizing_grade.clone();
    position_plan.sizing_reason = sizing_assessment.sizing_reason.clone();
    position_plan.sizing_risk_flags = sizing_assessment.sizing_risk_flags.clone();
    position_plan.entry_plan.entry_mode =
        build_entry_mode_from_entry_grade(&position_plan.entry_grade);
    position_plan.entry_plan.trigger_condition =
        build_entry_trigger_condition(&position_plan.entry_grade, sizing_assessment);
    position_plan.entry_plan.starter_gross_pct = sizing_assessment.starter_gross_pct;
    position_plan.entry_plan.notes = format!(
        "首仓方案依据入场等级 {} / sizing {}",
        position_plan.entry_grade, sizing_assessment.sizing_grade
    );
    position_plan.add_plan.allow_add = sizing_assessment.allow_add;
    position_plan.add_plan.trigger_condition =
        build_add_trigger_condition(sizing_assessment.allow_add);
    position_plan.add_plan.max_gross_pct = sizing_assessment.max_gross_pct;
    position_plan.add_plan.notes = build_add_notes(sizing_assessment.allow_add);
    position_plan.reduce_plan.allow_reduce = sizing_assessment.allow_reduce;
    position_plan.reduce_plan.trigger_condition =
        build_reduce_trigger_condition(sizing_assessment.allow_reduce);
    position_plan.reduce_plan.target_gross_pct = sizing_assessment.reduce_target_gross_pct;
    position_plan.reduce_plan.notes = build_reduce_notes(sizing_assessment.allow_reduce);
    position_plan.stop_loss_plan.hard_stop_condition = if plan_status == "blocked" {
        "不执行，无止损动作".to_string()
    } else {
        format!(
            "跌破 {:.1}% 风险线则执行硬止损",
            position_plan.stop_loss_plan.stop_loss_pct * 100.0
        )
    };
    position_plan.take_profit_plan.partial_exit_rule = if plan_status == "blocked" {
        "不执行，无止盈动作".to_string()
    } else {
        "第一目标减仓三分之一，第二目标继续兑现".to_string()
    };
    position_plan.cancel_conditions = build_cancel_conditions(&plan_status, final_recommendation);
    position_plan.sizing_rationale = build_sizing_rationale_lines(
        &position_plan.entry_grade,
        sizing_assessment,
        confidence_score,
        warn_count,
    );
}

fn build_entry_mode_from_entry_grade(entry_grade: &str) -> String {
    match entry_grade {
        "blocked" => "disabled".to_string(),
        "watch" => "probe".to_string(),
        _ => "staged".to_string(),
    }
}

fn build_entry_trigger_condition(
    entry_grade: &str,
    sizing_assessment: &SecuritySizingAssessment,
) -> String {
    match entry_grade {
        "blocked" => "当前不允许建仓".to_string(),
        "watch" => format!(
            "仅允许观察仓 {:.1}% ，待量化与主席动作确认后再评估",
            sizing_assessment.starter_gross_pct * 100.0
        ),
        _ => format!(
            "首仓 {:.1}% ，满足投决条件后执行",
            sizing_assessment.starter_gross_pct * 100.0
        ),
    }
}

fn build_add_trigger_condition(allow_add: bool) -> String {
    if allow_add {
        "回踩确认或突破延续后允许加仓".to_string()
    } else {
        "当前不允许加仓".to_string()
    }
}

fn build_add_notes(allow_add: bool) -> String {
    if allow_add {
        "加仓前需继续满足风险闸门要求".to_string()
    } else {
        "补证据或风险解除前禁止加仓".to_string()
    }
}

fn build_reduce_trigger_condition(allow_reduce: bool) -> String {
    if allow_reduce {
        "达到第一目标位或市场环境转弱时允许主动减仓".to_string()
    } else {
        "当前无持仓可减".to_string()
    }
}

fn build_reduce_notes(allow_reduce: bool) -> String {
    if allow_reduce {
        "减仓规则用于把仓位降回更稳健区间，避免只定义加仓而不定义收缩。".to_string()
    } else {
        "blocked 状态下不生成减仓动作。".to_string()
    }
}

fn build_cancel_conditions(plan_status: &str, final_recommendation: &str) -> Vec<String> {
    match plan_status {
        "blocked" => vec![
            "当前风险闸门未通过，不进入执行。".to_string(),
            final_recommendation.to_string(),
        ],
        "probe_only" => vec![
            "补齐证据前不得扩大仓位。".to_string(),
            "若出现新增阻断性风险阀门，则取消执行。".to_string(),
        ],
        _ => vec![
            "若跌破止损条件则取消后续加仓。".to_string(),
            "若市场或板块环境明显转逆风，则暂停执行。".to_string(),
        ],
    }
}

fn build_sizing_rationale_lines(
    entry_grade: &str,
    sizing_assessment: &SecuritySizingAssessment,
    confidence_score: f64,
    warn_count: usize,
) -> Vec<String> {
    let mut lines = vec![
        format!(
            "entry grade `{}` mapped to sizing grade `{}`",
            entry_grade, sizing_assessment.sizing_grade
        ),
        sizing_assessment.sizing_reason.clone(),
        format!(
            "target {:.2}% / starter {:.2}% / max {:.2}% with confidence {:.2}",
            sizing_assessment.target_gross_pct * 100.0,
            sizing_assessment.starter_gross_pct * 100.0,
            sizing_assessment.max_gross_pct * 100.0,
            confidence_score
        ),
    ];
    if warn_count > 0 {
        lines.push(format!("warning gate count {}", warn_count));
    }
    if !sizing_assessment.sizing_risk_flags.is_empty() {
        lines.push(format!(
            "risk flags {}",
            sizing_assessment.sizing_risk_flags.join(",")
        ));
    }
    lines
}

fn normalize_plan_direction(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "long" => "Long".to_string(),
        "short" => "Short".to_string(),
        "hedge" => "Hedge".to_string(),
        "neutral" => "NoTrade".to_string(),
        _ => "NoTrade".to_string(),
    }
}

fn parse_percent(value: &str) -> Option<f64> {
    value
        .trim()
        .trim_end_matches('%')
        .parse::<f64>()
        .ok()
        .map(|v| v / 100.0)
}

fn parse_percent_midpoint(value: &str) -> f64 {
    let (first, second) = parse_percent_range(value);
    if second <= f64::EPSILON {
        first
    } else {
        (first + second) / 2.0
    }
}

fn parse_percent_range(value: &str) -> (f64, f64) {
    let values: Vec<f64> = value
        .split('-')
        .filter_map(|part| parse_percent(part))
        .collect();
    let first = values.first().copied().unwrap_or(0.0);
    let second = values.get(1).copied().unwrap_or(first);
    (first, second)
}

fn collect_blocking_gate_names(committee: &SecurityDecisionCommitteeResult) -> Vec<String> {
    committee
        .risk_gates
        .iter()
        .filter(|gate| gate.blocking && gate.result == "fail")
        .map(|gate| gate.gate_name.clone())
        .collect()
}

fn resolve_expected_entry_metrics(
    committee: &SecurityDecisionCommitteeResult,
    master_scorecard: Option<&SecurityMasterScorecardDocument>,
) -> (Option<f64>, Option<f64>) {
    if let Some(master_scorecard) = master_scorecard {
        if let Some(prediction_summary) = master_scorecard.prediction_summary.as_ref() {
            return (
                prediction_summary.regression_line.expected_return,
                prediction_summary.risk_line.expected_drawdown,
            );
        }

        if master_scorecard
            .trained_head_summary
            .expected_return
            .is_some()
            || master_scorecard
                .trained_head_summary
                .expected_drawdown
                .is_some()
        {
            return (
                master_scorecard.trained_head_summary.expected_return,
                master_scorecard.trained_head_summary.expected_drawdown,
            );
        }
    }

    (
        Some(parse_percent_midpoint(
            &committee.decision_card.expected_return_range,
        )),
        parse_percent(&committee.decision_card.downside_risk),
    )
}

fn default_entry_grade() -> String {
    "watch".to_string()
}

fn default_sizing_grade() -> String {
    "watch_probe".to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        SecurityEntryAssessmentInput, SecuritySizingAssessmentInput,
        build_security_entry_assessment, build_security_sizing_assessment,
    };

    #[test]
    fn security_entry_assessment_marks_blocked_when_blocking_gate_exists() {
        let result = build_security_entry_assessment(&SecurityEntryAssessmentInput {
            plan_direction: "Long".to_string(),
            committee_status: "blocked".to_string(),
            score_status: "ready".to_string(),
            chair_action: "buy".to_string(),
            confidence_score: 0.82,
            expected_return_pct: Some(0.12),
            expected_drawdown_pct: Some(0.04),
            warn_count: 0,
            blocking_gate_names: vec!["risk_reward_gate".to_string()],
        });

        assert_eq!(result.entry_grade, "blocked");
        assert!(
            result
                .entry_blockers
                .iter()
                .any(|item| item == "risk_reward_gate")
        );
    }

    #[test]
    fn security_entry_assessment_marks_watch_when_scorecard_is_not_ready() {
        let result = build_security_entry_assessment(&SecurityEntryAssessmentInput {
            plan_direction: "Long".to_string(),
            committee_status: "ready_for_review".to_string(),
            score_status: "model_unavailable".to_string(),
            chair_action: "buy".to_string(),
            confidence_score: 0.82,
            expected_return_pct: Some(0.12),
            expected_drawdown_pct: Some(0.04),
            warn_count: 0,
            blocking_gate_names: Vec::new(),
        });

        assert_eq!(result.entry_grade, "watch");
        assert!(
            result
                .entry_blockers
                .iter()
                .any(|item| item == "model_unavailable")
        );
    }

    #[test]
    fn security_entry_assessment_marks_pilot_long_when_edge_is_thin() {
        let result = build_security_entry_assessment(&SecurityEntryAssessmentInput {
            plan_direction: "Long".to_string(),
            committee_status: "ready_for_review".to_string(),
            score_status: "ready".to_string(),
            chair_action: "buy".to_string(),
            confidence_score: 0.71,
            expected_return_pct: Some(0.09),
            expected_drawdown_pct: Some(0.05),
            warn_count: 1,
            blocking_gate_names: Vec::new(),
        });

        assert_eq!(result.entry_grade, "pilot_long");
    }

    #[test]
    fn security_entry_assessment_marks_standard_long_when_edge_is_strong() {
        let result = build_security_entry_assessment(&SecurityEntryAssessmentInput {
            plan_direction: "Long".to_string(),
            committee_status: "ready_for_review".to_string(),
            score_status: "ready".to_string(),
            chair_action: "buy".to_string(),
            confidence_score: 0.84,
            expected_return_pct: Some(0.12),
            expected_drawdown_pct: Some(0.04),
            warn_count: 0,
            blocking_gate_names: Vec::new(),
        });

        assert_eq!(result.entry_grade, "standard_long");
    }

    #[test]
    fn security_sizing_assessment_marks_watch_probe_with_tiny_target() {
        let result = build_security_sizing_assessment(&SecuritySizingAssessmentInput {
            entry_grade: "watch".to_string(),
            plan_direction: "NoTrade".to_string(),
            score_status: "ready".to_string(),
            chair_action: "abstain".to_string(),
            confidence_score: 0.66,
            expected_return_pct: Some(0.08),
            expected_drawdown_pct: Some(0.05),
            warn_count: 1,
        });

        assert_eq!(result.target_gross_pct, 0.01);
        assert_eq!(result.sizing_grade, "watch_probe");
        assert_eq!(result.allow_add, false);
    }

    #[test]
    fn security_sizing_assessment_marks_pilot_build_between_watch_and_standard() {
        let result = build_security_sizing_assessment(&SecuritySizingAssessmentInput {
            entry_grade: "pilot_long".to_string(),
            plan_direction: "Long".to_string(),
            score_status: "ready".to_string(),
            chair_action: "buy".to_string(),
            confidence_score: 0.73,
            expected_return_pct: Some(0.10),
            expected_drawdown_pct: Some(0.05),
            warn_count: 1,
        });

        assert_eq!(result.target_gross_pct, 0.06);
        assert_eq!(result.sizing_grade, "pilot_build");
        assert_eq!(result.allow_add, true);
    }

    #[test]
    fn security_sizing_assessment_marks_standard_build_with_larger_target() {
        let result = build_security_sizing_assessment(&SecuritySizingAssessmentInput {
            entry_grade: "standard_long".to_string(),
            plan_direction: "Long".to_string(),
            score_status: "ready".to_string(),
            chair_action: "buy".to_string(),
            confidence_score: 0.86,
            expected_return_pct: Some(0.12),
            expected_drawdown_pct: Some(0.04),
            warn_count: 0,
        });

        assert_eq!(result.target_gross_pct, 0.12);
        assert_eq!(result.sizing_grade, "standard_build");
        assert_eq!(result.allow_add, true);
    }
}
