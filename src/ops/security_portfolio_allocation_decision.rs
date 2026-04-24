use std::collections::{BTreeMap, BTreeSet};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_account_objective_contract::{
    SecurityAccountObjectiveContractDocument, SecurityPortfolioCandidateSet,
};
use crate::ops::stock::security_portfolio_replacement_plan::{
    SecurityPortfolioReplacementPlanDocument, SecurityPortfolioWeightSnapshot,
};

const SECURITY_PORTFOLIO_ALLOCATION_DECISION_DOCUMENT_TYPE: &str =
    "security_portfolio_allocation_decision";
const SECURITY_PORTFOLIO_ALLOCATION_DECISION_VERSION: &str =
    "security_portfolio_allocation_decision.v1";

// 2026-04-20 CST: Added because P12 now needs one explicit public request shell
// above the implemented P10 and P11 contracts.
// Reason: the approved route requires the final decision layer to consume only
// governed upstream documents instead of raw account fragments.
// Purpose: define the formal P12 request boundary for final allocation freeze.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityPortfolioAllocationDecisionRequest {
    pub account_objective_contract: SecurityAccountObjectiveContractDocument,
    pub portfolio_candidate_set: SecurityPortfolioCandidateSet,
    pub portfolio_replacement_plan: SecurityPortfolioReplacementPlanDocument,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-20 CST: Added because P12 must freeze one explicit per-symbol final
// allocation row rather than leaving downstream consumers to infer deltas.
// Reason: the decision layer should restate current, target, and actionability
// in one deterministic row shape.
// Purpose: define the reusable final allocation row emitted by the P12 document.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioAllocationDecisionRow {
    pub symbol: String,
    pub current_weight_pct: f64,
    pub target_weight_pct: f64,
    pub weight_delta_pct: f64,
    pub decision_action: String,
    pub allocation_source: String,
}

// 2026-04-20 CST: Added because the final decision layer must expose which
// hard governance checks were passed before readiness can be declared.
// Reason: later approval or execution-bridge consumers should not reverse
// engineer constraint status from summary prose.
// Purpose: define the structured constraint-check rows for the P12 contract.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioAllocationConstraintCheck {
    pub check_name: String,
    pub status: String,
    pub observed_value: f64,
    pub limit_value: f64,
    pub detail: String,
}

// 2026-04-20 CST: Added because P12 needs one governed document that freezes
// final allocation, residual cash, readiness, and rationale after P11.
// Reason: the approved stage split separates decision freeze from both solving
// and later execution-bridge work.
// Purpose: define the first formal P12 portfolio allocation decision document.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioAllocationDecisionDocument {
    pub portfolio_allocation_decision_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub account_id: String,
    pub account_objective_contract_ref: String,
    pub portfolio_candidate_set_ref: String,
    pub portfolio_replacement_plan_ref: String,
    pub baseline_target_allocations: Vec<SecurityPortfolioAllocationDecisionRow>,
    pub final_target_allocations: Vec<SecurityPortfolioAllocationDecisionRow>,
    pub baseline_residual_cash_weight_pct: f64,
    pub residual_cash_weight_pct: f64,
    pub refinement_applied: bool,
    pub turnover_slack_weight_pct_before_refinement: f64,
    pub turnover_slack_weight_pct_after_refinement: f64,
    pub capital_base_amount_before: f64,
    pub capital_base_amount_after: f64,
    pub rebase_context_applied: bool,
    pub constraint_checks: Vec<SecurityPortfolioAllocationConstraintCheck>,
    pub readiness_status: String,
    pub blockers: Vec<String>,
    pub decision_rationale: Vec<String>,
    pub allocation_refinement_summary: Vec<String>,
    pub conflict_resolution_summary: Vec<String>,
    pub decision_summary: String,
}

// 2026-04-20 CST: Added because the public stock tool route should return one
// named result wrapper instead of a bare document.
// Reason: future P12 hardening can extend metadata without changing the public
// top-level response shape.
// Purpose: wrap the portfolio allocation decision in a stable result shell.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioAllocationDecisionResult {
    pub portfolio_allocation_decision: SecurityPortfolioAllocationDecisionDocument,
}

