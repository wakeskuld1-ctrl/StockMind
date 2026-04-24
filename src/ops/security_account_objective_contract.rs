use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_account_open_position_snapshot::{
    SecurityActivePositionBookDocument, SecurityActivePositionDocument,
};
use crate::ops::stock::security_monitoring_evidence_package::{
    SecurityMonitoringActivePositionSummary, SecurityMonitoringEvidencePackage,
};
use crate::ops::stock::security_portfolio_position_plan::SecurityMonitoringActionCandidate;
use crate::ops::stock::security_position_contract::SecurityPositionContract;

const SECURITY_ACCOUNT_OBJECTIVE_CONTRACT_DOCUMENT_TYPE: &str =
    "security_account_objective_contract";
const SECURITY_ACCOUNT_OBJECTIVE_CONTRACT_VERSION: &str = "security_account_objective_contract.v1";
const SECURITY_PORTFOLIO_CANDIDATE_SET_DOCUMENT_TYPE: &str = "security_portfolio_candidate_set";
const SECURITY_PORTFOLIO_CANDIDATE_SET_VERSION: &str = "security_portfolio_candidate_set.v1";
const APPROVED_CANDIDATE_ONLY_BOUNDARY: &str = "approved-candidate-only";

// 2026-04-19 CST: Added because Task 1 needs one explicit upstream-approved
// candidate input shell before the account-level portfolio core is allowed to
// compete live holdings against new entrants.
// Reason: P10 should only accept governed entrants and must not inherit raw
// research payloads directly.
// Purpose: define the minimal approved-candidate DTO consumed by the account objective builder.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityApprovedPortfolioCandidateInput {
    pub candidate_id: String,
    pub account_id: String,
    pub symbol: String,
    #[serde(default)]
    pub security_name: Option<String>,
    pub approval_status: String,
    pub position_management_ready: bool,
    pub approved_open_position_packet_ref: String,
    pub expected_annual_return_pct: f64,
    pub expected_drawdown_pct: f64,
    pub target_weight_pct: f64,
    pub max_weight_pct: f64,
    pub risk_budget_pct: f64,
    #[serde(default)]
    pub sector_tag: Option<String>,
}

// 2026-04-19 CST: Added because P10 should freeze one formal account-level
// optimization request before any replacement solver is introduced.
// Reason: the approved design starts from one governed account problem, not
// from ad hoc helper composition at each caller.
// Purpose: define the public request shell for account objective normalization.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityAccountObjectiveContractRequest {
    pub active_position_book: SecurityActivePositionBookDocument,
    pub position_contracts: Vec<SecurityPositionContract>,
    pub monitoring_evidence_package: SecurityMonitoringEvidencePackage,
    pub approved_candidates: Vec<SecurityApprovedPortfolioCandidateInput>,
    pub target_return_objective: f64,
    pub max_drawdown_limit: f64,
    pub risk_budget_limit: f64,
    pub turnover_limit: f64,
    pub position_count_limit: usize,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-19 CST: Added because Task 1 needs one governed account-level
// objective document with explicit capital and constraint boundaries.
// Reason: later portfolio replacement and allocation stages should consume one
// stable objective object instead of re-deriving account constraints.
// Purpose: define the first formal P10 account objective document.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityAccountObjectiveContractDocument {
    pub account_objective_contract_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub account_id: String,
    pub capital_base_amount: f64,
    pub target_return_objective: f64,
    pub max_drawdown_limit: f64,
    pub risk_budget_limit: f64,
    pub turnover_limit: f64,
    pub position_count_limit: usize,
    pub constraint_version: String,
    pub objective_summary: String,
}

// 2026-04-19 CST: Added because P10 candidate normalization should preserve one
// compact live-position row shape for the portfolio competition set.
// Reason: later solver stages need the current live state plus governed target
// hints without reparsing multiple upstream documents.
// Purpose: define the normalized live-position entry emitted by the candidate set.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioLivePositionEntry {
    pub account_id: String,
    pub symbol: String,
    pub candidate_status: String,
    pub capital_base_amount: f64,
    pub selection_boundary_ref: String,
    pub position_state: String,
    pub current_weight_pct: f64,
    #[serde(default)]
    pub current_price: Option<f64>,
    #[serde(default)]
    pub holding_total_return_pct: Option<f64>,
    #[serde(default)]
    pub recommended_action: Option<String>,
    #[serde(default)]
    pub target_weight_pct: Option<f64>,
    #[serde(default)]
    pub max_weight_pct: Option<f64>,
    #[serde(default)]
    pub expected_annual_return_pct: Option<f64>,
    #[serde(default)]
    pub expected_drawdown_pct: Option<f64>,
    #[serde(default)]
    pub risk_budget_pct: Option<f64>,
    #[serde(default)]
    pub sector_tag: Option<String>,
    #[serde(default)]
    pub position_contract_ref: Option<String>,
}

