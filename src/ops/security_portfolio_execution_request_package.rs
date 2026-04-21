use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_portfolio_execution_preview::SecurityPortfolioExecutionPreviewDocument;

const SECURITY_PORTFOLIO_EXECUTION_REQUEST_PACKAGE_DOCUMENT_TYPE: &str =
    "security_portfolio_execution_request_package";
const SECURITY_PORTFOLIO_EXECUTION_REQUEST_PACKAGE_VERSION: &str =
    "security_portfolio_execution_request_package.v1";

// 2026-04-20 CST: Added because P13 needs one explicit public request shell
// above the already-governed preview document.
// Reason: the approved route upgrades preview output into a formal request
// package without reopening raw portfolio-core fragments.
// Purpose: define the public request contract for the P13 request bridge.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityPortfolioExecutionRequestPackageRequest {
    pub portfolio_execution_preview: SecurityPortfolioExecutionPreviewDocument,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-20 CST: Added because the request bridge needs one explicit per-row
// request representation distinct from both preview rows and execution facts.
// Reason: later execution stages should consume one formal request row shape
// instead of parsing preview text or inferring hold semantics indirectly.
// Purpose: define the reusable request row emitted by the P13 package.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionRequestRow {
    pub symbol: String,
    pub request_action: String,
    pub requested_gross_pct: f64,
    pub request_status: String,
    pub request_summary: String,
    pub source_preview_action: String,
    pub source_weight_delta_pct: f64,
}

// 2026-04-20 CST: Added because P13 needs one named formal package document
// between the preview-only bridge and any later execution-writing stage.
// Reason: the current mainline should advance with an explicit request package
// instead of treating request derivation as ad hoc downstream glue code.
// Purpose: define the first formal portfolio execution request package.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionRequestPackageDocument {
    pub portfolio_execution_request_package_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub account_id: String,
    pub portfolio_execution_preview_ref: String,
    pub portfolio_allocation_decision_ref: String,
    pub request_rows: Vec<SecurityPortfolioExecutionRequestRow>,
    pub ready_request_count: usize,
    pub blocked_request_count: usize,
    pub hold_request_count: usize,
    pub readiness_status: String,
    pub blockers: Vec<String>,
    pub request_rationale: Vec<String>,
    pub request_summary: String,
}

// 2026-04-20 CST: Added because the public stock tool route should return one
// named wrapper instead of a bare request package document.
// Reason: keeping a stable top-level shell matches the current stock tool style.
// Purpose: wrap the P13 package in one extensible result shape.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionRequestPackageResult {
    pub portfolio_execution_request_package: SecurityPortfolioExecutionRequestPackageDocument,
}

// 2026-04-20 CST: Added because P13 must reject malformed preview input
// instead of silently repairing lineage or action drift.
// Reason: this stage is a request packager, not a fallback normalization layer.
// Purpose: keep P13 failures explicit and auditable.
#[derive(Debug, Error)]
pub enum SecurityPortfolioExecutionRequestPackageError {
    #[error(
        "security portfolio execution request package build failed: account id is missing from the preview document"
    )]
    MissingAccountId,
    #[error(
        "security portfolio execution request package build failed: preview ref is missing from the preview document"
    )]
    MissingPreviewRef,
    #[error(
        "security portfolio execution request package build failed: allocation decision ref is missing from the preview document"
    )]
    MissingAllocationDecisionRef,
    #[error(
        "security portfolio execution request package build failed: unsupported preview action `{1}` on `{0}`"
    )]
    UnsupportedPreviewAction(String, String),
    #[error(
        "security portfolio execution request package build failed: preview action and weight delta drift detected on `{0}`"
    )]
    PreviewActionWeightDrift(String),
    #[error(
        "security portfolio execution request package build failed: preview gross weight drift detected on `{0}`"
    )]
    PreviewGrossMismatch(String),
    #[error(
        "security portfolio execution request package build failed: preview summary count mismatch on `{0}` observed `{1}` expected `{2}`"
    )]
    PreviewSummaryCountMismatch(String, usize, usize),
}

// 2026-04-20 CST: Added because P13 needs one public stock entry point that
// remains side-effect free while formalizing request packaging.
// Reason: callers should not rebuild request-package logic outside the formal bus.
// Purpose: expose the P13 request-package builder on the official stock surface.
pub fn security_portfolio_execution_request_package(
    request: &SecurityPortfolioExecutionRequestPackageRequest,
) -> Result<
    SecurityPortfolioExecutionRequestPackageResult,
    SecurityPortfolioExecutionRequestPackageError,
> {
    build_security_portfolio_execution_request_package(request)
}

pub fn build_security_portfolio_execution_request_package(
    request: &SecurityPortfolioExecutionRequestPackageRequest,
) -> Result<
    SecurityPortfolioExecutionRequestPackageResult,
    SecurityPortfolioExecutionRequestPackageError,