// 2026-04-20 CST: Added because P12 must hard-fail on upstream drift, malformed
// replacement-plan closure, and objective-limit mismatch.
// Reason: the final decision layer is a governed freeze boundary, not a repair
// layer that tolerates or silently fixes broken inputs.
// Purpose: keep P12 failures explicit and auditable.
#[derive(Debug, Error)]
pub enum SecurityPortfolioAllocationDecisionError {
    #[error(
        "security portfolio allocation decision build failed: candidate set account `{0}` does not match request account `{1}`"
    )]
    CandidateSetAccountMismatch(String, String),
    #[error(
        "security portfolio allocation decision build failed: replacement plan account `{0}` does not match request account `{1}`"
    )]
    ReplacementPlanAccountMismatch(String, String),
    #[error(
        "security portfolio allocation decision build failed: target weight symbol `{0}` is not present in the governed candidate set"
    )]
    TargetWeightSymbolNotInCandidateSet(String),
    #[error(
        "security portfolio allocation decision build failed: weight non-conservation detected on `{0}`"
    )]
    WeightNonConservation(String),
    #[error(
        "security portfolio allocation decision build failed: residual cash mismatch detected `{0}` vs `{1}`"
    )]
    ResidualCashMismatch(f64, f64),
    #[error("security portfolio allocation decision build failed: objective limit mismatch `{0}`")]
    ObjectiveLimitMismatch(String),
}

// 2026-04-20 CST: Added because P12 now needs one public orchestration entry
// on the formal stock tool surface.
// Reason: callers should not rebuild final validation and decision freeze logic
// outside the governed portfolio-core boundary.
// Purpose: expose the P12 allocation decision builder on the stock bus.
pub fn security_portfolio_allocation_decision(
    request: &SecurityPortfolioAllocationDecisionRequest,
) -> Result<SecurityPortfolioAllocationDecisionResult, SecurityPortfolioAllocationDecisionError> {
    build_security_portfolio_allocation_decision(request)
}

