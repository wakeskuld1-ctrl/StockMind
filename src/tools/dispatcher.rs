use crate::tools::contracts::{ToolRequest, ToolResponse};

mod stock_ops;

// 2026-04-16 CST: Added because StockMind is a stock-only snapshot and no longer ships the
// original workbook/foundation dispatcher tree.
// Reason: the standalone repo should route only the formal securities tool surface.
// Purpose: keep the public CLI bus small, explicit, and aligned with `tools::catalog`.
pub fn dispatch(request: ToolRequest) -> ToolResponse {
    match request.tool.as_str() {
        "tool_catalog" => ToolResponse::tool_catalog(),
        "import_stock_price_history" => {
            stock_ops::dispatch_import_stock_price_history(request.args)
        }
        "sync_stock_price_history" => stock_ops::dispatch_sync_stock_price_history(request.args),
        "security_fundamental_history_live_backfill" => {
            stock_ops::dispatch_security_fundamental_history_live_backfill(request.args)
        }
        "security_fundamental_history_backfill" => {
            stock_ops::dispatch_security_fundamental_history_backfill(request.args)
        }
        "security_disclosure_history_live_backfill" => {
            stock_ops::dispatch_security_disclosure_history_live_backfill(request.args)
        }
        "security_disclosure_history_backfill" => {
            stock_ops::dispatch_security_disclosure_history_backfill(request.args)
        }
        "security_external_proxy_backfill" => {
            stock_ops::dispatch_security_external_proxy_backfill(request.args)
        }
        "security_external_proxy_history_import" => {
            stock_ops::dispatch_security_external_proxy_history_import(request.args)
        }
        "stock_training_data_backfill" => {
            stock_ops::dispatch_stock_training_data_backfill(request.args)
        }
        "stock_training_data_coverage_audit" => {
            stock_ops::dispatch_stock_training_data_coverage_audit(request.args)
        }
        "security_real_data_validation_backfill" => {
            stock_ops::dispatch_security_real_data_validation_backfill(request.args)
        }
        "technical_consultation_basic" => {
            stock_ops::dispatch_technical_consultation_basic(request.args)
        }
        "security_analysis_contextual" => {
            stock_ops::dispatch_security_analysis_contextual(request.args)
        }
        "security_analysis_fullstack" => {
            stock_ops::dispatch_security_analysis_fullstack(request.args)
        }
        "security_independent_advice" => {
            stock_ops::dispatch_security_independent_advice(request.args)
        }
        "security_decision_evidence_bundle" => {
            stock_ops::dispatch_security_decision_evidence_bundle(request.args)
        }
        "security_etf_resonance_trust_pack" => {
            stock_ops::dispatch_security_etf_resonance_trust_pack(request.args)
        }
        "security_decision_briefing" => {
            stock_ops::dispatch_security_decision_briefing(request.args)
        }
        "security_decision_committee" => {
            stock_ops::dispatch_security_decision_committee(request.args)
        }
        "security_committee_vote" => stock_ops::dispatch_security_committee_vote(request.args),
        "security_committee_member_agent" => {
            stock_ops::dispatch_security_committee_member_agent(request.args)
        }
        "security_chair_resolution" => stock_ops::dispatch_security_chair_resolution(request.args),
        "security_decision_package" => stock_ops::dispatch_security_decision_package(request.args),
        "security_decision_verify_package" => {
            stock_ops::dispatch_security_decision_verify_package(request.args)
        }
        "security_decision_package_revision" => {
            stock_ops::dispatch_security_decision_package_revision(request.args)
        }
        "security_decision_submit_approval" => {
            stock_ops::dispatch_security_decision_submit_approval(request.args)
        }
        "security_condition_review" => stock_ops::dispatch_security_condition_review(request.args),
        "security_position_plan" => stock_ops::dispatch_security_position_plan(request.args),
        "security_position_plan_record" => {
            stock_ops::dispatch_security_position_plan_record(request.args)
        }
        "security_portfolio_position_plan" => {
            stock_ops::dispatch_security_portfolio_position_plan(request.args)
        }
        "security_account_objective_contract" => {
            stock_ops::dispatch_security_account_objective_contract(request.args)
        }
        "security_portfolio_replacement_plan" => {
            stock_ops::dispatch_security_portfolio_replacement_plan(request.args)
        }
        "security_position_contract" => {
            stock_ops::dispatch_security_position_contract(request.args)
        }
        "security_execution_record" => stock_ops::dispatch_security_execution_record(request.args),
        "security_execution_journal" => {
            stock_ops::dispatch_security_execution_journal(request.args)
        }
        "security_account_open_position_snapshot" => {
            stock_ops::dispatch_security_account_open_position_snapshot(request.args)
        }
        "security_monitoring_evidence_package" => {
            stock_ops::dispatch_security_monitoring_evidence_package(request.args)
        }
        "security_capital_rebase" => stock_ops::dispatch_security_capital_rebase(request.args),
        "security_record_position_adjustment" => {
            stock_ops::dispatch_security_record_position_adjustment(request.args)
        }
        "security_post_trade_review" => {
            stock_ops::dispatch_security_post_trade_review(request.args)
        }
        "security_post_meeting_conclusion" => {
            stock_ops::dispatch_security_post_meeting_conclusion(request.args)
        }
        "security_record_post_meeting_conclusion" => {
            stock_ops::dispatch_security_record_post_meeting_conclusion(request.args)
        }
        "security_feature_snapshot" => stock_ops::dispatch_security_feature_snapshot(request.args),
        "security_forward_outcome" => stock_ops::dispatch_security_forward_outcome(request.args),
        "security_master_scorecard" => stock_ops::dispatch_security_master_scorecard(request.args),
        "security_scorecard_refit" => stock_ops::dispatch_security_scorecard_refit(request.args),
        "security_scorecard_training" => {
            stock_ops::dispatch_security_scorecard_training(request.args)
        }
        "security_model_promotion" => stock_ops::dispatch_security_model_promotion(request.args),
        "register_resonance_factor" => stock_ops::dispatch_register_resonance_factor(request.args),
        "append_resonance_factor_series" => {
            stock_ops::dispatch_append_resonance_factor_series(request.args)
        }
        "append_resonance_event_tags" => {
            stock_ops::dispatch_append_resonance_event_tags(request.args)
        }
        "bootstrap_resonance_template_factors" => {
            stock_ops::dispatch_bootstrap_resonance_template_factors(request.args)
        }
        "evaluate_security_resonance" => {
            stock_ops::dispatch_evaluate_security_resonance(request.args)
        }
        "security_analysis_resonance" => {
            stock_ops::dispatch_security_analysis_resonance(request.args)
        }
        "security_history_expansion" => {
            stock_ops::dispatch_security_history_expansion(request.args)
        }
        "security_shadow_evaluation" => {
            stock_ops::dispatch_security_shadow_evaluation(request.args)
        }
        "record_security_signal_snapshot" => {
            stock_ops::dispatch_record_security_signal_snapshot(request.args)
        }
        "backfill_security_signal_outcomes" => {
            stock_ops::dispatch_backfill_security_signal_outcomes(request.args)
        }
        "study_security_signal_analogs" => {
            stock_ops::dispatch_study_security_signal_analogs(request.args)
        }
        "signal_outcome_research_summary" => {
            stock_ops::dispatch_signal_outcome_research_summary(request.args)
        }
        "sync_template_resonance_factors" => {
            stock_ops::dispatch_sync_template_resonance_factors(request.args)
        }
        other => ToolResponse::error(format!("unsupported tool: {other}")),
    }
}
