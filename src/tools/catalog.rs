pub const STOCK_TOOL_NAMES: &[&str] = &[
    "import_stock_price_history",
    "sync_stock_price_history",
    "security_fundamental_history_live_backfill",
    "security_disclosure_history_live_backfill",
    "security_external_proxy_backfill",
    "stock_training_data_backfill",
    "stock_training_data_coverage_audit",
    "security_real_data_validation_backfill",
    "technical_consultation_basic",
    "security_analysis_contextual",
    "security_analysis_fullstack",
    "security_independent_advice",
    "security_decision_evidence_bundle",
    "security_etf_resonance_trust_pack",
    "security_committee_vote",
    "security_chair_resolution",
    "security_decision_package",
    "security_decision_verify_package",
    "security_decision_package_revision",
    "security_decision_submit_approval",
    "security_condition_review",
    "security_position_plan",
    "security_portfolio_position_plan",
    "security_execution_record",
    "security_execution_journal",
    "security_account_open_position_snapshot",
    "security_post_trade_review",
    "security_post_meeting_conclusion",
    "security_record_post_meeting_conclusion",
    "security_feature_snapshot",
    "security_forward_outcome",
    "security_master_scorecard",
    "security_scorecard_refit",
    "security_scorecard_training",
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
