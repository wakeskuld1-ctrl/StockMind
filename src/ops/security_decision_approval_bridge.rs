use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ops::stock::security_decision_approval_brief::{
    SecurityApprovalBriefBuildInput, SecurityDecisionApprovalBrief,
    build_security_decision_approval_brief,
};
use crate::ops::stock::security_legacy_committee_compat::LegacySecurityDecisionCommitteeResult as SecurityDecisionCommitteeResult;
use crate::ops::stock::security_position_plan::{
    SecurityPositionPlan, SecurityPositionPlanBuildInput, build_security_position_plan,
};

// 2026-04-02 CST: 这里定义桥接输入，原因是证券投决结果进入审批主线时，还需要审批规则参数和时间锚点；
// 目的：把桥接层所需的最小策略参数集中收口，避免 submit Tool 内部到处散着默认值。
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityDecisionApprovalBridgeOptions {
    pub scene_name: String,
    pub created_at: String,
    pub min_approvals: u8,
    pub require_risk_signoff: bool,
}

// 2026-04-02 CST: 这里定义桥接结果，原因是提交层需要一次拿到审批卡、审批请求、事件文件和审计起始记录；
// 目的：让后续持久化层只关注写文件，不再重复做证券到审批语义映射。
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityDecisionApprovalBridgeResult {
    pub approval_brief: SecurityDecisionApprovalBrief,
    pub decision_card: PersistedDecisionCard,
    pub approval_request: PersistedApprovalRequest,
    pub position_plan: SecurityPositionPlan,
    pub approval_events: Vec<serde_json::Value>,
    pub audit_record: PersistedDecisionAuditRecord,
    pub decision_ref: String,
    pub approval_ref: String,
}

