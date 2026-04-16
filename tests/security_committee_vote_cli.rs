mod common;

use serde_json::json;

use crate::common::run_cli_with_json;

#[test]
fn tool_catalog_promotes_security_committee_vote_and_hides_legacy_committee_routes() {
    let output = run_cli_with_json("");

    // 2026-04-16 CST: Added because the legacy committee public-surface closeout
    // must not create a discovery hole after removing the old committee routes
    // from the formal stock catalog.
    // Purpose: prove the public catalog now exposes the formal committee mainline
    // while keeping the frozen legacy committee routes undiscoverable.
    let catalog = output["data"]["tool_catalog"]
        .as_array()
        .expect("tool catalog should be an array");

    assert!(
        catalog.iter().any(|tool| tool == "security_committee_vote"),
        "public tool catalog should include security_committee_vote"
    );
    assert!(
        !catalog
            .iter()
            .any(|tool| tool == "security_decision_committee"),
        "public tool catalog should not include frozen legacy security_decision_committee"
    );
    assert!(
        !catalog
            .iter()
            .any(|tool| tool == "security_committee_member_agent"),
        "public tool catalog should not include legacy/internal security_committee_member_agent"
    );
}

// 2026-04-16 CST: Add a direct CLI reproduction for the seat-agent contract,
// because the execution_record failure currently comes from a child-process
// dispatcher mismatch rather than from execution_record math itself.
// Purpose: lock the internal `security_committee_member_agent` tool onto the
// new `seat_role + committee_payload + committee_mode` request shape so the
// old `member_id` parser cannot silently reappear behind the CLI boundary.
#[test]
fn security_committee_member_agent_accepts_vote_contract_request_shape() {
    let request = json!({
        "tool": "security_committee_member_agent",
        "args": {
            "committee_payload": {
                "symbol": "601916.SH",
                "analysis_date": "2025-09-17",
                "recommended_action": "buy",
                "confidence": "medium",
                "subject_profile": {
                    "asset_class": "equity",
                    "market_scope": "china",
                    "committee_focus": "stock_review"
                },
                "risk_breakdown": {
                    "technical": [
                        {
                            "category": "technical",
                            "severity": "medium",
                            "headline": "breakout needs confirmation",
                            "rationale": "fixture rationale"
                        }
                    ],
                    "fundamental": [],
                    "resonance": [],
                    "execution": []
                },
                "key_risks": [
                    "breakout needs confirmation"
                ],
                "minority_objection_points": [],
                "evidence_version": "fixture-evidence",
                "briefing_digest": "fixture briefing digest",
                "committee_schema_version": "committee-payload:v1",
                "recommendation_digest": {
                    "final_stance": "constructive",
                    "action_bias": "buy",
                    "summary": "fixture summary",
                    "confidence": "medium"
                },
                "execution_digest": {
                    "add_trigger_price": 0.0,
                    "add_trigger_volume_ratio": 0.0,
                    "add_position_pct": 0.0,
                    "reduce_trigger_price": 0.0,
                    "reduce_position_pct": 0.0,
                    "stop_loss_price": 0.0,
                    "invalidation_price": 0.0,
                    "rejection_zone": "none",
                    "watch_points": [],
                    "explanation": []
                },
                "resonance_digest": {
                    "resonance_score": 0.0,
                    "action_bias": "neutral",
                    "top_positive_driver_names": [],
                    "top_negative_driver_names": [],
                    "event_override_titles": []
                },
                "evidence_checks": {
                    "fundamental_ready": true,
                    "technical_ready": true,
                    "resonance_ready": true,
                    "execution_ready": true,
                    "briefing_ready": true
                },
                "historical_digest": {
                    "status": "available",
                    "historical_confidence": "medium",
                    "analog_sample_count": 1,
                    "analog_win_rate_10d": 0.6,
                    "analog_loss_rate_10d": 0.4,
                    "analog_flat_rate_10d": 0.0,
                    "analog_avg_return_10d": 0.08,
                    "analog_median_return_10d": 0.08,
                    "analog_avg_win_return_10d": 0.12,
                    "analog_avg_loss_return_10d": -0.04,
                    "analog_payoff_ratio_10d": 3.0,
                    "analog_expectancy_10d": 0.056,
                    "expected_return_window": "10d",
                    "expected_drawdown_window": "10d",
                    "research_limitations": []
                },
                "odds_digest": {
                    "status": "available",
                    "historical_confidence": "medium",
                    "sample_count": 1,
                    "win_rate_10d": 0.6,
                    "loss_rate_10d": 0.4,
                    "flat_rate_10d": 0.0,
                    "avg_return_10d": 0.08,
                    "median_return_10d": 0.08,
                    "avg_win_return_10d": 0.12,
                    "avg_loss_return_10d": -0.04,
                    "payoff_ratio_10d": 3.0,
                    "expectancy_10d": 0.056,
                    "expected_return_window": "10d",
                    "expected_drawdown_window": "10d",
                    "odds_grade": "good",
                    "confidence_grade": "medium",
                    "rationale": [],
                    "research_limitations": []
                },
                "position_digest": {
                    "position_action": "build",
                    "entry_mode": "starter",
                    "starter_position_pct": 0.08,
                    "max_position_pct": 0.12,
                    "add_on_trigger": "breakout",
                    "reduce_on_trigger": "loss_of_momentum",
                    "hard_stop_trigger": "stop",
                    "liquidity_cap": "standard",
                    "position_risk_grade": "moderate",
                    "regime_adjustment": "neutral",
                    "execution_notes": [],
                    "rationale": []
                }
            },
            "committee_mode": "standard",
            "seat_role": "chair",
            "meeting_id": "fixture-meeting-001"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(output["data"]["role"], "chair");
    assert_eq!(output["data"]["member_id"], "committee-chair-001");
    assert_eq!(output["data"]["seat_kind"], "deliberation");
    // 2026-04-16 CST: Do not bind this CLI-surface regression test to a
    // specific runtime mode, because direct CLI invocation is allowed to run
    // as `in_process_fallback` while the real committee parent still drives
    // child-process execution for seat isolation.
    // Purpose: keep the test focused on the fixed request contract boundary
    // instead of overfitting to one invocation context.
    assert!(
        output["data"]["execution_mode"]
            .as_str()
            .expect("execution_mode should be present")
            .len()
            > 0
    );
}