pub fn build_security_portfolio_allocation_decision(
    request: &SecurityPortfolioAllocationDecisionRequest,
) -> Result<SecurityPortfolioAllocationDecisionResult, SecurityPortfolioAllocationDecisionError> {
    validate_request_accounts(request)?;

    let generated_at = normalize_created_at(&request.created_at);
    let account_id = request
        .account_objective_contract
        .account_id
        .trim()
        .to_string();
    let current_weight_map = build_weight_map(&request.portfolio_replacement_plan.current_weights);
    let baseline_target_weight_map =
        build_weight_map(&request.portfolio_replacement_plan.target_weights);
    let governed_candidate_symbols =
        build_governed_candidate_symbols(&request.portfolio_candidate_set);

    validate_weight_symbols(&baseline_target_weight_map, &governed_candidate_symbols)?;

    let total_current_weight_pct = round_pct(current_weight_map.values().sum());
    let baseline_total_target_weight_pct = round_pct(baseline_target_weight_map.values().sum());
    validate_weight_conservation("current_weights", total_current_weight_pct)?;
    validate_weight_conservation("target_weights", baseline_total_target_weight_pct)?;
    validate_weight_conservation_against_plan(
        "current_weights",
        total_current_weight_pct,
        request
            .portfolio_replacement_plan
            .capital_migration_plan
            .total_current_weight_pct,
    )?;
    validate_weight_conservation_against_plan(
        "target_weights",
        baseline_total_target_weight_pct,
        request
            .portfolio_replacement_plan
            .capital_migration_plan
            .total_target_weight_pct,
    )?;

    let baseline_residual_cash_weight_pct = round_pct(1.0 - baseline_total_target_weight_pct);
    validate_residual_cash(
        baseline_residual_cash_weight_pct,
        request
            .portfolio_replacement_plan
            .capital_migration_plan
            .residual_cash_weight_pct,
    )?;

    let baseline_gross_turnover_weight_pct = round_pct(compute_gross_turnover_weight_pct(
        &current_weight_map,
        &baseline_target_weight_map,
    ));
    validate_turnover_against_plan(
        baseline_gross_turnover_weight_pct,
        request
            .portfolio_replacement_plan
            .capital_migration_plan
            .gross_turnover_weight_pct,
    )?;

    let turnover_slack_weight_pct_before_refinement = round_pct(
        (request.account_objective_contract.turnover_limit - baseline_gross_turnover_weight_pct)
            .max(0.0),
    );
    let (
        refined_target_weight_map,
        refinement_applied,
        allocation_refinement_summary,
        turnover_slack_weight_pct_after_refinement,
    ) = apply_residual_cash_priority_fill(
        &baseline_target_weight_map,
        &request.portfolio_candidate_set,
        turnover_slack_weight_pct_before_refinement,
        baseline_residual_cash_weight_pct,
    );

    let total_target_weight_pct = round_pct(refined_target_weight_map.values().sum());
    validate_weight_conservation("refined_target_weights", total_target_weight_pct)?;
    let residual_cash_weight_pct = round_pct(1.0 - total_target_weight_pct);
    validate_residual_cash_floor(residual_cash_weight_pct)?;
    let gross_turnover_weight_pct = round_pct(compute_gross_turnover_weight_pct(
        &current_weight_map,
        &refined_target_weight_map,
    ));

    let target_position_count = refined_target_weight_map
        .values()
        .filter(|weight_pct| **weight_pct > 0.0)
        .count();
    if target_position_count > request.account_objective_contract.position_count_limit {
        return Err(
            SecurityPortfolioAllocationDecisionError::ObjectiveLimitMismatch(
                "position count limit exceeded".to_string(),
            ),
        );
    }
    if gross_turnover_weight_pct > request.account_objective_contract.turnover_limit + 1e-9 {
        return Err(
            SecurityPortfolioAllocationDecisionError::ObjectiveLimitMismatch(
                "turnover limit exceeded".to_string(),
            ),
        );
    }

    let total_target_risk_budget_pct = round_pct(compute_total_target_risk_budget_pct(
        &request.portfolio_candidate_set,
        &refined_target_weight_map,
    ));
    if total_target_risk_budget_pct > request.account_objective_contract.risk_budget_limit + 1e-9 {
        return Err(
            SecurityPortfolioAllocationDecisionError::ObjectiveLimitMismatch(
                "risk budget limit exceeded".to_string(),
            ),
        );
    }

    validate_max_weight_caps(&request.portfolio_candidate_set, &refined_target_weight_map)?;

    let baseline_target_allocations = build_final_target_allocations(
        &request.portfolio_replacement_plan.target_weights,
        &current_weight_map,
        &request.portfolio_candidate_set,
    )?;
    let refined_target_weights = build_weight_snapshots(&refined_target_weight_map);
    let final_target_allocations = build_final_target_allocations(
        &refined_target_weights,
        &current_weight_map,
        &request.portfolio_candidate_set,
    )?;
    let constraint_checks = build_constraint_checks(
        total_target_risk_budget_pct,
        request.account_objective_contract.risk_budget_limit,
        gross_turnover_weight_pct,
        request.account_objective_contract.turnover_limit,
        target_position_count as f64,
        request.account_objective_contract.position_count_limit as f64,
        residual_cash_weight_pct,
    );
    let blockers = Vec::new();
    let readiness_status = if blockers.is_empty() {
        "ready".to_string()
    } else {
        "blocked".to_string()
    };
    let mut decision_rationale = vec![
        "final allocation frozen from governed P10 objective and P11 replacement outputs"
            .to_string(),
        format!(
            "baseline target weight {:.4} left {:.4} residual cash before refinement",
            baseline_total_target_weight_pct, baseline_residual_cash_weight_pct
        ),
        format!(
            "validated {:.4} target risk budget and {:.4} turnover against the formal objective shell",
            total_target_risk_budget_pct, gross_turnover_weight_pct
        ),
    ];
    if refinement_applied {
        decision_rationale.push(format!(
            "residual cash refinement consumed turnover slack from {:.4} to {:.4}",
            turnover_slack_weight_pct_before_refinement, turnover_slack_weight_pct_after_refinement
        ));
    } else {
        decision_rationale.push(
            "no refinement applied because turnover slack or symbol spare capacity was exhausted"
                .to_string(),
        );
    }

    Ok(SecurityPortfolioAllocationDecisionResult {
        portfolio_allocation_decision: SecurityPortfolioAllocationDecisionDocument {
            portfolio_allocation_decision_id: format!(
                "portfolio-allocation-decision:{}:{}",
                account_id, generated_at
            ),
            contract_version: SECURITY_PORTFOLIO_ALLOCATION_DECISION_VERSION.to_string(),
            document_type: SECURITY_PORTFOLIO_ALLOCATION_DECISION_DOCUMENT_TYPE.to_string(),
            generated_at,
            account_id: account_id.clone(),
            account_objective_contract_ref: request
                .account_objective_contract
                .account_objective_contract_id
                .clone(),
            portfolio_candidate_set_ref: request
                .portfolio_candidate_set
                .portfolio_candidate_set_id
                .clone(),
            portfolio_replacement_plan_ref: request
                .portfolio_replacement_plan
                .portfolio_replacement_plan_id
                .clone(),
            baseline_target_allocations,
            final_target_allocations,
            baseline_residual_cash_weight_pct,
            residual_cash_weight_pct,
            refinement_applied,
            turnover_slack_weight_pct_before_refinement,
            turnover_slack_weight_pct_after_refinement,
            capital_base_amount_before: request
                .portfolio_replacement_plan
                .capital_migration_plan
                .capital_base_amount_before,
            capital_base_amount_after: request
                .portfolio_replacement_plan
                .capital_migration_plan
                .capital_base_amount_after,
            rebase_context_applied: request
                .portfolio_replacement_plan
                .capital_migration_plan
                .rebase_context_applied,
            constraint_checks,
            readiness_status,
            blockers,
            decision_rationale,
            allocation_refinement_summary,
            conflict_resolution_summary: request
                .portfolio_replacement_plan
                .conflict_resolution_summary
                .clone(),
            decision_summary: format!(
                "account {} governed allocation decision frozen with {:.4} target weight, {:.4} residual cash, refinement_applied={}",
                account_id, total_target_weight_pct, residual_cash_weight_pct, refinement_applied
            ),
        },
    })
}

