use serde::{Deserialize, Serialize};

use crate::ops::stock::security_composite_scorecard::{
    SecurityCompositeScorecardBuildInput, SecurityCompositeScorecardDocument,
    build_security_composite_scorecard,
};
use crate::ops::stock::security_decision_briefing::{
    CommitteeEvidenceChecks, CommitteeExecutionDigest, CommitteeHistoricalDigest, CommitteePayload,
    CommitteeRecommendationDigest, CommitteeResonanceDigest, CommitteeRiskBreakdown,
    CommitteeRiskItem, CommitteeSubjectProfile, OddsBrief, PositionPlan,
};
use crate::ops::stock::security_decision_card::SecurityDecisionCard;
use crate::ops::stock::security_master_scorecard::{
    SecurityMasterScorecardDocument, SecurityMasterScorecardPredictionSummary,
};
use crate::ops::stock::security_risk_gates::SecurityRiskGateResult;

const COMMITTEE_SCHEMA_VERSION: &str = "committee-payload:v1";

// 2026-04-16 CST: Added because approved plan A needs one governed adapter that bridges
// the new composite business object into the old committee payload contract.
// Reason: we must land the composite scorecard on the formal committee payload path
// without rewriting the existing seven-seat committee engine or creating recursion.
// Purpose: keep the new business-layer synthesis and the old governed committee contract
// aligned through one small builder that can later be promoted deeper into the chain.
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityCompositeCommitteePayloadAdapterBuildInput {
    pub generated_at: String,
    pub master_scorecard: SecurityMasterScorecardDocument,
    pub decision_card: SecurityDecisionCard,
    pub risk_gates: Vec<SecurityRiskGateResult>,
    pub market_profile: Option<String>,
    pub sector_profile: Option<String>,
}

// 2026-04-16 CST: Added because the adapter needs to emit both the new composite object
// and the old committee payload in one atomic result.
// Reason: downstream code should not rebuild the composite scorecard twice just to expose
// the new document and also keep the committee payload on the formal path.
// Purpose: make one builder call produce the whole approved step-1 bridge output.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCompositeCommitteePayloadAdapterResult {
    pub composite_scorecard: SecurityCompositeScorecardDocument,
    pub committee_payload: CommitteePayload,
}

// 2026-04-16 CST: Added because plan A explicitly freezes the adapter as a pure builder
// over already-governed objects.
// Reason: this lets us keep the first integration slice low-risk, deterministic, and easy
// to test before any committee-engine rewiring happens.
// Purpose: transform the current master-scorecard layer plus decision facts into a formal
// committee payload that still respects risk_breakdown as the single source of truth.
pub fn build_security_composite_committee_payload_adapter(
    input: &SecurityCompositeCommitteePayloadAdapterBuildInput,
) -> SecurityCompositeCommitteePayloadAdapterResult {
    let composite_scorecard =
        build_security_composite_scorecard(&SecurityCompositeScorecardBuildInput {
            generated_at: input.generated_at.clone(),
            decision_card: input.decision_card.clone(),
            risk_gates: input.risk_gates.clone(),
            master_scorecard: input.master_scorecard.clone(),
        });
    let subject_profile = build_subject_profile(input);
    let risk_breakdown = build_risk_breakdown(&input.risk_gates);
    let key_risks = derive_key_risks(&risk_breakdown);
    let odds_digest = build_odds_digest(&input.master_scorecard);
    let position_digest = build_position_digest(&input.decision_card, &composite_scorecard);
    let committee_payload = CommitteePayload {
        symbol: input.master_scorecard.symbol.clone(),
        analysis_date: input.master_scorecard.analysis_date.clone(),
        recommended_action: input.decision_card.recommendation_action.clone(),
        confidence: confidence_label(input.decision_card.confidence_score).to_string(),
        subject_profile,
        risk_breakdown,
        key_risks,
        minority_objection_points: composite_scorecard.top_negative_drivers.clone(),
        evidence_version: format!(
            "security-composite-committee-adapter:{}:{}:v1",
            input.master_scorecard.symbol, input.master_scorecard.analysis_date
        ),
        briefing_digest: build_briefing_digest(&composite_scorecard, &input.master_scorecard),
        committee_schema_version: COMMITTEE_SCHEMA_VERSION.to_string(),
        recommendation_digest: build_recommendation_digest(
            &input.decision_card,
            &composite_scorecard,
        ),
        execution_digest: build_execution_digest(&input.decision_card),
        resonance_digest: build_resonance_digest(&composite_scorecard, &input.master_scorecard),
        evidence_checks: build_evidence_checks(&input.decision_card, &composite_scorecard),
        historical_digest: build_historical_digest(&input.master_scorecard),
        odds_digest,
        position_digest,
    };

    SecurityCompositeCommitteePayloadAdapterResult {
        composite_scorecard,
        committee_payload,
    }
}

