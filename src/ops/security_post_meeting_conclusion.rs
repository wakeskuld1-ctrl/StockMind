use chrono::Utc;
use serde::{Deserialize, Serialize};

// 2026-04-08 CST: 这里新增正式会后结论对象合同，原因是 Task 3 需要把“会后采纳结论”从隐式状态升级为独立可落盘对象；
// 目的：让后续 revision、verify、Skill 展示层都围绕统一合同消费，而不是继续散落在 approval_request/status 或事件摘要里。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPostMeetingConclusion {
    pub conclusion_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub scene_name: String,
    pub decision_id: String,
    pub decision_ref: String,
    pub approval_ref: String,
    pub symbol: String,
    pub analysis_date: String,
    pub source_package_path: String,
    pub source_package_version: u32,
    pub source_brief_ref: String,
    pub final_disposition: String,
    pub disposition_reason: String,
    pub key_reasons: Vec<String>,
    pub required_follow_ups: Vec<String>,
    pub reviewer_notes: String,
    pub reviewer: String,
    pub reviewer_role: String,
    pub revision_reason: String,
    pub governance_binding: SecurityPostMeetingGovernanceBinding,
    pub brief_pairing: SecurityPostMeetingBriefPairing,
}

// 2026-04-08 CST: 这里把会后结论与 package 治理链的绑定显式收口，原因是后续 verify 需要知道这份结论绑定到哪个 package 版本；
// 目的：为下一轮把 post_meeting_conclusion 正式挂入 object_graph / artifact_manifest 预留稳定字段。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPostMeetingGovernanceBinding {
    pub decision_ref: String,
    pub approval_ref: String,
    pub decision_id: String,
    pub source_package_path: String,
    pub source_package_version: u32,
    pub binding_status: String,
}

// 2026-04-08 CST: 这里保留会前 brief 与会后结论的轻量配对关系，原因是 Task 3 设计要求形成“会前/会后”成对治理对象；
// 目的：先把配对元数据稳定下来，后续再把 brief 本体上的 pairing 字段补齐。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPostMeetingBriefPairing {
    pub pre_meeting_brief_ref: String,
    pub pre_meeting_brief_path: String,
    pub pairing_status: String,
    pub pairing_summary: String,
}

// 2026-04-08 CST: 这里集中定义 builder 输入，原因是会后结论会被 Tool、回放、后续回填脚本多处复用；
// 目的：避免调用方自行拼字段导致合同漂移。
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityPostMeetingConclusionBuildInput {
    pub generated_at: String,
    pub scene_name: String,
    pub decision_id: String,
    pub decision_ref: String,
    pub approval_ref: String,
    pub symbol: String,
    pub analysis_date: String,
    pub source_package_path: String,
    pub source_package_version: u32,
    pub source_brief_ref: String,
    pub source_brief_path: String,
    pub final_disposition: String,
    pub disposition_reason: String,
    pub key_reasons: Vec<String>,
    pub required_follow_ups: Vec<String>,
    pub reviewer_notes: String,
    pub reviewer: String,
    pub reviewer_role: String,
    pub revision_reason: String,
}

// 2026-04-08 CST: 这里提供最小 builder，原因是红测当前只需要“正式对象可构建并落盘”；
// 目的：先冻结对象边界，后续再在不破坏合同的前提下扩字段。
pub fn build_security_post_meeting_conclusion(
    input: SecurityPostMeetingConclusionBuildInput,
) -> SecurityPostMeetingConclusion {
    SecurityPostMeetingConclusion {
        conclusion_id: format!("post-conclusion-{}", input.decision_id),
        contract_version: "security_post_meeting_conclusion.v1".to_string(),
        document_type: "security_post_meeting_conclusion".to_string(),
        generated_at: normalize_generated_at(&input.generated_at),
        scene_name: input.scene_name,
        decision_id: input.decision_id.clone(),
        decision_ref: input.decision_ref.clone(),
        approval_ref: input.approval_ref.clone(),
        symbol: input.symbol,
        analysis_date: input.analysis_date,
        source_package_path: input.source_package_path.clone(),
        source_package_version: input.source_package_version.max(1),
        source_brief_ref: input.source_brief_ref.clone(),
        final_disposition: input.final_disposition,
        disposition_reason: input.disposition_reason,
        key_reasons: input.key_reasons,
        required_follow_ups: input.required_follow_ups,
        reviewer_notes: input.reviewer_notes,
        reviewer: input.reviewer,
        reviewer_role: input.reviewer_role,
        revision_reason: input.revision_reason,
        governance_binding: SecurityPostMeetingGovernanceBinding {
            decision_ref: input.decision_ref,
            approval_ref: input.approval_ref,
            decision_id: input.decision_id,
            source_package_path: input.source_package_path,
            source_package_version: input.source_package_version.max(1),
            binding_status: "bound_to_source_package".to_string(),
        },
        brief_pairing: SecurityPostMeetingBriefPairing {
            pre_meeting_brief_ref: input.source_brief_ref,
            pre_meeting_brief_path: input.source_brief_path,
            pairing_status: "paired_with_pre_meeting_brief".to_string(),
            pairing_summary: "post meeting conclusion paired with approval brief".to_string(),
        },
    }
}

fn normalize_generated_at(value: &str) -> String {
    if value.trim().is_empty() {
        Utc::now().to_rfc3339()
    } else {
        value.trim().to_string()
    }
}