fn validate_request_accounts(
    request: &SecurityPortfolioAllocationDecisionRequest,
) -> Result<(), SecurityPortfolioAllocationDecisionError> {
    // 2026-04-20 CST: Added because P12 must hard-stop on account drift before
    // any final allocation decision is frozen.
    // Reason: downstream approval and execution bridges assume one governed account scope.
    // Purpose: keep account identity validation explicit at the P12 boundary.
    let request_account = request
        .account_objective_contract
        .account_id
        .trim()
        .to_string();

    if request.portfolio_candidate_set.account_id.trim() != request_account {
        return Err(
            SecurityPortfolioAllocationDecisionError::CandidateSetAccountMismatch(
                request.portfolio_candidate_set.account_id.clone(),
                request_account,
            ),
        );
    }

    if request.portfolio_replacement_plan.account_id.trim() != request_account {
        return Err(
            SecurityPortfolioAllocationDecisionError::ReplacementPlanAccountMismatch(
                request.portfolio_replacement_plan.account_id.clone(),
                request_account,
            ),
        );
    }

    Ok(())
}

fn build_governed_candidate_symbols(
    portfolio_candidate_set: &SecurityPortfolioCandidateSet,
) -> BTreeSet<String> {
    // 2026-04-20 CST: Added because P12 must verify that final target rows still
    // belong to the governed P10 candidate universe.
    // Reason: the approved route forbids raw symbol bypass at the decision-freeze layer.
    // Purpose: collect the legal symbol set for final allocation validation.
    portfolio_candidate_set
        .live_positions
        .iter()
        .map(|live_position| live_position.symbol.clone())
        .chain(
            portfolio_candidate_set
                .approved_candidate_entries
                .iter()
                .map(|candidate| candidate.symbol.clone()),
        )
        .collect()
}

fn validate_weight_symbols(
    target_weight_map: &BTreeMap<String, f64>,
    governed_candidate_symbols: &BTreeSet<String>,
) -> Result<(), SecurityPortfolioAllocationDecisionError> {
    // 2026-04-20 CST: Added because malformed target rows must fail instead of
    // being silently accepted at the final decision layer.
    // Reason: P12 is a governed freeze boundary, not a repair stage.
    // Purpose: reject target weights that drift outside the governed candidate set.
    for symbol in target_weight_map.keys() {
        if !governed_candidate_symbols.contains(symbol) {
            return Err(
                SecurityPortfolioAllocationDecisionError::TargetWeightSymbolNotInCandidateSet(
                    symbol.clone(),
                ),
            );
        }
    }

    Ok(())
}

fn build_weight_map(weights: &[SecurityPortfolioWeightSnapshot]) -> BTreeMap<String, f64> {
    // 2026-04-20 CST: Added because P12 must recompute closure and turnover from
    // stable keyed weights rather than positional arrays.
    // Reason: deterministic validation is easier to audit on symbol-keyed maps.
    // Purpose: normalize weight snapshots into one reusable lookup map.
    weights
        .iter()
        .map(|weight| (weight.symbol.clone(), round_pct(weight.weight_pct)))
        .collect()
}

fn validate_weight_conservation(
    section_name: &str,
    total_weight_pct: f64,
) -> Result<(), SecurityPortfolioAllocationDecisionError> {
    // 2026-04-20 CST: Added because P12 must keep the same hard-fail closure
    // discipline that already exists in the upstream portfolio-core stages.
    // Reason: non-conserving weights cannot be frozen into a governed decision.
    // Purpose: reject invalid current or target weight totals.
    if total_weight_pct < -1e-9 || total_weight_pct > 1.0 + 1e-9 {
        return Err(
            SecurityPortfolioAllocationDecisionError::WeightNonConservation(
                section_name.to_string(),
            ),
        );
    }

    Ok(())
}

