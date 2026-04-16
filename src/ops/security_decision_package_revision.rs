use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::ops::stock::security_condition_review::SecurityConditionReviewDocument;
use crate::ops::stock::security_decision_package::{
    SecurityDecisionPackageArtifact, SecurityDecisionPackageBuildInput,
    SecurityDecisionPackageDocument, SecurityDecisionPackageLifecycleGovernanceSummary,
    build_security_decision_package, sha256_for_bytes, sha256_for_json_value,
};
use crate::ops::stock::security_decision_verify_package::{
    SecurityDecisionVerifyPackageRequest, security_decision_verify_package,
};
use crate::ops::stock::security_execution_record::SecurityExecutionRecordDocument;
use crate::ops::stock::security_post_trade_review::SecurityPostTradeReviewDocument;
use crate::ops::stock::security_record_post_meeting_conclusion::SecurityPostMeetingConclusionDocument;

// 2026-04-02 CST: 这里定义审批包版本化请求，原因是 P0-6 需要一个正式 Tool 把旧 package 升级成下一版本；
// 目的：把版本化所需的包路径、修订原因和是否重跑校验等参数统一收口，避免调用方自己拼接内部步骤。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityDecisionPackageRevisionRequest {
    pub package_path: String,
    #[serde(default = "default_revision_reason")]
    pub revision_reason: String,
    #[serde(default = "default_reverify_after_revision")]
    pub reverify_after_revision: bool,
    #[serde(default)]
    pub condition_review_path: Option<String>,
    #[serde(default)]
    pub execution_record_path: Option<String>,
    #[serde(default)]
    pub post_trade_review_path: Option<String>,
    #[serde(default)]
    pub approval_brief_signing_key_secret: Option<String>,
    #[serde(default)]
    pub approval_brief_signing_key_secret_env: Option<String>,
}

// 2026-04-02 CST: 这里定义审批包版本化结果，原因是上层调用方除了新 package 以外，还需要知道 lineage 和可选 verification 工件；
// 目的：让 CLI / Skill 能直接消费 v2 package 结果，而不再手工拼路径和再调一次 verify。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityDecisionPackageRevisionResult {
    pub decision_package: Value,
    pub decision_package_path: String,
    pub package_version: u32,
    pub previous_package_path: String,
    pub revision_reason: String,
    pub trigger_event_summary: String,
    pub verification_report_path: Option<String>,
}

#[derive(Debug, Error)]
pub enum SecurityDecisionPackageRevisionError {
    #[error("证券审批包版本化失败: {0}")]
    Revision(String),
}

// 2026-04-12 CST: Track optional lifecycle attachments during revision, because
// P8 needs condition/execution/post-trade records to join the formal package
// only after those events exist.
// Purpose: keep lifecycle refs, paths, payloads, and feedback summary aligned in one temporary context.
#[derive(Debug, Clone, Default)]
struct LifecycleAttachmentContext {
    condition_review_ref: Option<String>,
    execution_record_ref: Option<String>,
    post_trade_review_ref: Option<String>,
    condition_review_path: Option<String>,
    execution_record_path: Option<String>,
    post_trade_review_path: Option<String>,
    condition_review_value: Option<Value>,
    execution_record_value: Option<Value>,
    post_trade_review_value: Option<Value>,
    lifecycle_governance_summary: Option<SecurityDecisionPackageLifecycleGovernanceSummary>,
}