// 2026-04-19 CST: Added because the candidate set also needs one normalized
// entrant row shape for approved new positions.
// Reason: P10 should keep upstream-governed candidate facts explicit before
// P11 later decides whether to admit, replace, or reject them.
// Purpose: define the approved entrant entry emitted by the candidate set.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioApprovedCandidateEntry {
    pub candidate_id: String,
    pub account_id: String,
    pub symbol: String,
    pub candidate_status: String,
    pub capital_base_amount: f64,
    pub selection_boundary_ref: String,
    #[serde(default)]
    pub security_name: Option<String>,
    pub approved_open_position_packet_ref: String,
    pub expected_annual_return_pct: f64,
    pub expected_drawdown_pct: f64,
    pub target_weight_pct: f64,
    pub max_weight_pct: f64,
    pub risk_budget_pct: f64,
    #[serde(default)]
    pub sector_tag: Option<String>,
}

// 2026-04-19 CST: Added because Task 1 should emit one formal capital
// competition set beside the account objective contract.
// Reason: later unified replacement logic must start from one normalized set of
// live holdings and approved entrants.
// Purpose: define the P10 portfolio candidate set document.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioCandidateSet {
    pub portfolio_candidate_set_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub account_id: String,
    pub capital_base_amount: f64,
    pub live_positions: Vec<SecurityPortfolioLivePositionEntry>,
    pub approved_candidate_entries: Vec<SecurityPortfolioApprovedCandidateEntry>,
    pub candidate_exits: Vec<SecurityMonitoringActionCandidate>,
    pub selection_boundary_ref: String,
}

// 2026-04-19 CST: Added because the public stock tool route should return one
// named result wrapper instead of anonymous objective and candidate payloads.
// Reason: later P11/P12 expansion can extend the response without changing the
// top-level tool contract.
// Purpose: wrap the account objective and candidate set in one stable result shell.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityAccountObjectiveContractResult {
    pub account_objective_contract: SecurityAccountObjectiveContractDocument,
    pub portfolio_candidate_set: SecurityPortfolioCandidateSet,
}

// 2026-04-19 CST: Added because P10 needs one explicit hard-fail boundary for
// account identity drift, invalid constraints, and unapproved entrants.
// Reason: the approved stage design requires deterministic rejection before any
// portfolio optimization work starts.
// Purpose: keep Task 1 failures explicit and auditable.
#[derive(Debug, Error)]
pub enum SecurityAccountObjectiveContractError {
    #[error(
        "security account objective contract build failed: monitoring package account `{0}` does not match request account `{1}`"
    )]
    MonitoringAccountMismatch(String, String),
    #[error(
        "security account objective contract build failed: position contract `{0}` account `{1}` does not match request account `{2}`"
    )]
    PositionContractAccountMismatch(String, String, String),
    #[error(
        "security account objective contract build failed: approved candidate `{0}` account `{1}` does not match request account `{2}`"
    )]
    ApprovedCandidateAccountMismatch(String, String, String),
    #[error(
        "security account objective contract build failed: capital base must be present and positive across governed position contracts"
    )]
    MissingCapitalBase,
    #[error(
        "security account objective contract build failed: objective constraint `{0}` is invalid"
    )]
    InvalidObjectiveConstraint(String),
    #[error(
        "security account objective contract build failed: approved candidate `{0}` must have approval_status `approved`"
    )]
    CandidateNotApproved(String),
    #[error(
        "security account objective contract build failed: approved candidate `{0}` must be position-management ready"
    )]
    CandidateNotPositionManagementReady(String),
    #[error(
        "security account objective contract build failed: duplicate symbol `{0}` is not allowed across the portfolio candidate set"
    )]
    DuplicateSymbol(String),
}

