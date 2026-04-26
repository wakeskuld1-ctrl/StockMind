use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_execution_journal::{
    SecurityExecutionJournalResult, SecurityExecutionTradeInput,
};
use crate::ops::stock::security_execution_record::{
    SecurityExecutionRecordError, SecurityExecutionRecordRequest, SecurityExecutionRecordResult,
    load_runtime_package_context, load_runtime_scorecard, resolve_market_symbol_from_profile,
    resolve_sector_symbol_from_profile, security_execution_record,
};
use crate::ops::stock::security_portfolio_position_plan::SecurityPortfolioPositionPlanDocument;
use crate::ops::stock::security_position_plan::SecurityPositionPlanResult;
use crate::ops::stock::security_post_trade_review_assembler::SecurityPostTradeReviewAssembler;
use crate::runtime::security_execution_store::SecurityExecutionStore;

// 2026-04-09 CST: Added because the formal post-trade review contract must reuse the
// execution mainline input without hand-built wrappers; purpose: keep the post-trade
// chain aligned with execution journal and execution record.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPostTradeReviewRequest {
    pub symbol: String,
    #[serde(default)]
    pub analysis_date: Option<String>,
    #[serde(default)]
    pub decision_ref: Option<String>,
    #[serde(default)]
    pub approval_ref: Option<String>,
    #[serde(default)]
    pub position_plan_ref: Option<String>,
    #[serde(default)]
    pub execution_record_ref: Option<String>,
    #[serde(default)]
    pub review_status: Option<String>,
    #[serde(default)]
    pub review_summary: Option<String>,
    #[serde(default)]
    pub attribution: Option<serde_json::Value>,
    #[serde(default)]
    pub recommended_governance_action: Option<String>,
    #[serde(default)]
    pub market_symbol: Option<String>,
    #[serde(default)]
    pub sector_symbol: Option<String>,
    #[serde(default)]
    pub market_regime: String,
    #[serde(default)]
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

#[derive(Debug, Clone)]
struct LifecyclePostTradeOverlay {
    analysis_date: String,
    position_plan_ref: String,
    review_id: String,
    execution_record_ref: String,
    review_status: Option<String>,
    review_summary: Option<String>,
    recommended_governance_action: Option<String>,
}