// 2026-04-02 CST: 这里实现正式审批包版本化入口，原因是审批包需要随着审批动作生成后续版本，而不是停留在初始提交态；
// 目的：读取旧 package 与最新审批工件，生成新版本 package，并在需要时立即附带新的 verification report。
pub fn security_decision_package_revision(
    request: &SecurityDecisionPackageRevisionRequest,
) -> Result<SecurityDecisionPackageRevisionResult, SecurityDecisionPackageRevisionError> {
    if request.package_path.trim().is_empty() {
        return Err(SecurityDecisionPackageRevisionError::Revision(
            "package_path cannot be empty".to_string(),
        ));
    }

    let previous_package_path = PathBuf::from(request.package_path.trim());
    let previous_package: SecurityDecisionPackageDocument = serde_json::from_slice(
        &fs::read(&previous_package_path)
            .map_err(|error| SecurityDecisionPackageRevisionError::Revision(error.to_string()))?,
    )
    .map_err(|error| SecurityDecisionPackageRevisionError::Revision(error.to_string()))?;
    let lifecycle_context = load_lifecycle_attachment_context(request, &previous_package)?;

    let updated_artifact_manifest =
        rebuild_artifact_manifest(&previous_package.artifact_manifest, &lifecycle_context)?;
    let trigger_event_summary = infer_trigger_event_summary(&updated_artifact_manifest);
    let next_version = previous_package.package_version.saturating_add(1);
    let revised_package_path = resolve_revision_package_path(
        &previous_package_path,
        &previous_package.decision_id,
        next_version,
    )?;

    let approval_request_status =
        infer_approval_status(&updated_artifact_manifest).unwrap_or_else(|| "Pending".to_string());
    let revised_package = build_security_decision_package(SecurityDecisionPackageBuildInput {
        created_at: chrono::Utc::now().to_rfc3339(),
        package_version: next_version,
        previous_package_path: Some(previous_package_path.to_string_lossy().to_string()),
        revision_reason: request.revision_reason.trim().to_string(),
        trigger_event_summary: trigger_event_summary.clone(),
        scene_name: previous_package.scene_name.clone(),
        decision_id: previous_package.decision_id.clone(),
        decision_ref: previous_package.decision_ref.clone(),
        approval_ref: previous_package.approval_ref.clone(),
        symbol: previous_package.symbol.clone(),
        analysis_date: previous_package.analysis_date.clone(),
        decision_status: previous_package.package_status.clone(),
        approval_status: approval_request_status,
        model_grade_summary: previous_package.model_grade_summary.clone(),
        // 2026-04-11 CST: Carry forward package-level governance summary during
        // revision, because P6 needs versioned packages to preserve shadow
        // counts and blocker context across manifest-only upgrades.
        // Purpose: keep package lineage complete when approval artifacts are
        // revised without changing the underlying governance verdict.
        model_governance_summary: previous_package.model_governance_summary.clone(),
        // 2026-04-12 CST: Carry lifecycle feedback into revised packages, because
        // P8 needs post-review governance actions and attribution to remain visible
        // after condition/execution/review artifacts are attached.
        // Purpose: keep operator-facing lifecycle feedback inside the formal package contract.
        lifecycle_governance_summary: lifecycle_context
            .lifecycle_governance_summary
            .clone()
            .or_else(|| previous_package.lifecycle_governance_summary.clone()),
        // 2026-04-08 CST: 这里沿用上一版 package 的对象图绑定，原因是 Task 1 新增的显式对象图不能在 revision 时丢失；
        // 目的：确保 package 版本升级只更新版本与 manifest，而不破坏已经冻结的正式对象引用。
        position_plan_ref: previous_package.object_graph.position_plan_ref.clone(),
        approval_brief_ref: previous_package.object_graph.approval_brief_ref.clone(),
        scorecard_ref: previous_package.object_graph.scorecard_ref.clone(),
        chair_resolution_ref: resolve_chair_resolution_ref(&previous_package)?,
        condition_review_ref: lifecycle_context
            .condition_review_ref
            .clone()
            .or_else(|| previous_package.object_graph.condition_review_ref.clone()),
        execution_record_ref: lifecycle_context
            .execution_record_ref
            .clone()
            .or_else(|| previous_package.object_graph.execution_record_ref.clone()),
        post_trade_review_ref: lifecycle_context
            .post_trade_review_ref
            .clone()
            .or_else(|| previous_package.object_graph.post_trade_review_ref.clone()),
        decision_card_path: previous_package.object_graph.decision_card_path.clone(),
        approval_request_path: previous_package.object_graph.approval_request_path.clone(),
        position_plan_path: previous_package.object_graph.position_plan_path.clone(),
        approval_brief_path: previous_package.object_graph.approval_brief_path.clone(),
        scorecard_path: previous_package.object_graph.scorecard_path.clone(),
        condition_review_path: lifecycle_context
            .condition_review_path
            .clone()
            .or_else(|| previous_package.object_graph.condition_review_path.clone()),
        execution_record_path: lifecycle_context
            .execution_record_path
            .clone()
            .or_else(|| previous_package.object_graph.execution_record_path.clone()),
        post_trade_review_path: lifecycle_context
            .post_trade_review_path
            .clone()
            .or_else(|| previous_package.object_graph.post_trade_review_path.clone()),
        evidence_hash: previous_package.governance_binding.evidence_hash.clone(),
        governance_hash: previous_package.governance_binding.governance_hash.clone(),
        artifact_manifest: updated_artifact_manifest,
    });

    persist_json(&revised_package_path, &revised_package)?;

    let verification_report_path = if request.reverify_after_revision {
        let verification =
            security_decision_verify_package(&SecurityDecisionVerifyPackageRequest {
                package_path: revised_package_path.to_string_lossy().to_string(),
                approval_brief_signing_key_secret: request
                    .approval_brief_signing_key_secret
                    .clone(),
                approval_brief_signing_key_secret_env: request
                    .approval_brief_signing_key_secret_env
                    .clone(),
                write_report: true,
            })
            .map_err(|error| SecurityDecisionPackageRevisionError::Revision(error.to_string()))?;
        verification.verification_report_path
    } else {
        None
    };

    Ok(SecurityDecisionPackageRevisionResult {
        decision_package: serde_json::to_value(&revised_package)
            .map_err(|error| SecurityDecisionPackageRevisionError::Revision(error.to_string()))?,
        decision_package_path: revised_package_path.to_string_lossy().to_string(),
        package_version: revised_package.package_version,
        previous_package_path: previous_package_path.to_string_lossy().to_string(),
        revision_reason: revised_package.revision_reason.clone(),
        trigger_event_summary,
        verification_report_path,
    })
}

