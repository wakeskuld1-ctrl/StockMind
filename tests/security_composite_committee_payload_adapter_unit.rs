use excel_skill::ops::stock::security_decision_briefing::CommitteePayload;
use excel_skill::ops::stock::security_decision_card::SecurityDecisionCard;
use excel_skill::ops::stock::security_master_scorecard::{
    SecurityMasterScorecardDocument, SecurityMasterScorecardPredictionClusterLine,
    SecurityMasterScorecardPredictionRegressionLine, SecurityMasterScorecardPredictionRiskLine,
    SecurityMasterScorecardPredictionSummary, SecurityMasterScorecardTrainedHeadSummary,
};
use excel_skill::ops::stock::security_risk_gates::SecurityRiskGateResult;
use excel_skill::ops::stock::stock_investment_case_entry::security_composite_committee_payload_adapter::{
    SecurityCompositeCommitteePayloadAdapterBuildInput,
    build_security_composite_committee_payload_adapter,
};

// 2026-04-16 CST: Added because step 1 of the approved plan A needs a real failing test
// for "composite scorecard -> committee payload adapter" before any implementation lands.
// Reason: this adapter is the first official bridge from the new composite business object
// into the governed committee payload layer.
// Purpose: lock both outputs at once so later changes do not re-split the facts.
#[test]
fn adapter_builds_composite_scorecard_and_committee_payload_with_derived_key_risks() {
    let result = build_security_composite_committee_payload_adapter(
        &SecurityCompositeCommitteePayloadAdapterBuildInput {
            generated_at: "2026-04-16T12:00:00+08:00".to_string(),
            master_scorecard: sample_master_scorecard(Some(sample_prediction_summary())),
            decision_card: sample_decision_card("blocked", "avoid", 0.68),
            risk_gates: vec![
                sample_gate("analysis_date_gate", "pass", false, "analysis date frozen"),
                sample_gate(
                    "event_risk_gate",
                    "warn",
                    false,
                    "event confirmation is still pending",
                ),
                sample_gate(
                    "risk_reward_gate",
                    "fail",
                    true,
                    "target stop ratio is below minimum threshold",
                ),
            ],
            market_profile: Some("a_share_core".to_string()),
            sector_profile: Some("a_share_bank".to_string()),
        },
    );

    assert_eq!(
        result.composite_scorecard.document_type,
        "security_composite_scorecard"
    );
    assert_eq!(
        result.committee_payload.committee_schema_version,
        "committee-payload:v1"
    );
    assert_eq!(result.committee_payload.symbol, "601916.SH");
    assert_eq!(result.committee_payload.recommended_action, "avoid");
    assert_eq!(
        result.committee_payload.key_risks,
        derive_key_risks(&result.committee_payload)
    );
    assert_eq!(
        result.committee_payload.key_risks,
        vec![
            "analysis date frozen".to_string(),
            "event confirmation is still pending".to_string(),
            "target stop ratio is below minimum threshold".to_string(),
        ]
    );
}

// 2026-04-16 CST: Added because the adapter must keep the committee payload usable
// even when the prediction layer has not been prepared yet.
// Reason: the approved composite design explicitly makes prediction an auxiliary layer.
// Purpose: prove the adapter degrades cleanly instead of blocking the whole committee payload.
#[test]
fn adapter_degrades_committee_payload_when_prediction_layer_is_missing() {
    let result = build_security_composite_committee_payload_adapter(
        &SecurityCompositeCommitteePayloadAdapterBuildInput {
            generated_at: "2026-04-16T12:10:00+08:00".to_string(),
            master_scorecard: sample_master_scorecard(None),
            decision_card: sample_decision_card("ready_for_review", "buy", 0.74),
            risk_gates: vec![
                sample_gate("analysis_date_gate", "pass", false, "analysis date frozen"),
                sample_gate("market_alignment_gate", "pass", false, "market aligned"),
            ],
            market_profile: Some("a_share_core".to_string()),
            sector_profile: Some("a_share_bank".to_string()),
        },
    );

    assert_eq!(result.composite_scorecard.prediction_score, None);
    assert_eq!(result.committee_payload.odds_digest.status, "unavailable");
    assert_eq!(
        result.committee_payload.position_digest.position_action,
        "build"
    );
    assert_eq!(
        result.committee_payload.subject_profile.asset_class,
        "equity"
    );
    assert!(
        result
            .committee_payload
            .briefing_digest
            .contains("review_ready")
    );
}

fn derive_key_risks(payload: &CommitteePayload) -> Vec<String> {
    let mut key_risks = Vec::new();
    for items in [
        &payload.risk_breakdown.technical,
        &payload.risk_breakdown.fundamental,
        &payload.risk_breakdown.resonance,
        &payload.risk_breakdown.execution,
    ] {
        if let Some(item) = items.first() {
            key_risks.push(item.headline.clone());
        }
    }
    key_risks
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
        generated_at: "2026-04-16T12:00:00+08:00".to_string(),
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
