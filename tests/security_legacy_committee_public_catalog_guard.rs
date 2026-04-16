use std::fs;

const LEGACY_PLAN_DOC: &str =
    "docs/plans/2026-04-16-security-legacy-committee-governance-design.md";
const HANDOFF_DOC: &str = "docs/AI_HANDOFF.md";

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n")
}

#[test]
fn public_stock_catalog_promotes_formal_committee_mainline_only() {
    // 2026-04-16 CST: Added because the last legacy-governance gap was no longer
    // inside src/ops or dispatcher labeling, but in the public catalog itself.
    // Reason: leaving the frozen legacy committee in public discovery would still
    // let later sessions or operators treat it as a first-class formal tool.
    // Purpose: fail fast when catalog discovery stops reflecting the approved
    // public mainline `security_committee_vote -> security_chair_resolution`.
    let catalog = normalize_newlines(
        &fs::read_to_string("src/tools/catalog.rs").expect("read src/tools/catalog.rs"),
    );
    let handoff =
        normalize_newlines(&fs::read_to_string(HANDOFF_DOC).expect("read docs/AI_HANDOFF.md"));

    assert!(
        catalog.contains("\"security_committee_vote\""),
        "Public-surface governance drift detected in src/tools/catalog.rs: missing formal `security_committee_vote`. Review {LEGACY_PLAN_DOC} and {HANDOFF_DOC} before changing public committee discovery."
    );
    assert!(
        !catalog.contains("\"security_decision_committee\""),
        "Public-surface governance drift detected in src/tools/catalog.rs: frozen legacy `security_decision_committee` must not remain in public catalog. Review {LEGACY_PLAN_DOC} and {HANDOFF_DOC} before changing public committee discovery."
    );
    assert!(
        !catalog.contains("\"security_committee_member_agent\""),
        "Public-surface governance drift detected in src/tools/catalog.rs: internal or legacy seat-agent route must not remain in public catalog. Review {LEGACY_PLAN_DOC} and {HANDOFF_DOC} before changing public committee discovery."
    );
    assert!(
        handoff.contains("legacy committee public discovery"),
        "Public-surface governance drift detected in {HANDOFF_DOC}: missing public-discovery closeout rule. Review {LEGACY_PLAN_DOC} before changing public committee discovery."
    );
}

#[test]
fn dispatcher_may_keep_legacy_committee_route_but_catalog_must_not() {
    // 2026-04-16 CST: Added because the approved closeout keeps the legacy route
    // callable for compatibility, but no longer discoverable on the formal public surface.
    // Purpose: lock that split explicitly so later sessions cannot re-couple
    // public catalog exposure and compatibility dispatcher retention.
    let dispatcher = normalize_newlines(
        &fs::read_to_string("src/tools/dispatcher.rs").expect("read src/tools/dispatcher.rs"),
    );
    let catalog = normalize_newlines(
        &fs::read_to_string("src/tools/catalog.rs").expect("read src/tools/catalog.rs"),
    );

    assert!(
        dispatcher.contains("\"security_decision_committee\" =>"),
        "Public-surface governance drift detected in src/tools/dispatcher.rs: compatibility dispatcher route for legacy committee unexpectedly disappeared. Review {LEGACY_PLAN_DOC} before changing the approved closeout shape."
    );
    assert!(
        !catalog.contains("\"security_decision_committee\""),
        "Public-surface governance drift detected: legacy committee may remain in dispatcher for compatibility, but it must not reappear in public catalog. Review {LEGACY_PLAN_DOC} and {HANDOFF_DOC} before changing this split."
    );
}
