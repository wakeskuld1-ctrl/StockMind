use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_execution_journal::{
    SecurityExecutionJournalResult, SecurityExecutionTradeInput,
};
use crate::ops::stock::security_execution_record::{
    SecurityExecutionRecordError, SecurityExecutionRecordRequest, SecurityExecutionRecordResult,
    security_execution_record,
};
use crate::ops::stock::security_portfolio_position_plan::SecurityPortfolioPositionPlanDocument;
use crate::ops::stock::security_position_plan::SecurityPositionPlanResult;
use crate::ops::stock::security_post_trade_review_assembler::SecurityPostTradeReviewAssembler;

// 2026-04-09 CST: Added because the formal post-trade review contract must reuse the
// execution mainline input without hand-built wrappers; purpose: keep the post-trade
// chain aligned with execution journal and execution record.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPostTradeReviewRequest {
    pub symbol: String,
    #[serde(default)]
    pub market_symbol: Option<String>,
    #[serde(default)]
    pub sector_symbol: Option<String>,
    pub market_regime: String,
    pub sector_template: String,
    #[serde(default)]
    pub market_profile: Option<String>,
    #[serde(default)]
    pub sector_profile: Option<String>,
    #[serde(default)]
    pub as_of_date: Option<String>,
    #[serde(default = "default_review_horizon_days")]
    pub review_horizon_days: usize,
    #[serde(default = "default_lookback_days")]
    pub lookback_days: usize,
    #[serde(default = "default_factor_lookback_days")]
    pub factor_lookback_days: usize,
    #[serde(default = "default_disclosure_limit")]
    pub disclosure_limit: usize,
    #[serde(default = "default_stop_loss_pct")]
    pub stop_loss_pct: f64,
    #[serde(default = "default_target_return_pct")]
    pub target_return_pct: f64,
    #[serde(default)]
    pub actual_entry_date: String,
    #[serde(default)]
    pub actual_entry_price: f64,
    #[serde(default)]
    pub actual_position_pct: f64,
    #[serde(default)]
    pub actual_exit_date: String,
    #[serde(default)]
    pub actual_exit_price: f64,
    #[serde(default)]
    pub exit_reason: String,
    #[serde(default)]
    pub execution_trades: Vec<SecurityExecutionTradeInput>,
    #[serde(default)]
    pub execution_journal_notes: Vec<String>,
    #[serde(default)]
    pub execution_record_notes: Vec<String>,
    #[serde(default)]
    pub portfolio_position_plan_document: Option<SecurityPortfolioPositionPlanDocument>,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-09 CST: Added because the post-trade chain needs a stable document rather than
