use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_account_open_position_snapshot::{
    SecurityActivePositionBookDocument, SecurityActivePositionDocument,
};
use crate::ops::stock::security_master_scorecard::SecurityMasterScorecardDocument;
use crate::ops::stock::security_position_contract::SecurityPositionContract;

const SECURITY_PER_POSITION_EVALUATION_DOCUMENT_TYPE: &str = "security_per_position_evaluation";
const SECURITY_PER_POSITION_EVALUATION_VERSION: &str = "security_per_position_evaluation.v1";

// 2026-04-18 CST: Added because Task 4 needs one formal account-level request
// shell for the daily monitoring evaluation pass.
// Reason: later monitoring evidence should consume one governed evaluation input
// bundle instead of reassembling active positions, contracts, and scorecards ad hoc.
// Purpose: freeze the minimal public request surface for the per-position evaluation layer.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPerPositionEvaluationRequest {
    pub active_position_book: SecurityActivePositionBookDocument,
    pub position_contracts: Vec<SecurityPositionContract>,
    #[serde(default)]
    pub master_scorecards: Vec<SecurityMasterScorecardDocument>,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-18 CST: Added because the monitoring layer should persist explicit
// actionability numbers rather than hide them inside one free-form summary string.
// Reason: Task 5 will need direct numeric action candidates for account aggregation.
// Purpose: define a compact governed score surface for hold/add/trim/replace/exit.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPerPositionActionScores {
    pub hold_score: f64,
    pub add_score: f64,
    pub trim_score: f64,
    pub replace_score: f64,
    pub exit_score: f64,
}

// 2026-04-18 CST: Added because Task 4 introduces the first formal single-name
// monitoring artifact between the active-position book and account aggregation.
// Reason: the approved design requires one stable business object per live holding
// before portfolio-level evidence packages are assembled.
// Purpose: define the governed single-position evaluation document.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPerPositionEvaluation {
    pub per_position_evaluation_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub account_id: String,
    pub symbol: String,
    #[serde(default)]
    pub security_name: Option<String>,
    pub analysis_date: String,
    pub contract_status: String,
    pub position_state: String,
    pub current_weight_pct: f64,
    pub target_weight_pct: f64,
    pub max_weight_pct: f64,
    pub current_vs_target_gap_pct: f64,
    pub current_vs_max_gap_pct: f64,
    pub updated_expected_return_pct: f64,
    pub updated_expected_drawdown_pct: f64,
    #[serde(default)]
    pub expected_payoff_ratio: Option<f64>,
    pub action_scores: SecurityPerPositionActionScores,
    pub recommended_action: String,
    pub expectation_source: String,
    #[serde(default)]
    pub price_as_of_date: Option<String>,
    #[serde(default)]
    pub resolved_trade_date: Option<String>,
    #[serde(default)]
    pub current_price: Option<f64>,
    #[serde(default)]
    pub holding_total_return_pct: Option<f64>,
    #[serde(default)]
    pub breakeven_price: Option<f64>,
    #[serde(default)]
    pub sector_tag: Option<String>,
    pub position_contract_ref: String,
    pub active_position_book_ref: String,
    #[serde(default)]
    pub source_execution_record_ref: Option<String>,
    #[serde(default)]
    pub master_scorecard_ref: Option<String>,
    pub evaluation_summary: String,
}

// 2026-04-18 CST: Added because the public tool route should return one named
// account-level evaluation package instead of a bare vector.
// Reason: this keeps the CLI contract extensible for the next monitoring-evidence task.
// Purpose: wrap the account evaluation batch in a stable result shell.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPerPositionEvaluationResult {
    pub account_id: String,
    pub evaluation_count: usize,
    pub source_active_position_book_ref: String,
    pub evaluations: Vec<SecurityPerPositionEvaluation>,
}

