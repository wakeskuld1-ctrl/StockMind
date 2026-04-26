mod common;

use serde_json::{Value, json};

use crate::common::run_cli_with_json;

// 2026-04-25 CST: Added because P19B dry-run executor must become a formal
// public boundary only after the user approved the B1 executor design.
// Reason: catalog visibility is the first observable contract for P19B.
// Purpose: lock discovery before implementation and dispatcher routing.
#[test]
fn tool_catalog_includes_security_portfolio_execution_replay_executor() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_portfolio_execution_replay_executor")
    );
}

// 2026-04-25 CST: Added because a no-replay P19A request package should not
// invent executor work.
// Reason: P19B dry-run validates replay work only when P19A exposes request rows.
// Purpose: prove an empty P19A package produces an empty dry-run executor document.
#[test]
fn security_portfolio_execution_replay_executor_emits_no_work_for_empty_replay_package() {
    let request = json!({
        "tool": "security_portfolio_execution_replay_executor",
        "args": {
            "portfolio_execution_replay_request_package": build_replay_request_package(
                "no_replay_requested",
                vec![]
            ),
            "execution_mode": "dry_run",
            "created_at": "2026-04-25T14:00:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_replay_executor"];
    assert_eq!(
        document["document_type"],
        "security_portfolio_execution_replay_executor"
    );
    assert_eq!(
        document["contract_version"],
        "security_portfolio_execution_replay_executor.v1"
    );
    assert_eq!(document["execution_mode"], "dry_run");
    assert_eq!(document["dry_run_status"], "no_replay_work");
    assert_eq!(document["dry_run_row_count"], 0);
    assert_eq!(
        document["executor_rows"]
            .as_array()
            .expect("executor rows should be an array")
            .len(),
        0
    );
}

// 2026-04-25 CST: Added because P19B should freeze deterministic executor
// validation without creating runtime refs.
// Reason: dry-run executor output must be auditable but side-effect-free in this phase.
// Purpose: prove one ready replay request row becomes one validated dry-run row.
#[test]
fn security_portfolio_execution_replay_executor_validates_ready_rows_in_dry_run() {
    let request = json!({
        "tool": "security_portfolio_execution_replay_executor",
        "args": {
            "portfolio_execution_replay_request_package": build_replay_request_package(
                "replay_requested",
                vec![replay_request_row(
                    "8306.T",
                    "ready_for_replay_request",
                    vec!["execution_record_ref:execution-record-retry", "repair_blocker_retry_or_replay_signal"]
                )]
            ),
            "execution_mode": "dry_run",
            "created_at": "2026-04-25T14:05:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_replay_executor"];
    assert_eq!(document["dry_run_status"], "validated_for_dry_run");
    assert_eq!(document["dry_run_row_count"], 1);
    assert_eq!(document["runtime_write_count"], 0);
    let row = &document["executor_rows"][0];
    assert_eq!(row["symbol"], "8306.T");
    assert_eq!(row["dry_run_status"], "validated_for_dry_run");
    assert_eq!(
        row["planned_execution_record_ref"],
        "dry-run:portfolio-execution-replay-request-package:acct-1:2026-04-25T13:00:00+08:00:8306.T"
    );
    assert_eq!(row["runtime_execution_record_ref"], Value::Null);
    assert!(
        row["idempotency_key"]
            .as_str()
            .expect("idempotency key should be text")
            .contains("acct-1|2026-04-24|8306.T|buy|0.08|portfolio-execution-replay-request-package:acct-1:2026-04-25T13:00:00+08:00")
    );
}

// 2026-04-25 CST: Added because commit-mode replay is explicitly outside this
// dry-run-first P19B phase.
// Reason: runtime writes need a separate approved contract for idempotency and rollback semantics.
// Purpose: prove commit mode hard-fails instead of silently behaving like dry-run.
#[test]
fn security_portfolio_execution_replay_executor_rejects_commit_mode() {
    let request = json!({
        "tool": "security_portfolio_execution_replay_executor",
        "args": {
            "portfolio_execution_replay_request_package": build_replay_request_package(
                "replay_requested",
                vec![replay_request_row("8306.T", "ready_for_replay_request", vec!["execution_record_ref:execution-record-retry"])]
            ),
            "execution_mode": "commit",
            "created_at": "2026-04-25T14:10:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .expect("error should be text")
            .contains("unsupported execution mode `commit`"),
        "unexpected output: {output}"
    );
}

// 2026-04-25 CST: Added because replay executor dry-run needs deterministic
// duplicate protection before any future commit mode can exist.
// Reason: duplicate idempotency keys would become duplicate runtime writes in a later executor.
// Purpose: prove duplicate keys hard-fail inside one P19B document.
#[test]
fn security_portfolio_execution_replay_executor_rejects_duplicate_idempotency_keys() {
    let request = json!({
        "tool": "security_portfolio_execution_replay_executor",
        "args": {
            "portfolio_execution_replay_request_package": build_replay_request_package(
                "replay_requested",
                vec![
                    replay_request_row("8306.T", "ready_for_replay_request", vec!["execution_record_ref:execution-record-retry"]),
                    replay_request_row("8306.T", "ready_for_replay_request", vec!["execution_record_ref:execution-record-retry"])
                ]
            ),
            "execution_mode": "dry_run",
            "created_at": "2026-04-25T14:15:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .expect("error should be text")
            .contains("duplicate idempotency key"),
        "unexpected output: {output}"
    );
}

// 2026-04-25 CST: Added because P19B must not dry-run rows that P19A did not
// actually prove as replayable.
// Reason: missing evidence would create a fake executor-ready row.
// Purpose: prove empty replay evidence refs hard-fail.
#[test]
fn security_portfolio_execution_replay_executor_rejects_rows_without_replay_evidence() {
    let request = json!({
        "tool": "security_portfolio_execution_replay_executor",
        "args": {
            "portfolio_execution_replay_request_package": build_replay_request_package(
                "replay_requested",
                vec![replay_request_row("8306.T", "ready_for_replay_request", vec![])]
            ),
            "execution_mode": "dry_run",
            "created_at": "2026-04-25T14:20:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .expect("error should be text")
            .contains("missing replay evidence for `8306.T`"),
        "unexpected output: {output}"
    );
}

// 2026-04-25 CST: Added because P19B should only dry-run rows that P19A
// marked ready for replay request.
// Reason: executor validation must reject drifted or future row states.
// Purpose: prove non-ready replay rows hard-fail.
#[test]
fn security_portfolio_execution_replay_executor_rejects_non_ready_replay_rows() {
    let request = json!({
        "tool": "security_portfolio_execution_replay_executor",
        "args": {
            "portfolio_execution_replay_request_package": build_replay_request_package(
                "replay_requested",
                vec![replay_request_row("8306.T", "pending_replay_decision", vec!["execution_record_ref:execution-record-retry"])]
            ),
            "execution_mode": "dry_run",
            "created_at": "2026-04-25T14:25:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .expect("error should be text")
            .contains("unsupported replay request row status `pending_replay_decision`"),
        "unexpected output: {output}"
    );
}

fn build_replay_request_package(
    replay_request_status: &str,
    replay_request_rows: Vec<Value>,
) -> Value {
    let replay_request_count = replay_request_rows.len();
    json!({
        "portfolio_execution_replay_request_package_id": "portfolio-execution-replay-request-package:acct-1:2026-04-25T13:00:00+08:00",
        "contract_version": "security_portfolio_execution_replay_request_package.v1",
        "document_type": "security_portfolio_execution_replay_request_package",
        "generated_at": "2026-04-25T13:00:00+08:00",
        "analysis_date": "2026-04-24",
        "account_id": "acct-1",
        "portfolio_execution_repair_package_ref": "portfolio-execution-repair-package:acct-1:2026-04-25T12:00:00+08:00",
        "portfolio_execution_reconciliation_bridge_ref": "portfolio-execution-reconciliation-bridge:acct-1:2026-04-25T11:00:00+08:00",
        "portfolio_execution_status_bridge_ref": "portfolio-execution-status-bridge:acct-1:2026-04-25T10:00:00+08:00",
        "portfolio_execution_apply_bridge_ref": "portfolio-execution-apply-bridge:acct-1:2026-04-25T09:30:00+08:00",
        "portfolio_execution_request_enrichment_ref": "portfolio-execution-request-enrichment:acct-1:2026-04-25T09:00:00+08:00",
        "portfolio_execution_request_package_ref": "portfolio-execution-request-package:acct-1:2026-04-25T08:55:00+08:00",
        "portfolio_execution_preview_ref": "portfolio-execution-preview:acct-1:2026-04-25T08:50:00+08:00",
        "portfolio_allocation_decision_ref": "portfolio-allocation-decision:acct-1:2026-04-25T08:45:00+08:00",
        "replay_request_rows": replay_request_rows,
        "governed_retry_candidate_count": replay_request_count,
        "excluded_manual_follow_up_count": 0,
        "excluded_blocked_pending_decision_count": 0,
        "replay_request_count": replay_request_count,
        "replay_request_status": replay_request_status,
        "blockers": [],
        "replay_request_rationale": [
            "execution replay request package includes only governed retry candidates"
        ],
        "replay_request_summary": "test replay request package"
    })
}

fn replay_request_row(
    symbol: &str,
    replay_request_status: &str,
    replay_evidence_refs: Vec<&str>,
) -> Value {
    json!({
        "symbol": symbol,
        "request_action": "buy",
        "requested_gross_pct": 0.08,
        "repair_class": "governed_retry_candidate",
        "replay_request_status": replay_request_status,
        "portfolio_execution_repair_package_ref": "portfolio-execution-repair-package:acct-1:2026-04-25T12:00:00+08:00",
        "execution_record_ref": "execution-record-retry",
        "execution_journal_ref": Value::Null,
        "replay_evidence_refs": replay_evidence_refs,
        "replay_blockers": ["8306.T retryable execution record mismatch"],
        "replay_request_summary": format!("{symbol} frozen as governed replay request")
    })
}
