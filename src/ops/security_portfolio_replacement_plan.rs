use std::collections::BTreeMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_account_objective_contract::{
    SecurityAccountObjectiveContractDocument, SecurityPortfolioApprovedCandidateEntry,
    SecurityPortfolioCandidateSet, SecurityPortfolioLivePositionEntry,
};
use crate::ops::stock::security_capital_rebase::SecurityAccountRebaseSnapshot;

const SECURITY_PORTFOLIO_REPLACEMENT_PLAN_DOCUMENT_TYPE: &str =
    "security_portfolio_replacement_plan";
const SECURITY_PORTFOLIO_REPLACEMENT_PLAN_VERSION: &str = "security_portfolio_replacement_plan.v1";
const APPROVED_CANDIDATE_ONLY_BOUNDARY: &str = "approved-candidate-only";

// 2026-04-19 CST: Added because Task 3 needs one explicit P11 request shell
// that consumes only the formal P10 outputs.
// Reason: the unified replacement solver should start from governed account
// objective and candidate-set documents instead of raw upstream fragments.
// Purpose: define the public request contract for the first P11 solver pass.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioReplacementPlanRequest {
    pub account_objective_contract: SecurityAccountObjectiveContractDocument,
    pub portfolio_candidate_set: SecurityPortfolioCandidateSet,
    #[serde(default)]
    pub account_rebase_snapshot: Option<SecurityAccountRebaseSnapshot>,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-19 CST: Added because the P11 replacement plan should expose stable
// before/after portfolio weights that later allocation layers can reuse.
// Reason: downstream consumers must not infer weight closure from free-form text.
// Purpose: define the reusable weight row used by current and target sections.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioWeightSnapshot {
    pub symbol: String,
    pub weight_pct: f64,
}

// 2026-04-19 CST: Added because the first P11 contract needs compact action
// rows for entry/trim/exit summaries.
// Reason: later governance and execution consumers should read one normalized
// action shape instead of solver-specific internal deltas.
// Purpose: define the reusable action row for the replacement plan.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioReplacementAction {
    pub symbol: String,
    pub before_weight_pct: f64,
    pub after_weight_pct: f64,
    pub weight_delta_pct: f64,
    pub action_reason: String,
}

// 2026-04-19 CST: Added because Task 3 also needs one explicit replacement
// linkage between outgoing capital and incoming capital use.
// Reason: unified replacement should stay auditable even in the simple first pass.
// Purpose: define the first replacement-pair row for the P11 contract.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioReplacementPair {
    pub from_symbol: String,
    pub to_symbol: String,
    pub migrated_weight_pct: f64,
}

// 2026-04-19 CST: Added because the replacement plan must preserve capital
// closure metrics explicitly.
// Reason: later stages should reuse a governed migration summary rather than
// recomputing current/target totals and residual cash separately.
// Purpose: define the capital migration summary section for P11.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCapitalMigrationPlan {
    pub total_current_weight_pct: f64,
    pub total_target_weight_pct: f64,
    pub gross_turnover_weight_pct: f64,
    pub residual_cash_weight_pct: f64,
    pub capital_base_amount_before: f64,
    pub capital_base_amount_after: f64,
    #[serde(default)]
    pub rebase_policy: Option<String>,
    pub rebase_context_applied: bool,
}

// 2026-04-19 CST: Added because Task 4 needs one compact structured action
// count summary when the P11 solver emits mixed entry/trim/exit behavior.
// Reason: later audits should not have to recount array sections to understand
// whether the request produced a unified action mix.
// Purpose: define the first structured action summary for the replacement plan.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioReplacementActionSummary {
    pub entry_count: usize,
    pub trim_count: usize,
    pub exit_count: usize,
    pub replacement_pair_count: usize,
}

// 2026-04-19 CST: Added because Task 3 needs one formal P11 output document
// before P12 can freeze any governed allocation decision.
// Reason: the approved plan separates objective, replacement solve, and final
// allocation decision into explicit contracts.
// Purpose: define the first unified portfolio replacement plan document.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioReplacementPlanDocument {
    pub portfolio_replacement_plan_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub account_id: String,
    pub current_weights: Vec<SecurityPortfolioWeightSnapshot>,
    pub target_weights: Vec<SecurityPortfolioWeightSnapshot>,
    pub entry_actions: Vec<SecurityPortfolioReplacementAction>,
    pub trim_actions: Vec<SecurityPortfolioReplacementAction>,
    pub exit_actions: Vec<SecurityPortfolioReplacementAction>,
    pub replacement_pairs: Vec<SecurityPortfolioReplacementPair>,
    pub capital_migration_plan: SecurityCapitalMigrationPlan,
    pub solver_action_summary: SecurityPortfolioReplacementActionSummary,
    pub conflict_resolution_summary: Vec<String>,
    pub solver_summary: String,
}

