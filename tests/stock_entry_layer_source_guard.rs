use std::fs;

const ENTRY_LAYER_PLAN_DOC: &str = "docs/plans/2026-04-15-stock-application-entry-layer-design.md";
const HANDOFF_DOC: &str = "docs/AI_HANDOFF.md";

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n")
}

#[test]
fn stock_boundary_exposes_first_entry_layer_modules() {
    // 2026-04-15 CST: Added because the third-layer stock application entry architecture
    // is now the approved next phase after grouped gateways landed.
    // Purpose: fail fast when the stock boundary forgets to expose the first stable
    // scenario-entry modules for data readiness and investment-case construction.
    let source = fs::read_to_string("src/ops/stock.rs").expect("read src/ops/stock.rs");
    let normalized = normalize_newlines(&source);

    for required_entry in [
        "pub mod stock_data_readiness_entry;",
        "pub mod stock_investment_case_entry;",
        "pub mod stock_governed_action_entry;",
        "pub mod stock_position_management_entry;",
        "pub mod stock_post_trade_learning_entry;",
        "pub mod stock_research_sidecar_entry;",
    ] {
        assert!(
            normalized.contains(required_entry),
            "Entry-layer drift detected in src/ops/stock.rs: missing `{required_entry}`. Review {ENTRY_LAYER_PLAN_DOC} and {HANDOFF_DOC} before changing the stock application entry layout."
        );
    }
}

#[test]
fn first_entry_layer_modules_stay_above_grouped_gateways() {
    // 2026-04-15 CST: Added because the new entry layer is only an orchestration layer
    // above grouped gateways, not a place to reopen runtime-facing imports.
    // Purpose: fail fast when future entry modules bypass grouped gateways or reach
    // directly into runtime modules.
    let guarded_sources = [
        (
            "src/ops/stock_data_readiness_entry.rs",
            "stock_data_pipeline::",
            "crate::runtime::",
        ),
        (
            "src/ops/stock_investment_case_entry.rs",
            "stock_pre_trade::",
            "crate::runtime::",
        ),
        (
            "src/ops/stock_governed_action_entry.rs",
            "stock_governance_and_positioning::",
            "crate::runtime::",
        ),
        (
            "src/ops/stock_position_management_entry.rs",
            "stock_execution_and_position_management::",
            "crate::runtime::",
        ),
        (
            "src/ops/stock_post_trade_learning_entry.rs",
            "stock_post_trade::",
            "crate::runtime::",
        ),
        (
            "src/ops/stock_research_sidecar_entry.rs",
            "stock_research_sidecar::",
            "crate::runtime::",
        ),
    ];

    for (path, required_gateway_marker, forbidden_runtime_marker) in guarded_sources {
        let source = fs::read_to_string(path).unwrap_or_else(|_| panic!("read {path}"));
        let normalized = normalize_newlines(&source);
        assert!(
            normalized.contains(required_gateway_marker),
            "Entry-layer drift detected in {path}: entry modules must compose grouped gateways such as `{required_gateway_marker}`. Review {ENTRY_LAYER_PLAN_DOC} before changing this dependency shape."
        );
        assert!(
            !normalized.contains(forbidden_runtime_marker),
            "Entry-layer drift detected in {path}: direct runtime import `{forbidden_runtime_marker}` is forbidden. Review {ENTRY_LAYER_PLAN_DOC} and {HANDOFF_DOC} before changing entry-layer boundaries."
        );
    }
}

