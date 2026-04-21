use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_execution_record::{
    SecurityExecutionRecordRequest, load_planned_entry_price, security_execution_record,
};
use crate::ops::stock::security_portfolio_execution_request_enrichment::{
    SecurityExecutionApplyContext, SecurityPortfolioEnrichedExecutionRequestRow,
    SecurityPortfolioExecutionRequestEnrichmentDocument,
};
use crate::runtime::stock_history_store::StockHistoryStore;

const SECURITY_PORTFOLIO_EXECUTION_APPLY_BRIDGE_DOCUMENT_TYPE: &str =
    "security_portfolio_execution_apply_bridge";
const SECURITY_PORTFOLIO_EXECUTION_APPLY_BRIDGE_VERSION: &str =
    "security_portfolio_execution_apply_bridge.v1";
const DEFAULT_REVIEW_HORIZON_DAYS: usize = 20;
const DEFAULT_LOOKBACK_DAYS: usize = 260;
const DEFAULT_FACTOR_LOOKBACK_DAYS: usize = 120;
const DEFAULT_DISCLOSURE_LIMIT: usize = 6;
const DEFAULT_STOP_LOSS_PCT: f64 = 0.05;
const DEFAULT_TARGET_RETURN_PCT: f64 = 0.12;

// 2026-04-21 CST: Added because P15 now needs one explicit public request shell
// downstream of the governed P14 enrichment document.
// Reason: the approved route allows apply only from the formal enrichment bundle.
// Purpose: define the public request contract for the P15 apply bridge.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityPortfolioExecutionApplyBridgeRequest {
    pub portfolio_execution_request_enrichment: SecurityPortfolioExecutionRequestEnrichmentDocument,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-21 CST: Added because P15 must return one explicit row-level apply
// status surface instead of making callers infer runtime writes from prose.
// Reason: the apply bridge needs auditable per-row refs and skip semantics.
// Purpose: define the reusable apply row emitted by the P15 bridge.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionApplyRow {
    pub symbol: String,
    pub request_action: String,
    pub requested_gross_pct: f64,
    pub enrichment_status: String,
    pub apply_status: String,
    #[serde(default)]
    pub execution_record_ref: Option<String>,
    #[serde(default)]
    pub execution_journal_ref: Option<String>,
    pub apply_summary: String,
}

// 2026-04-21 CST: Added because P15 needs one formal batch-level apply
// document after runtime-backed execution records are created.
// Reason: later review and handoff work should consume one explicit apply artifact.
// Purpose: define the first governed execution apply-bridge document.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionApplyBridgeDocument {
    pub portfolio_execution_apply_bridge_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub analysis_date: String,
    pub account_id: String,
    pub portfolio_execution_request_enrichment_ref: String,
    pub portfolio_execution_request_package_ref: String,
    pub portfolio_execution_preview_ref: String,
    pub portfolio_allocation_decision_ref: String,
    pub apply_rows: Vec<SecurityPortfolioExecutionApplyRow>,
    pub applied_count: usize,
    pub skipped_hold_count: usize,
    pub failed_apply_count: usize,
    pub apply_status: String,
    pub blockers: Vec<String>,
    pub non_atomicity_notice: String,
    pub apply_rationale: Vec<String>,
    pub apply_summary: String,
}

// 2026-04-21 CST: Added because the public stock tool route should return one
// named wrapper instead of a bare apply-bridge document.
// Reason: keeping a stable top-level shell matches the existing stock tool style.
// Purpose: wrap the P15 apply document in one extensible result shape.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionApplyBridgeResult {
    pub portfolio_execution_apply_bridge: SecurityPortfolioExecutionApplyBridgeDocument,
}

// 2026-04-21 CST: Added because P15 must reject malformed enrichment bundles
// before the first runtime write starts.
// Reason: the approved route requires bundle-level preflight instead of hidden repair.
// Purpose: keep P15 failures explicit and auditable.
#[derive(Debug, Error)]
pub enum SecurityPortfolioExecutionApplyBridgeError {
    #[error(
        "security portfolio execution apply bridge build failed: account id is missing from the enrichment document"
    )]
    MissingAccountId,
    #[error(
        "security portfolio execution apply bridge build failed: enrichment ref is missing from the enrichment document"
    )]
    MissingEnrichmentRef,
    #[error(
        "security portfolio execution apply bridge build failed: request package ref is missing from the enrichment document"
    )]
    MissingRequestPackageRef,
    #[error(
        "security portfolio execution apply bridge build failed: preview ref is missing from the enrichment document"
    )]
    MissingPreviewRef,
    #[error(
        "security portfolio execution apply bridge build failed: allocation decision ref is missing from the enrichment document"
    )]
    MissingAllocationDecisionRef,
    #[error(
        "security portfolio execution apply bridge build failed: blocked rows are present in the enrichment bundle"
    )]
    BlockedBundle,
    #[error(
        "security portfolio execution apply bridge build failed: unsupported enrichment status `{1}` on `{0}`"
    )]
    UnsupportedEnrichmentStatus(String, String),
    #[error(
        "security portfolio execution apply bridge build failed: enrichment summary count mismatch on `{0}` observed `{1}` expected `{2}`"
    )]
    EnrichmentSummaryCountMismatch(String, usize, usize),
    #[error(
        "security portfolio execution apply bridge build failed: execution apply context is missing on `{0}`"
    )]
    MissingExecutionApplyContext(String),
    #[error(
        "security portfolio execution apply bridge build failed: as_of_date is missing on `{0}`"
    )]
    MissingAsOfDate(String),
}

