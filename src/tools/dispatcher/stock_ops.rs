use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::ops::stock::stock_data_pipeline::import_stock_price_history::{
    ImportStockPriceHistoryRequest, import_stock_price_history,
};
use crate::ops::stock::stock_execution_and_position_management::security_account_open_position_snapshot::{
    SecurityAccountOpenPositionSnapshotRequest, security_account_open_position_snapshot,
};
use crate::ops::stock::stock_execution_and_position_management::security_account_objective_contract::{
    SecurityAccountObjectiveContractRequest, security_account_objective_contract,
};
use crate::ops::stock::stock_execution_and_position_management::security_portfolio_replacement_plan::{
    SecurityPortfolioReplacementPlanRequest, security_portfolio_replacement_plan,
};
use crate::ops::stock::stock_execution_and_position_management::security_portfolio_allocation_decision::{
    SecurityPortfolioAllocationDecisionRequest, security_portfolio_allocation_decision,
};
use crate::ops::stock::stock_execution_and_position_management::security_portfolio_execution_preview::{
    SecurityPortfolioExecutionPreviewRequest, security_portfolio_execution_preview,
};
use crate::ops::stock::stock_execution_and_position_management::security_portfolio_execution_request_package::{
    SecurityPortfolioExecutionRequestPackageRequest, security_portfolio_execution_request_package,
};
use crate::ops::stock::stock_execution_and_position_management::security_portfolio_execution_request_enrichment::{
    SecurityPortfolioExecutionRequestEnrichmentRequest, security_portfolio_execution_request_enrichment,
};
use crate::ops::stock::stock_execution_and_position_management::security_portfolio_execution_apply_bridge::{
    SecurityPortfolioExecutionApplyBridgeRequest, security_portfolio_execution_apply_bridge,
};
use crate::ops::stock::stock_execution_and_position_management::security_position_contract::{
    SecurityPositionContractRequest, build_security_position_contract,
};
use crate::ops::stock::stock_execution_and_position_management::security_monitoring_evidence_package::{
    SecurityMonitoringEvidencePackageRequest, build_security_monitoring_evidence_package,
};
use crate::ops::stock::stock_execution_and_position_management::security_capital_rebase::{
    SecurityCapitalRebaseRequest, security_capital_rebase,
};
use crate::ops::stock::stock_pre_trade::security_analysis_contextual::{
    SecurityAnalysisContextualRequest, security_analysis_contextual,
};
use crate::ops::stock::stock_pre_trade::security_analysis_fullstack::{
    SecurityAnalysisFullstackRequest, security_analysis_fullstack,
};
use crate::ops::stock::stock_research_sidecar::security_analysis_resonance::{
    AppendResonanceEventTagsRequest, AppendResonanceFactorSeriesRequest,
    BootstrapResonanceTemplateFactorsRequest, EvaluateSecurityResonanceRequest,
    RegisterResonanceFactorRequest, SecurityAnalysisResonanceRequest, append_resonance_event_tags,
    append_resonance_factor_series, bootstrap_resonance_template_factors,
    evaluate_security_resonance, register_resonance_factor, security_analysis_resonance,
};
use crate::ops::stock::stock_governance_and_positioning::security_chair_resolution::{
    SecurityChairResolutionRequest, security_chair_resolution,
};
use crate::ops::stock::stock_research_sidecar::security_committee_vote::{
    SecurityCommitteeMemberAgentRequest as SecurityCommitteeVoteMemberAgentRequest,
    SecurityCommitteeVoteRequest, security_committee_member_agent as security_committee_vote_member_agent,
    security_committee_vote,
};
use crate::ops::stock::stock_governance_and_positioning::security_condition_review::{
    SecurityConditionReviewRequest, security_condition_review,
};
use crate::ops::stock::stock_governance_and_positioning::security_decision_briefing::{
    SecurityDecisionBriefingRequest, security_decision_briefing,
};
// 2026-04-16 CST: Added because dispatcher still carries one controlled legacy
// committee entry while the formal mainline continues on briefing ->
// committee_vote -> chair_resolution.
// Reason: the old committee route is not retired yet, but it must remain
// explicitly labeled as legacy at the application surface so later sessions do
// not treat it as the preferred governance path.
// Purpose: keep dispatcher naming and aliasing unambiguous until the legacy
// committee tool is intentionally retired.
use crate::ops::stock::stock_governance_and_positioning::security_decision_committee::{
    SecurityCommitteeMemberAgentRequest as LegacySecurityCommitteeMemberAgentRequest,
    SecurityDecisionCommitteeRequest,
    security_committee_member_agent as legacy_security_committee_member_agent,
    security_decision_committee,
};
use crate::ops::stock::stock_pre_trade::security_decision_evidence_bundle::{
    SecurityDecisionEvidenceBundleRequest, security_decision_evidence_bundle,
};
use crate::ops::stock::stock_governance_and_positioning::security_decision_package::{
    SecurityDecisionPackageDocument, SecurityDecisionPackageRequest, security_decision_package,
};
use crate::ops::stock::stock_governance_and_positioning::security_decision_package_revision::{
    SecurityDecisionPackageRevisionRequest, security_decision_package_revision,
};
use crate::ops::stock::stock_governance_and_positioning::security_decision_submit_approval::{
    SecurityDecisionSubmitApprovalRequest, security_decision_submit_approval,
};
use crate::ops::stock::stock_governance_and_positioning::security_decision_verify_package::{
    SecurityDecisionVerifyPackageRequest, security_decision_verify_package,
};
use crate::ops::stock::stock_data_pipeline::security_disclosure_history_live_backfill::{
    SecurityDisclosureHistoryLiveBackfillRequest, security_disclosure_history_live_backfill,
};
use crate::ops::stock::stock_data_pipeline::security_disclosure_history_backfill::{
    SecurityDisclosureHistoryBackfillRequest, security_disclosure_history_backfill,
};
use crate::ops::stock::stock_pre_trade::security_etf_resonance_trust_pack::{
    SecurityEtfResonanceTrustPackRequest, security_etf_resonance_trust_pack,
};
use crate::ops::stock::stock_execution_and_position_management::security_execution_journal::{
    SecurityExecutionJournalRequest, security_execution_journal,
};
use crate::ops::stock::stock_execution_and_position_management::security_execution_record::{
    SecurityExecutionRecordRequest, security_execution_record,
};
use crate::ops::stock::stock_data_pipeline::security_external_proxy_backfill::{
    SecurityExternalProxyBackfillRequest, security_external_proxy_backfill,
};
use crate::ops::stock::stock_data_pipeline::security_external_proxy_history_import::{
    SecurityExternalProxyHistoryImportRequest, security_external_proxy_history_import,
};
use crate::ops::stock::stock_modeling_and_training::security_feature_snapshot::{
    SecurityFeatureSnapshotRequest, security_feature_snapshot,
};
use crate::ops::stock::stock_modeling_and_training::security_forward_outcome::{
    SecurityForwardOutcomeRequest, security_forward_outcome,
};
use crate::ops::stock::stock_modeling_and_training::security_master_scorecard::{
    SecurityMasterScorecardRequest, security_master_scorecard,
};
use crate::ops::stock::stock_data_pipeline::security_fundamental_history_live_backfill::{
    SecurityFundamentalHistoryLiveBackfillRequest, security_fundamental_history_live_backfill,
};
use crate::ops::stock::stock_data_pipeline::security_fundamental_history_backfill::{
    SecurityFundamentalHistoryBackfillRequest, security_fundamental_history_backfill,
};
use crate::ops::stock::stock_pre_trade::security_independent_advice::{
    SecurityIndependentAdviceRequest, security_independent_advice,
};
use crate::ops::stock::stock_governance_and_positioning::security_portfolio_position_plan::{
    SecurityPortfolioPositionPlanRequest, security_portfolio_position_plan,
};
use crate::ops::stock::stock_governance_and_positioning::security_position_plan::{
    SecurityPositionPlanRequest, security_position_plan,
};
use crate::ops::stock::stock_governance_and_positioning::security_position_plan_record::security_position_plan_record;
use crate::ops::stock::stock_post_trade::security_post_meeting_conclusion::{
    SecurityPostMeetingConclusionBuildInput, build_security_post_meeting_conclusion,
};
use crate::ops::stock::stock_post_trade::security_post_trade_review::{
    SecurityPostTradeReviewRequest, security_post_trade_review,
};
use crate::ops::stock::stock_execution_and_position_management::security_record_position_adjustment::security_record_position_adjustment;
use crate::ops::stock::stock_post_trade::security_record_post_meeting_conclusion::{
    SecurityPostMeetingConclusionRequest, security_record_post_meeting_conclusion,
};
use crate::ops::stock::stock_modeling_and_training::security_scorecard_refit_run::{
    SecurityScorecardRefitRequest, security_scorecard_refit,
};
use crate::ops::stock::stock_modeling_and_training::security_scorecard_training::{
    SecurityScorecardTrainingRequest, security_scorecard_training,
};
use crate::ops::stock::stock_modeling_and_training::security_model_promotion::{
    SecurityModelPromotionRequest, security_model_promotion,
};
use crate::ops::stock::stock_research_sidecar::signal_outcome_research::{
    BackfillSecuritySignalOutcomesRequest, RecordSecuritySignalSnapshotRequest,
    SignalOutcomeResearchSummaryRequest, StudySecuritySignalAnalogsRequest,
    backfill_security_signal_outcomes, record_security_signal_snapshot,
    signal_outcome_research_summary, study_security_signal_analogs,
};
use crate::ops::stock::stock_research_sidecar::security_history_expansion::{
    SecurityHistoryExpansionRequest, security_history_expansion,
};
use crate::ops::stock::stock_research_sidecar::security_shadow_evaluation::{
    SecurityShadowEvaluationRequest, security_shadow_evaluation,
};
use crate::ops::stock::stock_data_pipeline::stock_training_data_backfill::{
    StockTrainingDataBackfillRequest, stock_training_data_backfill,
};
use crate::ops::stock::stock_data_pipeline::stock_training_data_coverage_audit::{
    StockTrainingDataCoverageAuditRequest, stock_training_data_coverage_audit,
};
use crate::ops::stock::stock_data_pipeline::security_real_data_validation_backfill::{
    SecurityRealDataValidationBackfillRequest, security_real_data_validation_backfill,
};
use crate::ops::stock::stock_data_pipeline::sync_stock_price_history::{
    SyncStockPriceHistoryRequest, sync_stock_price_history,
};
use crate::ops::stock::stock_research_sidecar::sync_template_resonance_factors::{
    SyncTemplateResonanceFactorsRequest, sync_template_resonance_factors,
};
use crate::ops::stock::stock_pre_trade::technical_consultation_basic::{
    TechnicalConsultationBasicRequest, technical_consultation_basic,
};
use crate::tools::contracts::{
    SecurityPositionPlanRecordRequest, SecurityRecordPositionAdjustmentRequest, ToolResponse,
};

