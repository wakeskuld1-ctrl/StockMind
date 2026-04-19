use serde::{Deserialize, Serialize};
use thiserror::Error;

const SECURITY_APPROVED_OPEN_POSITION_PACKET_DOCUMENT_TYPE: &str =
    "security_approved_open_position_packet";
const APPROVED_STATUS: &str = "approved";

// 2026-04-18 CST: Added because Task 1 freezes the only formal post-open intake
// contract for the pure data-side position-management system.
// Reason: the approved design requires one explicit packet boundary after the
// committee and chair chain finish approval, instead of allowing downstream
// modules to reconstruct state from scattered approval artifacts.
// Purpose: keep post-open processing anchored on one governed, auditable request shell.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityApprovedOpenPositionPacketRequest {
    pub packet_id: String,
    pub account_id: String,
    pub approval_session_id: String,
    pub approval_status: String,
    pub approved_at: String,
    pub effective_trade_date: String,
    pub capital_base_amount: f64,
    pub intended_principal_amount: f64,
    pub target_annual_return_pct: f64,
    pub max_drawdown_pct: f64,
    pub min_cash_reserve_pct: f64,
    pub max_single_position_pct: f64,
    pub max_sector_exposure_pct: f64,
    pub max_portfolio_risk_budget_pct: f64,
    pub max_single_trade_risk_budget_pct: f64,
    pub symbol: String,
    #[serde(default)]
    pub security_name: Option<String>,
    pub direction: String,
    pub recommended_entry_mode: String,
    pub recommended_starter_weight_pct: f64,
    pub recommended_target_weight_pct: f64,
    pub recommended_max_weight_pct: f64,
    pub expected_annual_return_pct: f64,
    pub expected_drawdown_pct: f64,
    pub position_management_ready: bool,
    pub entry_thesis: String,
    pub add_condition_summary: String,
    pub trim_condition_summary: String,
    pub replace_condition_summary: String,
    pub exit_condition_summary: String,
    pub target_achievement_condition: String,
    pub committee_resolution_ref: String,
    pub chair_resolution_ref: String,
    pub source_packet_version: String,
}

// 2026-04-18 CST: Added because the intake boundary must return a stable
// normalized packet instead of echoing unvalidated user input back to callers.
// Reason: later post-open modules should consume one sanitized document type
// with fixed identity fields and trimmed governance anchors.
// Purpose: define the minimal approved packet that downstream position-management
// tasks can trust as the only formal active intake object.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityApprovedOpenPositionPacketDocument {
    pub document_type: String,
    pub contract_version: String,
    pub packet_id: String,
    pub account_id: String,
    pub approval_session_id: String,
    pub approval_status: String,
    pub approved_at: String,
    pub effective_trade_date: String,
    pub capital_base_amount: f64,
    pub intended_principal_amount: f64,
    pub target_annual_return_pct: f64,
    pub max_drawdown_pct: f64,
    pub min_cash_reserve_pct: f64,
    pub max_single_position_pct: f64,
    pub max_sector_exposure_pct: f64,
    pub max_portfolio_risk_budget_pct: f64,
    pub max_single_trade_risk_budget_pct: f64,
    pub symbol: String,
    pub security_name: Option<String>,
    pub direction: String,
    pub recommended_entry_mode: String,
    pub recommended_starter_weight_pct: f64,
    pub recommended_target_weight_pct: f64,
    pub recommended_max_weight_pct: f64,
    pub expected_annual_return_pct: f64,
    pub expected_drawdown_pct: f64,
    pub position_management_ready: bool,
    pub entry_thesis: String,
    pub add_condition_summary: String,
    pub trim_condition_summary: String,
    pub replace_condition_summary: String,
    pub exit_condition_summary: String,
    pub target_achievement_condition: String,
    pub committee_resolution_ref: String,
    pub chair_resolution_ref: String,
}

// 2026-04-18 CST: Added because Task 1 needs one explicit error boundary for the
// approved intake contract before more post-open objects are introduced.
// Reason: CLI callers must fail fast on governance-gate violations rather than
// receiving a silently normalized packet that looks executable.
// Purpose: keep the intake boundary auditable and predictable during daily ops.
#[derive(Debug, Error)]
pub enum SecurityApprovedOpenPositionPacketError {
    #[error(
        "security approved open position packet validation failed: packet_id must not be empty"
    )]
    MissingPacketId,
    #[error(
        "security approved open position packet validation failed: account_id must not be empty"
    )]
    MissingAccountId,
    #[error(
        "security approved open position packet validation failed: approval_session_id must not be empty"
    )]
    MissingApprovalSessionId,
    #[error(
        "security approved open position packet validation failed: approval_status must be approved"
    )]
    ApprovalStatusNotApproved,
    #[error(
        "security approved open position packet validation failed: position_management_ready must be true"
    )]
    PositionManagementNotReady,
    #[error(
        "security approved open position packet validation failed: committee_resolution_ref must not be empty"
    )]
    MissingCommitteeResolutionRef,
    #[error(
        "security approved open position packet validation failed: chair_resolution_ref must not be empty"
    )]
    MissingChairResolutionRef,
    #[error(
        "security approved open position packet validation failed: source_packet_version must not be empty"
    )]
    MissingSourcePacketVersion,
}

