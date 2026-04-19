mod common;

use serde_json::{Value, json};

use crate::common::run_cli_with_json;

// 2026-04-18 CST: Added because Task 5 must expose the monitoring evidence
// package as a first-class stock tool before later governance packaging layers land.
// Reason: future committee-facing consumers should discover one formal evidence
// package entry instead of reconstructing monitoring summaries from private modules.
// Purpose: lock catalog visibility for the monitoring evidence package tool.
#[test]
fn tool_catalog_includes_security_monitoring_evidence_package() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_monitoring_evidence_package")
    );
}

// 2026-04-18 CST: Added because Task 5 should aggregate account-level expected
// payoff, downside, risk-budget usage, and action candidates from the completed
// per-position evaluation layer.
// Reason: the approved post-open design makes MonitoringEvidencePackage the
// standardized evidence handoff before any future committee review happens.
// Purpose: freeze the first account aggregation and top-candidate output on the CLI surface.
#[test]
fn security_monitoring_evidence_package_builds_account_aggregation_and_action_candidates() {
    let request = json!({
        "tool": "security_monitoring_evidence_package",
        "args": {
            "active_position_book": active_position_book_document(),
            "position_contracts": [
                position_contract_accumulate_document(),
                position_contract_trim_document()
            ],
            "per_position_evaluations": [
                per_position_evaluation_add_document(),
                per_position_evaluation_trim_document()
            ],
            "created_at": "2026-04-18T11:30:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["monitoring_evidence_package"]["document_type"],
        "security_monitoring_evidence_package"
    );
    assert_eq!(
        output["data"]["monitoring_evidence_package"]["account_aggregation"]["active_position_count"],
        2
    );
    assert_eq!(
        output["data"]["monitoring_evidence_package"]["account_aggregation"]["total_active_weight_pct"],
        0.12
    );
    assert_eq!(
        output["data"]["monitoring_evidence_package"]["account_aggregation"]["weighted_expected_return_pct"],
        0.0975
    );
    assert_eq!(
        output["data"]["monitoring_evidence_package"]["account_aggregation"]["weighted_expected_drawdown_pct"],
        0.075
    );
    assert_eq!(
        output["data"]["monitoring_evidence_package"]["account_aggregation"]["total_risk_budget_pct"],
        0.028
    );
    assert_eq!(
        output["data"]["monitoring_evidence_package"]["source_evaluation_refs"],
        json!([
            "per-position-evaluation:acct-1:600919.SH",
            "per-position-evaluation:acct-1:601916.SH"
        ])
    );
    assert_eq!(
        output["data"]["monitoring_evidence_package"]["action_candidates"]["top_add_candidates"][0]
            ["symbol"],
        "601916.SH"
    );
    assert_eq!(
        output["data"]["monitoring_evidence_package"]["action_candidates"]["top_trim_candidates"]
            [0]["symbol"],
        "600919.SH"
    );
    assert!(
        output["data"]["monitoring_evidence_package"]["warnings"]
            .as_array()
            .expect("warnings should be an array")
            .iter()
            .any(|warning| warning == "single_name_over_max:600919.SH"),
        "output={output}"
    );
}

// 2026-04-18 CST: Added because the monitoring layer must tolerate an account
// with no active holdings and still return one explicit empty evidence package.
// Reason: later committee-facing adapters should consume a governed empty package
// instead of inferring emptiness from an error or missing data structure.
// Purpose: lock the empty-account monitoring-evidence contract before Task 6 starts.
#[test]
fn security_monitoring_evidence_package_returns_empty_package_for_empty_account() {
    let request = json!({
        "tool": "security_monitoring_evidence_package",
        "args": {
            "active_position_book": empty_active_position_book_document(),
            "position_contracts": [],
            "per_position_evaluations": [],
            "created_at": "2026-04-18T11:40:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["monitoring_evidence_package"]["account_aggregation"]["active_position_count"],
        0
    );
    assert_eq!(
        output["data"]["monitoring_evidence_package"]["account_aggregation"]["total_active_weight_pct"],
        0.0
    );
    assert_eq!(
        output["data"]["monitoring_evidence_package"]["action_candidates"]["top_add_candidates"],
        json!([])
    );
    assert_eq!(
        output["data"]["monitoring_evidence_package"]["source_evaluation_refs"],
        json!([])
    );
    assert_eq!(
        output["data"]["monitoring_evidence_package"]["package_status"],
        "ready_for_committee_review"
    );
}

// 2026-04-19 CST: Added because the monitoring package must reject mixed-account
// evaluation payloads before any downstream governance consumer reads the evidence.
// Reason: Task 5 is the first standardized committee-facing handoff, so account
// identity must stay explicit and auditable.
// Purpose: freeze the evaluation/account mismatch failure at the CLI surface.
#[test]
fn security_monitoring_evidence_package_rejects_evaluation_account_mismatch() {
    let mut mismatched_evaluation = per_position_evaluation_add_document();
    mismatched_evaluation["account_id"] = json!("acct-other");

    let request = json!({
        "tool": "security_monitoring_evidence_package",
        "args": {
            "active_position_book": active_position_book_document(),
            "position_contracts": [
                position_contract_accumulate_document(),
                position_contract_trim_document()
            ],
            "per_position_evaluations": [
                mismatched_evaluation,
                per_position_evaluation_trim_document()
            ],
            "created_at": "2026-04-19T09:10:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error text should exist")
            .contains("does not belong to account"),
        "unexpected error payload: {output}"
    );
}

// 2026-04-19 CST: Added because the monitoring package must also reject live
// contracts that point at a different account than the active book.
// Reason: Task 5 should not silently assemble cross-account evidence packages from mixed refs.
// Purpose: freeze the contract/account mismatch failure at the CLI surface.
#[test]
fn security_monitoring_evidence_package_rejects_position_contract_account_mismatch() {
    let mut mismatched_contract = position_contract_accumulate_document();
    mismatched_contract["account_id"] = json!("acct-other");

    let request = json!({
        "tool": "security_monitoring_evidence_package",
        "args": {
            "active_position_book": active_position_book_document(),
            "position_contracts": [
                mismatched_contract,
                position_contract_trim_document()
            ],
            "per_position_evaluations": [
                per_position_evaluation_add_document(),
                per_position_evaluation_trim_document()
            ],
            "created_at": "2026-04-19T09:20:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error text should exist")
            .contains("does not belong to account"),
        "unexpected error payload: {output}"
    );
}

// 2026-04-19 CST: Added because Task 5 should surface explicit account-level
// pressure once the aggregated live contracts consume too much risk budget.
// Reason: later capital and adjustment tasks depend on a stable high-pressure warning
// instead of inferring it from scattered contract rows.
// Purpose: lock the risk-budget-pressure warning on the package output.
#[test]
fn security_monitoring_evidence_package_surfaces_high_risk_budget_pressure_warning() {
    let mut high_risk_contract_a = position_contract_accumulate_document();
    high_risk_contract_a["risk_budget_pct"] = json!(0.065);

    let mut high_risk_contract_b = position_contract_trim_document();
    high_risk_contract_b["risk_budget_pct"] = json!(0.055);

    let request = json!({
        "tool": "security_monitoring_evidence_package",
        "args": {
            "active_position_book": active_position_book_document(),
            "position_contracts": [
                high_risk_contract_a,
                high_risk_contract_b
            ],
            "per_position_evaluations": [
                per_position_evaluation_add_document(),
                per_position_evaluation_trim_document()
            ],
            "created_at": "2026-04-19T09:30:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["monitoring_evidence_package"]["account_aggregation"]["total_risk_budget_pct"],
        0.12
    );
    assert!(
        output["data"]["monitoring_evidence_package"]["warnings"]
            .as_array()
            .expect("warnings should be an array")
            .iter()
            .any(|warning| warning == "risk_budget_pressure_high"),
        "output={output}"
    );
}

// 2026-04-19 CST: Added because the Task 5 candidate lists should keep a stable
// descending-by-score order once multiple evaluated symbols compete in the same action bucket.
// Reason: later committee-facing consumers should not guess whether the first row is truly the best candidate.
// Purpose: freeze the ranked action-candidate ordering for add/replace/exit views.
#[test]
fn security_monitoring_evidence_package_sorts_action_candidates_by_score_desc() {
    let request = json!({
        "tool": "security_monitoring_evidence_package",
        "args": {
            "active_position_book": active_position_book_three_symbol_document(),
            "position_contracts": [
                position_contract_accumulate_document(),
                position_contract_trim_document(),
                position_contract_replace_document()
            ],
            "per_position_evaluations": [
                per_position_evaluation_add_document(),
                per_position_evaluation_trim_document(),
                per_position_evaluation_replace_document()
            ],
            "created_at": "2026-04-19T09:40:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["monitoring_evidence_package"]["action_candidates"]["top_add_candidates"][0]
            ["symbol"],
        "601916.SH"
    );
    assert_eq!(
        output["data"]["monitoring_evidence_package"]["action_candidates"]["top_add_candidates"][1]
            ["symbol"],
        "300750.SZ"
    );
    assert_eq!(
        output["data"]["monitoring_evidence_package"]["action_candidates"]["top_replace_candidates"]
            [0]["symbol"],
        "300750.SZ"
    );
    assert_eq!(
        output["data"]["monitoring_evidence_package"]["action_candidates"]["top_exit_candidates"]
            [0]["symbol"],
        "300750.SZ"
    );
}

fn active_position_book_document() -> Value {
    json!({
        "active_position_book_id": "active-position-book:acct-1:2026-04-18T10:30:00+08:00",
        "contract_version": "security_active_position_book.v1",
        "document_type": "security_active_position_book",
        "generated_at": "2026-04-18T10:30:00+08:00",
        "account_id": "acct-1",
        "source_snapshot_ref": "account-open-position-snapshot:acct-1:2026-04-18T10:30:00+08:00",
        "active_position_count": 2,
        "active_positions": [
            {
                "symbol": "601916.SH",
                "position_state": "open",
                "current_weight_pct": 0.03,
                "price_as_of_date": "2026-04-18",
                "resolved_trade_date": "2026-04-18",
                "current_price": 4.82,
                "share_adjustment_factor": 1.0,
                "cumulative_cash_dividend_per_share": 0.05,
                "dividend_adjusted_cost_basis": 4.65,
                "holding_total_return_pct": 0.0365,
                "breakeven_price": 4.60,
                "corporate_action_summary": "no material corporate action drift",
                "sector_tag": "bank",
                "source_execution_record_ref": "record-601916.SH-open"
            },
            {
                "symbol": "600919.SH",
                "position_state": "open",
                "current_weight_pct": 0.09,
                "price_as_of_date": "2026-04-18",
                "resolved_trade_date": "2026-04-18",
                "current_price": 6.40,
                "share_adjustment_factor": 1.0,
                "cumulative_cash_dividend_per_share": 0.04,
                "dividend_adjusted_cost_basis": 6.58,
                "holding_total_return_pct": -0.022,
                "breakeven_price": 6.55,
                "corporate_action_summary": "cash dividend absorbed",
                "sector_tag": "bank",
                "source_execution_record_ref": "record-600919.SH-open"
            }
        ],
        "source_execution_record_refs": [
            "record-600919.SH-open",
            "record-601916.SH-open"
        ],
        "book_summary": "account acct-1 currently has 2 active positions ready for monitoring"
    })
}

fn empty_active_position_book_document() -> Value {
    json!({
        "active_position_book_id": "active-position-book:acct-empty:2026-04-18T10:30:00+08:00",
        "contract_version": "security_active_position_book.v1",
        "document_type": "security_active_position_book",
        "generated_at": "2026-04-18T10:30:00+08:00",
        "account_id": "acct-empty",
        "source_snapshot_ref": "account-open-position-snapshot:acct-empty:2026-04-18T10:30:00+08:00",
        "active_position_count": 0,
        "active_positions": [],
        "source_execution_record_refs": [],
        "book_summary": "account acct-empty currently has 0 active positions ready for monitoring"
    })
}

fn active_position_book_three_symbol_document() -> Value {
    json!({
        "active_position_book_id": "active-position-book:acct-1:2026-04-19T09:40:00+08:00",
        "contract_version": "security_active_position_book.v1",
        "document_type": "security_active_position_book",
        "generated_at": "2026-04-19T09:40:00+08:00",
        "account_id": "acct-1",
        "source_snapshot_ref": "account-open-position-snapshot:acct-1:2026-04-19T09:40:00+08:00",
        "active_position_count": 3,
        "active_positions": [
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
            },
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
                "symbol": "300750.SZ",
                "position_state": "open",
                "current_weight_pct": 0.04,
                "price_as_of_date": "2026-04-19",
                "resolved_trade_date": "2026-04-19",
                "current_price": 205.60,
                "share_adjustment_factor": 1.0,
                "cumulative_cash_dividend_per_share": 0.0,
                "dividend_adjusted_cost_basis": 214.20,
                "holding_total_return_pct": -0.0401,
                "breakeven_price": 214.20,
                "corporate_action_summary": "no corporate action event",
                "sector_tag": "battery",
                "source_execution_record_ref": "record-300750.SZ-open"
            }
        ],
        "source_execution_record_refs": [
            "record-300750.SZ-open",
            "record-600919.SH-open",
            "record-601916.SH-open"
        ],
        "book_summary": "account acct-1 currently has 3 active positions ready for monitoring"
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

fn position_contract_replace_document() -> Value {
    json!({
        "position_contract_id": "position-contract:acct-1:packet-contract-3",
        "contract_version": "security_position_contract.v1",
        "document_type": "security_position_contract",
        "generated_at": "2026-04-19T09:05:00+08:00",
        "packet_id": "packet-contract-3",
        "account_id": "acct-1",
        "approval_session_id": "approval-session-3",
        "symbol": "300750.SZ",
        "security_name": "CATL",
        "analysis_date": "2026-04-19",
        "effective_trade_date": "2026-04-19",
        "direction": "long",
        "contract_status": "active",
        "entry_mode": "staged",
        "initial_weight_pct": 0.03,
        "target_weight_pct": 0.05,
        "max_weight_pct": 0.07,
        "capital_base_amount": 100000.0,
        "intended_principal_amount": 5000.0,
        "expected_annual_return_pct": 0.10,
        "expected_drawdown_pct": 0.11,
        "risk_budget_pct": 0.020,
        "liquidity_guardrail": "daily_turnover_guardrail",
        "concentration_guardrail": "single_position_cap=7.00%; sector_cap=20.00%",
        "correlation_guardrail": null,
        "add_policy": "Add only after governance review.",
        "trim_policy": "Trim when risk-adjusted edge weakens.",
        "replace_policy": "Replace when a better candidate is approved.",
        "exit_policy": "Exit when thesis breaks.",
        "target_achievement_policy": "Target reached.",
        "rebase_policy": "proportional_rebase_on_capital_event.v1",
        "approval_binding_ref": "approval-binding:approval-session-3:committee-resolution-3:chair-resolution-3",
        "source_position_plan_ref": "position-plan-300750.SZ-2026-04-19",
        "last_rebased_at": null,
        "closed_reason": null
    })
}

fn per_position_evaluation_add_document() -> Value {
    json!({
        "per_position_evaluation_id": "per-position-evaluation:acct-1:601916.SH",
        "contract_version": "security_per_position_evaluation.v1",
        "document_type": "security_per_position_evaluation",
        "generated_at": "2026-04-18T10:45:00+08:00",
        "account_id": "acct-1",
        "symbol": "601916.SH",
        "security_name": "Zheshang Bank",
        "analysis_date": "2026-04-18",
        "contract_status": "active",
        "position_state": "open",
        "current_weight_pct": 0.03,
        "target_weight_pct": 0.08,
        "max_weight_pct": 0.12,
        "current_vs_target_gap_pct": 0.05,
        "current_vs_max_gap_pct": 0.09,
        "updated_expected_return_pct": 0.18,
        "updated_expected_drawdown_pct": 0.06,
        "expected_payoff_ratio": 1.0,
        "action_scores": {
            "hold_score": 0.62,
            "add_score": 0.83,
            "trim_score": 0.11,
            "replace_score": 0.08,
            "exit_score": 0.05
        },
        "recommended_action": "add",
        "expectation_source": "master_scorecard_prediction_summary",
        "price_as_of_date": "2026-04-18",
        "resolved_trade_date": "2026-04-18",
        "current_price": 4.82,
        "holding_total_return_pct": 0.0365,
        "breakeven_price": 4.60,
        "sector_tag": "bank",
        "position_contract_ref": "position-contract:acct-1:packet-contract-1",
        "active_position_book_ref": "active-position-book:acct-1:2026-04-18T10:30:00+08:00",
        "source_execution_record_ref": "record-601916.SH-open",
        "master_scorecard_ref": "master-scorecard-decision-1",
        "evaluation_summary": "symbol 601916.SH scored add"
    })
}

fn per_position_evaluation_trim_document() -> Value {
    json!({
        "per_position_evaluation_id": "per-position-evaluation:acct-1:600919.SH",
        "contract_version": "security_per_position_evaluation.v1",
        "document_type": "security_per_position_evaluation",
        "generated_at": "2026-04-18T10:45:00+08:00",
        "account_id": "acct-1",
        "symbol": "600919.SH",
        "security_name": "Bank of Jiangsu",
        "analysis_date": "2026-04-18",
        "contract_status": "active",
        "position_state": "open",
        "current_weight_pct": 0.09,
        "target_weight_pct": 0.06,
        "max_weight_pct": 0.08,
        "current_vs_target_gap_pct": -0.03,
        "current_vs_max_gap_pct": -0.01,
        "updated_expected_return_pct": 0.07,
        "updated_expected_drawdown_pct": 0.08,
        "expected_payoff_ratio": 0.875,
        "action_scores": {
            "hold_score": 0.31,
            "add_score": 0.09,
            "trim_score": 0.88,
            "replace_score": 0.67,
            "exit_score": 0.61
        },
        "recommended_action": "trim",
        "expectation_source": "position_contract_fallback",
        "price_as_of_date": "2026-04-18",
        "resolved_trade_date": "2026-04-18",
        "current_price": 6.40,
        "holding_total_return_pct": -0.022,
        "breakeven_price": 6.55,
        "sector_tag": "bank",
        "position_contract_ref": "position-contract:acct-1:packet-contract-2",
        "active_position_book_ref": "active-position-book:acct-1:2026-04-18T10:30:00+08:00",
        "source_execution_record_ref": "record-600919.SH-open",
        "master_scorecard_ref": null,
        "evaluation_summary": "symbol 600919.SH scored trim"
    })
}

fn per_position_evaluation_replace_document() -> Value {
    json!({
        "per_position_evaluation_id": "per-position-evaluation:acct-1:300750.SZ",
        "contract_version": "security_per_position_evaluation.v1",
        "document_type": "security_per_position_evaluation",
        "generated_at": "2026-04-19T09:25:00+08:00",
        "account_id": "acct-1",
        "symbol": "300750.SZ",
        "security_name": "CATL",
        "analysis_date": "2026-04-19",
        "contract_status": "active",
        "position_state": "open",
        "current_weight_pct": 0.04,
        "target_weight_pct": 0.05,
        "max_weight_pct": 0.07,
        "current_vs_target_gap_pct": 0.01,
        "current_vs_max_gap_pct": 0.03,
        "updated_expected_return_pct": 0.05,
        "updated_expected_drawdown_pct": 0.12,
        "expected_payoff_ratio": 0.4167,
        "action_scores": {
            "hold_score": 0.22,
            "add_score": 0.42,
            "trim_score": 0.56,
            "replace_score": 0.93,
            "exit_score": 0.91
        },
        "recommended_action": "replace",
        "expectation_source": "position_contract_fallback",
        "price_as_of_date": "2026-04-19",
        "resolved_trade_date": "2026-04-19",
        "current_price": 205.60,
        "holding_total_return_pct": -0.0401,
        "breakeven_price": 214.20,
        "sector_tag": "battery",
        "position_contract_ref": "position-contract:acct-1:packet-contract-3",
        "active_position_book_ref": "active-position-book:acct-1:2026-04-19T09:40:00+08:00",
        "source_execution_record_ref": "record-300750.SZ-open",
        "master_scorecard_ref": null,
        "evaluation_summary": "symbol 300750.SZ scored replace"
    })
}