// 2026-04-16 CST: Added because the governed committee payload now distinguishes subject
// profile explicitly instead of inferring it later from symbol prefixes or ad-hoc callers.
// Reason: the adapter must make one minimal subject-profile decision now so later committee
// consumers do not reopen symbol heuristics on the side.
// Purpose: keep the first landing small while still exposing asset-class and market-scope facts.
fn build_subject_profile(
    input: &SecurityCompositeCommitteePayloadAdapterBuildInput,
) -> CommitteeSubjectProfile {
    let market_scope = match input.market_profile.as_deref() {
        Some(profile) if profile.contains("hk") || profile.contains("hong_kong") => "hong_kong",
        Some(profile) if profile.contains("us") || profile.contains("america") => "us",
        _ => "china",
    };
    let asset_class = if input
        .sector_profile
        .as_deref()
        .unwrap_or_default()
        .contains("etf")
        || input
            .market_profile
            .as_deref()
            .unwrap_or_default()
            .contains("etf")
    {
        "etf"
    } else {
        "equity"
    };

    CommitteeSubjectProfile {
        asset_class: asset_class.to_string(),
        market_scope: market_scope.to_string(),
        committee_focus: if asset_class == "etf" {
            "fund_review".to_string()
        } else {
            "stock_review".to_string()
        },
    }
}

// 2026-04-16 CST: Added because `key_risks` must remain a derived summary rather than a
// second hand-maintained source of truth.
// Reason: the committee vote contract already hard-requires `key_risks` to come from the
// first headline of each risk bucket.
// Purpose: map the current gate list into stable categorized buckets first, then derive the
// legacy summary from those buckets.
fn build_risk_breakdown(risk_gates: &[SecurityRiskGateResult]) -> CommitteeRiskBreakdown {
    let mut breakdown = CommitteeRiskBreakdown {
        technical: Vec::new(),
        fundamental: Vec::new(),
        resonance: Vec::new(),
        execution: Vec::new(),
    };

    for gate in risk_gates {
        let category = risk_bucket_name(gate.gate_name.as_str()).to_string();
        let item = CommitteeRiskItem {
            category: category.clone(),
            severity: map_gate_severity(gate),
            headline: gate.reason.clone(),
            rationale: format!(
                "derived from gate `{}` with result `{}` and metric snapshot count {}",
                gate.gate_name,
                gate.result,
                gate.metric_snapshot.len()
            ),
        };

        match category.as_str() {
            "technical" => breakdown.technical.push(item),
            "fundamental" => breakdown.fundamental.push(item),
            "resonance" => breakdown.resonance.push(item),
            _ => breakdown.execution.push(item),
        }
    }

    breakdown
}

