#![recursion_limit = "256"]

mod common;

use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{create_test_runtime_db, run_cli_with_json, run_cli_with_json_and_runtime};

#[test]
fn tool_catalog_includes_security_portfolio_execution_lifecycle_closeout_evidence_package() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| {
                tool == "security_portfolio_execution_lifecycle_closeout_evidence_package"
            }),
        "tool catalog should include P20B evidence package"
    );
}

#[test]
fn security_portfolio_execution_lifecycle_closeout_evidence_package_rejects_wrong_p20a_identity() {
    let request = evidence_request(p20a_document(vec![]).with_mutation(|document| {
        document["document_type"] =
            json!("security_portfolio_execution_lifecycle_closeout_readiness_draft")
    }));

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .unwrap_or_default()
            .contains("unsupported P20A document type"),
        "output={output}"
    );
}

#[test]
fn security_portfolio_execution_lifecycle_closeout_evidence_package_rejects_runtime_write_count() {
    let request = evidence_request(
        p20a_document(vec![]).with_mutation(|document| document["runtime_write_count"] = json!(1)),
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
fn security_portfolio_execution_lifecycle_closeout_evidence_package_rejects_missing_lineage() {
    let request = evidence_request(
        p20a_document(vec![]).with_mutation(|document| document["source_p19e_ref"] = json!("")),
    );

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .unwrap_or_default()
            .contains("source P19E ref"),
        "output={output}"
    );
}

#[test]
fn security_portfolio_execution_lifecycle_closeout_evidence_package_emits_no_candidates() {
    let request = evidence_request(p20a_document(vec![]));

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_lifecycle_closeout_evidence_package"];
    assert_eq!(
        document["document_type"],
        "security_portfolio_execution_lifecycle_closeout_evidence_package"
    );
    assert_eq!(
        document["evidence_status"],
        "no_closeout_evidence_candidates"
    );
    assert_eq!(document["evidence_row_count"], 0);
    assert_eq!(document["runtime_read_count"], 0);
    assert_eq!(document["runtime_write_count"], 0);
}

#[test]
fn security_portfolio_execution_lifecycle_closeout_evidence_package_preserves_p20a_blocked_rows_without_runtime_read()
 {
    let request = evidence_request(p20a_document(vec![p20a_blocked_row(
        "601916.SH",
        "blocked_metadata_mismatch",
        "p20b-key-blocked",
    )]));

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_lifecycle_closeout_evidence_package"];
    assert_eq!(document["evidence_status"], "blocked");
    assert_eq!(document["blocked_p20a_not_eligible_count"], 1);
    assert_eq!(document["runtime_read_count"], 0);
    assert_eq!(
        document["evidence_rows"][0]["evidence_status"],
        "blocked_p20a_not_eligible"
    );
}

#[test]
fn security_portfolio_execution_lifecycle_closeout_evidence_package_verifies_closed_runtime_evidence()
 {
    let runtime_db_path = create_test_runtime_db("p20b_evidence_ready");
    let row = p20a_eligible_row("601916.SH", "p20b-key-ready");
    seed_execution_record(
        &runtime_db_path,
        &row,
        RuntimeRecordFixture {
            position_state: "closed",
            actual_exit_date: "2025-10-02",
            actual_exit_price: 66.10,
            exit_reason: "target_hit",
            account_id: "acct-1",
            symbol: "601916.SH",
            runtime_key: "p20b-key-ready",
            runtime_hash: "sha256:p20b-key-ready",
            runtime_source_p19c_ref: SOURCE_P19C_REF,
        },
    );
    let request = evidence_request(p20a_document(vec![row]));

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-26 CST: Reason=P20B must require closed runtime evidence beyond P20A readiness.
    // Purpose=prove the evidence package is read-only pre-archive proof, not lifecycle closure.
    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_lifecycle_closeout_evidence_package"];
    assert_eq!(document["evidence_status"], "closeout_evidence_ready");
    assert_eq!(
        document["evidence_ready_for_closeout_archive_preflight_count"],
        1
    );
    assert_eq!(document["runtime_read_count"], 1);
    assert_eq!(document["runtime_write_count"], 0);
    assert_eq!(
        document["evidence_rows"][0]["evidence_status"],
        "evidence_ready_for_closeout_archive_preflight"
    );
    assert!(
        document["evidence_summary"]
            .as_str()
            .unwrap_or_default()
            .contains("not lifecycle closure")
    );
}

#[test]
fn security_portfolio_execution_lifecycle_closeout_evidence_package_blocks_missing_and_open_runtime_evidence()
 {
    let runtime_db_path = create_test_runtime_db("p20b_evidence_blocked");
    let missing = p20a_eligible_row("601916.SH", "p20b-key-missing");
    let open = p20a_eligible_row("601916.SH", "p20b-key-open");
    seed_execution_record(
        &runtime_db_path,
        &open,
        RuntimeRecordFixture {
            position_state: "open",
            actual_exit_date: "",
            actual_exit_price: 0.0,
            exit_reason: "position_still_open",
            account_id: "acct-1",
            symbol: "601916.SH",
            runtime_key: "p20b-key-open",
            runtime_hash: "sha256:p20b-key-open",
            runtime_source_p19c_ref: SOURCE_P19C_REF,
        },
    );
    let request = evidence_request(p20a_document(vec![missing, open]));

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_lifecycle_closeout_evidence_package"];
    assert_eq!(document["evidence_status"], "blocked");
    assert_eq!(document["blocked_missing_runtime_record_count"], 1);
    assert_eq!(document["blocked_runtime_record_not_closed_count"], 1);
    assert_eq!(document["runtime_read_count"], 2);
    assert_eq!(
        document["evidence_rows"][0]["evidence_status"],
        "blocked_missing_runtime_record"
    );
    assert_eq!(
        document["evidence_rows"][1]["evidence_status"],
        "blocked_runtime_record_not_closed"
    );
}

#[test]
fn security_portfolio_execution_lifecycle_closeout_evidence_package_blocks_exit_metadata_and_lineage_mismatches()
 {
    let runtime_db_path = create_test_runtime_db("p20b_evidence_mismatches");
    let missing_exit = p20a_eligible_row("601916.SH", "p20b-key-exit");
    let replay_mismatch = p20a_eligible_row("601916.SH", "p20b-key-replay");
    let account_mismatch = p20a_eligible_row("601916.SH", "p20b-key-account");
    seed_execution_record(
        &runtime_db_path,
        &missing_exit,
        RuntimeRecordFixture {
            position_state: "closed",
            actual_exit_date: "",
            actual_exit_price: 66.10,
            exit_reason: "target_hit",
            account_id: "acct-1",
            symbol: "601916.SH",
            runtime_key: "p20b-key-exit",
            runtime_hash: "sha256:p20b-key-exit",
            runtime_source_p19c_ref: SOURCE_P19C_REF,
        },
    );
    seed_execution_record(
        &runtime_db_path,
        &replay_mismatch,
        RuntimeRecordFixture {
            position_state: "closed",
            actual_exit_date: "2025-10-02",
            actual_exit_price: 66.10,
            exit_reason: "target_hit",
            account_id: "acct-1",
            symbol: "601916.SH",
            runtime_key: "p20b-key-replay",
            runtime_hash: "sha256:different",
            runtime_source_p19c_ref: SOURCE_P19C_REF,
        },
    );
    seed_execution_record(
        &runtime_db_path,
        &account_mismatch,
        RuntimeRecordFixture {
            position_state: "closed",
            actual_exit_date: "2025-10-02",
            actual_exit_price: 66.10,
            exit_reason: "target_hit",
            account_id: "acct-other",
            symbol: "601916.SH",
            runtime_key: "p20b-key-account",
            runtime_hash: "sha256:p20b-key-account",
            runtime_source_p19c_ref: SOURCE_P19C_REF,
        },
    );
    let request = evidence_request(p20a_document(vec![
        missing_exit,
        replay_mismatch,
        account_mismatch,
    ]));

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_lifecycle_closeout_evidence_package"];
    assert_eq!(document["evidence_status"], "blocked");
    assert_eq!(document["blocked_missing_exit_evidence_count"], 1);
    assert_eq!(document["blocked_replay_metadata_mismatch_count"], 1);
    assert_eq!(document["blocked_account_or_symbol_mismatch_count"], 1);
    assert_eq!(
        document["evidence_rows"][0]["evidence_status"],
        "blocked_missing_exit_evidence"
    );
    assert_eq!(
        document["evidence_rows"][1]["evidence_status"],
        "blocked_replay_metadata_mismatch"
    );
    assert_eq!(
        document["evidence_rows"][2]["evidence_status"],
        "blocked_account_or_symbol_mismatch"
    );
}

#[test]
fn security_portfolio_execution_lifecycle_closeout_evidence_package_reports_partial_evidence_readiness()
 {
    let runtime_db_path = create_test_runtime_db("p20b_evidence_partial");
    let ready = p20a_eligible_row("601916.SH", "p20b-key-ready-partial");
    let blocked = p20a_eligible_row("601916.SH", "p20b-key-open-partial");
    seed_execution_record(
        &runtime_db_path,
        &ready,
        RuntimeRecordFixture {
            position_state: "closed",
            actual_exit_date: "2025-10-02",
            actual_exit_price: 66.10,
            exit_reason: "target_hit",
            account_id: "acct-1",
            symbol: "601916.SH",
            runtime_key: "p20b-key-ready-partial",
            runtime_hash: "sha256:p20b-key-ready-partial",
            runtime_source_p19c_ref: SOURCE_P19C_REF,
        },
    );
    seed_execution_record(
        &runtime_db_path,
        &blocked,
        RuntimeRecordFixture {
            position_state: "open",
            actual_exit_date: "",
            actual_exit_price: 0.0,
            exit_reason: "position_still_open",
            account_id: "acct-1",
            symbol: "601916.SH",
            runtime_key: "p20b-key-open-partial",
            runtime_hash: "sha256:p20b-key-open-partial",
            runtime_source_p19c_ref: SOURCE_P19C_REF,
        },
    );
    let request = evidence_request(p20a_document(vec![ready, blocked]));

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_lifecycle_closeout_evidence_package"];
    assert_eq!(
        document["evidence_status"],
        "partial_closeout_evidence_ready"
    );
    assert_eq!(
        document["evidence_ready_for_closeout_archive_preflight_count"],
        1
    );
    assert_eq!(document["blocked_runtime_record_not_closed_count"], 1);
}

#[test]
fn security_portfolio_execution_lifecycle_closeout_evidence_package_source_guard_is_read_only() {
    let source_path = Path::new("src")
        .join("ops")
        .join("security_portfolio_execution_lifecycle_closeout_evidence_package.rs");
    let source = fs::read_to_string(&source_path)
        .expect("P20B evidence package source should exist for source guard");

    assert!(source.contains("load_execution_record("));
    assert!(!source.contains("security_execution_record("));
    assert!(!source.contains("security_post_trade_review("));
    assert!(!source.contains("security_closed_position_archive("));
    assert!(!source.contains("upsert_execution_record("));
    assert!(!source.contains("upsert_position_plan("));
    assert!(!source.contains("upsert_adjustment_event("));
    assert!(!source.contains(".execute("));
    assert!(!source.contains(".execute_batch("));
    assert!(!source.contains("open_session("));
    assert!(!source.contains("INSERT INTO security_execution_records"));
    assert!(source.contains("runtime_write_count: 0"));
    assert!(!source.contains("lifecycle closed"));
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

fn evidence_request(p20a: Value) -> Value {
    json!({
        "tool": "security_portfolio_execution_lifecycle_closeout_evidence_package",
        "args": {
            "portfolio_execution_lifecycle_closeout_readiness": p20a,
            "created_at": "2026-04-26T14:00:00+08:00"
        }
    })
}

fn p20a_document(rows: Vec<Value>) -> Value {
    let eligible_count = rows
        .iter()
        .filter(|row| row["readiness_status"] == "eligible_for_closeout_preflight")
        .count();
    let blocked_metadata_mismatch_count = rows
        .iter()
        .filter(|row| row["readiness_status"] == "blocked_metadata_mismatch")
        .count();

    json!({
        "portfolio_execution_lifecycle_closeout_readiness_id": "portfolio-execution-lifecycle-closeout-readiness:acct-1:2026-04-26T13:00:00+08:00",
        "contract_version": "security_portfolio_execution_lifecycle_closeout_readiness.v1",
        "document_type": "security_portfolio_execution_lifecycle_closeout_readiness",
        "generated_at": "2026-04-26T13:00:00+08:00",
        "analysis_date": "2025-09-17",
        "account_id": "acct-1",
        "source_p19e_ref": "portfolio-execution-replay-commit-audit:acct-1:2026-04-26T12:00:00+08:00",
        "source_p19d_ref": "portfolio-execution-replay-commit-writer:acct-1:2026-04-26T11:00:00+08:00",
        "source_p19c_ref": SOURCE_P19C_REF,
        "source_non_atomicity_notice": "controlled per-row writer: earlier rows may remain committed if a later row fails",
        "readiness_rows": rows,
        "readiness_row_count": rows.len(),
        "eligible_for_closeout_preflight_count": eligible_count,
        "blocked_missing_runtime_record_count": 0,
        "blocked_metadata_mismatch_count": blocked_metadata_mismatch_count,
        "blocked_commit_failed_count": 0,
        "blocked_idempotency_conflict_count": 0,
        "blocked_no_commit_work_count": 0,
        "blocked_not_auditable_count": 0,
        "blocked_unknown_audit_status_count": 0,
        "runtime_write_count": 0,
        "readiness_status": if rows.is_empty() {
            "no_closeout_candidates"
        } else if blocked_metadata_mismatch_count > 0 && eligible_count > 0 {
            "partial_closeout_preflight_ready"
        } else if eligible_count > 0 {
            "closeout_preflight_ready"
        } else {
            "blocked"
        },
        "blockers": [],
        "readiness_rationale": ["fixture"],
        "readiness_summary": "fixture"
    })
}

fn p20a_eligible_row(symbol: &str, key: &str) -> Value {
    let target_ref = format!("execution-record-replay:{key}");
    json!({
        "symbol": symbol,
        "source_p19e_audit_status": "verified",
        "readiness_status": "eligible_for_closeout_preflight",
        "commit_idempotency_key": key,
        "canonical_commit_payload_hash": format!("sha256:{key}"),
        "source_p19c_ref": SOURCE_P19C_REF,
        "target_execution_record_ref": target_ref,
        "runtime_execution_record_ref": target_ref,
        "runtime_replay_commit_idempotency_key": key,
        "runtime_replay_commit_payload_hash": format!("sha256:{key}"),
        "runtime_replay_commit_source_p19c_ref": SOURCE_P19C_REF,
        "closeout_preflight_eligible": true,
        "blockers": []
    })
}

fn p20a_blocked_row(symbol: &str, readiness_status: &str, key: &str) -> Value {
    json!({
        "symbol": symbol,
        "source_p19e_audit_status": "metadata_mismatch",
        "readiness_status": readiness_status,
        "commit_idempotency_key": key,
        "canonical_commit_payload_hash": format!("sha256:{key}"),
        "source_p19c_ref": SOURCE_P19C_REF,
        "target_execution_record_ref": format!("execution-record-replay:{key}"),
        "runtime_execution_record_ref": Value::Null,
        "runtime_replay_commit_idempotency_key": Value::Null,
        "runtime_replay_commit_payload_hash": Value::Null,
        "runtime_replay_commit_source_p19c_ref": Value::Null,
        "closeout_preflight_eligible": false,
        "blockers": ["P20A blocked fixture"]
    })
}

struct RuntimeRecordFixture<'a> {
    position_state: &'a str,
    actual_exit_date: &'a str,
    actual_exit_price: f64,
    exit_reason: &'a str,
    account_id: &'a str,
    symbol: &'a str,
    runtime_key: &'a str,
    runtime_hash: &'a str,
    runtime_source_p19c_ref: &'a str,
}

fn seed_execution_record(
    runtime_db_path: &PathBuf,
    p20a_row: &Value,
    fixture: RuntimeRecordFixture,
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

    let execution_record_id = p20a_row["target_execution_record_ref"]
        .as_str()
        .expect("target ref should be string");
    let payload = execution_record_payload(execution_record_id, &fixture);
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
                fixture.account_id,
                fixture.symbol,
                "2025-09-17",
                fixture.position_state,
                0.0_f64,
                "bank",
                payload.to_string()
            ],
        )
        .expect("execution record seed should insert");
}

fn execution_record_payload(execution_record_id: &str, fixture: &RuntimeRecordFixture) -> Value {
    json!({
        "execution_record_id": execution_record_id,
        "contract_version": "security_execution_record.v1",
        "document_type": "security_execution_record",
        "generated_at": "2026-04-26T13:30:00+08:00",
        "symbol": fixture.symbol,
        "analysis_date": "2025-09-17",
        "account_id": fixture.account_id,
        "sector_tag": "bank",
        "position_state": fixture.position_state,
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
        "current_position_pct": if fixture.position_state == "closed" { 0.0 } else { 0.12 },
        "actual_exit_date": fixture.actual_exit_date,
        "actual_exit_price": fixture.actual_exit_price,
        "exit_reason": fixture.exit_reason,
        "holding_days": 14,
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
        "replay_commit_idempotency_key": fixture.runtime_key,
        "replay_commit_payload_hash": fixture.runtime_hash,
        "replay_commit_source_p19c_ref": fixture.runtime_source_p19c_ref,
        "execution_record_notes": ["p20b evidence fixture"],
        "attribution_summary": "fixture"
    })
}