// 2026-04-18 CST: Added because Task 4 needs one explicit error boundary
// before later account aggregation depends on the evaluation layer.
// Reason: the approved flow does not allow active holdings to bypass the live contract layer.
// Purpose: keep missing or mismatched contract failures explicit and auditable.
#[derive(Debug, Error)]
pub enum SecurityPerPositionEvaluationError {
    #[error(
        "security per-position evaluation build failed: active position `{0}` has no matching position contract"
    )]
    MissingMatchingPositionContract(String),
    #[error(
        "security per-position evaluation build failed: position contract `{0}` does not belong to account `{1}`"
    )]
    PositionContractAccountMismatch(String, String),
}

// 2026-04-18 CST: Added because Task 4 needs one deterministic builder for a
// single active holding on the monitoring path.
// Reason: later tasks should reuse one symbol-local evaluation builder instead of
// duplicating scoring logic inside account aggregation and evidence packaging.
// Purpose: centralize the first single-position evaluation rule set.
pub fn build_security_per_position_evaluation(
    active_position_book: &SecurityActivePositionBookDocument,
    active_position: &SecurityActivePositionDocument,
    position_contract: &SecurityPositionContract,
    master_scorecard: Option<&SecurityMasterScorecardDocument>,
    created_at: &str,
) -> SecurityPerPositionEvaluation {
    let updated_expected_return_pct =
        resolve_expected_return_pct(position_contract, master_scorecard);
    let updated_expected_drawdown_pct =
        resolve_expected_drawdown_pct(position_contract, master_scorecard);
    let current_vs_target_gap_pct =
        round_pct(position_contract.target_weight_pct - active_position.current_weight_pct);
    let current_vs_max_gap_pct =
        round_pct(position_contract.max_weight_pct - active_position.current_weight_pct);
    let expected_payoff_ratio =
        compute_expected_payoff_ratio(updated_expected_return_pct, updated_expected_drawdown_pct);
    let action_scores = build_action_scores(
        active_position,
        position_contract,
        master_scorecard,
        updated_expected_return_pct,
        updated_expected_drawdown_pct,
    );
    let recommended_action = select_recommended_action(&action_scores);
    let expectation_source = if master_scorecard
        .and_then(|scorecard| scorecard.prediction_summary.as_ref())
        .is_some()
    {
        "master_scorecard_prediction_summary".to_string()
    } else {
        "position_contract_fallback".to_string()
    };
    let scorecard_signal = master_scorecard
        .map(|scorecard| scorecard.master_signal.clone())
        .unwrap_or_else(|| "unavailable".to_string());

    SecurityPerPositionEvaluation {
        per_position_evaluation_id: format!(
            "per-position-evaluation:{}:{}",
            active_position_book.account_id, active_position.symbol
        ),
        contract_version: SECURITY_PER_POSITION_EVALUATION_VERSION.to_string(),
        document_type: SECURITY_PER_POSITION_EVALUATION_DOCUMENT_TYPE.to_string(),
        generated_at: normalize_created_at(created_at),
        account_id: active_position_book.account_id.clone(),
        symbol: active_position.symbol.clone(),
        security_name: position_contract.security_name.clone(),
        analysis_date: position_contract.analysis_date.clone(),
        contract_status: position_contract.contract_status.clone(),
        position_state: active_position.position_state.clone(),
        current_weight_pct: active_position.current_weight_pct,
        target_weight_pct: position_contract.target_weight_pct,
        max_weight_pct: position_contract.max_weight_pct,
        current_vs_target_gap_pct,
        current_vs_max_gap_pct,
        updated_expected_return_pct: round_pct(updated_expected_return_pct),
        updated_expected_drawdown_pct: round_pct(updated_expected_drawdown_pct),
        expected_payoff_ratio,
        action_scores,
        recommended_action: recommended_action.clone(),
        expectation_source,
        price_as_of_date: active_position.price_as_of_date.clone(),
        resolved_trade_date: active_position.resolved_trade_date.clone(),
        current_price: active_position.current_price,
        holding_total_return_pct: active_position.holding_total_return_pct,
        breakeven_price: active_position.breakeven_price,
        sector_tag: active_position.sector_tag.clone(),
        position_contract_ref: position_contract.position_contract_id.clone(),
        active_position_book_ref: active_position_book.active_position_book_id.clone(),
        source_execution_record_ref: active_position.source_execution_record_ref.clone(),
        master_scorecard_ref: master_scorecard
            .map(|scorecard| scorecard.master_scorecard_id.clone()),
        evaluation_summary: format!(
            "symbol {} scored `{}` with signal `{}` on current_weight={:.2}%, target_weight={:.2}%, expected_return={:.2}%, expected_drawdown={:.2}%",
            active_position.symbol,
            recommended_action,
            scorecard_signal,
            active_position.current_weight_pct * 100.0,
            position_contract.target_weight_pct * 100.0,
            updated_expected_return_pct * 100.0,
            updated_expected_drawdown_pct * 100.0
        ),
    }
}

