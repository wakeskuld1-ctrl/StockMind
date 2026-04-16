use std::fs;

const STOCK_FLOW_BASELINE_DOC: &str = "docs/plans/2026-04-15-stock-business-flow-baseline.md";
const HANDOFF_DOC: &str = "docs/AI_HANDOFF.md";

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n")
}

fn index_of(haystack: &str, needle: &str) -> usize {
    haystack
        .find(needle)
        .unwrap_or_else(|| panic!("source should contain {needle}"))
}

#[test]
fn stock_catalog_keeps_grouped_business_sections_explicit() {
    // 2026-04-15 CST: Added because grouped gateways have already reached
    // ops and dispatcher, and the public discovery surface should now mirror
    // the same stock business flow.
    // Purpose: fail fast when catalog discovery drifts back into one flat
    // stock list with no grouped business ownership markers.
    let source = fs::read_to_string("src/tools/catalog.rs").expect("read src/tools/catalog.rs");
    let normalized = normalize_newlines(&source);

    for section_label in [
        "// data_pipeline: local-first preparation and governed gap-fill.",
        "// pre_trade: business computation before governance.",
        "// governance_and_positioning: confirm whether and how to act.",
        "// execution_and_position_management: execution facts and current position view.",
        "// post_trade: review and conclusion after the in-trade loop.",
        "// modeling_and_training: long-term governed learning assets.",
    ] {
        assert!(
            normalized.contains(section_label),
            "Catalog grouping drift detected in src/tools/catalog.rs: missing stock section marker `{section_label}`. Review {STOCK_FLOW_BASELINE_DOC} and {HANDOFF_DOC} before changing public stock discovery ordering."
        );
    }
}

#[test]
fn stock_catalog_orders_formal_tools_by_grouped_business_flow() {
    let source = fs::read_to_string("src/tools/catalog.rs").expect("read src/tools/catalog.rs");
    let normalized = normalize_newlines(&source);

    let data_pipeline_idx = index_of(&normalized, "\"import_stock_price_history\"");
    let pre_trade_idx = index_of(&normalized, "\"technical_consultation_basic\"");
    // 2026-04-16 CST: Modified because the public governance anchor is now the
    // formal `security_committee_vote` route instead of the frozen legacy committee tool.
    // Purpose: keep catalog business-order gating pinned to the formal public mainline.
    let governance_idx = index_of(&normalized, "\"security_committee_vote\"");
    let execution_idx = index_of(&normalized, "\"security_execution_record\"");
    let post_trade_idx = index_of(&normalized, "\"security_post_trade_review\"");
    let modeling_idx = index_of(&normalized, "\"security_feature_snapshot\"");

    assert!(
        data_pipeline_idx < pre_trade_idx,
        "Catalog grouping drift detected in src/tools/catalog.rs: data-pipeline discovery should precede pre-trade discovery. Review {STOCK_FLOW_BASELINE_DOC} before changing stock catalog ordering."
    );
    assert!(
        pre_trade_idx < governance_idx,
        "Catalog grouping drift detected in src/tools/catalog.rs: pre-trade discovery should precede governance-and-positioning discovery. Review {STOCK_FLOW_BASELINE_DOC} before changing stock catalog ordering."
    );
    assert!(
        governance_idx < execution_idx,
        "Catalog grouping drift detected in src/tools/catalog.rs: governance-and-positioning discovery should precede execution-and-position-management discovery. Review {STOCK_FLOW_BASELINE_DOC} before changing stock catalog ordering."
    );
    assert!(
        execution_idx < post_trade_idx,
        "Catalog grouping drift detected in src/tools/catalog.rs: execution-and-position-management discovery should precede post-trade discovery. Review {STOCK_FLOW_BASELINE_DOC} before changing stock catalog ordering."
    );
    assert!(
        post_trade_idx < modeling_idx,
        "Catalog grouping drift detected in src/tools/catalog.rs: post-trade discovery should precede modeling-and-training discovery. Review {STOCK_FLOW_BASELINE_DOC} before changing stock catalog ordering."
    );
}
