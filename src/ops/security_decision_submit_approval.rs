use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::ops::stock::security_approval_brief_signature::sign_security_approval_brief_document;
use crate::ops::stock::security_chair_resolution::derive_training_guarded_chair_action;
use crate::ops::stock::security_decision_approval_bridge::{
    PersistedApprovalPositionPlanBinding, PersistedDecisionAuditRecord,
    SecurityDecisionApprovalBridgeOptions, bridge_security_decision_to_approval,
};
use crate::ops::stock::security_decision_approval_brief::{
    SecurityApprovalBriefBuildInput, build_master_scorecard_summary,
    build_model_governance_summary, build_model_grade_summary,
    build_security_decision_approval_brief,
};
use crate::ops::stock::security_decision_evidence_bundle::SecurityExternalProxyInputs;
use crate::ops::stock::security_decision_package::{
    SecurityDecisionPackageArtifact, SecurityDecisionPackageBuildInput,
    SecurityDecisionPackageDocument, SecurityDecisionPackageModelGovernanceSummary,
    SecurityDecisionPackageModelGradeSummary, build_security_decision_package, sha256_for_bytes,
    sha256_for_json_value,
};
use crate::ops::stock::security_legacy_committee_compat::{
    LegacySecurityDecisionCommitteeError as SecurityDecisionCommitteeError,
    LegacySecurityDecisionCommitteeRequest as SecurityDecisionCommitteeRequest,
    LegacySecurityDecisionCommitteeResult as SecurityDecisionCommitteeResult,
    run_security_decision_committee_legacy_compat,
};
use crate::ops::stock::security_master_scorecard::{
    SecurityMasterScorecardRequest, security_master_scorecard,
};
use crate::ops::stock::security_model_promotion::SecurityModelPromotionDocument;
use crate::ops::stock::security_position_plan::{
    apply_security_entry_layer_to_position_plan, apply_security_sizing_layer_to_position_plan,
};
use crate::ops::stock::security_scorecard::{SecurityScorecardDocument, SecurityScorecardError};
use crate::ops::stock::security_scorecard_model_registry::SecurityScorecardModelRegistry;
use crate::ops::stock::security_shadow_evaluation::SecurityShadowEvaluationDocument;

// 2026-04-02 CST: 这里定义证券审批提交请求，原因是“提交到审批主线”除了投决参数，还需要审批运行时路径与治理默认值；
// 目的：把提交阶段所需的附加控制面参数集中收口，避免外层调用方自己拼路径和默认审批规则。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityDecisionSubmitApprovalRequest {
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
    #[serde(default = "default_scene_name")]
    pub scene_name: String,
    #[serde(default)]
    pub approval_runtime_root: Option<String>,
    #[serde(default = "default_created_at")]
    pub created_at: String,
    #[serde(default = "default_min_approvals")]
    pub min_approvals: u8,
    #[serde(default = "default_require_risk_signoff")]
    pub require_risk_signoff: bool,
    #[serde(default)]
    pub approval_brief_signing_key_id: Option<String>,
    #[serde(default)]
    pub approval_brief_signing_key_secret: Option<String>,
    #[serde(default)]
    pub approval_brief_signing_key_secret_env: Option<String>,
    #[serde(default)]
    pub scorecard_model_path: Option<String>,
    #[serde(default)]
    pub scorecard_model_registry_path: Option<String>,
    #[serde(default)]
    pub model_promotion_path: Option<String>,
    #[serde(default)]
    pub shadow_evaluation_path: Option<String>,
    #[serde(default)]
    pub external_proxy_inputs: Option<SecurityExternalProxyInputs>,
}

// 2026-04-02 CST: 这里定义证券审批提交结果，原因是调用方不仅需要看到 committee 结果，还需要知道具体写到了哪些审批工件；
// 目的：让 CLI / Skill 可以立即接着用 decision_ref、approval_ref 和路径工件做后续流程编排。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityDecisionSubmitApprovalResult {
    pub decision_ref: String,
    pub approval_ref: String,
    pub committee_result: SecurityDecisionCommitteeResult,
    pub approval_brief: serde_json::Value,
    pub approval_request: serde_json::Value,
    pub position_plan: serde_json::Value,
    pub scorecard: serde_json::Value,
    pub master_scorecard: serde_json::Value,
    pub decision_package: serde_json::Value,
    pub approval_brief_path: String,
    pub approval_brief_signature_path: Option<String>,
    pub decision_card_path: String,
    pub approval_request_path: String,
    pub approval_events_path: String,
    pub position_plan_path: String,
    pub scorecard_path: String,
    pub master_scorecard_path: String,
    pub decision_package_path: String,
    pub audit_log_path: String,
}

// 2026-04-11 CST: Add a local approval-stage model-grade view, because P5 needs
// approval and package builders to consume registry/promotion semantics without
// exposing storage-path parsing logic everywhere.
// Purpose: keep grade loading and approval-mode mapping centralized in this submit flow.
#[derive(Debug, Clone, PartialEq)]
struct LoadedModelGradeSummary {
    model_grade: String,
    grade_reason: String,
    approval_consumption_mode: String,
    shadow_observation_count: usize,
    shadow_consistency_status: String,
    shadow_window_count: usize,
    oot_stability_status: String,
    window_consistency_status: String,
    promotion_blockers: Vec<String>,
    promotion_evidence_notes: Vec<String>,
}

