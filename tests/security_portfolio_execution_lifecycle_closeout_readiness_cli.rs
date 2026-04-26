#![recursion_limit = "256"]

mod common;

use serde_json::{Value, json};
use std::fs;
use std::path::Path;

use crate::common::run_cli_with_json;

#[test]
fn tool_catalog_includes_security_portfolio_execution_lifecycle_closeout_readiness() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_portfolio_execution_lifecycle_closeout_readiness"),
        "tool catalog should include P20A readiness"
    );
}

#[test]
fn security_portfolio_execution_lifecycle_closeout_readiness_rejects_wrong_p19e_identity() {
    let request = readiness_request(p19e_document(vec![]).with_mutation(|document| {
        document["document_type"] = json!("security_portfolio_execution_replay_commit_writer")
    }));

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .unwrap_or_default()
            .contains("unsupported P19E document type"),
        "output={output}"
    );
}

#[test]
fn security_portfolio_execution_lifecycle_closeout_readiness_rejects_runtime_write_count() {
    let request = readiness_request(
        p19e_document(vec![]).with_mutation(|document| document["runtime_write_count"] = json!(1)),
    );

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .unwrap_or_default()
            .contains("runtime write count"),
        "output={output}"
    );
}

#[test]
fn security_portfolio_execution_lifecycle_closeout_readiness_rejects_missing_lineage() {
    let request = readiness_request(
        p19e_document(vec![]).with_mutation(|document| document["source_p19d_ref"] = json!("")),
    );

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .unwrap_or_default()
            .contains("source P19D ref"),
        "output={output}"
    );
}

#[test]
fn security_portfolio_execution_lifecycle_closeout_readiness_emits_no_candidates() {
    let request = readiness_request(p19e_document(vec![]));

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_lifecycle_closeout_readiness"];
    assert_eq!(
        document["document_type"],
        "security_portfolio_execution_lifecycle_closeout_readiness"
    );
    assert_eq!(document["readiness_status"], "no_closeout_candidates");
    assert_eq!(document["readiness_row_count"], 0);
    assert_eq!(document["runtime_write_count"], 0);
}

#[test]
fn security_portfolio_execution_lifecycle_closeout_readiness_marks_verified_rows_eligible() {
    let request = readiness_request(p19e_document(vec![
        p19e_row("601916.SH", "verified", "p20a-key-verified"),
        p19e_row(
            "601916.SH",
            "already_committed_verified",
            "p20a-key-already",
        ),
    ]));

    let output = run_cli_with_json(&request.to_string());

    // 2026-04-26 CST: Reason=P20A must turn only verified P19E rows into readiness facts.
    // Purpose=prove readiness is side-effect-free eligibility, not lifecycle closure.
    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_lifecycle_closeout_readiness"];
    assert_eq!(document["readiness_status"], "closeout_preflight_ready");
    assert_eq!(document["eligible_for_closeout_preflight_count"], 2);
    assert_eq!(
        document["readiness_rows"][0]["readiness_status"],
        "eligible_for_closeout_preflight"
    );
    assert_eq!(
        document["readiness_rows"][1]["readiness_status"],
        "eligible_for_closeout_preflight"
    );
    assert!(
        document["readiness_summary"]
            .as_str()
            .unwrap_or_default()
            .contains("not lifecycle closure")
    );
}

#[test]
fn security_portfolio_execution_lifecycle_closeout_readiness_preserves_blockers() {
    let request = readiness_request(p19e_document(vec![
        p19e_row("601916.SH", "missing_runtime_record", "p20a-key-missing"),
        p19e_row("601916.SH", "metadata_mismatch", "p20a-key-mismatch"),
        p19e_row("601916.SH", "commit_failed_preserved", "p20a-key-failed"),
        p19e_row(
            "601916.SH",
            "idempotency_conflict_confirmed",
            "p20a-key-conflict",
        ),
    ]));

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_lifecycle_closeout_readiness"];
    assert_eq!(document["readiness_status"], "blocked");
    assert_eq!(document["blocked_missing_runtime_record_count"], 1);
    assert_eq!(document["blocked_metadata_mismatch_count"], 1);
    assert_eq!(document["blocked_commit_failed_count"], 1);
    assert_eq!(document["blocked_idempotency_conflict_count"], 1);
    assert_eq!(
        document["readiness_rows"][0]["readiness_status"],
        "blocked_missing_runtime_record"
    );
    assert_eq!(
        document["readiness_rows"][1]["readiness_status"],
        "blocked_metadata_mismatch"
    );
    assert_eq!(
        document["readiness_rows"][2]["readiness_status"],
        "blocked_commit_failed"
    );
    assert_eq!(
        document["readiness_rows"][3]["readiness_status"],
        "blocked_idempotency_conflict"
    );
}

#[test]
fn security_portfolio_execution_lifecycle_closeout_readiness_reports_partial_readiness() {
    let request = readiness_request(p19e_document(vec![
        p19e_row("601916.SH", "verified", "p20a-key-ready"),
        p19e_row("601916.SH", "metadata_mismatch", "p20a-key-blocked"),
        p19e_row("601916.SH", "unknown_future_status", "p20a-key-unknown"),
    ]));

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_lifecycle_closeout_readiness"];
    assert_eq!(
        document["readiness_status"],
        "partial_closeout_preflight_ready"
    );
    assert_eq!(document["eligible_for_closeout_preflight_count"], 1);
    assert_eq!(document["blocked_unknown_audit_status_count"], 1);
    assert_eq!(
        document["readiness_rows"][2]["readiness_status"],
        "blocked_unknown_audit_status"
    );
}

