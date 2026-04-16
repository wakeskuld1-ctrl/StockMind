use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::ops::stock::security_legacy_committee_compat::LegacySecurityDecisionCommitteeResult as SecurityDecisionCommitteeResult;
use crate::ops::stock::security_position_plan::SecurityPositionPlan;

// 2026-04-02 CST: 这里定义正式审批简报文档，原因是 P0-3 目标不是继续堆临时摘要，而是输出可落盘、可签名、可进入 package 的正式对象；
// 目的：把审批阅读所需的核心信息集中收口为稳定合同，后续可单独落盘、签名和装入 decision package。
// 2026-04-02 CST: 这里补齐审批简报合同的反序列化能力，原因是 P0-5 需要回读 approval_brief 做 detached signature 与治理校验；
// 目的：让 verify Tool 可以按正式 brief 合同重建正文，而不是基于临时字段名做脆弱解析。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityDecisionApprovalBrief {
    pub brief_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub scene_name: String,
    pub decision_id: String,
    pub decision_ref: String,
    pub approval_ref: String,
    pub symbol: String,
    pub analysis_date: String,
    pub decision_status: String,
    pub approval_status: String,
    pub committee_status: String,
    pub direction: String,
    pub confidence_score: f64,
    pub confidence_band: String,
    pub executive_summary: String,
    pub bull_summary: Vec<String>,
    pub bear_summary: Vec<String>,
    pub core_supporting_points: Vec<String>,
    pub core_risks: Vec<String>,
    pub gate_summary: Vec<String>,
    pub gate_outcome_summary: Vec<String>,
    pub position_summary: String,
    pub risk_budget_summary: String,
    pub entry_summary: String,
    pub add_summary: String,
    pub stop_loss_summary: String,
    pub take_profit_summary: String,
    pub cancel_summary: String,
    pub position_plan_summary: SecurityApprovalBriefPositionPlanSummary,
    pub required_next_actions: Vec<String>,
    pub final_recommendation: String,
    pub recommended_review_action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub master_scorecard_summary: Option<SecurityApprovalBriefMasterScorecardSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_grade_summary: Option<SecurityApprovalBriefModelGradeSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_governance_summary: Option<SecurityApprovalBriefModelGovernanceSummary>,
    pub evidence_hash: String,
    pub governance_hash: String,
    pub package_binding: SecurityApprovalBriefPackageBinding,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityApprovalBriefMasterScorecardSummary {
    pub document_type: String,
    pub master_scorecard_ref: String,
    pub scorecard_ref: String,
    pub scorecard_status: String,
    pub aggregation_status: String,
    pub master_score: f64,
    pub master_signal: String,
    pub profitability_effectiveness_score: f64,
    pub risk_resilience_score: f64,
    pub path_quality_score: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityApprovalBriefModelGradeSummary {
    pub model_grade: String,
    pub grade_reason: String,
    pub approval_consumption_mode: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityApprovalBriefModelGovernanceSummary {
    pub model_grade: String,
    pub grade_reason: String,
    pub approval_consumption_mode: String,
    pub shadow_observation_count: usize,
    pub shadow_consistency_status: String,
    pub shadow_window_count: usize,
    pub oot_stability_status: String,
    pub window_consistency_status: String,
    pub promotion_blockers: Vec<String>,
    pub promotion_evidence_notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityApprovalBriefPositionPlanSummary {
    pub position_plan_status: String,
    pub risk_budget_summary: String,
    pub entry_summary: String,
    pub add_summary: String,
    pub stop_loss_summary: String,
    pub take_profit_summary: String,
    pub cancel_summary: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityApprovalBriefPackageBinding {
    pub artifact_role: String,
    pub brief_contract_version: String,
    pub decision_ref: String,
    pub approval_ref: String,
    pub decision_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SecurityApprovalBriefBuildInput {
    pub scene_name: String,
    pub generated_at: String,
    pub decision_id: String,
    pub decision_ref: String,
    pub approval_ref: String,
    pub approval_status: String,
    pub evidence_hash: String,
    pub governance_hash: String,
}

// 2026-04-02 CST: 这里集中生成正式审批简报文档，原因是审批阅读对象需要比普通接口摘要更稳定、更完整；
// 目的：让提交审批后产生的 brief 直接成为正式工件，而不是后续再临时拼接第二份“给人看的版本”。
pub fn build_security_decision_approval_brief(
    committee: &SecurityDecisionCommitteeResult,
    position_plan: &SecurityPositionPlan,
    input: &SecurityApprovalBriefBuildInput,
) -> SecurityDecisionApprovalBrief {
    let risk_budget_summary = format!("风险预算 {:.2}%", position_plan.risk_budget_pct * 100.0);
    let entry_summary = format!(
        "首仓 {:.2}%：{}",
        position_plan.starter_gross_pct * 100.0,
        position_plan.entry_plan.trigger_condition
    );
    let add_summary = format!(
        "加仓上限 {:.2}%：{}",
        position_plan.max_gross_pct * 100.0,
        position_plan.add_plan.trigger_condition
    );
    let stop_loss_summary = format!(
        "止损 {:.2}%：{}",
        position_plan.stop_loss_plan.stop_loss_pct * 100.0,
        position_plan.stop_loss_plan.hard_stop_condition
    );
    let take_profit_summary = format!(
        "止盈 {:.2}% / {:.2}%：{}",
        position_plan.take_profit_plan.first_target_pct * 100.0,
        position_plan.take_profit_plan.second_target_pct * 100.0,
        position_plan.take_profit_plan.partial_exit_rule
    );
    let cancel_summary = position_plan.cancel_conditions.join("；");
    let gate_summary: Vec<String> = committee
        .risk_gates
        .iter()
        .map(|gate| format!("{}:{}:{}", gate.gate_name, gate.result, gate.reason))
        .collect();
    let gate_outcome_summary: Vec<String> = committee
        .risk_gates
        .iter()
        .map(|gate| format!("{} -> {}", gate.gate_name, gate.result))
        .collect();
    let confidence_band = classify_confidence_band(committee.decision_card.confidence_score);
    let recommended_review_action =
        recommend_review_action(&committee.decision_card.status, &input.approval_status);

    SecurityDecisionApprovalBrief {
        brief_id: format!("brief-{}", input.decision_id),
        contract_version: "security_approval_brief.v1".to_string(),
        document_type: "security_approval_brief".to_string(),
        generated_at: normalize_generated_at(&input.generated_at),
        scene_name: input.scene_name.clone(),
        decision_id: input.decision_id.clone(),
        decision_ref: input.decision_ref.clone(),
        approval_ref: input.approval_ref.clone(),
        symbol: committee.symbol.clone(),
        analysis_date: committee.analysis_date.clone(),
        decision_status: committee.decision_card.status.clone(),
        approval_status: input.approval_status.clone(),
        committee_status: committee.decision_card.status.clone(),
        direction: committee.decision_card.direction.clone(),
        confidence_score: committee.decision_card.confidence_score,
        confidence_band,
        executive_summary: committee.decision_card.final_recommendation.clone(),
        bull_summary: committee.bull_case.thesis_points.clone(),
        bear_summary: committee.bear_case.thesis_points.clone(),
        core_supporting_points: committee.bull_case.thesis_points.clone(),
        core_risks: committee.bear_case.thesis_points.clone(),
        gate_summary,
        gate_outcome_summary,
        position_summary: committee.decision_card.position_size_suggestion.clone(),
        risk_budget_summary: risk_budget_summary.clone(),
        entry_summary: entry_summary.clone(),
        add_summary: add_summary.clone(),
        stop_loss_summary: stop_loss_summary.clone(),
        take_profit_summary: take_profit_summary.clone(),
        cancel_summary: cancel_summary.clone(),
        position_plan_summary: SecurityApprovalBriefPositionPlanSummary {
            position_plan_status: position_plan.plan_status.clone(),
            risk_budget_summary,
            entry_summary,
            add_summary,
            stop_loss_summary,
            take_profit_summary,
            cancel_summary,
        },
        required_next_actions: committee.decision_card.required_next_actions.clone(),
        final_recommendation: committee.decision_card.final_recommendation.clone(),
        recommended_review_action,
        master_scorecard_summary: None,
        model_grade_summary: None,
        model_governance_summary: None,
        evidence_hash: input.evidence_hash.clone(),
        governance_hash: input.governance_hash.clone(),
        package_binding: SecurityApprovalBriefPackageBinding {
            artifact_role: "approval_brief".to_string(),
            brief_contract_version: "security_approval_brief.v1".to_string(),
            decision_ref: input.decision_ref.clone(),
            approval_ref: input.approval_ref.clone(),
            decision_id: input.decision_id.clone(),
        },
    }
}

pub fn build_model_grade_summary(
    model_grade: &str,
    grade_reason: &str,
) -> SecurityApprovalBriefModelGradeSummary {
    // 2026-04-11 CST: Centralize approval consumption semantics for model grades,
    // because P5 requires approval objects to distinguish champion, shadow, and
    // candidate quant usage without each caller re-implementing grade mapping.
    // Purpose: keep approval brief and decision package aligned on one grade policy.
    let approval_consumption_mode = match model_grade {
        "champion" => "full_release_quant_context",
        "shadow" => "reference_only_quant_context",
        _ => "governance_only_quant_context",
    };
    SecurityApprovalBriefModelGradeSummary {
        model_grade: model_grade.to_string(),
        grade_reason: grade_reason.to_string(),
        approval_consumption_mode: approval_consumption_mode.to_string(),
    }
}

pub fn build_model_governance_summary(
    model_grade: &str,
    grade_reason: &str,
    shadow_observation_count: usize,
    shadow_consistency_status: &str,
    shadow_window_count: usize,
    oot_stability_status: &str,
    window_consistency_status: &str,
    promotion_blockers: &[String],
    promotion_evidence_notes: &[String],
) -> SecurityApprovalBriefModelGovernanceSummary {
    // 2026-04-11 CST: Add a richer governance summary object, because P6 needs
    // approval readers to see not only the grade but also why the model is still
    // blocked or releasable.
    // Purpose: keep approval brief, audit, and package consumers aligned on one governance explanation.
    let grade_summary = build_model_grade_summary(model_grade, grade_reason);
    SecurityApprovalBriefModelGovernanceSummary {
        model_grade: grade_summary.model_grade,
        grade_reason: grade_summary.grade_reason,
        approval_consumption_mode: grade_summary.approval_consumption_mode,
        shadow_observation_count,
        shadow_consistency_status: shadow_consistency_status.to_string(),
        shadow_window_count,
        oot_stability_status: oot_stability_status.to_string(),
        window_consistency_status: window_consistency_status.to_string(),
        promotion_blockers: promotion_blockers.to_vec(),
        promotion_evidence_notes: promotion_evidence_notes.to_vec(),
    }
}

fn classify_confidence_band(score: f64) -> String {
    if score >= 0.78 {
        "high".to_string()
    } else if score >= 0.58 {
        "medium".to_string()
    } else {
        "guarded".to_string()
    }
}

fn recommend_review_action(decision_status: &str, approval_status: &str) -> String {
    match (decision_status, approval_status) {
        ("blocked", _) => "request_more_evidence_or_reject".to_string(),
        ("needs_more_evidence", _) => "request_more_evidence".to_string(),
        (_, "NeedsMoreEvidence") => "request_more_evidence".to_string(),
        _ => "approve_with_standard_review".to_string(),
    }
}

fn normalize_generated_at(value: &str) -> String {
    if value.trim().is_empty() {
        Utc::now().to_rfc3339()
    } else {
        value.trim().to_string()
    }
}

pub fn build_master_scorecard_summary(
    master_scorecard: &crate::ops::stock::security_master_scorecard::SecurityMasterScorecardDocument,
) -> SecurityApprovalBriefMasterScorecardSummary {
    // 2026-04-11 CST: 这里新增审批简报里的总卡摘要映射，原因是投决会摘要需要能直接消费正式盈利质量总卡，
    // 但不应该把整份总卡文档原样塞进 brief 里导致阅读负担和合同膨胀。
    // 目的：把审批阅读最需要的总卡关键信号、总分与三条子分压缩成稳定摘要对象。
    SecurityApprovalBriefMasterScorecardSummary {
        document_type: master_scorecard.document_type.clone(),
        master_scorecard_ref: master_scorecard.master_scorecard_id.clone(),
        scorecard_ref: master_scorecard.scorecard_ref.clone(),
        scorecard_status: master_scorecard.scorecard_status.clone(),
        aggregation_status: master_scorecard.aggregation_status.clone(),
        master_score: master_scorecard.master_score,
        master_signal: master_scorecard.master_signal.clone(),
        profitability_effectiveness_score: master_scorecard.profitability_effectiveness_score,
        risk_resilience_score: master_scorecard.risk_resilience_score,
        path_quality_score: master_scorecard.path_quality_score,
    }
}
