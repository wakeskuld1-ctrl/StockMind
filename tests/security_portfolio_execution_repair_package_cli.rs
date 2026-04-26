mod common;

use serde_json::{Value, json};

use crate::common::run_cli_with_json;

// 2026-04-25 CST: Added because D:\SM lacked the P18 repair-intent package
// recorded after the P17 reconciliation closeout.
// Reason: catalog visibility is the first public contract for the recovered P18 tool.
// Purpose: lock P18 discovery before restoring implementation and dispatcher routing.
#[test]
fn tool_catalog_includes_security_portfolio_execution_repair_package() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_portfolio_execution_repair_package")
    );
}

// 2026-04-25 CST: Added because a fully settled P17 artifact should not invent
// repair rows downstream.
// Reason: P18 is repair-intent freeze only when unresolved reconciliation truth exists.
// Purpose: prove settled P17 input produces a no-repair package.
#[test]
fn security_portfolio_execution_repair_package_emits_no_repair_for_fully_settled() {
    let request = json!({
        "tool": "security_portfolio_execution_repair_package",
        "args": {
            "portfolio_execution_reconciliation_bridge": build_reconciliation_document(
                "fully_settled",
                2,
                0,
                0,
                0,
                vec![
                    reconciliation_row("601916.SH", "settled", false, Some("execution-record-1"), Some("execution-journal-1"), vec![]),
                    reconciliation_row("8306.T", "settled", false, Some("execution-record-2"), Some("execution-journal-2"), vec![]),
                ],
                vec![],
                vec![]
            ),
            "created_at": "2026-04-25T12:00:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_repair_package"];
    assert_eq!(
        document["document_type"],
        "security_portfolio_execution_repair_package"
    );
    assert_eq!(
        document["contract_version"],
        "security_portfolio_execution_repair_package.v1"
    );
    assert_eq!(document["repair_status"], "no_repair_required");
    assert_eq!(
        document["repair_rows"]
            .as_array()
            .expect("repair rows")
            .len(),
        0
    );
    assert_eq!(document["manual_follow_up_count"], 0);
    assert_eq!(document["governed_retry_candidate_count"], 0);
    assert_eq!(document["blocked_pending_decision_count"], 0);
}

// 2026-04-25 CST: Added because P18 manual-follow-up must be explicit and must
// not be inferred from every unresolved row.
// Reason: this locks the recovered design against the earlier ambiguous default.
// Purpose: prove P17 requires_manual_follow_up=true maps to manual_follow_up.
#[test]
fn security_portfolio_execution_repair_package_marks_manual_follow_up_rows() {
    let request = json!({
        "tool": "security_portfolio_execution_repair_package",
        "args": {
            "portfolio_execution_reconciliation_bridge": build_reconciliation_document(
                "reconciliation_required",
                1,
                0,
                1,
                1,
                vec![
                    reconciliation_row("601916.SH", "settled", false, Some("execution-record-1"), Some("execution-journal-1"), vec![]),
                    reconciliation_row("8306.T", "reconciliation_required", true, None, None, vec!["8306.T manual follow-up required after apply failure"]),
                ],
                vec!["8306.T manual follow-up required after apply failure"],
                vec!["8306.T requires reconciliation after status bridge portfolio-execution-status-bridge:acct-1:2026-04-25T10:00:00+08:00"]
            ),
            "created_at": "2026-04-25T12:05:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_repair_package"];
    assert_eq!(document["repair_status"], "repair_required");
    assert_eq!(document["manual_follow_up_count"], 1);
    assert_eq!(
        document["repair_rows"][0]["repair_class"],
        "manual_follow_up"
    );
    assert_eq!(document["repair_rows"][0]["symbol"], "8306.T");
}

// 2026-04-25 CST: Added because governance blockers must stop P18 from
// silently converting blocked rows into retry work.
// Reason: blocked/pending decision rows belong to governance, not executor retry.
// Purpose: prove blocker text drives blocked_pending_decision.
#[test]
fn security_portfolio_execution_repair_package_marks_blocked_pending_decision_rows() {
    let request = json!({
        "tool": "security_portfolio_execution_repair_package",
        "args": {
            "portfolio_execution_reconciliation_bridge": build_reconciliation_document(
                "reconciliation_required",
                0,
                0,
                1,
                0,
                vec![
                    reconciliation_row("8306.T", "reconciliation_required", false, None, None, vec!["8306.T blocked pending governance decision"])
                ],
                vec!["8306.T blocked pending governance decision"],
                vec![]
            ),
            "created_at": "2026-04-25T12:10:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_repair_package"];
    assert_eq!(document["blocked_pending_decision_count"], 1);
    assert_eq!(
        document["repair_rows"][0]["repair_class"],
        "blocked_pending_decision"
    );
}

// 2026-04-25 CST: Added because retry candidates are distinct from manual
// follow-up and blocked governance decisions.
// Reason: future P19 can only be designed safely if P18 exposes retryable intent explicitly.
// Purpose: prove retryable unresolved rows become governed_retry_candidate.
#[test]
fn security_portfolio_execution_repair_package_marks_governed_retry_candidate_rows() {
    let request = json!({
        "tool": "security_portfolio_execution_repair_package",
        "args": {
            "portfolio_execution_reconciliation_bridge": build_reconciliation_document(
                "reconciliation_required",
                0,
                0,
                1,
                0,
                vec![
                    reconciliation_row("8306.T", "reconciliation_required", false, Some("execution-record-retry"), None, vec!["8306.T retryable execution record mismatch"])
                ],
                vec![],
                vec!["8306.T retryable execution record mismatch"]
            ),
            "created_at": "2026-04-25T12:15:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_repair_package"];
    assert_eq!(document["governed_retry_candidate_count"], 1);
    assert_eq!(
        document["repair_rows"][0]["repair_class"],
        "governed_retry_candidate"
    );
}

// 2026-04-25 CST: Added because P18 must refuse unresolved rows when evidence
// is too weak to classify the next action.
// Reason: guessing ambiguous rows into manual follow-up was explicitly rejected.
// Purpose: prove ambiguous unresolved rows hard-fail instead of receiving a default repair class.
#[test]
fn security_portfolio_execution_repair_package_rejects_ambiguous_repair_classification() {
    let request = json!({
        "tool": "security_portfolio_execution_repair_package",
        "args": {
            "portfolio_execution_reconciliation_bridge": build_reconciliation_document(
                "reconciliation_required",
                0,
                0,
                1,
                0,
                vec![
                    reconciliation_row("8306.T", "reconciliation_required", false, None, None, vec!["8306.T needs review"])
                ],
                vec![],
                vec!["8306.T needs review"]
            ),
            "created_at": "2026-04-25T12:20:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .expect("error should be text")
            .contains("ambiguous repair classification on `8306.T`"),
        "unexpected output: {output}"
    );
}

fn build_reconciliation_document(
    reconciliation_status: &str,
    settled_count: usize,
    skipped_hold_count: usize,
    reconciliation_required_count: usize,
    manual_follow_up_count: usize,
    reconciliation_rows: Vec<Value>,
    blockers: Vec<&str>,
    pending_items: Vec<&str>,
) -> Value {
    json!({
        "portfolio_execution_reconciliation_bridge_id": "portfolio-execution-reconciliation-bridge:acct-1:2026-04-25T11:00:00+08:00",
        "contract_version": "security_portfolio_execution_reconciliation_bridge.v1",
        "document_type": "security_portfolio_execution_reconciliation_bridge",
        "generated_at": "2026-04-25T11:00:00+08:00",
        "analysis_date": "2026-04-24",
        "account_id": "acct-1",
        "portfolio_execution_status_bridge_ref": "portfolio-execution-status-bridge:acct-1:2026-04-25T10:00:00+08:00",
        "portfolio_execution_apply_bridge_ref": "portfolio-execution-apply-bridge:acct-1:2026-04-25T09:30:00+08:00",
        "portfolio_execution_request_enrichment_ref": "portfolio-execution-request-enrichment:acct-1:2026-04-25T09:00:00+08:00",
        "portfolio_execution_request_package_ref": "portfolio-execution-request-package:acct-1:2026-04-25T08:55:00+08:00",
        "portfolio_execution_preview_ref": "portfolio-execution-preview:acct-1:2026-04-25T08:50:00+08:00",
        "portfolio_allocation_decision_ref": "portfolio-allocation-decision:acct-1:2026-04-25T08:45:00+08:00",
        "reconciliation_rows": reconciliation_rows,
        "settled_count": settled_count,
        "skipped_hold_count": skipped_hold_count,
        "reconciliation_required_count": reconciliation_required_count,
        "manual_follow_up_count": manual_follow_up_count,
        "unresolved_count": reconciliation_required_count + manual_follow_up_count,
        "reconciliation_status": reconciliation_status,
        "pending_items": pending_items,
        "blockers": blockers,
        "reconciliation_rationale": [
            "execution reconciliation bridge consumed status bridge portfolio-execution-status-bridge:acct-1:2026-04-25T10:00:00+08:00"
        ],
        "reconciliation_summary": "test reconciliation summary"
    })
}

fn reconciliation_row(
    symbol: &str,
    reconciliation_status: &str,
    requires_manual_follow_up: bool,
    execution_record_ref: Option<&str>,
    execution_journal_ref: Option<&str>,
    blockers: Vec<&str>,
) -> Value {
    json!({
        "symbol": symbol,
        "request_action": "buy",
        "requested_gross_pct": 0.08,
        "enrichment_status": "ready_for_apply",
        "apply_status": if reconciliation_status == "settled" { "applied" } else { "apply_failed" },
        "execution_status": if reconciliation_status == "settled" { "applied" } else { "apply_failed" },
        "reconciliation_status": reconciliation_status,
        "execution_record_ref": execution_record_ref,
        "execution_journal_ref": execution_journal_ref,
        "requires_manual_follow_up": requires_manual_follow_up,
        "blockers": blockers,
        "reconciliation_summary": format!("{symbol} test reconciliation row")
    })
}
