mod common;

use serde_json::{Value, json};

use crate::common::run_cli_with_json;

// 2026-04-25 CST: Added because P16 was referenced by the public stock bus but
// the status-freeze module was missing from the merged branch.
// Reason: catalog visibility is part of the formal stock boundary contract.
// Purpose: lock the public tool name before restoring the implementation file.
#[test]
fn tool_catalog_includes_security_portfolio_execution_status_bridge() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_portfolio_execution_status_bridge")
    );
}

// 2026-04-25 CST: Added because P16 must freeze P15 apply truth without
// performing reconciliation, replay, or position materialization.
// Reason: a fully applied P15 document should become one auditable P16 status artifact.
// Purpose: prove the CLI route emits batch and row status from the upstream apply document only.
#[test]
fn security_portfolio_execution_status_bridge_freezes_fully_applied_apply_document() {
    let request = json!({
        "tool": "security_portfolio_execution_status_bridge",
        "args": {
            "portfolio_execution_apply_bridge": build_apply_document(
                "applied",
                2,
                0,
                0,
                vec![
                    apply_row("601916.SH", "buy", 0.12, "ready_for_apply", "applied", Some("execution-record-1"), Some("execution-journal-1")),
                    apply_row("8306.T", "buy", 0.08, "ready_for_apply", "applied", Some("execution-record-2"), Some("execution-journal-2")),
                ],
                vec![]
            ),
            "created_at": "2026-04-25T10:00:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_status_bridge"];
    assert_eq!(
        document["document_type"],
        "security_portfolio_execution_status_bridge"
    );
    assert_eq!(
        document["contract_version"],
        "security_portfolio_execution_status_bridge.v1"
    );
    assert_eq!(document["execution_status"], "fully_applied");
    assert_eq!(document["applied_count"], 2);
    assert_eq!(document["skipped_hold_count"], 0);
    assert_eq!(document["failed_apply_count"], 0);
    assert_eq!(document["pending_item_count"], 0);
    assert_eq!(
        document["portfolio_execution_apply_bridge_ref"],
        "portfolio-execution-apply-bridge:acct-1:2026-04-25T09:30:00+08:00"
    );
    assert_eq!(document["status_rows"][0]["execution_status"], "applied");
    assert_eq!(
        document["status_rows"][0]["execution_record_ref"],
        "execution-record-1"
    );
}

// 2026-04-25 CST: Added because P16 must preserve explicit rejection state
// instead of hiding it behind downstream repair or reconciliation assumptions.
// Reason: rejected P15 documents have no runtime writes and must remain auditable.
// Purpose: prove blockers and pending items survive the status-freeze boundary.
#[test]
fn security_portfolio_execution_status_bridge_preserves_rejected_apply_document() {
    let request = json!({
        "tool": "security_portfolio_execution_status_bridge",
        "args": {
            "portfolio_execution_apply_bridge": build_apply_document(
                "rejected",
                0,
                0,
                0,
                vec![],
                vec!["blocked rows are present in the enrichment bundle"]
            ),
            "created_at": "2026-04-25T10:05:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_status_bridge"];
    assert_eq!(document["execution_status"], "rejected");
    assert_eq!(document["pending_item_count"], 1);
    assert!(
        document["pending_items"][0]
            .as_str()
            .expect("pending item should be text")
            .contains("blocked rows"),
        "unexpected status payload: {output}"
    );
    assert_eq!(
        document["blockers"][0],
        "blocked rows are present in the enrichment bundle"
    );
}

fn build_apply_document(
    apply_status: &str,
    applied_count: usize,
    skipped_hold_count: usize,
    failed_apply_count: usize,
    apply_rows: Vec<Value>,
    blockers: Vec<&str>,
) -> Value {
    json!({
        "portfolio_execution_apply_bridge_id": "portfolio-execution-apply-bridge:acct-1:2026-04-25T09:30:00+08:00",
        "contract_version": "security_portfolio_execution_apply_bridge.v1",
        "document_type": "security_portfolio_execution_apply_bridge",
        "generated_at": "2026-04-25T09:30:00+08:00",
        "analysis_date": "2026-04-24",
        "account_id": "acct-1",
        "portfolio_execution_request_enrichment_ref": "portfolio-execution-request-enrichment:acct-1:2026-04-25T09:00:00+08:00",
        "portfolio_execution_request_package_ref": "portfolio-execution-request-package:acct-1:2026-04-25T08:55:00+08:00",
        "portfolio_execution_preview_ref": "portfolio-execution-preview:acct-1:2026-04-25T08:50:00+08:00",
        "portfolio_allocation_decision_ref": "portfolio-allocation-decision:acct-1:2026-04-25T08:45:00+08:00",
        "apply_rows": apply_rows,
        "applied_count": applied_count,
        "skipped_hold_count": skipped_hold_count,
        "failed_apply_count": failed_apply_count,
        "apply_status": apply_status,
        "blockers": blockers,
        "non_atomicity_notice": "this phase does not introduce cross-symbol rollback semantics",
        "apply_rationale": [
            "execution apply bridge consumed enrichment bundle portfolio-execution-request-enrichment:acct-1:2026-04-25T09:00:00+08:00"
        ],
        "apply_summary": "test apply summary"
    })
}

fn apply_row(
    symbol: &str,
    request_action: &str,
    requested_gross_pct: f64,
    enrichment_status: &str,
    apply_status: &str,
    execution_record_ref: Option<&str>,
    execution_journal_ref: Option<&str>,
) -> Value {
    json!({
        "symbol": symbol,
        "request_action": request_action,
        "requested_gross_pct": requested_gross_pct,
        "enrichment_status": enrichment_status,
        "apply_status": apply_status,
        "execution_record_ref": execution_record_ref,
        "execution_journal_ref": execution_journal_ref,
        "apply_summary": format!("{symbol} test apply row")
    })
}