// 2026-04-02 CST: 这里定义提交错误边界，原因是提交阶段同时会触发投决计算与文件持久化；
// 目的：给 dispatcher 一个统一错误口径，外层不需要区分是投决失败还是落盘失败。
#[derive(Debug, Error)]
pub enum SecurityDecisionSubmitApprovalError {
    #[error("证券审批提交前的投决计算失败: {0}")]
    Committee(#[from] SecurityDecisionCommitteeError),
    #[error("证券审批提交落盘失败: {0}")]
    Persist(String),
    #[error("证券评分卡构建失败: {0}")]
    Scorecard(#[from] SecurityScorecardError),
}

// 2026-04-02 CST: 这里实现证券审批提交总入口，原因是 P0-1 要让证券投决对象第一次正式进入审批主线；
// 目的：通过单一 Tool 完成“投决 -> 桥接 -> 审批工件持久化”，让后续多签和审计继续沿私有 workbench 运行。
pub fn security_decision_submit_approval(
    request: &SecurityDecisionSubmitApprovalRequest,
) -> Result<SecurityDecisionSubmitApprovalResult, SecurityDecisionSubmitApprovalError> {
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
        external_proxy_inputs: request.external_proxy_inputs.clone(),
    };
    let committee_result = run_security_decision_committee_legacy_compat(&committee_request)?;
    let mut bridge = bridge_security_decision_to_approval(
        &committee_result,
        &SecurityDecisionApprovalBridgeOptions {
            scene_name: request.scene_name.clone(),
            created_at: request.created_at.clone(),
            min_approvals: request.min_approvals,
            require_risk_signoff: request.require_risk_signoff,
        },
    );

    let runtime_root = resolve_runtime_root(request);
    let decision_path = runtime_root
        .join("decisions")
        .join(format!("{}.json", bridge.decision_card.decision_id));
    let approval_path = runtime_root
        .join("approvals")
        .join(format!("{}.json", sanitize_ref(&bridge.approval_ref)));
    let approval_events_path = runtime_root
        .join("approval_events")
        .join(format!("{}.json", bridge.decision_card.decision_id));
    let position_plan_path = runtime_root
        .join("position_plans")
        .join(format!("{}.json", bridge.decision_card.decision_id));
    let approval_brief_path = runtime_root
        .join("approval_briefs")
        .join(format!("{}.json", bridge.decision_card.decision_id));
    let scorecard_path = runtime_root
        .join("scorecards")
        .join(format!("{}.json", bridge.decision_card.decision_id));
    let master_scorecard_path = runtime_root
        .join("master_scorecards")
        .join(format!("{}.json", bridge.decision_card.decision_id));
    let decision_package_path = runtime_root
        .join("decision_packages")
        .join(format!("{}.json", bridge.decision_card.decision_id));
    let audit_log_path = runtime_root
        .join("audit_log")
        .join(format!("{}.jsonl", bridge.decision_card.decision_id));
    // 2026-04-16 CST: Added because submit_approval now feeds the formal
    // security_master_scorecard mainline in addition to the local scorecard path.
    // Reason: relative model artifact paths should resolve the same way across
    // approval CLI tests, runtime calls, and the nested mainline invocation.
    // Purpose: stabilize model-path handling before the approval flow delegates
    // master_scorecard construction to the formal tool path.
    let resolved_scorecard_model_path = request
        .scorecard_model_path
        .as_deref()
        .map(resolve_runtime_model_path);

    // 2026-04-08 CST: 这里在 submit 阶段补齐 approval_request 对仓位计划的正式绑定，原因是 Task 2 需要把 position_plan 真正挂进审批链；
    // 目的：让 approval_request 在落盘时就带上 plan 的 ref、路径、合同版本、摘要和方向，而不是事后再从 package 推断。
    bridge.approval_request.position_plan_binding = Some(build_position_plan_binding(
        &bridge.position_plan,
        &position_plan_path,
    )?);
    // 2026-04-16 CST: Changed because approved scheme A requires submit_approval
    // to consume the formal security_master_scorecard mainline instead of keeping
    // a private replay/unavailable builder branch in the approval flow.
    // Reason: source divergence here would let approval, CLI, and later bridge
    // consumers observe different master-scorecard semantics for the same input.
    // Purpose: unify approval-stage master_scorecard sourcing while preserving the
    // existing downstream approval_brief, position_plan, and persistence contract.
    let master_scorecard_result = security_master_scorecard(&SecurityMasterScorecardRequest {
        symbol: request.symbol.clone(),
        market_symbol: request.market_symbol.clone(),
        sector_symbol: request.sector_symbol.clone(),
        market_profile: request.market_profile.clone(),
        sector_profile: request.sector_profile.clone(),
        as_of_date: request.as_of_date.clone(),
        lookback_days: request.lookback_days,
        disclosure_limit: request.disclosure_limit,
        horizons: vec![5, 10, 20, 30, 60, 180],
        stop_loss_pct: request.stop_loss_pct,
        target_return_pct: request.target_return_pct,
        min_risk_reward_ratio: request.min_risk_reward_ratio,
        created_at: request.created_at.clone(),
        scorecard_model_path: resolved_scorecard_model_path,
        return_head_model_path: None,
        drawdown_head_model_path: None,
        path_quality_head_model_path: None,
        upside_first_head_model_path: None,
        stop_first_head_model_path: None,
        external_proxy_inputs: request.external_proxy_inputs.clone(),
        prediction_mode: None,
        prediction_horizon_days: 180,
    })
    .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?;
    // 2026-04-16 CST: Changed because submit_approval previously kept a second local
    // scorecard build beside the formal master-scorecard mainline invocation.
    // Reason: that duplicate build already drifted in tests, causing approval_brief
    // and returned scorecard payloads to disagree about the same request.
    // Purpose: make approval flow consume the single mainline scorecard source-of-truth.
    let scorecard = align_scorecard_for_approval_package(
        master_scorecard_result.scorecard,
        &committee_result,
        &bridge,
    );
    let master_scorecard = master_scorecard_result.master_scorecard;
    let model_grade_summary = load_model_grade_summary(request)?;
    // 2026-04-13 CST: Refresh the formal position_plan after scorecard and
    // master_scorecard land, because the first-stage entry layer depends on the
    // later quant readiness and chair-action semantics.
    // Purpose: keep the rollout incremental without moving bridge construction.
    let chair_action = derive_training_guarded_chair_action(&committee_result, &scorecard);
    apply_security_entry_layer_to_position_plan(
        &mut bridge.position_plan,
        &committee_result,
        &scorecard,
        Some(&master_scorecard),
        &chair_action,
    );
    // 2026-04-13 CST: Refresh the second-stage sizing layer right after the
    // entry layer, because approval reviewers now need a governed "how much"
    // answer on the same formal position_plan object.
    // Purpose: keep the rollout incremental while unifying approval output.
    apply_security_sizing_layer_to_position_plan(
        &mut bridge.position_plan,
        &committee_result,
        &scorecard,
        Some(&master_scorecard),
        &chair_action,
    );
    bridge.approval_request.position_plan_binding = Some(build_position_plan_binding(
        &bridge.position_plan,
        &position_plan_path,
    )?);
    bridge.approval_brief = build_security_decision_approval_brief(
        &committee_result,
        &bridge.position_plan,
        &SecurityApprovalBriefBuildInput {
            scene_name: request.scene_name.clone(),
            generated_at: bridge.approval_request.created_at.clone(),
            decision_id: bridge.decision_card.decision_id.clone(),
            decision_ref: bridge.decision_ref.clone(),
            approval_ref: bridge.approval_ref.clone(),
            approval_status: format!("{:?}", bridge.approval_request.status),
            evidence_hash: committee_result.evidence_bundle.evidence_hash.clone(),
            governance_hash: bridge
                .approval_request
                .governance_hash
                .clone()
                .unwrap_or_default(),
        },
    );
    // 2026-04-13 CST: Mirror the entry-grade summary into the approval brief,
    // because the user asked for unified presentation across approval surfaces.
    // Purpose: keep later AI or human reviewers from opening multiple artifacts.
    bridge.approval_brief.entry_summary = format!(
        "entry grade {} | target {:.2}% | 首仓 {:.2}% | max {:.2}% | reason {}{}",
        bridge.position_plan.entry_grade,
        bridge.position_plan.target_gross_pct * 100.0,
        bridge.position_plan.starter_gross_pct * 100.0,
        bridge.position_plan.max_gross_pct * 100.0,
        format!(
            "{} ; {}",
            bridge.position_plan.entry_reason, bridge.position_plan.sizing_reason
        ),
        if bridge.position_plan.entry_blockers.is_empty() {
            String::new()
        } else {
            format!(
                " | blockers {}",
                bridge.position_plan.entry_blockers.join(",")
            )
        }
    );
    bridge.approval_brief.position_plan_summary.entry_summary =
        bridge.approval_brief.entry_summary.clone();
    bridge.approval_brief.master_scorecard_summary =
        Some(build_master_scorecard_summary(&master_scorecard));
    bridge.approval_brief.model_grade_summary = model_grade_summary
        .as_ref()
        .map(|summary| build_model_grade_summary(&summary.model_grade, &summary.grade_reason));
    bridge.approval_brief.model_governance_summary = model_grade_summary.as_ref().map(|summary| {
        // 2026-04-11 CST: Mirror shadow-governance details into the approval
        // brief, because P6 wants approval reviewers to see blocker and
        // observation depth without opening promotion artifacts manually.
        // Purpose: keep brief/package governance disclosure aligned on one
        // loaded approval-stage summary.
        build_model_governance_summary(
            &summary.model_grade,
            &summary.grade_reason,
            summary.shadow_observation_count,
            &summary.shadow_consistency_status,
            summary.shadow_window_count,
            &summary.oot_stability_status,
            &summary.window_consistency_status,
            &summary.promotion_blockers,
            &summary.promotion_evidence_notes,
        )
    });
    apply_training_guardrail_to_approval_brief(
        &mut bridge.approval_brief,
        &scorecard,
        model_grade_summary.as_ref(),
    );

    persist_json(&decision_path, &bridge.decision_card)?;
    persist_json(&approval_path, &bridge.approval_request)?;
    persist_json(&approval_events_path, &bridge.approval_events)?;
    persist_json(&position_plan_path, &bridge.position_plan)?;
    persist_json(&approval_brief_path, &bridge.approval_brief)?;
    persist_json(&scorecard_path, &scorecard)?;
    persist_json(&master_scorecard_path, &master_scorecard)?;
    let approval_brief_signature_path = maybe_persist_approval_brief_signature(
        &approval_brief_path,
        request,
        &bridge.approval_brief,
    )?;
    persist_audit_record(&audit_log_path, &bridge.audit_record)?;
    let approval_brief_signature_value =
        load_optional_json_file(approval_brief_signature_path.as_deref())?;
    let decision_package = build_decision_package_document(
        request,
        &committee_result,
        &bridge,
        &decision_path,
        &approval_path,
        &approval_events_path,
        &position_plan_path,
        &approval_brief_path,
        &scorecard_path,
        &scorecard,
        &audit_log_path,
        approval_brief_signature_path.as_deref(),
        approval_brief_signature_value.as_ref(),
        model_grade_summary.as_ref(),
    )?;
    persist_json(&decision_package_path, &decision_package)?;

    Ok(SecurityDecisionSubmitApprovalResult {
        decision_ref: bridge.decision_ref,
        approval_ref: bridge.approval_ref,
        committee_result,
        approval_brief: serde_json::to_value(&bridge.approval_brief)
            .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?,
        approval_request: serde_json::to_value(&bridge.approval_request)
            .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?,
        position_plan: serde_json::to_value(&bridge.position_plan)
            .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?,
        scorecard: serde_json::to_value(&scorecard)
            .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?,
        master_scorecard: serde_json::to_value(&master_scorecard)
            .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?,
        decision_package: serde_json::to_value(&decision_package)
            .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?,
        approval_brief_path: approval_brief_path.to_string_lossy().to_string(),
        approval_brief_signature_path,
        decision_card_path: decision_path.to_string_lossy().to_string(),
        approval_request_path: approval_path.to_string_lossy().to_string(),
        approval_events_path: approval_events_path.to_string_lossy().to_string(),
        position_plan_path: position_plan_path.to_string_lossy().to_string(),
        scorecard_path: scorecard_path.to_string_lossy().to_string(),
        master_scorecard_path: master_scorecard_path.to_string_lossy().to_string(),
        decision_package_path: decision_package_path.to_string_lossy().to_string(),
        audit_log_path: audit_log_path.to_string_lossy().to_string(),
    })
}

// 2026-04-02 CST: 这里集中组装正式审批包，原因是 package 需要同时感知所有工件路径、哈希和治理绑定；
// 目的：把包级构造从提交主流程里抽出来，后续新增工件时只需要维护这一处 manifest 逻辑。
fn build_decision_package_document(
    request: &SecurityDecisionSubmitApprovalRequest,
    committee_result: &SecurityDecisionCommitteeResult,
    bridge: &crate::ops::stock::security_decision_approval_bridge::SecurityDecisionApprovalBridgeResult,
    decision_path: &Path,
    approval_path: &Path,
    approval_events_path: &Path,
    position_plan_path: &Path,
    approval_brief_path: &Path,
    scorecard_path: &Path,
    scorecard: &SecurityScorecardDocument,
    audit_log_path: &Path,
    approval_brief_signature_path: Option<&str>,
    approval_brief_signature_value: Option<&Value>,
    model_grade_summary: Option<&LoadedModelGradeSummary>,
) -> Result<SecurityDecisionPackageDocument, SecurityDecisionSubmitApprovalError> {
    let decision_value = serde_json::to_value(&bridge.decision_card)
        .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?;
    let approval_value = serde_json::to_value(&bridge.approval_request)
        .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?;
    let approval_events_value = serde_json::to_value(&bridge.approval_events)
        .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?;
    let position_plan_value = serde_json::to_value(&bridge.position_plan)
        .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?;
    let approval_brief_value = serde_json::to_value(&bridge.approval_brief)
        .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?;
    let scorecard_value = serde_json::to_value(scorecard)
        .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?;
    let audit_payload = fs::read(audit_log_path)
        .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?;

    let mut artifact_manifest = vec![
        build_package_artifact(
            "decision_card",
            decision_path,
            "sheetmind.decision_card.v1",
            true,
            Some(&decision_value),
            None,
        )?,
        build_package_artifact(
            "approval_request",
            approval_path,
            "sheetmind.approval_request.v1",
            true,
            Some(&approval_value),
            None,
        )?,
        build_package_artifact(
            "approval_events",
            approval_events_path,
            "security_approval_events.v1",
            true,
            Some(&approval_events_value),
            None,
        )?,
        build_package_artifact(
            "position_plan",
            position_plan_path,
            "security_position_plan.v2",
            true,
            Some(&position_plan_value),
            None,
        )?,
        build_package_artifact(
            "approval_brief",
            approval_brief_path,
            "security_approval_brief.v1",
            true,
            Some(&approval_brief_value),
            None,
        )?,
        build_package_artifact(
            "security_scorecard",
            scorecard_path,
            "security_scorecard.v1",
            true,
            Some(&scorecard_value),
            None,
        )?,
        build_package_artifact(
            "audit_log",
            audit_log_path,
            "security_decision_audit_log.v1",
            true,
            None,
            Some(&audit_payload),
        )?,
    ];
    artifact_manifest.push(build_optional_package_artifact(
        "approval_brief_signature",
        approval_brief_signature_path,
        "security_approval_brief_signature.v1",
        false,
        approval_brief_signature_value,
    )?);

    Ok(build_security_decision_package(
        SecurityDecisionPackageBuildInput {
            created_at: request.created_at.clone(),
            package_version: 1,
            previous_package_path: None,
            revision_reason: "initial_submission".to_string(),
            trigger_event_summary: "initial package generated from submit approval".to_string(),
            scene_name: request.scene_name.clone(),
            decision_id: bridge.decision_card.decision_id.clone(),
            decision_ref: bridge.decision_ref.clone(),
            approval_ref: bridge.approval_ref.clone(),
            symbol: committee_result.symbol.clone(),
            analysis_date: committee_result.analysis_date.clone(),
            decision_status: committee_result.decision_card.status.clone(),
            approval_status: format!("{:?}", bridge.approval_request.status),
            model_grade_summary: model_grade_summary.map(|summary| {
                SecurityDecisionPackageModelGradeSummary {
                    model_grade: summary.model_grade.clone(),
                    grade_reason: summary.grade_reason.clone(),
                    approval_consumption_mode: summary.approval_consumption_mode.clone(),
                }
            }),
            // 2026-04-08 CST: 这里把正式对象图一次性写入 package builder，原因是 Task 1 需要让 package 显式表达对象之间的稳定引用；
            // 目的：让后续 verify / revision 都消费统一的 object_graph，而不是继续从 artifact role 反推对象关系。
            position_plan_ref: bridge.position_plan.plan_id.clone(),
            // 2026-04-11 CST: Persist richer governance details into the package,
            // because P6 requires revision/audit consumers to retain blocker
            // context instead of only the coarse model-grade label.
            // Purpose: make shadow depth and promotion blockers part of the
            // formal decision-package contract.
            model_governance_summary: model_grade_summary.map(|summary| {
                SecurityDecisionPackageModelGovernanceSummary {
                    model_grade: summary.model_grade.clone(),
                    grade_reason: summary.grade_reason.clone(),
                    approval_consumption_mode: summary.approval_consumption_mode.clone(),
                    shadow_observation_count: summary.shadow_observation_count,
                    shadow_consistency_status: summary.shadow_consistency_status.clone(),
                    shadow_window_count: summary.shadow_window_count,
                    oot_stability_status: summary.oot_stability_status.clone(),
                    window_consistency_status: summary.window_consistency_status.clone(),
                    promotion_blockers: summary.promotion_blockers.clone(),
                    promotion_evidence_notes: summary.promotion_evidence_notes.clone(),
                }
            }),
            // 2026-04-12 CST: Keep lifecycle fields empty on the initial approval
            // package, because P8 binds review/execution/post-trade artifacts only
            // after those lifecycle events actually occur.
            // Purpose: let later package revisions attach governed lifecycle documents without inventing them up front.
            lifecycle_governance_summary: None,
            approval_brief_ref: bridge.approval_brief.brief_id.clone(),
            scorecard_ref: scorecard.scorecard_id.clone(),
            chair_resolution_ref: None,
            condition_review_ref: None,
            execution_record_ref: None,
            post_trade_review_ref: None,
            decision_card_path: decision_path.to_string_lossy().to_string(),
            approval_request_path: approval_path.to_string_lossy().to_string(),
            position_plan_path: position_plan_path.to_string_lossy().to_string(),
            approval_brief_path: approval_brief_path.to_string_lossy().to_string(),
            scorecard_path: scorecard_path.to_string_lossy().to_string(),
            condition_review_path: None,
            execution_record_path: None,
            post_trade_review_path: None,
            evidence_hash: committee_result.evidence_bundle.evidence_hash.clone(),
            governance_hash: bridge
                .approval_request
                .governance_hash
                .clone()
                .unwrap_or_default(),
            artifact_manifest,
        },
    ))
}

// 2026-04-16 CST: Reason=submit_approval now consumes the formal master-scorecard
// mainline, but the returned scorecard document keeps master-lineage refs that do
// not belong to the approval package contract.
// Purpose=rebind the persisted approval-package scorecard to the decision/approval
// chain without changing the separate master-scorecard mainline semantics.
fn align_scorecard_for_approval_package(
    mut scorecard: SecurityScorecardDocument,
    committee_result: &SecurityDecisionCommitteeResult,
    bridge: &crate::ops::stock::security_decision_approval_bridge::SecurityDecisionApprovalBridgeResult,
) -> SecurityScorecardDocument {
    scorecard.symbol = committee_result.symbol.clone();
    scorecard.analysis_date = committee_result.analysis_date.clone();
    scorecard.decision_id = bridge.decision_card.decision_id.clone();
    scorecard.decision_ref = bridge.decision_ref.clone();
    scorecard.approval_ref = bridge.approval_ref.clone();
    scorecard
}

fn resolve_runtime_model_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() || candidate.exists() {
        return candidate.to_string_lossy().to_string();
    }

    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(candidate)
        .to_string_lossy()
        .to_string()
}