// 2026-04-18 CST: Added because the post-open data system starts only after one
// approved packet passes all hard governance gates.
// Reason: the user explicitly fixed approval status, readiness, and governance
// references as mandatory entry conditions for downstream position management.
// Purpose: validate and normalize the approved intake packet into one stable document.
pub fn security_approved_open_position_packet(
    request: &SecurityApprovedOpenPositionPacketRequest,
) -> Result<SecurityApprovedOpenPositionPacketDocument, SecurityApprovedOpenPositionPacketError> {
    // 2026-04-18 CST: Updated because the next Task 1 boundary review showed
    // that normalized blank identity fields were still accepted as valid packets.
    // Reason: packet/account/session/version anchors are mandatory for the later
    // contract, monitoring, and audit layers to stay traceable.
    // Purpose: fail fast on empty normalized identity fields before any packet is emitted.
    let packet_id = normalize_text(&request.packet_id);
    if packet_id.is_empty() {
        return Err(SecurityApprovedOpenPositionPacketError::MissingPacketId);
    }

    let account_id = normalize_text(&request.account_id);
    if account_id.is_empty() {
        return Err(SecurityApprovedOpenPositionPacketError::MissingAccountId);
    }

    let approval_session_id = normalize_text(&request.approval_session_id);
    if approval_session_id.is_empty() {
        return Err(SecurityApprovedOpenPositionPacketError::MissingApprovalSessionId);
    }

    let source_packet_version = normalize_text(&request.source_packet_version);
    if source_packet_version.is_empty() {
        return Err(SecurityApprovedOpenPositionPacketError::MissingSourcePacketVersion);
    }

    let normalized_approval_status = normalize_lowercase(&request.approval_status);
    if normalized_approval_status != APPROVED_STATUS {
        return Err(SecurityApprovedOpenPositionPacketError::ApprovalStatusNotApproved);
    }

    if !request.position_management_ready {
        return Err(SecurityApprovedOpenPositionPacketError::PositionManagementNotReady);
    }

    let committee_resolution_ref = normalize_text(&request.committee_resolution_ref);
    if committee_resolution_ref.is_empty() {
        return Err(SecurityApprovedOpenPositionPacketError::MissingCommitteeResolutionRef);
    }

    let chair_resolution_ref = normalize_text(&request.chair_resolution_ref);
    if chair_resolution_ref.is_empty() {
        return Err(SecurityApprovedOpenPositionPacketError::MissingChairResolutionRef);
    }

    Ok(SecurityApprovedOpenPositionPacketDocument {
        document_type: SECURITY_APPROVED_OPEN_POSITION_PACKET_DOCUMENT_TYPE.to_string(),
        contract_version: source_packet_version,
        packet_id,
        account_id,
        approval_session_id,
        approval_status: normalized_approval_status,
        approved_at: normalize_text(&request.approved_at),
        effective_trade_date: normalize_text(&request.effective_trade_date),
        capital_base_amount: request.capital_base_amount,
        intended_principal_amount: request.intended_principal_amount,
        target_annual_return_pct: request.target_annual_return_pct,
        max_drawdown_pct: request.max_drawdown_pct,
        min_cash_reserve_pct: request.min_cash_reserve_pct,
        max_single_position_pct: request.max_single_position_pct,
        max_sector_exposure_pct: request.max_sector_exposure_pct,
        max_portfolio_risk_budget_pct: request.max_portfolio_risk_budget_pct,
        max_single_trade_risk_budget_pct: request.max_single_trade_risk_budget_pct,
        symbol: normalize_symbol(&request.symbol),
        security_name: normalize_optional_text(&request.security_name),
        direction: normalize_lowercase(&request.direction),
        recommended_entry_mode: normalize_lowercase(&request.recommended_entry_mode),
        recommended_starter_weight_pct: request.recommended_starter_weight_pct,
        recommended_target_weight_pct: request.recommended_target_weight_pct,
        recommended_max_weight_pct: request.recommended_max_weight_pct,
        expected_annual_return_pct: request.expected_annual_return_pct,
        expected_drawdown_pct: request.expected_drawdown_pct,
        position_management_ready: request.position_management_ready,
        entry_thesis: normalize_text(&request.entry_thesis),
        add_condition_summary: normalize_text(&request.add_condition_summary),
        trim_condition_summary: normalize_text(&request.trim_condition_summary),
        replace_condition_summary: normalize_text(&request.replace_condition_summary),
        exit_condition_summary: normalize_text(&request.exit_condition_summary),
        target_achievement_condition: normalize_text(&request.target_achievement_condition),
        committee_resolution_ref,
        chair_resolution_ref,
    })
}

// 2026-04-18 CST: Added because the approved packet should store stable trimmed
// string values without repeating ad-hoc whitespace logic in every field mapping.
// Reason: the data-side system will reuse the normalized packet many times, so
// the first boundary should remove layout noise once.
// Purpose: centralize the plain-text normalization rule for the intake contract.
fn normalize_text(value: &str) -> String {
    value.trim().to_string()
}

// 2026-04-18 CST: Added because status-like packet fields should normalize into
// one lowercase canonical form at the intake boundary.
// Reason: approval and direction checks should not depend on caller casing.
// Purpose: keep enum-like text fields deterministic for downstream consumers.
fn normalize_lowercase(value: &str) -> String {
    normalize_text(value).to_ascii_lowercase()
}

// 2026-04-18 CST: Added because A-share and cross-market symbols should keep a
// predictable uppercase canonical representation across downstream documents.
// Reason: symbol identity is a join key for later position contracts and monitoring packages.
// Purpose: normalize the primary security code at the first post-open boundary.
fn normalize_symbol(value: &str) -> String {
    normalize_text(value).to_ascii_uppercase()
}

// 2026-04-18 CST: Added because optional text fields still need trimming when
// present, but should remain absent when callers send empty content.
// Reason: storing Some("") would make downstream packet comparisons noisy and brittle.
// Purpose: normalize optional packet text without widening the public contract.
fn normalize_optional_text(value: &Option<String>) -> Option<String> {
    value.as_ref().and_then(|inner| {
        let normalized = normalize_text(inner);
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    })
}