// 2026-04-18 CST: Added because daily monitoring should evaluate every live
// holding in one governed batch for the account.
// Reason: later evidence packages and committee handoff need one account-scoped
// batch result rather than many ungrouped single-position calls.
// Purpose: expose the first formal account-level per-position evaluation builder.
pub fn build_security_per_position_evaluations_for_account(
    request: &SecurityPerPositionEvaluationRequest,
) -> Result<SecurityPerPositionEvaluationResult, SecurityPerPositionEvaluationError> {
    let mut evaluations = Vec::with_capacity(request.active_position_book.active_positions.len());

    for active_position in &request.active_position_book.active_positions {
        let Some(position_contract) = request
            .position_contracts
            .iter()
            .find(|contract| contract.symbol == active_position.symbol)
        else {
            return Err(
                SecurityPerPositionEvaluationError::MissingMatchingPositionContract(
                    active_position.symbol.clone(),
                ),
            );
        };

        if position_contract.account_id != request.active_position_book.account_id {
            return Err(
                SecurityPerPositionEvaluationError::PositionContractAccountMismatch(
                    position_contract.position_contract_id.clone(),
                    request.active_position_book.account_id.clone(),
                ),
            );
        }

        let master_scorecard = request
            .master_scorecards
            .iter()
            .find(|scorecard| scorecard.symbol == active_position.symbol);

        evaluations.push(build_security_per_position_evaluation(
            &request.active_position_book,
            active_position,
            position_contract,
            master_scorecard,
            &request.created_at,
        ));
    }

    Ok(SecurityPerPositionEvaluationResult {
        account_id: request.active_position_book.account_id.clone(),
        evaluation_count: evaluations.len(),
        source_active_position_book_ref: request
            .active_position_book
            .active_position_book_id
            .clone(),
        evaluations,
    })
}

// 2026-04-18 CST: Added because Task 4 should prefer refreshed prediction
// evidence when it exists, but still degrade to the contract baseline when absent.
// Reason: later monitoring should not fail just because one symbol lacks a fresh scorecard.
// Purpose: centralize the updated-expected-return resolution rule.
fn resolve_expected_return_pct(
    position_contract: &SecurityPositionContract,
    master_scorecard: Option<&SecurityMasterScorecardDocument>,
) -> f64 {
    master_scorecard
        .and_then(|scorecard| scorecard.prediction_summary.as_ref())
        .and_then(|summary| summary.regression_line.expected_return)
        .unwrap_or(position_contract.expected_annual_return_pct)
}

// 2026-04-18 CST: Added because the single-position evaluation should use the
// same fallback rule for downside expectations as for upside expectations.
// Reason: monitoring actionability depends on a complete payoff/drawdown pair.
// Purpose: centralize the updated-expected-drawdown resolution rule.
fn resolve_expected_drawdown_pct(
    position_contract: &SecurityPositionContract,
    master_scorecard: Option<&SecurityMasterScorecardDocument>,
) -> f64 {
    master_scorecard
        .and_then(|scorecard| scorecard.prediction_summary.as_ref())
        .and_then(|summary| summary.risk_line.expected_drawdown)
        .unwrap_or(position_contract.expected_drawdown_pct)
}