> {
    validate_portfolio_execution_preview(&request.portfolio_execution_preview)?;

    let generated_at = normalize_created_at(&request.created_at);
    let account_id = request
        .portfolio_execution_preview
        .account_id
        .trim()
        .to_string();
    let portfolio_execution_preview_ref = request
        .portfolio_execution_preview
        .portfolio_execution_preview_id
        .clone();
    let portfolio_allocation_decision_ref = request
        .portfolio_execution_preview
        .portfolio_allocation_decision_ref
        .clone();

    let mut ready_request_count = 0usize;
    let blocked_request_count = 0usize;
    let mut hold_request_count = 0usize;
    let request_rows = request
        .portfolio_execution_preview
        .preview_rows
        .iter()
        .map(|row| {
            let (request_action, request_status) = if row.preview_action == "hold" {
                hold_request_count += 1;
                ("hold", "non_executable_hold")
            } else {
                ready_request_count += 1;
                (row.preview_action.as_str(), "ready_request")
            };

            SecurityPortfolioExecutionRequestRow {
                symbol: row.symbol.clone(),
                request_action: request_action.to_string(),
                requested_gross_pct: round_pct(row.weight_delta_pct.abs()),
                request_status: request_status.to_string(),
                request_summary: format!(
                    "request {} {:.4} gross weight on {} from preview {}",
                    request_action,
                    row.weight_delta_pct.abs(),
                    row.symbol,
                    portfolio_execution_preview_ref
                ),
                source_preview_action: row.preview_action.clone(),
                source_weight_delta_pct: row.weight_delta_pct,
            }
        })
        .collect::<Vec<_>>();

    let request_rationale = vec![
        format!(
            "request package derived from preview document {}",
            portfolio_execution_preview_ref
        ),
        format!(
            "request package remains side-effect free and preserves allocation decision {}",
            portfolio_allocation_decision_ref
        ),
    ];

    Ok(SecurityPortfolioExecutionRequestPackageResult {
        portfolio_execution_request_package: SecurityPortfolioExecutionRequestPackageDocument {
            portfolio_execution_request_package_id: format!(
                "portfolio-execution-request-package:{}:{}",
                account_id, generated_at
            ),
            contract_version: SECURITY_PORTFOLIO_EXECUTION_REQUEST_PACKAGE_VERSION.to_string(),
            document_type: SECURITY_PORTFOLIO_EXECUTION_REQUEST_PACKAGE_DOCUMENT_TYPE.to_string(),
            generated_at,
            account_id: account_id.clone(),
            portfolio_execution_preview_ref: portfolio_execution_preview_ref.clone(),
            portfolio_allocation_decision_ref: portfolio_allocation_decision_ref.clone(),
            request_rows,
            ready_request_count,
            blocked_request_count,
            hold_request_count,
            readiness_status: "ready".to_string(),
            blockers: Vec::new(),
            request_rationale,
            request_summary: format!(
                "account {} packaged {} ready requests, {} blocked requests, and {} hold rows from preview {}",
                account_id,
                ready_request_count,
                blocked_request_count,
                hold_request_count,
                portfolio_execution_preview_ref
            ),
        },
    })
}

fn validate_portfolio_execution_preview(
    document: &SecurityPortfolioExecutionPreviewDocument,
) -> Result<(), SecurityPortfolioExecutionRequestPackageError> {
    // 2026-04-20 CST: Added because P13 should fail fast when preview lineage
    // or action semantics are already malformed upstream.
    // Reason: the request bridge is a downstream consumer of preview, not a
    // repair layer for malformed preview documents.
    // Purpose: keep P13 bounded to validation plus request packaging.
    if document.account_id.trim().is_empty() {
        return Err(SecurityPortfolioExecutionRequestPackageError::MissingAccountId);
    }

    if document.portfolio_execution_preview_id.trim().is_empty() {
        return Err(SecurityPortfolioExecutionRequestPackageError::MissingPreviewRef);
    }

    if document.portfolio_allocation_decision_ref.trim().is_empty() {
        return Err(SecurityPortfolioExecutionRequestPackageError::MissingAllocationDecisionRef);
    }

    let mut observed_buy_count = 0usize;
    let mut observed_sell_count = 0usize;
    let mut observed_hold_count = 0usize;

    for row in &document.preview_rows {
        match row.preview_action.as_str() {
            "buy" => {
                observed_buy_count += 1;
                if row.weight_delta_pct <= 1e-9 {
                    return Err(
                        SecurityPortfolioExecutionRequestPackageError::PreviewActionWeightDrift(
                            row.symbol.clone(),
                        ),
                    );
                }
            }
            "sell" => {
                observed_sell_count += 1;
                if row.weight_delta_pct >= -1e-9 {
                    return Err(
                        SecurityPortfolioExecutionRequestPackageError::PreviewActionWeightDrift(
                            row.symbol.clone(),
                        ),
                    );
                }
            }
            "hold" => {
                observed_hold_count += 1;
                if !approx_eq(row.weight_delta_pct, 0.0) {
                    return Err(
                        SecurityPortfolioExecutionRequestPackageError::PreviewActionWeightDrift(
                            row.symbol.clone(),
                        ),
                    );
                }
            }
            other => {
                return Err(
                    SecurityPortfolioExecutionRequestPackageError::UnsupportedPreviewAction(
                        row.symbol.clone(),
                        other.to_string(),
                    ),
                );
            }
        }

        let expected_gross_pct = round_pct(row.weight_delta_pct.abs());
        if !approx_eq(expected_gross_pct, row.preview_trade_gross_pct) {
            return Err(
                SecurityPortfolioExecutionRequestPackageError::PreviewGrossMismatch(
                    row.symbol.clone(),
                ),
            );
        }
    }

    if observed_buy_count != document.buy_count {
        return Err(
            SecurityPortfolioExecutionRequestPackageError::PreviewSummaryCountMismatch(
                "buy_count".to_string(),
                observed_buy_count,
                document.buy_count,
            ),
        );
    }
    if observed_sell_count != document.sell_count {
        return Err(
            SecurityPortfolioExecutionRequestPackageError::PreviewSummaryCountMismatch(
                "sell_count".to_string(),
                observed_sell_count,
                document.sell_count,
            ),
        );
    }
    if observed_hold_count != document.hold_count {
        return Err(
            SecurityPortfolioExecutionRequestPackageError::PreviewSummaryCountMismatch(
                "hold_count".to_string(),
                observed_hold_count,
                document.hold_count,
            ),
        );
    }

    Ok(())
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

fn round_pct(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn approx_eq(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-9
}
