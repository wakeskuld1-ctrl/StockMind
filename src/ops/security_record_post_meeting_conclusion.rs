use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_chair_resolution::{
    SecurityChairResolutionError, SecurityChairResolutionRequest, SecurityChairResolutionResult,
    security_chair_resolution,
};

// 2026-04-09 CST: 这里新增会后结论请求合同，原因是 Task 6 需要把“主席决议之后的正式落地结论”
// 从口头描述升级成正式 Tool；目的：给 package / verify / audit 提供稳定可挂接的会后文档入口。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPostMeetingConclusionRequest {
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
    pub execution_notes: Vec<String>,
    #[serde(default)]
    pub follow_up_actions: Vec<String>,
}

// 2026-04-09 CST: 这里固化正式会后结论文档，原因是用户明确要求“会后结论”不能只存在文档口径里，
// 必须成为可验证、可挂接的正式对象；目的：让 package 能显式引用最终执行意见和后续跟踪动作。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPostMeetingConclusionDocument {
    pub post_meeting_conclusion_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub symbol: String,
    pub analysis_date: String,
    pub decision_id: String,
    pub chair_resolution_ref: String,
    pub final_action: String,
    pub chair_process_action: String,
    pub final_trading_stance: String,
    pub final_exposure_side: String,
    pub final_confidence: f64,
    pub revision_required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_to_stage: Option<String>,
    pub execution_notes: Vec<String>,
    pub follow_up_actions: Vec<String>,
    pub conclusion_summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityPostMeetingConclusionResult {
    pub chair_resolution_result: SecurityChairResolutionResult,
    pub post_meeting_conclusion: SecurityPostMeetingConclusionDocument,
}

#[derive(Debug, Error)]
pub enum SecurityPostMeetingConclusionError {
    #[error("security post meeting conclusion chair preparation failed: {0}")]
    Chair(#[from] SecurityChairResolutionError),
}

pub fn security_record_post_meeting_conclusion(
    request: &SecurityPostMeetingConclusionRequest,
) -> Result<SecurityPostMeetingConclusionResult, SecurityPostMeetingConclusionError> {
    let chair_request = SecurityChairResolutionRequest {
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
        created_at: request.created_at.clone(),
        scorecard_model_path: request.scorecard_model_path.clone(),
        independent_advice_document: None,
        // 2026-04-13 CST: 这里显式补空独立建议，原因是主席请求合同已升级为可接独立建议输入；
        // 目的：保持旧会后结论主链继续可编译，同时不在这里隐式创造额外建议源。
        independent_advice: None,
    };
    let chair_resolution_result = security_chair_resolution(&chair_request)?;
    let post_meeting_conclusion =
        build_security_post_meeting_conclusion(&chair_resolution_result, request);

    Ok(SecurityPostMeetingConclusionResult {
        chair_resolution_result,
        post_meeting_conclusion,
    })
}

// 2026-04-09 CST: 这里单独暴露 builder，原因是 decision_package 需要复用同一套会后结论装配逻辑，
// 目的：避免 post_meeting Tool 和 package Tool 维护两套语义。
pub fn build_security_post_meeting_conclusion(
    chair_resolution_result: &SecurityChairResolutionResult,
    request: &SecurityPostMeetingConclusionRequest,
) -> SecurityPostMeetingConclusionDocument {
    let chair = &chair_resolution_result.chair_resolution;
    let execution_notes = normalize_lines(&request.execution_notes, &chair.execution_constraints);
    let follow_up_actions = normalize_lines(
        &request.follow_up_actions,
        &chair_resolution_result
            .committee_result
            .decision_card
            .required_next_actions,
    );

    SecurityPostMeetingConclusionDocument {
        post_meeting_conclusion_id: format!("post-meeting-{}", chair.decision_id),
        contract_version: "security_post_meeting_conclusion.v1".to_string(),
        document_type: "security_post_meeting_conclusion".to_string(),
        generated_at: normalize_created_at(&request.created_at),
        symbol: chair.symbol.clone(),
        analysis_date: chair.analysis_date.clone(),
        decision_id: chair.decision_id.clone(),
        chair_resolution_ref: chair.chair_resolution_id.clone(),
        final_action: chair.selected_action.clone(),
        chair_process_action: chair.final_action.clone(),
        final_trading_stance: chair.final_stance.clone(),
        final_exposure_side: chair.selected_exposure_side.clone(),
        final_confidence: chair.final_confidence,
        revision_required: chair.revision_required,
        return_to_stage: chair.return_to_stage.clone(),
        execution_notes,
        follow_up_actions,
        conclusion_summary: format!(
            "会后结论确认 `{}` 为当前正式执行动作，后续跟踪锚定于主席决议 `{}`。",
            chair.selected_action, chair.chair_resolution_id
        ),
    }
}

fn normalize_lines(preferred: &[String], fallback: &[String]) -> Vec<String> {
    let mut lines = Vec::new();
    for item in preferred.iter().chain(fallback.iter()) {
        let value = item.trim();
        if value.is_empty() {
            continue;
        }
        if !lines.iter().any(|existing| existing == value) {
            lines.push(value.to_string());
        }
    }
    lines
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
