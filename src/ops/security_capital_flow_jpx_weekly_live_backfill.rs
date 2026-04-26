use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// 2026-04-25 CST: Added because the JPX live backfill route is registered on
// the stock bus while its module file is absent.
// Reason: recovery must not silently perform network crawling without a fresh contract.
// Purpose: preserve the public route as an explicit pending-adapter artifact.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityCapitalFlowJpxWeeklyLiveBackfillRequest {
    #[serde(default = "default_created_at")]
    pub created_at: String,
    #[serde(default)]
    pub archive_url: Option<String>,
    #[serde(default)]
    pub capital_flow_runtime_root: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityCapitalFlowJpxWeeklyLiveBackfillResult {
    pub document_type: String,
    pub generated_at: String,
    pub downloaded_file_count: usize,
    pub status: String,
    pub summary: String,
}

#[derive(Debug, Error)]
pub enum SecurityCapitalFlowJpxWeeklyLiveBackfillError {}

pub fn security_capital_flow_jpx_weekly_live_backfill(
    request: &SecurityCapitalFlowJpxWeeklyLiveBackfillRequest,
) -> Result<
    SecurityCapitalFlowJpxWeeklyLiveBackfillResult,
    SecurityCapitalFlowJpxWeeklyLiveBackfillError,
> {
    Ok(SecurityCapitalFlowJpxWeeklyLiveBackfillResult {
        document_type: "security_capital_flow_jpx_weekly_live_backfill".to_string(),
        generated_at: normalize_created_at(&request.created_at),
        downloaded_file_count: 0,
        status: "contract_restored_pending_jpx_live_adapter".to_string(),
        summary: "JPX live backfill contract restored; archive crawling requires a separate approved recovery"
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
