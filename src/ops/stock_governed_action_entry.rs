// 2026-04-15 CST: Added because the approved third-layer stock application architecture
// now needs one explicit governed-action entry above grouped gateways.
// Reason: governance capabilities already exist, but later AI sessions still need one
// stable formal scenario boundary for committee review, package flow, approval, and
// position-plan preparation after the investment case is complete.
// Purpose: expose the governed-action mainline through a thin scenario-entry shell
// without reopening runtime ownership or changing the underlying governance modules.

pub use super::stock_governance_and_positioning::security_approval_brief_signature;
pub use super::stock_governance_and_positioning::security_chair_resolution;
pub use super::stock_governance_and_positioning::security_condition_review;
pub use super::stock_governance_and_positioning::security_decision_approval_bridge;
pub use super::stock_governance_and_positioning::security_decision_approval_brief;
// 2026-04-16 CST: Added because scheme A now closes the governed-action entry
// surface into a complete formal scenario shell rather than a partial gateway view.
// Reason: decision briefing, decision card, and position-plan record remain formal
// governance artifacts in the same approval journey and should not require callers
// to drop below the entry layer.
// Purpose: keep governed-action callers on one explicit formal boundary without
// changing governance implementation ownership or touching scorecard logic.
pub use super::stock_governance_and_positioning::security_decision_briefing;
pub use super::stock_governance_and_positioning::security_decision_card;
pub use super::stock_governance_and_positioning::security_decision_committee;
pub use super::stock_governance_and_positioning::security_decision_package;
pub use super::stock_governance_and_positioning::security_decision_package_revision;
pub use super::stock_governance_and_positioning::security_decision_submit_approval;
pub use super::stock_governance_and_positioning::security_decision_verify_package;
pub use super::stock_governance_and_positioning::security_portfolio_position_plan;
pub use super::stock_governance_and_positioning::security_position_plan;
pub use super::stock_governance_and_positioning::security_position_plan_record;
