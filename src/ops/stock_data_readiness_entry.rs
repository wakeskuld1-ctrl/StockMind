// 2026-04-15 CST: Added because the approved third-layer stock application architecture
// now needs one explicit data-readiness entry above grouped gateways.
// Reason: grouped capability views alone still do not answer which formal scenario
// starts the local-first data sufficiency and governed gap-fill flow.
// Purpose: expose the stable data-readiness entry surface without reopening
// runtime ownership or changing the underlying stock data-pipeline modules.

pub use super::stock_data_pipeline::import_stock_price_history;
pub use super::stock_data_pipeline::security_disclosure_history_live_backfill;
pub use super::stock_data_pipeline::security_external_proxy_backfill;
pub use super::stock_data_pipeline::security_fundamental_history_live_backfill;
pub use super::stock_data_pipeline::stock_training_data_backfill;
pub use super::stock_data_pipeline::stock_training_data_coverage_audit;
pub use super::stock_data_pipeline::sync_stock_price_history;
