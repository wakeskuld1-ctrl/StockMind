use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_monitoring_evidence_package::SecurityMonitoringEvidencePackage;
use crate::ops::stock::security_portfolio_position_plan::{
    SecurityCapitalRebalanceSimulationItem, build_capital_rebalance_simulation,
};
use crate::ops::stock::security_position_contract::{
    SecurityPositionContract, rebase_security_position_contract_with_overrides,
};

const SECURITY_CAPITAL_EVENT_DOCUMENT_TYPE: &str = "security_capital_event";
const SECURITY_CAPITAL_EVENT_VERSION: &str = "security_capital_event.v1";
const SECURITY_ACCOUNT_REBASE_SNAPSHOT_DOCUMENT_TYPE: &str = "security_account_rebase_snapshot";
const SECURITY_ACCOUNT_REBASE_SNAPSHOT_VERSION: &str = "security_account_rebase_snapshot.v1";
const SECURITY_CAPITAL_REBALANCE_EVIDENCE_DOCUMENT_TYPE: &str =
    "security_capital_rebalance_evidence_package";
const SECURITY_CAPITAL_REBALANCE_EVIDENCE_VERSION: &str =
    "security_capital_rebalance_evidence_package.v1";

// 2026-04-19 CST: Added because Task 6 needs one thin public input for the
// capital-event object before the rest of the rebase chain is built.
// Reason: the approved design treats capital changes as first-class account events
// instead of hidden mutations inside later package builders.
// Purpose: freeze the input contract for governed capital-event normalization.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCapitalEventInput {
    pub event_id: String,
    pub account_id: String,
    pub event_type: String,
    pub event_amount: f64,
    pub effective_date: String,
    pub capital_before_event: f64,
    pub policy_tag: String,
    #[serde(default)]
    pub target_annual_return_pct_after: Option<f64>,
    #[serde(default)]
    pub max_drawdown_pct_after: Option<f64>,
    #[serde(default)]
    pub min_cash_reserve_pct_before: Option<f64>,
    #[serde(default)]
    pub min_cash_reserve_pct_after: Option<f64>,
    #[serde(default)]
    pub max_single_position_pct_after: Option<f64>,
    #[serde(default)]
    pub max_single_trade_risk_budget_pct_after: Option<f64>,
    #[serde(default)]
    pub notes: Option<String>,
}

// 2026-04-19 CST: Added because Task 6 should return a normalized capital event
// document instead of exposing the raw input payload directly.
// Reason: later rebase snapshots and evidence packages should anchor themselves
// on one auditable event object with computed before/after capital state.
// Purpose: define the formal capital-event document.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCapitalEvent {
    pub document_type: String,
    pub contract_version: String,
    pub event_id: String,
    pub account_id: String,
    pub event_type: String,
    pub event_amount: f64,
    pub effective_date: String,
    pub capital_before_event: f64,
    pub capital_after_event: f64,
    pub policy_tag: String,
    #[serde(default)]
    pub target_annual_return_pct_after: Option<f64>,
    #[serde(default)]
    pub max_drawdown_pct_after: Option<f64>,
    #[serde(default)]
    pub min_cash_reserve_pct_before: Option<f64>,
    #[serde(default)]
    pub min_cash_reserve_pct_after: Option<f64>,
    #[serde(default)]
    pub max_single_position_pct_after: Option<f64>,
    #[serde(default)]
    pub max_single_trade_risk_budget_pct_after: Option<f64>,
    #[serde(default)]
    pub notes: Option<String>,
}

// 2026-04-19 CST: Added because the capital rebase tool should gather all
// upstream objects needed for one account-level rebasing pass in a single request.
// Reason: later governance consumers should not have to coordinate event, package,
// and contract payloads through multiple ad hoc calls.
// Purpose: define the public request shell for Task 6.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCapitalRebaseRequest {
    pub capital_event_input: SecurityCapitalEventInput,
    pub monitoring_evidence_package: SecurityMonitoringEvidencePackage,
    pub position_contracts: Vec<SecurityPositionContract>,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-19 CST: Added because Task 6 needs one explicit account-level snapshot
