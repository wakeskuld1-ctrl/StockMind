use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_portfolio_allocation_decision::SecurityPortfolioAllocationDecisionDocument;

const SECURITY_PORTFOLIO_EXECUTION_PREVIEW_DOCUMENT_TYPE: &str =
    "security_portfolio_execution_preview";
const SECURITY_PORTFOLIO_EXECUTION_PREVIEW_VERSION: &str =
    "security_portfolio_execution_preview.v1";

// 2026-04-20 CST: Added because the first post-P12 downstream bridge needs one
// explicit formal request shell above the existing allocation decision contract.
// Reason: the approved route allows execution preparation only through a preview
// boundary, not by calling runtime execution directly.
// Purpose: define the public request contract for the execution preview bridge.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityPortfolioExecutionPreviewRequest {
    pub portfolio_allocation_decision: SecurityPortfolioAllocationDecisionDocument,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-20 CST: Added because preview consumers need one explicit per-symbol
// row instead of reverse-engineering buy/sell intent from free-form text.
// Reason: the bridge should preserve the governed allocation deltas while keeping
// the result clearly separate from real execution facts.
// 2026-04-20 CST: Extended because the approved enhancement now needs one
// nested request-aligned preview subset without removing the existing readable row fields.
// Purpose: define the reusable execution preview row emitted by the new document.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionPreviewRow {
    pub symbol: String,
    pub current_weight_pct: f64,
    pub target_weight_pct: f64,
    pub weight_delta_pct: f64,
    pub preview_action: String,
    pub preview_trade_gross_pct: f64,
    pub execution_request_preview_summary: String,
    pub execution_record_request_preview: SecurityExecutionRecordRequestPreview,
}

// 2026-04-20 CST: Added because the standardized preview enhancement needs one
// explicit nested object that aligns to the safe subset of SecurityExecutionRecordRequest.
// Reason: later execution work should reuse a stable preview contract instead of parsing prose.
// Purpose: freeze the request-aligned preview boundary while keeping this bridge preview-only.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityExecutionRecordRequestPreview {
    pub symbol: String,
    pub account_id: String,
    pub decision_ref: String,
    pub execution_action: String,
    pub execution_status: String,
    pub executed_gross_pct: f64,
    pub execution_summary: String,
}

// 2026-04-20 CST: Added because the preview bridge needs one named formal
// downstream document after the portfolio-core chain completes at P12.
// Reason: later sessions should consume one explicit preview object instead of
// treating the bridge as ad hoc derived UI logic.
// Purpose: define the first post-P12 execution preview document.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionPreviewDocument {
    pub portfolio_execution_preview_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub account_id: String,
    pub portfolio_allocation_decision_ref: String,
    pub preview_rows: Vec<SecurityPortfolioExecutionPreviewRow>,
    pub buy_count: usize,
    pub sell_count: usize,
    pub hold_count: usize,
    pub readiness_status: String,
    pub blockers: Vec<String>,
    pub preview_rationale: Vec<String>,
    pub preview_summary: String,
}

// 2026-04-20 CST: Added because the public stock tool route should return one
// named result wrapper instead of a bare preview document.
// Reason: keeping a stable top-level shell matches the existing stock tool style.
// Purpose: wrap the execution preview document in a stable result shape.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioExecutionPreviewResult {
    pub portfolio_execution_preview: SecurityPortfolioExecutionPreviewDocument,
}

// 2026-04-20 CST: Added because the new bridge must reject malformed P12 input
// instead of silently masking allocation drift under downstream summaries.
// Reason: this stage is a preview consumer, not a repair layer.
// Purpose: keep preview failures explicit and auditable.
#[derive(Debug, Error)]
pub enum SecurityPortfolioExecutionPreviewError {
    #[error(
        "security portfolio execution preview build failed: account id is missing from the governed allocation decision"
    )]
    MissingAccountId,
    #[error(
        "security portfolio execution preview build failed: allocation closure mismatch detected `{0}` vs `{1}`"
    )]
    AllocationClosureMismatch(f64, f64),
    #[error(
        "security portfolio execution preview build failed: weight delta mismatch detected on `{0}`"
    )]
    WeightDeltaMismatch(String),
}

// 2026-04-20 CST: Added because the new preview bridge needs one public stock
// entry point that remains side-effect free.
// Reason: callers should not rebuild execution-preview logic outside the formal bus.
// Purpose: expose the execution preview builder on the official stock surface.
pub fn security_portfolio_execution_preview(
    request: &SecurityPortfolioExecutionPreviewRequest,
) -> Result<SecurityPortfolioExecutionPreviewResult, SecurityPortfolioExecutionPreviewError> {
    build_security_portfolio_execution_preview(request)
}

