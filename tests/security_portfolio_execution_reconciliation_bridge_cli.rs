mod common;

use serde_json::{Value, json};

use crate::common::run_cli_with_json;

// 2026-04-25 CST: Added because D:\SM carried P16 but lacked the downstream
// P17 reconciliation bridge recorded in handoff notes.
// Reason: public catalog visibility is the first observable contract for the recovered P17 tool.
// Purpose: lock the tool name before restoring implementation and dispatcher routing.
#[test]
fn tool_catalog_includes_security_portfolio_execution_reconciliation_bridge() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_portfolio_execution_reconciliation_bridge")
    );
}

// 2026-04-25 CST: Added because P17 must freeze settled execution truth from
// P16 without re-reading runtime execution facts.
// Reason: fully applied P16 status should become a fully settled reconciliation artifact.
// Purpose: prove the P17 route preserves lineage and row-level settled refs.
#[test]
fn security_portfolio_execution_reconciliation_bridge_freezes_fully_settled_status_document() {
    let request = json!({
        "tool": "security_portfolio_execution_reconciliation_bridge",
        "args": {
            "portfolio_execution_status_bridge": build_status_document(
                "fully_applied",
                2,
                0,
                0,
                vec![
                    status_row("601916.SH", "buy", 0.12, "ready_for_apply", "applied", "applied", Some("execution-record-1"), Some("execution-journal-1")),
                    status_row("8306.T", "buy", 0.08, "ready_for_apply", "applied", "applied", Some("execution-record-2"), Some("execution-journal-2")),
                ],
                vec![],
                vec![]
            ),
            "created_at": "2026-04-25T11:00:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_reconciliation_bridge"];
    assert_eq!(
        document["document_type"],
        "security_portfolio_execution_reconciliation_bridge"
    );
    assert_eq!(
        document["contract_version"],
        "security_portfolio_execution_reconciliation_bridge.v1"
    );
    assert_eq!(document["reconciliation_status"], "fully_settled");
    assert_eq!(document["settled_count"], 2);
    assert_eq!(document["reconciliation_required_count"], 0);
    assert_eq!(document["manual_follow_up_count"], 0);
    assert_eq!(
        document["portfolio_execution_status_bridge_ref"],
        "portfolio-execution-status-bridge:acct-1:2026-04-25T10:00:00+08:00"
    );
    assert_eq!(
        document["reconciliation_rows"][0]["reconciliation_status"],
        "settled"
    );
    assert_eq!(
        document["reconciliation_rows"][0]["execution_record_ref"],
        "execution-record-1"
    );
}

// 2026-04-25 CST: Added because failed P15 rows must remain unresolved after
// P16 and become explicit P17 reconciliation work, not hidden status prose.
// Reason: P18 can only classify repairs if P17 exposes unresolved row truth.
// Purpose: prove apply-failed rows become reconciliation-required rows with blockers.
#[test]
fn security_portfolio_execution_reconciliation_bridge_marks_failed_rows_unresolved() {
    let request = json!({
        "tool": "security_portfolio_execution_reconciliation_bridge",
        "args": {
            "portfolio_execution_status_bridge": build_status_document(
                "partial_failure",
                1,
                0,
                1,
                vec![
                    status_row("601916.SH", "buy", 0.12, "ready_for_apply", "applied", "applied", Some("execution-record-1"), Some("execution-journal-1")),
                    status_row("8306.T", "buy", 0.08, "ready_for_apply", "apply_failed", "apply_failed", None, None),
                ],
                vec!["8306.T failed during apply"],
                vec!["8306.T remains apply_failed after apply bridge portfolio-execution-apply-bridge:acct-1:2026-04-25T09:30:00+08:00"]
            ),
            "created_at": "2026-04-25T11:05:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_reconciliation_bridge"];
    assert_eq!(document["reconciliation_status"], "reconciliation_required");
    assert_eq!(document["settled_count"], 1);
    assert_eq!(document["reconciliation_required_count"], 1);
    assert_eq!(
        document["reconciliation_rows"][1]["reconciliation_status"],
        "reconciliation_required"
    );
    assert_eq!(
        document["reconciliation_rows"][1]["requires_manual_follow_up"],
        false
    );
    assert!(
        document["blockers"][0]
            .as_str()
            .expect("blocker should be text")
            .contains("failed during apply"),
        "unexpected output: {output}"
    );
}

// 2026-04-25 CST: Added because P17 must reject unsupported P16 batch states
// instead of inventing reconciliation semantics after the fact.
// Reason: status vocabulary drift is contract corruption at the P16/P17 boundary.
// Purpose: prove unsupported execution_status hard-fails through the CLI.
#[test]
fn security_portfolio_execution_reconciliation_bridge_rejects_unsupported_status() {
    let request = json!({
        "tool": "security_portfolio_execution_reconciliation_bridge",
        "args": {
            "portfolio_execution_status_bridge": build_status_document(
                "mystery_status",
                0,
                0,
                0,
                vec![],
                vec![],
                vec![]
            ),
            "created_at": "2026-04-25T11:10:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .expect("error should be text")
            .contains("unsupported execution status `mystery_status`"),
        "unexpected output: {output}"
    );
}

fn build_status_document(
    execution_status: &str,
    applied_count: usize,
    skipped_hold_count: usize,
    failed_apply_count: usize,
    status_rows: Vec<Value>,
    blockers: Vec<&str>,
    pending_items: Vec<&str>,
) -> Value {
    json!({
        "portfolio_execution_status_bridge_id": "portfolio-execution-status-bridge:acct-1:2026-04-25T10:00:00+08:00",
        "contract_version": "security_portfolio_execution_status_bridge.v1",
        "document_type": "security_portfolio_execution_status_bridge",
        "generated_at": "2026-04-25T10:00:00+08:00",
        "analysis_date": "2026-04-24",
        "account_id": "acct-1",
        "portfolio_execution_apply_bridge_ref": "portfolio-execution-apply-bridge:acct-1:2026-04-25T09:30:00+08:00",
        "portfolio_execution_request_enrichment_ref": "portfolio-execution-request-enrichment:acct-1:2026-04-25T09:00:00+08:00",
        "portfolio_execution_request_package_ref": "portfolio-execution-request-package:acct-1:2026-04-25T08:55:00+08:00",
        "portfolio_execution_preview_ref": "portfolio-execution-preview:acct-1:2026-04-25T08:50:00+08:00",
        "portfolio_allocation_decision_ref": "portfolio-allocation-decision:acct-1:2026-04-25T08:45:00+08:00",
        "status_rows": status_rows,
        "applied_count": applied_count,
        "skipped_hold_count": skipped_hold_count,
        "failed_apply_count": failed_apply_count,
        "pending_item_count": pending_items.len(),
        "execution_status": execution_status,
        "pending_items": pending_items,
        "blockers": blockers,
        "status_rationale": [
            "execution status bridge consumed apply bridge portfolio-execution-apply-bridge:acct-1:2026-04-25T09:30:00+08:00"
        ],
        "status_summary": "test status summary"
    })
}

fn status_row(
    symbol: &str,
    request_action: &str,
    requested_gross_pct: f64,
    enrichment_status: &str,
    apply_status: &str,
    execution_status: &str,
    execution_record_ref: Option<&str>,
    execution_journal_ref: Option<&str>,
) -> Value {
    json!({
        "symbol": symbol,
        "request_action": request_action,
        "requested_gross_pct": requested_gross_pct,
        "enrichment_status": enrichment_status,
        "apply_status": apply_status,
        "execution_status": execution_status,
        "execution_record_ref": execution_record_ref,
        "execution_journal_ref": execution_journal_ref,
        "status_summary": format!("{symbol} test status row")
    })
}
