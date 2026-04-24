mod common;

use serde_json::{Value, json};

use crate::common::run_cli_with_json;

// 2026-04-19 CST: Added because Task 3 should first expose the unified
// portfolio replacement solver on the public stock catalog before any account-
// level replacement math can be consumed downstream.
// Reason: later P11/P12 stages should discover one formal replacement entry
// instead of reconstructing portfolio shifts from scattered helpers.
// Purpose: lock catalog visibility for the new P11 tool.
#[test]
fn tool_catalog_includes_security_portfolio_replacement_plan() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_portfolio_replacement_plan")
    );
}

// 2026-04-19 CST: Added because P11 should consume the formal P10 outputs and
// emit one governed unified replacement plan before any later allocation
// decision layer is introduced.
// Reason: the approved plan explicitly moves from account objective plus
// candidate set into one deterministic replacement contract.
// Purpose: freeze the first P11 replacement-plan output shape on the CLI surface.
#[test]
fn security_portfolio_replacement_plan_builds_unified_plan_from_p10_outputs() {
    let (account_objective_contract, portfolio_candidate_set) = build_p10_documents();

    let request = json!({
        "tool": "security_portfolio_replacement_plan",
        "args": {
            "account_objective_contract": account_objective_contract,
            "portfolio_candidate_set": portfolio_candidate_set,
            "created_at": "2026-04-19T23:50:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["portfolio_replacement_plan"]["document_type"],
        "security_portfolio_replacement_plan"
    );
    assert_eq!(
        output["data"]["portfolio_replacement_plan"]["account_id"],
        "acct-1"
    );
    assert_eq!(
        output["data"]["portfolio_replacement_plan"]["current_weights"][0]["symbol"],
        "600919.SH"
    );
    assert_eq!(
        output["data"]["portfolio_replacement_plan"]["target_weights"][2]["symbol"],
        "601916.SH"
    );
    assert_eq!(
        output["data"]["portfolio_replacement_plan"]["entry_actions"][0]["symbol"],
        "300750.SZ"
    );
    assert_eq!(
        output["data"]["portfolio_replacement_plan"]["trim_actions"][0]["symbol"],
        "600919.SH"
    );
    assert!(
        output["data"]["portfolio_replacement_plan"]["replacement_pairs"]
            .as_array()
            .expect("replacement_pairs should be an array")
            .len()
            >= 1
    );
    assert_eq!(
        output["data"]["portfolio_replacement_plan"]["capital_migration_plan"]["residual_cash_weight_pct"],
        0.81
    );
}

// 2026-04-19 CST: Added because P11 must hard-fail when the account objective
// makes the proposed candidate set impossible to fit into a governed portfolio.
// Reason: no-feasible-solution is a stage-level hard boundary in the approved plan.
// Purpose: freeze infeasible-allocation rejection on the CLI surface.
#[test]
fn security_portfolio_replacement_plan_rejects_infeasible_allocation() {
    let (mut account_objective_contract, portfolio_candidate_set) = build_p10_documents();
    account_objective_contract["risk_budget_limit"] = json!(0.01);

    let request = json!({
        "tool": "security_portfolio_replacement_plan",
        "args": {
            "account_objective_contract": account_objective_contract,
            "portfolio_candidate_set": portfolio_candidate_set,
            "created_at": "2026-04-19T23:55:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error text should exist")
            .contains("infeasible allocation"),
        "unexpected error payload: {output}"
    );
}

// 2026-04-19 CST: Added because P11 must reject malformed candidate sets that
// already violate portfolio weight closure before any replacement math begins.
// Reason: the approved plan treats weight non-conservation as a hard-fail, not
// a warning to be carried downstream.
// Purpose: freeze weight-closure rejection on the CLI surface.
#[test]
fn security_portfolio_replacement_plan_rejects_weight_non_conservation() {
    let (account_objective_contract, mut portfolio_candidate_set) = build_p10_documents();
    portfolio_candidate_set["live_positions"][0]["current_weight_pct"] = json!(0.70);
    portfolio_candidate_set["live_positions"][1]["current_weight_pct"] = json!(0.50);

    let request = json!({
        "tool": "security_portfolio_replacement_plan",
        "args": {
            "account_objective_contract": account_objective_contract,
            "portfolio_candidate_set": portfolio_candidate_set,
            "created_at": "2026-04-20T00:00:00+08:00"
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

// 2026-04-19 CST: Added because P11 should stop immediately when the formal P10
// inputs drift across accounts.
// Reason: cross-account contamination at the replacement layer would poison
// every later allocation and governance decision.
// Purpose: freeze cross-account rejection for the unified replacement plan.
#[test]
fn security_portfolio_replacement_plan_rejects_cross_account_drift() {
    let (account_objective_contract, mut portfolio_candidate_set) = build_p10_documents();
    portfolio_candidate_set["account_id"] = json!("acct-other");

    let request = json!({
        "tool": "security_portfolio_replacement_plan",
        "args": {
            "account_objective_contract": account_objective_contract,
            "portfolio_candidate_set": portfolio_candidate_set,
            "created_at": "2026-04-20T00:05:00+08:00"
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

// 2026-04-19 CST: Added because Task 4 must harden the approved-candidate-only
// boundary inside the P11 solver, not just at the top-level P10 candidate-set shell.
// Reason: later replacement math must fail if a candidate row stops representing
// an upstream-approved entrant even when the outer document still looks valid.
// Purpose: freeze row-level approved-candidate boundary rejection on the CLI surface.
#[test]
fn security_portfolio_replacement_plan_rejects_candidate_boundary_drift() {
    let (account_objective_contract, mut portfolio_candidate_set) = build_p10_documents();
    portfolio_candidate_set["approved_candidate_entries"][0]["candidate_status"] =
        json!("raw_candidate");

    let request = json!({
        "tool": "security_portfolio_replacement_plan",
        "args": {
            "account_objective_contract": account_objective_contract,
            "portfolio_candidate_set": portfolio_candidate_set,
            "created_at": "2026-04-20T00:10:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error text should exist")
            .contains("approved candidate boundary"),
        "unexpected error payload: {output}"
    );
}

// 2026-04-19 CST: Added because Task 4 should preserve capital migration
// context when a rebase snapshot is supplied to the P11 solver.
// Reason: the solver must not drop the before/after capital basis once a capital
// event has already been normalized upstream.
// Purpose: freeze rebase-aware capital migration metadata on the CLI surface.
#[test]
fn security_portfolio_replacement_plan_carries_rebase_context_into_capital_migration_plan() {
    let (account_objective_contract, portfolio_candidate_set) = build_p10_documents();

    let request = json!({
        "tool": "security_portfolio_replacement_plan",
        "args": {
            "account_objective_contract": account_objective_contract,
            "portfolio_candidate_set": portfolio_candidate_set,
            "account_rebase_snapshot": account_rebase_snapshot_document(),
            "created_at": "2026-04-20T00:15:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["portfolio_replacement_plan"]["capital_migration_plan"]["capital_base_amount_before"],
        100000.0
    );
    assert_eq!(
        output["data"]["portfolio_replacement_plan"]["capital_migration_plan"]["capital_base_amount_after"],
        150000.0
    );
    assert_eq!(
        output["data"]["portfolio_replacement_plan"]["capital_migration_plan"]["rebase_policy"],
        "proportional_rebase_on_capital_event.v1"
    );
    assert_eq!(
        output["data"]["portfolio_replacement_plan"]["capital_migration_plan"]["rebase_context_applied"],
        true
    );
}

// 2026-04-19 CST: Added because Task 4 should make the P11 conflict handling
// auditable before the final P12 decision layer exists.
// Reason: even the first deterministic solver pass should explain how it treats
// missing Kelly/volatility inputs versus hard account constraints.
// Purpose: freeze conflict-resolution summary fields on the CLI surface.
#[test]
fn security_portfolio_replacement_plan_emits_conflict_resolution_summary() {
    let (account_objective_contract, portfolio_candidate_set) = build_p10_documents();

    let request = json!({
        "tool": "security_portfolio_replacement_plan",
        "args": {
            "account_objective_contract": account_objective_contract,
            "portfolio_candidate_set": portfolio_candidate_set,
            "created_at": "2026-04-20T00:20:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["portfolio_replacement_plan"]["conflict_resolution_summary"][0],
        "kelly_signal=not_provided; fallback=target_weight_constraints"
    );
    assert_eq!(
        output["data"]["portfolio_replacement_plan"]["conflict_resolution_summary"][1],
        "volatility_target=not_provided; fallback=constraint_only_solver"
    );
    assert_eq!(
        output["data"]["portfolio_replacement_plan"]["conflict_resolution_summary"][2],
        "risk_budget_limit=applied"
    );
}

// 2026-04-19 CST: Added because Task 4 should prove the P11 solver can
// summarize one request that mixes add, replace, and exit behavior together.
// Reason: the user explicitly rejected separate rule islands in favor of one
// unified replacement engine.
// Purpose: freeze simultaneous add/replace/exit action summary on the CLI surface.
#[test]
fn security_portfolio_replacement_plan_summarizes_simultaneous_add_replace_and_exit() {
    let (account_objective_contract, mut portfolio_candidate_set) = build_p10_documents();
    portfolio_candidate_set["live_positions"][0]["target_weight_pct"] = json!(0.0);
    portfolio_candidate_set["live_positions"][1]["target_weight_pct"] = json!(0.02);

    let request = json!({
        "tool": "security_portfolio_replacement_plan",
        "args": {
            "account_objective_contract": account_objective_contract,
            "portfolio_candidate_set": portfolio_candidate_set,
            "created_at": "2026-04-20T00:25:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["portfolio_replacement_plan"]["solver_action_summary"]["entry_count"],
        1
    );
    assert_eq!(
        output["data"]["portfolio_replacement_plan"]["solver_action_summary"]["trim_count"],
        1
    );
    assert_eq!(
        output["data"]["portfolio_replacement_plan"]["solver_action_summary"]["exit_count"],
        1
    );
    assert_eq!(
        output["data"]["portfolio_replacement_plan"]["solver_action_summary"]["replacement_pair_count"],
        1
    );
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

fn account_rebase_snapshot_document() -> Value {
    serde_json::from_str(
        r#"{
            "account_rebase_snapshot_id": "account-rebase-snapshot:acct-1:2026-04-19T23:46:00+08:00",
            "contract_version": "security_account_rebase_snapshot.v1",
            "document_type": "security_account_rebase_snapshot",
            "generated_at": "2026-04-19T23:46:00+08:00",
            "account_id": "acct-1",
            "capital_event_ref": "capital-event-1",
            "source_monitoring_evidence_package_ref": "monitoring-evidence-package:acct-1:2026-04-19T23:00:00+08:00",
            "target_annual_return_pct_before": 0.0975,
            "target_annual_return_pct_after": 0.12,
            "max_drawdown_pct_before": 0.075,
            "max_drawdown_pct_after": 0.08,
            "min_cash_reserve_pct_before": 0.0,
            "min_cash_reserve_pct_after": 0.0,
            "risk_budget_pct_before": 0.028,
            "risk_budget_pct_after": 0.028,
            "rebase_policy": "proportional_rebase_on_capital_event.v1",
            "rebase_required": true,
            "rebased_position_contracts": [
                {
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
                    "contract_status": "rebasing",
                    "entry_mode": "probe",
                    "initial_weight_pct": 0.03,
                    "target_weight_pct": 0.08,
                    "max_weight_pct": 0.12,
                    "capital_base_amount": 150000.0,
                    "intended_principal_amount": 12000.0,
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
                    "last_rebased_at": "2026-04-19T23:46:00+08:00",
                    "closed_reason": null
                },
                {
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
                    "contract_status": "rebasing",
                    "entry_mode": "staged",
                    "initial_weight_pct": 0.04,
                    "target_weight_pct": 0.06,
                    "max_weight_pct": 0.08,
                    "capital_base_amount": 150000.0,
                    "intended_principal_amount": 9000.0,
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
                    "last_rebased_at": "2026-04-19T23:46:00+08:00",
                    "closed_reason": null
                }
            ],
            "rebase_completed_at": "2026-04-19T23:46:00+08:00",
            "rebase_evidence_package_ref": "capital-rebalance-evidence-package:acct-1:2026-04-19T23:46:00+08:00",
            "notes": "capital top-up completed"
        }"#,
    )
    .expect("account rebase snapshot fixture should be valid json")
}
