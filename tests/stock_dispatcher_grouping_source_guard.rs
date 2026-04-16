use std::fs;

const STOCK_FLOW_BASELINE_DOC: &str = "docs/plans/2026-04-15-stock-business-flow-baseline.md";
const HANDOFF_DOC: &str = "docs/AI_HANDOFF.md";

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n")
}

#[test]
fn stock_dispatcher_imports_grouped_gateways_for_formal_business_flows() {
    // 2026-04-15 CST: Added because the grouped stock gateway landed first,
    // and the next application-layer step should route dispatcher imports through
    // that grouping instead of reopening the old flat mental model.
    // Purpose: fail fast when stock dispatcher imports drift back to the flat
    // module surface for formal business flows.
    let source =
        fs::read_to_string("src/tools/dispatcher/stock_ops.rs").expect("read stock dispatcher");
    let normalized = normalize_newlines(&source);

    for required_group in [
        "crate::ops::stock::stock_data_pipeline::",
        "crate::ops::stock::stock_pre_trade::",
        "crate::ops::stock::stock_governance_and_positioning::",
        "crate::ops::stock::stock_execution_and_position_management::",
        "crate::ops::stock::stock_post_trade::",
        "crate::ops::stock::stock_modeling_and_training::",
        "crate::ops::stock::stock_research_sidecar::",
    ] {
        assert!(
            normalized.contains(required_group),
            "Dispatcher grouping drift detected in src/tools/dispatcher/stock_ops.rs: missing grouped gateway import `{required_group}`. Review {STOCK_FLOW_BASELINE_DOC} and {HANDOFF_DOC} before changing stock dispatcher structure."
        );
    }

    for forbidden_flat_import in [
        "use crate::ops::stock::import_stock_price_history::",
        "use crate::ops::stock::security_analysis_contextual::",
        "use crate::ops::stock::security_decision_package::",
        "use crate::ops::stock::security_execution_record::",
        "use crate::ops::stock::security_post_trade_review::",
        "use crate::ops::stock::security_scorecard_training::",
        "use crate::ops::stock::signal_outcome_research::",
    ] {
        assert!(
            !normalized.contains(forbidden_flat_import),
            "Dispatcher grouping drift detected in src/tools/dispatcher/stock_ops.rs: forbidden flat import `{forbidden_flat_import}` found. Review {STOCK_FLOW_BASELINE_DOC} and continue from grouped gateways instead of reopening the flat dispatcher surface."
        );
    }
}