fn persist_json<T: SerializeSized>(
    path: &Path,
    value: &T,
) -> Result<(), SecurityDecisionSubmitApprovalError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?;
    }
    let payload = serde_json::to_vec_pretty(value)
        .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?;
    fs::write(path, payload)
        .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))
}

// 2026-04-02 CST: 这里在提交阶段读取可选签名文件，原因是 package manifest 需要把“已存在的签名工件”纳入清单；
// 目的：避免 package 构造层再次理解签名业务，只消费一个可选 JSON 值即可。
fn load_optional_json_file(
    path: Option<&str>,
) -> Result<Option<Value>, SecurityDecisionSubmitApprovalError> {
    let Some(path) = path else {
        return Ok(None);
    };
    let payload = fs::read(path)
        .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?;
    let value = serde_json::from_slice(&payload)
        .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?;
    Ok(Some(value))
}

fn load_model_grade_summary(
    request: &SecurityDecisionSubmitApprovalRequest,
) -> Result<Option<LoadedModelGradeSummary>, SecurityDecisionSubmitApprovalError> {
    // 2026-04-11 CST: Prefer explicit promotion records over raw registry state,
    // because P5 wants approval semantics to follow the latest governed grade decision.
    // Purpose: let approval consume one authoritative model-grade summary without manual branching.
    if let Some(path) = request
        .model_promotion_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let promotion = load_json_file::<SecurityModelPromotionDocument>(path)?;
        let summary = build_loaded_model_grade_summary(
            &promotion.approved_model_grade,
            &promotion.promotion_reason,
        );
        return Ok(Some(merge_shadow_governance_into_grade_summary(
            request,
            LoadedModelGradeSummary {
                shadow_observation_count: promotion.shadow_observation_count,
                shadow_consistency_status: promotion.shadow_consistency_status.clone(),
                shadow_window_count: promotion.shadow_window_count,
                oot_stability_status: promotion.oot_stability_status.clone(),
                window_consistency_status: promotion.window_consistency_status.clone(),
                promotion_blockers: promotion.promotion_blockers.clone(),
                promotion_evidence_notes: promotion.promotion_evidence_notes.clone(),
                ..summary
            },
        )?));
    }

    if let Some(path) = request
        .scorecard_model_registry_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let registry = load_json_file::<SecurityScorecardModelRegistry>(path)?;
        let summary =
            build_loaded_model_grade_summary(&registry.model_grade, &registry.grade_reason);
        return Ok(Some(merge_shadow_governance_into_grade_summary(
            request, summary,
        )?));
    }

    Ok(None)
}

