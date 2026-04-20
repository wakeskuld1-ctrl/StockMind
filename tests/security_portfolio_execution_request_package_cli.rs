mod common;

use serde_json::{json, Value};

use crate::common::run_cli_with_json;

// 2026-04-20 CST: Added because the new P13 request bridge must appear on the
// public stock tool catalog before any downstream workflow can rely on it.
// Reason: the approved route adds one formal request-package stage after the
// preview bridge, not an internal-only helper.
// Purpose: lock catalog visibility for the new P13 execution request bridge.
#[test]
fn tool_catalog_includes_security_portfolio_execution_request_package() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_portfolio_execution_request_package")
    );
}

// 2026-04-20 CST: Added because P13 must prove that the governed preview
// document can flow into a formal request package without becoming real execution.
// Reason: the request bridge should continue the approved P10 -> P11 -> P12 ->
// preview mainline instead of inventing a parallel request input path.
// Purpose: lock the happy-path request-package contract on the CLI surface.
#[test]
fn security_portfolio_execution_request_package_builds_formal_requests_from_preview_output() {
    let portfolio_execution_preview = build_preview_document();
    let request = json!({
        "tool": "security_portfolio_execution_request_package",
        "args": {
            "portfolio_execution_preview": portfolio_execution_preview,
            "created_at": "2026-04-20T17:00:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["portfolio_execution_request_package"]["document_type"],
        "security_portfolio_execution_request_package"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_package"]["account_id"],
        "acct-1"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_package"]["portfolio_execution_preview_ref"],
        "portfolio-execution-preview:acct-1:2026-04-20T16:00:00+08:00"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_package"]["portfolio_allocation_decision_ref"],
        "portfolio-allocation-decision:acct-1:2026-04-20T00:00:00+08:00"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_package"]["request_rows"][0]["request_action"],
        "buy"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_package"]["request_rows"][0]["request_status"],
        "ready_request"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_package"]["request_rows"][1]["request_action"],
        "sell"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_package"]["request_rows"][1]["request_status"],
        "ready_request"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_package"]["ready_request_count"],
        3
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_package"]["hold_request_count"],
        0
    );
}

// 2026-04-20 CST: Added because P13 must keep explicit hold rows visible
// without promoting them into executable requests.
// Reason: a hold row still belongs in the request package as traceability
// evidence, but it must stay non-executable.
// Purpose: freeze hold-row semantics on the P13 CLI surface.
#[test]
fn security_portfolio_execution_request_package_keeps_hold_rows_non_executable() {
    let mut portfolio_execution_preview = build_preview_document();
    portfolio_execution_preview["preview_rows"][1]["preview_action"] = json!("hold");
    portfolio_execution_preview["preview_rows"][1]["weight_delta_pct"] = json!(0.0);
    portfolio_execution_preview["preview_rows"][1]["preview_trade_gross_pct"] = json!(0.0);
    portfolio_execution_preview["preview_rows"][1]["execution_record_request_preview"]
        ["execution_action"] = json!("hold");
    portfolio_execution_preview["preview_rows"][1]["execution_record_request_preview"]
        ["executed_gross_pct"] = json!(0.0);
    portfolio_execution_preview["sell_count"] = json!(0);
    portfolio_execution_preview["hold_count"] = json!(1);
    portfolio_execution_preview["buy_count"] = json!(2);

    let request = json!({
        "tool": "security_portfolio_execution_request_package",
        "args": {
            "portfolio_execution_preview": portfolio_execution_preview,
            "created_at": "2026-04-20T17:05:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["portfolio_execution_request_package"]["request_rows"][1]["request_action"],
        "hold"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_package"]["request_rows"][1]["request_status"],
        "non_executable_hold"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_package"]["hold_request_count"],
        1
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_package"]["ready_request_count"],
        2
    );
}

// 2026-04-20 CST: Added because P13 must reject malformed preview lineage or
// unsupported action drift instead of repairing upstream preview data.
// Reason: this bridge is a consumer of the governed preview document, not a
// fallback normalization layer.
// Purpose: freeze explicit rejection of invalid preview input.
#[test]
fn security_portfolio_execution_request_package_rejects_malformed_preview_document() {
    let mut portfolio_execution_preview = build_preview_document();
    portfolio_execution_preview["portfolio_allocation_decision_ref"] = json!("");

    let request = json!({
        "tool": "security_portfolio_execution_request_package",
        "args": {
            "portfolio_execution_preview": portfolio_execution_preview,
            "created_at": "2026-04-20T17:10:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error text should exist")
            .contains("allocation decision ref"),
        "unexpected error payload: {output}"
    );
}

// 2026-04-20 CST: Added because P13 happy-path tests need one governed preview
// document built from the same formal chain that already closes at post-P12 preview.
// Reason: reusing the chain fixture keeps the new request bridge anchored to the
// current approved mainline instead of a fabricated request-only sample.
// Purpose: derive one formal preview document for P13 tests.
fn build_preview_document() -> Value {
    let portfolio_allocation_decision = build_p12_document();
    let request = json!({
        "tool": "security_portfolio_execution_preview",
        "args": {
            "portfolio_allocation_decision": portfolio_allocation_decision,
            "created_at": "2026-04-20T16:00:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "preview output={output}");
    output["data"]["portfolio_execution_preview"].clone()
}

// 2026-04-20 CST: Added because the P13 tests still need one governed P12
// allocation decision document before the preview bridge can be exercised.
// Reason: the request bridge must remain downstream of P12 rather than consuming
// handcrafted rows that bypass portfolio-core contracts.
// Purpose: derive one formal P12 document from the existing portfolio-core fixtures.
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
