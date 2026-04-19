// 2026-04-15 CST: Added because the current refactor now treats execution and position
// management as one explicit in-trade closed loop.
// Reason: the older flat boundary made it too easy to mistake position handling for a
// passive snapshot instead of the center of the mid-trade feedback loop.
// Purpose: group execution facts, account-position views, and adjustment flow without
// changing the verified runtime facade mainline.

pub use super::security_account_open_position_snapshot;
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
pub use super::security_record_position_adjustment;