fn validate_weight_conservation_against_plan(
    section_name: &str,
    observed_weight_pct: f64,
    planned_weight_pct: f64,
) -> Result<(), SecurityPortfolioAllocationDecisionError> {
    // 2026-04-20 CST: Added because P12 should re-check that weight totals still
    // match the P11 capital migration summary before freezing the final decision.
    // Reason: plan/document drift must fail explicitly at the final governance boundary.
    // Purpose: keep target and current closure aligned with the upstream plan metadata.
    if (observed_weight_pct - round_pct(planned_weight_pct)).abs() > 1e-9 {
        return Err(
            SecurityPortfolioAllocationDecisionError::WeightNonConservation(
                section_name.to_string(),
            ),
        );
    }

    Ok(())
}

fn validate_residual_cash(
    observed_residual_cash_weight_pct: f64,
    planned_residual_cash_weight_pct: f64,
) -> Result<(), SecurityPortfolioAllocationDecisionError> {
    // 2026-04-20 CST: Added because residual cash is one of the core P12 frozen
    // outputs and must stay consistent with both weight closure and P11 metadata.
    // Reason: negative or drifted residual cash would break later execution interpretation.
    // Purpose: reject residual-cash contradictions before readiness can be declared.
    if observed_residual_cash_weight_pct < -1e-9 {
        return Err(
            SecurityPortfolioAllocationDecisionError::ResidualCashMismatch(
                observed_residual_cash_weight_pct,
                planned_residual_cash_weight_pct,
            ),
        );
    }

    if (observed_residual_cash_weight_pct - round_pct(planned_residual_cash_weight_pct)).abs()
        > 1e-9
    {
        return Err(
            SecurityPortfolioAllocationDecisionError::ResidualCashMismatch(
                observed_residual_cash_weight_pct,
                planned_residual_cash_weight_pct,
            ),
        );
    }

    Ok(())
}

fn validate_residual_cash_floor(
    observed_residual_cash_weight_pct: f64,
) -> Result<(), SecurityPortfolioAllocationDecisionError> {
    // 2026-04-20 CST: Added because enhanced P12 intentionally allows refined
    // residual cash to differ from the baseline P11 plan.
    // Reason: only the non-negative floor stays invariant after bounded refinement.
    // Purpose: reject over-allocation while allowing legal residual-cash deployment.
    if observed_residual_cash_weight_pct < -1e-9 {
        return Err(
            SecurityPortfolioAllocationDecisionError::ResidualCashMismatch(
                observed_residual_cash_weight_pct,
                0.0,
            ),
        );
    }

    Ok(())
}

fn validate_turnover_against_plan(
    observed_turnover_weight_pct: f64,
    planned_turnover_weight_pct: f64,
) -> Result<(), SecurityPortfolioAllocationDecisionError> {
    // 2026-04-20 CST: Added because P12 must verify that recomputed turnover
    // still matches the governed P11 plan before freezing the allocation decision.
    // Reason: a drifted turnover figure would weaken the explicit route separation.
    // Purpose: keep final decision validation aligned with upstream plan closure.
    if (observed_turnover_weight_pct - round_pct(planned_turnover_weight_pct)).abs() > 1e-9 {
        return Err(
            SecurityPortfolioAllocationDecisionError::WeightNonConservation(
                "gross_turnover_weight_pct".to_string(),
            ),
        );
    }

    Ok(())
}

fn compute_gross_turnover_weight_pct(
    current_weight_map: &BTreeMap<String, f64>,
    target_weight_map: &BTreeMap<String, f64>,
) -> f64 {
    // 2026-04-20 CST: Added because P12 must independently recompute turnover
    // from current and target allocations instead of trusting prose summaries.
    // Reason: the final decision layer is responsible for explicit objective conformance.
    // Purpose: derive one deterministic turnover metric for validation and reporting.
    let mut union_symbols = current_weight_map.keys().cloned().collect::<Vec<_>>();
    union_symbols.extend(
        target_weight_map
            .keys()
            .filter(|symbol| !current_weight_map.contains_key(*symbol))
            .cloned(),
    );

    union_symbols
        .iter()
        .map(|symbol| {
            let current_weight_pct = current_weight_map.get(symbol).copied().unwrap_or(0.0);
            let target_weight_pct = target_weight_map.get(symbol).copied().unwrap_or(0.0);
            (target_weight_pct - current_weight_pct).abs()
        })
        .sum()
}

