// 2026-04-15 CST: Added because exploratory stock capabilities should remain visible
// during the application-layer split without being mistaken for the formal business mainline.
// Reason: resonance, shadow evaluation, and signal research are still valuable, but they
// should read as sidecar research rather than parallel production flow.
// Purpose: group research-only modules inside the stock boundary while keeping their
// semantics separate from the formal pre-trade and in-trade groupings.

pub use super::security_analysis_resonance;
pub use super::security_committee_vote;
pub use super::security_history_expansion;
pub use super::security_shadow_evaluation;
pub use super::signal_outcome_research;
pub use super::sync_template_resonance_factors;