// capturing the before/after rebase state after a capital event lands.
// Reason: the approved design makes account rebasing a governed data artifact,
// not an invisible side effect of contract mutation.
// Purpose: define the formal account rebase snapshot.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityAccountRebaseSnapshot {
    pub account_rebase_snapshot_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub account_id: String,
    pub capital_event_ref: String,
    pub source_monitoring_evidence_package_ref: String,
    pub target_annual_return_pct_before: f64,
    pub target_annual_return_pct_after: f64,
    pub max_drawdown_pct_before: f64,
    pub max_drawdown_pct_after: f64,
    pub min_cash_reserve_pct_before: f64,
    pub min_cash_reserve_pct_after: f64,
    pub risk_budget_pct_before: f64,
    pub risk_budget_pct_after: f64,
    pub rebase_policy: String,
    pub rebase_required: bool,
    #[serde(default)]
    pub rebased_position_contracts: Vec<SecurityPositionContract>,
    #[serde(default)]
    pub rebase_completed_at: Option<String>,
    #[serde(default)]
    pub rebase_evidence_package_ref: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

// 2026-04-19 CST: Added because Task 6 also needs a standardized evidence
// package that a future committee flow can review after the rebase snapshot exists.
// Reason: the design requires capital rebalance evidence to remain distinct from
// both ordinary monitoring evidence and direct execution input.
// Purpose: define the formal capital rebalance evidence package.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCapitalRebalanceEvidencePackage {
    pub capital_rebalance_evidence_package_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub account_id: String,
    pub capital_event_ref: String,
    pub account_rebase_snapshot_ref: String,
    pub source_monitoring_evidence_package_ref: String,
    pub rebase_policy: String,
    pub produces_execution_input: bool,
    #[serde(default)]
    pub adjustment_input_package: Option<String>,
    pub rebalance_simulation: Vec<SecurityCapitalRebalanceSimulationItem>,
    pub warnings: Vec<String>,
    pub package_status: String,
    pub package_summary: String,
}

// 2026-04-19 CST: Added because the public Task 6 tool should return one
// explicit result wrapper instead of three anonymous top-level values.
// Reason: this keeps the stock tool response style aligned with the earlier post-open layers.
// Purpose: wrap the capital event, rebase snapshot, and evidence package together.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCapitalRebaseResult {
    pub capital_event: SecurityCapitalEvent,
    pub account_rebase_snapshot: SecurityAccountRebaseSnapshot,
    pub capital_rebalance_evidence_package: SecurityCapitalRebalanceEvidencePackage,
}

// 2026-04-19 CST: Added because Task 6 needs one explicit error boundary for
// capital-event normalization and account rebasing.
// Reason: account mismatches and invalid capital arithmetic should fail before
// any downstream evidence package is emitted.
// Purpose: keep Task 6 failures auditable and deterministic.
#[derive(Debug, Error)]
pub enum SecurityCapitalRebaseError {
    #[error("security capital event build failed: unsupported event_type `{0}`")]
    UnsupportedEventType(String),
    #[error("security capital event build failed: event_amount must be positive")]
    NonPositiveEventAmount,
    #[error(
        "security capital event build failed: capital_after_event must not be negative after `{0}`"
    )]
    NegativeCapitalAfterEvent(String),
    #[error(
        "security capital rebase build failed: monitoring package account `{0}` does not match capital event account `{1}`"
    )]
    MonitoringAccountMismatch(String, String),
    #[error(
        "security capital rebase build failed: position contract `{0}` does not belong to capital event account `{1}`"
    )]
    PositionContractAccountMismatch(String, String),
}

