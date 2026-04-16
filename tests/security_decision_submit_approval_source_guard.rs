use std::fs;

// 2026-04-16 CST: Added because approved scheme A needs a very low-risk guard
// on the approval-chain source of truth before any downstream contract expansion.
// Reason: submit_approval currently rebuilds master_scorecard locally, which lets
// the approval chain drift away from the formal security_master_scorecard mainline.
// Purpose: freeze the requirement that approval uses the formal master-scorecard
// tool path instead of maintaining a private replay builder branch.
#[test]
fn submit_approval_uses_formal_master_scorecard_mainline() {
    let source = fs::read_to_string("src/ops/security_decision_submit_approval.rs")
        .expect("security_decision_submit_approval source should be readable");

    assert!(
        source.contains("SecurityMasterScorecardRequest"),
        "submit_approval should build a formal SecurityMasterScorecardRequest"
    );
    assert!(
        source.contains("security_master_scorecard(&SecurityMasterScorecardRequest"),
        "submit_approval should call the formal security_master_scorecard mainline"
    );
    assert!(
        !source.contains("build_security_master_scorecard_document("),
        "submit_approval should not rebuild the formal master_scorecard locally"
    );
    assert!(
        !source.contains("build_unavailable_security_master_scorecard_document("),
        "submit_approval should not keep a private unavailable-master fallback builder"
    );
}
