use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_independent_advice::SecurityIndependentAdviceDocument;
use crate::ops::stock::security_legacy_committee_compat::{
    LegacySecurityDecisionCommitteeError as SecurityDecisionCommitteeError,
    LegacySecurityDecisionCommitteeRequest as SecurityDecisionCommitteeRequest,
    LegacySecurityDecisionCommitteeResult as SecurityDecisionCommitteeResult,
    run_security_decision_committee_legacy_compat,
};
use crate::ops::stock::security_scorecard::{
    SecurityScorecardBuildInput, SecurityScorecardDocument, SecurityScorecardError,
    build_security_scorecard,
};

// 2026-04-09 CST: 这里新增主席裁决请求合同，原因是 Task 1 要把“最终正式动作”从投委会线中拆出来，
// 目的：让主席线拥有独立 Tool 入口，后续 package / verify / audit 都可以围绕这条线接入。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityChairResolutionRequest {
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
    #[serde(default = "default_stop_loss_pct")]
    pub stop_loss_pct: f64,
    #[serde(default = "default_target_return_pct")]
    pub target_return_pct: f64,
    #[serde(default = "default_min_risk_reward_ratio")]
    pub min_risk_reward_ratio: f64,
    #[serde(default = "default_created_at")]
    pub created_at: String,
    #[serde(default)]
    pub scorecard_model_path: Option<String>,
    #[serde(default)]
    pub independent_advice_document: Option<SecurityIndependentAdviceDocument>,
    // 2026-04-13 CST: 这里补可选独立建议输入，原因是当前体系已明确为“议会制 + 数据建议 + 大模型独立建议 + 主席仲裁”。
    // 目的：第一版先把独立建议冻结成正式结构化输入，避免主席层自己补事实或自由发挥。
    #[serde(default)]
    pub independent_advice: Option<SecurityChairIndependentAdviceInput>,
}

// 2026-04-13 CST: 这里新增主席层可选独立建议输入，原因是主席仲裁要能读取外部独立建议而不破坏现有主链。
// 目的：后续真正接上大模型独立建议 Tool 时，可以直接复用这份合同，而不是推翻主席层边界。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityChairIndependentAdviceInput {
    pub source_type: String,
    pub suggested_stance: String,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub rationale: Option<String>,
    #[serde(default)]
    pub key_risks: Vec<String>,
}

// 2026-04-13 CST: 这里新增主席动作枚举，原因是用户明确主席不仅能批准/否决，还必须能退回补件后重审。
// 目的：把流程动作正式对象化，避免审批动作与交易建议混成一个自由文本字段。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityChairFinalAction {
    Approve,
    ConditionalApprove,
    Reject,
    ReturnForRevision,
    Defer,
}

impl SecurityChairFinalAction {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::ConditionalApprove => "conditional_approve",
            Self::Reject => "reject",
            Self::ReturnForRevision => "return_for_revision",
            Self::Defer => "defer",
        }
    }
}

// 2026-04-13 CST: 这里新增冲突等级枚举，原因是主席裁决必须先识别冲突，再决定能否审批。
// 目的：把 low/moderate/high 固化为正式可测试合同，而不是散在解释文本里。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityChairConflictLevel {
    Low,
    Moderate,
    High,
}

impl SecurityChairConflictLevel {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low_conflict",
            Self::Moderate => "moderate_conflict",
            Self::High => "high_conflict",
        }
    }
}

// 2026-04-13 CST: 这里新增证据充分度枚举，原因是主席层必须拥有“证据不足时退回补件”的正式流程权力。
// 目的：让 sufficient/partial/insufficient 成为稳定字段，后续可以直接进 verify/audit/复盘链。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityChairEvidenceSufficiency {
    Sufficient,
    Partial,
    Insufficient,
}

impl SecurityChairEvidenceSufficiency {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Sufficient => "sufficient",
            Self::Partial => "partial",
            Self::Insufficient => "insufficient",
        }
    }
}

