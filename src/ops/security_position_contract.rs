use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_approved_open_position_packet::SecurityApprovedOpenPositionPacketDocument;
use crate::ops::stock::security_position_plan::{
    SecurityPositionContractSeed, SecurityPositionPlanDocument,
    build_position_contract_seed_from_documents,
};

const SECURITY_POSITION_CONTRACT_DOCUMENT_TYPE: &str = "security_position_contract";
const SECURITY_POSITION_CONTRACT_VERSION: &str = "security_position_contract.v1";
const DEFAULT_REBASE_POLICY: &str = "proportional_rebase_on_capital_event.v1";

// 2026-04-18 CST: Added because Task 2 introduces the only formal live
// governance object that may sit between approved intake and active holdings.
// Reason: the user explicitly fixed `PositionContract` as the post-open live
// contract layer and asked us not to reuse the pre-trade plan document directly.
// Purpose: freeze one thin public request surface for live contract formation.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPositionContractRequest {
    pub approved_open_position_packet: SecurityApprovedOpenPositionPacketDocument,
    pub position_plan_document: SecurityPositionPlanDocument,
    #[serde(default = "default_created_at")]
    pub created_at: String,
}

// 2026-04-18 CST: Added because Task 2 needs one stable live contract document
// carrying the governed weight, risk, and policy state for a single position.
// Reason: later monitoring, rebasing, and approved adjustments should all read
// one post-open contract instead of mixing raw packet data with seed fragments.
// Purpose: define the first formal `PositionContract` shell.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPositionContract {
    pub position_contract_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub packet_id: String,
    pub account_id: String,
    pub approval_session_id: String,
    pub symbol: String,
    #[serde(default)]
    pub security_name: Option<String>,
    pub analysis_date: String,
    pub effective_trade_date: String,
    pub direction: String,
    pub contract_status: String,
    pub entry_mode: String,
    pub initial_weight_pct: f64,
    pub target_weight_pct: f64,
    pub max_weight_pct: f64,
    pub capital_base_amount: f64,
    pub intended_principal_amount: f64,
    pub expected_annual_return_pct: f64,
    pub expected_drawdown_pct: f64,
    pub risk_budget_pct: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liquidity_guardrail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concentration_guardrail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_guardrail: Option<String>,
    pub add_policy: String,
    pub trim_policy: String,
    pub replace_policy: String,
    pub exit_policy: String,
    pub target_achievement_policy: String,
    pub rebase_policy: String,
    pub approval_binding_ref: String,
    pub source_position_plan_ref: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_rebased_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_reason: Option<String>,
}

// 2026-04-18 CST: Added because the public tool route should return a named
// result object instead of exposing an anonymous live contract directly.
// Reason: this keeps CLI responses aligned with the repository's existing tool
// response style and gives later tasks room to expand metadata safely.
// Purpose: wrap the first `PositionContract` document in a stable tool result.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityPositionContractResult {
    pub position_contract: SecurityPositionContract,
}

// 2026-04-18 CST: Added because Task 2 needs a contract-local error boundary
// before monitoring and rebasing layers depend on the live contract builder.
// Reason: callers should fail fast when packet and plan seeds disagree about
// the governed symbol or when the contract id surface cannot be trusted.
// Purpose: keep Task 2 failures explicit and auditable.
#[derive(Debug, Error)]
pub enum SecurityPositionContractError {
    #[error(
        "security position contract build failed: approved packet symbol and position plan symbol must match"
    )]
    SymbolMismatch,
}

// 2026-04-18 CST: Added because the live contract layer should be built from
// one approved packet plus one pre-trade plan seed, not from either object alone.
// Reason: the user explicitly separated intake normalization from live contract formation.
// Purpose: expose the public builder used by the new CLI tool route.
pub fn build_security_position_contract(
    request: &SecurityPositionContractRequest,
) -> Result<SecurityPositionContractResult, SecurityPositionContractError> {
    let seed = build_position_contract_seed_from_documents(
        &request.approved_open_position_packet,
        &request.position_plan_document,
    );
    let position_contract = build_security_position_contract_from_approved_packet(
        &request.approved_open_position_packet,
        &seed,
        &request.created_at,
    )?;

    Ok(SecurityPositionContractResult { position_contract })
}

