#![recursion_limit = "256"]

mod common;

use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{create_test_runtime_db, run_cli_with_json, run_cli_with_json_and_runtime};

#[test]
fn tool_catalog_includes_security_portfolio_execution_replay_commit_audit() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_portfolio_execution_replay_commit_audit"),
        "tool catalog should include P19E audit"
    );
}

#[test]
fn security_portfolio_execution_replay_commit_audit_rejects_wrong_p19d_identity() {
    let request = audit_request(
        p19d_document(vec![], "committed").with_mutation(|document| {
            document["document_type"] =
                json!("security_portfolio_execution_replay_commit_preflight")
        }),
    );

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .unwrap_or_default()
            .contains("unsupported P19D document type"),
        "output={output}"
    );
}

#[test]
fn security_portfolio_execution_replay_commit_audit_rejects_missing_non_atomicity_notice() {
    let request = audit_request(p19d_document(vec![], "no_commit_work").with_mutation(
        |document| {
            document
                .as_object_mut()
                .expect("document should be object")
                .remove("non_atomicity_notice");
        },
    ));

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .unwrap_or_default()
            .contains("non_atomicity_notice"),
        "output={output}"
    );
}

#[test]
fn security_portfolio_execution_replay_commit_audit_emits_no_work_without_runtime_writes() {
    let request = audit_request(p19d_document(vec![], "no_commit_work"));

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_replay_commit_audit"];
    assert_eq!(
        document["document_type"],
        "security_portfolio_execution_replay_commit_audit"
    );
    assert_eq!(document["audit_status"], "no_commit_work");
    assert_eq!(document["audit_row_count"], 0);
    assert_eq!(document["runtime_write_count"], 0);
    assert_eq!(
        document["source_non_atomicity_notice"],
        "controlled per-row writer: earlier rows may remain committed if a later row fails"
    );
}

#[test]
fn security_portfolio_execution_replay_commit_audit_verifies_committed_runtime_metadata() {
    let runtime_db_path = create_test_runtime_db("p19e_commit_audit_verified");
    let row = p19d_row(
        "601916.SH",
        "committed",
        "p19e-key-verified",
        "sha256:p19e-hash-verified",
    );
    seed_execution_record(
        &runtime_db_path,
        &row,
        "p19e-key-verified",
        "sha256:p19e-hash-verified",
        SOURCE_P19C_REF,
    );
    let request = audit_request(p19d_document(vec![row], "committed"));

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-26 CST: Reason=P19E must prove P19D runtime writes with machine-readable metadata.
    // Purpose=verify the audit layer reads runtime facts without creating new execution records.
    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_replay_commit_audit"];
    assert_eq!(document["audit_status"], "verified");
    assert_eq!(document["runtime_write_count"], 0);
    assert_eq!(document["verified_count"], 1);
    assert_eq!(document["audit_rows"][0]["audit_status"], "verified");
}

#[test]
fn security_portfolio_execution_replay_commit_audit_marks_missing_runtime_record() {
    let runtime_db_path = create_test_runtime_db("p19e_commit_audit_missing");
    let row = p19d_row(
        "601916.SH",
        "committed",
        "p19e-key-missing",
        "sha256:p19e-hash-missing",
    );
    let request = audit_request(p19d_document(vec![row], "committed"));

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_replay_commit_audit"];
    assert_eq!(document["audit_status"], "partial_audit_failure");
    assert_eq!(document["missing_runtime_record_count"], 1);
    assert_eq!(
        document["audit_rows"][0]["audit_status"],
        "missing_runtime_record"
    );
}

#[test]
fn security_portfolio_execution_replay_commit_audit_marks_metadata_mismatch() {
    let runtime_db_path = create_test_runtime_db("p19e_commit_audit_mismatch");
    let row = p19d_row(
        "601916.SH",
        "committed",
        "p19e-key-mismatch",
        "sha256:p19e-hash-mismatch",
    );
    seed_execution_record(
        &runtime_db_path,
        &row,
        "p19e-key-mismatch",
        "sha256:different-runtime-hash",
        SOURCE_P19C_REF,
    );
    let request = audit_request(p19d_document(vec![row], "committed"));

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_replay_commit_audit"];
    assert_eq!(document["audit_status"], "partial_audit_failure");
    assert_eq!(document["metadata_mismatch_count"], 1);
    assert_eq!(
        document["audit_rows"][0]["audit_status"],
        "metadata_mismatch"
    );
}

