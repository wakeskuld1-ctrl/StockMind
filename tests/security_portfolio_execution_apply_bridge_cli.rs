mod common;

use chrono::{Duration, NaiveDate};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{
    create_test_runtime_db, run_cli_with_json, run_cli_with_json_and_runtime,
    run_cli_with_json_runtime_and_envs,
};

// 2026-04-21 CST: Added because P15 must appear on the public stock catalog
// before downstream apply automation can rely on it.
// Reason: the approved route lands one formal governed apply bridge, not an internal helper.
// Purpose: lock catalog visibility for the new P15 execution apply bridge.
#[test]
fn tool_catalog_includes_security_portfolio_execution_apply_bridge() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_portfolio_execution_apply_bridge")
    );
}

// 2026-04-21 CST: Added because P15 must prove that one governed P14 bundle
// can advance into runtime-backed execution records through the existing mainline.
// Reason: the approved route is a thin apply bridge, not another request-only stage.
// Purpose: freeze the happy-path apply contract on the CLI surface.
#[test]
fn security_portfolio_execution_apply_bridge_applies_ready_rows_into_execution_records() {
    let runtime_db_path = create_test_runtime_db("security_portfolio_execution_apply_bridge_ready");
    let server = prepare_security_environment(
        &runtime_db_path,
        "security_portfolio_execution_apply_bridge_ready",
    );
    let portfolio_execution_request_enrichment =
        build_enrichment_document(&runtime_db_path, &security_envs(&server));
    let request = json!({
        "tool": "security_portfolio_execution_apply_bridge",
        "args": {
            "portfolio_execution_request_enrichment": portfolio_execution_request_enrichment,
            "created_at": "2026-04-21T12:00:00+08:00"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["portfolio_execution_apply_bridge"]["document_type"],
        "security_portfolio_execution_apply_bridge"
    );
    assert_eq!(
        output["data"]["portfolio_execution_apply_bridge"]["account_id"],
        "acct-1"
    );
    // 2026-04-21 CST: Extended assertion diagnostics because the first P15 green
    // attempt reached runtime apply but did not expose the row-level failure payload.
    // Reason: root-cause debugging needs the exact apply-bridge output before changing implementation.
    // Purpose: keep the failing test evidence self-contained when the runtime path regresses again.
    assert_eq!(
        output["data"]["portfolio_execution_apply_bridge"]["apply_status"], "applied",
        "unexpected apply bridge payload: {output}"
    );
    assert_eq!(
        output["data"]["portfolio_execution_apply_bridge"]["applied_count"],
        3
    );
    assert_eq!(
        output["data"]["portfolio_execution_apply_bridge"]["failed_apply_count"],
        0
    );
    let applied_row = find_apply_row_by_symbol(
        output["data"]["portfolio_execution_apply_bridge"]["apply_rows"]
            .as_array()
            .expect("apply_rows should be an array"),
        "601916.SH",
    );
    assert_eq!(applied_row["apply_status"], "applied");
    assert!(
        applied_row["execution_record_ref"]
            .as_str()
            .expect("execution_record_ref should exist")
            .contains("execution-record-"),
        "unexpected execution_record_ref payload: {output}"
    );
    assert!(
        applied_row["execution_journal_ref"]
            .as_str()
            .expect("execution_journal_ref should exist")
            .contains("execution-journal-"),
        "unexpected execution_journal_ref payload: {output}"
    );
}

// 2026-04-21 CST: Added because P15 must keep hold rows explicit and skipped
// instead of silently turning them into execution writes.
// Reason: non-executable hold semantics remain a governed boundary even after apply lands.
// Purpose: freeze hold-skip behavior for the P15 CLI surface.
#[test]
fn security_portfolio_execution_apply_bridge_skips_hold_rows_without_runtime_write() {
    let runtime_db_path = create_test_runtime_db("security_portfolio_execution_apply_bridge_hold");
    let server = prepare_security_environment(
        &runtime_db_path,
        "security_portfolio_execution_apply_bridge_hold",
    );
    let mut portfolio_execution_request_enrichment =
        build_enrichment_document(&runtime_db_path, &security_envs(&server));
    let hold_row = find_mut_row_by_symbol(
        portfolio_execution_request_enrichment["enriched_request_rows"]
            .as_array_mut()
            .expect("enriched_request_rows should be mutable array"),
        "601916.SH",
    );
    hold_row["request_action"] = json!("hold");
    hold_row["request_status"] = json!("non_executable_hold");
    hold_row["execution_action"] = json!("hold");
    hold_row["execution_status"] = json!("non_executable_hold");
    hold_row["executed_gross_pct"] = json!(0.0);
    hold_row["enrichment_status"] = json!("non_executable_hold");
    portfolio_execution_request_enrichment["ready_for_apply_count"] = json!(2);
    portfolio_execution_request_enrichment["non_executable_hold_count"] = json!(1);

    let request = json!({
        "tool": "security_portfolio_execution_apply_bridge",
        "args": {
            "portfolio_execution_request_enrichment": portfolio_execution_request_enrichment,
            "created_at": "2026-04-21T12:05:00+08:00"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );

    assert_eq!(output["status"], "ok", "output={output}");
    // 2026-04-21 CST: Extended assertion diagnostics because the hold-path failure
    // currently collapses to a generic status mismatch without surfacing the bridge payload.
    // Reason: the shared runtime-apply bug must be diagnosed from concrete output, not guesses.
    // Purpose: preserve the exact failing payload when hold rows regress through the apply bridge.
    assert_eq!(
        output["data"]["portfolio_execution_apply_bridge"]["apply_status"],
        "applied_with_skipped_holds",
        "unexpected hold apply payload: {output}"
    );
    assert_eq!(
        output["data"]["portfolio_execution_apply_bridge"]["skipped_hold_count"],
        1
    );
    let hold_apply_row = find_apply_row_by_symbol(
        output["data"]["portfolio_execution_apply_bridge"]["apply_rows"]
            .as_array()
            .expect("apply_rows should be an array"),
        "601916.SH",
    );
    assert_eq!(
        hold_apply_row["apply_status"],
        "skipped_non_executable_hold"
    );
    assert!(hold_apply_row["execution_record_ref"].is_null());
    assert!(hold_apply_row["execution_journal_ref"].is_null());
}

// 2026-04-21 CST: Added because P15 must reject blocked bundles before the
// first runtime write instead of starting a partial apply.
// Reason: bundle-level governance rejection belongs ahead of side effects.
// Purpose: freeze blocked-bundle rejection semantics on the P15 CLI surface.
#[test]
fn security_portfolio_execution_apply_bridge_rejects_blocked_bundle_before_apply() {
    let runtime_db_path =
        create_test_runtime_db("security_portfolio_execution_apply_bridge_blocked");
    let server = prepare_security_environment(
        &runtime_db_path,
        "security_portfolio_execution_apply_bridge_blocked",
    );
    let mut portfolio_execution_request_enrichment =
        build_enrichment_document(&runtime_db_path, &security_envs(&server));
    let blocked_row = find_mut_row_by_symbol(
        portfolio_execution_request_enrichment["enriched_request_rows"]
            .as_array_mut()
            .expect("enriched_request_rows should be mutable array"),
        "601916.SH",
    );
    blocked_row["enrichment_status"] = json!("blocked");
    blocked_row["execution_status"] = json!("blocked");
    portfolio_execution_request_enrichment["ready_for_apply_count"] = json!(2);
    portfolio_execution_request_enrichment["blocked_enrichment_count"] = json!(1);
    portfolio_execution_request_enrichment["readiness_status"] = json!("blocked");
    portfolio_execution_request_enrichment["blockers"] =
        json!(["blocked request row exists in the enrichment bundle"]);

    let request = json!({
        "tool": "security_portfolio_execution_apply_bridge",
        "args": {
            "portfolio_execution_request_enrichment": portfolio_execution_request_enrichment,
            "created_at": "2026-04-21T12:10:00+08:00"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );

    assert_eq!(output["status"], "ok", "output={output}");
    assert!(
        output["data"]["portfolio_execution_apply_bridge"]["apply_status"] == "rejected",
        "unexpected rejection payload: {output}"
    );
    assert!(
        output["data"]["portfolio_execution_apply_bridge"]["blockers"][0]
            .as_str()
            .expect("blocker text should exist")
            .contains("blocked"),
        "unexpected blocker payload: {output}"
    );
    assert_eq!(execution_record_count(&runtime_db_path), 0);
}

// 2026-04-21 CST: Added because P15 must reconcile apply summary counts with
// row observations before any runtime write starts.
// Reason: summary drift is contract corruption and should hard-fail.
// Purpose: freeze count-drift rejection on the P15 CLI surface.
#[test]
fn security_portfolio_execution_apply_bridge_rejects_summary_count_drift() {
    let runtime_db_path = create_test_runtime_db("security_portfolio_execution_apply_bridge_drift");
    let server = prepare_security_environment(
        &runtime_db_path,
        "security_portfolio_execution_apply_bridge_drift",
    );
    let mut portfolio_execution_request_enrichment =
        build_enrichment_document(&runtime_db_path, &security_envs(&server));
    portfolio_execution_request_enrichment["ready_for_apply_count"] = json!(9);

    let request = json!({
        "tool": "security_portfolio_execution_apply_bridge",
        "args": {
            "portfolio_execution_request_enrichment": portfolio_execution_request_enrichment,
            "created_at": "2026-04-21T12:15:00+08:00"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );

    assert_eq!(output["status"], "ok", "output={output}");
    assert!(
        output["data"]["portfolio_execution_apply_bridge"]["apply_status"] == "rejected",
        "unexpected rejection payload: {output}"
    );
    assert!(
        output["data"]["portfolio_execution_apply_bridge"]["blockers"][0]
            .as_str()
            .expect("blocker text should exist")
            .contains("count mismatch"),
        "unexpected blocker payload: {output}"
    );
    assert_eq!(execution_record_count(&runtime_db_path), 0);
}

// 2026-04-21 CST: Added because the approved P15 route requires one deep
// bundle preflight before the first runtime-backed execution write starts.
// Reason: a malformed later row must still stop earlier ready rows from
// writing execution facts, otherwise the bridge violates the design contract.
// Purpose: prove that missing apply context is rejected before any ready row writes.
#[test]
fn security_portfolio_execution_apply_bridge_rejects_missing_as_of_date_before_first_write() {
    let runtime_db_path =
        create_test_runtime_db("security_portfolio_execution_apply_bridge_missing_as_of_date");
    let server = prepare_security_environment(
        &runtime_db_path,
        "security_portfolio_execution_apply_bridge_missing_as_of_date",
    );
    let mut portfolio_execution_request_enrichment =
        build_enrichment_document(&runtime_db_path, &security_envs(&server));
    let malformed_row = find_mut_row_by_symbol(
        portfolio_execution_request_enrichment["enriched_request_rows"]
            .as_array_mut()
            .expect("enriched_request_rows should be mutable array"),
        "300750.SZ",
    );
    malformed_row["execution_apply_context"]["as_of_date"] = json!("");

    let request = json!({
        "tool": "security_portfolio_execution_apply_bridge",
        "args": {
            "portfolio_execution_request_enrichment": portfolio_execution_request_enrichment,
            "created_at": "2026-04-21T12:20:00+08:00"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["portfolio_execution_apply_bridge"]["apply_status"],
        "rejected"
    );
    assert!(
        output["data"]["portfolio_execution_apply_bridge"]["blockers"][0]
            .as_str()
            .expect("blocker text should exist")
            .contains("as_of_date"),
        "unexpected blocker payload: {output}"
    );
    assert_eq!(
        output["data"]["portfolio_execution_apply_bridge"]["applied_count"],
        0
    );
    assert_eq!(
        output["data"]["portfolio_execution_apply_bridge"]["apply_rows"]
            .as_array()
            .expect("apply_rows should be an array")
            .len(),
        0
    );
    assert_eq!(execution_record_count(&runtime_db_path), 0);
}

// 2026-04-21 CST: Added because the approved P15 route must reject malformed
// enrichment lineage as a first-class preflight boundary.
// Reason: a missing upstream ref breaks the governed chain and must not degrade
// into a late runtime write attempt or a generic dispatcher error.
// Purpose: freeze rejected-document semantics for lineage corruption.
#[test]
fn security_portfolio_execution_apply_bridge_rejects_malformed_enrichment_lineage() {
    let runtime_db_path =
        create_test_runtime_db("security_portfolio_execution_apply_bridge_bad_lineage");
    let server = prepare_security_environment(
        &runtime_db_path,
        "security_portfolio_execution_apply_bridge_bad_lineage",
    );
    let mut portfolio_execution_request_enrichment =
        build_enrichment_document(&runtime_db_path, &security_envs(&server));
    portfolio_execution_request_enrichment["portfolio_execution_preview_ref"] = json!("");

    let request = json!({
        "tool": "security_portfolio_execution_apply_bridge",
        "args": {
            "portfolio_execution_request_enrichment": portfolio_execution_request_enrichment,
            "created_at": "2026-04-21T12:25:00+08:00"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["portfolio_execution_apply_bridge"]["apply_status"],
        "rejected"
    );
    assert!(
        output["data"]["portfolio_execution_apply_bridge"]["blockers"][0]
            .as_str()
            .expect("blocker text should exist")
            .contains("preview ref is missing"),
        "unexpected blocker payload: {output}"
    );
    assert_eq!(execution_record_count(&runtime_db_path), 0);
}

// 2026-04-21 CST: Added because the design acceptance list requires explicit
// rejection coverage for unsupported enrichment status drift.
// Reason: callers must not rely on hidden repairs when a P14 row carries a
// status outside the approved P15 status set.
// Purpose: freeze rejected-document semantics for unsupported enrichment status drift.
#[test]
fn security_portfolio_execution_apply_bridge_rejects_unsupported_enrichment_status_drift() {
    let runtime_db_path =
        create_test_runtime_db("security_portfolio_execution_apply_bridge_status_drift");
    let server = prepare_security_environment(
        &runtime_db_path,
        "security_portfolio_execution_apply_bridge_status_drift",
    );
    let mut portfolio_execution_request_enrichment =
        build_enrichment_document(&runtime_db_path, &security_envs(&server));
    let drifted_row = find_mut_row_by_symbol(
        portfolio_execution_request_enrichment["enriched_request_rows"]
            .as_array_mut()
            .expect("enriched_request_rows should be mutable array"),
        "601916.SH",
    );
    drifted_row["enrichment_status"] = json!("ready_for_manual_review");

    let request = json!({
        "tool": "security_portfolio_execution_apply_bridge",
        "args": {
            "portfolio_execution_request_enrichment": portfolio_execution_request_enrichment,
            "created_at": "2026-04-21T12:30:00+08:00"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["portfolio_execution_apply_bridge"]["apply_status"],
        "rejected"
    );
    assert!(
        output["data"]["portfolio_execution_apply_bridge"]["blockers"][0]
            .as_str()
            .expect("blocker text should exist")
            .contains("unsupported enrichment status"),
        "unexpected blocker payload: {output}"
    );
    assert_eq!(execution_record_count(&runtime_db_path), 0);
}

// 2026-04-21 CST: Added because the new P15 tests need one runtime-isolated
// P14 document built from the same governed chain as the shipped mainline.
// Purpose: keep apply tests anchored to approved upstream contracts instead of hand-built rows.
fn build_enrichment_document(runtime_db_path: &PathBuf, envs: &[(&str, String)]) -> Value {
    // 2026-04-21 CST: Expanded fixture coverage because P15 now routes into the
    // real execution_record mainline, which needs enough future rows and the exact
    // taxonomy-backed market/sector proxies to stay green.
    // Reason: the original 420-day fixture and proxy set were enough for P13/P14 but
    // not for forward-outcome windows or the 300750 sector proxy used by P15.
    // Purpose: keep the P15 CLI tests aligned with the governed execution dependencies.
    import_history_fixture(
        runtime_db_path,
        "601916.SH",
        &build_flat_history_rows(460, 5.00),
    );
    import_history_fixture(
        runtime_db_path,
        "600919.SH",
        &build_flat_history_rows(460, 6.20),
    );
    import_history_fixture(
        runtime_db_path,
        "300750.SZ",
        &build_flat_history_rows(460, 11.80),
    );
    import_history_fixture(
        runtime_db_path,
        "510300.SH",
        &build_flat_history_rows(460, 3200.0),
    );
    import_history_fixture(
        runtime_db_path,
        "512800.SH",
        &build_flat_history_rows(460, 960.0),
    );
    import_history_fixture(
        runtime_db_path,
        "159755.SZ",
        &build_flat_history_rows(460, 840.0),
    );
    import_history_fixture(
        runtime_db_path,
        "159992.SZ",
        &build_flat_history_rows(460, 840.0),
    );

    let portfolio_execution_request_package = build_request_package_document(runtime_db_path, envs);
    let request = json!({
        "tool": "security_portfolio_execution_request_enrichment",
        "args": {
            "portfolio_execution_request_package": portfolio_execution_request_package,
            "analysis_date": "2026-04-21",
            "created_at": "2026-04-21T11:50:00+08:00"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(&request.to_string(), runtime_db_path, envs);

    assert_eq!(output["status"], "ok", "p14 output={output}");
    output["data"]["portfolio_execution_request_enrichment"].clone()
}

// 2026-04-21 CST: Added because the new P15 tests still need one governed P13
// package before they can exercise the apply bridge.
// Purpose: derive one formal request package document for the apply tests.
fn build_request_package_document(runtime_db_path: &PathBuf, envs: &[(&str, String)]) -> Value {
    let portfolio_execution_preview = build_preview_document(runtime_db_path, envs);
    let request = json!({
        "tool": "security_portfolio_execution_request_package",
        "args": {
            "portfolio_execution_preview": portfolio_execution_preview,
            "created_at": "2026-04-21T11:40:00+08:00"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(&request.to_string(), runtime_db_path, envs);

    assert_eq!(output["status"], "ok", "p13 output={output}");
    output["data"]["portfolio_execution_request_package"].clone()
}

// 2026-04-21 CST: Added because the apply tests still need one governed
// execution preview document as the upstream seed for P13 and P14.
// Purpose: derive one formal preview document for the P15 tests.
fn build_preview_document(runtime_db_path: &PathBuf, envs: &[(&str, String)]) -> Value {
    let portfolio_allocation_decision = build_p12_document(runtime_db_path, envs);
    let request = json!({
        "tool": "security_portfolio_execution_preview",
        "args": {
            "portfolio_allocation_decision": portfolio_allocation_decision,
            "created_at": "2026-04-21T11:35:00+08:00"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(&request.to_string(), runtime_db_path, envs);

    assert_eq!(output["status"], "ok", "preview output={output}");
    output["data"]["portfolio_execution_preview"].clone()
}

// 2026-04-21 CST: Added because the apply tests must remain downstream of the
// existing P10 -> P11 -> P12 governed portfolio-core chain.
// Purpose: derive one formal P12 document from the shipped upstream fixtures.
fn build_p12_document(runtime_db_path: &PathBuf, envs: &[(&str, String)]) -> Value {
    let (account_objective_contract, portfolio_candidate_set, portfolio_replacement_plan) =
        build_p11_documents(runtime_db_path, envs);
    let request = json!({
        "tool": "security_portfolio_allocation_decision",
        "args": {
            "account_objective_contract": account_objective_contract,
            "portfolio_candidate_set": portfolio_candidate_set,
            "portfolio_replacement_plan": portfolio_replacement_plan,
            "created_at": "2026-04-21T11:30:00+08:00"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(&request.to_string(), runtime_db_path, envs);

    assert_eq!(output["status"], "ok", "p12 output={output}");
    output["data"]["portfolio_allocation_decision"].clone()
}

fn build_p11_documents(
    runtime_db_path: &PathBuf,
    envs: &[(&str, String)],
) -> (Value, Value, Value) {
    let (account_objective_contract, portfolio_candidate_set) =
        build_p10_documents(runtime_db_path, envs);
    let request = json!({
        "tool": "security_portfolio_replacement_plan",
        "args": {
            "account_objective_contract": account_objective_contract.clone(),
            "portfolio_candidate_set": portfolio_candidate_set.clone(),
            "created_at": "2026-04-21T11:25:00+08:00"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(&request.to_string(), runtime_db_path, envs);

    assert_eq!(output["status"], "ok", "p11 output={output}");
    (
        account_objective_contract,
        portfolio_candidate_set,
        output["data"]["portfolio_replacement_plan"].clone(),
    )
}

fn build_p10_documents(runtime_db_path: &PathBuf, envs: &[(&str, String)]) -> (Value, Value) {
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
            "created_at": "2026-04-21T11:20:00+08:00"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(&request.to_string(), runtime_db_path, envs);

    assert_eq!(output["status"], "ok", "p10 output={output}");
    (
        output["data"]["account_objective_contract"].clone(),
        output["data"]["portfolio_candidate_set"].clone(),
    )
}

fn active_position_book_document() -> Value {
    json!({
        "active_position_book_id": "active-position-book:acct-1:2026-04-21T11:00:00+08:00",
        "contract_version": "security_active_position_book.v1",
        "document_type": "security_active_position_book",
        "generated_at": "2026-04-21T11:00:00+08:00",
        "account_id": "acct-1",
        "source_snapshot_ref": "account-open-position-snapshot:acct-1:2026-04-21T11:00:00+08:00",
        "active_position_count": 2,
        "active_positions": [
            {
                "symbol": "600919.SH",
                "position_state": "open",
                "current_weight_pct": 0.09,
                "price_as_of_date": "2026-04-21",
                "resolved_trade_date": "2026-04-21",
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
                "price_as_of_date": "2026-04-21",
                "resolved_trade_date": "2026-04-21",
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
        "generated_at": "2026-04-21T09:30:00+08:00",
        "packet_id": "packet-contract-1",
        "account_id": "acct-1",
        "approval_session_id": "approval-session-1",
        "symbol": "601916.SH",
        "security_name": "Zheshang Bank",
        "analysis_date": "2026-04-21",
        "effective_trade_date": "2026-04-21",
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
        "source_position_plan_ref": "position-plan-601916.SH-2026-04-21",
        "last_rebased_at": null,
        "closed_reason": null
    })
}

fn position_contract_trim_document() -> Value {
    json!({
        "position_contract_id": "position-contract:acct-1:packet-contract-2",
        "contract_version": "security_position_contract.v1",
        "document_type": "security_position_contract",
        "generated_at": "2026-04-21T09:35:00+08:00",
        "packet_id": "packet-contract-2",
        "account_id": "acct-1",
        "approval_session_id": "approval-session-2",
        "symbol": "600919.SH",
        "security_name": "Bank of Jiangsu",
        "analysis_date": "2026-04-21",
        "effective_trade_date": "2026-04-21",
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
        "source_position_plan_ref": "position-plan-600919.SH-2026-04-21",
        "last_rebased_at": null,
        "closed_reason": null
    })
}

fn monitoring_evidence_package_document() -> Value {
    json!({
        "monitoring_evidence_package_id": "monitoring-evidence-package:acct-1:2026-04-21T11:00:00+08:00",
        "contract_version": "security_monitoring_evidence_package.v1",
        "document_type": "security_monitoring_evidence_package",
        "generated_at": "2026-04-21T11:00:00+08:00",
        "account_id": "acct-1",
        "source_active_position_book_ref": "active-position-book:acct-1:2026-04-21T11:00:00+08:00",
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

// 2026-04-21 CST: Added because the apply tests need deterministic local
// history rows for both direct execution and upstream preview contracts.
// Purpose: isolate the new P15 runtime writes from unrelated shared-state noise.
fn import_history_fixture(runtime_db_path: &PathBuf, symbol: &str, rows: &[String]) {
    let csv_path = create_stock_history_csv(symbol, rows);
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_portfolio_execution_apply_bridge_fixture"
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), runtime_db_path);
    assert_eq!(output["status"], "ok");
}

fn create_stock_history_csv(symbol: &str, rows: &[String]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_portfolio_execution_apply_bridge")
        .join(format!("{symbol}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("apply bridge fixture dir should exist");

    let csv_path = fixture_dir.join("history.csv");
    fs::write(&csv_path, rows.join("\n")).expect("apply bridge csv should be written");
    csv_path
}

fn build_flat_history_rows(day_count: usize, start_close: f64) -> Vec<String> {
    let mut rows = vec!["trade_date,open,high,low,close,adj_close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2025, 3, 1).expect("seed date should be valid");
    let mut close = start_close;

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let next_close = close + 0.02;
        rows.push(format!(
            "{},{:.2},{:.2},{:.2},{:.2},{:.2},{}",
            trade_date.format("%Y-%m-%d"),
            close,
            next_close + 0.10,
            close - 0.08,
            next_close,
            next_close,
            800_000 + offset as i64 * 500
        ));
        close = next_close;
    }

    rows
}

fn spawn_http_route_server(routes: Vec<(&str, &str, &str, &str)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test http server should bind");
    let address = format!(
        "http://{}",
        listener
            .local_addr()
            .expect("test http server should have local addr")
    );
    let route_map: std::collections::HashMap<String, (String, String, String)> = routes
        .into_iter()
        .map(|(path, status_line, body, content_type)| {
            (
                path.to_string(),
                (
                    status_line.to_string(),
                    body.to_string(),
                    content_type.to_string(),
                ),
            )
        })
        .collect();

    thread::spawn(move || {
        for _ in 0..route_map.len() + 6 {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            let mut buffer = [0_u8; 4096];
            let _ = stream.read(&mut buffer);
            let request_text = String::from_utf8_lossy(&buffer);
            let request_line = request_text.lines().next().unwrap_or_default();
            let request_path = request_line
                .split_whitespace()
                .nth(1)
                .unwrap_or("/")
                .split('?')
                .next()
                .unwrap_or("/");
            let (status_line, body, content_type) =
                route_map.get(request_path).cloned().unwrap_or_else(|| {
                    (
                        "HTTP/1.1 404 Not Found".to_string(),
                        "{\"error\":\"not found\"}".to_string(),
                        "application/json".to_string(),
                    )
                });
            let response = format!(
                "{status_line}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });

    address
}

fn prepare_security_environment(runtime_db_path: &Path, prefix: &str) -> String {
    // 2026-04-21 CST: Expanded proxy fixture coverage because the P15 execution path
    // now consumes taxonomy-resolved routing instead of only the blended preview-era proxies.
    // Reason: 300750.SZ resolves to 159755.SZ and the execution path also needs a longer
    // future window than the earlier 420-day fixture provided.
    // Purpose: keep the HTTP-backed apply tests deterministic under the real P15 routing surface.
    import_history_fixture(
        &runtime_db_path.to_path_buf(),
        "510300.SH",
        &build_flat_history_rows(460, 3200.0),
    );
    import_history_fixture(
        &runtime_db_path.to_path_buf(),
        "512800.SH",
        &build_flat_history_rows(460, 960.0),
    );
    import_history_fixture(
        &runtime_db_path.to_path_buf(),
        "159755.SZ",
        &build_flat_history_rows(460, 840.0),
    );
    import_history_fixture(
        &runtime_db_path.to_path_buf(),
        "159992.SZ",
        &build_flat_history_rows(460, 840.0),
    );

    let _ = prefix;
    spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 200 OK",
            r#"[
                {
                    "REPORT_DATE":"2025-12-31",
                    "NOTICE_DATE":"2026-03-28",
                    "TOTAL_OPERATE_INCOME":258000000000.0,
                    "YSTZ":5.20,
                    "PARENT_NETPROFIT":9500000000.0,
                    "SJLTZ":4.10,
                    "ROEJQ":11.20
                }
            ]"#,
            "application/json",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{
                "data":{
                    "list":[
                        {"notice_date":"2026-03-28","title":"2025 annual profit distribution proposal","art_code":"AN202603281010101010","columns":[{"column_name":"company_announcement"}]},
                        {"notice_date":"2026-03-20","title":"share repurchase update","art_code":"AN202603201010101011","columns":[{"column_name":"company_announcement"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ])
}

fn security_envs(server: &str) -> Vec<(&'static str, String)> {
    vec![
        (
            "EXCEL_SKILL_EASTMONEY_FINANCIAL_URL_BASE",
            format!("{server}/financials"),
        ),
        (
            "EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE",
            format!("{server}/announcements"),
        ),
        (
            "EXCEL_SKILL_OFFICIAL_FINANCIAL_URL_BASE",
            format!("{server}/financials"),
        ),
        (
            "EXCEL_SKILL_OFFICIAL_ANNOUNCEMENT_URL_BASE",
            format!("{server}/announcements"),
        ),
        (
            "EXCEL_SKILL_SINA_FINANCIAL_URL_BASE",
            format!("{server}/financials"),
        ),
        (
            "EXCEL_SKILL_SINA_ANNOUNCEMENT_URL_BASE",
            format!("{server}/announcements"),
        ),
    ]
}

// 2026-04-21 CST: Added because the P15 rejection-path tests must verify that
// deep preflight failures happen before the first execution record write.
// Reason: apply_status alone cannot prove whether a shallow implementation wrote
// an earlier ready row before rejecting a later malformed row.
// Purpose: count persisted execution records in the governed runtime store.
fn execution_record_count(runtime_db_path: &Path) -> usize {
    let execution_db_path = runtime_db_path
        .parent()
        .expect("runtime db path should have parent")
        .join("security_execution.db");
    if !execution_db_path.exists() {
        return 0;
    }

    let connection =
        Connection::open(&execution_db_path).expect("execution db should open for verification");
    connection
        .query_row("SELECT COUNT(*) FROM security_execution_records", [], |row| {
            row.get::<_, usize>(0)
        })
        .expect("execution record count should load")
}

fn find_apply_row_by_symbol<'a>(rows: &'a [Value], symbol: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["symbol"] == symbol)
        .unwrap_or_else(|| panic!("missing apply row for symbol {symbol}"))
}

fn find_mut_row_by_symbol<'a>(rows: &'a mut [Value], symbol: &str) -> &'a mut Value {
    rows.iter_mut()
        .find(|row| row["symbol"] == symbol)
        .unwrap_or_else(|| panic!("missing mutable row for symbol {symbol}"))
}
