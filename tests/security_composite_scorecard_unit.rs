use excel_skill::ops::stock::security_decision_card::SecurityDecisionCard;
use excel_skill::ops::stock::security_master_scorecard::{
    SecurityMasterScorecardDocument, SecurityMasterScorecardPredictionClusterLine,
    SecurityMasterScorecardPredictionRegressionLine, SecurityMasterScorecardPredictionRiskLine,
    SecurityMasterScorecardPredictionSummary, SecurityMasterScorecardTrainedHeadSummary,
};
use excel_skill::ops::stock::security_risk_gates::SecurityRiskGateResult;
use excel_skill::ops::stock::stock_investment_case_entry::security_composite_scorecard::{
    SecurityCompositeScorecardBuildInput, build_security_composite_scorecard,
};

// 2026-04-16 CST: Added because the approved plan A needs one first failing test
// for the new composite scorecard contract before any production implementation.
// Reason: we need to lock the smallest formal business object, not jump straight into
// runtime or dispatcher rewiring.
// Purpose: prove the new capability is absent first, then implement only enough code
// to surface the layered scorecard semantics.
#[test]
fn composite_scorecard_surfaces_layer_scores_and_blockers() {
    let document = build_security_composite_scorecard(&SecurityCompositeScorecardBuildInput {
        generated_at: "2026-04-16T10:00:00+08:00".to_string(),
        decision_card: sample_decision_card("blocked", "avoid", 0.68),
        risk_gates: vec![
            sample_gate("analysis_date_gate", "pass", false, "analysis date frozen"),
            sample_gate(
                "risk_reward_gate",
                "fail",
                true,
                "target/stop ratio below minimum",
            ),
            sample_gate(
                "event_risk_gate",
                "warn",
                false,
                "event confirmation pending",
            ),
        ],
        master_scorecard: sample_master_scorecard(Some(sample_prediction_summary())),
    });

    assert_eq!(document.document_type, "security_composite_scorecard");
    assert_eq!(document.current_state_score, 68.0);
    assert_eq!(document.gate_status, "blocked");
    assert_eq!(document.composite_actionability, "gated");
    assert_eq!(document.committee_payload.decision_status, "blocked");
    assert_eq!(document.committee_payload.recommendation_action, "avoid");
    assert_eq!(
        document.top_negative_drivers,
        vec![
            "gate_fail=risk_reward_gate".to_string(),
            "gate_warn=event_risk_gate".to_string(),
            "scorecard_status=model_unavailable".to_string(),
        ]
    );
    assert!(
        document
            .why_not_actionable
            .contains(&"blocking_gate:risk_reward_gate".to_string())
    );
    assert!(
        document
            .why_not_actionable
            .contains(&"warning_gate:event_risk_gate".to_string())
    );
    assert!(document.prediction_score.is_some());
}

// 2026-04-16 CST: Added because the first implementation must also prove graceful
// degradation when the prediction layer is not available yet.
// Reason: the design explicitly freezes prediction as an auxiliary layer rather than
// a hard blocker for the whole business object.
// Purpose: keep the composite scorecard usable during staged rollout and current
// model-readiness gaps.
#[test]
fn composite_scorecard_degrades_when_prediction_layer_is_missing() {
    let document = build_security_composite_scorecard(&SecurityCompositeScorecardBuildInput {
        generated_at: "2026-04-16T10:05:00+08:00".to_string(),
        decision_card: sample_decision_card("ready_for_review", "buy", 0.74),
        risk_gates: vec![
            sample_gate("analysis_date_gate", "pass", false, "analysis date frozen"),
            sample_gate("market_alignment_gate", "pass", false, "market aligned"),
        ],
        master_scorecard: sample_master_scorecard(None),
    });

    assert_eq!(document.gate_status, "clear");
    assert_eq!(document.composite_actionability, "review_ready");
    assert_eq!(document.prediction_score, None);
    assert!(
        document
            .why_not_actionable
            .contains(&"prediction_layer_unavailable".to_string())
    );
    assert!(document.composite_score >= 60.0);
}