// 2026-04-21 CST: Added because P15 needs one official stock entry point that
// writes execution facts only through the existing execution-record mainline.
// Reason: callers should not rebuild apply orchestration outside the formal stock bus.
// Purpose: expose the P15 apply bridge on the official stock surface.
pub fn security_portfolio_execution_apply_bridge(
    request: &SecurityPortfolioExecutionApplyBridgeRequest,
) -> Result<SecurityPortfolioExecutionApplyBridgeResult, SecurityPortfolioExecutionApplyBridgeError>
{
    build_security_portfolio_execution_apply_bridge(request)
}

pub fn build_security_portfolio_execution_apply_bridge(
    request: &SecurityPortfolioExecutionApplyBridgeRequest,
) -> Result<SecurityPortfolioExecutionApplyBridgeResult, SecurityPortfolioExecutionApplyBridgeError>
{
    validate_portfolio_execution_request_enrichment(
        &request.portfolio_execution_request_enrichment,
    )?;

    let generated_at = normalize_created_at(&request.created_at);
    let document = &request.portfolio_execution_request_enrichment;
    let account_id = document.account_id.trim().to_string();
    let enrichment_ref = document.portfolio_execution_request_enrichment_id.clone();
    let request_package_ref = document.portfolio_execution_request_package_ref.clone();
    let preview_ref = document.portfolio_execution_preview_ref.clone();
    let decision_ref = document.portfolio_allocation_decision_ref.clone();

    let mut applied_count = 0usize;
    let mut skipped_hold_count = 0usize;
    let mut failed_apply_count = 0usize;
    let mut apply_rows = Vec::with_capacity(document.enriched_request_rows.len());
    let store = StockHistoryStore::workspace_default().map_err(|error| {
        SecurityPortfolioExecutionApplyBridgeError::MissingAsOfDate(error.to_string())
    })?;

    for row in &document.enriched_request_rows {
        match row.enrichment_status.as_str() {
            "ready_for_apply" => {
                let execution_request =
                    build_execution_record_request(row, document, &generated_at, &store)?;
                match security_execution_record(&execution_request) {
                    Ok(result) => {
                        applied_count += 1;
                        apply_rows.push(SecurityPortfolioExecutionApplyRow {
                            symbol: row.symbol.clone(),
                            request_action: row.request_action.clone(),
                            requested_gross_pct: row.requested_gross_pct,
                            enrichment_status: row.enrichment_status.clone(),
                            apply_status: "applied".to_string(),
                            execution_record_ref: Some(result.execution_record.execution_record_id),
                            execution_journal_ref: Some(
                                result.execution_journal.execution_journal_id,
                            ),
                            apply_summary: format!(
                                "apply bridge created execution record for {} from enrichment {}",
                                row.symbol, enrichment_ref
                            ),
                        });
                    }
                    Err(error) => {
                        failed_apply_count += 1;
                        apply_rows.push(SecurityPortfolioExecutionApplyRow {
                            symbol: row.symbol.clone(),
                            request_action: row.request_action.clone(),
                            requested_gross_pct: row.requested_gross_pct,
                            enrichment_status: row.enrichment_status.clone(),
                            apply_status: "apply_failed".to_string(),
                            execution_record_ref: None,
                            execution_journal_ref: None,
                            apply_summary: format!(
                                "apply bridge failed on {}: {}",
                                row.symbol, error
                            ),
                        });
                    }
                }
            }
            "non_executable_hold" => {
                skipped_hold_count += 1;
                apply_rows.push(SecurityPortfolioExecutionApplyRow {
                    symbol: row.symbol.clone(),
                    request_action: row.request_action.clone(),
                    requested_gross_pct: row.requested_gross_pct,
                    enrichment_status: row.enrichment_status.clone(),
                    apply_status: "skipped_non_executable_hold".to_string(),
                    execution_record_ref: None,
                    execution_journal_ref: None,
                    apply_summary: format!(
                        "apply bridge skipped non-executable hold row {} from enrichment {}",
                        row.symbol, enrichment_ref
                    ),
                });
            }
            other => {
                return Err(
                    SecurityPortfolioExecutionApplyBridgeError::UnsupportedEnrichmentStatus(
                        row.symbol.clone(),
                        other.to_string(),
                    ),
                );
            }
        }
    }

    let apply_status = if failed_apply_count > 0 && applied_count > 0 {
        "partial_apply_failure"
    } else if failed_apply_count > 0 {
        "failed"
    } else if skipped_hold_count > 0 {
        "applied_with_skipped_holds"
    } else {
        "applied"
    };

    let blockers = if failed_apply_count > 0 {
        vec![format!(
            "apply bridge observed {} runtime apply failures after preflight validation",
            failed_apply_count
        )]
    } else {
        Vec::new()
    };
    let apply_rationale = vec![
        format!(
            "execution apply bridge consumed enrichment bundle {}",
            enrichment_ref
        ),
        "execution apply bridge writes runtime facts only through security_execution_record"
            .to_string(),
        "execution apply bridge does not introduce cross-symbol rollback semantics".to_string(),
    ];

    Ok(SecurityPortfolioExecutionApplyBridgeResult {
        portfolio_execution_apply_bridge: SecurityPortfolioExecutionApplyBridgeDocument {
            portfolio_execution_apply_bridge_id: format!(
                "portfolio-execution-apply-bridge:{}:{}",
                account_id, generated_at
            ),
            contract_version: SECURITY_PORTFOLIO_EXECUTION_APPLY_BRIDGE_VERSION.to_string(),
            document_type: SECURITY_PORTFOLIO_EXECUTION_APPLY_BRIDGE_DOCUMENT_TYPE.to_string(),
            generated_at,
            analysis_date: document.analysis_date.clone(),
            account_id: account_id.clone(),
            portfolio_execution_request_enrichment_ref: enrichment_ref.clone(),
            portfolio_execution_request_package_ref: request_package_ref.clone(),
            portfolio_execution_preview_ref: preview_ref.clone(),
            portfolio_allocation_decision_ref: decision_ref.clone(),
            apply_rows,
            applied_count,
            skipped_hold_count,
            failed_apply_count,
            apply_status: apply_status.to_string(),
            blockers,
            non_atomicity_notice: "this phase does not introduce cross-symbol rollback semantics"
                .to_string(),
            apply_rationale,
            apply_summary: format!(
                "account {} applied {} rows, skipped {} hold rows, and observed {} runtime failures from enrichment {}",
                account_id, applied_count, skipped_hold_count, failed_apply_count, enrichment_ref
            ),
        },
    })
}

