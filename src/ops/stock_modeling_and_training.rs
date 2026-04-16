// 2026-04-15 CST: Added because the stock application layer needs one explicit home
// for governed modeling, scorecards, and training artifacts.
// Reason: these modules were discoverable on the flat boundary, but their long-term
// learning role was visually mixed with analysis and execution concerns.
// Purpose: group training-oriented modules without reopening any lower-layer contracts
// or moving runtime persistence ownership.

// 2026-04-16 CST: Added because the current modeling family now needs one explicit
// thin split between request-time scoring and offline model lifecycle management.
// Reason: later AI sessions must stop treating master-score construction and model
// training or registry promotion as one interchangeable capability bucket.
// Purpose: keep the compatibility umbrella while surfacing two thinner internal
// subgroup shells underneath it.
#[path = "stock_online_scoring_and_aggregation.rs"]
pub mod stock_online_scoring_and_aggregation;

#[path = "stock_model_lifecycle.rs"]
pub mod stock_model_lifecycle;

pub use stock_model_lifecycle::security_model_promotion;
pub use stock_model_lifecycle::security_scorecard_model_registry;
pub use stock_model_lifecycle::security_scorecard_refit_run;
pub use stock_model_lifecycle::security_scorecard_training;
pub use stock_online_scoring_and_aggregation::security_feature_snapshot;
pub use stock_online_scoring_and_aggregation::security_forward_outcome;
pub use stock_online_scoring_and_aggregation::security_master_scorecard;
pub use stock_online_scoring_and_aggregation::security_scorecard;