fn sample_decision_card(
    status: &str,
    recommendation_action: &str,
    confidence_score: f64,
) -> SecurityDecisionCard {
    SecurityDecisionCard {
        decision_id: "601916.SH-2026-04-16".to_string(),
        symbol: "601916.SH".to_string(),
        analysis_date: "2026-04-16".to_string(),
        status: status.to_string(),
        recommendation_action: recommendation_action.to_string(),
        exposure_side: if recommendation_action == "avoid" {
            "neutral".to_string()
        } else {
            "long".to_string()
        },
        direction: if recommendation_action == "avoid" {
            "neutral".to_string()
        } else {
            "long".to_string()
        },
        confidence_score,
        expected_return_range: "8.0% - 12.0%".to_string(),
        downside_risk: "5.0%".to_string(),
        position_size_suggestion: "starter".to_string(),
        required_next_actions: vec![
            "recheck market alignment".to_string(),
            "refresh event view".to_string(),
        ],
        final_recommendation: "use governed committee review".to_string(),
    }
}

fn sample_gate(
    gate_name: &str,
    result: &str,
    blocking: bool,
    reason: &str,
) -> SecurityRiskGateResult {
    SecurityRiskGateResult {
        gate_name: gate_name.to_string(),
        result: result.to_string(),
        blocking,
        reason: reason.to_string(),
        metric_snapshot: vec![format!("{gate_name}={result}")],
        remediation: None,
    }
}

fn sample_master_scorecard(
    prediction_summary: Option<SecurityMasterScorecardPredictionSummary>,
) -> SecurityMasterScorecardDocument {
    SecurityMasterScorecardDocument {
        master_scorecard_id: "master-scorecard-001".to_string(),
        contract_version: "security_master_scorecard.v1".to_string(),
        document_type: "security_master_scorecard".to_string(),
        generated_at: "2026-04-16T10:00:00+08:00".to_string(),
        symbol: "601916.SH".to_string(),
        analysis_date: "2026-04-16".to_string(),
        decision_id: "601916.SH-2026-04-16".to_string(),
        committee_session_ref: "committee-601916.SH-2026-04-16".to_string(),
        scorecard_ref: "scorecard-001".to_string(),
        scorecard_status: "model_unavailable".to_string(),
        aggregation_version: "historical_replay_v1".to_string(),
        aggregation_status: "historical_replay_only".to_string(),
        profitability_effectiveness_score: 72.0,
        risk_resilience_score: 64.0,
        path_quality_score: 61.0,
        master_score: 68.0,
        master_signal: "historically_effective".to_string(),
        trained_head_summary: SecurityMasterScorecardTrainedHeadSummary {
            head_count: 3,
            availability_status: "partial".to_string(),
            expected_return: Some(0.11),
            expected_drawdown: Some(0.06),
            expected_path_quality: Some(0.63),
            expected_upside_first_probability: Some(0.59),
            expected_stop_first_probability: Some(0.31),
        },
        prediction_summary,
        horizon_breakdown: Vec::new(),
        limitations: vec![
            "prediction analog sample still thin".to_string(),
            "scorecard registry not promoted".to_string(),
        ],
    }
}

fn sample_prediction_summary() -> SecurityMasterScorecardPredictionSummary {
    SecurityMasterScorecardPredictionSummary {
        prediction_mode: "prediction".to_string(),
        prediction_horizon_days: 20,
        regression_line: SecurityMasterScorecardPredictionRegressionLine {
            expected_return: Some(0.12),
            expected_path_quality: Some(0.67),
        },
        risk_line: SecurityMasterScorecardPredictionRiskLine {
            expected_drawdown: Some(0.05),
            expected_upside_first_probability: Some(0.62),
            expected_stop_first_probability: Some(0.28),
        },
        cluster_line: SecurityMasterScorecardPredictionClusterLine {
            regime_cluster_id: "cluster-1".to_string(),
            regime_cluster_label: "trend_supportive".to_string(),
            analog_sample_count: 48,
            analog_avg_return: Some(0.09),
            analog_avg_drawdown: Some(0.04),
            cluster_rationale: "trend and payoff both supportive".to_string(),
        },
    }
}
