mod common;

use serde_json::{Value, json};

use crate::common::run_cli_with_json;

// 2026-04-20 CST: Added because the post-P12 execution preview bridge must
// first appear on the public stock tool catalog before downstream consumers can
// rely on it.
// Reason: the approved route adds one new formal preview-only tool instead of
// leaving downstream preview logic as an internal helper.
// Purpose: lock catalog visibility for the new execution preview bridge.
#[test]
fn tool_catalog_includes_security_portfolio_execution_preview() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_portfolio_execution_preview")
    );
}

// 2026-04-20 CST: Added because the first post-P12 bridge must prove that a
// governed allocation decision can flow into a side-effect-free execution preview.
// Reason: the new stage should reuse the existing P10 -> P11 -> P12 fixture chain
// instead of inventing a parallel downstream input path.
// Purpose: lock the happy-path preview output shape on the CLI surface.
#[test]
fn security_portfolio_execution_preview_builds_preview_rows_from_governed_p12_output() {
    let portfolio_allocation_decision = build_p12_document();
    let request = json!({
        "tool": "security_portfolio_execution_preview",
        "args": {
            "portfolio_allocation_decision": portfolio_allocation_decision,
            "created_at": "2026-04-20T16:00:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["portfolio_execution_preview"]["document_type"],
        "security_portfolio_execution_preview"
    );
    assert_eq!(
        output["data"]["portfolio_execution_preview"]["account_id"],
        "acct-1"
    );
    assert_eq!(
        output["data"]["portfolio_execution_preview"]["portfolio_allocation_decision_ref"],
        "portfolio-allocation-decision:acct-1:2026-04-20T00:00:00+08:00"
    );
    assert_eq!(
        output["data"]["portfolio_execution_preview"]["preview_rows"][0]["preview_action"],
        "buy"
    );
    assert_eq!(
        output["data"]["portfolio_execution_preview"]["preview_rows"][0]["execution_record_request_preview"]
            ["account_id"],
        "acct-1"
    );
    assert_eq!(
        output["data"]["portfolio_execution_preview"]["preview_rows"][0]["execution_record_request_preview"]
            ["decision_ref"],
        "portfolio-allocation-decision:acct-1:2026-04-20T00:00:00+08:00"
    );
    assert_eq!(
        output["data"]["portfolio_execution_preview"]["preview_rows"][0]["execution_record_request_preview"]
            ["execution_action"],
        "buy"
    );
    assert_eq!(
        output["data"]["portfolio_execution_preview"]["preview_rows"][0]["execution_record_request_preview"]
            ["execution_status"],
        "preview_only"
    );
    assert_eq!(
        output["data"]["portfolio_execution_preview"]["preview_rows"][0]["execution_record_request_preview"]
            ["executed_gross_pct"],
        json!(0.08)
    );
    assert_eq!(
        output["data"]["portfolio_execution_preview"]["preview_rows"][1]["preview_action"],
        "sell"
    );
    assert_eq!(
        output["data"]["portfolio_execution_preview"]["buy_count"],
        2
    );
    assert_eq!(
        output["data"]["portfolio_execution_preview"]["sell_count"],
        1
    );
    assert_eq!(
        output["data"]["portfolio_execution_preview"]["hold_count"],
        0
    );
}

// 2026-04-20 CST: Added because the preview bridge should keep zero-delta rows
// explicit instead of silently dropping them.
// Reason: downstream review needs to see that a symbol was intentionally held,
// not merely forgotten during preview expansion.
// Purpose: freeze the hold-row semantics on the CLI surface.
#[test]
fn security_portfolio_execution_preview_keeps_hold_rows_explicit() {
    let mut portfolio_allocation_decision = build_p12_document();
    portfolio_allocation_decision["final_target_allocations"][1]["target_weight_pct"] = json!(0.09);
    portfolio_allocation_decision["final_target_allocations"][1]["weight_delta_pct"] = json!(0.0);
    portfolio_allocation_decision["final_target_allocations"][2]["target_weight_pct"] = json!(0.09);
    portfolio_allocation_decision["final_target_allocations"][2]["weight_delta_pct"] = json!(0.06);

    let request = json!({
        "tool": "security_portfolio_execution_preview",
        "args": {
            "portfolio_allocation_decision": portfolio_allocation_decision,
            "created_at": "2026-04-20T16:05:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["portfolio_execution_preview"]["preview_rows"][1]["preview_action"],
        "hold"
    );
    assert_eq!(
        output["data"]["portfolio_execution_preview"]["preview_rows"][1]["execution_record_request_preview"]
            ["execution_action"],
        "hold"
    );
    assert_eq!(
        output["data"]["portfolio_execution_preview"]["preview_rows"][1]["execution_record_request_preview"]
            ["execution_status"],
        "preview_only"
    );
    assert_eq!(
        output["data"]["portfolio_execution_preview"]["hold_count"],
        1
    );
}

// 2026-04-20 CST: Added because the preview bridge must reject malformed P12
// documents instead of hiding allocation-closure drift under downstream prose.
// Reason: this bridge is a consumer of the governed P12 document, not a repair layer.
// Purpose: freeze explicit rejection of non-conserving preview input.
#[test]
fn security_portfolio_execution_preview_rejects_malformed_allocation_decision() {
    let mut portfolio_allocation_decision = build_p12_document();
    portfolio_allocation_decision["final_target_allocations"][0]["target_weight_pct"] = json!(0.50);
    portfolio_allocation_decision["final_target_allocations"][0]["weight_delta_pct"] = json!(0.42);

    let request = json!({
        "tool": "security_portfolio_execution_preview",
        "args": {
            "portfolio_allocation_decision": portfolio_allocation_decision,
            "created_at": "2026-04-20T16:10:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error text should exist")
            .contains("allocation closure"),
        "unexpected error payload: {output}"
    );
}

fn build_p12_document() -> Value {
    let (account_objective_contract, portfolio_candidate_set, portfolio_replacement_plan) =
        build_p11_documents();
    let request = json!({
        "tool": "security_portfolio_allocation_decision",
        "args": {
            "account_objective_contract": account_objective_contract,
            "portfolio_candidate_set": portfolio_candidate_set,
            "portfolio_replacement_plan": portfolio_replacement_plan,
            "created_at": "2026-04-20T00:00:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "p12 output={output}");
    output["data"]["portfolio_allocation_decision"].clone()
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