// 2026-04-19 CST: Added because the public tool route should return one named
// wrapper instead of a bare replacement plan document.
// Reason: later P11 hardening can extend result metadata without changing the
// tool-level contract shape.
// Purpose: wrap the unified replacement plan in a stable result shell.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPortfolioReplacementPlanResult {
    pub portfolio_replacement_plan: SecurityPortfolioReplacementPlanDocument,
}

// 2026-04-19 CST: Added because Task 3 needs one explicit error boundary for
// P11 replacement solving.
// Reason: cross-account drift, infeasible allocation, and broken weight closure
// must fail before later governance layers see any plan output.
// Purpose: keep the first P11 solver deterministic and auditable.
#[derive(Debug, Error)]
pub enum SecurityPortfolioReplacementPlanError {
    #[error(
        "security portfolio replacement plan build failed: candidate set account `{0}` does not match request account `{1}`"
    )]
    CandidateSetAccountMismatch(String, String),
    #[error(
        "security portfolio replacement plan build failed: candidate set capital base `{0}` does not match request capital base `{1}`"
    )]
    CapitalBaseMismatch(f64, f64),
    #[error(
        "security portfolio replacement plan build failed: candidate set selection boundary `{0}` is invalid"
    )]
    InvalidSelectionBoundary(String),
    #[error(
        "security portfolio replacement plan build failed: approved candidate boundary drift detected on `{0}`"
    )]
    ApprovedCandidateBoundaryDrift(String),
    #[error(
        "security portfolio replacement plan build failed: live position boundary drift detected on `{0}`"
    )]
    LivePositionBoundaryDrift(String),
    #[error(
        "security portfolio replacement plan build failed: weight non-conservation detected on `{0}`"
    )]
    WeightNonConservation(String),
    #[error("security portfolio replacement plan build failed: infeasible allocation `{0}`")]
    InfeasibleAllocation(String),
    #[error(
        "security portfolio replacement plan build failed: account rebase snapshot account `{0}` does not match request account `{1}`"
    )]
    AccountRebaseSnapshotAccountMismatch(String, String),
}

// 2026-04-19 CST: Added because Task 3 needs one public orchestration entry
// for the first deterministic unified replacement pass.
// Reason: callers should not rebuild validation, target formation, and action
// summarization outside the stock contract boundary.
// Purpose: expose the first P11 solver on the formal stock tool surface.
pub fn security_portfolio_replacement_plan(
    request: &SecurityPortfolioReplacementPlanRequest,
) -> Result<SecurityPortfolioReplacementPlanResult, SecurityPortfolioReplacementPlanError> {
    build_security_portfolio_replacement_plan(request)
}