fn build_loaded_model_grade_summary(
    model_grade: &str,
    grade_reason: &str,
) -> LoadedModelGradeSummary {
    let approval_consumption_mode = match model_grade {
        "champion" => "full_release_quant_context",
        "shadow" => "reference_only_quant_context",
        _ => "governance_only_quant_context",
    };
    LoadedModelGradeSummary {
        model_grade: model_grade.to_string(),
        grade_reason: grade_reason.to_string(),
        approval_consumption_mode: approval_consumption_mode.to_string(),
        shadow_observation_count: 0,
        shadow_consistency_status: "shadow_untracked".to_string(),
        shadow_window_count: 0,
        oot_stability_status: "oot_untracked".to_string(),
        window_consistency_status: "window_untracked".to_string(),
        promotion_blockers: Vec::new(),
        promotion_evidence_notes: Vec::new(),
    }
}

fn merge_shadow_governance_into_grade_summary(
    request: &SecurityDecisionSubmitApprovalRequest,
    mut summary: LoadedModelGradeSummary,
) -> Result<LoadedModelGradeSummary, SecurityDecisionSubmitApprovalError> {
    let Some(path) = request
        .shadow_evaluation_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(summary);
    };

    // 2026-04-11 CST: Merge shadow-evaluation governance into the approval-stage
    // grade snapshot, because P6 wants approval/package consumers to reuse one
    // loaded summary instead of independently reopening shadow documents.
    // Purpose: keep shadow count, consistency, and blockers synchronized across
    // the approval brief, package, and guardrail logic.
    let shadow_evaluation = load_json_file::<SecurityShadowEvaluationDocument>(path)?;
    summary.shadow_observation_count = shadow_evaluation.shadow_observation_count;
    summary.shadow_consistency_status = shadow_evaluation.shadow_consistency_status;
    summary.shadow_window_count = shadow_evaluation.shadow_window_count;
    summary.oot_stability_status = shadow_evaluation.oot_stability_status;
    summary.window_consistency_status = shadow_evaluation.window_consistency_status;
    summary.promotion_blockers = shadow_evaluation.promotion_blockers;
    summary.promotion_evidence_notes = shadow_evaluation.promotion_evidence_notes;
    Ok(summary)
}

