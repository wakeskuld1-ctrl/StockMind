// 2026-04-16 CST: Added because the approved third-layer stock application architecture
// still listed one dedicated research-sidecar scenario entry that had not landed in code.
// Reason: the rest of the scenario entry layer is already formalized, so leaving the
// sidecar entry as doc-only drift would keep the boundary half-closed.
// Purpose: expose the research-only scenario shell above the grouped sidecar gateway
// without changing runtime ownership or promoting sidecar flows into the formal mainline.

pub use super::stock_research_sidecar::security_analysis_resonance;
pub use super::stock_research_sidecar::security_committee_vote;
pub use super::stock_research_sidecar::security_history_expansion;
pub use super::stock_research_sidecar::security_shadow_evaluation;
pub use super::stock_research_sidecar::signal_outcome_research;
pub use super::stock_research_sidecar::sync_template_resonance_factors;
