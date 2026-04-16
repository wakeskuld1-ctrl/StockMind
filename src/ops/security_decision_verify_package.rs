use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::ops::stock::security_approval_brief_signature::{
    verify_security_approval_brief_document, SecurityApprovalBriefSignatureEnvelope,
};
use crate::ops::stock::security_decision_approval_bridge::{
    PersistedApprovalRequest, PersistedDecisionCard,
};
use crate::ops::stock::security_decision_approval_brief::SecurityDecisionApprovalBrief;
use crate::ops::stock::security_decision_package::{
    sha256_for_bytes, SecurityDecisionPackageArtifact, SecurityDecisionPackageDocument,
};
use crate::ops::stock::security_position_plan::SecurityPositionPlan;
use crate::ops::stock::security_record_post_meeting_conclusion::SecurityPostMeetingConclusionDocument;
use crate::ops::stock::security_scorecard::SecurityScorecardDocument;

// 2026-04-02 CST: 这里定义证券审批包校验请求，原因是 P0-5 需要一个正式 Tool 来执行 package 路径、签名 secret 和报告落盘策略；
// 目的：把 verify 所需的最小输入参数收口到稳定合同，避免调用方手工拼接内部校验细节。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityDecisionVerifyPackageRequest {
    pub package_path: String,
    #[serde(default)]
    pub approval_brief_signing_key_secret: Option<String>,
    #[serde(default)]
    pub approval_brief_signing_key_secret_env: Option<String>,
    #[serde(default = "default_write_report")]
    pub write_report: bool,
}

