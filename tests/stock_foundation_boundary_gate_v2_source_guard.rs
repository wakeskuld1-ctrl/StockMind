use std::fs;
use std::path::Path;

const SPLIT_MANIFEST_DOC: &str = "docs/plans/2026-04-15-stock-foundation-split-manifest-design.md";
const GATE_V2_PLAN_DOC: &str = "docs/plans/2026-04-15-stock-foundation-boundary-gate-v2-design.md";
const HANDOFF_DOC: &str = "docs/AI_HANDOFF.md";

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n")
}

#[test]
fn stock_entry_and_grouped_shells_do_not_reach_foundation_or_hold_zone() {
    // 2026-04-15 CST: Added because the next architecture risk is no longer only
    // direct stock-business import drift, but also thin shell layers quietly
    // reaching into foundation analytics or shared/runtime hold-zone files.
    // Purpose: keep entry and grouped shell files as stock-only orchestration
    // surfaces instead of turning them into new cross-block dependency points.
    let guarded_sources = [
        "src/ops/stock_data_readiness_entry.rs",
        "src/ops/stock_investment_case_entry.rs",
        "src/ops/stock_governed_action_entry.rs",
        "src/ops/stock_data_pipeline.rs",
        "src/ops/stock_pre_trade.rs",
        "src/ops/stock_governance_and_positioning.rs",
        "src/ops/stock_execution_and_position_management.rs",
        "src/ops/stock_post_trade.rs",
        "src/ops/stock_modeling_and_training.rs",
        "src/ops/stock_research_sidecar.rs",
    ];

    let forbidden_markers = [
        "crate::ops::linear_regression",
        "crate::ops::logistic_regression",
        "crate::ops::stat_summary",
        "crate::ops::correlation_analysis",
        "crate::ops::trend_analysis",
        "crate::ops::cluster_kmeans",
        "crate::ops::decision_assistant",
        "crate::ops::foundation::",
        "crate::tools::catalog",
        "crate::tools::dispatcher",
        "crate::tools::contracts",
        "crate::runtime::",
    ];

    for path in guarded_sources {
        let source = fs::read_to_string(path).unwrap_or_else(|_| panic!("read {path}"));
        let normalized = normalize_newlines(&source);

        assert!(
            normalized.contains("pub use super::"),
            "Boundary gate v2 drift detected in {path}: shell files should remain thin `pub use super::...` composition surfaces. Review {GATE_V2_PLAN_DOC} and {SPLIT_MANIFEST_DOC} before changing shell responsibilities.",
        );

        for forbidden in forbidden_markers {
            assert!(
                !normalized.contains(forbidden),
                "Boundary gate v2 drift detected in {path}: forbidden cross-block dependency `{forbidden}` reached a Stock shell layer. Review {GATE_V2_PLAN_DOC}, {SPLIT_MANIFEST_DOC}, and {HANDOFF_DOC} before changing this boundary.",
            );
        }
    }
}