pub fn build_security_portfolio_execution_preview(
    request: &SecurityPortfolioExecutionPreviewRequest,
) -> Result<SecurityPortfolioExecutionPreviewResult, SecurityPortfolioExecutionPreviewError> {
    validate_portfolio_allocation_decision(&request.portfolio_allocation_decision)?;

    let generated_at = normalize_created_at(&request.created_at);
    let account_id = request
        .portfolio_allocation_decision
        .account_id
        .trim()
        .to_string();
    let portfolio_allocation_decision_ref = request
        .portfolio_allocation_decision
        .portfolio_allocation_decision_id
        .clone();

    let mut buy_count = 0usize;
    let mut sell_count = 0usize;
    let mut hold_count = 0usize;
    let preview_rows = request
        .portfolio_allocation_decision
        .final_target_allocations
        .iter()
        .map(|row| {
            let preview_action = if row.weight_delta_pct > 1e-9 {
                buy_count += 1;
                "buy"
            } else if row.weight_delta_pct < -1e-9 {
                sell_count += 1;
                "sell"
            } else {
                hold_count += 1;
                "hold"
            };

            SecurityPortfolioExecutionPreviewRow {
                symbol: row.symbol.clone(),
                current_weight_pct: row.current_weight_pct,
                target_weight_pct: row.target_weight_pct,
                weight_delta_pct: row.weight_delta_pct,
                preview_action: preview_action.to_string(),
                preview_trade_gross_pct: round_pct(row.weight_delta_pct.abs()),
                execution_request_preview_summary: format!(
                    "preview {} {:.4} gross weight on {} from {}",
                    preview_action,
                    row.weight_delta_pct.abs(),
                    row.symbol,
                    portfolio_allocation_decision_ref
                ),
                execution_record_request_preview: SecurityExecutionRecordRequestPreview {
                    symbol: row.symbol.clone(),
                    account_id: account_id.clone(),
                    decision_ref: portfolio_allocation_decision_ref.clone(),
                    execution_action: preview_action.to_string(),
                    execution_status: "preview_only".to_string(),
                    executed_gross_pct: round_pct(row.weight_delta_pct.abs()),
                    execution_summary: format!(
                        "preview {} {:.4} gross weight on {} from governed allocation decision {}",
                        preview_action,
                        row.weight_delta_pct.abs(),
                        row.symbol,
                        portfolio_allocation_decision_ref
                    ),
                },
            }
        })
        .collect::<Vec<_>>();

    let preview_rationale = vec![
        format!(
            "preview derived from governed allocation decision {}",
            portfolio_allocation_decision_ref
        ),
        "preview rows remain side-effect free and do not write execution facts".to_string(),
    ];

    Ok(SecurityPortfolioExecutionPreviewResult {
        portfolio_execution_preview: SecurityPortfolioExecutionPreviewDocument {
            portfolio_execution_preview_id: format!(
                "portfolio-execution-preview:{}:{}",
                account_id, generated_at
            ),
            contract_version: SECURITY_PORTFOLIO_EXECUTION_PREVIEW_VERSION.to_string(),
            document_type: SECURITY_PORTFOLIO_EXECUTION_PREVIEW_DOCUMENT_TYPE.to_string(),
            generated_at,
            account_id: account_id.clone(),
            portfolio_allocation_decision_ref,
            preview_rows,
            buy_count,
            sell_count,
            hold_count,
            readiness_status: "ready".to_string(),
            blockers: Vec::new(),
            preview_rationale,
            preview_summary: format!(
                "account {} previewed {} buys, {} sells, {} holds from governed P12 output",
                account_id, buy_count, sell_count, hold_count
            ),
        },
    })
}

fn validate_portfolio_allocation_decision(
    document: &SecurityPortfolioAllocationDecisionDocument,
) -> Result<(), SecurityPortfolioExecutionPreviewError> {
    // 2026-04-20 CST: Added because the preview bridge should fail fast when
    // account identity or allocation closure drift is already present upstream.
    // Reason: preview is a downstream consumer of P12, not the place to repair it.
    // Purpose: keep the first post-P12 bridge bounded to validation plus projection.
    if document.account_id.trim().is_empty() {
        return Err(SecurityPortfolioExecutionPreviewError::MissingAccountId);
    }

    let total_target_weight_pct = round_pct(
        document
            .final_target_allocations
            .iter()
            .map(|row| row.target_weight_pct)
            .sum(),
    );
    let observed_total = round_pct(total_target_weight_pct + document.residual_cash_weight_pct);
    if !approx_eq(observed_total, 1.0) {
        return Err(SecurityPortfolioExecutionPreviewError::AllocationClosureMismatch(
            observed_total,
            1.0,
        ));
    }

    for row in &document.final_target_allocations {
        let expected_delta = round_pct(row.target_weight_pct - row.current_weight_pct);
        if !approx_eq(expected_delta, row.weight_delta_pct) {
            return Err(SecurityPortfolioExecutionPreviewError::WeightDeltaMismatch(
                row.symbol.clone(),
            ));
        }
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