// 2026-04-18 CST: Added because Task 2 needs a deterministic contract builder
// that transforms the approved packet into one governed live object.
// Reason: later tasks should modify one builder instead of reassembling the
// contract shape in multiple callers.
// Purpose: centralize the first live contract formation rule.
pub fn build_security_position_contract_from_approved_packet(
    approved_open_position_packet: &SecurityApprovedOpenPositionPacketDocument,
    seed: &SecurityPositionContractSeed,
    created_at: &str,
) -> Result<SecurityPositionContract, SecurityPositionContractError> {
    if approved_open_position_packet.symbol != seed.symbol {
        return Err(SecurityPositionContractError::SymbolMismatch);
    }

    Ok(SecurityPositionContract {
        position_contract_id: format!(
            "position-contract:{}:{}",
            approved_open_position_packet.account_id, approved_open_position_packet.packet_id
        ),
        contract_version: SECURITY_POSITION_CONTRACT_VERSION.to_string(),
        document_type: SECURITY_POSITION_CONTRACT_DOCUMENT_TYPE.to_string(),
        generated_at: normalize_created_at(created_at),
        packet_id: approved_open_position_packet.packet_id.clone(),
        account_id: approved_open_position_packet.account_id.clone(),
        approval_session_id: approved_open_position_packet.approval_session_id.clone(),
        symbol: approved_open_position_packet.symbol.clone(),
        security_name: approved_open_position_packet.security_name.clone(),
        analysis_date: seed.analysis_date.clone(),
        effective_trade_date: approved_open_position_packet.effective_trade_date.clone(),
        direction: approved_open_position_packet.direction.clone(),
        contract_status: "pending_open".to_string(),
        entry_mode: approved_open_position_packet.recommended_entry_mode.clone(),
        initial_weight_pct: approved_open_position_packet.recommended_starter_weight_pct,
        target_weight_pct: approved_open_position_packet.recommended_target_weight_pct,
        max_weight_pct: approved_open_position_packet.recommended_max_weight_pct,
        capital_base_amount: approved_open_position_packet.capital_base_amount,
        intended_principal_amount: approved_open_position_packet.intended_principal_amount,
        expected_annual_return_pct: approved_open_position_packet.expected_annual_return_pct,
        expected_drawdown_pct: approved_open_position_packet.expected_drawdown_pct,
        risk_budget_pct: seed.risk_budget_pct,
        liquidity_guardrail: seed.liquidity_guardrail.clone(),
        concentration_guardrail: seed.concentration_guardrail.clone(),
        correlation_guardrail: None,
        add_policy: approved_open_position_packet.add_condition_summary.clone(),
        trim_policy: approved_open_position_packet.trim_condition_summary.clone(),
        replace_policy: approved_open_position_packet
            .replace_condition_summary
            .clone(),
        exit_policy: approved_open_position_packet.exit_condition_summary.clone(),
        target_achievement_policy: approved_open_position_packet
            .target_achievement_condition
            .clone(),
        rebase_policy: DEFAULT_REBASE_POLICY.to_string(),
        approval_binding_ref: format!(
            "approval-binding:{}:{}:{}",
            approved_open_position_packet.approval_session_id,
            approved_open_position_packet.committee_resolution_ref,
            approved_open_position_packet.chair_resolution_ref
        ),
        source_position_plan_ref: seed.position_plan_ref.clone(),
        last_rebased_at: None,
        closed_reason: None,
    })
}

// 2026-04-18 CST: Added because the approved live contract will later need one
// governed rebasing path when account capital changes.
// Reason: the design makes capital rebasing a formal mutation path instead of
// letting downstream layers overwrite the contract ad hoc.
// Purpose: provide the first deterministic rebasing helper for later tasks.
pub fn rebase_security_position_contract(
    contract: &SecurityPositionContract,
    new_capital_base_amount: f64,
    rebased_at: &str,
) -> SecurityPositionContract {
    rebase_security_position_contract_with_overrides(
        contract,
        new_capital_base_amount,
        None,
        None,
        None,
        rebased_at,
    )
}

// 2026-04-19 CST: Added because Task 6 needs one governed rebasing helper that
// can also apply tighter post-event contract constraints when capital changes.
// Reason: the approved capital-event flow keeps target weights stable by default,
// but still allows event-level caps to tighten max weight and risk budget.
// Purpose: centralize capital-event-aware contract rebasing behind one deterministic helper.
pub fn rebase_security_position_contract_with_overrides(
    contract: &SecurityPositionContract,
    new_capital_base_amount: f64,
    target_weight_pct_after: Option<f64>,
    max_weight_pct_after: Option<f64>,
    risk_budget_pct_after: Option<f64>,
    rebased_at: &str,
) -> SecurityPositionContract {
    let mut rebased_contract = contract.clone();
    rebased_contract.contract_status = "rebasing".to_string();
    rebased_contract.capital_base_amount = new_capital_base_amount;
    if let Some(target_weight_pct_after) = target_weight_pct_after {
        rebased_contract.target_weight_pct = target_weight_pct_after;
    }
    if let Some(max_weight_pct_after) = max_weight_pct_after {
        rebased_contract.max_weight_pct = max_weight_pct_after;
    }
    if let Some(risk_budget_pct_after) = risk_budget_pct_after {
        rebased_contract.risk_budget_pct = risk_budget_pct_after;
    }
    rebased_contract.intended_principal_amount =
        round_amount(new_capital_base_amount * rebased_contract.target_weight_pct);
    rebased_contract.last_rebased_at = Some(normalize_created_at(rebased_at));
    rebased_contract
}