fn compute_total_target_risk_budget_pct(
    portfolio_candidate_set: &SecurityPortfolioCandidateSet,
    target_weight_map: &BTreeMap<String, f64>,
) -> f64 {
    // 2026-04-20 CST: Added because P12 must re-check target risk budget against
    // the account objective before readiness can be marked ready.
    // Reason: the approved route requires final validation, not blind trust of upstream artifacts.
    // Purpose: sum the governed risk-budget rows for symbols that remain in the target allocation.
    portfolio_candidate_set
        .live_positions
        .iter()
        .filter(|live_position| {
            target_weight_map
                .get(&live_position.symbol)
                .copied()
                .unwrap_or(0.0)
                > 0.0
        })
        .map(|live_position| live_position.risk_budget_pct.unwrap_or(0.0))
        .sum::<f64>()
        + portfolio_candidate_set
            .approved_candidate_entries
            .iter()
            .filter(|candidate| {
                target_weight_map
                    .get(&candidate.symbol)
                    .copied()
                    .unwrap_or(0.0)
                    > 0.0
            })
            .map(|candidate| candidate.risk_budget_pct)
            .sum::<f64>()
}

// 2026-04-20 CST: Added because enhanced P12 needs one compact internal row
// for ranking governed symbols during residual-cash refinement.
// Reason: keeping score and max-weight metadata together reduces drift across the refinement pass.
// Purpose: carry the minimum per-symbol refinement metadata inside the P12 module only.
#[derive(Debug, Clone)]
struct SecurityAllocationRefinementCandidate {
    symbol: String,
    priority_score: f64,
    max_weight_pct: f64,
}

fn apply_residual_cash_priority_fill(
    baseline_target_weight_map: &BTreeMap<String, f64>,
    portfolio_candidate_set: &SecurityPortfolioCandidateSet,
    turnover_slack_weight_pct_before_refinement: f64,
    baseline_residual_cash_weight_pct: f64,
) -> (BTreeMap<String, f64>, bool, Vec<String>, f64) {
    // 2026-04-20 CST: Added because enhanced P12 now needs one bounded second
    // pass that can improve the baseline plan without replacing P11.
    // Reason: the approved route allows only residual-cash deployment inside remaining turnover slack.
    // Purpose: allocate residual cash toward higher-priority governed symbols up to their max weights.
    let mut refined_target_weight_map = baseline_target_weight_map.clone();
    let mut remaining_turnover_slack = turnover_slack_weight_pct_before_refinement;
    let mut remaining_residual_cash = baseline_residual_cash_weight_pct;
    let mut allocation_refinement_summary = Vec::new();

    for candidate in
        build_refinement_candidates(portfolio_candidate_set, baseline_target_weight_map)
    {
        if remaining_turnover_slack <= 1e-9 || remaining_residual_cash <= 1e-9 {
            break;
        }

        let current_target_weight_pct = refined_target_weight_map
            .get(&candidate.symbol)
            .copied()
            .unwrap_or(0.0);
        let spare_capacity =
            round_pct((candidate.max_weight_pct - current_target_weight_pct).max(0.0));
        if spare_capacity <= 1e-9 {
            continue;
        }

        let added_weight_pct = round_pct(
            spare_capacity
                .min(remaining_turnover_slack)
                .min(remaining_residual_cash),
        );
        if added_weight_pct <= 1e-9 {
            continue;
        }

        let refined_target_weight_pct = round_pct(current_target_weight_pct + added_weight_pct);
        refined_target_weight_map.insert(candidate.symbol.clone(), refined_target_weight_pct);
        remaining_turnover_slack =
            round_pct((remaining_turnover_slack - added_weight_pct).max(0.0));
        remaining_residual_cash = round_pct((remaining_residual_cash - added_weight_pct).max(0.0));
        allocation_refinement_summary.push(format!(
            "{} priority_fill +{:.6} -> {:.6} (score={:.6})",
            candidate.symbol, added_weight_pct, refined_target_weight_pct, candidate.priority_score
        ));
    }

    let refinement_applied = !allocation_refinement_summary.is_empty();
    if !refinement_applied {
        allocation_refinement_summary
            .push("no_refinement:turnover_slack_exhausted_or_no_symbol_capacity".to_string());
    }

    (
        refined_target_weight_map,
        refinement_applied,
        allocation_refinement_summary,
        remaining_turnover_slack,
    )
}