// 2026-04-18 CST: Added because the first Task 4 scoring model must stay small,
// deterministic, and easy to audit.
// Reason: this slice is the pure mathematical data path, not the future LLM decision layer.
// Purpose: compute one bounded numeric action surface from payoff, risk, and sizing gaps.
fn build_action_scores(
    active_position: &SecurityActivePositionDocument,
    position_contract: &SecurityPositionContract,
    master_scorecard: Option<&SecurityMasterScorecardDocument>,
    updated_expected_return_pct: f64,
    updated_expected_drawdown_pct: f64,
) -> SecurityPerPositionActionScores {
    let return_quality = normalize_range(updated_expected_return_pct, -0.05, 0.25);
    let drawdown_quality = 1.0 - normalize_range(updated_expected_drawdown_pct, 0.02, 0.15);
    let payoff_quality = normalize_range(
        compute_expected_payoff_ratio_raw(
            updated_expected_return_pct,
            updated_expected_drawdown_pct,
        ),
        0.5,
        3.0,
    );
    let headroom_to_target = normalize_ratio(
        position_contract.target_weight_pct - active_position.current_weight_pct,
        position_contract.target_weight_pct.max(0.0001),
    );
    let overweight_pressure = normalize_ratio(
        active_position.current_weight_pct - position_contract.target_weight_pct,
        position_contract.max_weight_pct.max(0.0001),
    );
    let max_breach_pressure = normalize_ratio(
        active_position.current_weight_pct - position_contract.max_weight_pct,
        position_contract.max_weight_pct.max(0.0001),
    );
    let downside_pressure = normalize_range(updated_expected_drawdown_pct, 0.05, 0.18);
    let weak_edge_pressure = 1.0 - return_quality;
    let signal_bias = signal_bias(master_scorecard);
    let hold_balance = 1.0
        - normalize_ratio(
            (active_position.current_weight_pct - position_contract.target_weight_pct).abs(),
            position_contract.max_weight_pct.max(0.0001),
        );

    let hold_score = round_score(
        (return_quality * 0.32)
            + (drawdown_quality * 0.28)
            + (hold_balance * 0.20)
            + (signal_bias.max(0.0) * 0.20),
    );
    let add_score = round_score(
        (return_quality * 0.30)
            + (payoff_quality * 0.22)
            + (headroom_to_target * 0.28)
            + (signal_bias.max(0.0) * 0.20),
    );
    let trim_score = round_score(
        (overweight_pressure * 0.42)
            + (downside_pressure * 0.18)
            + (weak_edge_pressure * 0.18)
            + ((-signal_bias).max(0.0) * 0.22),
    );
    let replace_score = round_score(
        (weak_edge_pressure * 0.30)
            + (downside_pressure * 0.22)
            + ((-signal_bias).max(0.0) * 0.28)
            + ((1.0 - payoff_quality) * 0.20),
    );
    let exit_score = round_score(
        (max_breach_pressure * 0.36)
            + (downside_pressure * 0.24)
            + (weak_edge_pressure * 0.22)
            + ((-signal_bias).max(0.0) * 0.18),
    );

    SecurityPerPositionActionScores {
        hold_score,
        add_score,
        trim_score,
        replace_score,
        exit_score,
    }
}

// 2026-04-18 CST: Added because the Task 4 action layer needs a deterministic
// top action for downstream monitoring evidence.
// Reason: later aggregation should not guess which score is intended to dominate.
// Purpose: turn the numeric score surface into one canonical recommended action.
fn select_recommended_action(action_scores: &SecurityPerPositionActionScores) -> String {
    let candidates = [
        ("hold", action_scores.hold_score),
        ("add", action_scores.add_score),
        ("trim", action_scores.trim_score),
        ("replace", action_scores.replace_score),
        ("exit", action_scores.exit_score),
    ];

    candidates
        .into_iter()
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .map(|candidate| candidate.0.to_string())
        .unwrap_or_else(|| "hold".to_string())
}