fn load_json_file<T: serde::de::DeserializeOwned>(
    path: &str,
) -> Result<T, SecurityDecisionSubmitApprovalError> {
    let payload = fs::read(path)
        .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?;
    serde_json::from_slice::<T>(&payload)
        .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))
}

// 2026-04-11 CST: Add approval-stage training disclosure guardrail, reason:
// the user required approval objects to explicitly stop short of a normal
// pass-through review when scorecard training support is unavailable.
// Purpose: convert the brief into a "request more evidence" review mode and
// append a stable remediation note for later audit and replay.
fn apply_training_guardrail_to_approval_brief(
    approval_brief: &mut crate::ops::stock::security_decision_approval_brief::SecurityDecisionApprovalBrief,
    scorecard: &SecurityScorecardDocument,
    model_grade_summary: Option<&LoadedModelGradeSummary>,
) {
    if scorecard.score_status != "ready" {
        approval_brief.recommended_review_action = "request_more_evidence".to_string();
        if !approval_brief
            .required_next_actions
            .iter()
            .any(|item| item.contains("训练"))
        {
            approval_brief
                .required_next_actions
                .push("补齐训练 artifact 或继续回算后再提交审批".to_string());
        }
        if !approval_brief.final_recommendation.contains("训练支撑") {
            approval_brief.final_recommendation = format!(
                "{} 当前训练支撑尚未就绪，审批层不得按标准放行理解。",
                approval_brief.final_recommendation
            );
        }
        if !approval_brief.executive_summary.contains("训练支撑") {
            approval_brief.executive_summary = format!(
                "{} 当前训练支撑尚未就绪。",
                approval_brief.executive_summary
            );
        }
    }

    let Some(model_grade_summary) = model_grade_summary else {
        return;
    };
    if model_grade_summary.approval_consumption_mode == "full_release_quant_context" {
        return;
    }

    approval_brief.recommended_review_action = "request_more_evidence".to_string();
    let grade_note = match model_grade_summary.approval_consumption_mode.as_str() {
        "reference_only_quant_context" => {
            "当前量化模型仅处于 shadow 等级，只能作为参考上下文，不得单独放行。"
        }
        _ => "当前量化模型仍属 research/candidate 等级，仅可作为治理阻断信息。",
    };
    if !approval_brief.required_next_actions.iter().any(|item| {
        item.contains("shadow") || item.contains("candidate") || item.contains("champion")
    }) {
        approval_brief
            .required_next_actions
            .push(grade_note.to_string());
    }
    for blocker in &model_grade_summary.promotion_blockers {
        if approval_brief
            .required_next_actions
            .iter()
            .any(|item| item == blocker)
        {
            continue;
        }
        // 2026-04-11 CST: Surface explicit promotion blockers into the approval
        // action list, because P6 wants review follow-ups to preserve why a
        // model remained shadow/candidate instead of silently downgrading it.
        // Purpose: give approval reviewers a stable next-action queue that can
        // later be audited and replayed.
        approval_brief.required_next_actions.push(blocker.clone());
    }
    for evidence_note in &model_grade_summary.promotion_evidence_notes {
        if approval_brief
            .required_next_actions
            .iter()
            .any(|item| item == evidence_note)
        {
            continue;
        }
        // 2026-04-11 CST: Surface multi-window promotion evidence notes in the
        // approval action list, because P7 champion gating now depends on
        // comparison-window stability and OOT breadth in addition to blockers.
        // Purpose: keep approval follow-ups explicit when champion evidence is
        // still thin even though coarse blocker fields stayed empty.
        approval_brief
            .required_next_actions
            .push(evidence_note.clone());
    }
}

