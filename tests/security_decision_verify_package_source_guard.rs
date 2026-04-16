use std::fs;

const GOVERNANCE_PLAN_DOC: &str = "docs/plans/2026-04-15-stock-application-entry-layer-design.md";
const HANDOFF_DOC: &str = "docs/AI_HANDOFF.md";

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n")
}

#[test]
fn verify_package_keeps_post_meeting_chair_binding_visible() {
    // 2026-04-16 CST: Added because the next conservative governance round now
    // closes one downstream contract gap without reopening scorecard logic.
    // Reason: post_meeting_conclusion already carries chair_resolution_ref, but
    // verify_package still needs to read and validate that final governance anchor.
    // Purpose: fail fast when package verification stops treating the chair-bound
    // post-meeting conclusion as a first-class governed artifact.
    let source = fs::read_to_string("src/ops/security_decision_verify_package.rs")
        .expect("read src/ops/security_decision_verify_package.rs");
    let normalized = normalize_newlines(&source);

    for required_marker in [
        "SecurityPostMeetingConclusionDocument",
        "post_meeting_binding_consistent",
        "security_post_meeting_conclusion",
        "chair_resolution_ref",
    ] {
        assert!(
            normalized.contains(required_marker),
            "Governance drift detected in src/ops/security_decision_verify_package.rs: missing `{required_marker}`. Review {GOVERNANCE_PLAN_DOC} and {HANDOFF_DOC} before weakening package verification for post-meeting chair bindings."
        );
    }
}