fn rebuild_artifact_manifest(
    previous_artifacts: &[SecurityDecisionPackageArtifact],
    lifecycle_context: &LifecycleAttachmentContext,
) -> Result<Vec<SecurityDecisionPackageArtifact>, SecurityDecisionPackageRevisionError> {
    let mut rebuilt = Vec::new();
    for artifact in previous_artifacts {
        if !artifact.present || artifact.path.trim().is_empty() {
            rebuilt.push(SecurityDecisionPackageArtifact {
                artifact_role: artifact.artifact_role.clone(),
                path: artifact.path.clone(),
                sha256: String::new(),
                contract_version: artifact.contract_version.clone(),
                required: artifact.required,
                present: false,
            });
            continue;
        }

        let payload = fs::read(&artifact.path)
            .map_err(|error| SecurityDecisionPackageRevisionError::Revision(error.to_string()))?;
        let sha256 = compute_manifest_compatible_sha256(artifact, &payload)
            .map_err(SecurityDecisionPackageRevisionError::Revision)?;
        rebuilt.push(SecurityDecisionPackageArtifact {
            artifact_role: artifact.artifact_role.clone(),
            path: artifact.path.clone(),
            sha256,
            contract_version: artifact.contract_version.clone(),
            required: artifact.required,
            present: true,
        });
    }
    upsert_lifecycle_artifact(
        &mut rebuilt,
        "condition_review",
        "security_condition_review.v1",
        lifecycle_context.condition_review_path.as_deref(),
        lifecycle_context.condition_review_value.as_ref(),
    )?;
    upsert_lifecycle_artifact(
        &mut rebuilt,
        "execution_record",
        "security_execution_record.v1",
        lifecycle_context.execution_record_path.as_deref(),
        lifecycle_context.execution_record_value.as_ref(),
    )?;
    upsert_lifecycle_artifact(
        &mut rebuilt,
        "post_trade_review",
        "security_post_trade_review.v1",
        lifecycle_context.post_trade_review_path.as_deref(),
        lifecycle_context.post_trade_review_value.as_ref(),
    )?;
    Ok(rebuilt)
}