// 2026-04-19 CST: Added because Task 1 needs one formal public builder that
// converts governed account inputs into the first P10 output pair.
// Reason: callers should not reconstruct account objectives and candidate sets
// outside the stock contract boundary.
// Purpose: expose the deterministic account objective normalization entry.
pub fn build_security_account_objective_contract(
    request: &SecurityAccountObjectiveContractRequest,
) -> Result<SecurityAccountObjectiveContractResult, SecurityAccountObjectiveContractError> {
    validate_request_accounts(request)?;
    validate_objective_constraints(request)?;

    let capital_base_amount = resolve_capital_base_amount(&request.position_contracts)?;
    validate_duplicate_symbols(
        &request.active_position_book.active_positions,
        &request.approved_candidates,
    )?;
    let generated_at = normalize_created_at(&request.created_at);
    let account_id = normalize_text(&request.active_position_book.account_id);

    let account_objective_contract = SecurityAccountObjectiveContractDocument {
        account_objective_contract_id: format!(
            "account-objective-contract:{}:{}",
            account_id, generated_at
        ),
        contract_version: SECURITY_ACCOUNT_OBJECTIVE_CONTRACT_VERSION.to_string(),
        document_type: SECURITY_ACCOUNT_OBJECTIVE_CONTRACT_DOCUMENT_TYPE.to_string(),
        generated_at: generated_at.clone(),
        account_id: account_id.clone(),
        capital_base_amount,
        target_return_objective: request.target_return_objective,
        max_drawdown_limit: request.max_drawdown_limit,
        risk_budget_limit: request.risk_budget_limit,
        turnover_limit: request.turnover_limit,
        position_count_limit: request.position_count_limit,
        constraint_version: SECURITY_ACCOUNT_OBJECTIVE_CONTRACT_VERSION.to_string(),
        objective_summary: format!(
            "account {} objective targets {:.4} return with {:.4} max drawdown, {:.4} risk budget, {:.4} turnover, and {} positions",
            account_id,
            request.target_return_objective,
            request.max_drawdown_limit,
            request.risk_budget_limit,
            request.turnover_limit,
            request.position_count_limit
        ),
    };

    let portfolio_candidate_set = SecurityPortfolioCandidateSet {
        portfolio_candidate_set_id: format!(
            "portfolio-candidate-set:{}:{}",
            account_id, generated_at
        ),
        contract_version: SECURITY_PORTFOLIO_CANDIDATE_SET_VERSION.to_string(),
        document_type: SECURITY_PORTFOLIO_CANDIDATE_SET_DOCUMENT_TYPE.to_string(),
        generated_at,
        account_id: account_id.clone(),
        capital_base_amount,
        live_positions: build_live_positions(
            &request.active_position_book.active_positions,
            &request.position_contracts,
            &request.monitoring_evidence_package.active_positions_summary,
            &account_id,
            capital_base_amount,
        ),
        approved_candidate_entries: build_approved_candidate_entries(
            &request.approved_candidates,
            &account_id,
            capital_base_amount,
        )?,
        candidate_exits: request
            .monitoring_evidence_package
            .action_candidates
            .top_exit_candidates
            .clone(),
        selection_boundary_ref: APPROVED_CANDIDATE_ONLY_BOUNDARY.to_string(),
    };

    Ok(SecurityAccountObjectiveContractResult {
        account_objective_contract,
        portfolio_candidate_set,
    })
}

// 2026-04-19 CST: Added because the public stock tool name should map to one
// canonical orchestration function rather than exposing the internal builder
// choice in every caller.
// Reason: later P10 extensions may add non-builder preprocessing without changing dispatch code.
// Purpose: keep one stable tool-facing entry for account objective normalization.
pub fn security_account_objective_contract(
    request: &SecurityAccountObjectiveContractRequest,
) -> Result<SecurityAccountObjectiveContractResult, SecurityAccountObjectiveContractError> {
    build_security_account_objective_contract(request)
}