#[test]
fn security_portfolio_execution_replay_commit_audit_preserves_failed_and_conflict_rows() {
    let failed = p19d_row(
        "601916.SH",
        "commit_failed",
        "p19e-key-failed",
        "sha256:p19e-hash-failed",
    );
    let conflict = p19d_row(
        "601916.SH",
        "idempotency_conflict",
        "p19e-key-conflict",
        "sha256:p19e-hash-conflict",
    );
    let request = audit_request(p19d_document(
        vec![failed, conflict],
        "partial_commit_failure",
    ));

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_replay_commit_audit"];
    assert_eq!(document["audit_status"], "verified_with_preserved_failures");
    assert_eq!(document["commit_failed_preserved_count"], 1);
    assert_eq!(document["idempotency_conflict_confirmed_count"], 1);
    assert_eq!(
        document["audit_rows"][0]["audit_status"],
        "commit_failed_preserved"
    );
    assert_eq!(
        document["audit_rows"][1]["audit_status"],
        "idempotency_conflict_confirmed"
    );
}

#[test]
fn security_portfolio_execution_replay_commit_audit_source_guard_is_read_only() {
    let source_path = Path::new("src")
        .join("ops")
        .join("security_portfolio_execution_replay_commit_audit.rs");
    let source =
        fs::read_to_string(&source_path).expect("P19E audit source should exist for source guard");

    assert!(source.contains("load_execution_record("));
    assert!(!source.contains("security_execution_record("));
    assert!(!source.contains("upsert_execution_record("));
    assert!(!source.contains(".execute("));
    assert!(!source.contains("open_session("));
    assert!(!source.contains("INSERT INTO security_execution_records"));
    assert!(source.contains("runtime_write_count: 0"));
    assert!(!source.contains("lifecycle closeout"));
}

const SOURCE_P19C_REF: &str =
    "portfolio-execution-replay-commit-preflight:acct-1:2026-04-26T10:00:00+08:00";

trait MutateValue {
    fn with_mutation(self, mutate: impl FnOnce(&mut Value)) -> Value;
}

impl MutateValue for Value {
    fn with_mutation(mut self, mutate: impl FnOnce(&mut Value)) -> Value {
        mutate(&mut self);
        self
    }
}

fn audit_request(p19d: Value) -> Value {
    json!({
        "tool": "security_portfolio_execution_replay_commit_audit",
        "args": {
            "portfolio_execution_replay_commit_writer": p19d,
            "created_at": "2026-04-26T12:00:00+08:00"
        }
    })
}

fn p19d_document(rows: Vec<Value>, status: &str) -> Value {
    let committed_count = rows
        .iter()
        .filter(|row| row["row_status"] == "committed")
        .count();
    let already_committed_count = rows
        .iter()
        .filter(|row| row["row_status"] == "already_committed")
        .count();
    let failed_commit_count = rows
        .iter()
        .filter(|row| row["row_status"] == "commit_failed")
        .count();
    let idempotency_conflict_count = rows
        .iter()
        .filter(|row| row["row_status"] == "idempotency_conflict")
        .count();

    json!({
        "portfolio_execution_replay_commit_writer_id": "portfolio-execution-replay-commit-writer:acct-1:2026-04-26T11:00:00+08:00",
        "contract_version": "security_portfolio_execution_replay_commit_writer.v1",
        "document_type": "security_portfolio_execution_replay_commit_writer",
        "generated_at": "2026-04-26T11:00:00+08:00",
        "analysis_date": "2025-09-17",
        "account_id": "acct-1",
        "commit_mode": "controlled_per_row",
        "source_p19c_ref": SOURCE_P19C_REF,
        "commit_rows": rows,
        "commit_row_count": rows.len(),
        "committed_count": committed_count,
        "already_committed_count": already_committed_count,
        "failed_commit_count": failed_commit_count,
        "idempotency_conflict_count": idempotency_conflict_count,
        "runtime_write_count": committed_count,
        "commit_status": status,
        "blockers": [],
        "commit_rationale": ["fixture"],
        "non_atomicity_notice": "controlled per-row writer: earlier rows may remain committed if a later row fails",
        "commit_summary": "fixture"
    })
}