// 2026-04-16 CST: Added because the current gate set still lacks one formal category field.
// Reason: plan A must stay in the existing facts boundary, so the adapter needs one transparent
// mapping rule instead of inventing new upstream gate data.
// Purpose: keep the bucket rule centralized and easy to replace when gates later carry native
// categories.
fn risk_bucket_name(gate_name: &str) -> &'static str {
    if gate_name.contains("analysis_date")
        || gate_name.contains("market_alignment")
        || gate_name.contains("trend")
    {
        "technical"
    } else if gate_name.contains("fundamental")
        || gate_name.contains("financial")
        || gate_name.contains("disclosure")
    {
        "fundamental"
    } else if gate_name.contains("event")
        || gate_name.contains("resonance")
        || gate_name.contains("news")
    {
        "resonance"
    } else {
        "execution"
    }
}

// 2026-04-16 CST: Added because the committee payload contract expects structured risk items
// with a severity label, not only raw gate statuses.
// Reason: the adapter should preserve the relative seriousness of fail/warn/pass while staying
// deterministic and lightweight.
// Purpose: translate gate results into one minimal committee-friendly severity scale.
fn map_gate_severity(gate: &SecurityRiskGateResult) -> String {
    match (gate.result.as_str(), gate.blocking) {
        ("fail", true) => "high".to_string(),
        ("fail", false) => "medium".to_string(),
        ("warn", _) => "medium".to_string(),
        _ => "low".to_string(),
    }
}

// 2026-04-16 CST: Added because the adapter must follow the same strict derived-key-risk rule
// as the formal committee validator.
// Reason: otherwise the bridge would immediately drift from the governed payload contract.
// Purpose: keep risk_breakdown as the only source of truth for the legacy summary list.
fn derive_key_risks(risk_breakdown: &CommitteeRiskBreakdown) -> Vec<String> {
    let mut key_risks = Vec::new();
    for items in [
        &risk_breakdown.technical,
        &risk_breakdown.fundamental,
        &risk_breakdown.resonance,
        &risk_breakdown.execution,
    ] {
        if let Some(item) = items.first() {
            key_risks.push(item.headline.clone());
        }
    }

    key_risks
}

// 2026-04-16 CST: Added because the committee payload still needs a compact governed summary
// string even after the composite document exists.
// Reason: some existing consumers only scan one briefing digest before deciding whether the
// payload is worth presenting to later seats.
// Purpose: surface the approved actionability, gate state, and scorecard aggregation status
// in one small deterministic sentence.
fn build_briefing_digest(
    composite_scorecard: &SecurityCompositeScorecardDocument,
    master_scorecard: &SecurityMasterScorecardDocument,
) -> String {
    format!(
        "composite_actionability={} | gate_status={} | master_signal={} | aggregation_status={}",
        composite_scorecard.composite_actionability,
        composite_scorecard.gate_status,
        master_scorecard.master_signal,
        master_scorecard.aggregation_status
    )
}

// 2026-04-16 CST: Added because the old committee contract already consumes recommendation
// semantics through `final_stance` and `action_bias`.
// Reason: the bridge must populate these governed strings now, even if the scoring weights
// behind the new composite object are still bootstrap-level.
// Purpose: keep committee consumers on stable action labels without reaching back into the
// composite document internals.
fn build_recommendation_digest(
    decision_card: &SecurityDecisionCard,
    composite_scorecard: &SecurityCompositeScorecardDocument,
) -> CommitteeRecommendationDigest {
    CommitteeRecommendationDigest {
        final_stance: composite_scorecard.current_state_stance.clone(),
        action_bias: action_bias_label(decision_card.recommendation_action.as_str()).to_string(),
        summary: format!(
            "current_state={} | actionability={} | composite_score={:.1}",
            composite_scorecard.current_state_stance,
            composite_scorecard.composite_actionability,
            composite_scorecard.composite_score
        ),
        confidence: confidence_label(decision_card.confidence_score).to_string(),
    }
}

