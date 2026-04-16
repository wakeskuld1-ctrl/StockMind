use std::fs;

// 2026-04-16 CST: Added because approved scheme B needs one very low-risk guard
// on the master-scorecard mainline contract while the wider refactor is still active.
// Reason: public routing can be fixed while the mainline result silently drops either
// composite_scorecard or committee_payload_adapter in a later edit.
// Purpose: freeze the formal return-shape markers without relying on CLI subprocesses.
#[test]
fn master_scorecard_result_keeps_composite_and_committee_payload_markers() {
    let source = fs::read_to_string("src/ops/security_master_scorecard.rs")
        .expect("security_master_scorecard source should be readable");

    assert!(
        source.contains("pub composite_scorecard: SecurityCompositeScorecardDocument"),
        "master scorecard result should keep composite_scorecard on the formal contract"
    );
    assert!(
        source.contains("pub committee_payload_adapter: CommitteePayload"),
        "master scorecard result should keep committee_payload_adapter on the formal contract"
    );
    assert!(
        source.contains(
            "build_master_scorecard_adapter_outputs(&generated_at, &committee_result, &master_scorecard)"
        ),
        "master scorecard mainline should keep attaching the adapter outputs to the return path"
    );
}
