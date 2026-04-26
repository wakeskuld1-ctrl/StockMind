pub const STOCK_TOOL_NAMES: &[&str] = &[
    // data_pipeline: local-first preparation and governed gap-fill.
    "import_stock_price_history",
    "sync_stock_price_history",
    "security_fundamental_history_backfill",
    "security_fundamental_history_live_backfill",
    "security_disclosure_history_backfill",
    "security_disclosure_history_live_backfill",
    "security_capital_flow_backfill",
    "security_capital_flow_raw_audit",
    "security_volume_source_manifest",
    "security_nikkei_turnover_import",
    "security_capital_flow_jpx_weekly_import",
    "security_capital_flow_jpx_weekly_live_backfill",
    "security_capital_flow_mof_weekly_import",
    "security_capital_source_factor_snapshot",
    "security_capital_source_factor_audit",
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
    // 2026-04-26 CST: Added because the approved Nikkei ETF workflow needs one
    // daily side-effect-free target-position signal on the public Tool surface.
    // Purpose: make the index-anchored ETF signal discoverable for operator runs.
    "security_nikkei_etf_position_signal",
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
    // 2026-04-21 CST: Added because P14 now introduces one formal enrichment
    // stage after the request package and before any later execution apply bridge.
    // Purpose: make the enrichment bridge discoverable without implying runtime execution.
    "security_portfolio_execution_request_enrichment",
    // 2026-04-21 CST: Added because P15 now introduces one governed apply
    // bridge that writes runtime facts through the existing execution-record path.
    // Purpose: make the apply bridge discoverable on the public stock catalog.
    "security_portfolio_execution_apply_bridge",
    // 2026-04-22 CST: Added because P16 now introduces one pure status-freeze
    // layer downstream of the governed P15 apply bridge.
    // Purpose: make the execution-status bridge discoverable without implying reconciliation.
    "security_portfolio_execution_status_bridge",
    // 2026-04-25 CST: Added because P17 recovery now restores the formal
    // reconciliation bridge downstream of the P16 status artifact.
    // Purpose: make reconciliation truth discoverable without implying repair or replay.
    "security_portfolio_execution_reconciliation_bridge",
    // 2026-04-25 CST: Added because P18 recovery now restores the formal
    // repair-intent package downstream of the P17 reconciliation artifact.
    // Purpose: make repair intent discoverable without implying replay execution or lifecycle closeout.
    "security_portfolio_execution_repair_package",
    // 2026-04-25 CST: Added because P19A now freezes governed retry candidates
    // from the P18 repair package into replay request rows.
    // Purpose: make replay requests discoverable without implying replay execution.
    "security_portfolio_execution_replay_request_package",
    // 2026-04-25 CST: Added because P19B now exposes dry-run replay executor
    // validation after the P19A replay request package.
    // Purpose: make executor dry-run validation discoverable without implying runtime writes.
    "security_portfolio_execution_replay_executor",
    // 2026-04-26 CST: Added because P19C now freezes replay commit payload
    // readiness without becoming the runtime writer.
    // Purpose: make commit preflight discoverable without implying commit authority.
    "security_portfolio_execution_replay_commit_preflight",
    // 2026-04-26 CST: Added because P19D now owns controlled per-row runtime replay commits.
    // Purpose: expose commit writer authority separately from P19C preflight.
    "security_portfolio_execution_replay_commit_writer",
    // 2026-04-26 CST: Added because P19E now audits P19D commit results without writing runtime facts.
    // Purpose: make replay commit verification discoverable separately from commit authority.
    "security_portfolio_execution_replay_commit_audit",
    // 2026-04-26 CST: Added because P20A exposes closeout preflight readiness after P19E audit truth.
    // Purpose: make side-effect-free readiness discoverable without implying lifecycle closure.
    "security_portfolio_execution_lifecycle_closeout_readiness",
    // 2026-04-26 CST: Added because P20B exposes closeout evidence after P20A readiness.
    // Purpose: make read-only evidence packaging discoverable without implying archive production.
    "security_portfolio_execution_lifecycle_closeout_evidence_package",
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
