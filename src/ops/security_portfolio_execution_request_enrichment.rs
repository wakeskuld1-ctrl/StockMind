use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_portfolio_execution_request_package::{
    SecurityPortfolioExecutionRequestPackageDocument, SecurityPortfolioExecutionRequestRow,
};
use crate::ops::stock::security_symbol_taxonomy::resolve_effective_security_routing;

const SECURITY_PORTFOLIO_EXECUTION_REQUEST_ENRICHMENT_DOCUMENT_TYPE: &str =
    "security_portfolio_execution_request_enrichment";
const SECURITY_PORTFOLIO_EXECUTION_REQUEST_ENRICHMENT_VERSION: &str =
    "security_portfolio_execution_request_enrichment.v1";

// 2026-04-21 CST: Added because P14 needs one explicit public request shell
// downstream of the governed P13 package and upstream of any real execution apply stage.
// Reason: the approved route enriches only the formal request package instead of
// letting callers pass raw preview fragments or execution payloads directly.
// Purpose: define the public request contract for the P14 enrichment bridge.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityPortfolioExecutionRequestEnrichmentRequest {
    pub portfolio_execution_request_package: SecurityPortfolioExecutionRequestPackageDocument,
    pub analysis_date: String,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-21 CST: Added because P14 needs one reusable per-row request shape
// that stays execution-aligned without becoming execution fact.
// Reason: later apply work should consume explicit enriched rows instead of inferring
// readiness from prose or from the original P13 request rows.
// Purpose: define the row contract emitted by the P14 enrichment bridge.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioEnrichedExecutionRequestRow {
    pub symbol: String,
    pub request_action: String,
    pub requested_gross_pct: f64,
    pub request_status: String,
    pub analysis_date: String,
    pub decision_ref: String,
    pub execution_action: String,
    pub execution_status: String,
    pub executed_gross_pct: f64,
    pub execution_summary: String,
    pub enrichment_status: String,
    pub enrichment_summary: String,
    pub execution_apply_context: SecurityExecutionApplyContext,
}

// 2026-04-21 CST: Added because Option A for P15 requires P14 to carry the
// minimum explicit execution-routing context instead of forcing the apply stage
// to reconstruct it from hidden runtime lookups.
// Reason: the approved route keeps P15 thin and governed by extending the
// upstream enrichment contract with deterministic execution context.
// Purpose: define the minimal execution context that the later apply bridge consumes.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityExecutionApplyContext {
    pub as_of_date: String,
    #[serde(default)]
    pub market_symbol: Option<String>,
    #[serde(default)]
    pub sector_symbol: Option<String>,
    #[serde(default)]
    pub market_profile: Option<String>,
    #[serde(default)]
    pub sector_profile: Option<String>,
    pub market_regime: String,
    pub sector_template: String,
}

// 2026-04-21 CST: Added because P14 needs one formal enriched request bundle
// document before any future execution apply bridge can land cleanly.
// Reason: request enrichment should remain explicit and auditable instead of hiding
// new fields inside ad hoc downstream glue code.
// Purpose: define the first execution-request enrichment document.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionRequestEnrichmentDocument {
    pub portfolio_execution_request_enrichment_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub analysis_date: String,
    pub account_id: String,
    pub portfolio_execution_request_package_ref: String,
    pub portfolio_execution_preview_ref: String,
    pub portfolio_allocation_decision_ref: String,
    pub enriched_request_rows: Vec<SecurityPortfolioEnrichedExecutionRequestRow>,
    pub ready_for_apply_count: usize,
    pub non_executable_hold_count: usize,
    pub blocked_enrichment_count: usize,
    pub readiness_status: String,
    pub blockers: Vec<String>,
    pub enrichment_rationale: Vec<String>,
    pub enrichment_summary: String,
}

// 2026-04-21 CST: Added because the public stock tool route should return one
// named wrapper instead of a bare enrichment document.
// Reason: keeping a stable top-level shell matches the existing stock tool style.
// Purpose: wrap the P14 enrichment document in one extensible result shape.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionRequestEnrichmentResult {
    pub portfolio_execution_request_enrichment: SecurityPortfolioExecutionRequestEnrichmentDocument,
}

