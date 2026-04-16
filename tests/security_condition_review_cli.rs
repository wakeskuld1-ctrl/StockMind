mod common;

use serde_json::json;

use crate::common::run_cli_with_json;

#[test]
fn tool_catalog_includes_security_condition_review() {
    let output = run_cli_with_json("");

    // 2026-04-12 CST: Add a discovery red test for the formal condition review tool,
    // because P8 needs intraperiod review to become a first-class stock capability
    // instead of staying as an implied workflow step.
    // Purpose: lock catalog visibility before implementation wiring begins.
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_condition_review")
    );
}

#[test]
fn security_condition_review_cli_returns_structured_result() {
    let request = json!({
        "tool": "security_condition_review",
        "args": {
            "symbol": "601916.SH",
            "analysis_date": "2026-04-10",
            "decision_ref": "decision:601916.SH:2026-04-10:v1",
            "approval_ref": "approval:601916.SH:2026-04-10:v1",
            "position_plan_ref": "position-plan:601916.SH:2026-04-10:v1",
            "decision_package_path": "artifacts/decision_packages/601916.SH-2026-04-10.json",
            "review_trigger_type": "manual_review",
            "review_trigger_summary": "盘中人工复核：检查原有持仓计划是否仍成立",
            "created_at": "2026-04-12T10:00:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    // 2026-04-12 CST: Add a contract red test for structured condition-review output,
    // because P8 wants review results to be replayable runtime objects rather than
    // conversational notes.
    // Purpose: force the tool to emit a stable review document with follow-up action.
    assert_eq!(output["status"], "ok", "condition review output: {output}");
    assert_eq!(
        output["data"]["condition_review"]["document_type"],
        "security_condition_review"
    );
    assert_eq!(
        output["data"]["condition_review"]["recommended_follow_up_action"],
        "keep_plan"
    );
    assert_eq!(
        output["data"]["condition_review"]["binding"]["position_plan_ref"],
        "position-plan:601916.SH:2026-04-10:v1"
    );
}
