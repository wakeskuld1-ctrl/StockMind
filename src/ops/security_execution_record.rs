use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_decision_briefing::PositionPlan;
use crate::ops::stock::security_execution_journal::{
    SecurityExecutionJournalDocument, SecurityExecutionJournalError,
    SecurityExecutionJournalRequest, SecurityExecutionJournalResult, SecurityExecutionTradeInput,
    security_execution_journal,
};
use crate::ops::stock::security_execution_record_assembler::SecurityExecutionRecordAssembler;
use crate::ops::stock::security_forward_outcome::SecurityForwardOutcomeDocument;
use crate::ops::stock::security_open_position_corporate_action_summary::OpenPositionCorporateActionSummaryError;
use crate::ops::stock::security_portfolio_position_plan::SecurityPortfolioPositionPlanDocument;
use crate::ops::stock::security_position_plan::SecurityPositionPlanResult;
use crate::runtime::security_corporate_action_store::SecurityCorporateActionStoreError;
use crate::runtime::security_execution_store::{
    SecurityExecutionStore, SecurityExecutionStoreError,
};
use crate::runtime::stock_history_store::{StockHistoryStore, StockHistoryStoreError};
use crate::tools::contracts::SecurityPositionPlanRecordResult;

#[cfg(test)]
use crate::ops::stock::security_execution_account_binding_resolver::SecurityExecutionAccountBindingResolver;
#[cfg(test)]
use crate::ops::stock::security_portfolio_position_plan::PortfolioAllocationRecommendation;

// 2026-04-09 CST: Keep the formal execution-record request contract here because P1 still needs
// to accept both the historical single-trade fields and the newer journal-trade array input.
// Purpose: let execution_record consume the formal execution_journal shape while preserving the
// current supported request surface on the same mainline entry.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityExecutionRecordRequest {
    pub symbol: String,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub sector_tag: Option<String>,
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

