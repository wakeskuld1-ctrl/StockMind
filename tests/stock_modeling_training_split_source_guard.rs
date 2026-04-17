use std::fs;

const MODELING_SPLIT_PLAN_DOC: &str =
    "docs/plans/2026-04-16-stock-modeling-lifecycle-split-design.md";
const HANDOFF_DOC: &str = "docs/AI_HANDOFF.md";

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n")
}

#[test]
fn stock_modeling_gateway_declares_online_and_lifecycle_subgroups() {
    // 2026-04-16 CST: Added because the next boundary split must stop treating
    // online score construction and offline model lifecycle as one interchangeable bucket.
    // Purpose: fail fast when the modeling gateway does not declare the approved
    // thinner subgroup shells.
    let source = fs::read_to_string("src/ops/stock_modeling_and_training.rs")
        .expect("read src/ops/stock_modeling_and_training.rs");
    let normalized = normalize_newlines(&source);

    for required_marker in [
        "#[path = \"stock_online_scoring_and_aggregation.rs\"]",
        "pub mod stock_online_scoring_and_aggregation;",
        "#[path = \"stock_model_lifecycle.rs\"]",
        "pub mod stock_model_lifecycle;",
    ] {
        assert!(
            normalized.contains(required_marker),
            "Modeling split drift detected in src/ops/stock_modeling_and_training.rs: missing `{required_marker}`. Review {MODELING_SPLIT_PLAN_DOC} and {HANDOFF_DOC} before changing this boundary."
        );
    }
}

#[test]
fn online_and_lifecycle_subgroups_keep_module_ownership_separate() {
    // 2026-04-16 CST: Added because the thin split only works if each subgroup
    // owns a distinct responsibility family.
    // Purpose: fail fast when online scoring picks up lifecycle modules or lifecycle
    // picks up request-time scoring modules.
    let online_source = normalize_newlines(
        &fs::read_to_string("src/ops/stock_online_scoring_and_aggregation.rs")
            .expect("read src/ops/stock_online_scoring_and_aggregation.rs"),
    );
    let lifecycle_source = normalize_newlines(
        &fs::read_to_string("src/ops/stock_model_lifecycle.rs")
            .expect("read src/ops/stock_model_lifecycle.rs"),
    );

    for required_online_marker in [
        // 2026-04-17 CST: Updated because the subgroup is a thin shell over the
        // stock boundary, so the compile-true ownership path remains `super::super`.
        // Purpose: guard the real re-export shape instead of forcing a broken local path.
        "pub use super::super::security_feature_snapshot;",
        "pub use super::super::security_forward_outcome;",
        "pub use super::super::security_scorecard;",
        "pub use super::super::security_master_scorecard;",
    ] {
        assert!(
            online_source.contains(required_online_marker),
            "Modeling split drift detected in src/ops/stock_online_scoring_and_aggregation.rs: missing `{required_online_marker}`. Review {MODELING_SPLIT_PLAN_DOC} before changing online scoring ownership."
        );
    }

    for forbidden_online_marker in [
        "security_scorecard_training",
        "security_scorecard_refit_run",
        "security_scorecard_model_registry",
        "security_model_promotion",
        "crate::runtime::",
    ] {
        assert!(
            !online_source.contains(forbidden_online_marker),
            "Modeling split drift detected in src/ops/stock_online_scoring_and_aggregation.rs: forbidden marker `{forbidden_online_marker}` found. Review {MODELING_SPLIT_PLAN_DOC} and {HANDOFF_DOC} before changing online scoring ownership."
        );
    }

    for required_lifecycle_marker in [
        // 2026-04-17 CST: Updated because lifecycle follows the same compile-true
        // thin-shell rule and must re-export from the parent stock boundary.
        // Purpose: stop the guard from enforcing an invalid subgroup-local path.
        "pub use super::super::security_scorecard_model_registry;",
        "pub use super::super::security_scorecard_refit_run;",
        "pub use super::super::security_scorecard_training;",
        "pub use super::super::security_model_promotion;",
    ] {
        assert!(
            lifecycle_source.contains(required_lifecycle_marker),
            "Modeling split drift detected in src/ops/stock_model_lifecycle.rs: missing `{required_lifecycle_marker}`. Review {MODELING_SPLIT_PLAN_DOC} before changing lifecycle ownership."
        );
    }

    for forbidden_lifecycle_marker in [
        "security_feature_snapshot",
        "security_forward_outcome",
        "security_scorecard;",
        "security_master_scorecard",
        "crate::runtime::",
    ] {
        assert!(
            !lifecycle_source.contains(forbidden_lifecycle_marker),
            "Modeling split drift detected in src/ops/stock_model_lifecycle.rs: forbidden marker `{forbidden_lifecycle_marker}` found. Review {MODELING_SPLIT_PLAN_DOC} and {HANDOFF_DOC} before changing lifecycle ownership."
        );
    }
}