// 2026-04-02 CST: 这里定义证券审批包校验结果，原因是调用方需要同时拿到报告正文与落盘路径；
// 目的：让 CLI / Skill / 后续审批治理都能一次获得“是否有效、为什么、报告在哪”。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityDecisionVerifyPackageResult {
    pub report_id: String,
    pub contract_version: String,
    pub generated_at: String,
    pub package_path: String,
    pub package_id: String,
    pub decision_ref: String,
    pub approval_ref: String,
    pub package_valid: bool,
    pub artifact_checks: Vec<SecurityDecisionPackageArtifactCheck>,
    pub hash_checks: Vec<SecurityDecisionPackageHashCheck>,
    pub signature_checks: Vec<SecurityDecisionPackageSignatureCheck>,
    pub governance_checks: SecurityDecisionPackageGovernanceCheck,
    pub issues: Vec<String>,
    pub recommended_action: String,
    pub verification_report_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityDecisionPackageArtifactCheck {
    pub artifact_role: String,
    pub path: String,
    pub required: bool,
    pub present: bool,
    pub exists_on_disk: bool,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityDecisionPackageHashCheck {
    pub artifact_role: String,
    pub manifest_sha256: String,
    pub actual_sha256: String,
    pub matched: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityDecisionPackageSignatureCheck {
    pub artifact_role: String,
    pub algorithm: String,
    pub key_id: String,
    pub payload_sha256_matched: bool,
    pub signature_valid: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityDecisionPackageGovernanceCheck {
    pub decision_ref_matched: bool,
    pub approval_ref_matched: bool,
    pub evidence_hash_matched: bool,
    pub governance_hash_matched: bool,
    pub object_graph_consistent: bool,
    pub post_meeting_binding_consistent: bool,
    pub scorecard_binding_consistent: bool,
    pub scorecard_complete: bool,
    pub scorecard_action_aligned: bool,
    pub position_plan_binding_consistent: bool,
    pub position_plan_complete: bool,
    pub position_plan_direction_aligned: bool,
}

// 2026-04-02 CST: 这里定义校验错误边界，原因是 verify 阶段既可能失败在路径解析，也可能失败在落盘；
// 目的：让 dispatcher 继续拿到单一错误口径，同时把“包本身无效”和“工具执行失败”分开。
#[derive(Debug, thiserror::Error)]
pub enum SecurityDecisionVerifyPackageError {
    #[error("证券审批包校验执行失败: {0}")]
    Verify(String),
}

// 2026-04-02 CST: 这里实现正式证券审批包校验入口，原因是 P0-5 要把 package 从“可生成”升级成“可核验”；
// 目的：统一执行 manifest、哈希、签名与治理绑定校验，并生成正式 verification report。
pub fn security_decision_verify_package(
    request: &SecurityDecisionVerifyPackageRequest,
) -> Result<SecurityDecisionVerifyPackageResult, SecurityDecisionVerifyPackageError> {
    let package_path = PathBuf::from(request.package_path.trim());
    if request.package_path.trim().is_empty() {
        return Err(SecurityDecisionVerifyPackageError::Verify(
            "package_path cannot be empty".to_string(),
        ));
    }

    let package: SecurityDecisionPackageDocument = serde_json::from_slice(
        &fs::read(&package_path)
            .map_err(|error| SecurityDecisionVerifyPackageError::Verify(error.to_string()))?,
    )
    .map_err(|error| SecurityDecisionVerifyPackageError::Verify(error.to_string()))?;

    let mut issues = Vec::new();
    let artifact_checks = build_artifact_checks(&package.artifact_manifest, &mut issues);
    let hash_checks = build_hash_checks(&package.artifact_manifest, &mut issues);
    let signature_checks =
        build_signature_checks(&package.artifact_manifest, request, &mut issues)?;
    let governance_checks =
        build_governance_checks(&package, &package.artifact_manifest, &mut issues)?;
    let package_valid = issues.is_empty();
    let recommended_action = recommend_action(package_valid, &artifact_checks, &signature_checks);

    let mut result = SecurityDecisionVerifyPackageResult {
        report_id: format!("verification-{}", package.decision_id),
        contract_version: "security_decision_package_verification.v1".to_string(),
        generated_at: Utc::now().to_rfc3339(),
        package_path: package_path.to_string_lossy().to_string(),
        package_id: package.package_id.clone(),
        decision_ref: package.decision_ref.clone(),
        approval_ref: package.approval_ref.clone(),
        package_valid,
        artifact_checks,
        hash_checks,
        signature_checks,
        governance_checks,
        issues,
        recommended_action,
        verification_report_path: None,
    };

    if request.write_report {
        let report_path = resolve_verification_report_path(&package_path, &package.decision_id)?;
        persist_json(&report_path, &result)?;
        result.verification_report_path = Some(report_path.to_string_lossy().to_string());
    }

    Ok(result)
}

fn build_artifact_checks(
    artifacts: &[SecurityDecisionPackageArtifact],
    issues: &mut Vec<String>,
) -> Vec<SecurityDecisionPackageArtifactCheck> {
    let mut checks = Vec::new();
    for artifact in artifacts {
        let exists_on_disk = artifact.present
            && !artifact.path.trim().is_empty()
            && Path::new(&artifact.path).exists();
        let (status, message) = if artifact.required && !artifact.present {
            issues.push(format!(
                "required artifact `{}` is not present",
                artifact.artifact_role
            ));
            (
                "failed".to_string(),
                "required artifact missing from manifest".to_string(),
            )
        } else if artifact.present && !exists_on_disk {
            issues.push(format!(
                "artifact `{}` expected at `{}` but file does not exist",
                artifact.artifact_role, artifact.path
            ));
            (
                "failed".to_string(),
                "artifact file missing on disk".to_string(),
            )
        } else if !artifact.present && !artifact.required {
            (
                "warning".to_string(),
                "optional artifact not present".to_string(),
            )
        } else {
            ("passed".to_string(), "artifact present on disk".to_string())
        };
        checks.push(SecurityDecisionPackageArtifactCheck {
            artifact_role: artifact.artifact_role.clone(),
            path: artifact.path.clone(),
            required: artifact.required,
            present: artifact.present,
            exists_on_disk,
            status,
            message,
        });
    }
    checks
}

fn build_hash_checks(
    artifacts: &[SecurityDecisionPackageArtifact],
    issues: &mut Vec<String>,
) -> Vec<SecurityDecisionPackageHashCheck> {
    let mut checks = Vec::new();
    for artifact in artifacts {
        if !artifact.present || artifact.path.trim().is_empty() {
            continue;
        }
        let actual_sha256 = match fs::read(&artifact.path) {
            Ok(payload) => compute_manifest_compatible_sha256(artifact, &payload),
            Err(_) => String::new(),
        };
        let matched = !actual_sha256.is_empty() && actual_sha256 == artifact.sha256;
        if !matched {
            issues.push(format!(
                "artifact `{}` sha256 mismatch or unreadable",
                artifact.artifact_role
            ));
        }
        checks.push(SecurityDecisionPackageHashCheck {
            artifact_role: artifact.artifact_role.clone(),
            manifest_sha256: artifact.sha256.clone(),
            actual_sha256,
            matched,
        });
    }
    checks
}

// 2026-04-02 CST: 这里把 verify 阶段的哈希口径对齐到 package manifest，原因是 manifest 中 JSON 工件的哈希来自结构化 payload 而不是 pretty 文件字节；
// 目的：让 happy path 既能准确复现提交时摘要，又不会因为格式化空白差异产生误报；篡改内容时仍会稳定失配。
fn compute_manifest_compatible_sha256(
    artifact: &SecurityDecisionPackageArtifact,
    payload: &[u8],
) -> String {
    if artifact.path.ends_with(".json") {
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(payload) {
            if let Ok(sha256) =
                crate::ops::stock::security_decision_package::sha256_for_json_value(&value)
            {
                return sha256;
            }
        }
    }
    sha256_for_bytes(payload)
}

fn build_signature_checks(
    artifacts: &[SecurityDecisionPackageArtifact],
    request: &SecurityDecisionVerifyPackageRequest,
    issues: &mut Vec<String>,
) -> Result<Vec<SecurityDecisionPackageSignatureCheck>, SecurityDecisionVerifyPackageError> {
    let signature_artifact = artifacts
        .iter()
        .find(|artifact| artifact.artifact_role == "approval_brief_signature");
    let approval_brief_artifact = artifacts
        .iter()
        .find(|artifact| artifact.artifact_role == "approval_brief");

    let Some(signature_artifact) = signature_artifact else {
        return Ok(Vec::new());
    };
    if !signature_artifact.present {
        return Ok(vec![SecurityDecisionPackageSignatureCheck {
            artifact_role: "approval_brief_signature".to_string(),
            algorithm: "hmac_sha256".to_string(),
            key_id: String::new(),
            payload_sha256_matched: true,
            signature_valid: true,
            message: "optional signature artifact not present".to_string(),
        }]);
    }

    let Some(approval_brief_artifact) = approval_brief_artifact else {
        issues.push(
            "approval_brief_signature exists but approval_brief artifact is missing".to_string(),
        );
        return Ok(vec![SecurityDecisionPackageSignatureCheck {
            artifact_role: "approval_brief_signature".to_string(),
            algorithm: "hmac_sha256".to_string(),
            key_id: String::new(),
            payload_sha256_matched: false,
            signature_valid: false,
            message: "approval brief artifact missing".to_string(),
        }]);
    };

    let secret = resolve_optional_signing_secret(request)?;
    let brief: SecurityDecisionApprovalBrief = serde_json::from_slice(
        &fs::read(&approval_brief_artifact.path)
            .map_err(|error| SecurityDecisionVerifyPackageError::Verify(error.to_string()))?,
    )
    .map_err(|error| SecurityDecisionVerifyPackageError::Verify(error.to_string()))?;
    let envelope: SecurityApprovalBriefSignatureEnvelope = serde_json::from_slice(
        &fs::read(&signature_artifact.path)
            .map_err(|error| SecurityDecisionVerifyPackageError::Verify(error.to_string()))?,
    )
    .map_err(|error| SecurityDecisionVerifyPackageError::Verify(error.to_string()))?;

    let verification = verify_security_approval_brief_document(&brief, &envelope, &secret);
    let (payload_sha256_matched, signature_valid, message) = match verification {
        Ok(()) => (
            true,
            true,
            "approval brief detached signature verified".to_string(),
        ),
        Err(error) => {
            issues.push(format!(
                "approval brief detached signature verification failed: {error}"
            ));
            let payload_matches = !error.contains("payload sha256 mismatch");
            (payload_matches, false, error)
        }
    };

    Ok(vec![SecurityDecisionPackageSignatureCheck {
        artifact_role: "approval_brief_signature".to_string(),
        algorithm: envelope.algorithm.clone(),
        key_id: envelope.key_id.clone(),
        payload_sha256_matched,
        signature_valid,
        message,
    }])
}

fn build_governance_checks(
    package: &SecurityDecisionPackageDocument,
    artifacts: &[SecurityDecisionPackageArtifact],
    issues: &mut Vec<String>,
) -> Result<SecurityDecisionPackageGovernanceCheck, SecurityDecisionVerifyPackageError> {
    let decision_card = load_optional_json::<PersistedDecisionCard>(artifacts, "decision_card")?;
    let approval_request =
        load_optional_json::<PersistedApprovalRequest>(artifacts, "approval_request")?;
    let position_plan = load_optional_json::<SecurityPositionPlan>(artifacts, "position_plan")?;
    let approval_brief =
        load_optional_json::<SecurityDecisionApprovalBrief>(artifacts, "approval_brief")?;
    let post_meeting_conclusion = load_optional_json::<SecurityPostMeetingConclusionDocument>(
        artifacts,
        "security_post_meeting_conclusion",
    )?;
    let scorecard =
        load_optional_json::<SecurityScorecardDocument>(artifacts, "security_scorecard")?;

    let decision_ref_matched = decision_card
        .as_ref()
        .map(|card| card.decision_ref == package.decision_ref)
        .unwrap_or(false)
        && approval_request
            .as_ref()
            .and_then(|request| request.decision_ref.clone())
            .map(|value| value == package.decision_ref)
            .unwrap_or(false)
        && approval_brief
            .as_ref()
            .map(|brief| brief.decision_ref == package.decision_ref)
            .unwrap_or(false);
    if !decision_ref_matched {
        issues.push("governance decision_ref mismatch across package artifacts".to_string());
    }

    let approval_ref_matched = approval_request
        .as_ref()
        .map(|request| request.approval_ref == package.approval_ref)
        .unwrap_or(false)
        && approval_brief
            .as_ref()
            .map(|brief| brief.approval_ref == package.approval_ref)
            .unwrap_or(false);
    if !approval_ref_matched {
        issues.push("governance approval_ref mismatch across package artifacts".to_string());
    }

    let evidence_hash_matched = approval_request
        .as_ref()
        .and_then(|request| request.evidence_hash.clone())
        .map(|value| value == package.governance_binding.evidence_hash)
        .unwrap_or(false)
        && approval_brief
            .as_ref()
            .map(|brief| brief.evidence_hash == package.governance_binding.evidence_hash)
            .unwrap_or(false);
    if !evidence_hash_matched {
        issues.push("governance evidence_hash mismatch across package artifacts".to_string());
    }

    let governance_hash_matched = approval_request
        .as_ref()
        .and_then(|request| request.governance_hash.clone())
        .map(|value| value == package.governance_binding.governance_hash)
        .unwrap_or(false)
        && approval_brief
            .as_ref()
            .map(|brief| brief.governance_hash == package.governance_binding.governance_hash)
            .unwrap_or(false);
    if !governance_hash_matched {
        issues.push("governance governance_hash mismatch across package artifacts".to_string());
    }

    // 2026-04-08 CST: 这里新增对象图一致性校验，原因是 Task 1 要把 package 从“文件清单合同”提升为“正式对象图合同”；
    // 目的：就算文件还存在，只要对象引用或对象路径漂移，也能被 verify 明确拦下来。
    let object_graph_consistent = decision_card
        .as_ref()
        .map(|card| {
            package.object_graph.decision_ref == package.decision_ref
                && package.object_graph.decision_ref == card.decision_ref
                && package.object_graph.decision_card_path
                    == find_artifact_path(artifacts, "decision_card")
        })
        .unwrap_or(false)
        && approval_request
            .as_ref()
            .map(|request| {
                package.object_graph.approval_ref == package.approval_ref
                    && request.approval_ref == package.object_graph.approval_ref
                    && request
                        .decision_ref
                        .as_ref()
                        .map(|value| value == &package.object_graph.decision_ref)
                        .unwrap_or(false)
                    && package.object_graph.approval_request_path
                        == find_artifact_path(artifacts, "approval_request")
            })
            .unwrap_or(false)
        && position_plan
            .as_ref()
            .map(|plan| {
                plan.plan_id == package.object_graph.position_plan_ref
                    && plan.decision_ref == package.object_graph.decision_ref
                    && plan.approval_ref == package.object_graph.approval_ref
                    && package.object_graph.position_plan_path
                        == find_artifact_path(artifacts, "position_plan")
            })
            .unwrap_or(false)
        && approval_brief
            .as_ref()
            .map(|brief| {
                brief.brief_id == package.object_graph.approval_brief_ref
                    && brief.decision_ref == package.object_graph.decision_ref
                    && brief.approval_ref == package.object_graph.approval_ref
                    && package.object_graph.approval_brief_path
                        == find_artifact_path(artifacts, "approval_brief")
            })
            .unwrap_or(false)
        && scorecard
            .as_ref()
            .map(|scorecard| {
                scorecard.scorecard_id == package.object_graph.scorecard_ref
                    && scorecard.decision_ref == package.object_graph.decision_ref
                    && scorecard.approval_ref == package.object_graph.approval_ref
                    && package.object_graph.scorecard_path
                        == find_artifact_path(artifacts, "security_scorecard")
            })
            .unwrap_or(false);
    if !object_graph_consistent {
        issues.push("package object_graph mismatch across package artifacts".to_string());
    }

    // 2026-04-16 CST: Added because the current conservative governance round now
    // treats the chair-bound post-meeting conclusion as a first-class downstream
    // governance artifact whenever it appears in the package.
    // Reason: otherwise the final chair anchor can drift silently while still
    // looking like a valid post-meeting summary on disk.
    // Purpose: make verify_package reject empty chair references or basic package
    // anchor drift for post-meeting governance outputs.
    let has_post_meeting_artifact = artifacts.iter().any(|artifact| {
        artifact.artifact_role == "security_post_meeting_conclusion" && artifact.present
    });
    let post_meeting_binding_consistent = if has_post_meeting_artifact {
        post_meeting_conclusion
            .as_ref()
            .map(|document| {
                document.decision_id == package.decision_id
                    && document.symbol == package.symbol
                    && document.analysis_date == package.analysis_date
                    && !document.chair_resolution_ref.trim().is_empty()
                    && package
                        .object_graph
                        .chair_resolution_ref
                        .as_ref()
                        .map(|value| value == &document.chair_resolution_ref)
                        .unwrap_or(false)
            })
            .unwrap_or(false)
    } else {
        true
    };
    if !post_meeting_binding_consistent {
        issues.push(
            "security_post_meeting_conclusion binding mismatch across governance chain".to_string(),
        );
    }

    // 2026-04-09 CST: 这里新增评分卡绑定一致性校验，原因是评分卡已正式进入 package object graph 与 artifact manifest；
    // 目的：确保 scorecard 的 ref/path/approval 链接与 package 主锚点一致，而不是仅仅文件存在就算通过。
    // 2026-04-16 CST: Tighten scorecard identity binding after review found that
    // verify_package still allowed symbol/date/decision drift as long as ref/path matched.
    // Purpose=make approval-package verification reject scorecard metadata tampering, not
    // just broken refs.
    let scorecard_binding_consistent = scorecard
        .as_ref()
        .map(|scorecard| {
            scorecard.symbol == package.symbol
                && scorecard.analysis_date == package.analysis_date
                && scorecard.decision_id == package.decision_id
                && scorecard.decision_ref == package.decision_ref
                && scorecard.approval_ref == package.approval_ref
                && scorecard.scorecard_id == package.object_graph.scorecard_ref
                && package.object_graph.scorecard_path
                    == find_artifact_path(artifacts, "security_scorecard")
        })
        .unwrap_or(false);
    if !scorecard_binding_consistent {
        issues.push("security_scorecard binding mismatch across package artifacts".to_string());
    }

    // 2026-04-09 CST: 这里新增评分卡完整性校验，原因是正式 scorecard 至少要有状态、原始特征快照、限制说明和对象头；
    // 目的：把“有个文件”提升为“这份 scorecard 合同本身是完整的”。
    let scorecard_complete = scorecard
        .as_ref()
        .map(|scorecard| {
            !scorecard.contract_version.trim().is_empty()
                && !scorecard.document_type.trim().is_empty()
                && !scorecard.scorecard_id.trim().is_empty()
                && !scorecard.score_status.trim().is_empty()
                && !scorecard.raw_feature_snapshot.is_empty()
        })
        .unwrap_or(false);
    if !scorecard_complete {
        issues.push("security_scorecard formal contract is incomplete".to_string());
    }

    // 2026-04-09 CST: 这里新增评分卡动作对齐校验，原因是评分卡进入 package 后必须和最终投决动作保持一致；
    // 目的：避免后续出现“投决建议 avoid，但 scorecard 仍写成 buy”这类新的治理漂移。
    let scorecard_action_aligned = decision_card
        .as_ref()
        .zip(scorecard.as_ref())
        .map(|(card, scorecard)| {
            card.recommendation_action == scorecard.recommendation_action
                && card.exposure_side == scorecard.exposure_side
        })
        .unwrap_or(false);
    if !scorecard_action_aligned {
        issues.push("security_scorecard action does not align with decision_card".to_string());
    }

    // 2026-04-08 CST: 这里新增仓位计划审批绑定一致性校验，原因是 Task 2 要让 approval_request 正式声明它审批的是哪一份 position_plan；
    // 目的：确保 approval_request、position_plan 和 package.object_graph 三者围绕同一 plan 保持一致，而不是只在 package 层看得到文件。
    let position_plan_binding_consistent = approval_request
        .as_ref()
        .and_then(|request| request.position_plan_binding.as_ref())
        .zip(position_plan.as_ref())
        .map(|(binding, plan)| {
            let position_plan_artifact_path = find_artifact_path(artifacts, "position_plan");
            let position_plan_artifact_sha = find_artifact_sha(artifacts, "position_plan");
            binding.position_plan_ref == plan.plan_id
                && binding.position_plan_path == position_plan_artifact_path
                && binding.position_plan_contract_version == plan.contract_version
                && binding.position_plan_sha256 == position_plan_artifact_sha
                && binding.plan_status == plan.plan_status
                && binding.plan_direction == plan.plan_direction
                && binding.position_plan_ref == package.object_graph.position_plan_ref
                && binding.position_plan_path == package.object_graph.position_plan_path
                && plan.approval_binding.decision_ref == package.decision_ref
                && plan.approval_binding.approval_ref == package.approval_ref
                && plan.approval_binding.approval_request_ref == package.approval_ref
        })
        .unwrap_or(false);
    if !position_plan_binding_consistent {
        issues.push(
            "approval_request position_plan_binding mismatch across approval chain".to_string(),
        );
    }

    let position_plan_complete = position_plan
        .as_ref()
        .map(|plan| {
            !plan.contract_version.trim().is_empty()
                && !plan.document_type.trim().is_empty()
                && !plan.decision_id.trim().is_empty()
                && !plan.plan_direction.trim().is_empty()
                && !plan.approval_binding.package_scope.trim().is_empty()
                && !plan.approval_binding.binding_status.trim().is_empty()
                && !plan.reduce_plan.trigger_condition.trim().is_empty()
                && !plan.reduce_plan.notes.trim().is_empty()
        })
        .unwrap_or(false);
    if !position_plan_complete {
        issues.push("position_plan formal contract is incomplete".to_string());
    }

    let position_plan_direction_aligned = decision_card
        .as_ref()
        .zip(position_plan.as_ref())
        .map(|(card, plan)| persisted_direction_label(&card.direction) == plan.plan_direction)
        .unwrap_or(false);
    if !position_plan_direction_aligned {
        issues.push(
            "position_plan direction does not align with decision_card direction".to_string(),
        );
    }

    Ok(SecurityDecisionPackageGovernanceCheck {
        decision_ref_matched,
        approval_ref_matched,
        evidence_hash_matched,
        governance_hash_matched,
        object_graph_consistent,
        post_meeting_binding_consistent,
        scorecard_binding_consistent,
        scorecard_complete,
        scorecard_action_aligned,
        position_plan_binding_consistent,
        position_plan_complete,
        position_plan_direction_aligned,
    })
}

fn find_artifact_path(
    artifacts: &[SecurityDecisionPackageArtifact],
    artifact_role: &str,
) -> String {
    artifacts
        .iter()
        .find(|artifact| artifact.artifact_role == artifact_role)
        .map(|artifact| artifact.path.clone())
        .unwrap_or_default()
}

fn find_artifact_sha(artifacts: &[SecurityDecisionPackageArtifact], artifact_role: &str) -> String {
    artifacts
        .iter()
        .find(|artifact| artifact.artifact_role == artifact_role)
        .map(|artifact| artifact.sha256.clone())
        .unwrap_or_default()
}

fn persisted_direction_label(
    direction: &crate::ops::stock::security_decision_approval_bridge::PersistedDecisionDirection,
) -> String {
    match direction {
        crate::ops::stock::security_decision_approval_bridge::PersistedDecisionDirection::Long => {
            "Long".to_string()
        }
        crate::ops::stock::security_decision_approval_bridge::PersistedDecisionDirection::Short => {
            "Short".to_string()
        }
        crate::ops::stock::security_decision_approval_bridge::PersistedDecisionDirection::Hedge => {
            "Hedge".to_string()
        }
        crate::ops::stock::security_decision_approval_bridge::PersistedDecisionDirection::NoTrade => {
            "NoTrade".to_string()
        }
    }
}

fn load_optional_json<T: for<'de> Deserialize<'de>>(
    artifacts: &[SecurityDecisionPackageArtifact],
    artifact_role: &str,
) -> Result<Option<T>, SecurityDecisionVerifyPackageError> {
    let Some(artifact) = artifacts
        .iter()
        .find(|artifact| artifact.artifact_role == artifact_role)
    else {
        return Ok(None);
    };
    if !artifact.present || artifact.path.trim().is_empty() {
        return Ok(None);
    }

    let payload = match fs::read(&artifact.path) {
        // 2026-04-16 CST: Reason=missing governed artifacts should degrade to a
        // formal package verification failure instead of aborting the whole
        // tool run. Purpose=let downstream artifact/hash/governance checks
        // surface the package as invalid while keeping the verify API stable.
        Ok(payload) => payload,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(SecurityDecisionVerifyPackageError::Verify(
                error.to_string(),
            ));
        }
    };
    let value = serde_json::from_slice(&payload)
        .map_err(|error| SecurityDecisionVerifyPackageError::Verify(error.to_string()))?;
    Ok(Some(value))
}

fn recommend_action(
    package_valid: bool,
    artifact_checks: &[SecurityDecisionPackageArtifactCheck],
    signature_checks: &[SecurityDecisionPackageSignatureCheck],
) -> String {
    if !package_valid {
        return "quarantine_and_rebuild".to_string();
    }
    let has_optional_warning = artifact_checks.iter().any(|item| item.status == "warning")
        || signature_checks.iter().any(|item| !item.signature_valid);
    if has_optional_warning {
        "review_with_warning".to_string()
    } else {
        "proceed_with_review".to_string()
    }
}

fn resolve_optional_signing_secret(
    request: &SecurityDecisionVerifyPackageRequest,
) -> Result<String, SecurityDecisionVerifyPackageError> {
    if let Some(secret) = request.approval_brief_signing_key_secret.as_ref() {
        if !secret.trim().is_empty() {
            return Ok(secret.trim().to_string());
        }
        return Err(SecurityDecisionVerifyPackageError::Verify(
            "approval brief verification secret cannot be empty".to_string(),
        ));
    }

    if let Some(env_key) = request.approval_brief_signing_key_secret_env.as_ref() {
        let value = std::env::var(env_key)
            .map_err(|error| SecurityDecisionVerifyPackageError::Verify(error.to_string()))?;
        if !value.trim().is_empty() {
            return Ok(value);
        }
    }

    Err(SecurityDecisionVerifyPackageError::Verify(
        "approval brief signature exists but no verification secret was provided".to_string(),
    ))
}

fn resolve_verification_report_path(
    package_path: &Path,
    decision_id: &str,
) -> Result<PathBuf, SecurityDecisionVerifyPackageError> {
    let runtime_root = package_path
        .parent()
        .and_then(|path| path.parent())
        .ok_or_else(|| {
            SecurityDecisionVerifyPackageError::Verify(
                "failed to derive runtime root from package path".to_string(),
            )
        })?;
    Ok(runtime_root
        .join("decision_packages_verification")
        .join(format!("{decision_id}.verification.json")))
}

fn persist_json<T: Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), SecurityDecisionVerifyPackageError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| SecurityDecisionVerifyPackageError::Verify(error.to_string()))?;
    }
    let payload = serde_json::to_vec_pretty(value)
        .map_err(|error| SecurityDecisionVerifyPackageError::Verify(error.to_string()))?;
    fs::write(path, payload)
        .map_err(|error| SecurityDecisionVerifyPackageError::Verify(error.to_string()))
}

fn default_write_report() -> bool {
    true
}