// 2026-04-09 CST: Keep the formal execution-record document because P1 is not replacing record
// with journal; it needs both layers to coexist.
// Purpose: preserve a detailed journal fact object and a higher-level execution summary for
// review, package, and replay consumers at the same time.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityExecutionRecordDocument {
    pub execution_record_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub symbol: String,
    pub analysis_date: String,
    // 2026-04-10 CST: Added account identity because runtime-driven open snapshots need an
    // account dimension.
    // Purpose: let execution_record participate in an account-level state chain instead of
    // remaining an isolated trade summary.
    #[serde(default)]
    pub account_id: Option<String>,
    // 2026-04-10 CST: Added sector tagging because account snapshots still need sector-exposure
    // context when they are rebuilt from execution records.
    // Purpose: avoid losing sector constraints when the account-planning layer reconnects later.
    #[serde(default)]
    pub sector_tag: Option<String>,
    // 2026-04-10 CST: Added position state because execution_record now also represents still-open
    // position snapshots.
    // Purpose: let review, package, and later account layers explicitly distinguish closed trades
    // from in-flight holdings.
    pub position_state: String,
    pub portfolio_position_plan_ref: Option<String>,
    pub execution_journal_ref: String,
    pub position_plan_ref: String,
    pub snapshot_ref: String,
    pub outcome_ref: String,
    pub planned_entry_date: String,
    pub planned_entry_price: f64,
    pub planned_position_pct: f64,
    pub planned_max_position_pct: f64,
    pub actual_entry_date: String,
    pub actual_entry_price: f64,
    pub actual_position_pct: f64,
    // 2026-04-10 CST: Added current remaining position because the account snapshot layer needs
    // "how much is still held" rather than just the peak size.
    // Purpose: formalize current_position_pct so callers stop misreading peak position as the
    // current live holding.
    pub current_position_pct: f64,
    pub actual_exit_date: String,
    pub actual_exit_price: f64,
    pub exit_reason: String,
    pub holding_days: usize,
    pub planned_forward_return: f64,
    pub actual_return: f64,
    pub entry_slippage_pct: f64,
    pub position_size_gap_pct: f64,
    pub planned_tranche_action: Option<String>,
    pub planned_tranche_pct: Option<f64>,
    pub planned_peak_position_pct: Option<f64>,
    pub actual_tranche_action: Option<String>,
    pub actual_tranche_pct: Option<f64>,
    pub actual_peak_position_pct: Option<f64>,
    pub tranche_count_drift: Option<i32>,
    pub account_budget_alignment: Option<String>,
    pub execution_return_gap: f64,
    pub execution_quality: String,
    // 2026-04-16 CST: Added because P0-3 needs execution_record to expose the same
    // governed holding-economics surface that open_position_snapshot already uses.
    // Purpose: reduce semantic drift between runtime open snapshots and execution-level
    // records without rewriting the historical actual_return contract in this round.
    #[serde(default)]
    pub price_as_of_date: Option<String>,
    // 2026-04-16 CST: Added because the holding economics must state which latest local
    // trade date actually resolved the current price.
    // Purpose: stop execution_record consumers from assuming requested date always equals
    // a valid trade date when weekend or holiday normalization happened.
    #[serde(default)]
    pub resolved_trade_date: Option<String>,
    // 2026-04-16 CST: Added because corporate-action-adjusted holding economics need the
    // current local close used in the breakeven and return calculation.
    // Purpose: let downstream review and audit consumers see the price anchor directly.
    #[serde(default)]
    pub current_price: Option<f64>,
    // 2026-04-16 CST: Added because P0-2 already introduced split/bonus-aware holding
    // math and P0-3 now needs execution_record to carry the same share-count factor.
    // Purpose: keep execution-level holding economics interpretable after share-count
    // changes without rewriting older fields.
    #[serde(default)]
    pub share_adjustment_factor: Option<f64>,
    // 2026-04-16 CST: Added because dividend-adjusted breakeven needs the cumulative cash
    // payout since entry on the governed original-share basis.
    // Purpose: expose the cash component explicitly instead of forcing callers to parse
    // the textual summary.
    #[serde(default)]
    pub cumulative_cash_dividend_per_share: Option<f64>,
    // 2026-04-16 CST: Added because open-position execution records now need the formal
    // post-dividend, post-share-adjustment cost basis.
    // Purpose: align execution_record with the snapshot holding-economics semantics.
    #[serde(default)]
    pub dividend_adjusted_cost_basis: Option<f64>,
    // 2026-04-16 CST: Added because actual_return remains the historical execution result
    // field while P0-3 needs a separate live holding total return surface.
    // Purpose: expose current holding economics without changing old actual_return meaning.
    #[serde(default)]
    pub holding_total_return_pct: Option<f64>,
    // 2026-04-16 CST: Added because the user is explicitly prioritizing breakeven and
    // holding-income correctness before reopening the heavier training chain.
    // Purpose: let execution_record become a formal consumer of the governed breakeven path.
    #[serde(default)]
    pub breakeven_price: Option<f64>,
    // 2026-04-16 CST: Added because downstream consumers need one human-readable audit
    // line that explains whether dividends or share changes affected the holding math.
    // Purpose: preserve a minimal interpretable explanation beside the numeric fields.
    #[serde(default)]
    pub corporate_action_summary: Option<String>,
    pub execution_record_notes: Vec<String>,
    pub attribution_summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityExecutionRecordResult {
    pub execution_journal_result: SecurityExecutionJournalResult,
    pub execution_journal: SecurityExecutionJournalDocument,
    pub position_plan_result: SecurityPositionPlanResult,
    pub forward_outcome_result: SecurityExecutionRecordOutcomeBinding,
    pub execution_record: SecurityExecutionRecordDocument,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityExecutionRecordOutcomeBinding {
    pub snapshot: crate::ops::stock::security_feature_snapshot::SecurityFeatureSnapshot,
    pub selected_outcome: SecurityForwardOutcomeDocument,
    pub all_outcomes: Vec<SecurityForwardOutcomeDocument>,
}

#[derive(Debug, Error)]
pub enum SecurityExecutionRecordError {
    #[error("security execution record journal preparation failed: {0}")]
    ExecutionJournal(#[from] SecurityExecutionJournalError),
    #[error("security execution record history loading failed: {0}")]
    History(#[from] StockHistoryStoreError),
    #[error("security execution record runtime persistence failed: {0}")]
    Store(#[from] SecurityExecutionStoreError),
    #[error("security execution record corporate action runtime loading failed: {0}")]
    CorporateAction(#[from] SecurityCorporateActionStoreError),
    #[error("security execution record holding economics build failed: {0}")]
    HoldingEconomics(#[from] OpenPositionCorporateActionSummaryError),
    #[error("security execution record build failed: {0}")]
    Build(String),
}

pub fn security_execution_record(
    request: &SecurityExecutionRecordRequest,
) -> Result<SecurityExecutionRecordResult, SecurityExecutionRecordError> {
    let execution_journal_request = build_execution_journal_request(request);
    let execution_journal_result = security_execution_journal(&execution_journal_request)?;
    let forward_outcome_result = SecurityExecutionRecordOutcomeBinding {
        snapshot: execution_journal_result
            .forward_outcome_result
            .snapshot
            .clone(),
        selected_outcome: execution_journal_result
            .forward_outcome_result
            .selected_outcome
            .clone(),
        all_outcomes: execution_journal_result
            .forward_outcome_result
            .all_outcomes
            .clone(),
    };
    let execution_record = build_security_execution_record(
        &execution_journal_result.position_plan_result,
        &forward_outcome_result,
        &execution_journal_result.execution_journal,
        request,
    )?;
    // 2026-04-10 CST: Persist the formal execution record because plan B needs later account tools
    // to recover open-position state automatically.
    // Purpose: give account snapshots and planning a single runtime fact source instead of
    // depending on explicit caller-provided carry-over arguments.
    let store = SecurityExecutionStore::workspace_default()?;
    let position_plan_record =
        build_execution_store_position_plan_record(&execution_journal_result.position_plan_result);
    let session = store.open_session()?;
    session.upsert_position_plan(&position_plan_record)?;
    session.upsert_execution_record(&execution_record)?;
    session.commit()?;

    Ok(SecurityExecutionRecordResult {
        execution_journal_result: execution_journal_result.clone(),
        execution_journal: execution_journal_result.execution_journal.clone(),
        position_plan_result: execution_journal_result.position_plan_result.clone(),
        forward_outcome_result,
        execution_record,
    })
}

// 2026-04-09 CST: Expose the execution-record builder because review, package, and audit still
// reuse this aggregate execution summary.
// Purpose: centralize the rule set from journal aggregate to execution quality and attribution so
// multiple tools do not rebuild execution_quality independently.
pub fn build_security_execution_record(
    position_plan_result: &SecurityPositionPlanResult,
    outcome_binding: &SecurityExecutionRecordOutcomeBinding,
    execution_journal: &SecurityExecutionJournalDocument,
    request: &SecurityExecutionRecordRequest,
) -> Result<SecurityExecutionRecordDocument, SecurityExecutionRecordError> {
    // 2026-04-14 CST: Refactored because round 2 needs the formal execution-record
    // assembly extracted from the orchestration entry; purpose: keep behavior stable
    // while giving the bottom layer a dedicated class-like boundary.
    // 2026-04-14 CST: Simplified because round 2 validation confirmed the assembler is now the
    // only formal build path; purpose: remove legacy dead code so later AI sessions do not treat
    // the old inline builder body as an active implementation branch.
    SecurityExecutionRecordAssembler::new(
        position_plan_result,
        outcome_binding,
        execution_journal,
        request,
    )
    .assemble()
}

// 2026-04-15 CST: Added because the execution-record path needs one canonical projection from the
// current plan result shell into the execution-store plan-record contract.
// Purpose: keep the formal execution-store write path on one official projection entry so later
// AI sessions do not invent parallel plan-record builders for the same persistence target.
pub(crate) fn build_execution_store_position_plan_record(
    position_plan_result: &SecurityPositionPlanResult,
) -> SecurityPositionPlanRecordResult {
    let document = &position_plan_result.position_plan_document;
    SecurityPositionPlanRecordResult {
        position_plan_ref: document.position_plan_id.clone(),
        decision_ref: format!("execution-store-decision-ref:{}", document.position_plan_id),
        approval_ref: format!("execution-store-approval-ref:{}", document.position_plan_id),
        evidence_version: document.evidence_version.clone(),
        symbol: document.symbol.clone(),
        analysis_date: document.analysis_date.clone(),
        position_action: document.position_action.clone(),
        starter_position_pct: document.starter_position_pct,
        max_position_pct: document.max_position_pct,
        position_plan: PositionPlan {
            position_action: document.position_action.clone(),
            entry_mode: document.entry_mode.clone(),
            starter_position_pct: document.starter_position_pct,
            max_position_pct: document.max_position_pct,
            add_on_trigger: document.add_on_trigger.clone(),
            reduce_on_trigger: document.reduce_on_trigger.clone(),
            hard_stop_trigger: document.hard_stop_trigger.clone(),
            liquidity_cap: document.liquidity_cap.clone(),
            position_risk_grade: document.position_risk_grade.clone(),
            regime_adjustment: document.regime_adjustment.clone(),
            execution_notes: document.execution_notes.clone(),
            rationale: document.rationale.clone(),
        },
    }
}

fn build_execution_journal_request(
    request: &SecurityExecutionRecordRequest,
) -> SecurityExecutionJournalRequest {
    let execution_trades = if request.execution_trades.is_empty() {
        if !request.actual_exit_date.trim().is_empty() && request.actual_exit_price > 0.0 {
            vec![
                SecurityExecutionTradeInput {
                    trade_date: request.actual_entry_date.clone(),
                    side: "buy".to_string(),
                    price: request.actual_entry_price,
                    position_pct_delta: request.actual_position_pct,
                    reason: Some("entry".to_string()),
                    notes: Vec::new(),
                },
                SecurityExecutionTradeInput {
                    trade_date: request.actual_exit_date.clone(),
                    side: "sell".to_string(),
                    price: request.actual_exit_price,
                    position_pct_delta: request.actual_position_pct,
                    reason: Some(request.exit_reason.clone()),
                    notes: Vec::new(),
                },
            ]
        } else {
            vec![SecurityExecutionTradeInput {
                trade_date: request.actual_entry_date.clone(),
                side: "buy".to_string(),
                price: request.actual_entry_price,
                position_pct_delta: request.actual_position_pct,
                reason: Some("entry".to_string()),
                notes: Vec::new(),
            }]
        }
    } else {
        request.execution_trades.clone()
    };

    let execution_journal_notes = if request.execution_journal_notes.is_empty() {
        request.execution_record_notes.clone()
    } else {
        request.execution_journal_notes.clone()
    };

    SecurityExecutionJournalRequest {
        symbol: request.symbol.clone(),
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
        execution_trades,
        execution_journal_notes,
        created_at: request.created_at.clone(),
    }
}

pub(crate) fn load_planned_entry_price(
    store: &StockHistoryStore,
    symbol: &str,
    as_of_date: &str,
) -> Result<f64, SecurityExecutionRecordError> {
    let recent_rows = store.load_recent_rows(symbol, Some(as_of_date), 1)?;
    let entry_row = recent_rows.last().ok_or_else(|| {
        SecurityExecutionRecordError::Build(format!(
            "missing planned entry row for {} at {}",
            symbol, as_of_date
        ))
    })?;
    if entry_row.trade_date != as_of_date {
        return Err(SecurityExecutionRecordError::Build(format!(
            "planned entry row drift for {}: expected {}, got {}",
            symbol, as_of_date, entry_row.trade_date
        )));
    }
    if entry_row.adj_close <= 0.0 {
        return Err(SecurityExecutionRecordError::Build(format!(
            "planned entry price must be positive for {} at {}",
            symbol, as_of_date
        )));
    }
    Ok(entry_row.adj_close)
}

pub(crate) fn compute_holding_days(
    actual_entry_date: &str,
    actual_exit_date: &str,
) -> Result<usize, SecurityExecutionRecordError> {
    let start = parse_date(actual_entry_date, "actual_entry_date")?;
    let end = parse_date(actual_exit_date, "actual_exit_date")?;
    Ok(end.signed_duration_since(start).num_days() as usize)
}

fn parse_date(value: &str, field_name: &str) -> Result<NaiveDate, SecurityExecutionRecordError> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").map_err(|error| {
        SecurityExecutionRecordError::Build(format!(
            "{field_name} must be YYYY-MM-DD, got `{value}`: {error}"
        ))
    })
}

pub(crate) fn classify_execution_quality(
    position_state: &str,
    entry_slippage_pct: f64,
    position_size_gap_pct: f64,
    actual_return: f64,
    planned_forward_return: f64,
    actual_position_pct: f64,
    planned_max_position_pct: f64,
) -> String {
    if position_state == "open" {
        return "open_position_pending".to_string();
    }
    if actual_position_pct > planned_max_position_pct + 1e-9
        || entry_slippage_pct > 0.03
        || actual_return < planned_forward_return - 0.05
    {
        return "adverse".to_string();
    }
    if entry_slippage_pct.abs() <= 0.01 && position_size_gap_pct.abs() <= 0.03 {
        return "aligned".to_string();
    }
    "partial_drift".to_string()
}

// 2026-04-14 CST: Added because the refactor needs a stable attribution string contract that can
// be regression-tested separately from the larger assembler body.
pub(crate) fn build_attribution_summary(
    final_position_pct: f64,
    actual_return: f64,
    execution_return_gap: f64,
    entry_slippage_pct: f64,
    position_size_gap_pct: f64,
    execution_quality: &str,
) -> String {
    format!(
        "current_position={:.2}%, actual_return={:.2}%, return_gap={:.2}%, entry_slippage={:.2}%, position_gap={:.2}%, quality={}",
        final_position_pct * 100.0,
        actual_return * 100.0,
        execution_return_gap * 100.0,
        entry_slippage_pct * 100.0,
        position_size_gap_pct * 100.0,
        execution_quality
    )
}

// 2026-04-14 CST: Added because round 2 needs exit-reason resolution separated from the large
// builder body; purpose: keep open-position handling explicit and unit-testable.
pub(crate) fn resolve_exit_reason(
    position_state: &str,
    trades: &[crate::ops::stock::security_execution_journal::SecurityExecutionJournalTrade],
    request_exit_reason: &str,
) -> String {
    if position_state == "open" {
        return "position_still_open".to_string();
    }
    trades
        .iter()
        .rev()
        .find(|item| item.side == "sell")
        .map(|item| item.reason.clone())
        .filter(|item| !item.trim().is_empty())
        .unwrap_or_else(|| request_exit_reason.trim().to_string())
}

pub(crate) fn rounded_pct(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
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
    180
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::stock::security_committee_vote::SecurityCommitteeVoteResult;
    use crate::ops::stock::security_decision_briefing::{
        CommitteeEvidenceChecks, CommitteeExecutionDigest, CommitteeHistoricalDigest,
        CommitteePayload, CommitteeRecommendationDigest, CommitteeRecommendationEntry,
        CommitteeRecommendations, CommitteeResonanceDigest, CommitteeRiskBreakdown,
        CommitteeRiskItem, CommitteeSubjectProfile, ExecutionPlan, OddsBrief, PositionPlan,
        SecurityDecisionBriefingResult,
    };
    use crate::ops::stock::security_execution_journal::SecurityExecutionJournalTrade;
    use crate::ops::stock::security_portfolio_position_plan::SecurityPortfolioPositionPlanDocument;
    use crate::ops::stock::stock_analysis_data_guard::StockAnalysisDateGuard;
    use crate::runtime::stock_history_store::{StockHistoryRow, StockHistoryStore};
    use std::path::PathBuf;

    fn portfolio_plan_fixture() -> SecurityPortfolioPositionPlanDocument {
        SecurityPortfolioPositionPlanDocument {
            portfolio_position_plan_id: "portfolio-plan-1".to_string(),
            contract_version: "security_portfolio_position_plan.v1".to_string(),
            document_type: "security_portfolio_position_plan".to_string(),
            generated_at: "2026-04-14T16:00:00+08:00".to_string(),
            account_id: "acct-1".to_string(),
            total_equity: 100_000.0,
            available_cash: 30_000.0,
            current_cash_pct: 0.30,
            min_cash_reserve_pct: 0.10,
            deployable_cash_amount: 20_000.0,
            deployable_cash_pct: 0.20,
            current_invested_pct: 0.70,
            max_portfolio_risk_budget_pct: 0.10,
            current_portfolio_risk_budget_pct: 0.02,
            remaining_portfolio_risk_budget_pct: 0.08,
            max_single_trade_risk_budget_pct: 0.03,
            estimated_new_risk_budget_pct: 0.02,
            total_portfolio_risk_budget_pct: 0.04,
            concentration_warnings: Vec::new(),
            risk_budget_warnings: Vec::new(),
            allocations: vec![PortfolioAllocationRecommendation {
                symbol: "601916.SH".to_string(),
                action: "add".to_string(),
                sector_tag: "bank".to_string(),
                current_position_pct: 0.04,
                target_position_pct: 0.12,
                incremental_position_pct: 0.08,
                recommended_trade_amount: 8_000.0,
                estimated_risk_budget_pct: 0.02,
                suggested_tranche_action: "add_tranche".to_string(),
                suggested_tranche_pct: 0.08,
                remaining_tranche_count: 1,
                priority_score: 80,
                constraint_flags: Vec::new(),
                rationale: vec!["fixture".to_string()],
            }],
            portfolio_summary: "fixture".to_string(),
        }
    }

    fn execution_request_fixture() -> SecurityExecutionRecordRequest {
        SecurityExecutionRecordRequest {
            symbol: "601916.SH".to_string(),
            account_id: Some("acct-1".to_string()),
            sector_tag: Some("bank".to_string()),
            market_symbol: None,
            sector_symbol: None,
            market_regime: "a_share".to_string(),
            sector_template: "bank".to_string(),
            market_profile: None,
            sector_profile: None,
            as_of_date: Some("2025-09-17".to_string()),
            review_horizon_days: 20,
            lookback_days: 260,
            factor_lookback_days: 120,
            disclosure_limit: 6,
            stop_loss_pct: 0.05,
            target_return_pct: 0.12,
            actual_entry_date: "2025-09-18".to_string(),
            actual_entry_price: 62.4,
            actual_position_pct: 0.12,
            actual_exit_date: String::new(),
            actual_exit_price: 0.0,
            exit_reason: "manual".to_string(),
            execution_trades: Vec::new(),
            execution_journal_notes: Vec::new(),
            execution_record_notes: Vec::new(),
            portfolio_position_plan_document: Some(portfolio_plan_fixture()),
            created_at: "2026-04-14T16:00:00+08:00".to_string(),
        }
    }

    fn isolated_stock_store() -> StockHistoryStore {
        let mut db_path = std::env::temp_dir();
        let unique = format!(
            "security_execution_record_{}_{}.db",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        db_path.push(unique);
        let store = StockHistoryStore::new(PathBuf::from(&db_path));
        store
            .import_rows(
                "601916.SH",
                "unit-test",
                &[StockHistoryRow {
                    trade_date: "2025-09-17".to_string(),
                    open: 60.0,
                    high: 61.0,
                    low: 59.5,
                    close: 60.8,
                    adj_close: 61.0,
                    volume: 1_000_000,
                }],
            )
            .expect("fixture history import should succeed");
        store
    }

    fn with_stock_runtime<T>(store: &StockHistoryStore, run: impl FnOnce() -> T) -> T {
        let original_stock_db = std::env::var("EXCEL_SKILL_STOCK_DB").ok();
        let original_runtime_dir = std::env::var("EXCEL_SKILL_RUNTIME_DIR").ok();
        let original_runtime_db = std::env::var("EXCEL_SKILL_RUNTIME_DB").ok();

        // 2026-04-14 CST: Force the assembler to use the isolated stock store so this test only
        // validates execution-record behavior and not the caller's local workspace runtime.
        unsafe {
            std::env::set_var("EXCEL_SKILL_STOCK_DB", store.db_path());
            std::env::remove_var("EXCEL_SKILL_RUNTIME_DIR");
            std::env::remove_var("EXCEL_SKILL_RUNTIME_DB");
        }

        let output = run();

        unsafe {
            match original_stock_db {
                Some(value) => std::env::set_var("EXCEL_SKILL_STOCK_DB", value),
                None => std::env::remove_var("EXCEL_SKILL_STOCK_DB"),
            }
            match original_runtime_dir {
                Some(value) => std::env::set_var("EXCEL_SKILL_RUNTIME_DIR", value),
                None => std::env::remove_var("EXCEL_SKILL_RUNTIME_DIR"),
            }
            match original_runtime_db {
                Some(value) => std::env::set_var("EXCEL_SKILL_RUNTIME_DB", value),
                None => std::env::remove_var("EXCEL_SKILL_RUNTIME_DB"),
            }
        }

        output
    }

    fn execution_journal_fixture() -> SecurityExecutionJournalDocument {
        SecurityExecutionJournalDocument {
            execution_journal_id: "journal-1".to_string(),
            contract_version: "security_execution_journal.v1".to_string(),
            document_type: "security_execution_journal".to_string(),
            generated_at: "2026-04-14T16:00:00+08:00".to_string(),
            symbol: "601916.SH".to_string(),
            analysis_date: "2025-09-17".to_string(),
            position_state: "closed".to_string(),
            position_plan_ref: "position-plan-1".to_string(),
            snapshot_ref: "snapshot-1".to_string(),
            outcome_ref: "outcome-1".to_string(),
            trades: vec![
                SecurityExecutionJournalTrade {
                    trade_id: "trade-1".to_string(),
                    trade_date: "2025-09-18".to_string(),
                    side: "buy".to_string(),
                    price: 62.4,
                    position_pct_delta: 0.12,
                    resulting_position_pct: 0.12,
                    reason: "entry".to_string(),
                    notes: Vec::new(),
                },
                SecurityExecutionJournalTrade {
                    trade_id: "trade-2".to_string(),
                    trade_date: "2025-09-25".to_string(),
                    side: "sell".to_string(),
                    price: 66.0,
                    position_pct_delta: 0.12,
                    resulting_position_pct: 0.0,
                    reason: "target_hit".to_string(),
                    notes: Vec::new(),
                },
            ],
            trade_count: 2,
            entry_trade_count: 1,
            exit_trade_count: 1,
            holding_start_date: "2025-09-18".to_string(),
            holding_end_date: "2025-09-25".to_string(),
            peak_position_pct: 0.12,
            final_position_pct: 0.0,
            weighted_entry_price: 62.4,
            weighted_exit_price: 66.0,
            realized_return: 0.0576923077,
            execution_journal_notes: vec!["journal-note".to_string()],
            aggregation_summary: "fixture".to_string(),
        }
    }

    fn execution_record_outcome_fixture() -> SecurityExecutionRecordOutcomeBinding {
        SecurityExecutionRecordOutcomeBinding {
            snapshot: crate::ops::stock::security_feature_snapshot::SecurityFeatureSnapshot {
                snapshot_id: "snapshot-1".to_string(),
                contract_version: "security_feature_snapshot.v1".to_string(),
                document_type: "security_feature_snapshot".to_string(),
                symbol: "601916.SH".to_string(),
                market: "CN".to_string(),
                instrument_type: "stock".to_string(),
                as_of_date: "2025-09-17".to_string(),
                data_cutoff_at: "2025-09-17".to_string(),
                feature_set_version: "security_feature_snapshot.v1".to_string(),
                raw_features_json: Default::default(),
                group_features_json: Default::default(),
                data_quality_flags: Vec::new(),
                snapshot_hash: "hash-1".to_string(),
            },
            selected_outcome:
                crate::ops::stock::security_forward_outcome::SecurityForwardOutcomeDocument {
                    outcome_id: "outcome-1".to_string(),
                    contract_version: "security_forward_outcome.v1".to_string(),
                    document_type: "security_forward_outcome".to_string(),
                    snapshot_id: "snapshot-1".to_string(),
                    symbol: "601916.SH".to_string(),
                    market: "CN".to_string(),
                    instrument_type: "stock".to_string(),
                    as_of_date: "2025-09-17".to_string(),
                    horizon_days: 20,
                    forward_return: 0.08,
                    max_drawdown: -0.03,
                    max_runup: 0.09,
                    positive_return: true,
                    hit_upside_first: true,
                    hit_stop_first: false,
                    label_definition_version: "security_forward_outcome.v1".to_string(),
                },
            all_outcomes: Vec::new(),
        }
    }

    fn position_plan_result_fixture() -> SecurityPositionPlanResult {
        // 2026-04-14 CST: Only the position_plan_document is exercised by the execution-record
        // assembler, so the briefing shell stays inert in this unit fixture on purpose.
        let briefing_core = SecurityDecisionBriefingResult {
            symbol: "601916.SH".to_string(),
            analysis_date: "2025-09-17".to_string(),
            analysis_date_guard: analysis_date_guard_fixture(),
            summary: "fixture".to_string(),
            evidence_version: "fixture".to_string(),
            subject_profile: CommitteeSubjectProfile {
                asset_class: "equity".to_string(),
                market_scope: "china".to_string(),
                committee_focus: "stock_review".to_string(),
            },
            fundamental_brief: serde_json::json!({"status": "ok", "summary": "fixture"}),
            technical_brief: serde_json::json!({"status": "ok", "summary": "fixture"}),
            resonance_brief: serde_json::json!({"status": "ok", "summary": "fixture"}),
            execution_plan: ExecutionPlan {
                add_trigger_price: 0.0,
                add_trigger_volume_ratio: 0.0,
                add_position_pct: 0.0,
                reduce_trigger_price: 0.0,
                rejection_zone: "none".to_string(),
                reduce_position_pct: 0.0,
                stop_loss_price: 0.0,
                invalidation_price: 0.0,
                watch_points: Vec::new(),
                explanation: Vec::new(),
            },
            odds_brief: OddsBrief {
                status: "ok".to_string(),
                historical_confidence: "medium".to_string(),
                sample_count: 1,
                win_rate_10d: Some(0.6),
                loss_rate_10d: Some(0.4),
                flat_rate_10d: Some(0.0),
                avg_return_10d: Some(0.08),
                median_return_10d: Some(0.08),
                avg_win_return_10d: Some(0.12),
                avg_loss_return_10d: Some(-0.04),
                payoff_ratio_10d: Some(3.0),
                expectancy_10d: Some(0.056),
                expected_return_window: Some("10d".to_string()),
                expected_drawdown_window: Some("10d".to_string()),
                odds_grade: "good".to_string(),
                confidence_grade: "medium".to_string(),
                rationale: Vec::new(),
                research_limitations: Vec::new(),
            },
            position_plan: PositionPlan {
                position_action: "build".to_string(),
                entry_mode: "starter".to_string(),
                starter_position_pct: 0.08,
                max_position_pct: 0.12,
                add_on_trigger: "breakout".to_string(),
                reduce_on_trigger: "loss_of_momentum".to_string(),
                hard_stop_trigger: "stop".to_string(),
                liquidity_cap: "standard".to_string(),
                position_risk_grade: "moderate".to_string(),
                regime_adjustment: "neutral".to_string(),
                execution_notes: Vec::new(),
                rationale: Vec::new(),
            },
            committee_payload: committee_payload_fixture(),
            committee_recommendations: committee_recommendations_fixture(),
        };

        SecurityPositionPlanResult {
            briefing_core,
            position_plan_document:
                crate::ops::stock::security_position_plan::SecurityPositionPlanDocument {
                    position_plan_id: "position-plan-1".to_string(),
                    contract_version: "security_position_plan.v1".to_string(),
                    document_type: "security_position_plan".to_string(),
                    generated_at: "2026-04-14T16:00:00+08:00".to_string(),
                    symbol: "601916.SH".to_string(),
                    analysis_date: "2025-09-17".to_string(),
                    analysis_date_guard: analysis_date_guard_fixture(),
                    evidence_version: "fixture".to_string(),
                    briefing_ref: "briefing-1".to_string(),
                    committee_payload_ref: "committee-1".to_string(),
                    recommended_action: "build".to_string(),
                    confidence: "medium".to_string(),
                    odds_grade: "good".to_string(),
                    historical_confidence: "medium".to_string(),
                    confidence_grade: "medium".to_string(),
                    position_action: "build".to_string(),
                    entry_mode: "starter".to_string(),
                    starter_position_pct: 0.08,
                    max_position_pct: 0.12,
                    entry_tranche_pct: 0.08,
                    add_tranche_pct: 0.04,
                    reduce_tranche_pct: 0.04,
                    max_tranche_count: 2,
                    tranche_template: "standard".to_string(),
                    tranche_trigger_rules: vec!["breakout".to_string()],
                    cooldown_rule: "none".to_string(),
                    add_on_trigger: "breakout".to_string(),
                    reduce_on_trigger: "loss_of_momentum".to_string(),
                    hard_stop_trigger: "stop".to_string(),
                    liquidity_cap: "standard".to_string(),
                    position_risk_grade: "moderate".to_string(),
                    regime_adjustment: "neutral".to_string(),
                    execution_notes: vec!["fixture".to_string()],
                    rationale: vec!["fixture".to_string()],
                },
        }
    }

    fn analysis_date_guard_fixture() -> StockAnalysisDateGuard {
        StockAnalysisDateGuard {
            requested_as_of_date: "2025-09-17".to_string(),
            effective_analysis_date: "2025-09-17".to_string(),
            effective_trade_date: "2025-09-17".to_string(),
            local_data_last_date: Some("2025-09-17".to_string()),
            data_freshness_status: "fresh".to_string(),
            sync_attempted: false,
            sync_result: None,
            date_fallback_reason: None,
        }
    }

    fn committee_payload_fixture() -> CommitteePayload {
        CommitteePayload {
            symbol: "601916.SH".to_string(),
            analysis_date: "2025-09-17".to_string(),
            recommended_action: "buy".to_string(),
            confidence: "medium".to_string(),
            subject_profile: CommitteeSubjectProfile {
                asset_class: "equity".to_string(),
                market_scope: "china".to_string(),
                committee_focus: "stock_review".to_string(),
            },
            risk_breakdown: CommitteeRiskBreakdown {
                technical: vec![CommitteeRiskItem {
                    category: "technical".to_string(),
                    severity: "medium".to_string(),
                    headline: "breakout needs confirmation".to_string(),
                    rationale: "fixture".to_string(),
                }],
                fundamental: Vec::new(),
                resonance: Vec::new(),
                execution: Vec::new(),
            },
            key_risks: vec!["breakout needs confirmation".to_string()],
            minority_objection_points: Vec::new(),
            evidence_version: "fixture".to_string(),
            briefing_digest: "fixture".to_string(),
            committee_schema_version: "committee-payload:v1".to_string(),
            recommendation_digest: CommitteeRecommendationDigest {
                final_stance: "constructive".to_string(),
                action_bias: "buy".to_string(),
                summary: "fixture".to_string(),
                confidence: "medium".to_string(),
            },
            execution_digest: CommitteeExecutionDigest {
                add_trigger_price: 0.0,
                add_trigger_volume_ratio: 0.0,
                add_position_pct: 0.0,
                reduce_trigger_price: 0.0,
                reduce_position_pct: 0.0,
                stop_loss_price: 0.0,
                invalidation_price: 0.0,
                rejection_zone: "none".to_string(),
                watch_points: Vec::new(),
                explanation: Vec::new(),
            },
            resonance_digest: CommitteeResonanceDigest {
                resonance_score: 0.0,
                action_bias: "neutral".to_string(),
                top_positive_driver_names: Vec::new(),
                top_negative_driver_names: Vec::new(),
                event_override_titles: Vec::new(),
            },
            evidence_checks: CommitteeEvidenceChecks {
                fundamental_ready: true,
                technical_ready: true,
                resonance_ready: true,
                execution_ready: true,
                briefing_ready: true,
            },
            historical_digest: CommitteeHistoricalDigest {
                status: "ok".to_string(),
                historical_confidence: "medium".to_string(),
                analog_sample_count: 1,
                analog_win_rate_10d: Some(0.6),
                analog_loss_rate_10d: Some(0.4),
                analog_flat_rate_10d: Some(0.0),
                analog_avg_return_10d: Some(0.08),
                analog_median_return_10d: Some(0.08),
                analog_avg_win_return_10d: Some(0.12),
                analog_avg_loss_return_10d: Some(-0.04),
                analog_payoff_ratio_10d: Some(3.0),
                analog_expectancy_10d: Some(0.056),
                expected_return_window: Some("10d".to_string()),
                expected_drawdown_window: Some("10d".to_string()),
                research_limitations: Vec::new(),
            },
            odds_digest: OddsBrief {
                status: "ok".to_string(),
                historical_confidence: "medium".to_string(),
                sample_count: 1,
                win_rate_10d: Some(0.6),
                loss_rate_10d: Some(0.4),
                flat_rate_10d: Some(0.0),
                avg_return_10d: Some(0.08),
                median_return_10d: Some(0.08),
                avg_win_return_10d: Some(0.12),
                avg_loss_return_10d: Some(-0.04),
                payoff_ratio_10d: Some(3.0),
                expectancy_10d: Some(0.056),
                expected_return_window: Some("10d".to_string()),
                expected_drawdown_window: Some("10d".to_string()),
                odds_grade: "good".to_string(),
                confidence_grade: "medium".to_string(),
                rationale: Vec::new(),
                research_limitations: Vec::new(),
            },
            position_digest: PositionPlan {
                position_action: "build".to_string(),
                entry_mode: "starter".to_string(),
                starter_position_pct: 0.08,
                max_position_pct: 0.12,
                add_on_trigger: "breakout".to_string(),
                reduce_on_trigger: "loss_of_momentum".to_string(),
                hard_stop_trigger: "stop".to_string(),
                liquidity_cap: "standard".to_string(),
                position_risk_grade: "moderate".to_string(),
                regime_adjustment: "neutral".to_string(),
                execution_notes: Vec::new(),
                rationale: Vec::new(),
            },
        }
    }

    fn committee_recommendations_fixture() -> CommitteeRecommendations {
        CommitteeRecommendations {
            default_mode: "standard".to_string(),
            report_focus: "fixture".to_string(),
            standard: CommitteeRecommendationEntry {
                scenario: "base".to_string(),
                vote: committee_vote_fixture("standard"),
            },
            strict: CommitteeRecommendationEntry {
                scenario: "strict".to_string(),
                vote: committee_vote_fixture("strict"),
            },
            advisory: CommitteeRecommendationEntry {
                scenario: "advisory".to_string(),
                vote: committee_vote_fixture("advisory"),
            },
        }
    }

    fn committee_vote_fixture(mode: &str) -> SecurityCommitteeVoteResult {
        SecurityCommitteeVoteResult {
            symbol: "601916.SH".to_string(),
            analysis_date: "2025-09-17".to_string(),
            evidence_version: "fixture".to_string(),
            committee_engine: "fixture-engine".to_string(),
            committee_mode: mode.to_string(),
            deliberation_seat_count: 3,
            risk_seat_count: 1,
            majority_vote: "approve".to_string(),
            majority_count: 3,
            final_decision: "approve".to_string(),
            final_action: "buy".to_string(),
            final_confidence: "medium".to_string(),
            approval_ratio: 1.0,
            quorum_met: true,
            veto_triggered: false,
            veto_role: None,
            votes: Vec::new(),
            conditions: Vec::new(),
            key_disagreements: Vec::new(),
            warnings: Vec::new(),
            meeting_digest: "fixture".to_string(),
        }
    }

    #[test]
    fn account_binding_resolver_reads_matching_allocation() {
        let binding = SecurityExecutionAccountBindingResolver::bind(
            &execution_request_fixture(),
            "601916.SH",
        )
        .expect("binding should succeed")
        .expect("binding should exist");

        assert_eq!(binding.portfolio_position_plan_ref, "portfolio-plan-1");
        assert_eq!(binding.allocation.suggested_tranche_pct, 0.08);
    }

    #[test]
    fn resolve_exit_reason_marks_open_positions() {
        let trades = vec![SecurityExecutionJournalTrade {
            trade_id: "trade-1".to_string(),
            trade_date: "2025-09-18".to_string(),
            side: "buy".to_string(),
            price: 62.4,
            position_pct_delta: 0.12,
            resulting_position_pct: 0.12,
            reason: "entry".to_string(),
            notes: Vec::new(),
        }];

        assert_eq!(
            resolve_exit_reason("open", &trades, "manual"),
            "position_still_open"
        );
    }

    #[test]
    fn build_security_execution_record_source_keeps_single_assembler_path() {
        let source = include_str!("security_execution_record.rs");
        let start = source
            .find("pub fn build_security_execution_record(")
            .expect("build function should exist");
        let end = source[start..]
            .find("fn build_execution_journal_request(")
            .map(|offset| start + offset)
            .expect("next function should exist");
        let function_body = &source[start..end];

        assert!(function_body.contains("SecurityExecutionRecordAssembler::new("));
        assert!(function_body.contains(".assemble()"));
        assert!(!function_body.contains("let store = StockHistoryStore::workspace_default()?;"));
    }

    #[test]
    fn execution_store_position_plan_record_projection_preserves_plan_identity() {
        let record = build_execution_store_position_plan_record(&position_plan_result_fixture());

        assert_eq!(record.position_plan_ref, "position-plan-1");
        assert_eq!(
            record.decision_ref,
            "execution-store-decision-ref:position-plan-1"
        );
        assert_eq!(
            record.approval_ref,
            "execution-store-approval-ref:position-plan-1"
        );
        assert_eq!(record.evidence_version, "fixture");
        assert_eq!(record.symbol, "601916.SH");
        assert_eq!(record.analysis_date, "2025-09-17");
        assert_eq!(record.position_action, "build");
        assert_eq!(record.starter_position_pct, 0.08);
        assert_eq!(record.max_position_pct, 0.12);
        assert_eq!(record.position_plan.entry_mode, "starter");
        assert_eq!(record.position_plan.add_on_trigger, "breakout");
        assert_eq!(record.position_plan.reduce_on_trigger, "loss_of_momentum");
    }

    #[test]
    fn security_execution_record_source_uses_session_backed_grouped_runtime_writes() {
        let source = include_str!("security_execution_record.rs");
        let start = source
            .find("pub fn security_execution_record(")
            .expect("security_execution_record function should exist");
        let end = source[start..]
            .find("pub fn build_security_execution_record(")
            .map(|offset| start + offset)
            .expect("next function should exist");
        let function_body = &source[start..end];

        assert!(function_body.contains("build_execution_store_position_plan_record("));
        assert!(function_body.contains("let session = store.open_session()?;"));
        assert!(function_body.contains("session.upsert_position_plan(&position_plan_record)?;"));
        assert!(function_body.contains("session.upsert_execution_record(&execution_record)?;"));
        assert!(function_body.contains("session.commit()?;"));
        assert!(!function_body.contains("store.upsert_execution_record(&execution_record)?;"));
    }

    #[test]
    fn assembler_builds_stable_ascii_attribution_summary() {
        let store = isolated_stock_store();
        let result = with_stock_runtime(&store, || {
            build_security_execution_record(
                &position_plan_result_fixture(),
                &execution_record_outcome_fixture(),
                &execution_journal_fixture(),
                &execution_request_fixture(),
            )
        })
        .expect("execution record should build");

        assert!(
            result
                .attribution_summary
                .starts_with("current_position=0.00%, actual_return=")
        );
        assert!(result.attribution_summary.contains("return_gap="));
        assert!(result.attribution_summary.contains("entry_slippage="));
        assert!(result.attribution_summary.contains("position_gap="));
        assert!(result.attribution_summary.contains("quality="));
        assert!(result.attribution_summary.is_ascii());
    }
}