pub fn security_post_trade_review(
    request: &SecurityPostTradeReviewRequest,
) -> Result<SecurityPostTradeReviewResult, SecurityPostTradeReviewError> {
    let (effective_request, lifecycle_overlay) = adapt_post_trade_review_request(request)?;
    let execution_record_result = security_execution_record(&SecurityExecutionRecordRequest {
        symbol: effective_request.symbol.clone(),
        analysis_date: effective_request.analysis_date.clone(),
        decision_ref: effective_request.decision_ref.clone(),
        approval_ref: effective_request.approval_ref.clone(),
        position_plan_ref: effective_request.position_plan_ref.clone(),
        condition_review_ref: None,
        execution_action: None,
        execution_status: None,
        executed_gross_pct: None,
        execution_summary: None,
        account_id: None,
        sector_tag: None,
        market_symbol: effective_request.market_symbol.clone(),
        sector_symbol: effective_request.sector_symbol.clone(),
        market_regime: effective_request.market_regime.clone(),
        sector_template: effective_request.sector_template.clone(),
        market_profile: effective_request.market_profile.clone(),
        sector_profile: effective_request.sector_profile.clone(),
        as_of_date: effective_request.as_of_date.clone(),
        review_horizon_days: effective_request.review_horizon_days,
        lookback_days: effective_request.lookback_days,
        factor_lookback_days: effective_request.factor_lookback_days,
        disclosure_limit: effective_request.disclosure_limit,
        stop_loss_pct: effective_request.stop_loss_pct,
        target_return_pct: effective_request.target_return_pct,
        actual_entry_date: effective_request.actual_entry_date.clone(),
        actual_entry_price: effective_request.actual_entry_price,
        actual_position_pct: effective_request.actual_position_pct,
        actual_exit_date: effective_request.actual_exit_date.clone(),
        actual_exit_price: effective_request.actual_exit_price,
        exit_reason: effective_request.exit_reason.clone(),
        execution_trades: effective_request.execution_trades.clone(),
        execution_journal_notes: effective_request.execution_journal_notes.clone(),
        execution_record_notes: effective_request.execution_record_notes.clone(),
        portfolio_position_plan_document: effective_request
            .portfolio_position_plan_document
            .clone(),
        replay_commit_control: None,
        created_at: effective_request.created_at.clone(),
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
    let mut post_trade_review = build_security_post_trade_review(
        &execution_record_result,
        &outcome_binding,
        &effective_request,
    )?;
    if let Some(overlay) = lifecycle_overlay.as_ref() {
        apply_lifecycle_post_trade_overlay(&mut post_trade_review, overlay);
    }

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

// 2026-04-17 CST: Reason=lifecycle validation now reaches post_trade_review with a
// ref-based governance contract instead of the older execution-analysis inputs.
// Purpose=rebuild the legacy request shape from the persisted execution record so the
// formal review assembler can stay unchanged behind one adapter.
fn adapt_post_trade_review_request(
    request: &SecurityPostTradeReviewRequest,
) -> Result<
    (
        SecurityPostTradeReviewRequest,
        Option<LifecyclePostTradeOverlay>,
    ),
    SecurityPostTradeReviewError,
> {
    let lifecycle_requested = request
        .execution_record_ref
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
        && request.market_regime.trim().is_empty();
    if !lifecycle_requested {
        return Ok((request.clone(), None));
    }

    let store = SecurityExecutionStore::workspace_default()
        .map_err(|error| SecurityPostTradeReviewError::Build(error.to_string()))?;
    let execution_record_ref = request
        .execution_record_ref
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_string();
    let execution_record = store
        .load_execution_record(&execution_record_ref)
        .map_err(|error| SecurityPostTradeReviewError::Build(error.to_string()))?
        .ok_or_else(|| {
            SecurityPostTradeReviewError::Build(format!(
                "execution record `{execution_record_ref}` does not exist in runtime store"
            ))
        })?;
    let package = load_runtime_package_context(
        &execution_record.symbol,
        request.decision_ref.as_deref(),
        request.approval_ref.as_deref(),
        Some(&execution_record.position_plan_ref),
    )
    .map_err(|error| SecurityPostTradeReviewError::Build(error.to_string()))?;
    let scorecard = load_runtime_scorecard(&package)
        .map_err(|error| SecurityPostTradeReviewError::Build(error.to_string()))?;

    let mut effective_request = request.clone();
    effective_request.symbol = execution_record.symbol.clone();
    effective_request.position_plan_ref = Some(execution_record.position_plan_ref.clone());
    effective_request.market_regime = scorecard
        .raw_feature_snapshot
        .get("market_regime")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("a_share")
        .to_string();
    effective_request.sector_template = scorecard
        .raw_feature_snapshot
        .get("industry_bucket")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("general")
        .to_string();
    effective_request.market_profile = effective_request.market_profile.clone().or_else(|| {
        scorecard
            .raw_feature_snapshot
            .get("market_profile")
            .and_then(serde_json::Value::as_str)
            .map(|value| value.to_string())
    });
    // 2026-04-17 CST: Reason=the lifecycle contract reconstructs market inputs from the
    // scorecard first and only then derives the proxy symbol; purpose=avoid resolving the
    // proxy against an empty profile during formal post-trade validation.
    effective_request.market_symbol = effective_request.market_symbol.clone().or_else(|| {
        resolve_market_symbol_from_profile(effective_request.market_profile.as_deref())
    });
    effective_request.sector_profile = effective_request.sector_profile.clone().or_else(|| {
        scorecard
            .raw_feature_snapshot
            .get("sector_profile")
            .and_then(serde_json::Value::as_str)
            .map(|value| value.to_string())
    });
    // 2026-04-17 CST: Reason=sector proxy resolution depends on the lifecycle-restored
    // sector profile; purpose=ensure the legacy execution review path receives a concrete
    // sector symbol instead of failing with a missing configuration error.
    effective_request.sector_symbol = effective_request.sector_symbol.clone().or_else(|| {
        resolve_sector_symbol_from_profile(
            &execution_record.symbol,
            effective_request.sector_profile.as_deref(),
        )
    });
    // 2026-04-17 CST: Reason=the post-trade lifecycle adapter must replay the exact
    // trade anchor that the persisted execution record already validated against local
    // history; purpose=prevent the legacy forward-outcome builder from re-anchoring on
    // the package analysis date and demanding unavailable future rows.
    let lifecycle_trade_anchor = execution_record.actual_entry_date.trim().to_string();
    effective_request.as_of_date = Some(if lifecycle_trade_anchor.is_empty() {
        execution_record.analysis_date.clone()
    } else {
        lifecycle_trade_anchor
    });
    effective_request.actual_entry_date = execution_record.actual_entry_date.clone();
    effective_request.actual_entry_price = execution_record.actual_entry_price;
    effective_request.actual_position_pct = execution_record.actual_position_pct;
    effective_request.actual_exit_date = execution_record.actual_exit_date.clone();
    effective_request.actual_exit_price = execution_record.actual_exit_price;
    effective_request.exit_reason = execution_record.exit_reason.clone();
    effective_request.execution_trades = if execution_record.actual_exit_date.trim().is_empty() {
        vec![SecurityExecutionTradeInput {
            trade_date: execution_record.actual_entry_date.clone(),
            side: "buy".to_string(),
            price: execution_record.actual_entry_price,
            position_pct_delta: execution_record.actual_position_pct,
            reason: Some("entry".to_string()),
            notes: Vec::new(),
        }]
    } else {
        vec![
            SecurityExecutionTradeInput {
                trade_date: execution_record.actual_entry_date.clone(),
                side: "buy".to_string(),
                price: execution_record.actual_entry_price,
                position_pct_delta: execution_record.actual_position_pct,
                reason: Some("entry".to_string()),
                notes: Vec::new(),
            },
            SecurityExecutionTradeInput {
                trade_date: execution_record.actual_exit_date.clone(),
                side: "sell".to_string(),
                price: execution_record.actual_exit_price,
                position_pct_delta: execution_record.actual_position_pct,
                reason: Some(execution_record.exit_reason.clone()),
                notes: Vec::new(),
            },
        ]
    };

    let lifecycle_review_horizon_days = effective_request.review_horizon_days;
    Ok((
        effective_request,
        Some(LifecyclePostTradeOverlay {
            analysis_date: execution_record.analysis_date.clone(),
            position_plan_ref: execution_record.position_plan_ref.clone(),
            review_id: format!(
                "post-trade-review-{}-{}d",
                execution_record.position_plan_ref, lifecycle_review_horizon_days
            ),
            execution_record_ref: execution_record.execution_record_id.clone(),
            review_status: request.review_status.clone(),
            review_summary: request.review_summary.clone(),
            recommended_governance_action: request.recommended_governance_action.clone(),
        }),
    ))
}

fn apply_lifecycle_post_trade_overlay(
    post_trade_review: &mut SecurityPostTradeReviewDocument,
    overlay: &LifecyclePostTradeOverlay,
) {
    // 2026-04-17 CST: Reason=revision validates the post-trade document against the
    // package object graph rather than the legacy replay plan rebuilt underneath.
    // Purpose=project the replayed review back onto the governed lifecycle identity.
    post_trade_review.analysis_date = overlay.analysis_date.clone();
    post_trade_review.position_plan_ref = overlay.position_plan_ref.clone();
    post_trade_review.review_id = overlay.review_id.clone();
    post_trade_review.execution_record_ref = overlay.execution_record_ref.clone();
    if let Some(review_status) = overlay
        .review_status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        post_trade_review.review_status = review_status.to_string();
    }
    if let Some(review_summary) = overlay
        .review_summary
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        post_trade_review.review_summary = review_summary.to_string();
    }
    if let Some(recommended_governance_action) = overlay
        .recommended_governance_action
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        post_trade_review.next_account_adjustment_hint =
            Some(recommended_governance_action.to_string());
        post_trade_review.next_adjustment_hint = recommended_governance_action.to_string();
    }
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