// 2026-04-18 CST: Added because scorecard-level stance should influence the
// deterministic action surface without introducing any future LLM behavior here.
// Reason: existing master-scorecard signals already encode a quantified directional bias.
// Purpose: convert the current signal text into one bounded numeric bias.
fn signal_bias(master_scorecard: Option<&SecurityMasterScorecardDocument>) -> f64 {
    let Some(master_scorecard) = master_scorecard else {
        return 0.0;
    };

    match master_scorecard.master_signal.to_ascii_lowercase().as_str() {
        "accumulate" | "buy" | "positive" | "bullish" => 1.0,
        "hold" | "neutral" => 0.4,
        "trim" | "reduce" | "negative" | "bearish" => -0.7,
        "exit" | "avoid" => -1.0,
        _ => 0.0,
    }
}

// 2026-04-18 CST: Added because payoff/drawdown should stay visible as a tiny
// derived monitoring metric on the evaluation document.
// Reason: later aggregation and human review both benefit from one precomputed odds proxy.
// Purpose: compute a rounded payoff ratio only when drawdown is meaningfully positive.
fn compute_expected_payoff_ratio(
    updated_expected_return_pct: f64,
    updated_expected_drawdown_pct: f64,
) -> Option<f64> {
    if updated_expected_drawdown_pct.abs() <= f64::EPSILON {
        None
    } else {
        Some(round_score(
            updated_expected_return_pct / updated_expected_drawdown_pct,
        ))
    }
}

// 2026-04-18 CST: Added because the action-score layer still needs the raw
// payoff ratio even when the rounded document field is optional.
// Reason: internal scoring should not depend on optional serialization choices.
// Purpose: provide a non-optional payoff ratio for the bounded scoring helpers.
fn compute_expected_payoff_ratio_raw(
    updated_expected_return_pct: f64,
    updated_expected_drawdown_pct: f64,
) -> f64 {
    if updated_expected_drawdown_pct.abs() <= f64::EPSILON {
        0.0
    } else {
        updated_expected_return_pct / updated_expected_drawdown_pct
    }
}

// 2026-04-18 CST: Added because Task 4 should normalize its own generated_at
// value the same way as the earlier post-open document layers.
// Reason: callers should not have to pre-fill timestamps for a pure data builder.
// Purpose: centralize created_at normalization for the evaluation layer.
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

// 2026-04-18 CST: Added because the first scoring helpers should stay bounded
// and stable when sizing gaps exceed the target range.
// Reason: raw ratios can easily exceed 1.0 and distort downstream comparisons.
// Purpose: clamp ratio-like values to the standard 0..1 score interval.
fn normalize_ratio(numerator: f64, denominator: f64) -> f64 {
    if denominator.abs() <= f64::EPSILON {
        return 0.0;
    }

    (numerator / denominator).clamp(0.0, 1.0)
}

// 2026-04-18 CST: Added because the scoring model uses multiple raw metrics
// with different native scales.
// Reason: the first Task 4 implementation should keep all metrics comparable on a 0..1 interval.
// Purpose: normalize one numeric value into the bounded score range.
fn normalize_range(value: f64, min: f64, max: f64) -> f64 {
    if (max - min).abs() <= f64::EPSILON {
        return 0.0;
    }

    ((value - min) / (max - min)).clamp(0.0, 1.0)
}

// 2026-04-18 CST: Added because the evaluation layer should serialize short,
// stable numeric values instead of long floating-point tails.
// Reason: monitoring evidence will likely be reviewed by both humans and later tools.
// Purpose: keep 0..1 action scores compact and readable.
fn round_score(value: f64) -> f64 {
    ((value.clamp(0.0, 1.0)) * 10000.0).round() / 10000.0
}

// 2026-04-18 CST: Added because percentage-like monitoring fields should also
// avoid long binary float tails in serialized output.
// Reason: later evidence packages will likely compare and display these values directly.
// Purpose: keep percentage fields stable and readable.
fn round_pct(value: f64) -> f64 {
    (value * 10000.0).round() / 10000.0
}