fn build_refinement_candidates(
    portfolio_candidate_set: &SecurityPortfolioCandidateSet,
    baseline_target_weight_map: &BTreeMap<String, f64>,
) -> Vec<SecurityAllocationRefinementCandidate> {
    // 2026-04-20 CST: Added because the bounded refinement pass needs one
    // deterministic priority ordering over governed symbols.
    // Reason: stable score ordering keeps the enhancement auditable and repeatable.
    // Purpose: collect symbol score and max-weight capacity from the governed candidate set.
    let mut candidates = portfolio_candidate_set
        .live_positions
        .iter()
        .map(|live_position| SecurityAllocationRefinementCandidate {
            symbol: live_position.symbol.clone(),
            priority_score: compute_priority_score(
                live_position.expected_annual_return_pct.unwrap_or(0.0),
                live_position.expected_drawdown_pct.unwrap_or(0.01),
            ),
            max_weight_pct: round_pct(live_position.max_weight_pct.unwrap_or_else(|| {
                baseline_target_weight_map
                    .get(&live_position.symbol)
                    .copied()
                    .unwrap_or(0.0)
            })),
        })
        .chain(
            portfolio_candidate_set
                .approved_candidate_entries
                .iter()
                .map(|candidate| SecurityAllocationRefinementCandidate {
                    symbol: candidate.symbol.clone(),
                    priority_score: compute_priority_score(
                        candidate.expected_annual_return_pct,
                        candidate.expected_drawdown_pct,
                    ),
                    max_weight_pct: round_pct(candidate.max_weight_pct),
                }),
        )
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        right
            .priority_score
            .partial_cmp(&left.priority_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(left.symbol.cmp(&right.symbol))
    });
    candidates
}

fn compute_priority_score(expected_annual_return_pct: f64, expected_drawdown_pct: f64) -> f64 {
    // 2026-04-20 CST: Added because enhanced P12 needs one simple governed
    // ranking signal before it can deploy spare residual cash.
    // Reason: the approved route calls for bounded refinement, not an opaque heuristic mix.
    // Purpose: derive one deterministic score from expected return and expected drawdown.
    let normalized_drawdown_pct = expected_drawdown_pct.abs().max(0.01);
    round_pct(expected_annual_return_pct / normalized_drawdown_pct)
}

fn validate_max_weight_caps(
    portfolio_candidate_set: &SecurityPortfolioCandidateSet,
    refined_target_weight_map: &BTreeMap<String, f64>,
) -> Result<(), SecurityPortfolioAllocationDecisionError> {
    // 2026-04-20 CST: Added because enhanced P12 must stop at symbol max weight
    // even when residual cash and turnover slack still remain.
    // Reason: the approved route allows more allocation only inside governed symbol caps.
    // Purpose: reject refined targets that exceed their explicit max-weight limit.
    let max_weight_map = portfolio_candidate_set
        .live_positions
        .iter()
        .filter_map(|live_position| {
            live_position
                .max_weight_pct
                .map(|max_weight_pct| (live_position.symbol.clone(), max_weight_pct))
        })
        .chain(
            portfolio_candidate_set
                .approved_candidate_entries
                .iter()
                .map(|candidate| (candidate.symbol.clone(), candidate.max_weight_pct)),
        )
        .collect::<BTreeMap<_, _>>();

    for (symbol, target_weight_pct) in refined_target_weight_map {
        if let Some(max_weight_pct) = max_weight_map.get(symbol) {
            if *target_weight_pct > round_pct(*max_weight_pct) + 1e-9 {
                return Err(
                    SecurityPortfolioAllocationDecisionError::ObjectiveLimitMismatch(format!(
                        "max weight exceeded on {symbol}"
                    )),
                );
            }
        }
    }

    Ok(())
}

fn build_weight_snapshots(
    target_weight_map: &BTreeMap<String, f64>,
) -> Vec<SecurityPortfolioWeightSnapshot> {
    // 2026-04-20 CST: Added because enhanced P12 needs to convert its refined
    // map form back into the shared weight-snapshot shape for final row rendering.
    // Reason: reusing the same row builder keeps baseline and refined rendering aligned.
    // Purpose: normalize refined target maps into sorted snapshot rows.
    target_weight_map
        .iter()
        .map(|(symbol, weight_pct)| SecurityPortfolioWeightSnapshot {
            symbol: symbol.clone(),
            weight_pct: round_pct(*weight_pct),
        })
        .collect()
}