// 2026-04-16 CST: Added because the adapter must produce a formally shaped execution digest
// without reopening technical-indicator dependencies.
// Reason: step 1 is intentionally limited to existing facts, so execution thresholds need a
// transparent temporary projection rather than a second analytics pass.
// Purpose: provide sane non-zero execution placeholders that remain auditable and can later be
// replaced by richer pre-trade execution builders.
fn build_execution_digest(decision_card: &SecurityDecisionCard) -> CommitteeExecutionDigest {
    CommitteeExecutionDigest {
        add_trigger_price: 1.0,
        add_trigger_volume_ratio: 1.05,
        add_position_pct: if decision_card.recommendation_action == "buy" {
            0.10
        } else {
            0.0
        },
        reduce_trigger_price: 0.95,
        reduce_position_pct: if decision_card.recommendation_action == "reduce" {
            0.15
        } else {
            0.08
        },
        stop_loss_price: 0.90,
        invalidation_price: 0.85,
        rejection_zone: "adapter_placeholder_zone".to_string(),
        watch_points: decision_card.required_next_actions.clone(),
        explanation: vec![
            "adapter uses temporary governed execution placeholders".to_string(),
            "later execution builders should replace these thresholds with indicator-derived values"
                .to_string(),
        ],
    }
}

// 2026-04-16 CST: Added because the old committee payload still expects a compact resonance
// digest rather than reading the whole composite document.
// Reason: we need a minimal but consistent way to surface positive/negative driver counts and
// prediction-event context.
// Purpose: project composite-driver information into the stable committee digest shape.
fn build_resonance_digest(
    composite_scorecard: &SecurityCompositeScorecardDocument,
    master_scorecard: &SecurityMasterScorecardDocument,
) -> CommitteeResonanceDigest {
    CommitteeResonanceDigest {
        resonance_score: (composite_scorecard.composite_score / 100.0).clamp(0.0, 1.0),
        action_bias: action_bias_from_actionability(
            composite_scorecard.composite_actionability.as_str(),
            composite_scorecard
                .committee_payload
                .recommendation_action
                .as_str(),
        )
        .to_string(),
        top_positive_driver_names: composite_scorecard.top_positive_drivers.clone(),
        top_negative_driver_names: composite_scorecard.top_negative_drivers.clone(),
        event_override_titles: if master_scorecard.prediction_summary.is_some() {
            vec!["prediction_layer_attached".to_string()]
        } else {
            vec!["prediction_layer_missing".to_string()]
        },
    }
}

// 2026-04-16 CST: Added because the committee payload contract needs explicit readiness booleans.
// Reason: the bridge should express whether the payload is review-ready or still gated without
// making seats re-derive that from free-text strings.
// Purpose: map the composite actionability into the current minimum evidence-check surface.
fn build_evidence_checks(
    decision_card: &SecurityDecisionCard,
    composite_scorecard: &SecurityCompositeScorecardDocument,
) -> CommitteeEvidenceChecks {
    let briefing_ready = composite_scorecard.composite_actionability == "review_ready";
    let blocked = decision_card.status == "blocked";

    CommitteeEvidenceChecks {
        fundamental_ready: !blocked,
        technical_ready: !blocked,
        resonance_ready: !blocked,
        execution_ready: true,
        briefing_ready,
    }
}

// 2026-04-16 CST: Added because the adapter must not pretend historical research exists when
// this step only projects composite facts into the committee contract.
// Reason: formal committee consumers prefer an explicit unavailable boundary over fabricated
// analog-study numbers.
// Purpose: expose a stable unavailable historical digest until the true research layer is wired in.
fn build_historical_digest(
    master_scorecard: &SecurityMasterScorecardDocument,
) -> CommitteeHistoricalDigest {
    CommitteeHistoricalDigest {
        status: "unavailable".to_string(),
        historical_confidence: "unknown".to_string(),
        analog_sample_count: 0,
        analog_win_rate_10d: None,
        analog_loss_rate_10d: None,
        analog_flat_rate_10d: None,
        analog_avg_return_10d: None,
        analog_median_return_10d: None,
        analog_avg_win_return_10d: None,
        analog_avg_loss_return_10d: None,
        analog_payoff_ratio_10d: None,
        analog_expectancy_10d: None,
        expected_return_window: None,
        expected_drawdown_window: None,
        research_limitations: vec![format!(
            "adapter-only committee payload for {} still has no historical digest bridge",
            master_scorecard.master_scorecard_id
        )],
    }
}

