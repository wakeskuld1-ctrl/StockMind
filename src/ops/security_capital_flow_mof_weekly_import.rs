use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// 2026-04-25 CST: Added because the MOF weekly import route is registered on
// the stock bus while its implementation file is absent.
// Reason: restore compile-time contract shape without fabricating CSV parsing behavior.
// Purpose: mark MOF source import as pending a separate adapter restoration.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityCapitalFlowMofWeeklyImportRequest {
    #[serde(default = "default_created_at")]
    pub created_at: String,
    #[serde(default)]
    pub csv_path: String,
    #[serde(default)]
    pub capital_flow_runtime_root: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityCapitalFlowMofWeeklyImportResult {
    pub document_type: String,
    pub generated_at: String,
    pub imported_row_count: usize,
    pub status: String,
    pub summary: String,
}

#[derive(Debug, Error)]
pub enum SecurityCapitalFlowMofWeeklyImportError {}

pub fn security_capital_flow_mof_weekly_import(
    request: &SecurityCapitalFlowMofWeeklyImportRequest,
) -> Result<SecurityCapitalFlowMofWeeklyImportResult, SecurityCapitalFlowMofWeeklyImportError> {
    Ok(SecurityCapitalFlowMofWeeklyImportResult {
        document_type: "security_capital_flow_mof_weekly_import".to_string(),
        generated_at: normalize_created_at(&request.created_at),
        imported_row_count: 0,
        status: "contract_restored_pending_mof_adapter".to_string(),
        summary:
            "MOF weekly import contract restored; CSV parsing requires a separate approved recovery"
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
