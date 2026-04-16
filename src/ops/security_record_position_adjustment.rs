use thiserror::Error;

use crate::runtime::security_execution_store::SecurityExecutionStore;
use crate::runtime::security_execution_store::SecurityExecutionStoreError;
use crate::tools::contracts::{
    PositionAdjustmentEventType, SecurityRecordPositionAdjustmentRequest,
    SecurityRecordPositionAdjustmentResult,
};

#[derive(Debug, Error)]
pub enum SecurityRecordPositionAdjustmentError {
    #[error("security_record_position_adjustment 缺少 decision_ref")]
    MissingDecisionRef,
    #[error("security_record_position_adjustment 缺少 approval_ref")]
    MissingApprovalRef,
    #[error("security_record_position_adjustment 缺少 evidence_version")]
    MissingEvidenceVersion,
    #[error("security_record_position_adjustment 缺少 position_plan_ref")]
    MissingPositionPlanRef,
    #[error("security_record_position_adjustment 缺少 symbol")]
    MissingSymbol,
    #[error("security_record_position_adjustment 缺少 event_date")]
    MissingEventDate,
    #[error("security_record_position_adjustment 缺少 trigger_reason")]
    MissingTriggerReason,
    #[error("{0}")]
    Store(#[from] SecurityExecutionStoreError),
}

// 2026-04-08 CST: 这里新增正式调仓事件记录 Tool，原因是证券主链已经进入“投中执行”阶段，
// 目的：让同一 position_plan_ref 下的实际加仓、减仓、退出等动作可以沿正式 Tool 主链记录，而不是继续停留在对话层临时文本里。
pub fn security_record_position_adjustment(
    request: &SecurityRecordPositionAdjustmentRequest,
) -> Result<SecurityRecordPositionAdjustmentResult, SecurityRecordPositionAdjustmentError> {
    validate_position_adjustment_request(request)?;

    // 2026-04-08 CST: 这里先用确定性事件引用规则生成 adjustment_event_ref，原因是 Task 4 只要求最小可引用事件对象，
    // 目的：先给后续 post_trade_review 提供稳定事件锚点，同时避免本轮提前引入 runtime 持久化复杂度。
    let adjustment_event_ref = format!(
        "position-adjustment:{}:{}:{}:v1",
        request.symbol.trim(),
        request.event_date.trim(),
        event_type_label(&request.event_type)
    );

    let result =
        SecurityRecordPositionAdjustmentResult::from_request(adjustment_event_ref, request.clone());
    // 2026-04-08 CST: 这里把正式调仓事件落到执行层 runtime，原因是投后复盘必须能顺着 adjustment_event_ref 聚合真实事件链；
    // 目的：避免调仓 Tool 只返回一个 ref 却没有后续可回读事实，导致 review 仍然只能做伪聚合。
    let store = SecurityExecutionStore::workspace_default()?;
    // 2026-04-15 CST: Route this high-level write through the internal session boundary because
    // round 2 plan B now requires formal stock ops to converge on one governed transaction entry.
    // Purpose: keep adjustment-event persistence aligned with the execution-record bridge instead
    // of leaving a second high-level write path on direct facade writes.
    let session = store.open_session()?;
    session.upsert_adjustment_event(&result)?;
    session.commit()?;
    Ok(result)
}

// 2026-04-08 CST: 这里集中校验调仓事件请求边界，原因是 decision/approval/evidence/plan_ref 会成为后续复盘与审批回指主键，
// 目的：先把最小引用锚点和执行原因收口到单点校验，避免 dispatcher、测试和后续 Skill 各自脑补边界规则。
fn validate_position_adjustment_request(
    request: &SecurityRecordPositionAdjustmentRequest,
) -> Result<(), SecurityRecordPositionAdjustmentError> {
    if request.decision_ref.trim().is_empty() {
        return Err(SecurityRecordPositionAdjustmentError::MissingDecisionRef);
    }
    if request.approval_ref.trim().is_empty() {
        return Err(SecurityRecordPositionAdjustmentError::MissingApprovalRef);
    }
    if request.evidence_version.trim().is_empty() {
        return Err(SecurityRecordPositionAdjustmentError::MissingEvidenceVersion);
    }
    if request.position_plan_ref.trim().is_empty() {
        return Err(SecurityRecordPositionAdjustmentError::MissingPositionPlanRef);
    }
    if request.symbol.trim().is_empty() {
        return Err(SecurityRecordPositionAdjustmentError::MissingSymbol);
    }
    if request.event_date.trim().is_empty() {
        return Err(SecurityRecordPositionAdjustmentError::MissingEventDate);
    }
    if request.trigger_reason.trim().is_empty() {
        return Err(SecurityRecordPositionAdjustmentError::MissingTriggerReason);
    }

    Ok(())
}

// 2026-04-08 CST: 这里补最小事件类型标签转换，原因是 adjustment_event_ref 需要稳定、可读且和合同枚举值一致，
// 目的：避免后续 ref 拼接口径散落在多个 dispatcher 或测试里，导致事件引用命名漂移。
fn event_type_label(event_type: &PositionAdjustmentEventType) -> &'static str {
    match event_type {
        PositionAdjustmentEventType::Build => "build",
        PositionAdjustmentEventType::Add => "add",
        PositionAdjustmentEventType::Reduce => "reduce",
        PositionAdjustmentEventType::Exit => "exit",
        PositionAdjustmentEventType::RiskUpdate => "risk_update",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::contracts::PositionPlanAlignment;

    #[test]
    fn event_type_label_keeps_runtime_ref_suffix_stable() {
        assert_eq!(
            event_type_label(&PositionAdjustmentEventType::Build),
            "build"
        );
        assert_eq!(event_type_label(&PositionAdjustmentEventType::Add), "add");
        assert_eq!(
            event_type_label(&PositionAdjustmentEventType::Reduce),
            "reduce"
        );
        assert_eq!(event_type_label(&PositionAdjustmentEventType::Exit), "exit");
        assert_eq!(
            event_type_label(&PositionAdjustmentEventType::RiskUpdate),
            "risk_update"
        );
    }

    #[test]
    fn security_record_position_adjustment_source_uses_session_write_path() {
        let source = include_str!("security_record_position_adjustment.rs");
        let start = source
            .find("pub fn security_record_position_adjustment(")
            .expect("security_record_position_adjustment function should exist");
        let end = source[start..]
            .find("fn validate_position_adjustment_request(")
            .map(|offset| start + offset)
            .expect("next function should exist");
        let function_body = &source[start..end];

        assert!(function_body.contains("let session = store.open_session()?;"));
        assert!(function_body.contains("session.upsert_adjustment_event(&result)?;"));
        assert!(function_body.contains("session.commit()?;"));
        assert!(!function_body.contains("store.upsert_adjustment_event(&result)?;"));
    }

    #[test]
    fn validate_position_adjustment_request_rejects_missing_trigger_reason() {
        let request = SecurityRecordPositionAdjustmentRequest {
            decision_ref: "decision-1".to_string(),
            approval_ref: "approval-1".to_string(),
            evidence_version: "v1".to_string(),
            position_plan_ref: "plan-1".to_string(),
            symbol: "601916.SH".to_string(),
            event_type: PositionAdjustmentEventType::Add,
            event_date: "2026-04-15".to_string(),
            before_position_pct: 0.05,
            after_position_pct: 0.08,
            trigger_reason: String::new(),
            plan_alignment: PositionPlanAlignment::OnPlan,
        };

        let error = validate_position_adjustment_request(&request)
            .expect_err("empty trigger reason should be rejected");
        assert!(matches!(
            error,
            SecurityRecordPositionAdjustmentError::MissingTriggerReason
        ));
    }
}