// 2026-04-19 CST: Added because Task 6 needs one formal tool entry that builds
// the whole capital-event rebase chain end to end.
// Reason: callers should not manually sequence event normalization, snapshot building,
// and evidence package creation outside the governed module.
// Purpose: expose the Task 6 orchestration entry.
pub fn security_capital_rebase(
    request: &SecurityCapitalRebaseRequest,
) -> Result<SecurityCapitalRebaseResult, SecurityCapitalRebaseError> {
    let capital_event = build_security_capital_event(&request.capital_event_input)?;
    let mut account_rebase_snapshot = build_security_account_rebase_snapshot(
        &capital_event,
        &request.monitoring_evidence_package,
        &request.position_contracts,
        &request.created_at,
    )?;
    let capital_rebalance_evidence_package = build_security_capital_rebalance_evidence_package(
        &capital_event,
        &account_rebase_snapshot,
        &request.monitoring_evidence_package,
        &request.position_contracts,
        &request.created_at,
    )?;
    account_rebase_snapshot.rebase_completed_at = Some(normalize_created_at(&request.created_at));
    account_rebase_snapshot.rebase_evidence_package_ref = Some(
        capital_rebalance_evidence_package
            .capital_rebalance_evidence_package_id
            .clone(),
    );

    Ok(SecurityCapitalRebaseResult {
        capital_event,
        account_rebase_snapshot,
        capital_rebalance_evidence_package,
    })
}

// 2026-04-19 CST: Added because Task 6 needs one deterministic builder that
// converts raw capital-event input into a normalized governed event document.
// Reason: the capital-event object is the formal entry to the rebase path.
// Purpose: centralize capital-before/after computation and event normalization.
pub fn build_security_capital_event(
    input: &SecurityCapitalEventInput,
) -> Result<SecurityCapitalEvent, SecurityCapitalRebaseError> {
    if input.event_amount <= 0.0 {
        return Err(SecurityCapitalRebaseError::NonPositiveEventAmount);
    }

    let normalized_event_type = input.event_type.trim().to_ascii_lowercase();
    let capital_after_event = match normalized_event_type.as_str() {
        "add_capital" | "dividend_reinvest" => input.capital_before_event + input.event_amount,
        "withdraw_capital" | "external_cash_out" => input.capital_before_event - input.event_amount,
        _ => {
            return Err(SecurityCapitalRebaseError::UnsupportedEventType(
                input.event_type.clone(),
            ));
        }
    };

    if capital_after_event < 0.0 {
        return Err(SecurityCapitalRebaseError::NegativeCapitalAfterEvent(
            normalized_event_type,
        ));
    }

    Ok(SecurityCapitalEvent {
        document_type: SECURITY_CAPITAL_EVENT_DOCUMENT_TYPE.to_string(),
        contract_version: SECURITY_CAPITAL_EVENT_VERSION.to_string(),
        event_id: input.event_id.trim().to_string(),
        account_id: input.account_id.trim().to_string(),
        event_type: normalized_event_type,
        event_amount: round_amount(input.event_amount),
        effective_date: input.effective_date.trim().to_string(),
        capital_before_event: round_amount(input.capital_before_event),
        capital_after_event: round_amount(capital_after_event),
        policy_tag: input.policy_tag.trim().to_string(),
        target_annual_return_pct_after: input.target_annual_return_pct_after,
        max_drawdown_pct_after: input.max_drawdown_pct_after,
        min_cash_reserve_pct_before: input.min_cash_reserve_pct_before,
        min_cash_reserve_pct_after: input.min_cash_reserve_pct_after,
        max_single_position_pct_after: input.max_single_position_pct_after,
        max_single_trade_risk_budget_pct_after: input.max_single_trade_risk_budget_pct_after,
        notes: normalize_optional_text(&input.notes),
    })
}

