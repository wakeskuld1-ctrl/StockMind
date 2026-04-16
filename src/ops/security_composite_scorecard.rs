use serde::{Deserialize, Serialize};

use crate::ops::stock::security_decision_card::SecurityDecisionCard;
use crate::ops::stock::security_master_scorecard::{
    SecurityMasterScorecardDocument, SecurityMasterScorecardPredictionSummary,
};
use crate::ops::stock::security_risk_gates::SecurityRiskGateResult;

const SECURITY_COMPOSITE_SCORECARD_CONTRACT_VERSION: &str = "security_composite_scorecard.v1";

// 2026-04-16 CST: Added because plan A needs one formal business-layer composite object
// that can be landed in parallel with the current refactor.
// Reason: the approved product direction is no longer "single tomorrow up/down output",
// but a governed synthesis of current state, payoff/drawdown, and prediction assistance.
// Purpose: create one minimal, stable contract that later committee/chair adapters can consume
// without reopening runtime ownership or creating a second securities mainline.
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityCompositeScorecardBuildInput {
    pub generated_at: String,
    pub decision_card: SecurityDecisionCard,
    pub risk_gates: Vec<SecurityRiskGateResult>,
    pub master_scorecard: SecurityMasterScorecardDocument,
}

// 2026-04-16 CST: Added because the composite scorecard still needs one compact downstream
// payload for governance consumers.
// Reason: later committee/chair integration should not re-read every raw score field just to
// know what status, action, and next steps are currently governed.
// Purpose: provide a stable adapter-shaped payload without yet changing the existing committee path.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCompositeCommitteePayload {
    pub decision_status: String,
    pub recommendation_action: String,
    pub exposure_side: String,
    pub required_next_actions: Vec<String>,
}

// 2026-04-16 CST: Added because the business layer needs a first formal artifact for the
// "present state + payoff/drawdown + prediction assistance + gate" synthesis.
// Reason: this must become a real governed object before we decide how far to wire it into
// committee, chair, and later presentation flows.
// Purpose: freeze the minimum stable field set for the composite scorecard while keeping the
// implementation small and refactor-safe.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCompositeScorecardDocument {
    pub composite_scorecard_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub symbol: String,
    pub analysis_date: String,
    pub decision_id: String,
    pub committee_session_ref: String,
    pub master_scorecard_ref: String,
    pub current_state_score: f64,
    pub current_state_stance: String,
    pub payoff_drawdown_score: f64,
    pub prediction_score: Option<f64>,
    pub composite_score: f64,
    pub composite_actionability: String,
    pub gate_status: String,
    pub top_positive_drivers: Vec<String>,
    pub top_negative_drivers: Vec<String>,
    pub why_not_actionable: Vec<String>,
    pub committee_payload: SecurityCompositeCommitteePayload,
}

// 2026-04-16 CST: Added because the first landing of the composite scorecard should remain a
// pure builder over already-governed inputs.
// Reason: this lets us validate the new business object with TDD before touching dispatcher,
// runtime, or the current committee execution path.
// Purpose: turn existing decision-card, gate, and master-scorecard outputs into one stable
// composite document with explicit layer scores and caveats.
pub fn build_security_composite_scorecard(
    input: &SecurityCompositeScorecardBuildInput,
) -> SecurityCompositeScorecardDocument {
    let current_state_score = round_score(input.decision_card.confidence_score * 100.0);
    let payoff_drawdown_score = compute_payoff_drawdown_score(&input.master_scorecard);
    let prediction_score =
        compute_prediction_score(input.master_scorecard.prediction_summary.as_ref());
    let gate_status = derive_gate_status(&input.risk_gates);
    let composite_score =
        compute_composite_score(current_state_score, payoff_drawdown_score, prediction_score);
    let top_positive_drivers = collect_positive_drivers(
        &input.decision_card,
        &input.risk_gates,
        &input.master_scorecard,
    );
    let top_negative_drivers = collect_negative_drivers(&input.risk_gates, &input.master_scorecard);
    let why_not_actionable =
        collect_actionability_caveats(&input.risk_gates, &input.master_scorecard);
    let composite_actionability =
        derive_composite_actionability(&gate_status, &input.decision_card.status, composite_score);

    SecurityCompositeScorecardDocument {
        composite_scorecard_id: format!("composite-scorecard-{}", input.decision_card.decision_id),
        contract_version: SECURITY_COMPOSITE_SCORECARD_CONTRACT_VERSION.to_string(),
        document_type: "security_composite_scorecard".to_string(),
        generated_at: normalize_generated_at(input),
        symbol: input.decision_card.symbol.clone(),
        analysis_date: input.decision_card.analysis_date.clone(),
        decision_id: input.decision_card.decision_id.clone(),
        committee_session_ref: input.master_scorecard.committee_session_ref.clone(),
        master_scorecard_ref: input.master_scorecard.master_scorecard_id.clone(),
        current_state_score,
        current_state_stance: map_current_state_stance(&input.decision_card.recommendation_action),
        payoff_drawdown_score,
        prediction_score,
        composite_score,
        composite_actionability,
        gate_status,
        top_positive_drivers,
        top_negative_drivers,
        why_not_actionable,
        committee_payload: SecurityCompositeCommitteePayload {
            decision_status: input.decision_card.status.clone(),
            recommendation_action: input.decision_card.recommendation_action.clone(),
            exposure_side: input.decision_card.exposure_side.clone(),
            required_next_actions: input.decision_card.required_next_actions.clone(),
        },
    }
}

