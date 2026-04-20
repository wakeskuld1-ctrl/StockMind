mod common;

use serde_json::{json, Value};

use crate::common::run_cli_with_json;

// 2026-04-20 CST: Added because P12 must first appear on the public stock tool
// catalog before the final portfolio-core stage can be called or audited.
// Reason: the approved design requires one discoverable governed decision tool
// instead of leaving final allocation freeze as an internal-only helper.
// Purpose: lock catalog visibility for the new P12 tool.
#[test]
fn tool_catalog_includes_security_portfolio_allocation_decision() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_portfolio_allocation_decision")
    );
}

// 2026-04-20 CST: Updated because the approved enhanced P12 route now allows a
// bounded second-pass refinement when baseline residual cash and turnover slack exist.
// Reason: the final decision layer should improve the governed baseline without
// replacing the formal P11 replacement solver.
// Purpose: lock the enhanced happy-path P12 output shape on the CLI surface.
#[test]
fn security_portfolio_allocation_decision_applies_priority_fill_when_residual_cash_and_turnover_slack_exist() {
    let (account_objective_contract, portfolio_candidate_set, portfolio_replacement_plan) =
        build_p11_documents();

    let request = json!({
        "tool": "security_portfolio_allocation_decision",
        "args": {
            "account_objective_contract": account_objective_contract,
            "portfolio_candidate_set": portfolio_candidate_set,
            "portfolio_replacement_plan": portfolio_replacement_plan,
            "created_at": "2026-04-20T10:00:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["portfolio_allocation_decision"]["document_type"],
        "security_portfolio_allocation_decision"
    );
    assert_eq!(
        output["data"]["portfolio_allocation_decision"]["refinement_applied"],
        true
    );
    assert_eq!(
        output["data"]["portfolio_allocation_decision"]["baseline_residual_cash_weight_pct"],
        0.81
    );
    assert_eq!(
        output["data"]["portfolio_allocation_decision"]["account_id"],
        "acct-1"
    );
    assert_eq!(
        output["data"]["portfolio_allocation_decision"]["final_target_allocations"][0]["symbol"],
        "300750.SZ"
    );
    assert_eq!(
        output["data"]["portfolio_allocation_decision"]["residual_cash_weight_pct"],
        0.74
    );
    assert_eq!(
        output["data"]["portfolio_allocation_decision"]["baseline_target_allocations"][0]["target_weight_pct"],
        0.05
    );
    assert_eq!(
        output["data"]["portfolio_allocation_decision"]["final_target_allocations"][0]["target_weight_pct"],
        0.08
    );
    assert_eq!(
        output["data"]["portfolio_allocation_decision"]["final_target_allocations"][2]["target_weight_pct"],
        0.12
    );
    assert_eq!(
        output["data"]["portfolio_allocation_decision"]["readiness_status"],
        "ready"
    );
    assert_eq!(
        output["data"]["portfolio_allocation_decision"]["blockers"]
            .as_array()
            .expect("blockers should be an array")
            .len(),
        0
    );
    assert_eq!(
        output["data"]["portfolio_allocation_decision"]["portfolio_replacement_plan_ref"],
        "portfolio-replacement-plan:acct-1:2026-04-19T23:50:00+08:00"
    );
    assert!(
        output["data"]["portfolio_allocation_decision"]["allocation_refinement_summary"]
            .as_array()
            .expect("allocation_refinement_summary should be an array")
            .iter()
            .any(|entry| {
                entry.as_str()
                    .expect("summary entry should be a string")
                    .contains("601916.SH")
            }),
        "expected refinement summary to mention the highest-priority symbol, output={output}"
    );
}

// 2026-04-20 CST: Added because enhanced P12 should stay bounded and must not
// invent a refinement when the baseline plan already consumes all legal turnover slack.
// Reason: the approved enhancement route is residual-cash priority fill, not an
// excuse to overrun the existing objective shell.
// Purpose: freeze the no-refinement path on the CLI surface when slack is exhausted.
#[test]
fn security_portfolio_allocation_decision_keeps_baseline_when_turnover_slack_is_exhausted() {
    let (mut account_objective_contract, portfolio_candidate_set, portfolio_replacement_plan) =
        build_p11_documents();
    account_objective_contract["turnover_limit"] = json!(0.13);

    let request = json!({
        "tool": "security_portfolio_allocation_decision",
        "args": {
            "account_objective_contract": account_objective_contract,
            "portfolio_candidate_set": portfolio_candidate_set,
            "portfolio_replacement_plan": portfolio_replacement_plan,
            "created_at": "2026-04-20T10:02:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["portfolio_allocation_decision"]["refinement_applied"],
        false
    );
    assert_eq!(
        output["data"]["portfolio_allocation_decision"]["baseline_residual_cash_weight_pct"],
        0.81
    );
    assert_eq!(
        output["data"]["portfolio_allocation_decision"]["residual_cash_weight_pct"],
        0.81
    );
    assert_eq!(
        output["data"]["portfolio_allocation_decision"]["turnover_slack_weight_pct_before_refinement"],
        0.0
    );
    assert_eq!(
        output["data"]["portfolio_allocation_decision"]["turnover_slack_weight_pct_after_refinement"],
        0.0
    );
    assert_eq!(
        output["data"]["portfolio_allocation_decision"]["baseline_target_allocations"][2]["target_weight_pct"],
        output["data"]["portfolio_allocation_decision"]["final_target_allocations"][2]["target_weight_pct"]
    );
}

// 2026-04-20 CST: Added because P12 must stop immediately when the final
// replacement document drifts away from the governed account objective.
// Reason: a cross-account final decision would invalidate every downstream
// approval or execution bridge that assumes one account-scoped allocation.
// Purpose: freeze account-drift rejection on the P12 CLI surface.
#[test]
fn security_portfolio_allocation_decision_rejects_cross_account_drift() {
    let (account_objective_contract, portfolio_candidate_set, mut portfolio_replacement_plan) =
        build_p11_documents();
    portfolio_replacement_plan["account_id"] = json!("acct-other");

    let request = json!({
        "tool": "security_portfolio_allocation_decision",
        "args": {
            "account_objective_contract": account_objective_contract,
            "portfolio_candidate_set": portfolio_candidate_set,
            "portfolio_replacement_plan": portfolio_replacement_plan,
            "created_at": "2026-04-20T10:05:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error text should exist")
            .contains("does not match request account"),
        "unexpected error payload: {output}"
    );
}

// 2026-04-20 CST: Added because P12 must reject malformed replacement-plan
// output instead of silently freezing a non-conserving final allocation.
// Reason: the final decision layer is a governance freeze, not a place to
// auto-repair broken target weights from upstream drift.
// Purpose: freeze target-weight non-conservation rejection on the CLI surface.
#[test]
fn security_portfolio_allocation_decision_rejects_weight_non_conservation() {
    let (account_objective_contract, portfolio_candidate_set, mut portfolio_replacement_plan) =
        build_p11_documents();
    portfolio_replacement_plan["target_weights"][0]["weight_pct"] = json!(0.70);

    let request = json!({
        "tool": "security_portfolio_allocation_decision",
        "args": {
            "account_objective_contract": account_objective_contract,
            "portfolio_candidate_set": portfolio_candidate_set,
            "portfolio_replacement_plan": portfolio_replacement_plan,
            "created_at": "2026-04-20T10:10:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error text should exist")
            .contains("weight non-conservation"),
        "unexpected error payload: {output}"
    );
}

// 2026-04-20 CST: Added because P12 must re-check objective limits before the
// final allocation decision is frozen as governed output.
// Reason: the approved route requires the decision layer to validate conformance,
// not blindly trust that every upstream document still matches the objective shell.
// Purpose: freeze objective-limit mismatch rejection on the CLI surface.
#[test]
fn security_portfolio_allocation_decision_rejects_objective_limit_mismatch() {
    let (mut account_objective_contract, portfolio_candidate_set, portfolio_replacement_plan) =
        build_p11_documents();
    account_objective_contract["position_count_limit"] = json!(2);

    let request = json!({
        "tool": "security_portfolio_allocation_decision",
        "args": {
            "account_objective_contract": account_objective_contract,
            "portfolio_candidate_set": portfolio_candidate_set,
            "portfolio_replacement_plan": portfolio_replacement_plan,
            "created_at": "2026-04-20T10:15:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error text should exist")
            .contains("position count"),
        "unexpected error payload: {output}"
    );
}

// 2026-04-20 CST: Added because P12 must refuse final target rows that no
// longer map back to the governed candidate universe from P10.
// Reason: allowing symbol drift here would re-open the exact raw-input bypass
// path that the portfolio-core contracts were added to prevent.
// Purpose: freeze candidate-set symbol-drift rejection on the CLI surface.
#[test]
fn security_portfolio_allocation_decision_rejects_candidate_symbol_drift() {
    let (account_objective_contract, portfolio_candidate_set, mut portfolio_replacement_plan) =
        build_p11_documents();
    portfolio_replacement_plan["target_weights"][2]["symbol"] = json!("raw-candidate");

    let request = json!({
        "tool": "security_portfolio_allocation_decision",
        "args": {
            "account_objective_contract": account_objective_contract,
            "portfolio_candidate_set": portfolio_candidate_set,
            "portfolio_replacement_plan": portfolio_replacement_plan,
            "created_at": "2026-04-20T10:20:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error text should exist")
            .contains("candidate set"),
        "unexpected error payload: {output}"
    );
}

fn build_p11_documents() -> (Value, Value, Value) {
    let (account_objective_contract, portfolio_candidate_set) = build_p10_documents();
    let request = json!({
        "tool": "security_portfolio_replacement_plan",
        "args": {
            "account_objective_contract": account_objective_contract.clone(),
            "portfolio_candidate_set": portfolio_candidate_set.clone(),
            "created_at": "2026-04-19T23:50:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "p11 output={output}");
    (
        account_objective_contract,
        portfolio_candidate_set,
        output["data"]["portfolio_replacement_plan"].clone(),
    )
}

fn build_p10_documents() -> (Value, Value) {
    let request = json!({
        "tool": "security_account_objective_contract",
        "args": {
            "active_position_book": active_position_book_document(),
            "position_contracts": [
                position_contract_accumulate_document(),
                position_contract_trim_document()
            ],
            "monitoring_evidence_package": monitoring_evidence_package_document(),
            "approved_candidates": [
                approved_candidate_document()
            ],
            "target_return_objective": 0.25,
            "max_drawdown_limit": 0.08,
            "risk_budget_limit": 0.12,
            "turnover_limit": 0.20,
            "position_count_limit": 5,
            "created_at": "2026-04-19T23:45:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "p10 output={output}");
    (
        output["data"]["account_objective_contract"].clone(),
        output["data"]["portfolio_candidate_set"].clone(),
    )
}

fn active_position_book_document() -> Value {
    json!({
        "active_position_book_id": "active-position-book:acct-1:2026-04-19T23:00:00+08:00",
        "contract_version": "security_active_position_book.v1",
        "document_type": "security_active_position_book",
        "generated_at": "2026-04-19T23:00:00+08:00",
        "account_id": "acct-1",
        "source_snapshot_ref": "account-open-position-snapshot:acct-1:2026-04-19T23:00:00+08:00",
        "active_position_count": 2,
        "active_positions": [
            {
                "symbol": "600919.SH",
                "position_state": "open",
                "current_weight_pct": 0.09,
                "price_as_of_date": "2026-04-19",
                "resolved_trade_date": "2026-04-19",
                "current_price": 6.40,
                "share_adjustment_factor": 1.0,
                "cumulative_cash_dividend_per_share": 0.04,
                "dividend_adjusted_cost_basis": 6.58,
                "holding_total_return_pct": -0.022,
                "breakeven_price": 6.55,
                "corporate_action_summary": "cash dividend absorbed",
                "sector_tag": "bank",
                "source_execution_record_ref": "record-600919.SH-open"
            },
            {
                "symbol": "601916.SH",
                "position_state": "open",
                "current_weight_pct": 0.03,
                "price_as_of_date": "2026-04-19",
                "resolved_trade_date": "2026-04-19",
                "current_price": 4.82,
                "share_adjustment_factor": 1.0,
                "cumulative_cash_dividend_per_share": 0.05,
                "dividend_adjusted_cost_basis": 4.65,
                "holding_total_return_pct": 0.0365,
                "breakeven_price": 4.60,
                "corporate_action_summary": "no material corporate action drift",
                "sector_tag": "bank",
                "source_execution_record_ref": "record-601916.SH-open"
            }
        ],
        "source_execution_record_refs": [
            "record-600919.SH-open",
            "record-601916.SH-open"
        ],
        "book_summary": "account acct-1 currently has 2 active positions ready for monitoring"
    })
}

fn position_contract_accumulate_document() -> Value {
    json!({
        "position_contract_id": "position-contract:acct-1:packet-contract-1",
        "contract_version": "security_position_contract.v1",
        "document_type": "security_position_contract",
        "generated_at": "2026-04-19T09:30:00+08:00",
        "packet_id": "packet-contract-1",
        "account_id": "acct-1",
        "approval_session_id": "approval-session-1",
        "symbol": "601916.SH",
        "security_name": "Zheshang Bank",
        "analysis_date": "2026-04-19",
        "effective_trade_date": "2026-04-19",
        "direction": "long",
        "contract_status": "active",
        "entry_mode": "probe",
        "initial_weight_pct": 0.03,
        "target_weight_pct": 0.08,
        "max_weight_pct": 0.12,
        "capital_base_amount": 100000.0,
        "intended_principal_amount": 8000.0,
        "expected_annual_return_pct": 0.50,
        "expected_drawdown_pct": 0.05,
        "risk_budget_pct": 0.018,
        "liquidity_guardrail": "daily_turnover_guardrail",
        "concentration_guardrail": "single_position_cap=15.00%; sector_cap=30.00%",
        "correlation_guardrail": null,
        "add_policy": "Add only after governance review.",
        "trim_policy": "Trim when risk-adjusted edge weakens.",
        "replace_policy": "Replace when a better candidate is approved.",
        "exit_policy": "Exit when thesis breaks.",
        "target_achievement_policy": "Target reached.",
        "rebase_policy": "proportional_rebase_on_capital_event.v1",
        "approval_binding_ref": "approval-binding:approval-session-1:committee-resolution-1:chair-resolution-1",
        "source_position_plan_ref": "position-plan-601916.SH-2026-04-19",
        "last_rebased_at": null,
        "closed_reason": null
    })
}

fn position_contract_trim_document() -> Value {
    json!({
        "position_contract_id": "position-contract:acct-1:packet-contract-2",
        "contract_version": "security_position_contract.v1",
        "document_type": "security_position_contract",
        "generated_at": "2026-04-19T09:35:00+08:00",
        "packet_id": "packet-contract-2",
        "account_id": "acct-1",
        "approval_session_id": "approval-session-2",
        "symbol": "600919.SH",
        "security_name": "Bank of Jiangsu",
        "analysis_date": "2026-04-19",
        "effective_trade_date": "2026-04-19",
        "direction": "long",
        "contract_status": "active",
        "entry_mode": "staged",
        "initial_weight_pct": 0.04,
        "target_weight_pct": 0.06,
        "max_weight_pct": 0.08,
        "capital_base_amount": 100000.0,
        "intended_principal_amount": 6000.0,
        "expected_annual_return_pct": 0.12,
        "expected_drawdown_pct": 0.07,
        "risk_budget_pct": 0.010,
        "liquidity_guardrail": "daily_turnover_guardrail",
        "concentration_guardrail": "single_position_cap=10.00%; sector_cap=30.00%",
        "correlation_guardrail": null,
        "add_policy": "Add only after governance review.",
        "trim_policy": "Trim when risk-adjusted edge weakens.",
        "replace_policy": "Replace when a better candidate is approved.",
        "exit_policy": "Exit when thesis breaks.",
        "target_achievement_policy": "Target reached.",
        "rebase_policy": "proportional_rebase_on_capital_event.v1",
        "approval_binding_ref": "approval-binding:approval-session-2:committee-resolution-2:chair-resolution-2",
        "source_position_plan_ref": "position-plan-600919.SH-2026-04-19",
        "last_rebased_at": null,
        "closed_reason": null
    })
}

fn monitoring_evidence_package_document() -> Value {
    json!({
        "monitoring_evidence_package_id": "monitoring-evidence-package:acct-1:2026-04-19T23:00:00+08:00",
        "contract_version": "security_monitoring_evidence_package.v1",
        "document_type": "security_monitoring_evidence_package",
        "generated_at": "2026-04-19T23:00:00+08:00",
        "account_id": "acct-1",
        "source_active_position_book_ref": "active-position-book:acct-1:2026-04-19T23:00:00+08:00",
        "source_evaluation_refs": [],
        "account_aggregation": {
            "active_position_count": 2,
            "total_active_weight_pct": 0.12,
            "weighted_expected_return_pct": 0.0975,
            "weighted_expected_drawdown_pct": 0.075,
            "total_risk_budget_pct": 0.028,
            "concentration_warnings": [],
            "correlation_warnings": [],
            "risk_budget_warnings": [],
            "aggregation_summary": "account acct-1 aggregation prepared"
        },
        "active_positions_summary": [
            {
                "symbol": "600919.SH",
                "current_weight_pct": 0.09,
                "current_price": 6.40,
                "holding_total_return_pct": -0.022,
                "recommended_action": "trim"
            },
            {
                "symbol": "601916.SH",
                "current_weight_pct": 0.03,
                "current_price": 4.82,
                "holding_total_return_pct": 0.0365,
                "recommended_action": "add"
            }
        ],
        "per_position_evaluations": [],
        "action_candidates": {
            "top_add_candidates": [],
            "top_trim_candidates": [],
            "top_replace_candidates": [],
            "top_exit_candidates": [
                {
                    "symbol": "600919.SH",
                    "score": 0.61,
                    "recommended_action": "exit",
                    "current_weight_pct": 0.09,
                    "target_weight_pct": 0.06,
                    "current_vs_target_gap_pct": -0.03,
                    "per_position_evaluation_ref": "evaluation-600919.SH"
                }
            ]
        },
        "warnings": [],
        "package_status": "ready_for_committee_review",
        "monitoring_summary": "account acct-1 monitoring package prepared with 0 live evaluations"
    })
}

fn approved_candidate_document() -> Value {
    json!({
        "candidate_id": "approved-candidate:acct-1:300750.SZ",
        "account_id": "acct-1",
        "symbol": "300750.SZ",
        "security_name": "CATL",
        "approval_status": "approved",
        "position_management_ready": true,
        "approved_open_position_packet_ref": "packet-300750",
        "expected_annual_return_pct": 0.42,
        "expected_drawdown_pct": 0.09,
        "target_weight_pct": 0.05,
        "max_weight_pct": 0.08,
        "risk_budget_pct": 0.014,
        "sector_tag": "battery"
    })
}