// 2026-04-21 CST: Added because P14 must reject malformed request-package input
// instead of silently repairing lineage, row semantics, or summary drift.
// Reason: this stage is an enrichment bridge, not a normalization fallback.
// Purpose: keep P14 failures explicit and auditable.
#[derive(Debug, Error)]
pub enum SecurityPortfolioExecutionRequestEnrichmentError {
    #[error(
        "security portfolio execution request enrichment build failed: account id is missing from the request package"
    )]
    MissingAccountId,
    #[error(
        "security portfolio execution request enrichment build failed: request package ref is missing from the request package"
    )]
    MissingRequestPackageRef,
    #[error(
        "security portfolio execution request enrichment build failed: preview ref is missing from the request package"
    )]
    MissingPreviewRef,
    #[error(
        "security portfolio execution request enrichment build failed: allocation decision ref is missing from the request package"
    )]
    MissingAllocationDecisionRef,
    #[error(
        "security portfolio execution request enrichment build failed: analysis date is required"
    )]
    MissingAnalysisDate,
    #[error(
        "security portfolio execution request enrichment build failed: unsupported request action `{1}` on `{0}`"
    )]
    UnsupportedRequestAction(String, String),
    #[error(
        "security portfolio execution request enrichment build failed: unsupported request status `{1}` on `{0}`"
    )]
    UnsupportedRequestStatus(String, String),
    #[error(
        "security portfolio execution request enrichment build failed: request action and request status drift detected on `{0}`"
    )]
    RequestActionStatusDrift(String),
    #[error(
        "security portfolio execution request enrichment build failed: request gross weight drift detected on `{0}`"
    )]
    RequestGrossMismatch(String),
    #[error(
        "security portfolio execution request enrichment build failed: request package summary count mismatch on `{0}` observed `{1}` expected `{2}`"
    )]
    RequestSummaryCountMismatch(String, usize, usize),
}

// 2026-04-21 CST: Added because P14 needs one public stock entry point that
// remains side-effect free while enriching governed request packages.
// Reason: callers should not rebuild enrichment logic outside the formal stock bus.
// Purpose: expose the P14 request-enrichment bridge on the official stock surface.
pub fn security_portfolio_execution_request_enrichment(
    request: &SecurityPortfolioExecutionRequestEnrichmentRequest,
) -> Result<
    SecurityPortfolioExecutionRequestEnrichmentResult,
    SecurityPortfolioExecutionRequestEnrichmentError,
> {
    build_security_portfolio_execution_request_enrichment(request)
}

pub fn build_security_portfolio_execution_request_enrichment(
    request: &SecurityPortfolioExecutionRequestEnrichmentRequest,
) -> Result<
    SecurityPortfolioExecutionRequestEnrichmentResult,
    SecurityPortfolioExecutionRequestEnrichmentError,
