use serde::{Deserialize, Serialize};
use thiserror::Error;

// 2026-04-12 CST: Add the formal condition-review request contract, because P8
// needs intraperiod review to become a replayable stock object instead of a
// conversational note.
// Purpose: keep trigger metadata and lifecycle bindings stable for CLI and later package wiring.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityConditionReviewRequest {
    pub symbol: String,
    pub analysis_date: String,
    pub decision_ref: String,
    pub approval_ref: String,
    pub position_plan_ref: String,
    #[serde(default)]
    pub decision_package_path: Option<String>,
    pub review_trigger_type: String,
    pub review_trigger_summary: String,
    pub created_at: String,
}

// 2026-04-12 CST: Add the formal review binding block, because downstream
// execution and replay objects must inherit the same refs without re-inferring them.
// Purpose: expose one stable location for lifecycle linkage fields.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityConditionReviewBinding {
    pub decision_ref: String,
    pub approval_ref: String,
    pub position_plan_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_package_path: Option<String>,
}

// 2026-04-12 CST: Add the formal condition-review document, because P8 starts
// by making review triggers first-class stock artifacts.
// Purpose: preserve trigger, status, follow-up action, and bindings in a stable contract.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityConditionReviewDocument {
    pub contract_version: String,
    pub document_type: String,
    pub condition_review_id: String,
    pub symbol: String,
    pub analysis_date: String,
    pub review_trigger_type: String,
    pub review_trigger_summary: String,
    pub review_status: String,
    pub recommended_follow_up_action: String,
    pub review_notes: Vec<String>,
    pub binding: SecurityConditionReviewBinding,
    pub created_at: String,
}

// 2026-04-12 CST: Wrap the review document in a stable result envelope, because
// other stock lifecycle tools already return named payload objects.
// Purpose: keep the CLI contract extensible without changing the outer response shape.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityConditionReviewResult {
    pub condition_review: SecurityConditionReviewDocument,
}

#[derive(Debug, Error)]
pub enum SecurityConditionReviewError {
    #[error("condition review analysis_date cannot be empty")]
    EmptyAnalysisDate,
    #[error("condition review trigger type cannot be empty")]
    EmptyTriggerType,
    #[error("condition review trigger summary cannot be empty")]
    EmptyTriggerSummary,
}

// 2026-04-12 CST: Add the first minimal formal review builder, because P8 needs
// deterministic lifecycle review semantics before execution and post-trade tools arrive.
// Purpose: produce a replay-friendly document with governed next-step vocabulary.
pub fn security_condition_review(
    request: &SecurityConditionReviewRequest,
) -> Result<SecurityConditionReviewResult, SecurityConditionReviewError> {
    if request.analysis_date.trim().is_empty() {
        return Err(SecurityConditionReviewError::EmptyAnalysisDate);
    }
    if request.review_trigger_type.trim().is_empty() {
        return Err(SecurityConditionReviewError::EmptyTriggerType);
    }
    if request.review_trigger_summary.trim().is_empty() {
        return Err(SecurityConditionReviewError::EmptyTriggerSummary);
    }

    let normalized_trigger_type = request.review_trigger_type.trim().to_ascii_lowercase();
    let follow_up_action =
        derive_follow_up_action(&normalized_trigger_type, &request.review_trigger_summary);
    let review_status = if follow_up_action == "freeze_execution" {
        "escalated"
    } else {
        "recorded"
    };

    Ok(SecurityConditionReviewResult {
        condition_review: SecurityConditionReviewDocument {
            contract_version: "security_condition_review.v1".to_string(),
            document_type: "security_condition_review".to_string(),
            condition_review_id: format!(
                "condition-review:{}:{}:{}:v1",
                request.symbol, request.analysis_date, normalized_trigger_type
            ),
            symbol: request.symbol.clone(),
            analysis_date: request.analysis_date.clone(),
            review_trigger_type: normalized_trigger_type,
            review_trigger_summary: request.review_trigger_summary.clone(),
            review_status: review_status.to_string(),
            recommended_follow_up_action: follow_up_action.to_string(),
            review_notes: build_review_notes(follow_up_action),
            binding: SecurityConditionReviewBinding {
                decision_ref: request.decision_ref.clone(),
                approval_ref: request.approval_ref.clone(),
                position_plan_ref: request.position_plan_ref.clone(),
                decision_package_path: request.decision_package_path.clone(),
            },
            created_at: request.created_at.clone(),
        },
    })
}

// 2026-04-12 CST: Keep the first-pass action derivation rule-based, because P8
// only needs a deterministic action vocabulary before more advanced replay logic is added.
// Purpose: prevent the new tool from returning free-form lifecycle actions.
fn derive_follow_up_action(trigger_type: &str, trigger_summary: &str) -> &'static str {
    if trigger_summary.contains("冻结")
        || trigger_summary.contains("停牌")
        || trigger_summary.contains("止损")
        || trigger_summary.contains("重大负面")
    {
        return "freeze_execution";
    }

    match trigger_type {
        "manual_review" => "keep_plan",
        "end_of_day_review" => "update_position_plan",
        "event_review" => "reopen_committee",
        "data_staleness_review" => "reopen_research",
        _ => "request_more_evidence",
    }
}

// 2026-04-12 CST: Emit short structured notes, because replay and audit consumers
// need a stable minimal reasoning trail even in the first implementation.
// Purpose: keep review output machine-readable and later report-friendly.
fn build_review_notes(follow_up_action: &str) -> Vec<String> {
    match follow_up_action {
        "freeze_execution" => vec![
            "review escalated into a freeze action".to_string(),
            "execution must pause until follow-up governance completes".to_string(),
        ],
        "update_position_plan" => {
            vec!["review suggests the current position plan should be refreshed".to_string()]
        }
        "reopen_committee" => {
            vec!["review requires the committee to revisit the existing decision".to_string()]
        }
        "reopen_research" => {
            vec!["review indicates supporting research is stale and must be refreshed".to_string()]
        }
        "request_more_evidence" => {
            vec!["review could not determine a stronger governed action".to_string()]
        }
        _ => vec!["review preserved the current plan pending further lifecycle events".to_string()],
    }
}
