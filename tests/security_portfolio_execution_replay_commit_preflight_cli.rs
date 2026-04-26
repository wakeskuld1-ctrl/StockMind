mod common;

use std::fs;

use serde_json::{Value, json};

use crate::common::run_cli_with_json;

// 2026-04-26 CST: Added because P19C must become a public preflight boundary
// only after the approved scheme A design.
// Reason: catalog visibility is the first observable contract for the new preflight tool.
// Purpose: lock P19C discovery without implying runtime commit authority.
#[test]
fn tool_catalog_includes_security_portfolio_execution_replay_commit_preflight() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_portfolio_execution_replay_commit_preflight")
    );
}

// 2026-04-26 CST: Added because P19C must not invent commit work when P19B
// produced a valid dry-run no-work document.
// Reason: replay commit readiness must remain downstream of P19B executor rows only.
// Purpose: prove empty P19B dry-run truth produces a side-effect-free no-work preflight.
#[test]
fn security_portfolio_execution_replay_commit_preflight_emits_no_work_for_empty_executor() {
    let request = json!({
        "tool": "security_portfolio_execution_replay_commit_preflight",
        "args": {
            "portfolio_execution_replay_executor": build_replay_executor_document("no_replay_work", vec![]),
            "portfolio_execution_request_enrichment": build_enrichment_document(vec![]),
            "preflight_mode": "commit_preflight_only",
            "created_at": "2026-04-26T10:00:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_replay_commit_preflight"];
    assert_eq!(
        document["document_type"],
        "security_portfolio_execution_replay_commit_preflight"
    );
    assert_eq!(document["preflight_mode"], "commit_preflight_only");
    assert_eq!(document["preflight_status"], "no_commit_work");
    assert_eq!(document["preflight_row_count"], 0);
    assert_eq!(document["runtime_write_count"], 0);
}

// 2026-04-26 CST: Added because P19C must freeze structured commit inputs
// without writing runtime facts.
// Reason: P19D needs canonical payload and idempotency evidence before it can own writes.
// Purpose: prove one P19B row plus one matching P14 row becomes preflight-ready.
#[test]
fn security_portfolio_execution_replay_commit_preflight_builds_ready_commit_payload_preview() {
    let request = json!({
        "tool": "security_portfolio_execution_replay_commit_preflight",
        "args": {
            "portfolio_execution_replay_executor": build_replay_executor_document(
                "validated_for_dry_run",
                vec![executor_row("8306.T", "buy", 0.08, None)]
            ),
            "portfolio_execution_request_enrichment": build_enrichment_document(vec![
                enrichment_row("8306.T", "buy", 0.08, "ready_for_apply")
            ]),
            "preflight_mode": "commit_preflight_only",
            "created_at": "2026-04-26T10:05:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_replay_commit_preflight"];
    assert_eq!(document["preflight_status"], "commit_preflight_ready");
    assert_eq!(document["preflight_row_count"], 1);
    assert_eq!(document["runtime_write_count"], 0);
    let row = &document["preflight_rows"][0];
    assert_eq!(row["symbol"], "8306.T");
    assert_eq!(row["preflight_status"], "preflight_ready");
    assert_eq!(row["runtime_execution_record_ref"], Value::Null);
    assert!(
        row["commit_idempotency_key"]
            .as_str()
            .expect("commit idempotency key should be text")
            .contains("p19c|acct-1|2026-04-24|8306.T|buy|0.08")
    );
    assert!(
        row["canonical_commit_payload_hash"]
            .as_str()
            .expect("payload hash should be text")
            .starts_with("sha256:")
    );
    assert_eq!(row["commit_payload_preview"]["execution_action"], "buy");
    assert_eq!(
        row["commit_payload_preview"]["execution_status"],
        "preflight_ready"
    );
}

// 2026-04-26 CST: Added because P19C must not become an alias for runtime
// commit mode.
// Reason: commit authorization belongs to a later P19D runtime writer contract.
// Purpose: prove execution_mode-style commit authorization is rejected at P19C.
#[test]
fn security_portfolio_execution_replay_commit_preflight_rejects_commit_authorization() {
    let request = json!({
        "tool": "security_portfolio_execution_replay_commit_preflight",
        "args": {
            "portfolio_execution_replay_executor": build_replay_executor_document(
                "validated_for_dry_run",
                vec![executor_row("8306.T", "buy", 0.08, None)]
            ),
            "portfolio_execution_request_enrichment": build_enrichment_document(vec![
                enrichment_row("8306.T", "buy", 0.08, "ready_for_apply")
            ]),
            "preflight_mode": "commit",
            "created_at": "2026-04-26T10:10:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .expect("error should be text")
            .contains("unsupported preflight mode `commit`"),
        "unexpected output: {output}"
    );
}

// 2026-04-26 CST: Added because P19C may only consume P19B dry-run truth.
// Reason: changing P19B into commit mode would bypass the approved P19C/P19D split.
// Purpose: prove non-dry-run P19B inputs hard fail.
#[test]
fn security_portfolio_execution_replay_commit_preflight_rejects_non_dry_run_executor() {
    let mut executor = build_replay_executor_document(
        "validated_for_dry_run",
        vec![executor_row("8306.T", "buy", 0.08, None)],
    );
    executor["execution_mode"] = json!("commit");
    let request = json!({
        "tool": "security_portfolio_execution_replay_commit_preflight",
        "args": {
            "portfolio_execution_replay_executor": executor,
            "portfolio_execution_request_enrichment": build_enrichment_document(vec![
                enrichment_row("8306.T", "buy", 0.08, "ready_for_apply")
            ]),
            "preflight_mode": "commit_preflight_only",
            "created_at": "2026-04-26T10:15:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .expect("error should be text")
            .contains("unsupported replay executor mode `commit`"),
        "unexpected output: {output}"
    );
}

// 2026-04-26 CST: Added because the independent risk pass found P19C was not
// hard-checking the formal P19B document identity.
// Reason: structure-compatible non-P19B artifacts must not enter the commit-preflight boundary.
// Purpose: prove P19C consumes only the frozen P19B replay executor contract.
#[test]
fn security_portfolio_execution_replay_commit_preflight_rejects_wrong_p19b_identity() {
    let mut executor = build_replay_executor_document(
        "validated_for_dry_run",
        vec![executor_row("8306.T", "buy", 0.08, None)],
    );
    executor["document_type"] = json!("security_portfolio_execution_replay_request_package");
    executor["contract_version"] = json!("security_portfolio_execution_replay_executor.v0");
    let request = json!({
        "tool": "security_portfolio_execution_replay_commit_preflight",
        "args": {
            "portfolio_execution_replay_executor": executor,
            "portfolio_execution_request_enrichment": build_enrichment_document(vec![
                enrichment_row("8306.T", "buy", 0.08, "ready_for_apply")
            ]),
            "preflight_mode": "commit_preflight_only",
            "created_at": "2026-04-26T10:17:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .expect("error should be text")
            .contains("unsupported replay executor document type"),
        "unexpected output: {output}"
    );
}

// 2026-04-26 CST: Added because the independent risk pass found P19C was not
// hard-checking the formal P14 enrichment document identity.
// Reason: commit payload previews require the exact P14 enrichment contract, not a lookalike document.
// Purpose: prove P19C rejects wrong enrichment document type/version before row matching.
#[test]
fn security_portfolio_execution_replay_commit_preflight_rejects_wrong_p14_identity() {
    let mut enrichment = build_enrichment_document(vec![enrichment_row(
        "8306.T",
        "buy",
        0.08,
        "ready_for_apply",
    )]);
    enrichment["document_type"] = json!("security_portfolio_execution_request_package");
    enrichment["contract_version"] = json!("security_portfolio_execution_request_enrichment.v0");
    let request = json!({
        "tool": "security_portfolio_execution_replay_commit_preflight",
        "args": {
            "portfolio_execution_replay_executor": build_replay_executor_document(
                "validated_for_dry_run",
                vec![executor_row("8306.T", "buy", 0.08, None)]
            ),
            "portfolio_execution_request_enrichment": enrichment,
            "preflight_mode": "commit_preflight_only",
            "created_at": "2026-04-26T10:18:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .expect("error should be text")
            .contains("unsupported request enrichment document type"),
        "unexpected output: {output}"
    );
}

// 2026-04-26 CST: Added because the independent risk pass found blocked P14
// bundles could be hidden by an empty P19B executor.
// Reason: no-work preflight must not convert an upstream blocked enrichment artifact into readiness.
// Purpose: prove P19C rejects blocked enrichment bundles even when there are no executor rows.
#[test]
fn security_portfolio_execution_replay_commit_preflight_rejects_blocked_empty_enrichment_bundle() {
    let request = json!({
        "tool": "security_portfolio_execution_replay_commit_preflight",
        "args": {
            "portfolio_execution_replay_executor": build_replay_executor_document("no_replay_work", vec![]),
            "portfolio_execution_request_enrichment": build_enrichment_document(vec![
                enrichment_row("8306.T", "buy", 0.08, "blocked")
            ]),
            "preflight_mode": "commit_preflight_only",
            "created_at": "2026-04-26T10:19:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .expect("error should be text")
            .contains("unsupported request enrichment readiness status `blocked`"),
        "unexpected output: {output}"
    );
}

// 2026-04-26 CST: Added because P19C must reject runtime ref pollution from
// any upstream fixture or future accidental write.
// Reason: preflight documents may carry planned refs only, never persisted runtime refs.
// Purpose: prove non-empty runtime_execution_record_ref hard fails.
#[test]
fn security_portfolio_execution_replay_commit_preflight_rejects_runtime_refs() {
    let request = json!({
        "tool": "security_portfolio_execution_replay_commit_preflight",
        "args": {
            "portfolio_execution_replay_executor": build_replay_executor_document(
                "validated_for_dry_run",
                vec![executor_row("8306.T", "buy", 0.08, Some("execution-record-runtime"))]
            ),
            "portfolio_execution_request_enrichment": build_enrichment_document(vec![
                enrichment_row("8306.T", "buy", 0.08, "ready_for_apply")
            ]),
            "preflight_mode": "commit_preflight_only",
            "created_at": "2026-04-26T10:20:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .expect("error should be text")
            .contains("runtime execution ref is not allowed"),
        "unexpected output: {output}"
    );
}

// 2026-04-26 CST: Added because P19C must not infer missing execution-record
// context from P19B dry-run rows.
// Reason: future commit payloads need a precise P14 enriched row match.
// Purpose: prove missing P14 match hard fails.
#[test]
fn security_portfolio_execution_replay_commit_preflight_rejects_missing_enrichment_match() {
    let request = json!({
        "tool": "security_portfolio_execution_replay_commit_preflight",
        "args": {
            "portfolio_execution_replay_executor": build_replay_executor_document(
                "validated_for_dry_run",
                vec![executor_row("8306.T", "buy", 0.08, None)]
            ),
            "portfolio_execution_request_enrichment": build_enrichment_document(vec![]),
            "preflight_mode": "commit_preflight_only",
            "created_at": "2026-04-26T10:25:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .expect("error should be text")
            .contains("missing enrichment match for `8306.T`"),
        "unexpected output: {output}"
    );
}

// 2026-04-26 CST: Added because ambiguous P14 matches would make commit payload
// hashes unstable.
// Reason: P19C must freeze one canonical payload per replay row.
// Purpose: prove multiple P14 matches hard fail.
#[test]
fn security_portfolio_execution_replay_commit_preflight_rejects_ambiguous_enrichment_match() {
    let request = json!({
        "tool": "security_portfolio_execution_replay_commit_preflight",
        "args": {
            "portfolio_execution_replay_executor": build_replay_executor_document(
                "validated_for_dry_run",
                vec![executor_row("8306.T", "buy", 0.08, None)]
            ),
            "portfolio_execution_request_enrichment": build_enrichment_document(vec![
                enrichment_row("8306.T", "buy", 0.08, "ready_for_apply"),
                enrichment_row("8306.T", "buy", 0.08, "ready_for_apply")
            ]),
            "preflight_mode": "commit_preflight_only",
            "created_at": "2026-04-26T10:30:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .expect("error should be text")
            .contains("ambiguous enrichment match for `8306.T`"),
        "unexpected output: {output}"
    );
}

// 2026-04-26 CST: Added because only ready-for-apply P14 rows are structured
// enough for future commit.
// Reason: blocked or hold rows must remain outside replay commit readiness.
// Purpose: prove non-ready P14 enrichment status hard fails.
#[test]
fn security_portfolio_execution_replay_commit_preflight_rejects_non_ready_enrichment_rows() {
    let request = json!({
        "tool": "security_portfolio_execution_replay_commit_preflight",
        "args": {
            "portfolio_execution_replay_executor": build_replay_executor_document(
                "validated_for_dry_run",
                vec![executor_row("8306.T", "buy", 0.08, None)]
            ),
            "portfolio_execution_request_enrichment": build_enrichment_document(vec![
                enrichment_row("8306.T", "buy", 0.08, "non_executable_hold")
            ]),
            "preflight_mode": "commit_preflight_only",
            "created_at": "2026-04-26T10:35:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .expect("error should be text")
            .contains("unsupported enrichment status `non_executable_hold` on `8306.T`"),
        "unexpected output: {output}"
    );
}

// 2026-04-26 CST: Added because commit preflight keys are the last durable
// candidate before a future P19D ledger.
// Reason: duplicate keys would make later already-committed detection ambiguous.
// Purpose: prove duplicate commit idempotency keys hard fail.
#[test]
fn security_portfolio_execution_replay_commit_preflight_rejects_duplicate_commit_keys() {
    let request = json!({
        "tool": "security_portfolio_execution_replay_commit_preflight",
        "args": {
            "portfolio_execution_replay_executor": build_replay_executor_document(
                "validated_for_dry_run",
                vec![
                    executor_row("8306.T", "buy", 0.08, None),
                    executor_row("8306.T", "buy", 0.08, None)
                ]
            ),
            "portfolio_execution_request_enrichment": build_enrichment_document(vec![
                enrichment_row("8306.T", "buy", 0.08, "ready_for_apply")
            ]),
            "preflight_mode": "commit_preflight_only",
            "created_at": "2026-04-26T10:40:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .expect("error should be text")
            .contains("duplicate commit idempotency key"),
        "unexpected output: {output}"
    );
}

// 2026-04-26 CST: Added because the independent risk pass requires a durable
// guard against P19C drifting into the future P19D runtime writer.
// Reason: behavior tests alone may not fail if a later edit wires runtime writes behind existing outputs.
// Purpose: freeze the P19C source as preflight-only and block direct runtime-write adapters.
#[test]
fn security_portfolio_execution_replay_commit_preflight_source_stays_preflight_only() {
    let source =
        fs::read_to_string("src/ops/security_portfolio_execution_replay_commit_preflight.rs")
            .expect("read P19C source");

    for forbidden in [
        "security_execution_record(",
        "std::fs",
        "OpenOptions",
        "File::create",
        "write_all",
        "create_dir_all",
        "crate::runtime::",
    ] {
        assert!(
            !source.contains(forbidden),
            "P19C source must remain preflight-only and not contain `{forbidden}`"
        );
    }
}

fn build_replay_executor_document(dry_run_status: &str, executor_rows: Vec<Value>) -> Value {
    json!({
        "portfolio_execution_replay_executor_id": "portfolio-execution-replay-executor:acct-1:2026-04-25T14:00:00+08:00",
        "contract_version": "security_portfolio_execution_replay_executor.v1",
        "document_type": "security_portfolio_execution_replay_executor",
        "generated_at": "2026-04-25T14:00:00+08:00",
        "analysis_date": "2026-04-24",
        "account_id": "acct-1",
        "execution_mode": "dry_run",
        "portfolio_execution_replay_request_package_ref": "portfolio-execution-replay-request-package:acct-1:2026-04-25T13:00:00+08:00",
        "portfolio_execution_repair_package_ref": "portfolio-execution-repair-package:acct-1:2026-04-25T12:00:00+08:00",
        "portfolio_execution_reconciliation_bridge_ref": "portfolio-execution-reconciliation-bridge:acct-1:2026-04-25T11:00:00+08:00",
        "portfolio_execution_status_bridge_ref": "portfolio-execution-status-bridge:acct-1:2026-04-25T10:00:00+08:00",
        "portfolio_execution_apply_bridge_ref": "portfolio-execution-apply-bridge:acct-1:2026-04-25T09:30:00+08:00",
        "portfolio_execution_request_enrichment_ref": "portfolio-execution-request-enrichment:acct-1:2026-04-25T09:00:00+08:00",
        "portfolio_execution_request_package_ref": "portfolio-execution-request-package:acct-1:2026-04-25T08:55:00+08:00",
        "portfolio_execution_preview_ref": "portfolio-execution-preview:acct-1:2026-04-25T08:50:00+08:00",
        "portfolio_allocation_decision_ref": "portfolio-allocation-decision:acct-1:2026-04-25T08:45:00+08:00",
        "executor_rows": executor_rows,
        "dry_run_row_count": executor_rows.len(),
        "runtime_write_count": 0,
        "dry_run_status": dry_run_status,
        "blockers": [],
        "executor_rationale": ["fixture"],
        "executor_summary": "fixture"
    })
}

fn executor_row(
    symbol: &str,
    request_action: &str,
    requested_gross_pct: f64,
    runtime_ref: Option<&str>,
) -> Value {
    json!({
        "symbol": symbol,
        "request_action": request_action,
        "requested_gross_pct": requested_gross_pct,
        "repair_class": "governed_retry_candidate",
        "replay_request_status": "ready_for_replay_request",
        "dry_run_status": "validated_for_dry_run",
        "idempotency_key": format!("acct-1|2026-04-24|{symbol}|{request_action}|{requested_gross_pct}|portfolio-execution-replay-request-package:acct-1:2026-04-25T13:00:00+08:00|execution_record_ref:retry"),
        "planned_execution_record_ref": format!("dry-run:portfolio-execution-replay-request-package:acct-1:2026-04-25T13:00:00+08:00:{symbol}"),
        "runtime_execution_record_ref": runtime_ref,
        "replay_evidence_refs": ["execution_record_ref:retry"],
        "executor_summary": "fixture"
    })
}

fn build_enrichment_document(rows: Vec<Value>) -> Value {
    let ready_for_apply_count = rows
        .iter()
        .filter(|row| row["enrichment_status"] == "ready_for_apply")
        .count();
    let non_executable_hold_count = rows
        .iter()
        .filter(|row| row["enrichment_status"] == "non_executable_hold")
        .count();
    let blocked_enrichment_count = rows
        .iter()
        .filter(|row| row["enrichment_status"] == "blocked")
        .count();
    let readiness_status = if blocked_enrichment_count > 0 {
        "blocked"
    } else {
        "ready"
    };

    json!({
        "portfolio_execution_request_enrichment_id": "portfolio-execution-request-enrichment:acct-1:2026-04-25T09:00:00+08:00",
        "contract_version": "security_portfolio_execution_request_enrichment.v1",
        "document_type": "security_portfolio_execution_request_enrichment",
        "generated_at": "2026-04-25T09:00:00+08:00",
        "analysis_date": "2026-04-24",
        "account_id": "acct-1",
        "portfolio_execution_request_package_ref": "portfolio-execution-request-package:acct-1:2026-04-25T08:55:00+08:00",
        "portfolio_execution_preview_ref": "portfolio-execution-preview:acct-1:2026-04-25T08:50:00+08:00",
        "portfolio_allocation_decision_ref": "portfolio-allocation-decision:acct-1:2026-04-25T08:45:00+08:00",
        "enriched_request_rows": rows,
        "ready_for_apply_count": ready_for_apply_count,
        "non_executable_hold_count": non_executable_hold_count,
        "blocked_enrichment_count": blocked_enrichment_count,
        "readiness_status": readiness_status,
        "blockers": [],
        "enrichment_rationale": ["fixture"],
        "enrichment_summary": "fixture"
    })
}

fn enrichment_row(
    symbol: &str,
    request_action: &str,
    requested_gross_pct: f64,
    enrichment_status: &str,
) -> Value {
    json!({
        "symbol": symbol,
        "request_action": request_action,
        "requested_gross_pct": requested_gross_pct,
        "request_status": "ready_for_execution",
        "analysis_date": "2026-04-24",
        "decision_ref": "portfolio-allocation-decision:acct-1:2026-04-25T08:45:00+08:00",
        "execution_action": request_action,
        "execution_status": "ready_for_apply",
        "executed_gross_pct": requested_gross_pct,
        "execution_summary": "fixture",
        "enrichment_status": enrichment_status,
        "enrichment_summary": "fixture",
        "execution_apply_context": {
            "as_of_date": "2026-04-24",
            "market_symbol": "NK225.IDX",
            "sector_symbol": "BANK.JP",
            "market_profile": "global_index",
            "sector_profile": "bank",
            "market_regime": "risk_on",
            "sector_template": "bank"
        }
    })
}