> {
    validate_portfolio_execution_request_package(&request.portfolio_execution_request_package)?;

    let analysis_date = normalize_analysis_date(&request.analysis_date)?;
    let generated_at = normalize_created_at(&request.created_at);
    let package = &request.portfolio_execution_request_package;
    let account_id = package.account_id.trim().to_string();
    let package_ref = package.portfolio_execution_request_package_id.clone();
    let preview_ref = package.portfolio_execution_preview_ref.clone();
    let decision_ref = package.portfolio_allocation_decision_ref.clone();

    let mut ready_for_apply_count = 0usize;
    let mut non_executable_hold_count = 0usize;
    let mut blocked_enrichment_count = 0usize;
    let enriched_request_rows = package
        .request_rows
        .iter()
        .map(|row| {
            let derived = derive_enriched_row(row, &analysis_date, &decision_ref, &package_ref);
            if let Ok(ref derived_row) = derived {
                match derived_row.enrichment_status.as_str() {
                    "ready_for_apply" => ready_for_apply_count += 1,
                    "non_executable_hold" => non_executable_hold_count += 1,
                    "blocked" => blocked_enrichment_count += 1,
                    _ => {}
                }
            }
            derived
        })
        .collect::<Result<Vec<_>, _>>()?;

    let readiness_status = if blocked_enrichment_count > 0 {
        "blocked"
    } else {
        "ready"
    };
    let blockers = if blocked_enrichment_count > 0 {
        vec![format!(
            "request package {} contains {} blocked request rows that cannot advance to apply",
            package_ref, blocked_enrichment_count
        )]
    } else {
        Vec::new()
    };
    let enrichment_rationale = vec![
        format!(
            "execution request enrichment derived from request package {}",
            package_ref
        ),
        format!(
            "execution request enrichment preserves preview {} and allocation decision {} lineage",
            preview_ref, decision_ref
        ),
        "execution request enrichment remains side-effect free and does not write execution facts"
            .to_string(),
    ];

    Ok(SecurityPortfolioExecutionRequestEnrichmentResult {
        portfolio_execution_request_enrichment:
            SecurityPortfolioExecutionRequestEnrichmentDocument {
                portfolio_execution_request_enrichment_id: format!(
                    "portfolio-execution-request-enrichment:{}:{}",
                    account_id, generated_at
                ),
                contract_version: SECURITY_PORTFOLIO_EXECUTION_REQUEST_ENRICHMENT_VERSION
                    .to_string(),
                document_type: SECURITY_PORTFOLIO_EXECUTION_REQUEST_ENRICHMENT_DOCUMENT_TYPE
                    .to_string(),
                generated_at,
                analysis_date: analysis_date.clone(),
                account_id: account_id.clone(),
                portfolio_execution_request_package_ref: package_ref.clone(),
                portfolio_execution_preview_ref: preview_ref.clone(),
                portfolio_allocation_decision_ref: decision_ref.clone(),
                enriched_request_rows,
                ready_for_apply_count,
                non_executable_hold_count,
                blocked_enrichment_count,
                readiness_status: readiness_status.to_string(),
                blockers,
                enrichment_rationale,
                enrichment_summary: format!(
                    "account {} enriched {} apply-ready rows, {} non-executable holds, and {} blocked rows from request package {} on {}",
                    account_id,
                    ready_for_apply_count,
                    non_executable_hold_count,
                    blocked_enrichment_count,
                    package_ref,
                    analysis_date
                ),
            },
    })
}

fn validate_portfolio_execution_request_package(
    document: &SecurityPortfolioExecutionRequestPackageDocument,
) -> Result<(), SecurityPortfolioExecutionRequestEnrichmentError> {
    // 2026-04-21 CST: Added because P14 should fail fast when package lineage,
    // row semantics, or summary counts are already malformed upstream.
    // Reason: the enrichment bridge consumes the formal P13 contract and must not
    // repair corrupted package state silently.
    // Purpose: keep P14 bounded to validation plus deterministic enrichment.
    if document.account_id.trim().is_empty() {
        return Err(SecurityPortfolioExecutionRequestEnrichmentError::MissingAccountId);
    }

    if document
        .portfolio_execution_request_package_id
        .trim()
        .is_empty()
    {
        return Err(SecurityPortfolioExecutionRequestEnrichmentError::MissingRequestPackageRef);
    }

    if document.portfolio_execution_preview_ref.trim().is_empty() {
        return Err(SecurityPortfolioExecutionRequestEnrichmentError::MissingPreviewRef);
    }

    if document.portfolio_allocation_decision_ref.trim().is_empty() {
        return Err(SecurityPortfolioExecutionRequestEnrichmentError::MissingAllocationDecisionRef);
    }

    let mut observed_ready_request_count = 0usize;
    let mut observed_blocked_request_count = 0usize;
    let mut observed_hold_request_count = 0usize;

    for row in &document.request_rows {
        validate_request_row(row)?;

        match row.request_status.as_str() {
            "ready_request" => observed_ready_request_count += 1,
            "blocked_request" => observed_blocked_request_count += 1,
            "non_executable_hold" => observed_hold_request_count += 1,
            _ => unreachable!("unsupported request status should have returned earlier"),
        }
    }

    if observed_ready_request_count != document.ready_request_count {
        return Err(
            SecurityPortfolioExecutionRequestEnrichmentError::RequestSummaryCountMismatch(
                "ready_request_count".to_string(),
                observed_ready_request_count,
                document.ready_request_count,
            ),
        );
    }
    if observed_blocked_request_count != document.blocked_request_count {
        return Err(
            SecurityPortfolioExecutionRequestEnrichmentError::RequestSummaryCountMismatch(
                "blocked_request_count".to_string(),
                observed_blocked_request_count,
                document.blocked_request_count,
            ),
        );
    }
    if observed_hold_request_count != document.hold_request_count {
        return Err(
            SecurityPortfolioExecutionRequestEnrichmentError::RequestSummaryCountMismatch(
                "hold_request_count".to_string(),
                observed_hold_request_count,
                document.hold_request_count,
            ),
        );
    }

    Ok(())
}