// 2026-04-21 CST: Added because P15 must perform one full bundle-level preflight
// before any runtime write starts.
// Reason: the approved route rejects blocked or drifted bundles ahead of side effects.
// Purpose: validate the enrichment contract at the apply boundary.
fn validate_portfolio_execution_request_enrichment(
    document: &SecurityPortfolioExecutionRequestEnrichmentDocument,
) -> Result<(), SecurityPortfolioExecutionApplyBridgeError> {
    if document.account_id.trim().is_empty() {
        return Err(SecurityPortfolioExecutionApplyBridgeError::MissingAccountId);
    }
    if document
        .portfolio_execution_request_enrichment_id
        .trim()
        .is_empty()
    {
        return Err(SecurityPortfolioExecutionApplyBridgeError::MissingEnrichmentRef);
    }
    if document
        .portfolio_execution_request_package_ref
        .trim()
        .is_empty()
    {
        return Err(SecurityPortfolioExecutionApplyBridgeError::MissingRequestPackageRef);
    }
    if document.portfolio_execution_preview_ref.trim().is_empty() {
        return Err(SecurityPortfolioExecutionApplyBridgeError::MissingPreviewRef);
    }
    if document.portfolio_allocation_decision_ref.trim().is_empty() {
        return Err(SecurityPortfolioExecutionApplyBridgeError::MissingAllocationDecisionRef);
    }
    if document.blocked_enrichment_count > 0 || document.readiness_status == "blocked" {
        return Err(SecurityPortfolioExecutionApplyBridgeError::BlockedBundle);
    }

    let mut observed_ready_for_apply_count = 0usize;
    let mut observed_non_executable_hold_count = 0usize;
    let mut observed_blocked_count = 0usize;

    for row in &document.enriched_request_rows {
        match row.enrichment_status.as_str() {
            "ready_for_apply" => observed_ready_for_apply_count += 1,
            "non_executable_hold" => observed_non_executable_hold_count += 1,
            "blocked" => observed_blocked_count += 1,
            other => {
                return Err(
                    SecurityPortfolioExecutionApplyBridgeError::UnsupportedEnrichmentStatus(
                        row.symbol.clone(),
                        other.to_string(),
                    ),
                );
            }
        }
    }

    if observed_ready_for_apply_count != document.ready_for_apply_count {
        return Err(
            SecurityPortfolioExecutionApplyBridgeError::EnrichmentSummaryCountMismatch(
                "ready_for_apply_count".to_string(),
                observed_ready_for_apply_count,
                document.ready_for_apply_count,
            ),
        );
    }
    if observed_non_executable_hold_count != document.non_executable_hold_count {
        return Err(
            SecurityPortfolioExecutionApplyBridgeError::EnrichmentSummaryCountMismatch(
                "non_executable_hold_count".to_string(),
                observed_non_executable_hold_count,
                document.non_executable_hold_count,
            ),
        );
    }
    if observed_blocked_count != document.blocked_enrichment_count {
        return Err(
            SecurityPortfolioExecutionApplyBridgeError::EnrichmentSummaryCountMismatch(
                "blocked_enrichment_count".to_string(),
                observed_blocked_count,
                document.blocked_enrichment_count,
            ),
        );
    }

    Ok(())
}