fn p19d_row(symbol: &str, row_status: &str, key: &str, hash: &str) -> Value {
    let target_ref = format!("execution-record-replay:{key}");
    json!({
        "symbol": symbol,
        "row_status": row_status,
        "commit_idempotency_key": key,
        "canonical_commit_payload_hash": hash,
        "planned_execution_record_ref": format!("preflight:{key}"),
        "target_execution_record_ref": target_ref,
        "runtime_execution_record_ref": if matches!(row_status, "committed" | "already_committed") {
            json!(target_ref)
        } else {
            Value::Null
        },
        "failure_reason": Value::Null
    })
}

fn seed_execution_record(
    runtime_db_path: &PathBuf,
    p19d_row: &Value,
    runtime_key: &str,
    runtime_hash: &str,
    runtime_source_p19c_ref: &str,
) {
    let execution_db_path = runtime_db_path
        .parent()
        .expect("runtime db path should have parent")
        .join("security_execution.db");
    let connection = Connection::open(execution_db_path).expect("execution db should open");
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS security_execution_records (
                execution_record_id TEXT PRIMARY KEY,
                account_id TEXT,
                symbol TEXT NOT NULL,
                analysis_date TEXT NOT NULL,
                position_state TEXT NOT NULL,
                current_position_pct REAL NOT NULL,
                sector_tag TEXT,
                payload_json TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .expect("execution schema should exist");

    let execution_record_id = p19d_row["target_execution_record_ref"]
        .as_str()
        .expect("target ref should be string");
    let payload = execution_record_payload(
        execution_record_id,
        p19d_row["symbol"].as_str().unwrap_or("601916.SH"),
        runtime_key,
        runtime_hash,
        runtime_source_p19c_ref,
    );
    connection
        .execute(
            "INSERT INTO security_execution_records (
                execution_record_id,
                account_id,
                symbol,
                analysis_date,
                position_state,
                current_position_pct,
                sector_tag,
                payload_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                execution_record_id,
                "acct-1",
                p19d_row["symbol"].as_str().unwrap_or("601916.SH"),
                "2025-09-17",
                "open",
                0.12_f64,
                "bank",
                payload.to_string()
            ],
        )
        .expect("execution record seed should insert");
}

fn execution_record_payload(
    execution_record_id: &str,
    symbol: &str,
    runtime_key: &str,
    runtime_hash: &str,
    runtime_source_p19c_ref: &str,
) -> Value {
    json!({
        "execution_record_id": execution_record_id,
        "contract_version": "security_execution_record.v1",
        "document_type": "security_execution_record",
        "generated_at": "2026-04-26T11:01:00+08:00",
        "symbol": symbol,
        "analysis_date": "2025-09-17",
        "account_id": "acct-1",
        "sector_tag": "bank",
        "position_state": "open",
        "portfolio_position_plan_ref": Value::Null,
        "execution_journal_ref": "execution-journal:fixture",
        "position_plan_ref": "position-plan:fixture",
        "snapshot_ref": "snapshot:fixture",
        "outcome_ref": "outcome:fixture",
        "planned_entry_date": "2025-09-18",
        "planned_entry_price": 62.40,
        "planned_position_pct": 0.12,
        "planned_max_position_pct": 0.12,
        "actual_entry_date": "2025-09-18",
        "actual_entry_price": 62.40,
        "actual_position_pct": 0.12,
        "current_position_pct": 0.12,
        "actual_exit_date": "",
        "actual_exit_price": 0.0,
        "exit_reason": "position_still_open",
        "holding_days": 0,
        "planned_forward_return": 0.0,
        "actual_return": 0.0,
        "entry_slippage_pct": 0.0,
        "position_size_gap_pct": 0.0,
        "planned_tranche_action": Value::Null,
        "planned_tranche_pct": Value::Null,
        "planned_peak_position_pct": Value::Null,
        "actual_tranche_action": Value::Null,
        "actual_tranche_pct": Value::Null,
        "actual_peak_position_pct": Value::Null,
        "tranche_count_drift": Value::Null,
        "account_budget_alignment": Value::Null,
        "execution_return_gap": 0.0,
        "execution_quality": "fixture",
        "replay_commit_idempotency_key": runtime_key,
        "replay_commit_payload_hash": runtime_hash,
        "replay_commit_source_p19c_ref": runtime_source_p19c_ref,
        "execution_record_notes": ["p19e audit fixture"],
        "attribution_summary": "fixture"
    })
}