pub(super) fn dispatch_import_stock_price_history(args: Value) -> ToolResponse {
    // 2026-03-31 CST：这里把股票历史导入请求收口到 stock dispatcher，原因是股票导入已不再属于 foundation 分析域；
    // 目的：让 “CSV -> SQLite” 的股票入口单独沿 stock 模块扩展，而不是继续挂在通用分析分发层里。
    let request = match serde_json::from_value::<ImportStockPriceHistoryRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match import_stock_price_history(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_sync_stock_price_history(args: Value) -> ToolResponse {
    // 2026-03-31 CST：这里把股票历史同步请求收口到 stock dispatcher，原因是 provider 顺序和补数逻辑属于股票域内部细节；
    // 目的：避免后续继续在 foundation 分发层追加股票专属解析分支。
    let request = match serde_json::from_value::<SyncStockPriceHistoryRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match sync_stock_price_history(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_fundamental_history_live_backfill(args: Value) -> ToolResponse {
    // 2026-04-14 CST: Added because plan A+ must let governed multi-period financial history enter
    // the formal stock dispatcher instead of being reachable only by direct module wiring.
    // Purpose: make stock training-data thickening callable from the public tool surface.
    let request =
        match serde_json::from_value::<SecurityFundamentalHistoryLiveBackfillRequest>(args) {
            Ok(request) => request,
            Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
        };

    match security_fundamental_history_live_backfill(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_fundamental_history_backfill(args: Value) -> ToolResponse {
    // 2026-04-17 CST: Added because StockMind phase-1 public-surface closeout now
    // exposes governed historical fundamentals on the formal dispatcher, matching the
    // copied stock boundary and CLI contract tests.
    // Purpose: keep data-pipeline backfill discoverability and routing consistent.
    let request = match serde_json::from_value::<SecurityFundamentalHistoryBackfillRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_fundamental_history_backfill(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_disclosure_history_live_backfill(args: Value) -> ToolResponse {
    // 2026-04-14 CST: Added because plan A+ also needs announcement history to be available on the
    // same stock dispatcher surface used by CLI and later batch backfill orchestration.
    // Purpose: expose governed disclosure live backfill as a first-class stock tool route.
    let request = match serde_json::from_value::<SecurityDisclosureHistoryLiveBackfillRequest>(args)
    {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_disclosure_history_live_backfill(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_disclosure_history_backfill(args: Value) -> ToolResponse {
    // 2026-04-17 CST: Added because phase-1 boundary closeout restores the governed
    // historical disclosure batch route to the public stock dispatcher.
    // Purpose: align CLI discovery, dispatcher routing, and copied stock modules.
    let request = match serde_json::from_value::<SecurityDisclosureHistoryBackfillRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_disclosure_history_backfill(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_stock_training_data_backfill(args: Value) -> ToolResponse {
    // 2026-04-14 CST: Added because plan A+ needs one stock-only batch entrypoint that composes
    // existing price, financial-history, and disclosure-history tools before retraining.
    // Purpose: expose a single CLI/Skill contract for stock training-data thickening.
    let request = match serde_json::from_value::<StockTrainingDataBackfillRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match stock_training_data_backfill(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_stock_training_data_coverage_audit(args: Value) -> ToolResponse {
    // 2026-04-14 CST: Added because stock-first real-trading readiness needs one formal
    // post-backfill audit that tells operators which symbols are train-ready.
    // Purpose: expose stock-pool coverage gating on the public stock dispatcher surface.
    let request = match serde_json::from_value::<StockTrainingDataCoverageAuditRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match stock_training_data_coverage_audit(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

// 2026-04-02 CST: 这里补模板级共振因子同步 dispatcher，原因是方案C要求“模板补数”必须走正式 stock Tool 主链；
// 目的：让银行宏观代理序列的同步、转换和落库不再依赖外部脚本，而是可以被 CLI / Skill 稳定发现和调用。
pub(super) fn dispatch_security_external_proxy_backfill(args: Value) -> ToolResponse {
    // 2026-04-15 CST: Added because ETF trust replay depends on governed dated proxy ingestion.
    // Reason: the backfill op already exists, but the current dispatcher surface could not call it.
    // Purpose: make external proxy history import reachable from the formal stock tool bus.
    let request = match serde_json::from_value::<SecurityExternalProxyBackfillRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_external_proxy_backfill(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_real_data_validation_backfill(args: Value) -> ToolResponse {
    // 2026-04-16 CST: Added because the formal-boundary gate exposed that the governed
    // validation-slice backfill still existed in code/docs/tests but had lost its public route.
    // Purpose: restore one official dispatcher entrypoint for slice-local real-data validation replay.
    let request = match serde_json::from_value::<SecurityRealDataValidationBackfillRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_real_data_validation_backfill(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_external_proxy_history_import(args: Value) -> ToolResponse {
    // 2026-04-17 CST: Added because StockMind now publishes the governed file-based
    // proxy-history import tool on the same public dispatcher as the rest of the
    // stock data pipeline.
    // Purpose: remove the split between exported module presence and dispatcher access.
    let request = match serde_json::from_value::<SecurityExternalProxyHistoryImportRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_external_proxy_history_import(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_sync_template_resonance_factors(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SyncTemplateResonanceFactorsRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match sync_template_resonance_factors(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_technical_consultation_basic(args: Value) -> ToolResponse {
    // 2026-03-31 CST：这里把股票技术面咨询请求收口到 stock dispatcher，原因是技术面咨询已是独立业务模块；
    // 目的：确保后续新增指标、评分和多周期分析时，都沿 stock 业务域演进，不再反向污染 foundation。
    let request = match serde_json::from_value::<TechnicalConsultationBasicRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match technical_consultation_basic(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_analysis_contextual(args: Value) -> ToolResponse {
    // 2026-04-01 CST：这里接入综合证券分析 contextual Tool，原因是用户已批准在技术面上层叠加大盘与板块环境；
    // 目的：保持 `technical_consultation_basic` 边界不变，同时为 CLI / Skill 暴露统一的综合证券分析入口。
    let request = match serde_json::from_value::<SecurityAnalysisContextualRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_analysis_contextual(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

// 2026-04-13 CST: 这里接入独立建议 Tool 的 stock dispatcher 分支，原因是方案B要求把主席外部独立建议做成正式可发现入口；
// 目的：让 CLI / Skill / 主席链可以沿统一 dispatcher 获取标准独立建议文档，而不是继续内嵌自由对象。
pub(super) fn dispatch_security_independent_advice(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityIndependentAdviceRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    ToolResponse::ok_serialized(&security_independent_advice(&request))
}

pub(super) fn dispatch_security_analysis_fullstack(args: Value) -> ToolResponse {
    // 2026-04-01 CST：这里接入 fullstack Tool，原因是既有主链已经确定要把技术、财报、公告和行业统一聚合；
    // 目的：让 CLI / Skill 直接消费完整证券分析结果，而不是在外层继续手工拼接信息面。
    let request = match serde_json::from_value::<SecurityAnalysisFullstackRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_analysis_fullstack(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_decision_evidence_bundle(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityDecisionEvidenceBundleRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_decision_evidence_bundle(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_etf_resonance_trust_pack(args: Value) -> ToolResponse {
    // 2026-04-15 CST: Added because ETF trust validation now has a formal stock dispatcher route.
    // Reason: callers should not reach the trust-pack through private module wiring or ad-hoc scripts.
    // Purpose: expose current ETF evidence plus historical replay validation on the public tool bus.
    let request = match serde_json::from_value::<SecurityEtfResonanceTrustPackRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_etf_resonance_trust_pack(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_decision_committee(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityDecisionCommitteeRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_decision_committee(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_decision_briefing(args: Value) -> ToolResponse {
    // 2026-04-02 CST: 这里接入 security_decision_briefing 的 stock dispatcher 分支，原因是统一 briefing 已经成为咨询与投决共用的事实入口；
    // 目的：让 CLI / Skill 可以直接走正式 Tool 主链拿到单一 briefing，而不是在外层手工串 fullstack 与 resonance。
    let request = match serde_json::from_value::<SecurityDecisionBriefingRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_decision_briefing(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

// 2026-04-10 CST: 这里接入 security_decision_submit_approval 的 stock dispatcher 分支，原因是当前分支已导入正式审批提交实现，
// 目的：让外层通过统一 Tool 合同触发“投决 -> 审批桥接 -> 工件落盘”主链，而不是直接引用内部函数。
pub(super) fn dispatch_security_decision_submit_approval(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityDecisionSubmitApprovalRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_decision_submit_approval(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

// 2026-04-10 CST: 这里接入 security_condition_review 的 stock dispatcher 分支，原因是投中条件复核已经成为正式最小闭环的一部分，
// 目的：让 CLI / Skill 直接消费结构化复核结果，并为后续 reopen / freeze / keep_plan 路由保留统一入口。
pub(super) fn dispatch_security_condition_review(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityConditionReviewRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_condition_review(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

// 2026-04-10 CST: 这里接入轻量 security_post_meeting_conclusion dispatcher，原因是方案A先补正式对象能力，
// 目的：让 CLI / Skill 能直接基于 submit_approval 产物生成独立会后结论对象，而不强绑定较重的 record 主链。
pub(super) fn dispatch_security_post_meeting_conclusion(args: Value) -> ToolResponse {
    let scene_name = args
        .get("scene_name")
        .and_then(Value::as_str)
        .unwrap_or("security_decision_committee")
        .to_string();
    let decision_id = match args.get("decision_id").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => return ToolResponse::error("request parsing failed: decision_id is required"),
    };
    let decision_ref = match args.get("decision_ref").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => return ToolResponse::error("request parsing failed: decision_ref is required"),
    };
    let approval_ref = match args.get("approval_ref").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => return ToolResponse::error("request parsing failed: approval_ref is required"),
    };
    let symbol = match args.get("symbol").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => return ToolResponse::error("request parsing failed: symbol is required"),
    };
    let analysis_date = match args.get("analysis_date").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => return ToolResponse::error("request parsing failed: analysis_date is required"),
    };
    let source_package_path = match args.get("source_package_path").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => return ToolResponse::error("request parsing failed: source_package_path is required"),
    };
    let source_brief_ref = match args.get("source_brief_ref").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => return ToolResponse::error("request parsing failed: source_brief_ref is required"),
    };
    let source_brief_path = match args.get("source_brief_path").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => return ToolResponse::error("request parsing failed: source_brief_path is required"),
    };
    let final_disposition = match args.get("final_disposition").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => return ToolResponse::error("request parsing failed: final_disposition is required"),
    };
    let disposition_reason = match args.get("disposition_reason").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => return ToolResponse::error("request parsing failed: disposition_reason is required"),
    };
    let reviewer_notes = args
        .get("reviewer_notes")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let reviewer = args
        .get("reviewer")
        .and_then(Value::as_str)
        .unwrap_or("unknown_reviewer")
        .trim()
        .to_string();
    let reviewer_role = args
        .get("reviewer_role")
        .and_then(Value::as_str)
        .unwrap_or("UnknownRole")
        .trim()
        .to_string();
    let revision_reason = args
        .get("revision_reason")
        .and_then(Value::as_str)
        .unwrap_or("post_meeting_conclusion_recorded")
        .trim()
        .to_string();
    let generated_at = args
        .get("generated_at")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let source_package_version = args
        .get("source_package_version")
        .and_then(Value::as_u64)
        .unwrap_or(1) as u32;
    let key_reasons = args
        .get("key_reasons")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let required_follow_ups = args
        .get("required_follow_ups")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let result = build_security_post_meeting_conclusion(SecurityPostMeetingConclusionBuildInput {
        generated_at,
        scene_name,
        decision_id,
        decision_ref,
        approval_ref,
        symbol,
        analysis_date,
        source_package_path,
        source_package_version,
        source_brief_ref,
        source_brief_path,
        final_disposition,
        disposition_reason,
        key_reasons,
        required_follow_ups,
        reviewer_notes,
        reviewer,
        reviewer_role,
        revision_reason,
    });

    ToolResponse::ok(json!(result))
}

// 2026-04-08 CST: 这里接入 security_position_plan_record 的 stock dispatcher 分支，原因是仓位计划正式化必须沿证券主链标准入口暴露；
// 目的：让 CLI / Skill 能直接把 briefing 派生仓位计划升级成 record 对象，而不是在外层继续手工维护 position_plan 片段。
pub(super) fn dispatch_security_position_plan_record(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityPositionPlanRecordRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_position_plan_record(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

// 2026-04-08 CST: 这里接入 security_post_trade_review 的 stock dispatcher 分支，原因是投后复盘必须沿证券主链正式入口暴露；
// 目的：让 CLI / Skill 能只传 position_plan_ref 与 adjustment_event_refs 就拿到结构化复盘，而不是在外层手工拼总结文本。
// 2026-04-02 CST: 这里接入 security_committee_vote 的 stock dispatcher，原因是投决会必须沿正式 Tool 主链暴露，
// 目的：让上层只传 committee payload / committee_mode 就能拿到结构化表决结果，而不是再去拼第二套流程。
pub(super) fn dispatch_security_position_plan(args: Value) -> ToolResponse {
    // 2026-04-09 CST: 这里接入 security_position_plan 的 stock dispatcher 分支，原因是 Task 7 要把 briefing 内仓位层正式升级为独立 Tool；
    // 目的：让 CLI / Skill 直接消费正式仓位文档，同时保持事实源仍来自统一 briefing 主链。
    let request = match serde_json::from_value::<SecurityPositionPlanRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_position_plan(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_portfolio_position_plan(args: Value) -> ToolResponse {
    // 2026-04-09 CST: 这里接入 security_portfolio_position_plan 的 stock dispatcher 分支，原因是方案A要把账户级仓位建议升级为正式 Tool；
    // 目的：让 CLI / Skill 直接消费账户级配置建议，而不是继续在对话里手工算总仓和单票上限。
    let request = match serde_json::from_value::<SecurityPortfolioPositionPlanRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_portfolio_position_plan(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_account_open_position_snapshot(args: Value) -> ToolResponse {
    // 2026-04-10 CST: 这里接入 security_account_open_position_snapshot 的 stock dispatcher 分支，原因是方案B要把 runtime 自动回接上一轮 open 持仓做成正式 Tool；
    // 目的：让 CLI / Skill 直接消费账户 open snapshot 对象，再由账户计划显式承接，而不是继续手工传裸数组。
    let request = match serde_json::from_value::<SecurityAccountOpenPositionSnapshotRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_account_open_position_snapshot(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

// 2026-04-19 CST: Added because P10 now starts the portfolio-core expansion
// from one formal account objective and candidate-set builder on the public stock bus.
// Reason: the current RED test correctly fails until the dispatcher recognizes
// this new tool and routes governed inputs into the new P10 module.
// Purpose: expose the account objective contract builder on the official stock dispatcher.
pub(super) fn dispatch_security_account_objective_contract(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityAccountObjectiveContractRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_account_objective_contract(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

// 2026-04-19 CST: Added because Task 3 now exposes the first P11 unified
// replacement solver on the public stock dispatcher.
// Reason: the current RED test correctly fails until the stock bus recognizes
// and routes the formal replacement-plan contract.
// Purpose: route portfolio replacement plan requests through the official stock dispatcher.
pub(super) fn dispatch_security_portfolio_replacement_plan(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityPortfolioReplacementPlanRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_portfolio_replacement_plan(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

// 2026-04-20 CST: Added because P12 now exposes the minimum governed final
// allocation decision on the public stock dispatcher.
// Reason: the current RED test correctly fails until the stock bus recognizes
// and routes the formal decision-freeze contract.
// Purpose: route portfolio allocation decision requests through the official stock dispatcher.
pub(super) fn dispatch_security_portfolio_allocation_decision(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityPortfolioAllocationDecisionRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_portfolio_allocation_decision(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

// 2026-04-20 CST: Added because the approved next step after P12 is one
// side-effect-free execution preview bridge on the public stock dispatcher.
// Reason: the RED test correctly fails until the stock bus recognizes and
// routes the new preview-only downstream contract.
// Purpose: route portfolio execution preview requests through the official stock dispatcher.
pub(super) fn dispatch_security_portfolio_execution_preview(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityPortfolioExecutionPreviewRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_portfolio_execution_preview(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

// 2026-04-20 CST: Added because P13 now introduces one formal request-package
// bridge after the standardized preview document on the public stock dispatcher.
// Reason: the RED test should only turn green once the stock bus recognizes
// and routes the new P13 contract explicitly.
// Purpose: route portfolio execution request-package requests through the official dispatcher.
pub(super) fn dispatch_security_portfolio_execution_request_package(args: Value) -> ToolResponse {
    let request =
        match serde_json::from_value::<SecurityPortfolioExecutionRequestPackageRequest>(args) {
            Ok(request) => request,
            Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
        };

    match security_portfolio_execution_request_package(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

// 2026-04-21 CST: Added because P14 now introduces one formal request-enrichment
// bridge after the governed P13 request package on the public stock dispatcher.
// Reason: the RED test should only turn green once the stock bus recognizes
// and routes the new P14 contract explicitly.
// Purpose: route portfolio execution request-enrichment requests through the official dispatcher.
pub(super) fn dispatch_security_portfolio_execution_request_enrichment(
    args: Value,
) -> ToolResponse {
    let request =
        match serde_json::from_value::<SecurityPortfolioExecutionRequestEnrichmentRequest>(args) {
            Ok(request) => request,
            Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
        };

    match security_portfolio_execution_request_enrichment(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

// 2026-04-21 CST: Added because P15 now exposes one governed apply bridge on
// the public stock dispatcher.
// Reason: the approved route must remain callable through the official stock bus.
// Purpose: route portfolio execution apply requests through the official dispatcher.
pub(super) fn dispatch_security_portfolio_execution_apply_bridge(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityPortfolioExecutionApplyBridgeRequest>(args)
    {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_portfolio_execution_apply_bridge(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_position_contract(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityPositionContractRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match build_security_position_contract(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_monitoring_evidence_package(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityMonitoringEvidencePackageRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match build_security_monitoring_evidence_package(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_capital_rebase(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityCapitalRebaseRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_capital_rebase(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

// 2026-04-19 CST: Added because Task 7 now exposes the governed committee
// decision package as its own public stock tool.
// Reason: without this branch, the catalog-visible committee package contract
// still falls through to "unsupported tool" even after the module exists.
// Purpose: route the formal post-open committee handoff through the official stock bus.
// 2026-04-19 CST: Added because the newly approved adjustment-input bridge must
// be reachable from the public stock dispatcher instead of remaining an internal-only contract.
// Reason: the current RED test correctly fails until the stock bus recognizes this tool.
// Purpose: route the formal approved downstream bridge through the official stock dispatcher.
pub(super) fn dispatch_security_post_trade_review(args: Value) -> ToolResponse {
    // 2026-04-09 CST: 这里接入 security_post_trade_review 的 stock dispatcher 分支，原因是 Task 8 要把投后复盘升级为正式 Tool；
    // 目的：让 CLI / Skill 直接消费正式复盘文档，并保持其事实源复用 position_plan 与 forward_outcome 主链。
    let request = match serde_json::from_value::<SecurityPostTradeReviewRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_post_trade_review(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_execution_record(args: Value) -> ToolResponse {
    // 2026-04-09 CST: 这里接入 security_execution_record 的 stock dispatcher 分支，原因是 Task 10 要把真实执行对象升级为正式 Tool；
    // 目的：让 CLI / Skill 直接消费正式执行归因文档，并沿统一主链进入 review/package/governance。
    let request = match serde_json::from_value::<SecurityExecutionRecordRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_execution_record(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_execution_journal(args: Value) -> ToolResponse {
    // 2026-04-09 CST: 这里接入 security_execution_journal 的 stock dispatcher 分支，原因是 P1 要让多笔成交成为正式 Tool；
    // 目的：让 CLI / Skill 直接消费结构化 journal，并把它作为 execution_record 的事实底座。
    let request = match serde_json::from_value::<SecurityExecutionJournalRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_execution_journal(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_committee_vote(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityCommitteeVoteRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_committee_vote(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

// 2026-04-08 CST: 这里补七席委员会内部 seat agent 分发，原因是独立执行证明要求每个委员都经由单独 CLI 子进程产出投票；
// 目的：把内部子进程调用也绑定在现有 stock dispatcher 上，保证委员会正式入口仍然只有 briefing 与 vote 两层，对外不新增目录噪音。
pub(super) fn dispatch_security_committee_member_agent(args: Value) -> ToolResponse {
    // 2026-04-16 CST: Route the internal seat-agent tool to the formal
    // `security_committee_vote` contract, because the execution_record ->
    // briefing -> committee path now spawns seat children with `seat_role`
    // instead of the legacy `member_id` payload.
    match serde_json::from_value::<SecurityCommitteeVoteMemberAgentRequest>(args.clone()) {
        Ok(request) => match security_committee_vote_member_agent(&request) {
            Ok(result) => ToolResponse::ok_serialized(&result),
            Err(error) => ToolResponse::error(error.to_string()),
        },
        Err(_) => {
            // 2026-04-16 CST: Fall back to the legacy committee child-agent
            // request, because submit_approval / decision_committee tests still
            // spawn seat children with `member_id + market_context +
            // evidence_bundle` while the public CLI regression already locks the
            // new vote-contract request shape.
            // Purpose: restore the existing submit/revision mainline without
            // reopening committee business logic or weakening the newer seat-role path.
            let request =
                match serde_json::from_value::<LegacySecurityCommitteeMemberAgentRequest>(args) {
                    Ok(request) => request,
                    Err(error) => {
                        return ToolResponse::error(format!("request parsing failed: {error}"));
                    }
                };

            match legacy_security_committee_member_agent(&request) {
                Ok(result) => ToolResponse::ok_serialized(&result),
                Err(error) => ToolResponse::error(error.to_string()),
            }
        }
    }
}

pub(super) fn dispatch_security_chair_resolution(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityChairResolutionRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_chair_resolution(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_record_post_meeting_conclusion(args: Value) -> ToolResponse {
    if args.get("package_path").is_none() {
        let request = match serde_json::from_value::<SecurityPostMeetingConclusionRequest>(args) {
            Ok(request) => request,
            Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
        };

        return match security_record_post_meeting_conclusion(&request) {
            Ok(result) => ToolResponse::ok(json!(result)),
            Err(error) => ToolResponse::error(error.to_string()),
        };
    }

    let package_path = match args.get("package_path").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => return ToolResponse::error("request parsing failed: package_path is required"),
    };
    let final_disposition = match args.get("final_disposition").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => return ToolResponse::error("request parsing failed: final_disposition is required"),
    };
    let disposition_reason = match args.get("disposition_reason").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => return ToolResponse::error("request parsing failed: disposition_reason is required"),
    };
    let reviewer_notes = args
        .get("reviewer_notes")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let reviewer = args
        .get("reviewer")
        .and_then(Value::as_str)
        .unwrap_or("unknown_reviewer")
        .trim()
        .to_string();
    let reviewer_role = args
        .get("reviewer_role")
        .and_then(Value::as_str)
        .unwrap_or("UnknownRole")
        .trim()
        .to_string();
    let revision_reason = args
        .get("revision_reason")
        .and_then(Value::as_str)
        .unwrap_or("post_meeting_conclusion_recorded")
        .trim()
        .to_string();
    let reverify_after_revision = args
        .get("reverify_after_revision")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let generated_at = args
        .get("generated_at")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let approval_brief_signing_key_secret = args
        .get("approval_brief_signing_key_secret")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    let approval_brief_signing_key_secret_env = args
        .get("approval_brief_signing_key_secret_env")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    let key_reasons = args
        .get("key_reasons")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let required_follow_ups = args
        .get("required_follow_ups")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // 2026-04-17 CST: Reason=the record_post_meeting entry is the orchestration surface
    // that starts from an existing decision package, not the lighter symbol-only chair
    // replay request. Purpose=load the governed package first so the recorded conclusion
    // inherits the frozen decision identity before package revision runs.
    let package = match load_decision_package_document(&package_path) {
        Ok(package) => package,
        Err(error) => return ToolResponse::error(error),
    };
    let post_meeting_conclusion =
        build_security_post_meeting_conclusion(SecurityPostMeetingConclusionBuildInput {
            generated_at,
            scene_name: package.scene_name.clone(),
            decision_id: package.decision_id.clone(),
            decision_ref: package.decision_ref.clone(),
            approval_ref: package.approval_ref.clone(),
            symbol: package.symbol.clone(),
            analysis_date: package.analysis_date.clone(),
            source_package_path: package_path.clone(),
            source_package_version: package.package_version,
            source_brief_ref: package.object_graph.approval_brief_ref.clone(),
            source_brief_path: package.object_graph.approval_brief_path.clone(),
            final_disposition,
            disposition_reason,
            key_reasons,
            required_follow_ups,
            reviewer_notes,
            reviewer,
            reviewer_role,
            revision_reason: revision_reason.clone(),
        });
    let post_meeting_conclusion_path = match resolve_post_meeting_conclusion_path(
        Path::new(&package_path),
        &package.decision_id,
    ) {
        Ok(path) => path,
        Err(error) => return ToolResponse::error(error),
    };
    if let Err(error) = persist_json_pretty(&post_meeting_conclusion_path, &post_meeting_conclusion)
    {
        return ToolResponse::error(error);
    }

    let revision_result =
        match security_decision_package_revision(&SecurityDecisionPackageRevisionRequest {
            package_path: package_path.clone(),
            revision_reason,
            reverify_after_revision,
            condition_review_path: None,
            execution_record_path: None,
            post_trade_review_path: None,
            approval_brief_signing_key_secret,
            approval_brief_signing_key_secret_env,
        }) {
            Ok(result) => result,
            Err(error) => return ToolResponse::error(error.to_string()),
        };

    ToolResponse::ok(json!({
        "post_meeting_conclusion": post_meeting_conclusion,
        "post_meeting_conclusion_path": post_meeting_conclusion_path.to_string_lossy().to_string(),
        "decision_package": revision_result.decision_package,
        "decision_package_path": revision_result.decision_package_path,
        "package_version": revision_result.package_version,
        "previous_package_path": revision_result.previous_package_path,
        "revision_reason": revision_result.revision_reason,
        "trigger_event_summary": revision_result.trigger_event_summary,
        "verification_report_path": revision_result.verification_report_path,
    }))
}

// 2026-04-17 CST: Reason=record_post_meeting now starts from an existing package file
// rather than a raw chair request. Purpose=centralize package loading and keep dispatcher
// error messages stable for the orchestration entry.
fn load_decision_package_document(
    package_path: &str,
) -> Result<SecurityDecisionPackageDocument, String> {
    serde_json::from_slice::<SecurityDecisionPackageDocument>(
        &fs::read(package_path).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

// 2026-04-17 CST: Reason=the recorded post-meeting artifact needs one deterministic
// governed path beside the existing scenes_runtime bundle. Purpose=keep the new record
// path stable for later revision/verify wiring instead of scattering ad-hoc filenames.
fn resolve_post_meeting_conclusion_path(
    package_path: &Path,
    decision_id: &str,
) -> Result<PathBuf, String> {
    let runtime_root = package_path
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "failed to resolve runtime root from package_path".to_string())?;
    let post_meeting_dir = runtime_root.join("post_meeting_conclusions");
    fs::create_dir_all(&post_meeting_dir).map_err(|error| error.to_string())?;
    Ok(post_meeting_dir.join(format!("{decision_id}.json")))
}

// 2026-04-17 CST: Reason=the dispatcher now persists a standalone post-meeting artifact
// before package revision runs. Purpose=write one readable JSON document without duplicating
// persistence snippets inside the orchestration body.
fn persist_json_pretty<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), String> {
    fs::write(
        path,
        serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

pub(super) fn dispatch_security_decision_package(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityDecisionPackageRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_decision_package(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_decision_verify_package(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityDecisionVerifyPackageRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_decision_verify_package(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_decision_package_revision(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityDecisionPackageRevisionRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_decision_package_revision(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_feature_snapshot(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityFeatureSnapshotRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_feature_snapshot(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_forward_outcome(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityForwardOutcomeRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_forward_outcome(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_master_scorecard(args: Value) -> ToolResponse {
    // 2026-04-16 CST: Added because approved scheme B closes the last stock dispatcher gap
    // for the formal master scorecard route.
    // Reason: without this branch, the catalog-visible tool still returns "unsupported tool".
    // Purpose: route the existing master scorecard business object through the official stock bus.
    let request = match serde_json::from_value::<SecurityMasterScorecardRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_master_scorecard(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_scorecard_refit(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityScorecardRefitRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_scorecard_refit(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_scorecard_training(args: Value) -> ToolResponse {
    // 2026-04-09 CST: 这里新增正式 scorecard training dispatcher 入口，原因是 Task 5 需要把训练主链接入统一 stock 路由；
    // 目的：让 CLI / Skill / 回算编排都能通过同一个 dispatcher 获取 artifact、refit_run 与 model_registry。
    let request = match serde_json::from_value::<SecurityScorecardTrainingRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_scorecard_training(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_model_promotion(args: Value) -> ToolResponse {
    // 2026-04-17 CST: Added because the standalone repo now treats governed model
    // promotion as a public lifecycle tool instead of leaving it exported-but-unrouted.
    // Purpose: close the public tool-surface gap for promotion governance.
    let request = match serde_json::from_value::<SecurityModelPromotionRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_model_promotion(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_register_resonance_factor(args: Value) -> ToolResponse {
    // 2026-04-02 CST：这里接入因子注册入口，原因是方案 3 已确认先做“平台底层”，而不是只做一次性分析输出；
    // 目的：让新共振想法可以先注册为正式因子，再落序列、跑评估和进入分析主链。
    let request = match serde_json::from_value::<RegisterResonanceFactorRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match register_resonance_factor(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_append_resonance_factor_series(args: Value) -> ToolResponse {
    // 2026-04-02 CST：这里接入因子序列写库入口，原因是用户要求“算出来以后写到数据库里，再把相关性强的拉出来评估”；
    // 目的：把价格、运价、汇率等候选因子沉淀成正式日度序列资产。
    let request = match serde_json::from_value::<AppendResonanceFactorSeriesRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match append_resonance_factor_series(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_append_resonance_event_tags(args: Value) -> ToolResponse {
    // 2026-04-02 CST：这里接入事件标签写库入口，原因是事件标签已被纳入第一版平台而不是后补；
    // 目的：让地缘、政策、运输瓶颈等非价格事件也能通过正式 Tool 主链进入平台。
    let request = match serde_json::from_value::<AppendResonanceEventTagsRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match append_resonance_event_tags(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_bootstrap_resonance_template_factors(args: Value) -> ToolResponse {
    // 2026-04-02 CST：这里接入模板池初始化入口，原因是第二阶段方案 B 要把传统行业候选因子池正式暴露给 Tool 主链；
    // 目的：让 Agent/Skill 可以先初始化行业底座，再做独立评估或最终分析。
    let request = match serde_json::from_value::<BootstrapResonanceTemplateFactorsRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match bootstrap_resonance_template_factors(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_evaluate_security_resonance(args: Value) -> ToolResponse {
    // 2026-04-02 CST：这里接入独立评估入口，原因是第二阶段已经确认“研究评估”和“fullstack 最终分析”需要拆开；
    // 目的：让 Agent/Skill 可以只跑共振评估并落快照，而不是所有场景都强绑信息面抓取。
    let request = match serde_json::from_value::<EvaluateSecurityResonanceRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match evaluate_security_resonance(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_analysis_resonance(args: Value) -> ToolResponse {
    // 2026-04-02 CST：这里接入共振平台分析入口，原因是用户已经明确要求国际与行业证券分析必须显式暴露共振驱动；
    // 目的：复用 fullstack 主链，再把板块、商品、事件和注册因子一起聚合成正式分析结果。
    let request = match serde_json::from_value::<SecurityAnalysisResonanceRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_analysis_resonance(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_history_expansion(args: Value) -> ToolResponse {
    // 2026-04-17 CST: Added because historical proxy coverage expansion is part of
    // the copied StockMind research sidecar surface and should remain publicly callable.
    // Purpose: align research-sidecar discovery with dispatcher reality.
    let request = match serde_json::from_value::<SecurityHistoryExpansionRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_history_expansion(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_security_shadow_evaluation(args: Value) -> ToolResponse {
    // 2026-04-17 CST: Added because phase-1 public-surface closeout promotes shadow
    // governance review into the same discoverable research tool surface as history expansion.
    // Purpose: keep lifecycle governance prerequisites callable from the standalone repo.
    let request = match serde_json::from_value::<SecurityShadowEvaluationRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_shadow_evaluation(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_record_security_signal_snapshot(args: Value) -> ToolResponse {
    // 2026-04-02 CST: 这里接入 research snapshot Tool，原因是方案C第一批任务要求把“当前完整指标状态”做成正式研究平台入口，
    // 目的：让上层先能稳定触发并落库 snapshot，后续再围绕同一主键扩展 forward returns 与 analog study。
    let request = match serde_json::from_value::<RecordSecuritySignalSnapshotRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match record_security_signal_snapshot(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

pub(super) fn dispatch_backfill_security_signal_outcomes(args: Value) -> ToolResponse {
    // 2026-04-02 CST: 这里接入 forward returns 回填 Tool，原因是方案C第二步要求把 snapshot 后续收益研究做成正式平台链路，
    // 目的：让研究层可以围绕已落库快照统一回填 1/3/5/10/20 日结果，而不是每次由上层临时扫描历史。
    let request = match serde_json::from_value::<BackfillSecuritySignalOutcomesRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match backfill_security_signal_outcomes(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

// 2026-04-02 CST: 这里接入历史相似研究 Tool，原因是用户明确要求把银行体系内“共振 + MACD/RSRS 等技术状态相似”
// 的样本统计做成正式平台入口；目的：让上层能用统一 Tool 主链生成并持久化 analog study，而不是手工离线统计。
pub(super) fn dispatch_study_security_signal_analogs(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<StudySecuritySignalAnalogsRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match study_security_signal_analogs(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

// 2026-04-02 CST: 这里接入历史研究摘要读取 Tool，原因是 security_decision_briefing / committee payload
// 要读取统一研究结论，而不是各层自行拼接；目的：让咨询与投决共享同一份 historical digest 数据源。
pub(super) fn dispatch_signal_outcome_research_summary(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SignalOutcomeResearchSummaryRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match signal_outcome_research_summary(&request) {
        Ok(result) => ToolResponse::ok(json!(result)),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}

// 2026-04-08 CST: 这里接入 security_record_position_adjustment 的 stock dispatcher 分支，原因是正式调仓事件需要沿证券主链标准入口暴露，
// 目的：让 CLI / Skill 能直接把同一 position_plan_ref 下的执行动作升级成正式事件对象，而不是在外层手工维护交易日志片段。
pub(super) fn dispatch_security_record_position_adjustment(args: Value) -> ToolResponse {
    let request = match serde_json::from_value::<SecurityRecordPositionAdjustmentRequest>(args) {
        Ok(request) => request,
        Err(error) => return ToolResponse::error(format!("request parsing failed: {error}")),
    };

    match security_record_position_adjustment(&request) {
        Ok(result) => ToolResponse::ok_serialized(&result),
        Err(error) => ToolResponse::error(error.to_string()),
    }
}
