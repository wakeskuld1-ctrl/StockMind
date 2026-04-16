// 2026-04-15 CST: Added because the application-layer regrouping needs a dedicated
// pre-trade view that stays separate from data supply and in-trade execution.
// Reason: the flat stock boundary visually over-weighted analysis while still hiding
// where pre-trade responsibilities should stop.
// Purpose: make research, analysis, evidence assembly, and risk prechecks discoverable
// as one business-stage grouping without changing existing module paths.

pub use super::security_analysis_contextual;
pub use super::security_analysis_fullstack;
// 2026-04-16 CST: Added because the first composite scorecard landing belongs to the pre-trade
// investment-case stage, not runtime execution.
// Reason: we are formalizing a business-layer synthesis object before governance rewiring.
// Purpose: make the composite scorecard discoverable on the grouped pre-trade gateway.
pub use super::security_composite_scorecard;
// 2026-04-16 CST: Added because plan A step 1 now includes the governed adapter that turns
// the composite scorecard into a formal committee payload.
// Reason: pre-trade callers should discover the bridge on the same grouped gateway as the
// composite scorecard itself.
// Purpose: keep investment-case assembly on one stable pre-trade surface.
pub use super::security_composite_committee_payload_adapter;
pub use super::security_decision_evidence_bundle;
pub use super::security_etf_resonance_trust_pack;
pub use super::security_independent_advice;
pub use super::security_risk_gates;
pub use super::technical_consultation_basic;