fn build_position_plan_binding(
    position_plan: &crate::ops::stock::security_position_plan::SecurityPositionPlan,
    position_plan_path: &Path,
) -> Result<PersistedApprovalPositionPlanBinding, SecurityDecisionSubmitApprovalError> {
    let position_plan_value = serde_json::to_value(position_plan)
        .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?;
    let position_plan_sha256 = sha256_for_json_value(&position_plan_value)
        .map_err(SecurityDecisionSubmitApprovalError::Persist)?;
    Ok(PersistedApprovalPositionPlanBinding {
        position_plan_ref: position_plan.plan_id.clone(),
        position_plan_path: position_plan_path.to_string_lossy().to_string(),
        position_plan_contract_version: position_plan.contract_version.clone(),
        position_plan_sha256,
        plan_status: position_plan.plan_status.clone(),
        plan_direction: position_plan.plan_direction.clone(),
        gross_limit_summary: format!(
            "suggested {:.2}% / starter {:.2}% / max {:.2}%",
            position_plan.suggested_gross_pct * 100.0,
            position_plan.starter_gross_pct * 100.0,
            position_plan.max_gross_pct * 100.0
        ),
    })
}

fn maybe_persist_approval_brief_signature(
    approval_brief_path: &Path,
    request: &SecurityDecisionSubmitApprovalRequest,
    approval_brief: &crate::ops::stock::security_decision_approval_brief::SecurityDecisionApprovalBrief,
) -> Result<Option<String>, SecurityDecisionSubmitApprovalError> {
    let Some(key_id) = request.approval_brief_signing_key_id.as_deref() else {
        return Ok(None);
    };
    let secret = resolve_optional_signing_secret(request)?;
    let envelope = sign_security_approval_brief_document(approval_brief, key_id, &secret)
        .map_err(SecurityDecisionSubmitApprovalError::Persist)?;
    let signature_path = approval_brief_path.with_extension("signature.json");
    persist_json(&signature_path, &envelope)?;
    Ok(Some(signature_path.to_string_lossy().to_string()))
}

