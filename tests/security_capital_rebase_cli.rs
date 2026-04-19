mod common;

use serde_json::{Value, json};

use crate::common::run_cli_with_json;

// 2026-04-19 CST: Added because Task 6 must expose the capital rebase layer
// as a first-class stock tool before later adjustment-input bridging lands.
// Reason: capital events are independent governed account events rather than
// hidden mutations inside monitoring or execution helpers.
// Purpose: lock catalog visibility for the capital rebase tool.
#[test]
fn tool_catalog_includes_security_capital_rebase() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_capital_rebase")
    );
}

// 2026-04-19 CST: Added because add-capital events should recompute the account
// baseline and all contract target principal amounts on the governed rebase path.
// Reason: the approved Task 6 scope treats capital inflow as a first-class account event,
// not as a fake add-position action.
// Purpose: freeze the add-capital rebase baseline and target-amount behavior at the CLI surface.
#[test]
fn security_capital_rebase_add_capital_recomputes_account_baseline_and_target_amounts() {
    let request = json!({
        "tool": "security_capital_rebase",
        "args": {
            "capital_event_input": {
                "event_id": "capital-event-add-1",
                "account_id": "acct-1",
                "event_type": "add_capital",
                "event_amount": 50000.0,
                "effective_date": "2026-04-19",
                "capital_before_event": 100000.0,
                "policy_tag": "manual_funding.v1"
            },
            "monitoring_evidence_package": monitoring_evidence_package_document(),
            "position_contracts": [
                position_contract_accumulate_document(),
                position_contract_trim_document()
            ],
            "created_at": "2026-04-19T10:05:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["capital_event"]["document_type"],
        "security_capital_event"
    );
    assert_eq!(
        output["data"]["capital_event"]["capital_after_event"],
        150000.0
    );
    assert_eq!(
        output["data"]["account_rebase_snapshot"]["document_type"],
        "security_account_rebase_snapshot"
    );
    assert_eq!(
        output["data"]["account_rebase_snapshot"]["target_annual_return_pct_before"],
        0.0975
    );
    assert_eq!(
        output["data"]["account_rebase_snapshot"]["target_annual_return_pct_after"],
        0.0975
    );
    assert_eq!(
        output["data"]["account_rebase_snapshot"]["rebased_position_contracts"][1]["symbol"],
        "601916.SH"
    );
    assert_eq!(
        output["data"]["account_rebase_snapshot"]["rebased_position_contracts"][1]["capital_base_amount"],
        150000.0
    );
    assert_eq!(
        output["data"]["account_rebase_snapshot"]["rebased_position_contracts"][1]["intended_principal_amount"],
        12000.0
    );
}

// 2026-04-19 CST: Added because capital withdrawal may tighten account-level
// guardrails and should cap contract risk budgets and max weights on rebasing.
// Reason: the approved Task 6 scheme keeps target weights stable by default but
// still allows event-level constraint tightening to flow into live contracts.
// Purpose: freeze the withdraw-capital rebase caps at the CLI surface.
#[test]
fn security_capital_rebase_withdraw_capital_recomputes_risk_budget_and_max_weights() {
    let request = json!({
        "tool": "security_capital_rebase",
        "args": {
            "capital_event_input": {
                "event_id": "capital-event-withdraw-1",
                "account_id": "acct-1",
                "event_type": "withdraw_capital",
                "event_amount": 40000.0,
                "effective_date": "2026-04-19",
                "capital_before_event": 100000.0,
                "policy_tag": "cash_withdrawal.v1",
                "max_single_position_pct_after": 0.07,
                "max_single_trade_risk_budget_pct_after": 0.008
            },
            "monitoring_evidence_package": monitoring_evidence_package_document(),
            "position_contracts": [
                position_contract_accumulate_document(),
                position_contract_trim_document()
            ],
            "created_at": "2026-04-19T10:15:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["capital_event"]["capital_after_event"],
        60000.0
    );
    assert_eq!(
        output["data"]["account_rebase_snapshot"]["risk_budget_pct_before"],
        0.028
    );
    assert_eq!(
        output["data"]["account_rebase_snapshot"]["risk_budget_pct_after"],
        0.016
    );
    assert_eq!(
        output["data"]["account_rebase_snapshot"]["rebased_position_contracts"][1]["max_weight_pct"],
        0.07
    );
    assert_eq!(
        output["data"]["account_rebase_snapshot"]["rebased_position_contracts"][1]["risk_budget_pct"],
        0.008
    );
}

// 2026-04-19 CST: Added because the capital rebalance evidence package must
// remain a data-only governance handoff rather than a direct execution bridge.
// Reason: the approved post-open flow requires future committee/chair review before
// any adjustment input package can be formed from a capital event.
// Purpose: freeze the no-direct-execution boundary for Task 6.
#[test]
fn security_capital_rebase_evidence_package_does_not_directly_produce_execution_input() {
    let request = json!({
        "tool": "security_capital_rebase",
        "args": {
            "capital_event_input": {
                "event_id": "capital-event-add-2",
                "account_id": "acct-1",
                "event_type": "add_capital",
                "event_amount": 20000.0,
                "effective_date": "2026-04-19",
                "capital_before_event": 100000.0,
                "policy_tag": "manual_funding.v1"
            },
            "monitoring_evidence_package": monitoring_evidence_package_document(),
            "position_contracts": [
                position_contract_accumulate_document(),
                position_contract_trim_document()
            ],
            "created_at": "2026-04-19T10:25:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["capital_rebalance_evidence_package"]["document_type"],
        "security_capital_rebalance_evidence_package"
    );
    assert_eq!(
        output["data"]["capital_rebalance_evidence_package"]["produces_execution_input"],
        false
    );
    assert_eq!(
        output["data"]["capital_rebalance_evidence_package"]["adjustment_input_package"],
        Value::Null
    );
}

// 2026-04-19 CST: Added because Task 6 must reject unsupported capital-event
// types before any account rebase snapshot or evidence package is emitted.
// Reason: the capital rebase layer is a governed account-event boundary rather
// than a generic cash-flow parser that can infer arbitrary semantics.
// Purpose: freeze the event-type validation failure at the CLI surface.
#[test]
fn security_capital_rebase_rejects_unsupported_event_type() {
    let request = json!({
        "tool": "security_capital_rebase",
        "args": {
            "capital_event_input": {
                "event_id": "capital-event-unsupported-1",
                "account_id": "acct-1",
                "event_type": "bonus_cash",
                "event_amount": 10000.0,
                "effective_date": "2026-04-19",
                "capital_before_event": 100000.0,
                "policy_tag": "unsupported_case.v1"
            },
            "monitoring_evidence_package": monitoring_evidence_package_document(),
            "position_contracts": [
                position_contract_accumulate_document(),
                position_contract_trim_document()
            ],
            "created_at": "2026-04-19T10:35:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error text should exist")
            .contains("unsupported event_type"),
        "unexpected error payload: {output}"
    );
}

// 2026-04-19 CST: Added because Task 6 must fail fast when a governed cash-out
// would make the account capital negative after normalization.
// Reason: withdrawal normalization is the earliest safe boundary to stop invalid
// account arithmetic before any rebased contract is formed.
// Purpose: lock the negative-capital rejection path at the CLI surface.
#[test]
fn security_capital_rebase_rejects_withdrawal_that_exceeds_account_capital() {
    let request = json!({
        "tool": "security_capital_rebase",
        "args": {
            "capital_event_input": {
                "event_id": "capital-event-withdraw-oversize-1",
                "account_id": "acct-1",
                "event_type": "withdraw_capital",
                "event_amount": 120000.0,
                "effective_date": "2026-04-19",
                "capital_before_event": 100000.0,
                "policy_tag": "cash_withdrawal.v1"
            },
            "monitoring_evidence_package": monitoring_evidence_package_document(),
            "position_contracts": [
                position_contract_accumulate_document(),
                position_contract_trim_document()
            ],
            "created_at": "2026-04-19T10:40:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error text should exist")
            .contains("capital_after_event must not be negative"),
        "unexpected error payload: {output}"
    );
}

// 2026-04-19 CST: Added because dividend reinvestment is an internal capital
// inflow event and should reuse the same governed rebase arithmetic as add capital.
// Reason: Task 6 should normalize equivalent inflow semantics explicitly instead
// of forcing upstream callers to fake the event type.
// Purpose: freeze the dividend-reinvest inflow path and keep outflow warnings absent.
#[test]
fn security_capital_rebase_treats_dividend_reinvest_as_capital_inflow() {
    let request = json!({
        "tool": "security_capital_rebase",
        "args": {
            "capital_event_input": {
                "event_id": "capital-event-dividend-1",
                "account_id": "acct-1",
                "event_type": "dividend_reinvest",
                "event_amount": 8000.0,
                "effective_date": "2026-04-19",
                "capital_before_event": 100000.0,
                "policy_tag": "dividend_reinvest.v1"
            },
            "monitoring_evidence_package": monitoring_evidence_package_document(),
            "position_contracts": [
                position_contract_accumulate_document(),
                position_contract_trim_document()
            ],
            "created_at": "2026-04-19T10:45:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["capital_event"]["event_type"],
        "dividend_reinvest"
    );
    assert_eq!(
        output["data"]["capital_event"]["capital_after_event"],
        108000.0
    );
    assert!(
        !output["data"]["capital_rebalance_evidence_package"]["warnings"]
            .as_array()
            .expect("warnings should be an array")
            .iter()
            .any(|warning| warning == "capital_outflow_requires_governance_review"),
        "unexpected outflow warning in output={output}"
    );
}

// 2026-04-19 CST: Added because external cash out is an account-level outflow
// event distinct from trimming a position and should carry an explicit governance warning.
// Reason: Task 6 must keep capital events and position-adjustment events separate
// while still flagging outflows for later approval review.
// Purpose: freeze the external-cash-out normalization and warning semantics.
#[test]
fn security_capital_rebase_treats_external_cash_out_as_capital_outflow() {
    let request = json!({
        "tool": "security_capital_rebase",
        "args": {
            "capital_event_input": {
                "event_id": "capital-event-cash-out-1",
                "account_id": "acct-1",
                "event_type": "external_cash_out",
                "event_amount": 12000.0,
                "effective_date": "2026-04-19",
                "capital_before_event": 100000.0,
                "policy_tag": "cash_outflow.v1"
            },
            "monitoring_evidence_package": monitoring_evidence_package_document(),
            "position_contracts": [
                position_contract_accumulate_document(),
                position_contract_trim_document()
            ],
            "created_at": "2026-04-19T10:50:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["capital_event"]["event_type"],
        "external_cash_out"
    );
    assert_eq!(
        output["data"]["capital_event"]["capital_after_event"],
        88000.0
    );
    assert!(
        output["data"]["capital_rebalance_evidence_package"]["warnings"]
            .as_array()
            .expect("warnings should be an array")
            .iter()
            .any(|warning| warning == "capital_outflow_requires_governance_review"),
        "expected outflow warning in output={output}"
    );
}

// 2026-04-19 CST: Added because Task 6 must reject a monitoring package from
// another account before building any rebased contracts.
// Reason: the capital rebase layer is an account-scoped data boundary and cannot
// silently merge cross-account evidence into one governed snapshot.
// Purpose: freeze the monitoring/account mismatch failure at the CLI surface.
#[test]
fn security_capital_rebase_rejects_monitoring_package_account_mismatch() {
    let mut mismatched_monitoring_package = monitoring_evidence_package_document();
    mismatched_monitoring_package["account_id"] = json!("acct-other");

    let request = json!({
        "tool": "security_capital_rebase",
        "args": {
            "capital_event_input": {
                "event_id": "capital-event-add-3",
                "account_id": "acct-1",
                "event_type": "add_capital",
                "event_amount": 15000.0,
                "effective_date": "2026-04-19",
                "capital_before_event": 100000.0,
                "policy_tag": "manual_funding.v1"
            },
            "monitoring_evidence_package": mismatched_monitoring_package,
            "position_contracts": [
                position_contract_accumulate_document(),
                position_contract_trim_document()
            ],
            "created_at": "2026-04-19T10:55:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error text should exist")
            .contains("monitoring package account"),
        "unexpected error payload: {output}"
    );
}

// 2026-04-19 CST: Added because Task 6 must also reject live contracts that do
// not belong to the governed account named by the capital event.
// Reason: a rebasing pass across mixed-account contracts would corrupt account-level
// targets and risk-budget arithmetic for downstream governance review.
// Purpose: freeze the position-contract/account mismatch failure at the CLI surface.
#[test]
fn security_capital_rebase_rejects_position_contract_account_mismatch() {
    let mut mismatched_contract = position_contract_accumulate_document();
    mismatched_contract["account_id"] = json!("acct-other");

    let request = json!({
        "tool": "security_capital_rebase",
        "args": {
            "capital_event_input": {
                "event_id": "capital-event-add-4",
                "account_id": "acct-1",
                "event_type": "add_capital",
                "event_amount": 15000.0,
                "effective_date": "2026-04-19",
                "capital_before_event": 100000.0,
                "policy_tag": "manual_funding.v1"
            },
            "monitoring_evidence_package": monitoring_evidence_package_document(),
            "position_contracts": [
                mismatched_contract,
                position_contract_trim_document()
            ],
            "created_at": "2026-04-19T11:00:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error text should exist")
            .contains("does not belong to capital event account"),
        "unexpected error payload: {output}"
    );
}

fn monitoring_evidence_package_document() -> Value {
    json!({
        "monitoring_evidence_package_id": "monitoring-evidence-package:acct-1:2026-04-18T11:30:00+08:00",
        "contract_version": "security_monitoring_evidence_package.v1",
        "document_type": "security_monitoring_evidence_package",
        "generated_at": "2026-04-18T11:30:00+08:00",
        "account_id": "acct-1",
        "source_active_position_book_ref": "active-position-book:acct-1:2026-04-18T10:30:00+08:00",
        "source_evaluation_refs": [
            "per-position-evaluation:acct-1:600919.SH",
            "per-position-evaluation:acct-1:601916.SH"
        ],
        "account_aggregation": {
            "active_position_count": 2,
            "total_active_weight_pct": 0.12,
            "weighted_expected_return_pct": 0.0975,
            "weighted_expected_drawdown_pct": 0.075,
            "total_risk_budget_pct": 0.028,
            "concentration_warnings": ["single_name_over_max:600919.SH"],
            "correlation_warnings": [],
            "risk_budget_warnings": [],
            "aggregation_summary": "monitoring aggregation covers 2 active positions with 12.00% total active weight"
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
            "top_add_candidates": [
                {
                    "symbol": "601916.SH",
                    "score": 0.83,
                    "recommended_action": "add",
                    "current_weight_pct": 0.03,
                    "target_weight_pct": 0.08,
                    "current_vs_target_gap_pct": 0.05,
                    "per_position_evaluation_ref": "per-position-evaluation:acct-1:601916.SH"
                }
            ],
            "top_trim_candidates": [
                {
                    "symbol": "600919.SH",
                    "score": 0.88,
                    "recommended_action": "trim",
                    "current_weight_pct": 0.09,
                    "target_weight_pct": 0.06,
                    "current_vs_target_gap_pct": -0.03,
                    "per_position_evaluation_ref": "per-position-evaluation:acct-1:600919.SH"
                }
            ],
            "top_replace_candidates": [],
            "top_exit_candidates": []
        },
        "warnings": ["single_name_over_max:600919.SH"],
        "package_status": "ready_for_committee_review",
        "monitoring_summary": "account acct-1 monitoring package prepared with 2 live evaluations"
    })
}

fn position_contract_accumulate_document() -> Value {
    json!({
        "position_contract_id": "position-contract:acct-1:packet-contract-1",
        "contract_version": "security_position_contract.v1",
        "document_type": "security_position_contract",
        "generated_at": "2026-04-18T09:30:00+08:00",
        "packet_id": "packet-contract-1",
        "account_id": "acct-1",
        "approval_session_id": "approval-session-1",
        "symbol": "601916.SH",
        "security_name": "Zheshang Bank",
        "analysis_date": "2026-04-18",
        "effective_trade_date": "2026-04-18",
        "direction": "long",
        "contract_status": "active",
        "entry_mode": "probe",
        "initial_weight_pct": 0.03,
        "target_weight_pct": 0.08,
        "max_weight_pct": 0.12,
        "capital_base_amount": 100000.0,
        "intended_principal_amount": 8000.0,
        "expected_annual_return_pct": 0.5,
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
        "source_position_plan_ref": "position-plan-601916.SH-2026-04-18",
        "last_rebased_at": null,
        "closed_reason": null
    })
}

fn position_contract_trim_document() -> Value {
    json!({
        "position_contract_id": "position-contract:acct-1:packet-contract-2",
        "contract_version": "security_position_contract.v1",
        "document_type": "security_position_contract",
        "generated_at": "2026-04-18T09:35:00+08:00",
        "packet_id": "packet-contract-2",
        "account_id": "acct-1",
        "approval_session_id": "approval-session-2",
        "symbol": "600919.SH",
        "security_name": "Bank of Jiangsu",
        "analysis_date": "2026-04-18",
        "effective_trade_date": "2026-04-18",
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
        "source_position_plan_ref": "position-plan-600919.SH-2026-04-18",
        "last_rebased_at": null,
        "closed_reason": null
    })
}
