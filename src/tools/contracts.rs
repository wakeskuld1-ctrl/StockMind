use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::ops::stock::security_decision_briefing::PositionPlan;
use crate::tools::catalog;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolRequest {
    pub tool: String,
    #[serde(default)]
    pub args: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolResponse {
    pub status: String,
    pub data: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ToolResponse {
    pub fn ok(data: Value) -> Self {
        Self {
            status: "ok".to_string(),
            data,
            error: None,
        }
    }

    pub fn ok_serialized<T: Serialize>(data: &T) -> Self {
        let serialized =
            serde_json::to_value(data).expect("tool response serialization should succeed");
        Self::ok(serialized)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            data: json!({}),
            error: Some(message.into()),
        }
    }

    pub fn tool_catalog() -> Self {
        Self::ok(json!({
            "tool_catalog": catalog::tool_names(),
            "tool_catalog_modules": {
                "stock": catalog::stock_tool_names(),
            }
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPositionPlanRecordRequest {
    pub decision_ref: String,
    pub approval_ref: String,
    pub evidence_version: String,
    pub symbol: String,
    pub analysis_date: String,
    pub position_plan: PositionPlan,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPositionPlanRecordResult {
    pub position_plan_ref: String,
    pub decision_ref: String,
    pub approval_ref: String,
    pub evidence_version: String,
    pub symbol: String,
    pub analysis_date: String,
    pub position_action: String,
    pub starter_position_pct: f64,
    pub max_position_pct: f64,
    pub position_plan: PositionPlan,
}

impl SecurityPositionPlanRecordResult {
    // 2026-04-16 CST: Added because the split repo keeps the formal position-plan contract intact.
    // Reason: runtime persistence and downstream execution logic still depend on the exact projection shape.
    // Purpose: preserve stable assembly for execution-store and CLI callers during the migration snapshot.
    pub fn from_position_plan(
        position_plan_ref: String,
        request: SecurityPositionPlanRecordRequest,
    ) -> Self {
        let (position_action, starter_position_pct, max_position_pct) =
            request.position_plan.record_projection();

        Self {
            position_plan_ref,
            decision_ref: request.decision_ref,
            approval_ref: request.approval_ref,
            evidence_version: request.evidence_version,
            symbol: request.symbol,
            analysis_date: request.analysis_date,
            position_action: position_action.to_string(),
            starter_position_pct,
            max_position_pct,
            position_plan: request.position_plan,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PositionAdjustmentEventType {
    Build,
    Add,
    Reduce,
    Exit,
    RiskUpdate,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PositionPlanAlignment {
    OnPlan,
    JustifiedDeviation,
    OffPlan,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityRecordPositionAdjustmentRequest {
    pub decision_ref: String,
    pub approval_ref: String,
    pub evidence_version: String,
    pub position_plan_ref: String,
    pub symbol: String,
    pub event_type: PositionAdjustmentEventType,
    pub event_date: String,
    pub before_position_pct: f64,
    pub after_position_pct: f64,
    pub trigger_reason: String,
    pub plan_alignment: PositionPlanAlignment,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityRecordPositionAdjustmentResult {
    pub adjustment_event_ref: String,
    pub decision_ref: String,
    pub approval_ref: String,
    pub evidence_version: String,
    pub position_plan_ref: String,
    pub symbol: String,
    pub event_type: PositionAdjustmentEventType,
    pub event_date: String,
    pub before_position_pct: f64,
    pub after_position_pct: f64,
    pub trigger_reason: String,
    pub plan_alignment: PositionPlanAlignment,
}

impl SecurityRecordPositionAdjustmentResult {
    // 2026-04-16 CST: Added because the split repo needs the same governed adjustment-event DTO.
    // Reason: runtime repositories serialize this exact contract into SQLite payload_json.
    // Purpose: preserve round-trip compatibility for post-trade review and execution-store tests.
    pub fn from_request(
        adjustment_event_ref: String,
        request: SecurityRecordPositionAdjustmentRequest,
    ) -> Self {
        Self {
            adjustment_event_ref,
            decision_ref: request.decision_ref,
            approval_ref: request.approval_ref,
            evidence_version: request.evidence_version,
            position_plan_ref: request.position_plan_ref,
            symbol: request.symbol,
            event_type: request.event_type,
            event_date: request.event_date,
            before_position_pct: request.before_position_pct,
            after_position_pct: request.after_position_pct,
            trigger_reason: request.trigger_reason,
            plan_alignment: request.plan_alignment,
        }
    }
}