#[test]
fn research_sidecar_entry_keeps_sidecar_scope_explicit() {
    // 2026-04-16 CST: Added because the design baseline already reserves one dedicated
    // sidecar entry above the grouped research gateway.
    // Reason: the boundary closeout should not leave the sidecar entry as a doc-only idea
    // while the rest of the scenario entry layer is already formalized.
    // Purpose: require the sidecar entry to stay a thin research-only composition shell.
    let source = fs::read_to_string("src/ops/stock_research_sidecar_entry.rs")
        .expect("read src/ops/stock_research_sidecar_entry.rs");
    let normalized = normalize_newlines(&source);

    for required_marker in [
        "stock_research_sidecar::security_analysis_resonance",
        "stock_research_sidecar::security_committee_vote",
        "stock_research_sidecar::signal_outcome_research",
        "stock_research_sidecar::sync_template_resonance_factors",
    ] {
        assert!(
            normalized.contains(required_marker),
            "Entry-layer drift detected in src/ops/stock_research_sidecar_entry.rs: missing sidecar export `{required_marker}`. Review {ENTRY_LAYER_PLAN_DOC} and {HANDOFF_DOC} before changing the sidecar entry scope."
        );
    }

    assert!(
        !normalized.contains("crate::runtime::"),
        "Entry-layer drift detected in src/ops/stock_research_sidecar_entry.rs: direct runtime imports remain forbidden. Review {ENTRY_LAYER_PLAN_DOC} and {HANDOFF_DOC} before changing entry-layer boundaries."
    );
}

#[test]
fn post_trade_learning_entry_keeps_training_composition_explicit() {
    // 2026-04-16 CST: Added because the final formal entry stage now needs to show
    // that post-trade closure and governed learning are one scenario boundary.
    // 2026-04-16 CST: Updated because the modeling family is now being split into
    // request-time scoring and offline lifecycle responsibilities.
    // Purpose: fail fast when post-trade learning entry stops composing the thinner
    // scoring plus lifecycle subgroups and drifts back into one blended modeling bucket.
    let source = fs::read_to_string("src/ops/stock_post_trade_learning_entry.rs")
        .expect("read src/ops/stock_post_trade_learning_entry.rs");
    let normalized = normalize_newlines(&source);

    assert!(
        normalized.contains("stock_online_scoring_and_aggregation::"),
        "Entry-layer drift detected in src/ops/stock_post_trade_learning_entry.rs: the post-trade learning scenario must compose `stock_online_scoring_and_aggregation::`. Review {ENTRY_LAYER_PLAN_DOC} and {HANDOFF_DOC} before changing this boundary."
    );
    assert!(
        normalized.contains("stock_model_lifecycle::"),
        "Entry-layer drift detected in src/ops/stock_post_trade_learning_entry.rs: the post-trade learning scenario must compose `stock_model_lifecycle::`. Review {ENTRY_LAYER_PLAN_DOC} and {HANDOFF_DOC} before changing this boundary."
    );
    assert!(
        !normalized.contains("crate::runtime::"),
        "Entry-layer drift detected in src/ops/stock_post_trade_learning_entry.rs: direct runtime imports remain forbidden. Review {ENTRY_LAYER_PLAN_DOC} and {HANDOFF_DOC} before changing entry-layer boundaries."
    );
}

#[test]
fn governed_action_entry_keeps_governance_surface_complete() {
    // 2026-04-16 CST: Added because scheme A now closes the governed-action entry
    // surface instead of leaving governance-only formal objects discoverable only on
    // the grouped gateway below it.
    // Reason: the approved entry-layer design defines governed action as the formal
    // scenario shell for committee, package, approval, and position-planning flow.
    // Purpose: fail fast when the governed-action entry omits formal governance
    // artifacts and drifts back into a partial pass-through.
    let source = fs::read_to_string("src/ops/stock_governed_action_entry.rs")
        .expect("read src/ops/stock_governed_action_entry.rs");
    let normalized = normalize_newlines(&source);

    for required_marker in [
        "stock_governance_and_positioning::security_decision_briefing",
        "stock_governance_and_positioning::security_decision_card",
        "stock_governance_and_positioning::security_position_plan_record",
    ] {
        assert!(
            normalized.contains(required_marker),
            "Entry-layer drift detected in src/ops/stock_governed_action_entry.rs: missing governed-action export `{required_marker}`. Review {ENTRY_LAYER_PLAN_DOC} and {HANDOFF_DOC} before changing the formal governed-action surface."
        );
    }
}