fn persist_audit_record(
    path: &Path,
    record: &PersistedDecisionAuditRecord,
) -> Result<(), SecurityDecisionSubmitApprovalError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?;
    }

    let prev_hash = fs::read_to_string(path)
        .ok()
        .and_then(|content| content.lines().last().map(|line| line.to_string()))
        .and_then(|line| serde_json::from_str::<PersistedDecisionAuditRecord>(&line).ok())
        .and_then(|record| record.record_hash);

    let mut chained = record.clone();
    chained.prev_hash = prev_hash;
    chained.record_hash = Some(compute_audit_hash(&chained)?);
    let payload = serde_json::to_string(&chained)
        .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?;
    fs::write(path, format!("{payload}\n"))
        .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))
}

// 2026-04-02 CST: 这里统一生成必选工件 manifest，原因是 package 中大部分工件都必须存在且需要稳定哈希；
// 目的：减少 role/path/hash/contract_version 在主流程中的样板代码，后续新增工件也能沿用同一入口。
fn build_package_artifact(
    artifact_role: &str,
    path: &Path,
    contract_version: &str,
    required: bool,
    json_value: Option<&Value>,
    raw_bytes: Option<&[u8]>,
) -> Result<SecurityDecisionPackageArtifact, SecurityDecisionSubmitApprovalError> {
    let sha256 = if let Some(value) = json_value {
        sha256_for_json_value(value).map_err(SecurityDecisionSubmitApprovalError::Persist)?
    } else if let Some(bytes) = raw_bytes {
        sha256_for_bytes(bytes)
    } else {
        return Err(SecurityDecisionSubmitApprovalError::Persist(format!(
            "package artifact `{artifact_role}` missing payload for hashing"
        )));
    };
    Ok(SecurityDecisionPackageArtifact {
        artifact_role: artifact_role.to_string(),
        path: path.to_string_lossy().to_string(),
        sha256,
        contract_version: contract_version.to_string(),
        required,
        present: true,
    })
}

