use crate::ops::stock::security_decision_committee::{
    SecurityDecisionCommitteeError, SecurityDecisionCommitteeRequest,
    SecurityDecisionCommitteeResult, security_decision_committee,
};

// 2026-04-16 CST: Added because the user explicitly required the old committee
// chain to shrink behind one controlled compatibility owner before the full
// consumer migration is finished.
// Reason: multiple business modules were still importing the legacy committee
// module directly, which kept the refactor blast radius wide and made future
// retirement harder to coordinate.
// Purpose: centralize the only approved business-layer invocation owner for the
// frozen legacy committee flow, so later migrations can replace one adapter
// instead of touching every downstream module again.
pub use crate::ops::stock::security_decision_committee::{
    SecurityDecisionCommitteeError as LegacySecurityDecisionCommitteeError,
    SecurityDecisionCommitteeRequest as LegacySecurityDecisionCommitteeRequest,
    SecurityDecisionCommitteeResult as LegacySecurityDecisionCommitteeResult,
};

// 2026-04-16 CST: Added because phase-2 governance needs one explicit adapter
// seam between the frozen legacy committee implementation and the still-live
// downstream business modules.
// Reason: this keeps existing downstream contracts stable while preventing new
// business files from importing the legacy module directly.
// Purpose: make later retirement work converge on this adapter instead of
// spreading direct legacy invocations across chair, submit, scorecard, and
// position-plan code again.
pub fn run_security_decision_committee_legacy_compat(
    request: &SecurityDecisionCommitteeRequest,
) -> Result<SecurityDecisionCommitteeResult, SecurityDecisionCommitteeError> {
    security_decision_committee(request)
}
