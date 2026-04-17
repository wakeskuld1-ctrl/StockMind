// 2026-04-15 CST: Added because the application-layer split now needs one explicit
// stock data-pipeline view above the unchanged runtime floor.
// Reason: data sufficiency checks and gap-fill tools were previously mixed into the flat
// stock gateway, which made later AI sessions drift back to a fetch-first mental model.
// Purpose: group stock data preparation capabilities without moving the underlying files
// or reopening runtime-layer ownership.

pub use super::import_stock_price_history;
pub use super::security_disclosure_history_backfill;
pub use super::security_disclosure_history_live_backfill;
pub use super::security_external_proxy_backfill;
// 2026-04-17 CST: Added because StockMind phase-1 boundary closeout promotes the
// governed file import bridge onto the same public data-pipeline surface as other
// history backfill tools.
// Purpose: keep import-based proxy history preparation discoverable through the
// grouped stock data pipeline instead of leaving it as a hidden flat module.
pub use super::security_external_proxy_history_import;
pub use super::security_fundamental_history_backfill;
pub use super::security_fundamental_history_live_backfill;
pub use super::stock_analysis_data_guard;
pub use super::stock_training_data_backfill;
pub use super::stock_training_data_coverage_audit;
// 2026-04-16 CST: Added because the governed validation-slice backfill belongs to the
// same local-first data-pipeline stage as stock gap-fill and readiness checks.
// Purpose: keep slice-local replay preparation grouped under the formal data pipeline view.
pub use super::security_real_data_validation_backfill;
pub use super::sync_stock_price_history;