fn build_final_target_allocations(
    target_weights: &[SecurityPortfolioWeightSnapshot],
    current_weight_map: &BTreeMap<String, f64>,
    portfolio_candidate_set: &SecurityPortfolioCandidateSet,
) -> Result<Vec<SecurityPortfolioAllocationDecisionRow>, SecurityPortfolioAllocationDecisionError> {
    // 2026-04-20 CST: Added because P12 must emit one explicit final allocation
    // table instead of leaving downstream consumers to infer per-symbol actions.
    // Reason: final decision freeze should be auditable at the row level.
    // Purpose: build the canonical current/target/action rows for the P12 document.
    let approved_candidate_symbols = portfolio_candidate_set
        .approved_candidate_entries
        .iter()
        .map(|candidate| candidate.symbol.clone())
        .collect::<BTreeSet<_>>();

    let mut final_target_allocations = target_weights
        .iter()
        .map(|target_weight| {
            let current_weight_pct = current_weight_map
                .get(&target_weight.symbol)
                .copied()
                .unwrap_or(0.0);
            let target_weight_pct = round_pct(target_weight.weight_pct);
            let weight_delta_pct = round_pct(target_weight_pct - current_weight_pct);

            let decision_action = if current_weight_pct <= 0.0 && target_weight_pct > 0.0 {
                "entry".to_string()
            } else if target_weight_pct <= 0.0 {
                "exit".to_string()
            } else if weight_delta_pct > 0.0 {
                "add".to_string()
            } else if weight_delta_pct < 0.0 {
                "trim".to_string()
            } else {
                "hold".to_string()
            };

            let allocation_source = if approved_candidate_symbols.contains(&target_weight.symbol) {
                "approved_candidate_entry".to_string()
            } else {
                "live_position_target".to_string()
            };

            Ok(SecurityPortfolioAllocationDecisionRow {
                symbol: target_weight.symbol.clone(),
                current_weight_pct,
                target_weight_pct,
                weight_delta_pct,
                decision_action,
                allocation_source,
            })
        })
        .collect::<Result<Vec<_>, SecurityPortfolioAllocationDecisionError>>()?;
    final_target_allocations.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    Ok(final_target_allocations)
}

fn build_constraint_checks(
    total_target_risk_budget_pct: f64,
    risk_budget_limit: f64,
    gross_turnover_weight_pct: f64,
    turnover_limit: f64,
    target_position_count: f64,
    position_count_limit: f64,
    residual_cash_weight_pct: f64,
) -> Vec<SecurityPortfolioAllocationConstraintCheck> {
    // 2026-04-20 CST: Added because P12 must expose structured constraint
    // evidence instead of only returning one free-form readiness string.
    // Reason: later approval consumers should read explicit pass/fail rows from the decision document.
    // Purpose: freeze the first deterministic constraint-check section for P12.
    vec![
        SecurityPortfolioAllocationConstraintCheck {
            check_name: "risk_budget_limit".to_string(),
            status: "passed".to_string(),
            observed_value: total_target_risk_budget_pct,
            limit_value: risk_budget_limit,
            detail: "target risk budget stayed inside the account objective".to_string(),
        },
        SecurityPortfolioAllocationConstraintCheck {
            check_name: "turnover_limit".to_string(),
            status: "passed".to_string(),
            observed_value: gross_turnover_weight_pct,
            limit_value: turnover_limit,
            detail: "gross turnover stayed inside the account objective".to_string(),
        },
        SecurityPortfolioAllocationConstraintCheck {
            check_name: "position_count_limit".to_string(),
            status: "passed".to_string(),
            observed_value: target_position_count,
            limit_value: position_count_limit,
            detail: "target position count stayed inside the account objective".to_string(),
        },
        SecurityPortfolioAllocationConstraintCheck {
            check_name: "residual_cash_weight_pct".to_string(),
            status: "passed".to_string(),
            observed_value: residual_cash_weight_pct,
            limit_value: 1.0,
            detail: "residual cash remained consistent with target-weight closure".to_string(),
        },
    ]
}

fn normalize_created_at(created_at: &str) -> String {
    // 2026-04-20 CST: Added because P12 document IDs and timestamps must stay
    // deterministic even when callers omit `created_at`.
    // Reason: all portfolio-core stages already normalize timestamps the same way.
    // Purpose: keep generated-at behavior stable across the new decision layer.
    let trimmed = created_at.trim();
    if trimmed.is_empty() {
        default_created_at()
    } else {
        trimmed.to_string()
    }
}

fn default_created_at() -> String {
    // 2026-04-20 CST: Added because the public request contract needs one stable
    // default timestamp source for omitted `created_at`.
    // Reason: the portfolio-core document family uses generated timestamps in IDs and lineage.
    // Purpose: provide the default wall-clock timestamp for P12 request normalization.
    Utc::now().to_rfc3339()
}

fn round_pct(value: f64) -> f64 {
    // 2026-04-20 CST: Added because P12 must compare weight and limit values with
    // the same fixed precision as the upstream portfolio-core stages.
    // Reason: deterministic rounding avoids spurious drift at the governance boundary.
    // Purpose: normalize floating-point values before validation and document emission.
    (value * 1_000_000.0).round() / 1_000_000.0
}