pub fn build_security_portfolio_replacement_plan(
    request: &SecurityPortfolioReplacementPlanRequest,
) -> Result<SecurityPortfolioReplacementPlanResult, SecurityPortfolioReplacementPlanError> {
    validate_request(request)?;

    let generated_at = normalize_created_at(&request.created_at);
    let account_id = request
        .account_objective_contract
        .account_id
        .trim()
        .to_string();

    let current_weights = build_current_weights(&request.portfolio_candidate_set.live_positions)?;
    let target_weights = build_target_weights(
        &request.portfolio_candidate_set.live_positions,
        &request.portfolio_candidate_set.approved_candidate_entries,
    )?;
    let current_weight_map = build_weight_map(&current_weights);
    let target_weight_map = build_weight_map(&target_weights);

    let total_current_weight_pct =
        round_pct(current_weights.iter().map(|row| row.weight_pct).sum());
    let total_target_weight_pct = round_pct(target_weights.iter().map(|row| row.weight_pct).sum());

    validate_weight_conservation("current_weights", total_current_weight_pct)?;
    validate_weight_conservation("target_weights", total_target_weight_pct)?;

    let gross_turnover_weight_pct = round_pct(compute_gross_turnover_weight_pct(
        &current_weight_map,
        &target_weight_map,
    ));
    let total_target_risk_budget_pct = round_pct(compute_total_target_risk_budget_pct(
        &request.portfolio_candidate_set.live_positions,
        &request.portfolio_candidate_set.approved_candidate_entries,
    ));
    let target_position_count = target_weights
        .iter()
        .filter(|row| row.weight_pct > 0.0)
        .count();

    if total_target_risk_budget_pct > request.account_objective_contract.risk_budget_limit + 1e-9 {
        return Err(SecurityPortfolioReplacementPlanError::InfeasibleAllocation(
            "risk budget limit exceeded".to_string(),
        ));
    }
    if target_position_count > request.account_objective_contract.position_count_limit {
        return Err(SecurityPortfolioReplacementPlanError::InfeasibleAllocation(
            "position count limit exceeded".to_string(),
        ));
    }
    if gross_turnover_weight_pct > request.account_objective_contract.turnover_limit + 1e-9 {
        return Err(SecurityPortfolioReplacementPlanError::InfeasibleAllocation(
            "turnover limit exceeded".to_string(),
        ));
    }

    let entry_actions =
        build_entry_actions(&request.portfolio_candidate_set.approved_candidate_entries);
    let trim_actions = build_trim_actions(&request.portfolio_candidate_set.live_positions);
    let exit_actions = build_exit_actions(&request.portfolio_candidate_set.live_positions);
    let replacement_pairs = build_replacement_pairs(&trim_actions, &entry_actions);
    let capital_base_amount_after = request
        .account_rebase_snapshot
        .as_ref()
        .and_then(resolve_rebased_capital_base_amount_after)
        .unwrap_or(request.account_objective_contract.capital_base_amount);
    let capital_migration_plan = SecurityCapitalMigrationPlan {
        total_current_weight_pct,
        total_target_weight_pct,
        gross_turnover_weight_pct,
        residual_cash_weight_pct: round_pct(1.0 - total_target_weight_pct),
        capital_base_amount_before: request.account_objective_contract.capital_base_amount,
        capital_base_amount_after,
        rebase_policy: request
            .account_rebase_snapshot
            .as_ref()
            .map(|snapshot| snapshot.rebase_policy.clone()),
        rebase_context_applied: request.account_rebase_snapshot.is_some(),
    };
    let solver_action_summary = SecurityPortfolioReplacementActionSummary {
        entry_count: entry_actions.len(),
        trim_count: trim_actions.len(),
        exit_count: exit_actions.len(),
        replacement_pair_count: replacement_pairs.len(),
    };
    let conflict_resolution_summary =
        build_conflict_resolution_summary(request.account_rebase_snapshot.is_some());

    Ok(SecurityPortfolioReplacementPlanResult {
        portfolio_replacement_plan: SecurityPortfolioReplacementPlanDocument {
            portfolio_replacement_plan_id: format!(
                "portfolio-replacement-plan:{}:{}",
                account_id, generated_at
            ),
            contract_version: SECURITY_PORTFOLIO_REPLACEMENT_PLAN_VERSION.to_string(),
            document_type: SECURITY_PORTFOLIO_REPLACEMENT_PLAN_DOCUMENT_TYPE.to_string(),
            generated_at,
            account_id: account_id.clone(),
            current_weights,
            target_weights,
            entry_actions,
            trim_actions,
            exit_actions,
            replacement_pairs,
            capital_migration_plan,
            solver_action_summary,
            conflict_resolution_summary,
            solver_summary: format!(
                "account {} unified replacement solved with {:.4} target weight, {:.4} risk budget, and {:.4} turnover",
                account_id,
                total_target_weight_pct,
                total_target_risk_budget_pct,
                gross_turnover_weight_pct
            ),
        },
    })
}