// 2026-04-19 CST: Added because the capital-event flow needs one governed
// snapshot showing how account metrics and live contracts changed.
// Reason: future governance review should read one explicit rebase snapshot instead
// of inferring before/after deltas from raw contracts.
// Purpose: build the formal account rebase snapshot for Task 6.
pub fn build_security_account_rebase_snapshot(
    capital_event: &SecurityCapitalEvent,
    monitoring_evidence_package: &SecurityMonitoringEvidencePackage,
    position_contracts: &[SecurityPositionContract],
    created_at: &str,
) -> Result<SecurityAccountRebaseSnapshot, SecurityCapitalRebaseError> {
    if monitoring_evidence_package.account_id != capital_event.account_id {
        return Err(SecurityCapitalRebaseError::MonitoringAccountMismatch(
            monitoring_evidence_package.account_id.clone(),
            capital_event.account_id.clone(),
        ));
    }

    let rebased_position_contracts =
        build_rebased_position_contracts(capital_event, position_contracts, created_at)?;
    let target_annual_return_pct_before = monitoring_evidence_package
        .account_aggregation
        .weighted_expected_return_pct;
    let target_annual_return_pct_after = capital_event
        .target_annual_return_pct_after
        .unwrap_or(target_annual_return_pct_before);
    let max_drawdown_pct_before = monitoring_evidence_package
        .account_aggregation
        .weighted_expected_drawdown_pct;
    let max_drawdown_pct_after = capital_event
        .max_drawdown_pct_after
        .unwrap_or(max_drawdown_pct_before);
    let min_cash_reserve_pct_before = capital_event.min_cash_reserve_pct_before.unwrap_or(0.0);
    let min_cash_reserve_pct_after = capital_event
        .min_cash_reserve_pct_after
        .unwrap_or(min_cash_reserve_pct_before);
    let risk_budget_pct_before = monitoring_evidence_package
        .account_aggregation
        .total_risk_budget_pct;
    let risk_budget_pct_after = round_pct(
        rebased_position_contracts
            .iter()
            .map(|contract| contract.risk_budget_pct)
            .sum(),
    );
    let rebase_policy = rebased_position_contracts
        .first()
        .map(|contract| contract.rebase_policy.clone())
        .unwrap_or_else(|| "proportional_rebase_on_capital_event.v1".to_string());
    let rebase_required = (capital_event.capital_after_event - capital_event.capital_before_event)
        .abs()
        > f64::EPSILON
        || capital_event.max_single_position_pct_after.is_some()
        || capital_event
            .max_single_trade_risk_budget_pct_after
            .is_some();

    Ok(SecurityAccountRebaseSnapshot {
        account_rebase_snapshot_id: format!(
            "account-rebase-snapshot:{}:{}",
            capital_event.account_id,
            normalize_created_at(created_at)
        ),
        contract_version: SECURITY_ACCOUNT_REBASE_SNAPSHOT_VERSION.to_string(),
        document_type: SECURITY_ACCOUNT_REBASE_SNAPSHOT_DOCUMENT_TYPE.to_string(),
        generated_at: normalize_created_at(created_at),
        account_id: capital_event.account_id.clone(),
        capital_event_ref: capital_event.event_id.clone(),
        source_monitoring_evidence_package_ref: monitoring_evidence_package
            .monitoring_evidence_package_id
            .clone(),
        target_annual_return_pct_before: round_pct(target_annual_return_pct_before),
        target_annual_return_pct_after: round_pct(target_annual_return_pct_after),
        max_drawdown_pct_before: round_pct(max_drawdown_pct_before),
        max_drawdown_pct_after: round_pct(max_drawdown_pct_after),
        min_cash_reserve_pct_before: round_pct(min_cash_reserve_pct_before),
        min_cash_reserve_pct_after: round_pct(min_cash_reserve_pct_after),
        risk_budget_pct_before: round_pct(risk_budget_pct_before),
        risk_budget_pct_after,
        rebase_policy,
        rebase_required,
        rebased_position_contracts,
        rebase_completed_at: None,
        rebase_evidence_package_ref: None,
        notes: capital_event.notes.clone(),
    })
}