fn validate_request_accounts(
    request: &SecurityAccountObjectiveContractRequest,
) -> Result<(), SecurityAccountObjectiveContractError> {
    let request_account = normalize_text(&request.active_position_book.account_id);

    if normalize_text(&request.monitoring_evidence_package.account_id) != request_account {
        return Err(
            SecurityAccountObjectiveContractError::MonitoringAccountMismatch(
                request.monitoring_evidence_package.account_id.clone(),
                request_account,
            ),
        );
    }

    for position_contract in &request.position_contracts {
        let position_contract_account = normalize_text(&position_contract.account_id);
        if position_contract_account != request_account {
            return Err(
                SecurityAccountObjectiveContractError::PositionContractAccountMismatch(
                    position_contract.position_contract_id.clone(),
                    position_contract.account_id.clone(),
                    request_account,
                ),
            );
        }
    }

    for approved_candidate in &request.approved_candidates {
        let approved_candidate_account = normalize_text(&approved_candidate.account_id);
        if approved_candidate_account != request_account {
            return Err(
                SecurityAccountObjectiveContractError::ApprovedCandidateAccountMismatch(
                    approved_candidate.candidate_id.clone(),
                    approved_candidate.account_id.clone(),
                    request_account,
                ),
            );
        }
    }

    Ok(())
}

fn validate_objective_constraints(
    request: &SecurityAccountObjectiveContractRequest,
) -> Result<(), SecurityAccountObjectiveContractError> {
    if request.target_return_objective <= 0.0 {
        return Err(
            SecurityAccountObjectiveContractError::InvalidObjectiveConstraint(
                "target_return_objective".to_string(),
            ),
        );
    }
    if request.max_drawdown_limit <= 0.0 {
        return Err(
            SecurityAccountObjectiveContractError::InvalidObjectiveConstraint(
                "max_drawdown_limit".to_string(),
            ),
        );
    }
    if request.risk_budget_limit <= 0.0 {
        return Err(
            SecurityAccountObjectiveContractError::InvalidObjectiveConstraint(
                "risk_budget_limit".to_string(),
            ),
        );
    }
    if request.turnover_limit < 0.0 {
        return Err(
            SecurityAccountObjectiveContractError::InvalidObjectiveConstraint(
                "turnover_limit".to_string(),
            ),
        );
    }
    if request.position_count_limit == 0 {
        return Err(
            SecurityAccountObjectiveContractError::InvalidObjectiveConstraint(
                "position_count_limit".to_string(),
            ),
        );
    }

    Ok(())
}

fn resolve_capital_base_amount(
    position_contracts: &[SecurityPositionContract],
) -> Result<f64, SecurityAccountObjectiveContractError> {
    position_contracts
        .iter()
        .find_map(|position_contract| {
            (position_contract.capital_base_amount > 0.0)
                .then_some(position_contract.capital_base_amount)
        })
        .ok_or(SecurityAccountObjectiveContractError::MissingCapitalBase)
}

fn build_live_positions(
    active_positions: &[SecurityActivePositionDocument],
    position_contracts: &[SecurityPositionContract],
    active_position_summaries: &[SecurityMonitoringActivePositionSummary],
    account_id: &str,
    capital_base_amount: f64,
) -> Vec<SecurityPortfolioLivePositionEntry> {
    let mut live_positions = active_positions
        .iter()
        .map(|active_position| {
            let matching_contract = position_contracts.iter().find(|position_contract| {
                normalize_symbol(&position_contract.symbol)
                    == normalize_symbol(&active_position.symbol)
            });
            let matching_summary = active_position_summaries.iter().find(|summary| {
                normalize_symbol(&summary.symbol) == normalize_symbol(&active_position.symbol)
            });

            SecurityPortfolioLivePositionEntry {
                account_id: account_id.to_string(),
                symbol: normalize_symbol(&active_position.symbol),
                candidate_status: "live_position".to_string(),
                capital_base_amount,
                selection_boundary_ref: APPROVED_CANDIDATE_ONLY_BOUNDARY.to_string(),
                position_state: normalize_text(&active_position.position_state),
                current_weight_pct: active_position.current_weight_pct,
                current_price: active_position.current_price,
                holding_total_return_pct: active_position.holding_total_return_pct,
                recommended_action: matching_summary
                    .map(|summary| normalize_text(&summary.recommended_action)),
                target_weight_pct: matching_contract
                    .map(|position_contract| position_contract.target_weight_pct),
                max_weight_pct: matching_contract
                    .map(|position_contract| position_contract.max_weight_pct),
                expected_annual_return_pct: matching_contract
                    .map(|position_contract| position_contract.expected_annual_return_pct),
                expected_drawdown_pct: matching_contract
                    .map(|position_contract| position_contract.expected_drawdown_pct),
                risk_budget_pct: matching_contract
                    .map(|position_contract| position_contract.risk_budget_pct),
                sector_tag: normalize_optional_text(&active_position.sector_tag),
                position_contract_ref: matching_contract
                    .map(|position_contract| position_contract.position_contract_id.clone()),
            }
        })
        .collect::<Vec<_>>();

    live_positions.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    live_positions
}