// 2026-04-02 CST: 这里统一生成可选工件 manifest，原因是 approval_brief_signature 这类工件在未开启签名时不应阻断 package 生成；
// 目的：让 package 明确表达“该工件是否存在”，而不是靠缺字段来猜。
fn build_optional_package_artifact(
    artifact_role: &str,
    path: Option<&str>,
    contract_version: &str,
    required: bool,
    json_value: Option<&Value>,
) -> Result<SecurityDecisionPackageArtifact, SecurityDecisionSubmitApprovalError> {
    match (path, json_value) {
        (Some(path), Some(value)) => Ok(SecurityDecisionPackageArtifact {
            artifact_role: artifact_role.to_string(),
            path: path.to_string(),
            sha256: sha256_for_json_value(value)
                .map_err(SecurityDecisionSubmitApprovalError::Persist)?,
            contract_version: contract_version.to_string(),
            required,
            present: true,
        }),
        _ => Ok(SecurityDecisionPackageArtifact {
            artifact_role: artifact_role.to_string(),
            path: String::new(),
            sha256: String::new(),
            contract_version: contract_version.to_string(),
            required,
            present: false,
        }),
    }
}

fn compute_audit_hash(
    record: &PersistedDecisionAuditRecord,
) -> Result<String, SecurityDecisionSubmitApprovalError> {
    let payload = serde_json::to_vec(record)
        .map_err(|error| SecurityDecisionSubmitApprovalError::Persist(error.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(payload);
    Ok(format!("{:x}", hasher.finalize()))
}

fn resolve_runtime_root(request: &SecurityDecisionSubmitApprovalRequest) -> PathBuf {
    request
        .approval_runtime_root
        .as_ref()
        .map(|path| PathBuf::from(path.trim()))
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| {
            PathBuf::from(".worktrees")
                .join("SheetMind-Scenes-inspect")
                .join(".sheetmind_scenes_runtime")
        })
}

fn sanitize_ref(raw: &str) -> String {
    raw.replace(':', "__")
}

fn default_scene_name() -> String {
    "security_decision_committee".to_string()
}

fn default_created_at() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn default_min_approvals() -> u8 {
    2
}

fn default_require_risk_signoff() -> bool {
    true
}

fn resolve_optional_signing_secret(
    request: &SecurityDecisionSubmitApprovalRequest,
) -> Result<String, SecurityDecisionSubmitApprovalError> {
    if let Some(secret) = request.approval_brief_signing_key_secret.as_ref() {
        if !secret.trim().is_empty() {
            return Ok(secret.trim().to_string());
        }
        return Err(SecurityDecisionSubmitApprovalError::Persist(
            "approval brief signing secret cannot be empty".to_string(),
        ));
    }

    if let Some(env_key) = request.approval_brief_signing_key_secret_env.as_ref() {
        let value = std::env::var(env_key).map_err(|error| {
            SecurityDecisionSubmitApprovalError::Persist(format!(
                "failed to read approval brief signing secret env `{env_key}`: {error}"
            ))
        })?;
        if value.trim().is_empty() {
            return Err(SecurityDecisionSubmitApprovalError::Persist(format!(
                "approval brief signing secret env `{env_key}` is empty"
            )));
        }
        return Ok(value);
    }

    Err(SecurityDecisionSubmitApprovalError::Persist(
        "approval brief signing requested but no signing secret provided".to_string(),
    ))
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

trait SerializeSized: Serialize {}

impl<T> SerializeSized for T where T: Serialize {}
