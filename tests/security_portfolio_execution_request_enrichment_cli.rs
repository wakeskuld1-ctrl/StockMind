mod common;

use serde_json::{Value, json};

use crate::common::run_cli_with_json;

// 2026-04-21 CST: Added because the new P14 enrichment bridge must appear on the
// public stock tool catalog before any downstream apply-stage work can rely on it.
// Reason: the approved route adds one formal request-enrichment stage after P13,
// not an internal-only helper.
// Purpose: lock catalog visibility for the new P14 execution request enrichment bridge.
#[test]
fn tool_catalog_includes_security_portfolio_execution_request_enrichment() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_portfolio_execution_request_enrichment")
    );
}

// 2026-04-21 CST: Added because P14 must prove that the governed P13 request
// package can advance into one richer request bundle without becoming execution fact.
// Reason: the approved route enriches request rows for a later apply bridge while
// preserving P13 lineage and explicit stage semantics.
// Purpose: lock the happy-path enrichment contract on the CLI surface.
#[test]
fn security_portfolio_execution_request_enrichment_builds_enriched_bundle_from_request_package() {
    let portfolio_execution_request_package = build_request_package_document();
    let request = json!({
        "tool": "security_portfolio_execution_request_enrichment",
        "args": {
            "portfolio_execution_request_package": portfolio_execution_request_package,
            "analysis_date": "2026-04-21",
            "created_at": "2026-04-21T10:00:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["document_type"],
        "security_portfolio_execution_request_enrichment"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["account_id"],
        "acct-1"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["portfolio_execution_request_package_ref"],
        "portfolio-execution-request-package:acct-1:2026-04-20T17:00:00+08:00"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["portfolio_execution_preview_ref"],
        "portfolio-execution-preview:acct-1:2026-04-20T16:00:00+08:00"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["portfolio_allocation_decision_ref"],
        "portfolio-allocation-decision:acct-1:2026-04-20T00:00:00+08:00"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["readiness_status"],
        "ready"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["ready_for_apply_count"],
        3
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["non_executable_hold_count"],
        0
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["blocked_enrichment_count"],
        0
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["enriched_request_rows"][0]["request_status"],
        "ready_request"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["enriched_request_rows"][0]["enrichment_status"],
        "ready_for_apply"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["enriched_request_rows"][0]["analysis_date"],
        "2026-04-21"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["enriched_request_rows"][0]["decision_ref"],
        "portfolio-allocation-decision:acct-1:2026-04-20T00:00:00+08:00"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["enriched_request_rows"][0]["execution_action"],
        "buy"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["enriched_request_rows"][0]["execution_status"],
        "ready_for_apply"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["enriched_request_rows"][0]["executed_gross_pct"],
        json!(0.08)
    );
    assert!(
        output["data"]["portfolio_execution_request_enrichment"]["enriched_request_rows"][0]["execution_summary"]
            .as_str()
            .expect("execution_summary should exist")
            .contains("ready for apply"),
        "unexpected execution summary payload: {output}"
    );

    // 2026-04-21 CST: Added because Option A for P15 requires P14 to carry one
    // explicit execution-apply context instead of forcing P15 to infer missing
    // execution-request routing fields from hidden runtime lookups.
    // Purpose: freeze the new P14 contract extension before the apply bridge lands.
    let apply_context_row = find_enriched_row_by_symbol(
        output["data"]["portfolio_execution_request_enrichment"]["enriched_request_rows"]
            .as_array()
            .expect("enriched_request_rows should be an array"),
        "601916.SH",
    );
    assert_eq!(
        apply_context_row["execution_apply_context"]["as_of_date"],
        "2026-04-21"
    );
    assert_eq!(
        apply_context_row["execution_apply_context"]["market_regime"],
        "a_share"
    );
    assert_eq!(
        apply_context_row["execution_apply_context"]["sector_template"],
        "bank"
    );
    assert_eq!(
        apply_context_row["execution_apply_context"]["market_symbol"],
        "510300.SH"
    );
    assert_eq!(
        apply_context_row["execution_apply_context"]["sector_symbol"],
        "512800.SH"
    );
}

// 2026-04-21 CST: Added because P14 must keep hold rows visible and explicitly
// non-executable instead of promoting them into apply-ready candidates.
// Reason: hold rows remain governance evidence even after request enrichment.
// Purpose: freeze hold-row semantics on the P14 CLI surface.
#[test]
fn security_portfolio_execution_request_enrichment_keeps_hold_rows_non_executable() {
    let mut portfolio_execution_request_package = build_request_package_document();
    portfolio_execution_request_package["request_rows"][1]["request_action"] = json!("hold");
    portfolio_execution_request_package["request_rows"][1]["requested_gross_pct"] = json!(0.0);
    portfolio_execution_request_package["request_rows"][1]["request_status"] =
        json!("non_executable_hold");
    portfolio_execution_request_package["ready_request_count"] = json!(2);
    portfolio_execution_request_package["hold_request_count"] = json!(1);

    let request = json!({
        "tool": "security_portfolio_execution_request_enrichment",
        "args": {
            "portfolio_execution_request_package": portfolio_execution_request_package,
            "analysis_date": "2026-04-21",
            "created_at": "2026-04-21T10:05:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["enriched_request_rows"][1]["request_status"],
        "non_executable_hold"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["enriched_request_rows"][1]["enrichment_status"],
        "non_executable_hold"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["enriched_request_rows"][1]["execution_status"],
        "non_executable_hold"
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["enriched_request_rows"][1]["executed_gross_pct"],
        json!(0.0)
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["ready_for_apply_count"],
        2
    );
    assert_eq!(
        output["data"]["portfolio_execution_request_enrichment"]["non_executable_hold_count"],
        1
    );
}

// 2026-04-21 CST: Added because P14 must reject malformed lineage refs instead
// of silently repairing broken upstream package identity.
// Reason: this bridge consumes the formal P13 package; it is not a fallback normalizer.
// Purpose: freeze explicit rejection of invalid package lineage on the CLI surface.
#[test]
fn security_portfolio_execution_request_enrichment_rejects_missing_lineage_refs() {
    let mut portfolio_execution_request_package = build_request_package_document();
    portfolio_execution_request_package["portfolio_execution_preview_ref"] = json!("");

    let request = json!({
        "tool": "security_portfolio_execution_request_enrichment",
        "args": {
            "portfolio_execution_request_package": portfolio_execution_request_package,
            "analysis_date": "2026-04-21",
            "created_at": "2026-04-21T10:10:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error text should exist")
            .contains("preview ref"),
        "unexpected error payload: {output}"
    );
}

// 2026-04-21 CST: Added because P14 must reject request-action or request-status
// drift instead of silently upgrading unsupported rows.
// Reason: enrichment is bounded to the governed P13 request semantics only.
// Purpose: freeze explicit rejection of unsupported request row semantics.
#[test]
fn security_portfolio_execution_request_enrichment_rejects_unsupported_request_status_drift() {
    let mut portfolio_execution_request_package = build_request_package_document();
    portfolio_execution_request_package["request_rows"][0]["request_status"] = json!("queued");

    let request = json!({
        "tool": "security_portfolio_execution_request_enrichment",
        "args": {
            "portfolio_execution_request_package": portfolio_execution_request_package,
            "analysis_date": "2026-04-21",
            "created_at": "2026-04-21T10:15:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error text should exist")
            .contains("request status"),
        "unexpected error payload: {output}"
    );
}

// 2026-04-21 CST: Added because P14 requires one explicit analysis date and must
// fail fast when callers omit that governance anchor.
// Reason: the first enrichment version is deterministic from P13 rows plus one required date.
// Purpose: freeze blank-analysis-date rejection on the CLI surface.
#[test]
fn security_portfolio_execution_request_enrichment_rejects_blank_analysis_date() {
    let portfolio_execution_request_package = build_request_package_document();
    let request = json!({
        "tool": "security_portfolio_execution_request_enrichment",
        "args": {
            "portfolio_execution_request_package": portfolio_execution_request_package,
            "analysis_date": "",
            "created_at": "2026-04-21T10:20:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error text should exist")
            .contains("analysis date"),
        "unexpected error payload: {output}"
    );
}

// 2026-04-21 CST: Added because P14 must reconcile row-derived counts with the
// incoming package summary instead of trusting drifted upstream metadata.
// Reason: summary mismatches are contract corruption and should block enrichment.
// Purpose: freeze hard-fail behavior for package summary-count drift.
#[test]
fn security_portfolio_execution_request_enrichment_rejects_summary_count_drift() {
    let mut portfolio_execution_request_package = build_request_package_document();
    portfolio_execution_request_package["ready_request_count"] = json!(9);

    let request = json!({
        "tool": "security_portfolio_execution_request_enrichment",
        "args": {
            "portfolio_execution_request_package": portfolio_execution_request_package,
            "analysis_date": "2026-04-21",
            "created_at": "2026-04-21T10:25:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error text should exist")
            .contains("count mismatch"),
        "unexpected error payload: {output}"
    );
}

// 2026-04-21 CST: Added because P14 tests need one formal P13 request package
// built from the same governed portfolio-core chain used by the existing mainline.
// Reason: reusing the full chain keeps the new bridge anchored to approved upstream
// contracts instead of fabricating request rows by hand.
// Purpose: derive one formal request package document for P14 tests.
fn build_request_package_document() -> Value {
    let portfolio_execution_preview = build_preview_document();
    let request = json!({
        "tool": "security_portfolio_execution_request_package",
        "args": {
            "portfolio_execution_preview": portfolio_execution_preview,
            "created_at": "2026-04-20T17:00:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "p13 output={output}");
    output["data"]["portfolio_execution_request_package"].clone()
}

// 2026-04-21 CST: Added because the new P14 apply-context assertions should
// target one stable governed symbol instead of depending on array order.
// Purpose: keep the red test deterministic while the upstream fixture chain stays reusable.
fn find_enriched_row_by_symbol<'a>(rows: &'a [Value], symbol: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["symbol"] == symbol)
        .unwrap_or_else(|| panic!("missing enriched row for symbol {symbol}"))
}

// 2026-04-21 CST: Added because the P14 tests still need one governed preview
// document before the P13 request-package bridge can be exercised.
// Reason: the enrichment bridge must remain downstream of preview and P13 rather
// than consuming a handcrafted request-only sample.
// Purpose: derive one formal preview document for P14 tests.
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

// 2026-04-21 CST: Added because the P14 tests still need one governed P12
// allocation decision document before the downstream bridges can be exercised.
// Reason: the enrichment bridge must stay downstream of P10 -> P11 -> P12.
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
