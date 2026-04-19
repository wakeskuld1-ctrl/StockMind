use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_account_open_position_snapshot::{
    SecurityActivePositionBookDocument, SecurityActivePositionDocument,
};
use crate::ops::stock::security_per_position_evaluation::SecurityPerPositionEvaluation;
use crate::ops::stock::security_portfolio_position_plan::{
    SecurityAdjustmentSimulationData, SecurityMonitoringAccountAggregation,
    build_adjustment_simulation_data, build_monitoring_account_aggregation,
};
use crate::ops::stock::security_position_contract::SecurityPositionContract;

const SECURITY_MONITORING_EVIDENCE_PACKAGE_DOCUMENT_TYPE: &str =
    "security_monitoring_evidence_package";
const SECURITY_MONITORING_EVIDENCE_PACKAGE_VERSION: &str =
    "security_monitoring_evidence_package.v1";

// 2026-04-18 CST: Added because Task 5 needs one formal request shell for the
// account-level monitoring evidence package.
// Reason: downstream monitoring consumers should hand one governed bundle into
// the package builder instead of rebuilding account aggregation by themselves.
// Purpose: freeze the minimal public request contract for monitoring evidence packaging.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityMonitoringEvidencePackageRequest {
    pub active_position_book: SecurityActivePositionBookDocument,
    pub position_contracts: Vec<SecurityPositionContract>,
    pub per_position_evaluations: Vec<SecurityPerPositionEvaluation>,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-18 CST: Added because the monitoring package should keep one compact
// active-position summary section beside the deeper per-position evaluation documents.
// Reason: future governance consumers need a fast top-line live-state view before
// reading the full evaluation set.
// Purpose: define the compact active-position summary payload for the package.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityMonitoringActivePositionSummary {
    pub symbol: String,
    pub current_weight_pct: f64,
    #[serde(default)]
    pub current_price: Option<f64>,
    #[serde(default)]
    pub holding_total_return_pct: Option<f64>,
    pub recommended_action: String,
}

// 2026-04-18 CST: Added because Task 5 introduces the standardized monitoring
// evidence object consumed by future committee-facing workflows.
// Reason: the approved design requires a stable account-level evidence handoff
// before adjustment simulation or governance review begins.
// Purpose: define the first formal monitoring evidence package document.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityMonitoringEvidencePackage {
    pub monitoring_evidence_package_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub account_id: String,
    pub source_active_position_book_ref: String,
    pub source_evaluation_refs: Vec<String>,
    pub account_aggregation: SecurityMonitoringAccountAggregation,
    pub active_positions_summary: Vec<SecurityMonitoringActivePositionSummary>,
    pub per_position_evaluations: Vec<SecurityPerPositionEvaluation>,
    pub action_candidates: SecurityAdjustmentSimulationData,
    pub warnings: Vec<String>,
    pub package_status: String,
    pub monitoring_summary: String,
}

// 2026-04-18 CST: Added because the public tool route should return one named
// result wrapper instead of a bare evidence package.
// Reason: this keeps the package tool response extensible for later governance adapters.
// Purpose: wrap the monitoring evidence package in a stable tool result shell.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityMonitoringEvidencePackageResult {
    pub monitoring_evidence_package: SecurityMonitoringEvidencePackage,
}

// 2026-04-18 CST: Added because Task 5 needs one explicit error boundary for
// account-level monitoring package formation.
// Reason: account identity mismatches must fail before future governance sees a mixed package.
// Purpose: keep package-building failures explicit and auditable.
#[derive(Debug, Error)]
pub enum SecurityMonitoringEvidencePackageError {
    #[error(
        "security monitoring evidence package build failed: per-position evaluation `{0}` does not belong to account `{1}`"
    )]
    EvaluationAccountMismatch(String, String),
    #[error(
        "security monitoring evidence package build failed: position contract `{0}` does not belong to account `{1}`"
    )]
    PositionContractAccountMismatch(String, String),
}

