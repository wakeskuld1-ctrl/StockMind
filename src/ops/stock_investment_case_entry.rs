// 2026-04-15 CST: Added because the approved third-layer stock application architecture
// now needs one explicit investment-case entry above grouped gateways.
// Reason: pre-trade research modules are already grouped, but later AI sessions still
// need one clear formal scenario boundary for case building and evidence preparation.
// Purpose: expose the stable investment-case entry surface without changing
// the existing stock pre-trade implementation modules or touching runtime internals.

pub use super::stock_pre_trade::security_analysis_contextual;
pub use super::stock_pre_trade::security_analysis_fullstack;
// 2026-04-16 CST: Added because the scenario-entry layer now needs to expose the new
// composite scorecard as part of formal investment-case assembly.
// Reason: callers should be able to start from the investment-case boundary without reaching
// into lower grouped modules manually.
// Purpose: surface the composite scorecard on the stable case-entry path.
pub use super::stock_pre_trade::security_composite_scorecard;
// 2026-04-16 CST: Added because step 1 of approved plan A also lands the formal adapter from
// composite scorecard to committee payload on the investment-case entry path.
// Reason: later callers should stay on the scenario boundary instead of importing the lower
// grouped module directly.
// Purpose: expose the bridge beside the composite scorecard on the same stable entry surface.
pub use super::stock_pre_trade::security_composite_committee_payload_adapter;
pub use super::stock_pre_trade::security_decision_evidence_bundle;
pub use super::stock_pre_trade::security_etf_resonance_trust_pack;
pub use super::stock_pre_trade::security_independent_advice;
pub use super::stock_pre_trade::security_risk_gates;
pub use super::stock_pre_trade::technical_consultation_basic;
