// 2026-04-16 CST: Added because the existing modeling family mixed request-time
// scoring and offline lifecycle work into one wide grouped bucket.
// Reason: post-trade learning and later public-surface cleanup need one thinner
// online scoring subgroup that only represents scored artifact construction.
// Purpose: expose request-time score construction and aggregation modules without
// pulling in registry, refit, or training ownership.

// 2026-04-17 CST: Updated because the previous guard-driven rewrite to local
// `super::...` imports broke the actual subgroup compile boundary in this repo.
// Reason: this subgroup is still a thin grouping shell over the parent stock
// boundary, so the re-export must keep pointing at the flat stock module layer.
// Purpose: restore the real ownership path before tightening the source guard.
pub use super::super::security_feature_snapshot;
pub use super::super::security_forward_outcome;
pub use super::super::security_master_scorecard;
pub use super::super::security_scorecard;