fn validate_request(
    request: &SecurityPortfolioReplacementPlanRequest,
) -> Result<(), SecurityPortfolioReplacementPlanError> {
    let request_account = request
        .account_objective_contract
        .account_id
        .trim()
        .to_string();
    let candidate_set_account = request
        .portfolio_candidate_set
        .account_id
        .trim()
        .to_string();
    if candidate_set_account != request_account {
        return Err(
            SecurityPortfolioReplacementPlanError::CandidateSetAccountMismatch(
                request.portfolio_candidate_set.account_id.clone(),
                request_account,
            ),
        );
    }

    if (request.portfolio_candidate_set.capital_base_amount
        - request.account_objective_contract.capital_base_amount)
        .abs()
        > 1e-9
    {
        return Err(SecurityPortfolioReplacementPlanError::CapitalBaseMismatch(
            request.portfolio_candidate_set.capital_base_amount,
            request.account_objective_contract.capital_base_amount,
        ));
    }

    if request.portfolio_candidate_set.selection_boundary_ref != APPROVED_CANDIDATE_ONLY_BOUNDARY {
        return Err(
            SecurityPortfolioReplacementPlanError::InvalidSelectionBoundary(
                request
                    .portfolio_candidate_set
                    .selection_boundary_ref
                    .clone(),
            ),
        );
    }

    for live_position in &request.portfolio_candidate_set.live_positions {
        if live_position.account_id.trim() != request_account
            || live_position.candidate_status != "live_position"
            || live_position.selection_boundary_ref != APPROVED_CANDIDATE_ONLY_BOUNDARY
        {
            return Err(
                SecurityPortfolioReplacementPlanError::LivePositionBoundaryDrift(
                    live_position.symbol.clone(),
                ),
            );
        }
    }

    for approved_candidate_entry in &request.portfolio_candidate_set.approved_candidate_entries {
        if approved_candidate_entry.account_id.trim() != request_account
            || approved_candidate_entry.candidate_status != "approved_new_candidate"
            || approved_candidate_entry.selection_boundary_ref != APPROVED_CANDIDATE_ONLY_BOUNDARY
        {
            return Err(
                SecurityPortfolioReplacementPlanError::ApprovedCandidateBoundaryDrift(
                    approved_candidate_entry.symbol.clone(),
                ),
            );
        }
    }

    if let Some(account_rebase_snapshot) = &request.account_rebase_snapshot {
        if account_rebase_snapshot.account_id.trim() != request_account {
            return Err(
                SecurityPortfolioReplacementPlanError::AccountRebaseSnapshotAccountMismatch(
                    account_rebase_snapshot.account_id.clone(),
                    request_account,
                ),
            );
        }
    }

    Ok(())
}

fn build_current_weights(
    live_positions: &[SecurityPortfolioLivePositionEntry],
) -> Result<Vec<SecurityPortfolioWeightSnapshot>, SecurityPortfolioReplacementPlanError> {
    let mut current_weights = live_positions
        .iter()
        .map(|live_position| SecurityPortfolioWeightSnapshot {
            symbol: live_position.symbol.clone(),
            weight_pct: round_pct(live_position.current_weight_pct),
        })
        .collect::<Vec<_>>();
    current_weights.sort_by(|left, right| left.symbol.cmp(&right.symbol));

    let total_current_weight_pct =
        round_pct(current_weights.iter().map(|row| row.weight_pct).sum());
    validate_weight_conservation("current_weights", total_current_weight_pct)?;
    Ok(current_weights)
}

fn build_target_weights(
    live_positions: &[SecurityPortfolioLivePositionEntry],
    approved_candidate_entries: &[SecurityPortfolioApprovedCandidateEntry],
) -> Result<Vec<SecurityPortfolioWeightSnapshot>, SecurityPortfolioReplacementPlanError> {
    let mut target_weights =
        live_positions
            .iter()
            .map(|live_position| SecurityPortfolioWeightSnapshot {
                symbol: live_position.symbol.clone(),
                weight_pct: round_pct(
                    live_position
                        .target_weight_pct
                        .unwrap_or(live_position.current_weight_pct),
                ),
            })
            .chain(approved_candidate_entries.iter().map(|candidate| {
                SecurityPortfolioWeightSnapshot {
                    symbol: candidate.symbol.clone(),
                    weight_pct: round_pct(candidate.target_weight_pct),
                }
            }))
            .collect::<Vec<_>>();
    target_weights.sort_by(|left, right| left.symbol.cmp(&right.symbol));

    let total_target_weight_pct = round_pct(target_weights.iter().map(|row| row.weight_pct).sum());
    validate_weight_conservation("target_weights", total_target_weight_pct)?;
    Ok(target_weights)
}

fn build_entry_actions(
    approved_candidate_entries: &[SecurityPortfolioApprovedCandidateEntry],
) -> Vec<SecurityPortfolioReplacementAction> {
    let mut entry_actions = approved_candidate_entries
        .iter()
        .filter(|candidate| candidate.target_weight_pct > 0.0)
        .map(|candidate| SecurityPortfolioReplacementAction {
            symbol: candidate.symbol.clone(),
            before_weight_pct: 0.0,
            after_weight_pct: round_pct(candidate.target_weight_pct),
            weight_delta_pct: round_pct(candidate.target_weight_pct),
            action_reason: "approved_candidate_entry".to_string(),
        })
        .collect::<Vec<_>>();
    entry_actions.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    entry_actions
}