fn resolve_chair_resolution_ref(
    previous_package: &SecurityDecisionPackageDocument,
) -> Result<Option<String>, SecurityDecisionPackageRevisionError> {
    if previous_package.object_graph.chair_resolution_ref.is_some() {
        return Ok(previous_package.object_graph.chair_resolution_ref.clone());
    }

    let Some(artifact) = previous_package
        .artifact_manifest
        .iter()
        .find(|artifact| artifact.artifact_role == "security_post_meeting_conclusion")
    else {
        return Ok(None);
    };
    if !artifact.present || artifact.path.trim().is_empty() {
        return Ok(None);
    }

    let payload = fs::read(&artifact.path)
        .map_err(|error| SecurityDecisionPackageRevisionError::Revision(error.to_string()))?;
    let document = serde_json::from_slice::<SecurityPostMeetingConclusionDocument>(&payload)
        .map_err(|error| SecurityDecisionPackageRevisionError::Revision(error.to_string()))?;

    if document.chair_resolution_ref.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(document.chair_resolution_ref))
    }
}

// 2026-04-12 CST: Load optional lifecycle attachments during package revision,
// because P8 needs review/execution/post-trade artifacts to become package refs
// only after those lifecycle events actually happen.
// Purpose: centralize binding validation and payload loading before package persistence.
fn load_lifecycle_attachment_context(
    request: &SecurityDecisionPackageRevisionRequest,
    previous_package: &SecurityDecisionPackageDocument,
) -> Result<LifecycleAttachmentContext, SecurityDecisionPackageRevisionError> {
    let condition_review = load_optional_json_file::<SecurityConditionReviewDocument>(
        request.condition_review_path.as_deref(),
    )?;
    if let Some(document) = condition_review.as_ref() {
        validate_condition_review_binding(document, previous_package)?;
    }

    let execution_record = load_optional_json_file::<SecurityExecutionRecordDocument>(
        request.execution_record_path.as_deref(),
    )?;
    if let Some(document) = execution_record.as_ref() {
        validate_execution_record_binding(document, previous_package, condition_review.as_ref())?;
    }

    let post_trade_review = load_optional_json_file::<SecurityPostTradeReviewDocument>(
        request.post_trade_review_path.as_deref(),
    )?;
    if let Some(document) = post_trade_review.as_ref() {
        validate_post_trade_review_binding(document, previous_package, execution_record.as_ref())?;
    }

    let lifecycle_governance_summary = build_lifecycle_governance_summary(
        condition_review.as_ref(),
        execution_record.as_ref(),
        post_trade_review.as_ref(),
    );

    Ok(LifecycleAttachmentContext {
        condition_review_ref: condition_review
            .as_ref()
            .map(|document| document.condition_review_id.clone()),
        execution_record_ref: execution_record
            .as_ref()
            .map(|document| document.execution_record_id.clone()),
        post_trade_review_ref: post_trade_review
            .as_ref()
            // 2026-04-14 CST: 这里改用正式 review_id，原因是 post_trade_review 合同已从
            // post_trade_review_id 收敛成 review_id，revision 还停留在旧字段。
            // 目的：让 package revision 直接消费当前正式复盘对象，不再强拉旧合同回退。
            .map(|document| document.review_id.clone()),
        condition_review_path: request.condition_review_path.clone(),
        execution_record_path: request.execution_record_path.clone(),
        post_trade_review_path: request.post_trade_review_path.clone(),
        condition_review_value: condition_review
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|error| SecurityDecisionPackageRevisionError::Revision(error.to_string()))?,
        execution_record_value: execution_record
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|error| SecurityDecisionPackageRevisionError::Revision(error.to_string()))?,
        post_trade_review_value: post_trade_review
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|error| SecurityDecisionPackageRevisionError::Revision(error.to_string()))?,
        lifecycle_governance_summary,
    })
}

fn validate_condition_review_binding(
    document: &SecurityConditionReviewDocument,
    previous_package: &SecurityDecisionPackageDocument,
) -> Result<(), SecurityDecisionPackageRevisionError> {
    if document.binding.decision_ref != previous_package.decision_ref
        || document.binding.approval_ref != previous_package.approval_ref
        || document.binding.position_plan_ref != previous_package.object_graph.position_plan_ref
    {
        return Err(SecurityDecisionPackageRevisionError::Revision(
            "condition review binding does not match the package object graph".to_string(),
        ));
    }
    Ok(())
}

