// 2026-04-15 CST: Added because the current refactor now treats execution and position
// management as one explicit in-trade closed loop.
// Reason: the older flat boundary made it too easy to mistake position handling for a
// passive snapshot instead of the center of the mid-trade feedback loop.
// Purpose: group execution facts, account-position views, and adjustment flow without
// changing the verified runtime facade mainline.

pub use super::security_account_open_position_snapshot;
// 2026-04-18 CST: Added because the approved intake packet is now the formal
// first document inside the post-open execution-and-position loop.
// Reason: grouping it here keeps the packet aligned with the mid-trade data path
// rather than letting it drift back into pre-trade governance modules.
// Purpose: expose the new intake contract on the explicit in-trade grouped gateway.
pub use super::security_approved_open_position_packet;
// 2026-04-19 CST: Added because P10 now introduces the account-level objective
// shell at the front of the portfolio-core expansion stage.
// Reason: the grouped in-trade gateway should expose the new account objective
// contract beside the other post-open mathematical artifacts.
// Purpose: make the P10 objective builder reachable through the grouped execution gateway.
pub use super::security_account_objective_contract;
// 2026-04-19 CST: Added because Task 3 now introduces the unified replacement
// solver as the first P11 contract in the portfolio-core expansion.
// Reason: the grouped in-trade gateway should expose the replacement plan next
// to the P10 account objective boundary it consumes.
// Purpose: make the P11 replacement plan reachable through the grouped execution gateway.
pub use super::security_portfolio_replacement_plan;
// 2026-04-20 CST: Added because P12 now introduces the final governed
// allocation decision freeze after the implemented replacement plan stage.
// Reason: the grouped in-trade gateway should expose the whole P10 -> P11 ->
// P12 portfolio-core chain on one explicit execution-and-position surface.
// Purpose: make the P12 allocation decision reachable through the grouped gateway.
pub use super::security_portfolio_allocation_decision;
// 2026-04-20 CST: Added because the approved next step after P12 is one
// side-effect-free execution preview bridge.
// Reason: grouping it here keeps the downstream preview on the same explicit
// execution-and-position surface without bypassing the portfolio-core chain.
// Purpose: make the post-P12 preview bridge reachable through the grouped gateway.
pub use super::security_portfolio_execution_preview;
// 2026-04-20 CST: Added because P13 now extends the explicit post-P12 path
// from preview into one formal request-package bridge.
// Reason: grouping it here keeps the request bridge on the same execution-and-
// position surface without implying runtime execution facts.
// Purpose: make the P13 request bridge reachable through the grouped gateway.
pub use super::security_portfolio_execution_request_package;
// 2026-04-21 CST: Added because P14 now extends the explicit post-P12 path
// from request packaging into one enrichment bridge while still stopping short of execution.
// Reason: grouping it here keeps the enrichment bridge on the same execution-and-
// position surface without implying runtime execution facts.
// Purpose: make the P14 request enrichment bridge reachable through the grouped gateway.
pub use super::security_execution_journal;
pub use super::security_execution_record;
pub use super::security_portfolio_execution_request_enrichment;
// 2026-04-21 CST: Added because P15 now extends the explicit post-P12 path
// from request enrichment into one governed apply bridge.
// Reason: grouping it here keeps the new apply stage on the same execution-and-
// position surface while reusing the existing execution-record runtime path.
// Purpose: make the P15 apply bridge reachable through the grouped gateway.
pub use super::security_portfolio_execution_apply_bridge;
// 2026-04-18 CST: Added because Task 2 introduces the live contract object that
// stands between approved intake and later active-position state.
// Reason: the user approved keeping the live contract on the post-open data path.
// Purpose: expose `PositionContract` from the grouped in-trade gateway.
pub use super::security_position_contract;
// 2026-04-18 CST: Added because Task 4 inserts one explicit single-position
// evaluation layer into the in-trade monitoring loop.
// Reason: later account evidence should consume a named evaluation object rather
// than rebuilding actionability directly from snapshots and contracts.
// Purpose: expose the per-position evaluation tool from the grouped in-trade gateway.
pub use super::security_per_position_evaluation;
// 2026-04-18 CST: Added because Task 5 now exposes one standardized monitoring
// evidence package built on top of active positions and evaluations.
// Reason: the in-trade monitoring loop needs a named account-level evidence artifact.
// Purpose: expose the monitoring evidence package from the grouped gateway.
pub use super::security_monitoring_evidence_package;
// 2026-04-19 CST: Added because Task 6 now exposes the account-level capital
// rebase chain on the same post-open management surface.
// Reason: capital events are governed live-state operations, not pre-trade planning artifacts.
// Purpose: expose the capital rebase module from the grouped gateway.
pub use super::security_capital_rebase;
pub use super::security_record_position_adjustment;
