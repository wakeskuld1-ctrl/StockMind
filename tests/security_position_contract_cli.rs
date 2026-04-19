mod common;

use serde_json::{Value, json};

use crate::common::run_cli_with_json;

// 2026-04-18 CST: Added because Task 2 must first freeze public discovery of
// the post-open live contract layer before deeper monitoring logic is built.
// Reason: the user approved `PositionContract` as the only formal live-governance
// object, so the tool itself must be discoverable from the public catalog.
// Purpose: lock the CLI surface before the implementation lands.
#[test]
fn tool_catalog_includes_security_position_contract() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_position_contract")
    );
}

// 2026-04-18 CST: Added because Task 2 needs one red-green contract proving
// the live contract can be formed only after the approved intake packet exists.
// Reason: the user fixed `ApprovedOpenPositionPacket -> PositionContract` as the
// first two objects on the pure post-open data path.
// Purpose: freeze the minimal contract shell and its core sizing fields.
#[test]
fn security_position_contract_builds_from_approved_packet_and_plan_seed() {
    let request = json!({
        "tool": "security_position_contract",
        "args": {
            "approved_open_position_packet": approved_open_position_packet_document(),
            "position_plan_document": position_plan_document()
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["position_contract"]["document_type"],
        "security_position_contract"
    );
    assert_eq!(
        output["data"]["position_contract"]["packet_id"],
        "packet-contract-1"
    );
    assert_eq!(output["data"]["position_contract"]["symbol"], "601916.SH");
    assert_eq!(
        output["data"]["position_contract"]["contract_status"],
        "pending_open"
    );
    assert_eq!(
        output["data"]["position_contract"]["initial_weight_pct"],
        0.03
    );
    assert_eq!(
        output["data"]["position_contract"]["target_weight_pct"],
        0.08
    );
    assert_eq!(output["data"]["position_contract"]["max_weight_pct"], 0.12);
    assert_eq!(
        output["data"]["position_contract"]["risk_budget_pct"],
        0.018
    );
}

// 2026-04-18 CST: Added because the live contract also needs to preserve
// governance-facing management rules, not only position sizing numbers.
// Reason: later monitoring and rebasing depend on stable exit and rebase policy
// text instead of reconstructing them from scattered packet fragments.
// Purpose: freeze the first policy-carrying version of `PositionContract`.
#[test]
fn security_position_contract_preserves_rebase_and_exit_policy() {
    let request = json!({
        "tool": "security_position_contract",
        "args": {
            "approved_open_position_packet": approved_open_position_packet_document(),
            "position_plan_document": position_plan_document()
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["position_contract"]["exit_policy"],
        "Exit when thesis breaks."
    );
    assert_eq!(
        output["data"]["position_contract"]["target_achievement_policy"],
        "Target reached."
    );
    assert_eq!(
        output["data"]["position_contract"]["rebase_policy"],
        "proportional_rebase_on_capital_event.v1"
    );
    assert_eq!(
        output["data"]["position_contract"]["approval_binding_ref"],
        "approval-binding:approval-session-1:committee-resolution-1:chair-resolution-1"
    );
}

// 2026-04-18 CST: Added because the live contract must reject mismatched
// packet/plan identity before any post-open state is created.
// Reason: `PositionContract` is the first formal live-governance object, so a
// symbol mismatch must fail fast instead of creating a mixed-source contract.
// Purpose: freeze the seed/packet symbol consistency rule at the CLI surface.
#[test]
fn security_position_contract_rejects_symbol_mismatch_between_packet_and_plan() {
    let mut position_plan_document = position_plan_document();
    position_plan_document["symbol"] = json!("600000.SH");

    let request = json!({
        "tool": "security_position_contract",
        "args": {
            "approved_open_position_packet": approved_open_position_packet_document(),
            "position_plan_document": position_plan_document
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error text should exist")
            .contains("symbol"),
        "unexpected error payload: {output}"
    );
}

// 2026-04-18 CST: Added because the merged Task 2 seed rule must cap the
// contract risk budget by the approved packet's single-trade ceiling.
// Reason: if the merged seed does not respect the approved packet cap, later
// monitoring and rebasing would start from an already invalid risk budget.
// Purpose: freeze the first packet-plus-plan risk-budget merge rule.
#[test]
fn security_position_contract_caps_risk_budget_by_packet_single_trade_limit() {
    let mut approved_open_position_packet = approved_open_position_packet_document();
    approved_open_position_packet["max_single_trade_risk_budget_pct"] = json!(0.012);

    let mut position_plan_document = position_plan_document();
    position_plan_document["risk_budget_pct"] = json!(0.018);

    let request = json!({
        "tool": "security_position_contract",
        "args": {
            "approved_open_position_packet": approved_open_position_packet,
            "position_plan_document": position_plan_document
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["position_contract"]["risk_budget_pct"],
        0.012
    );
}

// 2026-04-18 CST: Added because older position-plan documents may still omit
// an explicit persisted risk budget while the new live contract requires one.
// Reason: Task 2 introduced a backward-compatible fallback based on the plan's
// persisted risk grade, and that rule should now be locked by regression coverage.
// Purpose: freeze the Task 2 risk-budget fallback behavior for legacy-compatible seeds.
#[test]
fn security_position_contract_falls_back_to_risk_grade_when_plan_budget_is_zero() {
    let mut approved_open_position_packet = approved_open_position_packet_document();
    approved_open_position_packet["max_single_trade_risk_budget_pct"] = json!(0.03);

    let mut position_plan_document = position_plan_document();
    position_plan_document["risk_budget_pct"] = json!(0.0);
    position_plan_document["position_risk_grade"] = json!("medium");

    let request = json!({
        "tool": "security_position_contract",
        "args": {
            "approved_open_position_packet": approved_open_position_packet,
            "position_plan_document": position_plan_document
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["position_contract"]["risk_budget_pct"],
        0.006
    );
}

// 2026-04-18 CST: Added because Task 2 should test the post-open contract from
// a normalized approved packet shape instead of from raw approval fragments.
// Reason: this keeps the test aligned with the Task 1 boundary that was just frozen.
// Purpose: supply one stable approved intake document for the contract builder.
fn approved_open_position_packet_document() -> Value {
    json!({
        "document_type": "security_approved_open_position_packet",
        "contract_version": "approved_open_position_packet.v1",
        "packet_id": "packet-contract-1",
        "account_id": "acct-1",
        "approval_session_id": "approval-session-1",
        "approval_status": "approved",
        "approved_at": "2026-04-18T09:30:00+08:00",
        "effective_trade_date": "2026-04-18",
        "capital_base_amount": 100000.0,
        "intended_principal_amount": 10000.0,
        "target_annual_return_pct": 0.5,
        "max_drawdown_pct": 0.05,
        "min_cash_reserve_pct": 0.1,
        "max_single_position_pct": 0.15,
        "max_sector_exposure_pct": 0.30,
        "max_portfolio_risk_budget_pct": 0.12,
        "max_single_trade_risk_budget_pct": 0.03,
        "symbol": "601916.SH",
        "security_name": "Zheshang Bank",
        "direction": "long",
        "recommended_entry_mode": "probe",
        "recommended_starter_weight_pct": 0.03,
        "recommended_target_weight_pct": 0.08,
        "recommended_max_weight_pct": 0.12,
        "expected_annual_return_pct": 0.5,
        "expected_drawdown_pct": 0.05,
        "position_management_ready": true,
        "entry_thesis": "Approved thesis placeholder.",
        "add_condition_summary": "Add only after governance review.",
        "trim_condition_summary": "Trim when risk-adjusted edge weakens.",
        "replace_condition_summary": "Replace when a better candidate is approved.",
        "exit_condition_summary": "Exit when thesis breaks.",
        "target_achievement_condition": "Target reached.",
        "committee_resolution_ref": "committee-resolution-1",
        "chair_resolution_ref": "chair-resolution-1"
    })
}

// 2026-04-18 CST: Added because Task 2 also needs one stable pre-trade seed
// document that is clearly separate from the live contract object.
// Reason: the user explicitly required `SecurityPositionPlanDocument` to remain
// the pre-trade seed instead of being renamed into the live contract.
// Purpose: provide a minimal formal seed payload for contract formation tests.
fn position_plan_document() -> Value {
    json!({
        "position_plan_id": "position-plan-601916.SH-2026-04-18",
        "contract_version": "security_position_plan.v1",
        "document_type": "security_position_plan",
        "generated_at": "2026-04-18T09:00:00+08:00",
        "symbol": "601916.SH",
        "analysis_date": "2026-04-18",
        "analysis_date_guard": {
            "requested_as_of_date": "2026-04-18",
            "effective_analysis_date": "2026-04-18",
            "effective_trade_date": "2026-04-18",
            "local_data_last_date": "2026-04-18",
            "data_freshness_status": "ready",
            "sync_attempted": false,
            "sync_result": null,
            "date_fallback_reason": null
        },
        "evidence_version": "security_decision_briefing.v1",
        "briefing_ref": "briefing:601916.SH:2026-04-18",
        "committee_payload_ref": "committee-payload:601916.SH:2026-04-18",
        "recommended_action": "build_position",
        "confidence": "high",
        "odds_grade": "A",
        "historical_confidence": "stable",
        "confidence_grade": "high",
        "position_action": "build",
        "entry_mode": "probe",
        "starter_position_pct": 0.03,
        "max_position_pct": 0.12,
        "entry_tranche_pct": 0.03,
        "add_tranche_pct": 0.03,
        "reduce_tranche_pct": 0.02,
        "max_tranche_count": 4,
        "tranche_template": "starter_plus_adds",
        "tranche_trigger_rules": [
            "add only when confirmation persists"
        ],
        "cooldown_rule": "wait_one_review_cycle_between_adds",
        "add_on_trigger": "Add only after governance review.",
        "reduce_on_trigger": "Trim when risk-adjusted edge weakens.",
        "hard_stop_trigger": "Exit when thesis breaks.",
        "liquidity_cap": "daily_turnover_guardrail",
        "position_risk_grade": "medium",
        "regime_adjustment": "balanced",
        "execution_notes": [
            "Use staged execution."
        ],
        "rationale": [
            "Position plan rationale."
        ],
        "risk_budget_pct": 0.018
    })
}