#[test]
fn security_portfolio_execution_lifecycle_closeout_readiness_source_guard_is_side_effect_free() {
    let source_path = Path::new("src")
        .join("ops")
        .join("security_portfolio_execution_lifecycle_closeout_readiness.rs");
    let source = fs::read_to_string(&source_path)
        .expect("P20A readiness source should exist for source guard");

    assert!(!source.contains("security_execution_record("));
    assert!(!source.contains("security_post_trade_review("));
    assert!(!source.contains("security_closed_position_archive("));
    assert!(!source.contains("upsert_execution_record("));
    assert!(!source.contains(".execute("));
    assert!(!source.contains("open_session("));
    assert!(!source.contains("INSERT INTO security_execution_records"));
    assert!(source.contains("runtime_write_count: 0"));
    assert!(!source.contains("lifecycle closed"));
}

trait MutateValue {
    fn with_mutation(self, mutate: impl FnOnce(&mut Value)) -> Value;
}

impl MutateValue for Value {
    fn with_mutation(mut self, mutate: impl FnOnce(&mut Value)) -> Value {
        mutate(&mut self);
        self
    }
}

fn readiness_request(p19e: Value) -> Value {
    json!({
        "tool": "security_portfolio_execution_lifecycle_closeout_readiness",
        "args": {
            "portfolio_execution_replay_commit_audit": p19e,
            "created_at": "2026-04-26T13:00:00+08:00"
        }
    })
}

fn p19e_document(rows: Vec<Value>) -> Value {
    let verified_count = rows
        .iter()
        .filter(|row| row["audit_status"] == "verified")
        .count();
    let already_committed_verified_count = rows
        .iter()
        .filter(|row| row["audit_status"] == "already_committed_verified")
        .count();
    let missing_runtime_record_count = rows
        .iter()
        .filter(|row| row["audit_status"] == "missing_runtime_record")
        .count();
    let metadata_mismatch_count = rows
        .iter()
        .filter(|row| row["audit_status"] == "metadata_mismatch")
        .count();
    let idempotency_conflict_confirmed_count = rows
        .iter()
        .filter(|row| row["audit_status"] == "idempotency_conflict_confirmed")
        .count();
    let commit_failed_preserved_count = rows
        .iter()
        .filter(|row| row["audit_status"] == "commit_failed_preserved")
        .count();
    let not_auditable_count = rows
        .iter()
        .filter(|row| row["audit_status"] == "not_auditable")
        .count();

    json!({
        "portfolio_execution_replay_commit_audit_id": "portfolio-execution-replay-commit-audit:acct-1:2026-04-26T12:00:00+08:00",
        "contract_version": "security_portfolio_execution_replay_commit_audit.v1",
        "document_type": "security_portfolio_execution_replay_commit_audit",
        "generated_at": "2026-04-26T12:00:00+08:00",
        "analysis_date": "2025-09-17",
        "account_id": "acct-1",
        "source_p19d_ref": "portfolio-execution-replay-commit-writer:acct-1:2026-04-26T11:00:00+08:00",
        "source_p19c_ref": "portfolio-execution-replay-commit-preflight:acct-1:2026-04-26T10:00:00+08:00",
        "source_non_atomicity_notice": "controlled per-row writer: earlier rows may remain committed if a later row fails",
        "audit_rows": rows,
        "audit_row_count": rows.len(),
        "verified_count": verified_count,
        "already_committed_verified_count": already_committed_verified_count,
        "missing_runtime_record_count": missing_runtime_record_count,
        "metadata_mismatch_count": metadata_mismatch_count,
        "idempotency_conflict_confirmed_count": idempotency_conflict_confirmed_count,
        "commit_failed_preserved_count": commit_failed_preserved_count,
        "not_auditable_count": not_auditable_count,
        "runtime_write_count": 0,
        "audit_status": if rows.is_empty() { "no_commit_work" } else { "fixture" },
        "blockers": [],
        "audit_rationale": ["fixture"],
        "audit_summary": "fixture"
    })
}

fn p19e_row(symbol: &str, audit_status: &str, key: &str) -> Value {
    let target_ref = format!("execution-record-replay:{key}");
    json!({
        "symbol": symbol,
        "source_p19d_row_status": if matches!(audit_status, "verified" | "already_committed_verified") {
            "committed"
        } else {
            "commit_failed"
        },
        "audit_status": audit_status,
        "commit_idempotency_key": key,
        "canonical_commit_payload_hash": format!("sha256:{key}"),
        "source_p19c_ref": "portfolio-execution-replay-commit-preflight:acct-1:2026-04-26T10:00:00+08:00",
        "target_execution_record_ref": target_ref,
        "runtime_execution_record_ref": if matches!(audit_status, "verified" | "already_committed_verified") {
            json!(target_ref)
        } else {
            Value::Null
        },
        "runtime_replay_commit_idempotency_key": if matches!(audit_status, "verified" | "already_committed_verified") {
            json!(key)
        } else {
            Value::Null
        },
        "runtime_replay_commit_payload_hash": if matches!(audit_status, "verified" | "already_committed_verified") {
            json!(format!("sha256:{key}"))
        } else {
            Value::Null
        },
        "runtime_replay_commit_source_p19c_ref": if matches!(audit_status, "verified" | "already_committed_verified") {
            json!("portfolio-execution-replay-commit-preflight:acct-1:2026-04-26T10:00:00+08:00")
        } else {
            Value::Null
        },
        "runtime_record_present": matches!(audit_status, "verified" | "already_committed_verified"),
        "blockers": []
    })
}