fn validate_request_row(
    row: &SecurityPortfolioExecutionRequestRow,
) -> Result<(), SecurityPortfolioExecutionRequestEnrichmentError> {
    match row.request_action.as_str() {
        "buy" | "sell" | "hold" => {}
        other => {
            return Err(
                SecurityPortfolioExecutionRequestEnrichmentError::UnsupportedRequestAction(
                    row.symbol.clone(),
                    other.to_string(),
                ),
            );
        }
    }

    match row.request_status.as_str() {
        "ready_request" => {
            if row.request_action == "hold" {
                return Err(
                    SecurityPortfolioExecutionRequestEnrichmentError::RequestActionStatusDrift(
                        row.symbol.clone(),
                    ),
                );
            }
            if row.requested_gross_pct <= 1e-9 {
                return Err(
                    SecurityPortfolioExecutionRequestEnrichmentError::RequestGrossMismatch(
                        row.symbol.clone(),
                    ),
                );
            }
        }
        "non_executable_hold" => {
            if row.request_action != "hold" {
                return Err(
                    SecurityPortfolioExecutionRequestEnrichmentError::RequestActionStatusDrift(
                        row.symbol.clone(),
                    ),
                );
            }
            if !approx_eq(row.requested_gross_pct, 0.0) {
                return Err(
                    SecurityPortfolioExecutionRequestEnrichmentError::RequestGrossMismatch(
                        row.symbol.clone(),
                    ),
                );
            }
        }
        "blocked_request" => {
            if row.request_action == "hold" && !approx_eq(row.requested_gross_pct, 0.0) {
                return Err(
                    SecurityPortfolioExecutionRequestEnrichmentError::RequestGrossMismatch(
                        row.symbol.clone(),
                    ),
                );
            }
            if row.request_action != "hold" && row.requested_gross_pct <= 1e-9 {
                return Err(
                    SecurityPortfolioExecutionRequestEnrichmentError::RequestGrossMismatch(
                        row.symbol.clone(),
                    ),
                );
            }
        }
        other => {
            return Err(
                SecurityPortfolioExecutionRequestEnrichmentError::UnsupportedRequestStatus(
                    row.symbol.clone(),
                    other.to_string(),
                ),
            );
        }
    }

    Ok(())
}

fn derive_enriched_row(
    row: &SecurityPortfolioExecutionRequestRow,
    analysis_date: &str,
    decision_ref: &str,
    package_ref: &str,
) -> Result<
    SecurityPortfolioEnrichedExecutionRequestRow,
    SecurityPortfolioExecutionRequestEnrichmentError,
> {
    validate_request_row(row)?;

    let (
        execution_status,
        enrichment_status,
        executed_gross_pct,
        execution_summary,
        enrichment_summary,
    ) = match row.request_status.as_str() {
        "ready_request" => (
            "ready_for_apply".to_string(),
            "ready_for_apply".to_string(),
            round_pct(row.requested_gross_pct),
            format!(
                "execution {} {:.4} gross weight on {} is ready for apply from request package {}",
                row.request_action, row.requested_gross_pct, row.symbol, package_ref
            ),
            format!(
                "request row enriched for later apply on analysis date {} from decision {}",
                analysis_date, decision_ref
            ),
        ),
        "non_executable_hold" => (
            "non_executable_hold".to_string(),
            "non_executable_hold".to_string(),
            0.0,
            format!(
                "execution hold on {} remains non-executable after request enrichment from package {}",
                row.symbol, package_ref
            ),
            format!(
                "hold row remains explicit and non-executable on analysis date {}",
                analysis_date
            ),
        ),
        "blocked_request" => (
            "blocked".to_string(),
            "blocked".to_string(),
            round_pct(row.requested_gross_pct),
            format!(
                "execution {} {:.4} gross weight on {} remains blocked after request enrichment from package {}",
                row.request_action, row.requested_gross_pct, row.symbol, package_ref
            ),
            format!(
                "blocked request row preserved for later governance review on analysis date {}",
                analysis_date
            ),
        ),
        other => {
            return Err(
                SecurityPortfolioExecutionRequestEnrichmentError::UnsupportedRequestStatus(
                    row.symbol.clone(),
                    other.to_string(),
                ),
            );
        }
    };

    Ok(SecurityPortfolioEnrichedExecutionRequestRow {
        symbol: row.symbol.clone(),
        request_action: row.request_action.clone(),
        requested_gross_pct: round_pct(row.requested_gross_pct),
        request_status: row.request_status.clone(),
        analysis_date: analysis_date.to_string(),
        decision_ref: decision_ref.to_string(),
        execution_action: row.request_action.clone(),
        execution_status,
        executed_gross_pct,
        execution_summary,
        enrichment_status,
        enrichment_summary,
        execution_apply_context: build_execution_apply_context(&row.symbol, analysis_date),
    })
}