// 2026-04-16 CST: Added because prediction can help the committee payload immediately, but
// the design explicitly treats it as an auxiliary layer.
// Reason: the adapter must degrade gracefully when prediction_summary is absent.
// Purpose: expose a tiny governed odds digest that is either available or unavailable.
fn build_odds_digest(master_scorecard: &SecurityMasterScorecardDocument) -> OddsBrief {
    let Some(prediction_summary) = master_scorecard.prediction_summary.as_ref() else {
        return OddsBrief::default();
    };

    OddsBrief {
        status: "available".to_string(),
        historical_confidence: "low".to_string(),
        sample_count: prediction_summary.cluster_line.analog_sample_count,
        win_rate_10d: prediction_summary
            .risk_line
            .expected_upside_first_probability,
        loss_rate_10d: prediction_summary.risk_line.expected_stop_first_probability,
        flat_rate_10d: None,
        avg_return_10d: prediction_summary.regression_line.expected_return,
        median_return_10d: prediction_summary.cluster_line.analog_avg_return,
        avg_win_return_10d: prediction_summary.cluster_line.analog_avg_return,
        avg_loss_return_10d: prediction_summary
            .cluster_line
            .analog_avg_drawdown
            .map(|value| -value),
        payoff_ratio_10d: predicted_payoff_ratio(prediction_summary),
        expectancy_10d: predicted_expectancy(prediction_summary),
        expected_return_window: prediction_summary
            .regression_line
            .expected_return
            .map(|value| format!("{:.2}%", value * 100.0)),
        expected_drawdown_window: prediction_summary
            .risk_line
            .expected_drawdown
            .map(|value| format!("{:.2}%", value * 100.0)),
        odds_grade: "prediction_proxy".to_string(),
        confidence_grade: "auxiliary".to_string(),
        rationale: vec![prediction_summary.cluster_line.cluster_rationale.clone()],
        research_limitations: vec![
            "adapter reuses prediction_summary as a temporary odds proxy".to_string(),
        ],
    }
}

// 2026-04-16 CST: Added because committee consumers already expect a position digest and step 1
// should keep that field usable without requiring the full position-planning chain.
// Reason: the adapter should emit an action-shaped position plan now, not wait for a later
// runtime integration pass.
// Purpose: map the existing decision action into one minimal governed position suggestion.
fn build_position_digest(
    decision_card: &SecurityDecisionCard,
    composite_scorecard: &SecurityCompositeScorecardDocument,
) -> PositionPlan {
    let mut position_digest = PositionPlan::default();
    position_digest.position_action =
        position_action_label(decision_card.recommendation_action.as_str()).to_string();
    position_digest.entry_mode = if composite_scorecard.composite_actionability == "review_ready" {
        "review_ready".to_string()
    } else {
        "gated".to_string()
    };
    position_digest.starter_position_pct = if decision_card.recommendation_action == "buy" {
        0.10
    } else {
        0.0
    };
    position_digest.max_position_pct = if decision_card.recommendation_action == "buy" {
        0.25
    } else {
        0.10
    };
    position_digest.add_on_trigger = "wait_for_execution_builder".to_string();
    position_digest.reduce_on_trigger = "review_negative_driver_change".to_string();
    position_digest.hard_stop_trigger = "review_blocking_gate".to_string();
    position_digest.position_risk_grade =
        confidence_label(decision_card.confidence_score).to_string();
    position_digest.regime_adjustment = composite_scorecard.composite_actionability.clone();
    position_digest.execution_notes = decision_card.required_next_actions.clone();
    position_digest.rationale = vec![
        format!(
            "derived from recommendation_action={}",
            decision_card.recommendation_action
        ),
        format!(
            "derived from composite_actionability={}",
            composite_scorecard.composite_actionability
        ),
    ];
    position_digest
}

