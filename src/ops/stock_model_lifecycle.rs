// 2026-04-16 CST: Added because the existing modeling family mixed request-time
// scoring and offline lifecycle work into one wide grouped bucket.
// Reason: later AI sessions need one stable place for training, registry, refit,
// and promotion responsibilities that must stay separate from online scoring.
// Purpose: expose model-lifecycle modules without dragging request-time score
// builders back into lifecycle ownership.

// 2026-04-17 CST: Updated because the earlier guard-driven local `super::...`
// rewrite broke the real module ownership chain for the standalone repo.
// Reason: lifecycle remains a thin subgroup shell that re-exports stock-level
// lifecycle modules from the parent boundary, not locally defined children.
// Purpose: restore the compile-true lifecycle surface before guard cleanup.
pub use super::super::security_model_promotion;
pub use super::super::security_scorecard_model_registry;
pub use super::super::security_scorecard_refit_run;
pub use super::super::security_scorecard_training;
