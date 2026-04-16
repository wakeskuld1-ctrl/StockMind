use std::fs;

const GOVERNANCE_PLAN_DOC: &str = "docs/plans/2026-04-15-stock-application-entry-layer-design.md";
const HANDOFF_DOC: &str = "docs/AI_HANDOFF.md";

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n")
}

#[test]
fn decision_package_keeps_chair_resolution_as_explicit_object_graph_node() {
    // 2026-04-16 CST: Added because the next conservative governance step now
    // promotes the final chair anchor from implicit downstream text into an
    // explicit package object-graph node.
    // Reason: package, revision, and verify should all have one stable field for
    // the final chair decision instead of rediscovering it from nested artifacts.
    // Purpose: fail fast when package contract closure drops the explicit chair node.
    let package_source = fs::read_to_string("src/ops/security_decision_package.rs")
        .expect("read src/ops/security_decision_package.rs");
    let revision_source = fs::read_to_string("src/ops/security_decision_package_revision.rs")
        .expect("read src/ops/security_decision_package_revision.rs");
    let package_normalized = normalize_newlines(&package_source);
    let revision_normalized = normalize_newlines(&revision_source);

    assert!(
        package_normalized.contains("chair_resolution_ref"),
        "Governance drift detected in src/ops/security_decision_package.rs: missing `chair_resolution_ref`. Review {GOVERNANCE_PLAN_DOC} and {HANDOFF_DOC} before weakening the explicit chair node in package object graph."
    );
    assert!(
        revision_normalized.contains("chair_resolution_ref"),
        "Governance drift detected in src/ops/security_decision_package_revision.rs: missing `chair_resolution_ref`. Review {GOVERNANCE_PLAN_DOC} and {HANDOFF_DOC} before weakening chair-node carry-forward during package revision."
    );
}