// 2026-04-19 CST: Added because Task 6 also needs one standardized evidence
// package showing the contract deltas caused by a capital event.
// Reason: capital rebalance evidence must remain a governed handoff object before
// any downstream committee or chair decision can approve actions.
// Purpose: build the formal capital rebalance evidence package.
pub fn build_security_capital_rebalance_evidence_package(
    capital_event: &SecurityCapitalEvent,
    account_rebase_snapshot: &SecurityAccountRebaseSnapshot,
    monitoring_evidence_package: &SecurityMonitoringEvidencePackage,
    position_contracts: &[SecurityPositionContract],
    created_at: &str,
) -> Result<SecurityCapitalRebalanceEvidencePackage, SecurityCapitalRebaseError> {
    let rebalance_simulation = build_capital_rebalance_simulation(
        position_contracts,
        &account_rebase_snapshot.rebased_position_contracts,
    );
    let mut warnings = monitoring_evidence_package.warnings.clone();
    if capital_event.event_type == "withdraw_capital"
        || capital_event.event_type == "external_cash_out"
    {
        warnings.push("capital_outflow_requires_governance_review".to_string());
    }
    warnings.sort();
    warnings.dedup();

    Ok(SecurityCapitalRebalanceEvidencePackage {
        capital_rebalance_evidence_package_id: format!(
            "capital-rebalance-evidence-package:{}:{}",
            capital_event.account_id,
            normalize_created_at(created_at)
        ),
        contract_version: SECURITY_CAPITAL_REBALANCE_EVIDENCE_VERSION.to_string(),
        document_type: SECURITY_CAPITAL_REBALANCE_EVIDENCE_DOCUMENT_TYPE.to_string(),
        generated_at: normalize_created_at(created_at),
        account_id: capital_event.account_id.clone(),
        capital_event_ref: capital_event.event_id.clone(),
        account_rebase_snapshot_ref: account_rebase_snapshot.account_rebase_snapshot_id.clone(),
        source_monitoring_evidence_package_ref: monitoring_evidence_package
            .monitoring_evidence_package_id
            .clone(),
        rebase_policy: account_rebase_snapshot.rebase_policy.clone(),
        produces_execution_input: false,
        adjustment_input_package: None,
        rebalance_simulation,
        warnings,
        package_status: "ready_for_committee_review".to_string(),
        package_summary: format!(
            "capital rebase evidence prepared for account {} across {} contracts",
            capital_event.account_id,
            account_rebase_snapshot.rebased_position_contracts.len()
        ),
    })
}

fn build_rebased_position_contracts(
    capital_event: &SecurityCapitalEvent,
    position_contracts: &[SecurityPositionContract],
    created_at: &str,
) -> Result<Vec<SecurityPositionContract>, SecurityCapitalRebaseError> {
    let mut rebased_position_contracts = Vec::with_capacity(position_contracts.len());

    for contract in position_contracts {
        if contract.account_id != capital_event.account_id {
            return Err(SecurityCapitalRebaseError::PositionContractAccountMismatch(
                contract.position_contract_id.clone(),
                capital_event.account_id.clone(),
            ));
        }

        let max_weight_pct_after = capital_event
            .max_single_position_pct_after
            .map(|cap| contract.max_weight_pct.min(cap));
        let risk_budget_pct_after = capital_event
            .max_single_trade_risk_budget_pct_after
            .map(|cap| contract.risk_budget_pct.min(cap));
        let rebased_contract = rebase_security_position_contract_with_overrides(
            contract,
            capital_event.capital_after_event,
            None,
            max_weight_pct_after,
            risk_budget_pct_after,
            created_at,
        );
        rebased_position_contracts.push(rebased_contract);
    }

    rebased_position_contracts.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    Ok(rebased_position_contracts)
}

fn normalize_created_at(value: &str) -> String {
    if value.trim().is_empty() {
        Utc::now().to_rfc3339()
    } else {
        value.trim().to_string()
    }
}

fn default_created_at() -> String {
    Utc::now().to_rfc3339()
}

fn normalize_optional_text(value: &Option<String>) -> Option<String> {
    value.as_ref().and_then(|inner| {
        let trimmed = inner.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn round_amount(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round_pct(value: f64) -> f64 {
    (value * 10000.0).round() / 10000.0
}