// 2026-04-18 CST: Added because Task 5 needs the first governed builder for
// one account-level monitoring evidence package.
// Reason: this package becomes the standardized evidence handoff before later
// adjustment simulation and governance review layers.
// Purpose: combine active positions, evaluations, and account aggregation into one document.
pub fn build_security_monitoring_evidence_package(
    request: &SecurityMonitoringEvidencePackageRequest,
) -> Result<SecurityMonitoringEvidencePackageResult, SecurityMonitoringEvidencePackageError> {
    for evaluation in &request.per_position_evaluations {
        if evaluation.account_id != request.active_position_book.account_id {
            return Err(
                SecurityMonitoringEvidencePackageError::EvaluationAccountMismatch(
                    evaluation.per_position_evaluation_id.clone(),
                    request.active_position_book.account_id.clone(),
                ),
            );
        }
    }

    for position_contract in &request.position_contracts {
        if position_contract.account_id != request.active_position_book.account_id {
            return Err(
                SecurityMonitoringEvidencePackageError::PositionContractAccountMismatch(
                    position_contract.position_contract_id.clone(),
                    request.active_position_book.account_id.clone(),
                ),
            );
        }
    }

    let account_aggregation = build_monitoring_account_aggregation(
        &request.per_position_evaluations,
        &request.position_contracts,
    );
    let action_candidates = build_adjustment_simulation_data(&request.per_position_evaluations);
    let mut source_evaluation_refs = request
        .per_position_evaluations
        .iter()
        .map(|evaluation| evaluation.per_position_evaluation_id.clone())
        .collect::<Vec<_>>();
    source_evaluation_refs.sort();

    let mut warnings = account_aggregation.concentration_warnings.clone();
    warnings.extend(account_aggregation.correlation_warnings.clone());
    warnings.extend(account_aggregation.risk_budget_warnings.clone());
    warnings.sort();
    warnings.dedup();

    let active_positions_summary = build_active_positions_summary(
        &request.active_position_book.active_positions,
        &request.per_position_evaluations,
    );

    Ok(SecurityMonitoringEvidencePackageResult {
        monitoring_evidence_package: SecurityMonitoringEvidencePackage {
            monitoring_evidence_package_id: format!(
                "monitoring-evidence-package:{}:{}",
                request.active_position_book.account_id,
                normalize_created_at(&request.created_at)
            ),
            contract_version: SECURITY_MONITORING_EVIDENCE_PACKAGE_VERSION.to_string(),
            document_type: SECURITY_MONITORING_EVIDENCE_PACKAGE_DOCUMENT_TYPE.to_string(),
            generated_at: normalize_created_at(&request.created_at),
            account_id: request.active_position_book.account_id.clone(),
            source_active_position_book_ref: request
                .active_position_book
                .active_position_book_id
                .clone(),
            source_evaluation_refs,
            account_aggregation,
            active_positions_summary,
            per_position_evaluations: request.per_position_evaluations.clone(),
            action_candidates,
            warnings,
            package_status: "ready_for_committee_review".to_string(),
            monitoring_summary: format!(
                "account {} monitoring package prepared with {} live evaluations",
                request.active_position_book.account_id,
                request.per_position_evaluations.len()
            ),
        },
    })
}

// 2026-04-18 CST: Added because the evidence package should keep one compact
// top-line active holding summary for fast review.
// Reason: future consumers should not always need to parse the full evaluation document set.
// Purpose: build the package-level active-position summary section.
fn build_active_positions_summary(
    active_positions: &[SecurityActivePositionDocument],
    evaluations: &[SecurityPerPositionEvaluation],
) -> Vec<SecurityMonitoringActivePositionSummary> {
    let mut summaries = active_positions
        .iter()
        .map(|position| {
            let recommended_action = evaluations
                .iter()
                .find(|evaluation| evaluation.symbol == position.symbol)
                .map(|evaluation| evaluation.recommended_action.clone())
                .unwrap_or_else(|| "hold".to_string());

            SecurityMonitoringActivePositionSummary {
                symbol: position.symbol.clone(),
                current_weight_pct: position.current_weight_pct,
                current_price: position.current_price,
                holding_total_return_pct: position.holding_total_return_pct,
                recommended_action,
            }
        })
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    summaries
}

fn normalize_created_at(value: &str) -> String {
    if value.trim().is_empty() {
        Utc::now().to_rfc3339()
    } else {
        value.trim().to_string()
    }
}

fn default_created_at() -> String {
    Utc::now().to_rfc3339()
}
