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
pub use super::security_execution_journal;
pub use super::security_execution_record;
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