// 2026-04-13 CST: 这里新增证据映射合同，原因是主席层禁止创造新事实，所有关键裁决必须回指正式输入。
// 目的：为后续审计、回放和反幻觉校验保留最小可追溯证据链。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityChairEvidenceMapping {
    pub source: String,
    pub signal: String,
    pub influence: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityChairResolutionDocument {
    pub chair_resolution_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub symbol: String,
    pub analysis_date: String,
    pub decision_id: String,
    pub committee_session_ref: String,
    pub master_scorecard_ref: String,
    // 2026-04-13 CST: 这里新增正式最终审批动作与流程字段，原因是主席层现在不只给“交易动作”，还要给“审批动作”。
    // 目的：把流程决策、补件机制和证据状态显式暴露给上层，而不是继续隐含在 reasoning 文本里。
    pub final_action: String,
    pub final_stance: String,
    pub conflict_level: String,
    pub evidence_sufficiency: String,
    pub revision_required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_to_stage: Option<String>,
    pub required_materials: Vec<String>,
    pub blocking_reasons: Vec<String>,
    pub evidence_mapping: Vec<SecurityChairEvidenceMapping>,
    pub selected_action: String,
    pub selected_exposure_side: String,
    pub chair_reasoning: String,
    pub why_followed_quant: String,
    pub why_followed_committee: String,
    pub override_reason: Option<String>,
    pub execution_constraints: Vec<String>,
    pub final_confidence: f64,
    pub signed_off_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityChairResolutionResult {
    pub committee_result: SecurityDecisionCommitteeResult,
    pub scorecard: SecurityScorecardDocument,
    pub chair_resolution: SecurityChairResolutionDocument,
}

#[derive(Debug, Error)]
pub enum SecurityChairResolutionError {
    #[error("security chair resolution committee preparation failed: {0}")]
    Committee(#[from] SecurityDecisionCommitteeError),
    #[error("security chair resolution scorecard preparation failed: {0}")]
    Scorecard(#[from] SecurityScorecardError),
}

#[derive(Debug, Clone, PartialEq)]
struct ChairArbitrationOutcome {
    final_action: SecurityChairFinalAction,
    final_stance: String,
    conflict_level: SecurityChairConflictLevel,
    evidence_sufficiency: SecurityChairEvidenceSufficiency,
    revision_required: bool,
    return_to_stage: Option<String>,
    required_materials: Vec<String>,
    blocking_reasons: Vec<String>,
    evidence_mapping: Vec<SecurityChairEvidenceMapping>,
    override_reason: Option<String>,
    final_confidence: f64,
}

pub fn security_chair_resolution(
    request: &SecurityChairResolutionRequest,
) -> Result<SecurityChairResolutionResult, SecurityChairResolutionError> {
    let committee_request = SecurityDecisionCommitteeRequest {
        symbol: request.symbol.clone(),
        market_symbol: request.market_symbol.clone(),
        sector_symbol: request.sector_symbol.clone(),
        market_profile: request.market_profile.clone(),
        sector_profile: request.sector_profile.clone(),
        as_of_date: request.as_of_date.clone(),
        lookback_days: request.lookback_days,
        disclosure_limit: request.disclosure_limit,
        stop_loss_pct: request.stop_loss_pct,
        target_return_pct: request.target_return_pct,
        min_risk_reward_ratio: request.min_risk_reward_ratio,
        // 2026-04-14 CST: 这里补齐 committee 合同新增字段，原因是主席链当前没有单独传 external proxy 输入；
        // 目的：先用默认空值让结构兼容，再在后续专门整理主席链与外部代理链的正式接入。
        external_proxy_inputs: None,
    };
    let committee_result = run_security_decision_committee_legacy_compat(&committee_request)?;
    let scorecard = build_security_scorecard(
        &committee_result,
        &SecurityScorecardBuildInput {
            generated_at: request.created_at.clone(),
            decision_id: committee_result.decision_card.decision_id.clone(),
            decision_ref: committee_result.decision_card.decision_id.clone(),
            approval_ref: format!("chair-only-{}", committee_result.decision_card.decision_id),
            scorecard_model_path: request.scorecard_model_path.clone(),
        },
    )?;
    let independent_advice = materialize_independent_advice(
        request,
        &committee_result.analysis_date,
        &request.created_at,
    );
    let chair_resolution = build_security_chair_resolution(
        &committee_result,
        &scorecard,
        independent_advice.as_ref(),
        &request.created_at,
    );

    Ok(SecurityChairResolutionResult {
        committee_result,
        scorecard,
        chair_resolution,
    })
}

pub fn build_security_chair_resolution(
    committee_result: &SecurityDecisionCommitteeResult,
    scorecard: &SecurityScorecardDocument,
    independent_advice: Option<&SecurityIndependentAdviceDocument>,
    generated_at: &str,
) -> SecurityChairResolutionDocument {
    let selected_action = committee_result.decision_card.recommendation_action.clone();
    let selected_exposure_side = committee_result.decision_card.exposure_side.clone();
    let signed_off_at = normalize_created_at(generated_at);
    let committee_session_ref = committee_result.committee_session_ref.clone();
    let master_scorecard_ref = scorecard.scorecard_id.clone();
    let arbitration = derive_arbitration_outcome(
        committee_result,
        scorecard,
        independent_advice,
        &selected_action,
    );
    let execution_constraints =
        build_execution_constraints(committee_result, scorecard, &selected_action, &arbitration);

    SecurityChairResolutionDocument {
        chair_resolution_id: format!("chair-{}", committee_result.decision_card.decision_id),
        contract_version: "security_chair_resolution.v2".to_string(),
        document_type: "security_chair_resolution".to_string(),
        generated_at: signed_off_at.clone(),
        symbol: committee_result.symbol.clone(),
        analysis_date: committee_result.analysis_date.clone(),
        decision_id: committee_result.decision_card.decision_id.clone(),
        committee_session_ref,
        master_scorecard_ref,
        final_action: arbitration.final_action.as_str().to_string(),
        final_stance: arbitration.final_stance.clone(),
        conflict_level: arbitration.conflict_level.as_str().to_string(),
        evidence_sufficiency: arbitration.evidence_sufficiency.as_str().to_string(),
        revision_required: arbitration.revision_required,
        return_to_stage: arbitration.return_to_stage.clone(),
        required_materials: arbitration.required_materials.clone(),
        blocking_reasons: arbitration.blocking_reasons.clone(),
        evidence_mapping: arbitration.evidence_mapping.clone(),
        selected_action: selected_action.clone(),
        selected_exposure_side,
        chair_reasoning: build_chair_reasoning(
            committee_result,
            scorecard,
            independent_advice,
            &arbitration,
        ),
        why_followed_quant: build_quant_reason(scorecard),
        why_followed_committee: build_committee_reason(committee_result, &arbitration),
        override_reason: arbitration.override_reason.clone(),
        execution_constraints,
        final_confidence: arbitration.final_confidence,
        signed_off_at,
    }
}

// 2026-04-14 CST: 这里补回 submit_approval 依赖的轻量主席动作映射，原因是当前标准提交流程只需要一个保守的 entry-layer 门禁动作；
// 目的：在不反向耦合完整主席 runtime 的前提下，为 position_plan 第二阶段提供稳定、可解释的正式动作口径。
pub fn derive_training_guarded_chair_action(
    committee_result: &SecurityDecisionCommitteeResult,
    scorecard: &SecurityScorecardDocument,
) -> String {
    if scorecard.score_status != "ready" {
        return "return_for_revision".to_string();
    }

    match committee_result
        .decision_card
        .recommendation_action
        .as_str()
    {
        "buy" => "approve".to_string(),
        "hold" => "conditional_approve".to_string(),
        "reduce" => "conditional_approve".to_string(),
        "abstain" => "defer".to_string(),
        _ => "reject".to_string(),
    }
}

fn build_quant_reason(scorecard: &SecurityScorecardDocument) -> String {
    if scorecard.score_status == "ready" {
        return format!(
            "主席已参考量化线，量化立场 `{}` / 量化信号 `{}` 已完成正式打分。",
            scorecard.quant_stance, scorecard.quant_signal
        );
    }

    format!(
        "主席未将量化线作为唯一放行依据，原因是 scorecard 当前状态为 `{}`，并保留了 {} 条限制说明。",
        scorecard.score_status,
        scorecard.limitations.len()
    )
}

fn build_committee_reason(
    committee_result: &SecurityDecisionCommitteeResult,
    arbitration: &ChairArbitrationOutcome,
) -> String {
    format!(
        "主席参考投委会多数票 `{}`（{} 票），并读取风控席状态 `{}`，最终形成 `{}` 流程动作。",
        committee_result.vote_tally.majority_vote,
        committee_result.vote_tally.majority_count,
        committee_result.risk_veto.status,
        arbitration.final_action.as_str()
    )
}

fn build_chair_reasoning(
    committee_result: &SecurityDecisionCommitteeResult,
    scorecard: &SecurityScorecardDocument,
    independent_advice: Option<&SecurityIndependentAdviceDocument>,
    arbitration: &ChairArbitrationOutcome,
) -> String {
    let independent_summary = independent_advice
        .map(|advice| {
            format!(
                "独立建议 `{}`（source_type=`{}`）已纳入仲裁。",
                advice.suggested_stance, advice.source_type
            )
        })
        .unwrap_or_else(|| "本轮未提供独立建议输入。".to_string());
    format!(
        "主席在同读投委会线、量化线与独立建议后，正式签发 `{}` 流程动作，并对 `{}` 立场给出最终裁决；投委会多数票为 `{}`，量化线状态为 `{}`，冲突等级为 `{}`。{}",
        arbitration.final_action.as_str(),
        arbitration.final_stance,
        committee_result.vote_tally.majority_vote,
        scorecard.score_status,
        arbitration.conflict_level.as_str(),
        independent_summary
    )
}

fn build_execution_constraints(
    committee_result: &SecurityDecisionCommitteeResult,
    scorecard: &SecurityScorecardDocument,
    selected_action: &str,
    arbitration: &ChairArbitrationOutcome,
) -> Vec<String> {
    let mut constraints = Vec::new();
    constraints.push(format!(
        "主席批准流转前，必须遵守风险否决状态 `{}` 与最终流程动作 `{}`。",
        committee_result.risk_veto.status,
        arbitration.final_action.as_str()
    ));
    constraints.push(format!(
        "当前最终交易立场为 `{}`，对应委员会建议动作 `{selected_action}`。",
        arbitration.final_stance
    ));
    constraints.extend(
        committee_result
            .decision_card
            .required_next_actions
            .iter()
            .take(3)
            .cloned(),
    );
    constraints.extend(arbitration.required_materials.iter().take(3).cloned());
    constraints.extend(scorecard.limitations.iter().take(2).cloned());
    dedupe_strings(&mut constraints);
    constraints
}

// 2026-04-13 CST: 这里新增主席仲裁核心逻辑，原因是用户要求主席层从“简单签发器”升级为正式仲裁器，
// 并明确具备禁止审批、退回补件和冲突降级能力；目的：先以硬协议实现第一版裁决，不让主席层自由发挥创造新事实。
fn derive_arbitration_outcome(
    committee_result: &SecurityDecisionCommitteeResult,
    scorecard: &SecurityScorecardDocument,
    independent_advice: Option<&SecurityIndependentAdviceDocument>,
    selected_action: &str,
) -> ChairArbitrationOutcome {
    let mut required_materials = Vec::new();
    let mut blocking_reasons = Vec::new();
    let mut evidence_mapping = vec![
        SecurityChairEvidenceMapping {
            source: "committee_result".to_string(),
            signal: format!(
                "majority_vote={} risk_veto={}",
                committee_result.vote_tally.majority_vote, committee_result.risk_veto.status
            ),
            influence: "primary_governance_input".to_string(),
        },
        SecurityChairEvidenceMapping {
            source: "scorecard".to_string(),
            signal: format!(
                "score_status={} quant_signal={}",
                scorecard.score_status, scorecard.quant_signal
            ),
            influence: "quant_validation_input".to_string(),
        },
    ];

    let final_stance = normalize_final_stance(selected_action);
    let independent_conflict = independent_advice
        .map(|advice| {
            evidence_mapping.push(SecurityChairEvidenceMapping {
                source: format!("independent_advice:{}", advice.source_type),
                signal: format!("suggested_stance={}", advice.suggested_stance),
                influence: "independent_reference_input".to_string(),
            });
            let normalized = normalize_final_stance(&advice.suggested_stance);
            normalized != final_stance && normalized != "observe"
        })
        .unwrap_or(false);

    let conflict_level = if committee_result.risk_veto.status == "blocked" {
        SecurityChairConflictLevel::High
    } else if committee_result.vote_tally.majority_vote == "split" || independent_conflict {
        SecurityChairConflictLevel::High
    } else if committee_result.decision_card.confidence_score < 0.75 {
        SecurityChairConflictLevel::Moderate
    } else {
        SecurityChairConflictLevel::Low
    };

    let evidence_sufficiency = if committee_result.risk_veto.status == "blocked" {
        SecurityChairEvidenceSufficiency::Insufficient
    } else if committee_result.risk_veto.status == "needs_more_evidence"
        || scorecard.score_status != "ready"
    {
        SecurityChairEvidenceSufficiency::Partial
    } else {
        SecurityChairEvidenceSufficiency::Sufficient
    };

    let (final_action, return_to_stage, override_reason) =
        if committee_result.risk_veto.status == "blocked" {
            blocking_reasons.push(committee_result.risk_veto.reason.clone());
            (
                SecurityChairFinalAction::Reject,
                None,
                Some("risk_veto_blocked".to_string()),
            )
        } else if selected_action == "abstain"
            && scorecard.score_status != "ready"
            && independent_advice.is_none()
        {
            blocking_reasons.push("委员会未形成足够稳定的多数意见，主席暂缓签发。".to_string());
            (
                SecurityChairFinalAction::Defer,
                Some("security_decision_committee".to_string()),
                Some("committee_abstain".to_string()),
            )
        } else if committee_result.risk_veto.status == "needs_more_evidence" {
            required_materials.push("补充投委会要求的缺失证据与风险说明".to_string());
            blocking_reasons.push(committee_result.risk_veto.reason.clone());
            (
                SecurityChairFinalAction::ReturnForRevision,
                Some("security_decision_evidence_bundle".to_string()),
                Some("risk_veto_needs_more_evidence".to_string()),
            )
        } else if scorecard.score_status != "ready" {
            required_materials.push("补充量化评分卡可用模型或补齐缺失特征".to_string());
            blocking_reasons.push(format!(
                "scorecard 当前状态为 `{}`，不足以作为稳定放行依据。",
                scorecard.score_status
            ));
            (
                SecurityChairFinalAction::ReturnForRevision,
                Some("security_scorecard".to_string()),
                Some("scorecard_not_ready".to_string()),
            )
        } else if independent_conflict {
            required_materials.push("补充独立建议与数据建议冲突的解释材料".to_string());
            blocking_reasons
                .push("独立建议与委员会/量化立场存在高冲突，主席要求补件后重审。".to_string());
            (
                SecurityChairFinalAction::ReturnForRevision,
                Some("security_decision_committee".to_string()),
                Some("independent_advice_conflict".to_string()),
            )
        } else if committee_result.decision_card.confidence_score < 0.75 {
            required_materials.push("执行层只能按更保守仓位或附条件方式推进".to_string());
            (SecurityChairFinalAction::ConditionalApprove, None, None)
        } else {
            (SecurityChairFinalAction::Approve, None, None)
        };

    dedupe_strings(&mut required_materials);
    dedupe_strings(&mut blocking_reasons);

    ChairArbitrationOutcome {
        final_action: final_action.clone(),
        final_stance,
        conflict_level,
        evidence_sufficiency,
        revision_required: matches!(final_action, SecurityChairFinalAction::ReturnForRevision),
        return_to_stage,
        required_materials,
        blocking_reasons,
        evidence_mapping,
        override_reason,
        final_confidence: derive_final_confidence(
            committee_result.decision_card.confidence_score,
            scorecard,
            independent_advice,
        ),
    }
}

fn normalize_final_stance(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "buy" => "build".to_string(),
        "hold" => "hold".to_string(),
        "reduce" => "reduce".to_string(),
        "avoid" => "avoid".to_string(),
        "abstain" => "observe".to_string(),
        other => other.to_string(),
    }
}

fn derive_final_confidence(
    committee_confidence: f64,
    scorecard: &SecurityScorecardDocument,
    independent_advice: Option<&SecurityIndependentAdviceDocument>,
) -> f64 {
    let mut total = committee_confidence.clamp(0.0, 1.0);
    let mut weight = 1.0;

    if let Some(probability) = scorecard.success_probability {
        total += probability.clamp(0.0, 1.0);
        weight += 1.0;
    }
    if let Some(advice_confidence) = independent_advice.and_then(|advice| advice.confidence) {
        total += advice_confidence.clamp(0.0, 1.0);
        weight += 1.0;
    }

    (total / weight).clamp(0.0, 1.0)
}

// 2026-04-13 CST: 这里把旧嵌入式输入统一收口到正式独立建议文档，原因是方案B要让主席层优先消费标准 Tool 产物，同时保留旧调用兼容；
// 目的：让新旧两种调用路径最终都进入同一份独立建议对象，避免主席层维护两套判断分支。
fn materialize_independent_advice(
    request: &SecurityChairResolutionRequest,
    analysis_date: &str,
    generated_at: &str,
) -> Option<SecurityIndependentAdviceDocument> {
    if let Some(document) = request.independent_advice_document.as_ref() {
        return Some(document.clone());
    }

    request.independent_advice.as_ref().map(|advice| {
        let mut key_risks = advice
            .key_risks
            .iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>();
        if key_risks.is_empty() {
            key_risks.push("嵌入式独立建议未提供关键风险".to_string());
        }

        let rationale = advice
            .rationale
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        SecurityIndependentAdviceDocument {
            advice_id: format!(
                "embedded-independent-advice-{}-{}-{}",
                request.symbol,
                analysis_date,
                advice.source_type.trim()
            ),
            contract_version: "security_independent_advice.v1".to_string(),
            document_type: "security_independent_advice".to_string(),
            generated_at: normalize_created_at(generated_at),
            symbol: request.symbol.clone(),
            analysis_date: analysis_date.to_string(),
            source_type: advice.source_type.trim().to_string(),
            suggested_stance: advice.suggested_stance.trim().to_string(),
            confidence: advice.confidence.map(|value| value.clamp(0.0, 1.0)),
            rationale: rationale.clone(),
            key_risks,
            evidence_basis: vec!["embedded_chair_request".to_string()],
            advice_summary: rationale.unwrap_or_else(|| "未提供额外解释".to_string()),
        }
    })
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
fn default_lookback_days() -> usize {
    260
}
fn default_disclosure_limit() -> usize {
    8
}
fn default_stop_loss_pct() -> f64 {
    0.05
}
fn default_target_return_pct() -> f64 {
    0.12
}
fn default_min_risk_reward_ratio() -> f64 {
    2.0
}