// 2026-04-18 CST: Added because the new contract layer should normalize its own
// generated timestamp in the same way as the earlier formal document shells.
// Reason: callers should not need to duplicate blank-time handling before they
// can form a contract.
// Purpose: centralize Task 2 timestamp normalization.
fn normalize_created_at(value: &str) -> String {
    if value.trim().is_empty() {
        Utc::now().to_rfc3339()
    } else {
        value.trim().to_string()
    }
}

// 2026-04-18 CST: Added because the rebasing helper should not leak long float
// tails into a contract field that later user-facing evidence will reuse.
// Reason: contract principal amounts are operational numbers, not raw compute intermediates.
// Purpose: keep rebased principal values stable and readable.
fn round_amount(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn default_created_at() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_REBASE_POLICY, SECURITY_POSITION_CONTRACT_DOCUMENT_TYPE,
        SECURITY_POSITION_CONTRACT_VERSION, SecurityPositionContract,
        rebase_security_position_contract,
    };

    // 2026-04-18 CST: Added because the first rebasing helper should lock its
    // contract-state mutation semantics before the dedicated capital-event task lands.
    // Reason: later capital rebasing should build on a proven helper instead of
    // silently changing status, capital base, or principal math.
    // Purpose: freeze the minimal rebasing behavior for Task 2.
    #[test]
    fn rebase_security_position_contract_updates_status_capital_and_timestamp() {
        let contract = SecurityPositionContract {
            position_contract_id: "position-contract:acct-1:packet-contract-1".to_string(),
            contract_version: SECURITY_POSITION_CONTRACT_VERSION.to_string(),
            document_type: SECURITY_POSITION_CONTRACT_DOCUMENT_TYPE.to_string(),
            generated_at: "2026-04-18T09:30:00+08:00".to_string(),
            packet_id: "packet-contract-1".to_string(),
            account_id: "acct-1".to_string(),
            approval_session_id: "approval-session-1".to_string(),
            symbol: "601916.SH".to_string(),
            security_name: Some("Zheshang Bank".to_string()),
            analysis_date: "2026-04-18".to_string(),
            effective_trade_date: "2026-04-18".to_string(),
            direction: "long".to_string(),
            contract_status: "active".to_string(),
            entry_mode: "probe".to_string(),
            initial_weight_pct: 0.03,
            target_weight_pct: 0.08,
            max_weight_pct: 0.12,
            capital_base_amount: 100000.0,
            intended_principal_amount: 8000.0,
            expected_annual_return_pct: 0.5,
            expected_drawdown_pct: 0.05,
            risk_budget_pct: 0.012,
            liquidity_guardrail: Some("daily_turnover_guardrail".to_string()),
            concentration_guardrail: Some(
                "single_position_cap=15.00%; sector_cap=30.00%".to_string(),
            ),
            correlation_guardrail: None,
            add_policy: "Add only after governance review.".to_string(),
            trim_policy: "Trim when risk-adjusted edge weakens.".to_string(),
            replace_policy: "Replace when a better candidate is approved.".to_string(),
            exit_policy: "Exit when thesis breaks.".to_string(),
            target_achievement_policy: "Target reached.".to_string(),
            rebase_policy: DEFAULT_REBASE_POLICY.to_string(),
            approval_binding_ref:
                "approval-binding:approval-session-1:committee-resolution-1:chair-resolution-1"
                    .to_string(),
            source_position_plan_ref: "position-plan-601916.SH-2026-04-18".to_string(),
            last_rebased_at: None,
            closed_reason: None,
        };

        let rebased_contract =
            rebase_security_position_contract(&contract, 150000.0, "2026-04-19T10:30:00+08:00");

        assert_eq!(rebased_contract.contract_status, "rebasing");
        assert_eq!(rebased_contract.capital_base_amount, 150000.0);
        assert_eq!(rebased_contract.intended_principal_amount, 12000.0);
        assert_eq!(
            rebased_contract.last_rebased_at.as_deref(),
            Some("2026-04-19T10:30:00+08:00")
        );
    }
}