// an ad-hoc summary blob; purpose: keep review outputs reusable by package, audit, and replay.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPostTradeReviewDocument {
    pub review_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub symbol: String,
    pub analysis_date: String,
    pub snapshot_date: String,
    pub review_horizon_days: usize,
    pub review_status: String,
    pub position_plan_ref: String,
    pub snapshot_ref: String,
    pub outcome_ref: String,
    pub execution_journal_ref: String,
    pub execution_record_ref: String,
    pub planned_position: serde_json::Value,
    pub actual_result_window: String,
    pub realized_return: f64,
    pub executed_return: f64,
    pub max_drawdown_realized: f64,
    pub max_runup_realized: f64,
    pub thesis_status: String,
    pub execution_deviation: String,
    pub execution_return_gap: f64,
    pub account_plan_alignment: Option<String>,
    pub tranche_discipline: Option<String>,
    pub budget_drift_reason: Option<String>,
    pub model_miss_reason: String,
    pub next_account_adjustment_hint: Option<String>,
    pub next_adjustment_hint: String,
    pub review_summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityPostTradeReviewResult {
    pub position_plan_result: SecurityPositionPlanResult,
    pub forward_outcome_result: SecurityPostTradeReviewOutcomeBinding,
    pub execution_journal_result: SecurityExecutionJournalResult,
    pub execution_journal:
        crate::ops::stock::security_execution_journal::SecurityExecutionJournalDocument,
    pub execution_record_result: SecurityExecutionRecordResult,
    pub execution_record:
        crate::ops::stock::security_execution_record::SecurityExecutionRecordDocument,
    pub post_trade_review: SecurityPostTradeReviewDocument,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityPostTradeReviewOutcomeBinding {
    pub snapshot: crate::ops::stock::security_feature_snapshot::SecurityFeatureSnapshot,
    pub selected_outcome:
        crate::ops::stock::security_forward_outcome::SecurityForwardOutcomeDocument,
    pub all_outcomes:
        Vec<crate::ops::stock::security_forward_outcome::SecurityForwardOutcomeDocument>,
}

#[derive(Debug, Error)]
pub enum SecurityPostTradeReviewError {
    #[error("security post trade review execution preparation failed: {0}")]
    ExecutionRecord(#[from] SecurityExecutionRecordError),
    #[error("security post trade review build failed: {0}")]
    Build(String),
}

pub fn security_post_trade_review(
    request: &SecurityPostTradeReviewRequest,
) -> Result<SecurityPostTradeReviewResult, SecurityPostTradeReviewError> {
    let execution_record_result = security_execution_record(&SecurityExecutionRecordRequest {
        symbol: request.symbol.clone(),
        account_id: None,
        sector_tag: None,
        market_symbol: request.market_symbol.clone(),
        sector_symbol: request.sector_symbol.clone(),
        market_regime: request.market_regime.clone(),
        sector_template: request.sector_template.clone(),
        market_profile: request.market_profile.clone(),
        sector_profile: request.sector_profile.clone(),
        as_of_date: request.as_of_date.clone(),
        review_horizon_days: request.review_horizon_days,
        lookback_days: request.lookback_days,
        factor_lookback_days: request.factor_lookback_days,
        disclosure_limit: request.disclosure_limit,
        stop_loss_pct: request.stop_loss_pct,
        target_return_pct: request.target_return_pct,
        actual_entry_date: request.actual_entry_date.clone(),
        actual_entry_price: request.actual_entry_price,
        actual_position_pct: request.actual_position_pct,
        actual_exit_date: request.actual_exit_date.clone(),
        actual_exit_price: request.actual_exit_price,
        exit_reason: request.exit_reason.clone(),
        execution_trades: request.execution_trades.clone(),
        execution_journal_notes: request.execution_journal_notes.clone(),
        execution_record_notes: request.execution_record_notes.clone(),
        portfolio_position_plan_document: request.portfolio_position_plan_document.clone(),
        created_at: request.created_at.clone(),
    })?;
    let outcome_binding = SecurityPostTradeReviewOutcomeBinding {
        snapshot: execution_record_result
            .forward_outcome_result
            .snapshot
            .clone(),
        selected_outcome: execution_record_result
            .forward_outcome_result
            .selected_outcome
            .clone(),
        all_outcomes: execution_record_result
            .forward_outcome_result
            .all_outcomes
            .clone(),
    };
    let post_trade_review =
        build_security_post_trade_review(&execution_record_result, &outcome_binding, request)?;

    Ok(SecurityPostTradeReviewResult {
        position_plan_result: execution_record_result.position_plan_result.clone(),
        forward_outcome_result: outcome_binding,
        execution_journal_result: execution_record_result.execution_journal_result.clone(),
        execution_journal: execution_record_result.execution_journal.clone(),
        execution_record_result: execution_record_result.clone(),
        execution_record: execution_record_result.execution_record.clone(),
        post_trade_review,
    })
}

pub fn build_security_post_trade_review(
    execution_record_result: &SecurityExecutionRecordResult,
    outcome_binding: &SecurityPostTradeReviewOutcomeBinding,
    request: &SecurityPostTradeReviewRequest,
) -> Result<SecurityPostTradeReviewDocument, SecurityPostTradeReviewError> {
    SecurityPostTradeReviewAssembler::new(execution_record_result, outcome_binding, request)
        .assemble()
}

pub(crate) fn normalize_created_at(value: &str) -> String {
    if value.trim().is_empty() {
        Utc::now().to_rfc3339()
    } else {
        value.trim().to_string()
    }
}

fn default_created_at() -> String {
    Utc::now().to_rfc3339()
}

fn default_review_horizon_days() -> usize {
    20
}

fn default_lookback_days() -> usize {
    260
}

fn default_factor_lookback_days() -> usize {
    120
}

fn default_disclosure_limit() -> usize {
    6
}

fn default_stop_loss_pct() -> f64 {
    0.05
}

fn default_target_return_pct() -> f64 {
    0.12
}
