// 2026-04-16 CST: Added because the approved third-layer stock application architecture
// now needs one explicit position-management scenario entry above grouped gateways.
// Reason: execution recording, open-position reconstruction, and adjustment handling
// already form one closed loop in the design baseline, but callers still lacked one
// stable entry boundary for that loop.
// Purpose: expose the formal in-trade scenario shell without changing grouped gateway
// ownership or allowing direct runtime imports at the entry layer.

pub use super::stock_execution_and_position_management::security_account_open_position_snapshot;
pub use super::stock_execution_and_position_management::security_execution_journal;
pub use super::stock_execution_and_position_management::security_execution_record;
pub use super::stock_execution_and_position_management::security_record_position_adjustment;