// 2026-04-21 CST: Added because P15 now depends on one explicit execution
// context object that can be generated deterministically from governed symbol
// routing plus the approved analysis date.
// Reason: this keeps the later apply bridge thin without widening it into a
// hidden context-reconstruction layer.
// Purpose: derive the minimum execution-routing context directly inside P14.
fn build_execution_apply_context(
    symbol: &str,
    analysis_date: &str,
) -> SecurityExecutionApplyContext {
    let routing = resolve_effective_security_routing(symbol, None, None, None, None);
    let market_symbol = routing
        .market_symbol
        .clone()
        .or_else(|| default_market_symbol(symbol));
    let market_profile = routing
        .market_profile
        .clone()
        .or_else(|| default_market_profile(symbol));
    let sector_symbol = routing
        .sector_symbol
        .clone()
        .or_else(|| market_symbol.clone());
    let sector_profile = routing
        .sector_profile
        .clone()
        .or_else(|| market_profile.clone());

    SecurityExecutionApplyContext {
        as_of_date: analysis_date.to_string(),
        market_symbol,
        sector_symbol,
        market_profile: market_profile.clone(),
        sector_profile,
        market_regime: derive_market_regime(symbol, market_profile.as_deref()),
        sector_template: routing
            .industry_bucket
            .unwrap_or_else(|| "general".to_string()),
    }
}

// 2026-04-21 CST: Added because the thin apply bridge still needs one stable
// market-regime string that older execution contracts already understand.
// Reason: the taxonomy currently resolves profile-level routing, while the
// execution mainline still consumes the coarser market-regime label.
// Purpose: map routed market metadata into the legacy market-regime contract.
fn derive_market_regime(symbol: &str, market_profile: Option<&str>) -> String {
    if market_profile.is_some_and(|value| value.contains("a_share")) {
        return "a_share".to_string();
    }

    if symbol.ends_with(".SH") || symbol.ends_with(".SZ") {
        return "a_share".to_string();
    }

    "general".to_string()
}

// 2026-04-21 CST: Added because some governed A-share symbols used in the
// portfolio chain are still absent from the taxonomy document.
// Reason: P15 should inherit one legal market proxy from P14 instead of failing
// later inside security_execution_record with a missing-market-anchor error.
// Purpose: provide a bounded default market symbol for A-share execution context.
fn default_market_symbol(symbol: &str) -> Option<String> {
    if symbol.ends_with(".SH") || symbol.ends_with(".SZ") {
        return Some("510300.SH".to_string());
    }

    None
}

// 2026-04-21 CST: Added because the same A-share fallback must keep the market
// profile aligned with the default market symbol when taxonomy coverage is missing.
// Reason: downstream execution and outcome tools accept either explicit market_symbol
// or market_profile and should not see an empty market anchor for supported A-share names.
// Purpose: provide a bounded default market profile for A-share execution context.
fn default_market_profile(symbol: &str) -> Option<String> {
    if symbol.ends_with(".SH") || symbol.ends_with(".SZ") {
        return Some("a_share_core_v1".to_string());
    }

    None
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

fn normalize_analysis_date(
    analysis_date: &str,
) -> Result<String, SecurityPortfolioExecutionRequestEnrichmentError> {
    let trimmed = analysis_date.trim();
    if trimmed.is_empty() {
        Err(SecurityPortfolioExecutionRequestEnrichmentError::MissingAnalysisDate)
    } else {
        Ok(trimmed.to_string())
    }
}

fn round_pct(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn approx_eq(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-9
}