// 2026-04-16 CST: Added because step 1 still needs one stable qualitative confidence label
// for the committee payload.
// Reason: the old contract is string-based and downstream seats already consume those levels.
// Purpose: map the current numeric confidence into a governed tri-level label.
fn confidence_label(confidence_score: f64) -> &'static str {
    if confidence_score >= 0.75 {
        "high"
    } else if confidence_score >= 0.55 {
        "medium"
    } else {
        "low"
    }
}

// 2026-04-16 CST: Added because the committee payload still uses a small action-bias enum in
// several seat heuristics.
// Reason: the bridge should keep those existing heuristics working instead of introducing a
// new label family in step 1.
// Purpose: translate recommendation actions into the current governed action-bias strings.
fn action_bias_label(recommendation_action: &str) -> &'static str {
    match recommendation_action {
        "buy" => "build_long",
        "hold" => "hold_and_confirm",
        "reduce" | "avoid" => "reduce_or_exit",
        _ => "watch_conflict",
    }
}

// 2026-04-16 CST: Added because resonance digest consumers care about whether the object is
// actionable or still gated, not only the raw action.
// Reason: composite actionability already expresses the current governance readiness state.
// Purpose: keep the resonance action bias aligned with both action and actionability.
fn action_bias_from_actionability(
    composite_actionability: &str,
    recommendation_action: &str,
) -> &'static str {
    match composite_actionability {
        "review_ready" => action_bias_label(recommendation_action),
        "watchlist" => "hold_and_confirm",
        "gated" | "avoid_for_now" => "reduce_or_exit",
        _ => "watch_conflict",
    }
}

// 2026-04-16 CST: Added because downstream position consumers already think in action verbs,
// not raw recommendation labels.
// Reason: the position layer and the recommendation layer are related but not identical.
// Purpose: keep the adapter payload readable for position consumers without inventing new states.
fn position_action_label(recommendation_action: &str) -> &'static str {
    match recommendation_action {
        "buy" => "build",
        "hold" => "wait",
        "reduce" => "trim",
        "avoid" => "avoid",
        _ => "wait",
    }
}

// 2026-04-16 CST: Added because the temporary odds digest still benefits from a simple payoff
// estimate when both return and drawdown proxies are available.
// Reason: this keeps the adapter output slightly more informative without pretending to be
// a trained historical research result.
// Purpose: derive one bounded payoff ratio from the prediction proxy fields.
fn predicted_payoff_ratio(
    prediction_summary: &SecurityMasterScorecardPredictionSummary,
) -> Option<f64> {
    let expected_return = prediction_summary.regression_line.expected_return?;
    let expected_drawdown = prediction_summary.risk_line.expected_drawdown?;
    if expected_drawdown.abs() <= f64::EPSILON {
        return None;
    }

    Some(expected_return / expected_drawdown)
}

// 2026-04-16 CST: Added because a temporary odds proxy should still expose one simple expected
// value proxy when the required fields exist.
// Reason: later consumers can compare this small adapter proxy with the true research-based
// expectancy once that layer is wired in.
// Purpose: keep the current bridge informative while remaining transparent about its limits.
fn predicted_expectancy(
    prediction_summary: &SecurityMasterScorecardPredictionSummary,
) -> Option<f64> {
    let win_rate = prediction_summary
        .risk_line
        .expected_upside_first_probability?;
    let loss_rate = prediction_summary
        .risk_line
        .expected_stop_first_probability?;
    let avg_return = prediction_summary.regression_line.expected_return?;
    let avg_drawdown = prediction_summary.risk_line.expected_drawdown?;

    Some((win_rate * avg_return) - (loss_rate * avg_drawdown))
}
