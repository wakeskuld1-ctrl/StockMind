mod common;

use serde_json::{Value, json};

use crate::common::run_cli_with_json;

// 2026-04-25 CST: Added because P19A must become a formal public boundary
// only after the user approved the strict replay-request package design.
// Reason: catalog visibility is the first observable contract for P19A.
// Purpose: lock discovery before implementation and dispatcher routing.
#[test]
fn tool_catalog_includes_security_portfolio_execution_replay_request_package() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_portfolio_execution_replay_request_package")
    );
}

// 2026-04-25 CST: Added because no-repair P18 packages must not invent
// replay work downstream.
// Reason: P19A is a replay-request freeze only when P18 exposes governed retry candidates.
// Purpose: prove an empty P18 repair package produces an empty P19A request package.
#[test]
fn security_portfolio_execution_replay_request_package_emits_empty_package_for_no_repair() {
    let request = json!({
        "tool": "security_portfolio_execution_replay_request_package",
        "args": {
            "portfolio_execution_repair_package": build_repair_package_document(
                "no_repair_required",
                0,
                0,
                0,
                vec![],
                vec![]
            ),
            "created_at": "2026-04-25T13:00:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_replay_request_package"];
    assert_eq!(
        document["document_type"],
        "security_portfolio_execution_replay_request_package"
    );
    assert_eq!(
        document["contract_version"],
        "security_portfolio_execution_replay_request_package.v1"
    );
    assert_eq!(document["replay_request_status"], "no_replay_requested");
    assert_eq!(document["replay_request_count"], 0);
    assert_eq!(
        document["replay_request_rows"]
            .as_array()
            .expect("replay rows should be an array")
            .len(),
        0
    );
}

// 2026-04-25 CST: Added because P19A must not treat every repair row as
// replayable work.
// Reason: only P18 governed retry candidates are eligible for a later executor.
// Purpose: prove manual and governance rows are excluded while retry candidates are frozen.
#[test]
fn security_portfolio_execution_replay_request_package_includes_only_governed_retry_candidates() {
    let request = json!({
        "tool": "security_portfolio_execution_replay_request_package",
        "args": {
            "portfolio_execution_repair_package": build_repair_package_document(
                "repair_required",
                1,
                1,
                1,
                vec![
                    repair_row("601916.SH", "manual_follow_up", None, None, vec!["601916.SH manual follow-up required"]),
                    repair_row("8306.T", "governed_retry_candidate", Some("execution-record-retry"), None, vec!["8306.T retryable execution record mismatch"]),
                    repair_row("7203.T", "blocked_pending_decision", None, None, vec!["7203.T blocked pending governance decision"]),
                ],
                vec!["601916.SH manual follow-up required", "7203.T blocked pending governance decision"]
            ),
            "created_at": "2026-04-25T13:05:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_replay_request_package"];
    assert_eq!(document["replay_request_status"], "replay_requested");
    assert_eq!(document["replay_request_count"], 1);
    assert_eq!(document["governed_retry_candidate_count"], 1);
    assert_eq!(document["excluded_manual_follow_up_count"], 1);
    assert_eq!(document["excluded_blocked_pending_decision_count"], 1);
    assert_eq!(document["replay_request_rows"][0]["symbol"], "8306.T");
    assert_eq!(
        document["replay_request_rows"][0]["replay_request_status"],
        "ready_for_replay_request"
    );
    assert_eq!(
        document["replay_request_rows"][0]["portfolio_execution_repair_package_ref"],
        "portfolio-execution-repair-package:acct-1:2026-04-25T12:00:00+08:00"
    );
}

// 2026-04-25 CST: Added because P19A is a formal contract boundary and must
// refuse drifted P18 repair classes.
// Reason: unknown classes cannot be guessed into replay, manual, or governance buckets.
// Purpose: prove unsupported repair classes hard-fail.
#[test]
fn security_portfolio_execution_replay_request_package_rejects_unknown_repair_class() {
    let request = json!({
        "tool": "security_portfolio_execution_replay_request_package",
        "args": {
            "portfolio_execution_repair_package": build_repair_package_document(
                "repair_required",
                0,
                0,
                0,
                vec![repair_row("8306.T", "surprise_class", None, None, vec!["8306.T unknown repair class"])],
                vec![]
            ),
            "created_at": "2026-04-25T13:10:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .expect("error should be text")
            .contains("unsupported repair class `surprise_class`"),
        "unexpected output: {output}"
    );
}

// 2026-04-25 CST: Added because even P18 retry candidates need replay evidence
// before P19A can freeze request rows.
// Reason: a replay request without execution refs or retry/replay evidence would create a fake executor input.
// Purpose: prove retry candidates without evidence hard-fail.
#[test]
fn security_portfolio_execution_replay_request_package_rejects_retry_candidate_without_evidence() {
    let request = json!({
        "tool": "security_portfolio_execution_replay_request_package",
        "args": {
            "portfolio_execution_repair_package": build_repair_package_document(
                "repair_required",
                0,
                1,
                0,
                vec![repair_row("8306.T", "governed_retry_candidate", None, None, vec!["8306.T needs review"])],
                vec![]
            ),
            "created_at": "2026-04-25T13:15:00+08:00"
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

fn build_repair_package_document(
    repair_status: &str,
    manual_follow_up_count: usize,
    governed_retry_candidate_count: usize,
    blocked_pending_decision_count: usize,
    repair_rows: Vec<Value>,
    blockers: Vec<&str>,
) -> Value {
    json!({
        "portfolio_execution_repair_package_id": "portfolio-execution-repair-package:acct-1:2026-04-25T12:00:00+08:00",
        "contract_version": "security_portfolio_execution_repair_package.v1",
        "document_type": "security_portfolio_execution_repair_package",
        "generated_at": "2026-04-25T12:00:00+08:00",
        "analysis_date": "2026-04-24",
        "account_id": "acct-1",
        "portfolio_execution_reconciliation_bridge_ref": "portfolio-execution-reconciliation-bridge:acct-1:2026-04-25T11:00:00+08:00",
        "portfolio_execution_status_bridge_ref": "portfolio-execution-status-bridge:acct-1:2026-04-25T10:00:00+08:00",
        "portfolio_execution_apply_bridge_ref": "portfolio-execution-apply-bridge:acct-1:2026-04-25T09:30:00+08:00",
        "portfolio_execution_request_enrichment_ref": "portfolio-execution-request-enrichment:acct-1:2026-04-25T09:00:00+08:00",
        "portfolio_execution_request_package_ref": "portfolio-execution-request-package:acct-1:2026-04-25T08:55:00+08:00",
        "portfolio_execution_preview_ref": "portfolio-execution-preview:acct-1:2026-04-25T08:50:00+08:00",
        "portfolio_allocation_decision_ref": "portfolio-allocation-decision:acct-1:2026-04-25T08:45:00+08:00",
        "repair_rows": repair_rows,
        "manual_follow_up_count": manual_follow_up_count,
        "governed_retry_candidate_count": governed_retry_candidate_count,
        "blocked_pending_decision_count": blocked_pending_decision_count,
        "repair_required_count": manual_follow_up_count + governed_retry_candidate_count + blocked_pending_decision_count,
        "repair_status": repair_status,
        "blockers": blockers,
        "repair_rationale": [
            "execution repair package only freezes repair intent"
        ],
        "repair_summary": "test repair package"
    })
}

fn repair_row(
    symbol: &str,
    repair_class: &str,
    execution_record_ref: Option<&str>,
    execution_journal_ref: Option<&str>,
    repair_blockers: Vec<&str>,
) -> Value {
    json!({
        "symbol": symbol,
        "request_action": "buy",
        "requested_gross_pct": 0.08,
        "reconciliation_status": "reconciliation_required",
        "repair_class": repair_class,
        "execution_record_ref": execution_record_ref,
        "execution_journal_ref": execution_journal_ref,
        "repair_blockers": repair_blockers,
        "repair_summary": format!("{symbol} test repair row")
    })
}
