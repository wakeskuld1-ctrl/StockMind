use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// 2026-04-25 CST: Added because the standalone capital-source audit route is
// registered while the implementation file is absent after merge.
// Reason: this recovery must not overclaim factor backtest truth from missing logic.
// Purpose: preserve route/type contracts with an explicit pending-audit result.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityCapitalSourceFactorAuditRequest {
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
    #[serde(default)]
    pub price_history_runtime_root: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityCapitalSourceFactorAuditResult {
    pub document_type: String,
    pub generated_at: String,
    pub symbol: String,
    pub status: String,
    pub factor_reports: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Error)]
pub enum SecurityCapitalSourceFactorAuditError {}

pub fn security_capital_source_factor_audit(
    request: &SecurityCapitalSourceFactorAuditRequest,
) -> Result<SecurityCapitalSourceFactorAuditResult, SecurityCapitalSourceFactorAuditError> {
    Ok(SecurityCapitalSourceFactorAuditResult {
        document_type: "security_capital_source_factor_audit".to_string(),
        generated_at: normalize_created_at(&request.created_at),
        symbol: request.symbol.trim().to_string(),
        status: "contract_restored_pending_factor_audit".to_string(),
        factor_reports: Vec::new(),
        summary: "capital-source factor audit contract restored; factor replay remains pending"
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