// 2026-04-21 CST: Added because the thin apply bridge still needs one legal
// execution-record request per ready row without hiding the field mapping.
// Reason: the approved route reuses the existing execution-record mainline instead of a new writer.
// Purpose: map one enriched request row into the legacy execution-record request contract.
fn build_execution_record_request(
    row: &SecurityPortfolioEnrichedExecutionRequestRow,
    document: &SecurityPortfolioExecutionRequestEnrichmentDocument,
    generated_at: &str,
    store: &StockHistoryStore,
) -> Result<SecurityExecutionRecordRequest, SecurityPortfolioExecutionApplyBridgeError> {
    let context = resolve_execution_apply_context(row)?;
    let actual_entry_price = load_planned_entry_price(store, &row.symbol, &context.as_of_date)
        .map_err(|_| {
            SecurityPortfolioExecutionApplyBridgeError::MissingAsOfDate(row.symbol.clone())
        })?;

    Ok(SecurityExecutionRecordRequest {
        symbol: row.symbol.clone(),
        analysis_date: Some(document.analysis_date.clone()),
        decision_ref: Some(row.decision_ref.clone()),
        approval_ref: None,
        position_plan_ref: None,
        condition_review_ref: None,
        execution_action: Some(row.execution_action.clone()),
        execution_status: Some("applied".to_string()),
        executed_gross_pct: Some(row.executed_gross_pct),
        execution_summary: Some(format!(
            "apply bridge executed {} {:.4} gross weight on {} from enrichment {}",
            row.execution_action,
            row.executed_gross_pct,
            row.symbol,
            document.portfolio_execution_request_enrichment_id
        )),
        account_id: Some(document.account_id.clone()),
        sector_tag: Some(context.sector_template.clone()),
        market_symbol: context.market_symbol.clone(),
        sector_symbol: context.sector_symbol.clone(),
        market_regime: context.market_regime.clone(),
        sector_template: context.sector_template.clone(),
        market_profile: context.market_profile.clone(),
        sector_profile: context.sector_profile.clone(),
        as_of_date: Some(context.as_of_date.clone()),
        review_horizon_days: DEFAULT_REVIEW_HORIZON_DAYS,
        lookback_days: DEFAULT_LOOKBACK_DAYS,
        factor_lookback_days: DEFAULT_FACTOR_LOOKBACK_DAYS,
        disclosure_limit: DEFAULT_DISCLOSURE_LIMIT,
        stop_loss_pct: DEFAULT_STOP_LOSS_PCT,
        target_return_pct: DEFAULT_TARGET_RETURN_PCT,
        actual_entry_date: context.as_of_date.clone(),
        actual_entry_price,
        actual_position_pct: row.executed_gross_pct,
        actual_exit_date: String::new(),
        actual_exit_price: 0.0,
        exit_reason: "position_still_open".to_string(),
        execution_trades: Vec::new(),
        execution_journal_notes: Vec::new(),
        execution_record_notes: vec![format!(
            "applied by security_portfolio_execution_apply_bridge on {}",
            generated_at
        )],
        portfolio_position_plan_document: None,
        created_at: generated_at.to_string(),
    })
}

fn resolve_execution_apply_context(
    row: &SecurityPortfolioEnrichedExecutionRequestRow,
) -> Result<&SecurityExecutionApplyContext, SecurityPortfolioExecutionApplyBridgeError> {
    if row.execution_apply_context.as_of_date.trim().is_empty() {
        return Err(SecurityPortfolioExecutionApplyBridgeError::MissingAsOfDate(
            row.symbol.clone(),
        ));
    }
    Ok(&row.execution_apply_context)
}

fn default_created_at() -> String {
    Utc::now().to_rfc3339()
}

fn normalize_created_at(created_at: &str) -> String {
    let trimmed = created_at.trim();
    if trimmed.is_empty() {
        default_created_at()
    } else {
        trimmed.to_string()
    }
}
