// 2026-04-16 CST: Added because the approved third-layer stock application architecture
// now needs one explicit post-trade learning scenario entry above grouped gateways.
// Reason: post-trade review and governed model learning belong to one formal feedback
// stage in the stock mainline, but the entry layer still stopped before that closure.
// Purpose: expose the formal feedback-stage shell by composing post-trade and modeling
// gateways while keeping runtime access below the entry layer.

pub use super::stock_post_trade::security_post_meeting_conclusion;
pub use super::stock_post_trade::security_post_trade_review;
pub use super::stock_post_trade::security_record_post_meeting_conclusion;

// 2026-04-16 CST: Added because the approved design defines review closure and learning
// closure as one scenario boundary, not two unrelated entry surfaces.
// 2026-04-16 CST: Updated because the old modeling bucket is now being split into
// thinner online scoring and offline lifecycle responsibilities.
// Purpose: keep post-trade learning entry aligned with the new split while preserving
// the existing formal scenario surface.
pub use super::stock_modeling_and_training::stock_model_lifecycle::security_model_promotion;
pub use super::stock_modeling_and_training::stock_model_lifecycle::security_scorecard_model_registry;
pub use super::stock_modeling_and_training::stock_model_lifecycle::security_scorecard_refit_run;
pub use super::stock_modeling_and_training::stock_model_lifecycle::security_scorecard_training;
pub use super::stock_modeling_and_training::stock_online_scoring_and_aggregation::security_feature_snapshot;
pub use super::stock_modeling_and_training::stock_online_scoring_and_aggregation::security_forward_outcome;
pub use super::stock_modeling_and_training::stock_online_scoring_and_aggregation::security_master_scorecard;
pub use super::stock_modeling_and_training::stock_online_scoring_and_aggregation::security_scorecard;