#[test]
fn shared_and_runtime_hold_zone_files_remain_present_and_shared() {
    // 2026-04-15 CST: Added because the user explicitly wants the current split to
    // prevent later AI sessions from misclassifying shared entry infrastructure or
    // the securities runtime kernel as foundation-analytics ownership.
    // Purpose: fail fast if the hold-zone files disappear from their shared paths
    // or stop advertising the current shared/runtime boundary markers.
    for required_path in [
        "src/tools/catalog.rs",
        "src/tools/contracts.rs",
        "src/tools/dispatcher.rs",
        "src/tools/dispatcher/stock_ops.rs",
        "src/runtime/mod.rs",
        "src/runtime/formal_security_runtime_registry.rs",
        "src/runtime/security_execution_store.rs",
        "src/runtime/stock_history_store.rs",
        "src/runtime/security_external_proxy_store.rs",
        "src/runtime/security_fundamental_history_store.rs",
        "src/runtime/security_disclosure_history_store.rs",
        "src/runtime/security_resonance_store.rs",
        "src/runtime/signal_outcome_store.rs",
    ] {
        assert!(
            Path::new(required_path).exists(),
            "Hold-zone drift detected: required shared/runtime path `{required_path}` is missing. Review {SPLIT_MANIFEST_DOC} and {HANDOFF_DOC} before moving shared or runtime files."
        );
    }

    let catalog = normalize_newlines(
        &fs::read_to_string("src/tools/catalog.rs").expect("read src/tools/catalog.rs"),
    );
    assert!(
        catalog.contains("pub const FOUNDATION_TOOL_NAMES: &[&str] = &["),
        "Hold-zone drift detected in src/tools/catalog.rs: foundation tool surface marker is missing. Review {SPLIT_MANIFEST_DOC} before changing shared catalog ownership."
    );
    assert!(
        catalog.contains("pub const STOCK_TOOL_NAMES: &[&str] = &["),
        "Hold-zone drift detected in src/tools/catalog.rs: stock tool surface marker is missing. Review {SPLIT_MANIFEST_DOC} before changing shared catalog ownership."
    );

    let contracts = normalize_newlines(
        &fs::read_to_string("src/tools/contracts.rs").expect("read src/tools/contracts.rs"),
    );
    assert!(
        contracts.contains("crate::ops::foundation::knowledge_bundle::KnowledgeBundle"),
        "Hold-zone drift detected in src/tools/contracts.rs: the current shared foundation-side contract marker is missing. Review {SPLIT_MANIFEST_DOC} before reclassifying this file."
    );
    assert!(
        contracts.contains("crate::ops::stock::security_decision_briefing::PositionPlan"),
        "Hold-zone drift detected in src/tools/contracts.rs: the current shared stock-side contract marker is missing. Review {SPLIT_MANIFEST_DOC} before reclassifying this file."
    );

    let runtime_mod = normalize_newlines(
        &fs::read_to_string("src/runtime/mod.rs").expect("read src/runtime/mod.rs"),
    );
    assert!(
        runtime_mod.contains("pub mod formal_security_runtime_registry;"),
        "Hold-zone drift detected in src/runtime/mod.rs: the formal runtime registry must remain visible from the shared runtime root. Review {SPLIT_MANIFEST_DOC} before moving runtime ownership."
    );
    assert!(
        runtime_mod.contains("pub mod security_execution_store;"),
        "Hold-zone drift detected in src/runtime/mod.rs: governed securities runtime stores must remain grouped under the shared runtime root. Review {SPLIT_MANIFEST_DOC} before moving runtime files."
    );

    let registry = normalize_newlines(
        &fs::read_to_string("src/runtime/formal_security_runtime_registry.rs")
            .expect("read src/runtime/formal_security_runtime_registry.rs"),
    );
    for required_method in [
        "pub fn execution_store_db_path()",
        "pub fn stock_history_db_path()",
        "pub fn external_proxy_db_path()",
        "pub fn fundamental_history_db_path()",
        "pub fn disclosure_history_db_path()",
        "pub fn resonance_db_path()",
        "pub fn signal_outcome_db_path()",
    ] {
        assert!(
            registry.contains(required_method),
            "Hold-zone drift detected in src/runtime/formal_security_runtime_registry.rs: required runtime path method `{required_method}` is missing. Review {SPLIT_MANIFEST_DOC} before changing runtime ownership."
        );
    }
}

#[test]
fn split_manifest_and_gate_v2_baseline_are_recorded_in_docs() {
    // 2026-04-15 CST: Added because the user wants later AI sessions to be blocked
    // by documentation, handoff, and tests together instead of relying on memory.
    // Purpose: require the split manifest and gate-v2 design baseline to stay present
    // before anyone changes ownership or boundary rules again.
    let manifest = normalize_newlines(
        &fs::read_to_string(SPLIT_MANIFEST_DOC)
            .unwrap_or_else(|_| panic!("read {SPLIT_MANIFEST_DOC}")),
    );
    assert!(
        manifest.contains("## Shared / Runtime Hold Zone"),
        "Boundary gate v2 drift detected in {SPLIT_MANIFEST_DOC}: the hold-zone section is missing."
    );
    assert!(
        manifest.contains("## Adapter 规则"),
        "Boundary gate v2 drift detected in {SPLIT_MANIFEST_DOC}: the adapter rule section is missing."
    );

    let gate_v2 = normalize_newlines(
        &fs::read_to_string(GATE_V2_PLAN_DOC).unwrap_or_else(|_| panic!("read {GATE_V2_PLAN_DOC}")),
    );
    assert!(
        gate_v2.contains("采用 `方案 B`。"),
        "Boundary gate v2 drift detected in {GATE_V2_PLAN_DOC}: the approved implementation option marker is missing."
    );
    assert!(
        gate_v2.contains("Shared/Runtime Hold-Zone Guard"),
        "Boundary gate v2 drift detected in {GATE_V2_PLAN_DOC}: the hold-zone guard section is missing."
    );

    let handoff =
        normalize_newlines(&fs::read_to_string(HANDOFF_DOC).expect("read docs/AI_HANDOFF.md"));
    assert!(
        handoff.contains("Stock/Foundation Split Manifest Frozen"),
        "Boundary gate v2 drift detected in {HANDOFF_DOC}: the split manifest handoff section is missing."
    );
    assert!(
        handoff.contains("Stock/Foundation Boundary Gate V2"),
        "Boundary gate v2 drift detected in {HANDOFF_DOC}: the gate-v2 handoff section is missing."
    );
}