// 2026-04-16 CST: Added because the first implementation needs a deterministic fallback
// timestamp policy without introducing new runtime ownership.
// Reason: callers may stage the build input with or without an explicit generation time.
// Purpose: prefer the explicit build timestamp and otherwise fall back to the already-built
// master scorecard timestamp.
fn normalize_generated_at(input: &SecurityCompositeScorecardBuildInput) -> String {
    if input.generated_at.trim().is_empty() {
        return input.master_scorecard.generated_at.clone();
    }

    input.generated_at.clone()
}

// 2026-04-16 CST: Added because the product discussion explicitly froze current-state as the
// primary layer and kept it separate from predicted outcome quality.
// Reason: a clean label helps later consumers explain whether the current tape is constructive,
// watchful, or defensive.
// Purpose: map the existing decision action into one small, stable state label.
fn map_current_state_stance(recommendation_action: &str) -> String {
    match recommendation_action {
        "buy" => "constructive".to_string(),
        "hold" => "watchful".to_string(),
        "reduce" => "de_risk".to_string(),
        "avoid" => "defensive".to_string(),
        other => format!("mapped:{other}"),
    }
}

// 2026-04-16 CST: Added because payoff/drawdown should be a distinct layer rather than a hidden
// side effect of the final composite score.
// Reason: the user explicitly asked us not to collapse the system into a black-box total score.
// Purpose: keep a standalone odds-quality number built from the existing master-scorecard fields.
fn compute_payoff_drawdown_score(master_scorecard: &SecurityMasterScorecardDocument) -> f64 {
    let base_score = (master_scorecard.profitability_effectiveness_score * 0.55)
        + (master_scorecard.risk_resilience_score * 0.45);
    let probability_spread_bonus = master_scorecard
        .prediction_summary
        .as_ref()
        .and_then(|summary| {
            let upside = summary.risk_line.expected_upside_first_probability?;
            let stop = summary.risk_line.expected_stop_first_probability?;
            Some(((upside - stop) * 25.0).clamp(-10.0, 10.0))
        })
        .unwrap_or(0.0);

    round_score((base_score + probability_spread_bonus).clamp(0.0, 100.0))
}

// 2026-04-16 CST: Added because prediction remains a useful auxiliary layer, but not every
// governed path currently has a full prediction summary ready.
// Reason: the agreed design requires graceful degradation instead of hard failure when that
// auxiliary layer is temporarily unavailable.
// Purpose: produce a stable optional score only when the prediction summary exists.
fn compute_prediction_score(
    prediction_summary: Option<&SecurityMasterScorecardPredictionSummary>,
) -> Option<f64> {
    let summary = prediction_summary?;
    let mut components = Vec::new();

    if let Some(expected_return) = summary.regression_line.expected_return {
        // 2026-04-16 CST: Modified because expected return is already a bounded auxiliary signal,
        // not a full trading target by itself.
        // Purpose: normalize it into a 0-100 helper scale without pretending it is a calibrated
        // production weight.
        components.push((((expected_return + 0.05) / 0.20) * 100.0).clamp(0.0, 100.0));
    }
    if let Some(path_quality) = summary.regression_line.expected_path_quality {
        components.push((path_quality * 100.0).clamp(0.0, 100.0));
    }
    if let (Some(upside), Some(stop)) = (
        summary.risk_line.expected_upside_first_probability,
        summary.risk_line.expected_stop_first_probability,
    ) {
        components.push((((upside - stop) + 0.5) * 100.0).clamp(0.0, 100.0));
    }

    if components.is_empty() {
        return None;
    }

    Some(round_score(
        components.iter().sum::<f64>() / components.len() as f64,
    ))
}

// 2026-04-16 CST: Added because the current landing still needs one small deterministic fusion
// rule before we wire a trained fusion layer.
// Reason: the product discussion explicitly forbids pretending bootstrap weights are already
// validated production weights.
// Purpose: use transparent bootstrap weights that keep current-state dominant and prediction auxiliary.
fn compute_composite_score(
    current_state_score: f64,
    payoff_drawdown_score: f64,
    prediction_score: Option<f64>,
) -> f64 {
    let score = match prediction_score {
        Some(prediction_score) => {
            (current_state_score * 0.45)
                + (payoff_drawdown_score * 0.35)
                + (prediction_score * 0.20)
        }
        None => (current_state_score * 0.55) + (payoff_drawdown_score * 0.45),
    };

    round_score(score.clamp(0.0, 100.0))
}

