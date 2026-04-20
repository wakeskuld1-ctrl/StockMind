pub const STOCK_TOOL_NAMES: &[&str] = &[
    // data_pipeline: local-first preparation and governed gap-fill.
    "import_stock_price_history",
    "sync_stock_price_history",
    "security_fundamental_history_backfill",
    "security_fundamental_history_live_backfill",
    "security_disclosure_history_backfill",
    "security_disclosure_history_live_backfill",
    "security_external_proxy_history_import",
    "security_external_proxy_backfill",
    "stock_training_data_backfill",
    "stock_training_data_coverage_audit",
    "security_real_data_validation_backfill",
    // pre_trade: business computation before governance.
    "technical_consultation_basic",
    "security_analysis_contextual",
    "security_analysis_fullstack",
    "security_independent_advice",
    "security_decision_evidence_bundle",
    "security_etf_resonance_trust_pack",
    // governance_and_positioning: confirm whether and how to act.
    "security_decision_briefing",
    "security_committee_vote",
    "security_chair_resolution",
    "security_decision_package",
    "security_decision_verify_package",
    "security_decision_package_revision",
    "security_decision_submit_approval",
    "security_condition_review",
    "security_position_plan",
    "security_position_plan_record",
    "security_portfolio_position_plan",
    // execution_and_position_management: execution facts and current position view.
    "security_execution_record",
    "security_execution_journal",
    "security_account_open_position_snapshot",
    // 2026-04-19 CST: Added because P10 now starts the portfolio-core expansion
    // from one account-level objective contract on the public stock tool surface.
    // Purpose: make the first P10 contract discoverable from the catalog.
    "security_account_objective_contract",
    "security_position_contract",
    "security_monitoring_evidence_package",
    "security_capital_rebase",
    // 2026-04-19 CST: Added because Task 3 now exposes the first P11 unified
    // replacement solver on the public stock tool surface.
    // Purpose: make the portfolio replacement contract discoverable from the catalog.
    "security_portfolio_replacement_plan",
    // 2026-04-20 CST: Added because P12 now exposes the final governed
    // allocation decision on the public stock tool surface.
    // Purpose: make the portfolio-core decision-freeze contract discoverable from the catalog.
    "security_portfolio_allocation_decision",
    // 2026-04-20 CST: Added because the first downstream post-P12 bridge should
    // remain discoverable as a preview-only stock tool.
    // Purpose: make the execution preview bridge visible without implying real execution.
    "security_portfolio_execution_preview",
    // 2026-04-20 CST: Added because P13 now introduces one formal request
    // package stage after the preview-only bridge.
    // Purpose: make the request bridge discoverable without implying real execution facts.
    "security_portfolio_execution_request_package",
    "security_record_position_adjustment",
    // post_trade: review and conclusion after the in-trade loop.
    "security_post_trade_review",
    "security_post_meeting_conclusion",
    "security_record_post_meeting_conclusion",
    // modeling_and_training: long-term governed learning assets.
    "security_feature_snapshot",
    "security_forward_outcome",
    "security_master_scorecard",
    "security_scorecard_refit",
    "security_scorecard_training",
    "security_model_promotion",
    // research_sidecar: exploratory governed research and replay utilities.
    "security_history_expansion",
    "security_shadow_evaluation",
    "register_resonance_factor",
    "append_resonance_factor_series",
    "append_resonance_event_tags",
    "bootstrap_resonance_template_factors",
    "evaluate_security_resonance",
    "security_analysis_resonance",
    "record_security_signal_snapshot",
    "backfill_security_signal_outcomes",
    "study_security_signal_analogs",
    "signal_outcome_research_summary",
    "sync_template_resonance_factors",
];

pub const TOOL_NAMES: &[&str] = STOCK_TOOL_NAMES;

pub fn tool_names() -> &'static [&'static str] {
    TOOL_NAMES
}

pub fn stock_tool_names() -> &'static [&'static str] {
    STOCK_TOOL_NAMES
}

pub fn is_supported_tool(tool_name: &str) -> bool {
    TOOL_NAMES.contains(&tool_name)
}

pub fn is_stock_tool(tool_name: &str) -> bool {
    STOCK_TOOL_NAMES.contains(&tool_name)
}