fn build_trim_actions(
    live_positions: &[SecurityPortfolioLivePositionEntry],
) -> Vec<SecurityPortfolioReplacementAction> {
    let mut trim_actions = live_positions
        .iter()
        .filter_map(|live_position| {
            let target_weight_pct = live_position.target_weight_pct?;
            (target_weight_pct > 0.0 && target_weight_pct + 1e-9 < live_position.current_weight_pct)
                .then_some(SecurityPortfolioReplacementAction {
                    symbol: live_position.symbol.clone(),
                    before_weight_pct: round_pct(live_position.current_weight_pct),
                    after_weight_pct: round_pct(target_weight_pct),
                    weight_delta_pct: round_pct(
                        target_weight_pct - live_position.current_weight_pct,
                    ),
                    action_reason: "target_weight_below_current".to_string(),
                })
        })
        .collect::<Vec<_>>();
    trim_actions.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    trim_actions
}

fn build_exit_actions(
    live_positions: &[SecurityPortfolioLivePositionEntry],
) -> Vec<SecurityPortfolioReplacementAction> {
    let mut exit_actions = live_positions
        .iter()
        .filter_map(|live_position| {
            let target_weight_pct = live_position
                .target_weight_pct
                .unwrap_or(live_position.current_weight_pct);
            (target_weight_pct <= 0.0).then_some(SecurityPortfolioReplacementAction {
                symbol: live_position.symbol.clone(),
                before_weight_pct: round_pct(live_position.current_weight_pct),
                after_weight_pct: 0.0,
                weight_delta_pct: round_pct(-live_position.current_weight_pct),
                action_reason: "target_weight_zero".to_string(),
            })
        })
        .collect::<Vec<_>>();
    exit_actions.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    exit_actions
}

fn build_replacement_pairs(
    trim_actions: &[SecurityPortfolioReplacementAction],
    entry_actions: &[SecurityPortfolioReplacementAction],
) -> Vec<SecurityPortfolioReplacementPair> {
    let mut replacement_pairs = trim_actions
        .iter()
        .zip(entry_actions.iter())
        .map(
            |(trim_action, entry_action)| SecurityPortfolioReplacementPair {
                from_symbol: trim_action.symbol.clone(),
                to_symbol: entry_action.symbol.clone(),
                migrated_weight_pct: round_pct(
                    trim_action
                        .weight_delta_pct
                        .abs()
                        .min(entry_action.after_weight_pct),
                ),
            },
        )
        .collect::<Vec<_>>();
    replacement_pairs.sort_by(|left, right| {
        left.from_symbol
            .cmp(&right.from_symbol)
            .then(left.to_symbol.cmp(&right.to_symbol))
    });
    replacement_pairs
}

fn build_weight_map(weights: &[SecurityPortfolioWeightSnapshot]) -> BTreeMap<String, f64> {
    weights
        .iter()
        .map(|weight| (weight.symbol.clone(), weight.weight_pct))
        .collect()
}

fn compute_gross_turnover_weight_pct(
    current_weight_map: &BTreeMap<String, f64>,
    target_weight_map: &BTreeMap<String, f64>,
) -> f64 {
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
    live_positions: &[SecurityPortfolioLivePositionEntry],
    approved_candidate_entries: &[SecurityPortfolioApprovedCandidateEntry],
) -> f64 {
    live_positions
        .iter()
        .map(|live_position| live_position.risk_budget_pct.unwrap_or(0.0))
        .sum::<f64>()
        + approved_candidate_entries
            .iter()
            .map(|candidate| candidate.risk_budget_pct)
            .sum::<f64>()
}

fn validate_weight_conservation(
    section_name: &str,
    total_weight_pct: f64,
) -> Result<(), SecurityPortfolioReplacementPlanError> {
    if total_weight_pct < -1e-9 || total_weight_pct > 1.0 + 1e-9 {
        return Err(
            SecurityPortfolioReplacementPlanError::WeightNonConservation(section_name.to_string()),
        );
    }

    Ok(())
}

fn resolve_rebased_capital_base_amount_after(
    account_rebase_snapshot: &SecurityAccountRebaseSnapshot,
) -> Option<f64> {
    account_rebase_snapshot
        .rebased_position_contracts
        .first()
        .map(|position_contract| position_contract.capital_base_amount)
}

fn build_conflict_resolution_summary(has_rebase_context: bool) -> Vec<String> {
    let mut summary = vec![
        "kelly_signal=not_provided; fallback=target_weight_constraints".to_string(),
        "volatility_target=not_provided; fallback=constraint_only_solver".to_string(),
        "risk_budget_limit=applied".to_string(),
    ];

    if has_rebase_context {
        summary.push("capital_rebase_context=applied".to_string());
    }

    summary
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

fn round_pct(value: f64) -> f64 {
    (value * 10000.0).round() / 10000.0
}
