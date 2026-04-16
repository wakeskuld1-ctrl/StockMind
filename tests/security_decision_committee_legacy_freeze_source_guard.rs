use std::fs;

const LEGACY_COMMITTEE_PLAN_DOC: &str =
    "docs/plans/2026-04-16-security-legacy-committee-governance-design.md";
const HANDOFF_DOC: &str = "docs/AI_HANDOFF.md";
const README_DOC: &str = "README.md";
const LEGACY_FILE: &str = "src/ops/security_decision_committee.rs";

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n")
}

fn stable_fnv1a64(input: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[test]
fn security_decision_committee_stays_frozen_as_legacy_compatibility_zone() {
    // 2026-04-16 CST: Added because the user explicitly required the old
    // committee chain to become a cargo-test-intercepted freeze zone.
    // Purpose: fail fast when later sessions casually edit the legacy committee
    // file instead of continuing new work on the formal committee mainline.
    let source =
        fs::read_to_string(LEGACY_FILE).expect("read src/ops/security_decision_committee.rs");
    let normalized = normalize_newlines(&source);

    assert!(
        normalized.contains("LEGACY_COMMITTEE_ADAPTER_FROZEN"),
        "Legacy governance drift detected in {LEGACY_FILE}: missing `LEGACY_COMMITTEE_ADAPTER_FROZEN`. Review {LEGACY_COMMITTEE_PLAN_DOC} and {HANDOFF_DOC} before editing the legacy committee file."
    );

    let readme = normalize_newlines(&fs::read_to_string(README_DOC).expect("read README.md"));
    assert!(
        readme.contains("Committee Governance Freeze"),
        "Legacy governance drift detected in {README_DOC}: the committee governance freeze section is missing."
    );

    let handoff =
        normalize_newlines(&fs::read_to_string(HANDOFF_DOC).expect("read docs/AI_HANDOFF.md"));
    assert!(
        handoff.contains("Security Decision Committee Legacy Freeze"),
        "Legacy governance drift detected in {HANDOFF_DOC}: the legacy committee freeze handoff section is missing."
    );

    let fingerprint = stable_fnv1a64(&normalized);
    assert_eq!(
        fingerprint, 16227485453661136938,
        "Legacy governance drift detected in {LEGACY_FILE}: file content changed. Review {LEGACY_COMMITTEE_PLAN_DOC} and {HANDOFF_DOC}, then update this freeze gate only as part of an approved migration."
    );
}
