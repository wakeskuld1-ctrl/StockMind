use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// 2026-04-25 CST: Added because the raw capital-flow audit route is registered
// on the stock bus while the module file is absent.
// Reason: branch recovery should preserve the route contract without inventing audit truth.
// Purpose: expose an explicit pending-restoration audit artifact.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityCapitalFlowRawAuditRequest {
    #[serde(default = "default_created_at")]
    pub created_at: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub start_date: String,
    #[serde(default)]
    pub end_date: String,
    #[serde(default)]
    pub capital_flow_runtime_root: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityCapitalFlowRawAuditResult {
    pub document_type: String,
    pub generated_at: String,
    pub symbol: String,
    pub status: String,
    pub observations: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Error)]
pub enum SecurityCapitalFlowRawAuditError {}

pub fn security_capital_flow_raw_audit(
    request: &SecurityCapitalFlowRawAuditRequest,
) -> Result<SecurityCapitalFlowRawAuditResult, SecurityCapitalFlowRawAuditError> {
    Ok(SecurityCapitalFlowRawAuditResult {
        document_type: "security_capital_flow_raw_audit".to_string(),
        generated_at: normalize_created_at(&request.created_at),
        symbol: request.symbol.trim().to_string(),
        status: "contract_restored_pending_audit_adapter".to_string(),
        observations: Vec::new(),
        summary:
            "raw capital-flow audit contract restored; source-specific audit replay remains pending"
                .to_string(),
    })
}

fn normalize_created_at(created_at: &str) -> String {
    let trimmed = created_at.trim();
    if trimmed.is_empty() {
        Utc::now().to_rfc3339()
    } else {
        trimmed.to_string()
    }
}

fn default_created_at() -> String {
    Utc::now().to_rfc3339()
}