fn validate_execution_record_binding(
    document: &SecurityExecutionRecordDocument,
    previous_package: &SecurityDecisionPackageDocument,
    condition_review: Option<&SecurityConditionReviewDocument>,
) -> Result<(), SecurityDecisionPackageRevisionError> {
    // 2026-04-14 CST: 这里切到 execution_record 当前正式字段校验，原因是 binding.* 已经从
    // 新合同里移除，旧 revision 校验因此整段失效。
    // 目的：保留最关键的一致性约束，只校验 package 可验证的 symbol / analysis_date / plan ref。
    if document.symbol != previous_package.symbol
        || document.analysis_date != previous_package.analysis_date
        || document.position_plan_ref != previous_package.object_graph.position_plan_ref
    {
        return Err(SecurityDecisionPackageRevisionError::Revision(
            "execution record binding does not match the package object graph".to_string(),
        ));
    }
    if let Some(review) = condition_review {
        if review.binding.position_plan_ref != document.position_plan_ref {
            return Err(SecurityDecisionPackageRevisionError::Revision(
                "execution record position plan ref does not match the attached condition review"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

fn validate_post_trade_review_binding(
    document: &SecurityPostTradeReviewDocument,
    previous_package: &SecurityDecisionPackageDocument,
    execution_record: Option<&SecurityExecutionRecordDocument>,
) -> Result<(), SecurityDecisionPackageRevisionError> {
    // 2026-04-14 CST: 这里改成 post_trade_review 当前正式字段校验，原因是新合同已去掉 binding。
    // 目的：继续守住 package/object graph 的关键引用关系，同时避免旧字段把 revision 链卡死。
    if document.symbol != previous_package.symbol
        || document.analysis_date != previous_package.analysis_date
        || document.position_plan_ref != previous_package.object_graph.position_plan_ref
    {
        return Err(SecurityDecisionPackageRevisionError::Revision(
            "post trade review binding does not match the package object graph".to_string(),
        ));
    }
    if let Some(expected_execution_record) = execution_record {
        if document.execution_record_ref != expected_execution_record.execution_record_id {
            return Err(SecurityDecisionPackageRevisionError::Revision(
                "post trade review execution record ref does not match the attached execution record"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

fn build_lifecycle_governance_summary(
    condition_review: Option<&SecurityConditionReviewDocument>,
    execution_record: Option<&SecurityExecutionRecordDocument>,
    post_trade_review: Option<&SecurityPostTradeReviewDocument>,
) -> Option<SecurityDecisionPackageLifecycleGovernanceSummary> {
    let lifecycle_status = if let Some(review) = post_trade_review {
        review.review_status.clone()
    } else if let Some(record) = execution_record {
        // 2026-04-14 CST: 这里把旧 execution_status 映射为 position_state，原因是 execution
        // record 正式合同已经统一用持仓状态表达生命周期阶段。
        // 目的：先维持 package 生命周期摘要可生成，后续如需更细状态再单独设计映射表。
        record.position_state.clone()
    } else if let Some(review) = condition_review {
        review.review_status.clone()
    } else {
        return None;
    };

    // 2026-04-14 CST: 这里先保守退化 attribution/recommended action，原因是 post_trade_review
    // 新合同已不再内嵌 attribution 结构与 recommended_governance_action 字段。
    // 目的：先让 revision/package 能消费当前复盘合同，后续若要恢复细粒度归因，再由复盘对象显式补正式字段。
    let attribution_layers = Vec::new();

    Some(SecurityDecisionPackageLifecycleGovernanceSummary {
        lifecycle_status,
        condition_review_ref: condition_review.map(|review| review.condition_review_id.clone()),
        execution_record_ref: execution_record.map(|record| record.execution_record_id.clone()),
        post_trade_review_ref: post_trade_review.map(|review| review.review_id.clone()),
        recommended_governance_action: post_trade_review
            .and_then(|review| review.next_account_adjustment_hint.clone())
            .or_else(|| post_trade_review.map(|review| review.next_adjustment_hint.clone())),
        attribution_layers,
    })
}

fn load_optional_json_file<T: serde::de::DeserializeOwned>(
    path: Option<&str>,
) -> Result<Option<T>, SecurityDecisionPackageRevisionError> {
    let Some(path) = path.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let payload = fs::read(path)
        .map_err(|error| SecurityDecisionPackageRevisionError::Revision(error.to_string()))?;
    let value = serde_json::from_slice::<T>(&payload)
        .map_err(|error| SecurityDecisionPackageRevisionError::Revision(error.to_string()))?;
    Ok(Some(value))
}

fn upsert_lifecycle_artifact(
    artifacts: &mut Vec<SecurityDecisionPackageArtifact>,
    artifact_role: &str,
    contract_version: &str,
    path: Option<&str>,
    json_value: Option<&Value>,
) -> Result<(), SecurityDecisionPackageRevisionError> {
    let Some(path) = path else {
        return Ok(());
    };
    let Some(json_value) = json_value else {
        return Ok(());
    };

    let replacement = SecurityDecisionPackageArtifact {
        artifact_role: artifact_role.to_string(),
        path: path.to_string(),
        sha256: sha256_for_json_value(json_value)
            .map_err(SecurityDecisionPackageRevisionError::Revision)?,
        contract_version: contract_version.to_string(),
        required: false,
        present: true,
    };

    if let Some(existing) = artifacts
        .iter_mut()
        .find(|artifact| artifact.artifact_role == artifact_role)
    {
        *existing = replacement;
    } else {
        artifacts.push(replacement);
    }

    Ok(())
}

fn compute_manifest_compatible_sha256(
    artifact: &SecurityDecisionPackageArtifact,
    payload: &[u8],
) -> Result<String, String> {
    if artifact.path.ends_with(".json") {
        let value: Value = serde_json::from_slice(payload).map_err(|error| error.to_string())?;
        return sha256_for_json_value(&value);
    }
    Ok(sha256_for_bytes(payload))
}

fn infer_trigger_event_summary(artifacts: &[SecurityDecisionPackageArtifact]) -> String {
    let Some(events_artifact) = artifacts
        .iter()
        .find(|artifact| artifact.artifact_role == "approval_events" && artifact.present)
    else {
        return "approval package revised without approval event summary".to_string();
    };
    let Ok(payload) = fs::read(&events_artifact.path) else {
        return "approval package revised without approval event summary".to_string();
    };
    let Ok(value) = serde_json::from_slice::<Value>(&payload) else {
        return "approval package revised without approval event summary".to_string();
    };
    let Some(last_event) = value.as_array().and_then(|items| items.last()) else {
        return "approval package revised without approval event summary".to_string();
    };
    let reviewer = last_event
        .get("reviewer")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown_reviewer");
    let action = last_event
        .get("action")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown_action");
    let timestamp = last_event
        .get("timestamp")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown_timestamp");
    format!("{reviewer} {action} at {timestamp}")
}

fn infer_approval_status(artifacts: &[SecurityDecisionPackageArtifact]) -> Option<String> {
    let artifact = artifacts
        .iter()
        .find(|artifact| artifact.artifact_role == "approval_request" && artifact.present)?;
    let payload = fs::read(&artifact.path).ok()?;
    let value = serde_json::from_slice::<Value>(&payload).ok()?;
    value.get("status")?.as_str().map(|value| value.to_string())
}

fn resolve_revision_package_path(
    previous_package_path: &Path,
    decision_id: &str,
    next_version: u32,
) -> Result<PathBuf, SecurityDecisionPackageRevisionError> {
    let decision_packages_dir = find_decision_packages_dir(previous_package_path)?;
    let version_dir = decision_packages_dir.join(decision_id);
    Ok(version_dir.join(format!("v{next_version}.json")))
}

fn find_decision_packages_dir(
    package_path: &Path,
) -> Result<PathBuf, SecurityDecisionPackageRevisionError> {
    for ancestor in package_path.ancestors() {
        if ancestor
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name == "decision_packages")
            .unwrap_or(false)
        {
            return Ok(ancestor.to_path_buf());
        }
    }
    Err(SecurityDecisionPackageRevisionError::Revision(
        "failed to locate decision_packages directory from package path".to_string(),
    ))
}

fn persist_json<T: Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), SecurityDecisionPackageRevisionError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| SecurityDecisionPackageRevisionError::Revision(error.to_string()))?;
    }
    let payload = serde_json::to_vec_pretty(value)
        .map_err(|error| SecurityDecisionPackageRevisionError::Revision(error.to_string()))?;
    fs::write(path, payload)
        .map_err(|error| SecurityDecisionPackageRevisionError::Revision(error.to_string()))
}

fn default_revision_reason() -> String {
    "approval_state_transition".to_string()
}

fn default_reverify_after_revision() -> bool {
    true
}