// 2026-04-16 CST: Added because governance consumers need one compact gate summary instead of
// re-walking every individual gate just to decide whether the object is blocked or clear.
// Reason: the composite scorecard should surface a stable top-line gate signal.
// Purpose: compress the existing gate list into blocked/warning/clear.
fn derive_gate_status(risk_gates: &[SecurityRiskGateResult]) -> String {
    if risk_gates
        .iter()
        .any(|gate| gate.blocking && gate.result == "fail")
    {
        return "blocked".to_string();
    }
    if risk_gates.iter().any(|gate| gate.result == "warn") {
        return "warning".to_string();
    }

    "clear".to_string()
}

// 2026-04-16 CST: Added because the composite object needs a business-level actionability label
// that stays separate from the raw committee action.
// Reason: "buy/hold/avoid" and "can this enter review right now" are related but not identical.
// Purpose: encode the current governance readiness without mutating the existing decision card.
fn derive_composite_actionability(
    gate_status: &str,
    decision_status: &str,
    composite_score: f64,
) -> String {
    if gate_status == "blocked" {
        return "gated".to_string();
    }
    if decision_status == "needs_more_evidence" {
        return "needs_more_evidence".to_string();
    }
    if decision_status == "ready_for_review" && composite_score >= 60.0 {
        return "review_ready".to_string();
    }
    if composite_score >= 55.0 {
        return "watchlist".to_string();
    }

    "avoid_for_now".to_string()
}

// 2026-04-16 CST: Added because the first composite document still needs a compact explanation
// of what is working in its favor.
// Reason: later presentation layers should not have to invent positive driver labels from scratch.
// Purpose: surface three stable positive driver tags from already-governed inputs.
fn collect_positive_drivers(
    decision_card: &SecurityDecisionCard,
    risk_gates: &[SecurityRiskGateResult],
    master_scorecard: &SecurityMasterScorecardDocument,
) -> Vec<String> {
    let mut drivers = vec![
        format!("committee_action={}", decision_card.recommendation_action),
        format!("master_signal={}", master_scorecard.master_signal),
    ];
    if let Some(gate) = risk_gates.iter().find(|gate| gate.result == "pass") {
        drivers.push(format!("gate_pass={}", gate.gate_name));
    }

    drivers.truncate(3);
    drivers
}

// 2026-04-16 CST: Added because the user repeatedly asked for explicit reasons instead of an
// opaque conservative outcome.
// Reason: the composite object should make its main blockers obvious before any later UI wording.
// Purpose: surface fail/warn/model-readiness negatives in one deterministic order.
fn collect_negative_drivers(
    risk_gates: &[SecurityRiskGateResult],
    master_scorecard: &SecurityMasterScorecardDocument,
) -> Vec<String> {
    let mut drivers = risk_gates
        .iter()
        .filter(|gate| gate.result == "fail")
        .map(|gate| format!("gate_fail={}", gate.gate_name))
        .collect::<Vec<_>>();
    drivers.extend(
        risk_gates
            .iter()
            .filter(|gate| gate.result == "warn")
            .map(|gate| format!("gate_warn={}", gate.gate_name)),
    );
    if master_scorecard.scorecard_status != "ready" {
        drivers.push(format!(
            "scorecard_status={}",
            master_scorecard.scorecard_status
        ));
    }

    drivers.truncate(3);
    drivers
}

// 2026-04-16 CST: Added because the user explicitly called out the risk of a system that can
// always hide behind abstain or defer without saying why.
// Reason: the first composite artifact must preserve concrete caveats even when it is still usable.
// Purpose: collect blocking/warning gates and auxiliary layer gaps into a stable explanation list.
fn collect_actionability_caveats(
    risk_gates: &[SecurityRiskGateResult],
    master_scorecard: &SecurityMasterScorecardDocument,
) -> Vec<String> {
    let mut caveats = risk_gates
        .iter()
        .filter(|gate| gate.blocking && gate.result == "fail")
        .map(|gate| format!("blocking_gate:{}", gate.gate_name))
        .collect::<Vec<_>>();
    caveats.extend(
        risk_gates
            .iter()
            .filter(|gate| gate.result == "warn")
            .map(|gate| format!("warning_gate:{}", gate.gate_name)),
    );
    if master_scorecard.prediction_summary.is_none() {
        caveats.push("prediction_layer_unavailable".to_string());
    }

    caveats
}

// 2026-04-16 CST: Added because layered scores should stay stable at one decimal precision for
// tests and later UI consumption.
// Reason: we do not need noisy floating-point tails in the first formal contract.
// Purpose: keep output deterministic and easier to compare in tests/logs.
fn round_score(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}