fn build_approved_candidate_entries(
    approved_candidates: &[SecurityApprovedPortfolioCandidateInput],
    account_id: &str,
    capital_base_amount: f64,
) -> Result<Vec<SecurityPortfolioApprovedCandidateEntry>, SecurityAccountObjectiveContractError> {
    let mut approved_candidate_entries = Vec::with_capacity(approved_candidates.len());

    for approved_candidate in approved_candidates {
        let approval_status = normalize_lowercase(&approved_candidate.approval_status);
        if approval_status != "approved" {
            return Err(SecurityAccountObjectiveContractError::CandidateNotApproved(
                approved_candidate.candidate_id.clone(),
            ));
        }
        if !approved_candidate.position_management_ready {
            return Err(
                SecurityAccountObjectiveContractError::CandidateNotPositionManagementReady(
                    approved_candidate.candidate_id.clone(),
                ),
            );
        }

        approved_candidate_entries.push(SecurityPortfolioApprovedCandidateEntry {
            candidate_id: normalize_text(&approved_candidate.candidate_id),
            account_id: account_id.to_string(),
            symbol: normalize_symbol(&approved_candidate.symbol),
            candidate_status: "approved_new_candidate".to_string(),
            capital_base_amount,
            selection_boundary_ref: APPROVED_CANDIDATE_ONLY_BOUNDARY.to_string(),
            security_name: normalize_optional_text(&approved_candidate.security_name),
            approved_open_position_packet_ref: normalize_text(
                &approved_candidate.approved_open_position_packet_ref,
            ),
            expected_annual_return_pct: approved_candidate.expected_annual_return_pct,
            expected_drawdown_pct: approved_candidate.expected_drawdown_pct,
            target_weight_pct: approved_candidate.target_weight_pct,
            max_weight_pct: approved_candidate.max_weight_pct,
            risk_budget_pct: approved_candidate.risk_budget_pct,
            sector_tag: normalize_optional_text(&approved_candidate.sector_tag),
        });
    }

    approved_candidate_entries.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    Ok(approved_candidate_entries)
}

// 2026-04-19 CST: Added because Task 2 needs the candidate-set boundary to
// reject symbol identity drift before P11 starts solving capital competition.
// Reason: the same symbol cannot meaningfully appear twice across incumbent and
// entrant rows without corrupting replacement and migration math.
// Purpose: hard-fail duplicate symbol collisions inside the normalized candidate set.
fn validate_duplicate_symbols(
    active_positions: &[SecurityActivePositionDocument],
    approved_candidates: &[SecurityApprovedPortfolioCandidateInput],
) -> Result<(), SecurityAccountObjectiveContractError> {
    let mut seen_symbols = std::collections::BTreeSet::new();

    for active_position in active_positions {
        let symbol = normalize_symbol(&active_position.symbol);
        if !seen_symbols.insert(symbol.clone()) {
            return Err(SecurityAccountObjectiveContractError::DuplicateSymbol(
                symbol,
            ));
        }
    }

    for approved_candidate in approved_candidates {
        let symbol = normalize_symbol(&approved_candidate.symbol);
        if !seen_symbols.insert(symbol.clone()) {
            return Err(SecurityAccountObjectiveContractError::DuplicateSymbol(
                symbol,
            ));
        }
    }

    Ok(())
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

fn normalize_text(value: &str) -> String {
    value.trim().to_string()
}

fn normalize_lowercase(value: &str) -> String {
    normalize_text(value).to_ascii_lowercase()
}

fn normalize_symbol(value: &str) -> String {
    normalize_text(value).to_ascii_uppercase()
}

fn normalize_optional_text(value: &Option<String>) -> Option<String> {
    value.as_ref().and_then(|inner| {
        let normalized = normalize_text(inner);
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    })
}