// 2026-04-02 CST: 这里定义兼容私有审批主线的证券决策卡，原因是主仓不能直接依赖私有 crate，但要写出同构 JSON；
// 目的：在不打破仓库边界的前提下，让私有审批命令继续消费标准对象。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedDecisionCard {
    pub decision_ref: String,
    pub decision_id: String,
    pub scene_name: String,
    pub asset_id: String,
    pub instrument_type: String,
    pub strategy_type: String,
    pub horizon: String,
    // 2026-04-09 CST: 这里补齐正式动作语义落盘，原因是治理链后续需要直接读取“最终建议动作”，不能只看旧 direction；
    // 目的：让 approval、verify、复盘与 scorecard 都能围绕统一动作字段对齐。
    pub recommendation_action: String,
    // 2026-04-09 CST: 这里补齐正式暴露方向落盘，原因是 exposure_side 已经成为 decision_card 的主语义之一；
    // 目的：让私有审批映射与 scorecard 对齐时有明确 side 字段，而不是回退到含混旧 direction。
    pub exposure_side: String,
    pub direction: PersistedDecisionDirection,
    pub status: PersistedDecisionStatus,
    pub confidence_score: f64,
    pub expected_return_range: PersistedExpectedReturnRange,
    pub downside_risk: PersistedDownsideRisk,
    pub position_size_suggestion: PersistedPositionSizeSuggestion,
    pub key_supporting_points: Vec<String>,
    pub key_risks: Vec<String>,
    pub invalidation_conditions: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub portfolio_impact: PersistedPortfolioImpact,
    pub approval: PersistedApprovalState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PersistedDecisionDirection {
    Long,
    Short,
    Hedge,
    NoTrade,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PersistedDecisionStatus {
    Draft,
    NeedsMoreEvidence,
    Blocked,
    ReadyForReview,
    Approved,
    Rejected,
    ApprovedWithOverride,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedExpectedReturnRange {
    pub low: f64,
    pub base: f64,
    pub high: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedDownsideRisk {
    pub soft_stop: f64,
    pub hard_stop: f64,
    pub tail_risk_note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedPositionSizeSuggestion {
    pub gross_pct: f64,
    pub max_pct: f64,
    pub sizing_basis: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedPortfolioImpact {
    pub sector_exposure_delta: Option<f64>,
    pub factor_exposure_note: Option<String>,
    pub liquidity_class: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedApprovalState {
    pub required: bool,
    pub approval_state: PersistedApprovalStatus,
    pub approval_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PersistedApprovalStatus {
    Pending,
    Approved,
    Rejected,
    ApprovedWithOverride,
    NeedsMoreEvidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedApprovalRequest {
    pub approval_ref: String,
    pub decision_id: String,
    pub scene_name: String,
    pub status: PersistedApprovalStatus,
    pub created_at: String,
    pub decision_ref: Option<String>,
    pub evidence_hash: Option<String>,
    pub governance_hash: Option<String>,
    pub min_approvals: u8,
    pub approved_reviewers: Vec<String>,
    pub approved_signatures: Vec<serde_json::Value>,
    pub enforce_role_matrix: bool,
    pub require_risk_signoff: bool,
    pub auto_reject_recommended: bool,
    pub auto_reject_reason: Option<String>,
    pub auto_reject_gate_names: Vec<String>,
    pub recovery_action_required: bool,
    pub recovery_actions: Vec<String>,
    // 2026-04-08 CST: 这里补入审批请求对仓位计划的正式绑定，原因是 Task 2 需要让 approval_request 自己声明被审批的仓位计划；
    // 目的：把计划引用、路径、版本与方向摘要直接纳入审批对象，而不是继续只靠 package 间接引用。
    #[serde(default)]
    pub position_plan_binding: Option<PersistedApprovalPositionPlanBinding>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedApprovalPositionPlanBinding {
    pub position_plan_ref: String,
    pub position_plan_path: String,
    pub position_plan_contract_version: String,
    pub position_plan_sha256: String,
    pub plan_status: String,
    pub plan_direction: String,
    pub gross_limit_summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedDecisionAuditRecord {
    pub event_type: PersistedAuditEventType,
    pub timestamp: String,
    pub decision_id: String,
    pub decision_ref: Option<String>,
    pub approval_ref: Option<String>,
    pub evidence_hash: Option<String>,
    pub governance_hash: Option<String>,
    pub decision_status: Option<PersistedDecisionStatus>,
    pub approval_status: Option<PersistedApprovalStatus>,
    pub reviewer: Option<String>,
    pub reviewer_role: Option<String>,
    pub approval_action: Option<String>,
    pub notes: Option<String>,
    pub override_reason: Option<String>,
    pub decision_version: Option<u32>,
    pub signature_key_id: Option<String>,
    pub signature_algorithm: Option<String>,
    pub signature_path: Option<String>,
    pub signed_payload_sha256: Option<String>,
    pub signed_contract_version: Option<String>,
    pub prev_hash: Option<String>,
    pub record_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistedAuditEventType {
    DecisionPersisted,
}

// 2026-04-02 CST: 这里实现证券投决到审批对象的桥接，原因是我们要把主仓 committee 结果正式送进私有审批主线；
// 目的：把所有状态映射、默认审批规则和兼容对象构造集中在一处，避免 submit 层掺杂过多业务语义。
pub fn bridge_security_decision_to_approval(
    committee: &SecurityDecisionCommitteeResult,
    options: &SecurityDecisionApprovalBridgeOptions,
) -> SecurityDecisionApprovalBridgeResult {
    let decision_ref = create_runtime_ref("decision_ref");
    let approval_ref = create_runtime_ref("approval_ref");
    let governance_hash = build_governance_hash(committee, options);
    let decision_status = map_decision_status(&committee.decision_card.status);
    let approval_status = map_initial_approval_status(&committee.decision_card.status);
    let auto_reject_gate_names = collect_blocking_gate_names(committee);
    let auto_reject_recommended = !auto_reject_gate_names.is_empty();
    let auto_reject_reason = if auto_reject_recommended {
        Some(format!(
            "存在阻断性闸门未通过: {}",
            auto_reject_gate_names.join("、")
        ))
    } else {
        None
    };

    let expected_return = parse_percent_range(&committee.decision_card.expected_return_range);
    let downside_risk = parse_percent(&committee.decision_card.downside_risk).unwrap_or(0.0);
    let position_size = map_position_size(&committee.decision_card.position_size_suggestion);

    let mut key_risks = committee.bear_case.thesis_points.clone();
    for gate in &committee.risk_gates {
        if gate.result != "pass" {
            key_risks.push(format!("{}: {}", gate.gate_name, gate.reason));
        }
    }
    dedupe_strings(&mut key_risks);

    let decision_card = PersistedDecisionCard {
        decision_ref: decision_ref.clone(),
        decision_id: committee.decision_card.decision_id.clone(),
        scene_name: options.scene_name.clone(),
        asset_id: committee.symbol.clone(),
        instrument_type: "equity".to_string(),
        strategy_type: "security_decision_committee".to_string(),
        horizon: "swing".to_string(),
        recommendation_action: committee.decision_card.recommendation_action.clone(),
        exposure_side: committee.decision_card.exposure_side.clone(),
        direction: map_direction(&committee.decision_card.exposure_side),
        status: decision_status.clone(),
        confidence_score: committee.decision_card.confidence_score,
        expected_return_range: expected_return,
        downside_risk: PersistedDownsideRisk {
            soft_stop: downside_risk,
            hard_stop: downside_risk,
            tail_risk_note: Some(committee.bear_case.headline.clone()),
        },
        position_size_suggestion: position_size,
        key_supporting_points: committee.bull_case.thesis_points.clone(),
        key_risks,
        invalidation_conditions: committee.bull_case.invalidation_conditions.clone(),
        evidence_refs: vec![committee.evidence_bundle.evidence_hash.clone()],
        portfolio_impact: PersistedPortfolioImpact {
            sector_exposure_delta: None,
            factor_exposure_note: None,
            liquidity_class: Some("unknown".to_string()),
        },
        approval: PersistedApprovalState {
            required: true,
            approval_state: approval_status.clone(),
            approval_ref: Some(approval_ref.clone()),
        },
    };

    let approval_request = PersistedApprovalRequest {
        approval_ref: approval_ref.clone(),
        decision_id: decision_card.decision_id.clone(),
        scene_name: options.scene_name.clone(),
        status: approval_status.clone(),
        created_at: normalize_created_at(&options.created_at),
        decision_ref: Some(decision_ref.clone()),
        evidence_hash: Some(committee.evidence_bundle.evidence_hash.clone()),
        governance_hash: Some(governance_hash.clone()),
        min_approvals: options.min_approvals.max(1),
        approved_reviewers: Vec::new(),
        approved_signatures: Vec::new(),
        enforce_role_matrix: true,
        require_risk_signoff: options.require_risk_signoff,
        auto_reject_recommended,
        auto_reject_reason,
        auto_reject_gate_names,
        recovery_action_required: decision_card.status == PersistedDecisionStatus::Blocked,
        recovery_actions: if decision_card.status == PersistedDecisionStatus::Blocked {
            committee.decision_card.required_next_actions.clone()
        } else {
            Vec::new()
        },
        position_plan_binding: None,
    };

    let position_plan = build_security_position_plan(
        committee,
        &SecurityPositionPlanBuildInput {
            decision_id: decision_card.decision_id.clone(),
            decision_ref: decision_ref.clone(),
            approval_ref: approval_ref.clone(),
        },
    );
    let approval_brief = build_security_decision_approval_brief(
        committee,
        &position_plan,
        &SecurityApprovalBriefBuildInput {
            scene_name: options.scene_name.clone(),
            generated_at: approval_request.created_at.clone(),
            decision_id: decision_card.decision_id.clone(),
            decision_ref: decision_ref.clone(),
            approval_ref: approval_ref.clone(),
            approval_status: format!("{:?}", approval_request.status),
            evidence_hash: committee.evidence_bundle.evidence_hash.clone(),
            governance_hash: governance_hash.clone(),
        },
    );

    let audit_record = PersistedDecisionAuditRecord {
        event_type: PersistedAuditEventType::DecisionPersisted,
        timestamp: approval_request.created_at.clone(),
        decision_id: decision_card.decision_id.clone(),
        decision_ref: Some(decision_ref.clone()),
        approval_ref: Some(approval_ref.clone()),
        evidence_hash: Some(committee.evidence_bundle.evidence_hash.clone()),
        governance_hash: Some(governance_hash),
        decision_status: Some(decision_status),
        approval_status: Some(approval_status),
        reviewer: None,
        reviewer_role: None,
        approval_action: None,
        notes: Some(committee.decision_card.final_recommendation.clone()),
        override_reason: None,
        decision_version: Some(1),
        signature_key_id: None,
        signature_algorithm: None,
        signature_path: None,
        signed_payload_sha256: None,
        signed_contract_version: None,
        prev_hash: None,
        record_hash: None,
    };

    SecurityDecisionApprovalBridgeResult {
        approval_brief,
        decision_card,
        approval_request,
        position_plan,
        approval_events: Vec::new(),
        audit_record,
        decision_ref,
        approval_ref,
    }
}

fn create_runtime_ref(prefix: &str) -> String {
    format!(
        "{prefix}:{}:{}",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    )
}

fn build_governance_hash(
    committee: &SecurityDecisionCommitteeResult,
    options: &SecurityDecisionApprovalBridgeOptions,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(committee.evidence_bundle.evidence_hash.as_bytes());
    hasher.update(committee.decision_card.decision_id.as_bytes());
    hasher.update(options.scene_name.as_bytes());
    hasher.update(options.created_at.as_bytes());
    format!("gov-{:x}", hasher.finalize())
}

fn normalize_created_at(value: &str) -> String {
    if value.trim().is_empty() {
        Utc::now().to_rfc3339()
    } else {
        value.trim().to_string()
    }
}

fn collect_blocking_gate_names(committee: &SecurityDecisionCommitteeResult) -> Vec<String> {
    committee
        .risk_gates
        .iter()
        .filter(|gate| gate.blocking && gate.result == "fail")
        .map(|gate| gate.gate_name.clone())
        .collect()
}

fn map_direction(direction: &str) -> PersistedDecisionDirection {
    // 2026-04-11 CST: 这里补大小写无关的方向映射，原因是当前证券主链不同对象对 `exposure_side`
    // 的大小写口径并不完全一致，旧实现只认全小写会把真实 long/short 误落成 NoTrade。
    // 目的：让审批桥接层稳定消费正式动作方向，而不是把字符串大小写漂移放大成业务语义错误。
    match direction.trim().to_ascii_lowercase().as_str() {
        "long" => PersistedDecisionDirection::Long,
        "short" => PersistedDecisionDirection::Short,
        "hedge" => PersistedDecisionDirection::Hedge,
        _ => PersistedDecisionDirection::NoTrade,
    }
}

fn map_decision_status(status: &str) -> PersistedDecisionStatus {
    match status {
        "blocked" => PersistedDecisionStatus::Blocked,
        "needs_more_evidence" => PersistedDecisionStatus::NeedsMoreEvidence,
        "approved" => PersistedDecisionStatus::Approved,
        "rejected" => PersistedDecisionStatus::Rejected,
        "approved_with_override" => PersistedDecisionStatus::ApprovedWithOverride,
        _ => PersistedDecisionStatus::ReadyForReview,
    }
}

fn map_initial_approval_status(status: &str) -> PersistedApprovalStatus {
    match status {
        "blocked" => PersistedApprovalStatus::NeedsMoreEvidence,
        "approved" => PersistedApprovalStatus::Approved,
        "rejected" => PersistedApprovalStatus::Rejected,
        "approved_with_override" => PersistedApprovalStatus::ApprovedWithOverride,
        _ => PersistedApprovalStatus::Pending,
    }
}

fn parse_percent_range(value: &str) -> PersistedExpectedReturnRange {
    let values: Vec<f64> = value
        .split('-')
        .filter_map(|part| parse_percent(part))
        .collect();
    let low = values.first().copied().unwrap_or(0.0);
    let high = values.get(1).copied().unwrap_or(low);
    PersistedExpectedReturnRange {
        low,
        base: (low + high) / 2.0,
        high,
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

fn map_position_size(value: &str) -> PersistedPositionSizeSuggestion {
    match value {
        "starter" => PersistedPositionSizeSuggestion {
            gross_pct: 0.10,
            max_pct: 0.15,
            sizing_basis: "starter".to_string(),
        },
        "pilot" => PersistedPositionSizeSuggestion {
            gross_pct: 0.05,
            max_pct: 0.08,
            sizing_basis: "pilot".to_string(),
        },
        _ => PersistedPositionSizeSuggestion {
            gross_pct: 0.0,
            max_pct: 0.0,
            sizing_basis: "none".to_string(),
        },
    }
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
