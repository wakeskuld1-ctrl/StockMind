// 2026-04-15 CST: Added because the stock application layer now needs one business
// grouping that bridges pre-trade conclusions into governed action.
// Reason: governance and position planning previously looked like unrelated flat tools,
// which hid the fact that they are the formal transition from analysis to action.
// Purpose: keep approval, package, committee, and position-plan capabilities together
// without moving the original implementation modules.

pub use super::security_approval_brief_signature;
pub use super::security_chair_resolution;
pub use super::security_condition_review;
pub use super::security_decision_approval_bridge;
pub use super::security_decision_approval_brief;
pub use super::security_decision_briefing;
pub use super::security_decision_card;
// 2026-04-16 CST: Added because the legacy committee chain still needs one
// temporary grouped-gateway export while dispatcher compatibility is being kept
// alive during the refactor.
// Reason: removing this export right now would widen the blast radius across the
// current application layer, but leaving it unlabeled would make it too easy for
// later sessions to mistake it for the formal committee mainline.
// Purpose: keep this grouped export explicitly marked as legacy-facing surface
// until callers are fully migrated to `security_committee_vote` plus chair flow.
pub use super::security_decision_committee;
pub use super::security_decision_package;
pub use super::security_decision_package_revision;
pub use super::security_decision_submit_approval;
pub use super::security_decision_verify_package;
pub use super::security_portfolio_position_plan;
pub use super::security_position_plan;
pub use super::security_position_plan_record;
