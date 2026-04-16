use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::ops::stock::security_analysis_resonance::{
    SecurityAnalysisResonanceError, SecurityAnalysisResonanceRequest,
    SecurityAnalysisResonanceResult, security_analysis_resonance,
};
use crate::ops::stock::security_committee_vote::{
    SecurityCommitteeVoteError, SecurityCommitteeVoteRequest, SecurityCommitteeVoteResult,
    security_committee_vote,
};
use crate::ops::stock::signal_outcome_research::{
    SignalOutcomeResearchSummaryRequest, signal_outcome_research_summary,
};
use crate::ops::stock::stock_analysis_data_guard::StockAnalysisDateGuard;

// 2026-04-02 CST: 这里先定义 security_decision_briefing 的请求合同，原因是本轮第一步只需要先把 briefing Tool 的输入边界稳定下来，
// 目的：让后续 assembler、dispatcher 和 Skill 都围绕同一份强类型请求扩展，而不是继续在各层散落弱类型 JSON 参数解释。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityDecisionBriefingRequest {
    pub symbol: String,
    #[serde(default)]
    pub market_symbol: Option<String>,
    #[serde(default)]
    pub sector_symbol: Option<String>,
    pub market_regime: String,
    pub sector_template: String,
    #[serde(default)]
    pub as_of_date: Option<String>,
    #[serde(default = "default_lookback_days")]
    pub lookback_days: usize,
    #[serde(default = "default_factor_lookback_days")]
    pub factor_lookback_days: usize,
    #[serde(default = "default_disclosure_limit")]
    pub disclosure_limit: usize,
}

// 2026-04-02 CST: 这里先定义 briefing 顶层响应合同，原因是计划第一步要求先把咨询场景与投决场景共享的事实载体钉住，
// 目的：确保后续就算逐步补 assembler、执行层和 committee payload，也不会再改动对外字段骨架。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityDecisionBriefingResult {
    pub symbol: String,
    pub analysis_date: String,
    // 2026-04-14 CST: 这里补回 briefing 顶层日期门禁字段，原因是 position_plan 已经将 briefing 作为统一输入合同；
    // 目的：让仓位层直接消费 briefing 顶层日期事实，不再回头解嵌套结构。
    pub analysis_date_guard: StockAnalysisDateGuard,
    pub summary: String,
    pub evidence_version: String,
    // 2026-04-08 CST: 这里补充分析对象画像，原因是 ETF 与个股后续会共用同一条 briefing 主链；
    // 目的：让上层直接知道当前结果是在按个股口径还是按 ETF 口径解释，避免继续隐式猜测。
    pub subject_profile: CommitteeSubjectProfile,
    pub fundamental_brief: BriefingLayer,
    pub technical_brief: BriefingLayer,
    pub resonance_brief: BriefingLayer,
    pub execution_plan: ExecutionPlan,
    // 2026-04-08 CST: 这里新增赔率层，原因是闭环主线要求 briefing 正式回答“这笔交易值不值得做”；
    // 目的：把历史研究层的胜率、赔率比和期望值装配成可直接消费的结构化输出，而不是继续只留在 historical_digest。
    pub odds_brief: OddsBrief,
    // 2026-04-08 CST: 这里新增仓位层，原因是平台必须从“能分析”进入“能落仓位建议”的最小闭环；
    // 目的：让上层直接读取 starter/max position、加减仓与止损条件，而不是继续手工解释 execution_plan。
    pub position_plan: PositionPlan,
    pub committee_payload: CommitteePayload,
    pub committee_recommendations: CommitteeRecommendations,
}

// 2026-04-02 CST: 这里新增 briefing 默认携带的投决会建议集合，原因是用户明确要求普通个股分析报告也要默认带出投决建议，
// 目的：让上层用户不需要先知道“是否进入投决会”，而是在同一份 briefing 里直接看到 standard / strict / advisory 三种正式口径。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CommitteeRecommendations {
    pub default_mode: String,
    pub report_focus: String,
    pub standard: CommitteeRecommendationEntry,
    pub strict: CommitteeRecommendationEntry,
    pub advisory: CommitteeRecommendationEntry,
}

// 2026-04-02 CST: 这里把每种 committee 模式的适用场景和正式 vote 结果收口到同一结构里，原因是报告侧既要解释“什么时候看哪种建议”，
// 目的：也要保证展示内容直接复用正式 `security_committee_vote` 输出，而不是额外手写一份易漂移的摘要。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CommitteeRecommendationEntry {
    pub scenario: String,
    pub vote: SecurityCommitteeVoteResult,
}

// 2026-04-02 CST: 这里先定义交易执行层合同，原因是计划后续会在同一 briefing 中补齐可执行阈值而不是只给抽象方向判断，
// 目的：先把字段边界稳定住，后续再把每个阈值映射到真实技术指标来源。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExecutionPlan {
    pub add_trigger_price: f64,
    pub add_trigger_volume_ratio: f64,
    pub add_position_pct: f64,
    pub reduce_trigger_price: f64,
    pub rejection_zone: String,
    pub reduce_position_pct: f64,
    pub stop_loss_price: f64,
    pub invalidation_price: f64,
    pub watch_points: Vec<String>,
    pub explanation: Vec<String>,
}

// 2026-04-08 CST: 这里新增正式赔率层合同，原因是闭环研究主线不能只停在历史摘要，还需要给出“赔率是否值得做”的决策视图；
// 目的：把研究层统计结果收口成 briefing/committee 都可复用的同源事实对象。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct OddsBrief {
    pub status: String,
    pub historical_confidence: String,
    pub sample_count: usize,
    pub win_rate_10d: Option<f64>,
    pub loss_rate_10d: Option<f64>,
    pub flat_rate_10d: Option<f64>,
    pub avg_return_10d: Option<f64>,
    pub median_return_10d: Option<f64>,
    pub avg_win_return_10d: Option<f64>,
    pub avg_loss_return_10d: Option<f64>,
    pub payoff_ratio_10d: Option<f64>,
    pub expectancy_10d: Option<f64>,
    pub expected_return_window: Option<String>,
    pub expected_drawdown_window: Option<String>,
    pub odds_grade: String,
    pub confidence_grade: String,
    pub rationale: Vec<String>,
    pub research_limitations: Vec<String>,
}

impl Default for OddsBrief {
    fn default() -> Self {
        Self {
            status: "unavailable".to_string(),
            historical_confidence: "unknown".to_string(),
            sample_count: 0,
            win_rate_10d: None,
            loss_rate_10d: None,
            flat_rate_10d: None,
            avg_return_10d: None,
            median_return_10d: None,
            avg_win_return_10d: None,
            avg_loss_return_10d: None,
            payoff_ratio_10d: None,
            expectancy_10d: None,
            expected_return_window: None,
            expected_drawdown_window: None,
            odds_grade: "pending_research".to_string(),
            confidence_grade: "unknown".to_string(),
            rationale: Vec::new(),
            research_limitations: Vec::new(),
        }
    }
}

// 2026-04-08 CST: 这里新增正式仓位层合同，原因是执行计划只解决“阈值”，还没有正式回答“先下多少、最多下多少”；
// 目的：把赔率、共振和执行阈值装配成最小可执行的仓位建议层。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PositionPlan {
    pub position_action: String,
    pub entry_mode: String,
    pub starter_position_pct: f64,
    pub max_position_pct: f64,
    pub add_on_trigger: String,
    pub reduce_on_trigger: String,
    pub hard_stop_trigger: String,
    pub liquidity_cap: String,
    pub position_risk_grade: String,
    pub regime_adjustment: String,
    pub execution_notes: Vec<String>,
    pub rationale: Vec<String>,
}

impl Default for PositionPlan {
    fn default() -> Self {
        Self {
            position_action: "wait".to_string(),
            entry_mode: "research_pending".to_string(),
            starter_position_pct: 0.0,
            max_position_pct: 0.0,
            add_on_trigger: String::new(),
            reduce_on_trigger: String::new(),
            hard_stop_trigger: String::new(),
            liquidity_cap: "单次执行不超过计划仓位的 30%".to_string(),
            position_risk_grade: "high".to_string(),
            regime_adjustment: "历史研究未就绪时，先按等待或观察仓处理。".to_string(),
            execution_notes: Vec::new(),
            rationale: Vec::new(),
        }
    }
}

impl PositionPlan {
    // 2026-04-08 CST: 这里补仓位计划记录投影辅助入口，原因是 Task 1 需要把 briefing 内的 `position_plan`
    // 最小投影到正式 record 合同；目的：让 record 层只读取稳定的动作与仓位边界字段，而不在多个模块里重复手写同样的字段提取。
    pub fn record_projection(&self) -> (&str, f64, f64) {
        (
            self.position_action.as_str(),
            self.starter_position_pct,
            self.max_position_pct,
        )
    }
}

// 2026-04-02 CST: 这里先定义 committee payload 合同，原因是第一阶段虽然不实现投票引擎，但必须先把投决入口的数据口径稳定下来，
// 目的：让咨询模式和投决模式都消费同一份 factual payload，避免上层 Agent 各自再拼装一套事实。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CommitteePayload {
    pub symbol: String,
    pub analysis_date: String,
    pub recommended_action: String,
    pub confidence: String,
    // 2026-04-08 CST: 这里补充投决对象画像，原因是 vote 层需要区分“个股基本面缺口”和“ETF 天然不按财报投决”；
    // 目的：让 committee 规则能按资产类别走不同解释口径，同时保持主合同仍由同一 payload 承载。
    #[serde(default)]
    pub subject_profile: CommitteeSubjectProfile,
    // 2026-04-08 CST: 这里新增结构化风险合同，原因是标准收口版不能继续只靠扁平字符串数组传递风险事实；
    // 目的：把技术面、基本面、共振面、执行面的风险拆成稳定分类，供 briefing 展示与 committee 投票复用同一份风险底稿。
    pub risk_breakdown: CommitteeRiskBreakdown,
    // 2026-04-08 CST: 这里保留扁平摘要风险字段，原因是当前 vote 规则与部分外部调用方仍在消费 `key_risks`；
    // 目的：让旧字段明确退化成由 `risk_breakdown` 派生的摘要输出，而不是继续维护第二套平行事实。
    pub key_risks: Vec<String>,
    pub minority_objection_points: Vec<String>,
    pub evidence_version: String,
    pub briefing_digest: String,
    pub committee_schema_version: String,
    pub recommendation_digest: CommitteeRecommendationDigest,
    pub execution_digest: CommitteeExecutionDigest,
    pub resonance_digest: CommitteeResonanceDigest,
    pub evidence_checks: CommitteeEvidenceChecks,
    pub historical_digest: CommitteeHistoricalDigest,
    // 2026-04-08 CST: 这里把赔率摘要同步进 committee_payload，原因是 briefing 与投决会必须消费同一份赔率事实；
    // 目的：为后续投决会直接引用赔率等级、期望值与样本数打基础，同时避免重新拼一套平行摘要。
    #[serde(default)]
    pub odds_digest: OddsBrief,
    // 2026-04-08 CST: 这里把仓位摘要同步进 committee_payload，原因是闭环主线要求投决会能看到同源的仓位建议；
    // 目的：让 vote 层后续扩展时直接读取 position_digest，而不是回扫 briefing 顶层。
    #[serde(default)]
    pub position_digest: PositionPlan,
}

// 2026-04-02 CST: 这里把投决建议摘要收口成独立子层，原因是后续 chair 与基本面/技术面角色都要读取同一份推荐事实，
// 目的：让 vote Tool 只消费 committee payload，不再回头扫描 briefing 其他层拼接推荐语义。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CommitteeRecommendationDigest {
    pub final_stance: String,
    pub action_bias: String,
    pub summary: String,
    pub confidence: String,
}

// 2026-04-08 CST: 这里定义分析对象画像，原因是 ETF、个股、后续海外股票虽然共用同一条主链，但解释口径并不完全相同；
// 目的：先把“资产类别 + 市场范围 + 投决焦点”显式写进合同，为 ETF/海外标的的最小分流打底。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CommitteeSubjectProfile {
    pub asset_class: String,
    pub market_scope: String,
    pub committee_focus: String,
}

impl Default for CommitteeSubjectProfile {
    fn default() -> Self {
        Self {
            asset_class: "equity".to_string(),
            market_scope: "china".to_string(),
            committee_focus: "stock_review".to_string(),
        }
    }
}

// 2026-04-08 CST: 这里定义 committee 层统一风险分类合同，原因是标准收口版要让风险事实从“字符串列表”升级成“可分类、可扩展”的结构化对象；
// 目的：为后续 Skill 门禁、投决规则和报告模板共享同一风险模型打基础，同时保留现阶段最小可落地的四分类边界。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CommitteeRiskBreakdown {
    pub technical: Vec<CommitteeRiskItem>,
    pub fundamental: Vec<CommitteeRiskItem>,
    pub resonance: Vec<CommitteeRiskItem>,
    pub execution: Vec<CommitteeRiskItem>,
}

// 2026-04-08 CST: 这里定义单条结构化风险项，原因是 committee payload 需要承载比 headline 更稳定的风险语义；
// 目的：让上层既能直接展示 headline，也能在需要时继续读取 severity 与 rationale 做更细粒度解释。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CommitteeRiskItem {
    pub category: String,
    pub severity: String,
    pub headline: String,
    pub rationale: String,
}

// 2026-04-02 CST: 这里把执行层复制成 committee digest，原因是 execution reviewer 需要结构化阈值而不是只看 briefing 摘要，
// 目的：把“怎么做、在哪做、什么情况下撤”固定为可投票、可展示、可留痕的事实子层。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CommitteeExecutionDigest {
    pub add_trigger_price: f64,
    pub add_trigger_volume_ratio: f64,
    pub add_position_pct: f64,
    pub reduce_trigger_price: f64,
    pub reduce_position_pct: f64,
    pub stop_loss_price: f64,
    pub invalidation_price: f64,
    pub rejection_zone: String,
    pub watch_points: Vec<String>,
    pub explanation: Vec<String>,
}

// 2026-04-02 CST: 这里把共振层压成 committee 摘要，原因是投决角色更关心“驱动是什么、反向扰动是什么”，
// 目的：在不暴露整层复杂对象的前提下，把共振证据稳定映射成投票输入。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CommitteeResonanceDigest {
    pub resonance_score: f64,
    pub action_bias: String,
    pub top_positive_driver_names: Vec<String>,
    pub top_negative_driver_names: Vec<String>,
    pub event_override_titles: Vec<String>,
}

// 2026-04-02 CST: 这里显式写出证据就绪检查，原因是风险官与主席需要先判断“是否已具备正式表决条件”，
// 目的：把 briefing 各层 readiness 变成确定性的布尔信号，而不是让不同角色各自猜测。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CommitteeEvidenceChecks {
    pub fundamental_ready: bool,
    pub technical_ready: bool,
    pub resonance_ready: bool,
    pub execution_ready: bool,
    pub briefing_ready: bool,
}

// 2026-04-02 CST: 这里预留历史研究摘要，原因是方案 B 要让 vote Tool 先支持 unavailable 边界，再平滑接入研究增强，
// 目的：避免 signal outcome 研究层后续接入时再次改动 committee payload 主合同。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CommitteeHistoricalDigest {
    pub status: String,
    pub historical_confidence: String,
    pub analog_sample_count: usize,
    pub analog_win_rate_10d: Option<f64>,
    pub analog_loss_rate_10d: Option<f64>,
    pub analog_flat_rate_10d: Option<f64>,
    pub analog_avg_return_10d: Option<f64>,
    pub analog_median_return_10d: Option<f64>,
    pub analog_avg_win_return_10d: Option<f64>,
    pub analog_avg_loss_return_10d: Option<f64>,
    pub analog_payoff_ratio_10d: Option<f64>,
    pub analog_expectancy_10d: Option<f64>,
    pub expected_return_window: Option<String>,
    pub expected_drawdown_window: Option<String>,
    pub research_limitations: Vec<String>,
}

#[derive(Debug, Error)]
pub enum SecurityDecisionBriefingError {
    #[error("security_decision_briefing 复用共振分析失败: {0}")]
    Resonance(#[from] SecurityAnalysisResonanceError),
    #[error("security_decision_briefing 序列化子层失败: {0}")]
    Serialization(String),
    #[error("security_decision_briefing 生成默认投决会建议失败: {0}")]
    CommitteeVote(#[from] SecurityCommitteeVoteError),
}

// 2026-04-02 CST: 这里先定义 briefing 子层允许承载的结构化对象类型，原因是合同红测阶段需要锁定子层字段存在性而不是业务计算结果，
// 目的：为后续测试和调试保留轻量级占位载体，同时不阻断后续替换成真实事实层对象。
pub type BriefingLayer = Value;

// 2026-04-02 CST: 这里补 security_decision_briefing assembler 主入口，原因是第二步需要把已有 technical/fullstack/resonance
// 事实层装配成单一 briefing 结构；目的：让后续咨询、交易执行和 committee payload 都围绕同一份事实载体继续扩展。
pub fn security_decision_briefing(
    request: &SecurityDecisionBriefingRequest,
) -> Result<SecurityDecisionBriefingResult, SecurityDecisionBriefingError> {
    let resonance_request = SecurityAnalysisResonanceRequest {
        symbol: request.symbol.clone(),
        market_symbol: request.market_symbol.clone(),
        sector_symbol: request.sector_symbol.clone(),
        market_regime: request.market_regime.clone(),
        sector_template: request.sector_template.clone(),
        as_of_date: request.as_of_date.clone(),
        lookback_days: request.lookback_days,
        factor_lookback_days: request.factor_lookback_days,
        disclosure_limit: request.disclosure_limit,
    };
    let resonance_analysis = security_analysis_resonance(&resonance_request)?;
    assemble_security_decision_briefing(resonance_analysis)
}

// 2026-04-02 CST: 这里把 assembler 收口成单独函数，原因是后续还要继续给 execution_plan 与 committee_payload 做增量增强；
// 目的：让“复用既有事实层”和“生成 briefing 子层”分离，降低后续扩展交易执行层时的修改面。
fn assemble_security_decision_briefing(
    analysis: SecurityAnalysisResonanceResult,
) -> Result<SecurityDecisionBriefingResult, SecurityDecisionBriefingError> {
    // 2026-04-08 CST: 这里先推断分析对象画像，原因是 ETF 与个股需要共用 briefing 主链但解释口径不同；
    // 目的：让 summary、committee payload 与默认投决建议都能按同一份画像做最小分流。
    let subject_profile = infer_subject_profile(&analysis.symbol);
    let analysis_date = analysis
        .base_analysis
        .technical_context
        .stock_analysis
        .analysis_date
        .clone();
    let evidence_version = build_evidence_version(&analysis.symbol, &analysis_date);
    let summary = build_summary(&analysis, &subject_profile);
    let technical_brief =
        serialize_layer(&analysis.base_analysis.technical_context.stock_analysis)?;
    let fundamental_brief = serialize_layer(&analysis.base_analysis.fundamental_context)?;
    let resonance_brief = serialize_layer(&analysis.resonance_context)?;
    let execution_plan = build_execution_plan(&analysis);
    // 2026-04-08 CST: 这里先统一装配历史研究 -> 赔率 -> 仓位三层，原因是这三层必须共享同一份 analysis_date 与 subject_profile；
    // 目的：确保 briefing 顶层与 committee_payload 读取到的是同源事实，而不是各自再触发一轮独立计算。
    let historical_digest =
        build_historical_digest(&analysis.symbol, &analysis_date, &subject_profile);
    let odds_brief = build_odds_brief(&historical_digest);
    let position_plan = build_position_plan(
        &analysis,
        &execution_plan,
        &historical_digest,
        &odds_brief,
        &subject_profile,
    );
    let committee_payload = build_committee_payload(
        &analysis,
        &analysis_date,
        &summary,
        &evidence_version,
        &execution_plan,
        &subject_profile,
        &historical_digest,
        &odds_brief,
        &position_plan,
    );
    let committee_recommendations =
        build_committee_recommendations(&committee_payload, &subject_profile)?;

    Ok(SecurityDecisionBriefingResult {
        symbol: analysis.symbol,
        analysis_date,
        analysis_date_guard: analysis
            .base_analysis
            .technical_context
            .analysis_date_guard
            .clone(),
        summary,
        evidence_version,
        subject_profile,
        fundamental_brief,
        technical_brief,
        resonance_brief,
        execution_plan,
        odds_brief,
        position_plan,
        committee_payload,
        committee_recommendations,
    })
}

// 2026-04-02 CST: 这里统一做子层序列化，原因是 briefing 当前阶段只需要稳定输出结构化 JSON 合同，不需要把内部源对象继续暴露为耦合类型；
// 目的：让 assembler 在保持事实层完整度的同时，把对外合同稳定收口成可测试的 JSON 子对象。
fn serialize_layer<T: Serialize>(
    value: &T,
) -> Result<BriefingLayer, SecurityDecisionBriefingError> {
    serde_json::to_value(value)
        .map_err(|error| SecurityDecisionBriefingError::Serialization(error.to_string()))
}

// 2026-04-02 CST: 这里先生成统一 evidence_version，原因是 committee payload 阶段一已经要求咨询与投决共享同一份事实版本标识；
// 目的：让后续 vote Tool 可以用同一版本号识别 briefing 是否来自同一份底稿。
fn build_evidence_version(symbol: &str, analysis_date: &str) -> String {
    format!("security-decision-briefing:{symbol}:{analysis_date}:v1")
}

// 2026-04-02 CST: 这里先把 integrated_conclusion 和 resonance bias 拼成简报摘要，原因是当前 assembler 阶段需要先交付一个稳定 summary 字段；
// 目的：让上层调用方能先拿到“综合结论 + 共振偏向”的单句摘要，后续再继续增强交易执行与投决内容。
fn build_summary(
    analysis: &SecurityAnalysisResonanceResult,
    subject_profile: &CommitteeSubjectProfile,
) -> String {
    let headline = if subject_profile.asset_class == "etf" {
        analysis
            .base_analysis
            .technical_context
            .stock_analysis
            .consultation_conclusion
            .headline
            .clone()
    } else {
        analysis
            .base_analysis
            .integrated_conclusion
            .headline
            .clone()
    };

    format!(
        "{}；共振动作偏向为 {}。",
        headline, analysis.resonance_context.action_bias
    )
}

// 2026-04-02 CST: 这里把 execution_plan 正式改为指标派生结构，原因是 Task 3 要求 briefing 不再停留在占位阈值，而要输出可执行交易门槛；
// 目的：把阻力位、量比门槛、承接位、强弱分界与趋势失效位统一收口到 briefing 内，避免上层 Agent 再手工拼阈值。
fn build_execution_plan(analysis: &SecurityAnalysisResonanceResult) -> ExecutionPlan {
    let snapshot = &analysis
        .base_analysis
        .technical_context
        .stock_analysis
        .indicator_snapshot;
    let action_bias = analysis.resonance_context.action_bias.as_str();
    let add_trigger_price = round_price(snapshot.resistance_level_20.max(snapshot.close));
    let add_trigger_volume_ratio = round_ratio(snapshot.volume_ratio_20.max(1.05));
    let add_position_pct = match action_bias {
        "add_on_strength" => 0.12,
        "hold_and_confirm" => 0.06,
        "watch_conflict" => 0.04,
        _ => 0.03,
    };
    let reduce_trigger_price = round_price(snapshot.ema_10.min(snapshot.close));
    let reduce_position_pct = match action_bias {
        "reduce_or_exit" => 0.20,
        "watch_conflict" => 0.12,
        _ => 0.08,
    };
    let stop_loss_price = round_price(snapshot.boll_middle);
    let invalidation_price = round_price(snapshot.sma_50);
    let rejection_zone = format!(
        "{:.2}-{:.2}",
        snapshot.resistance_level_20,
        snapshot.resistance_level_20 + snapshot.atr_14.max(0.01)
    );
    let mut watch_points = analysis
        .base_analysis
        .technical_context
        .stock_analysis
        .watch_points
        .clone();
    watch_points.push(format!(
        "若量比未达到 {:.2} 以上，突破阻力位后不追价。",
        add_trigger_volume_ratio
    ));
    watch_points.push(format!(
        "若跌破 {:.2} 的短承接位，优先执行减仓观察。",
        reduce_trigger_price
    ));

    ExecutionPlan {
        add_trigger_price,
        add_trigger_volume_ratio,
        add_position_pct,
        reduce_trigger_price,
        rejection_zone,
        reduce_position_pct,
        stop_loss_price,
        invalidation_price,
        watch_points,
        explanation: vec![
            format!(
                "加仓触发价取自 resistance_level_20={:.2}，对应近期 20 日阻力位突破确认。",
                snapshot.resistance_level_20
            ),
            format!(
                "放量门槛基于 volume_ratio_20={:.2} 设定，避免无量突破误判。",
                snapshot.volume_ratio_20
            ),
            format!(
                "减仓触发价取自 ema_10={:.2}，用于识别短趋势承接是否失守。",
                snapshot.ema_10
            ),
            format!(
                "止损价取自 boll_middle={:.2}，用于监控强弱分界是否被跌破。",
                snapshot.boll_middle
            ),
            format!(
                "失效价取自 sma_50={:.2}，用于识别中期趋势是否正式破坏。",
                snapshot.sma_50
            ),
        ],
    }
}

// 2026-04-08 CST: 这里把历史研究摘要装配成正式赔率层，原因是闭环主线需要一个能直接回答“这笔交易赔率怎样”的稳定对象；
// 目的：让 briefing 与 committee 都消费同一份 odds_brief，而不是继续把概率、赔率和期望值散落在 historical_digest 中。
fn build_odds_brief(historical_digest: &CommitteeHistoricalDigest) -> OddsBrief {
    if historical_digest.status != "available" {
        return OddsBrief {
            status: historical_digest.status.clone(),
            historical_confidence: historical_digest.historical_confidence.clone(),
            sample_count: historical_digest.analog_sample_count,
            win_rate_10d: historical_digest.analog_win_rate_10d,
            loss_rate_10d: historical_digest.analog_loss_rate_10d,
            flat_rate_10d: historical_digest.analog_flat_rate_10d,
            avg_return_10d: historical_digest.analog_avg_return_10d,
            median_return_10d: historical_digest.analog_median_return_10d,
            avg_win_return_10d: historical_digest.analog_avg_win_return_10d,
            avg_loss_return_10d: historical_digest.analog_avg_loss_return_10d,
            payoff_ratio_10d: historical_digest.analog_payoff_ratio_10d,
            expectancy_10d: historical_digest.analog_expectancy_10d,
            expected_return_window: historical_digest.expected_return_window.clone(),
            expected_drawdown_window: historical_digest.expected_drawdown_window.clone(),
            odds_grade: "pending_research".to_string(),
            confidence_grade: "unknown".to_string(),
            rationale: vec![
                "历史研究尚未就绪，当前不输出正式赔率结论。".to_string(),
                "进入仓位决策时只允许等待或观察仓语义。".to_string(),
            ],
            research_limitations: historical_digest.research_limitations.clone(),
        };
    }

    let win_rate = historical_digest.analog_win_rate_10d;
    let payoff_ratio = historical_digest.analog_payoff_ratio_10d;
    let expectancy = historical_digest.analog_expectancy_10d;
    let odds_grade = classify_odds_grade(win_rate, payoff_ratio, expectancy);
    let confidence_grade = classify_confidence_grade(
        historical_digest.historical_confidence.as_str(),
        historical_digest.analog_sample_count,
    );

    OddsBrief {
        status: historical_digest.status.clone(),
        historical_confidence: historical_digest.historical_confidence.clone(),
        sample_count: historical_digest.analog_sample_count,
        win_rate_10d: historical_digest.analog_win_rate_10d,
        loss_rate_10d: historical_digest.analog_loss_rate_10d,
        flat_rate_10d: historical_digest.analog_flat_rate_10d,
        avg_return_10d: historical_digest.analog_avg_return_10d,
        median_return_10d: historical_digest.analog_median_return_10d,
        avg_win_return_10d: historical_digest.analog_avg_win_return_10d,
        avg_loss_return_10d: historical_digest.analog_avg_loss_return_10d,
        payoff_ratio_10d: historical_digest.analog_payoff_ratio_10d,
        expectancy_10d: historical_digest.analog_expectancy_10d,
        expected_return_window: historical_digest.expected_return_window.clone(),
        expected_drawdown_window: historical_digest.expected_drawdown_window.clone(),
        odds_grade: odds_grade.to_string(),
        confidence_grade,
        rationale: vec![
            format!(
                "10日胜率 {:.1}%，赔率比 {}，期望值 {}。",
                win_rate.unwrap_or(0.0) * 100.0,
                format_optional_ratio(payoff_ratio),
                format_optional_pct(expectancy)
            ),
            format!(
                "收益区间 {}；回撤区间 {}。",
                historical_digest
                    .expected_return_window
                    .clone()
                    .unwrap_or_else(|| "暂无".to_string()),
                historical_digest
                    .expected_drawdown_window
                    .clone()
                    .unwrap_or_else(|| "暂无".to_string())
            ),
            format!(
                "样本数 {}，历史置信度为 {}。",
                historical_digest.analog_sample_count, historical_digest.historical_confidence
            ),
        ],
        research_limitations: historical_digest.research_limitations.clone(),
    }
}

// 2026-04-08 CST: 这里把赔率层、共振层与执行层装配成正式仓位层，原因是 execution_plan 只有价位阈值，还没有正式仓位建议；
// 目的：用最小规则分档给出 starter/max position、加减仓条件与流动性限制，形成闭环主线的投中层。
fn build_position_plan(
    analysis: &SecurityAnalysisResonanceResult,
    execution_plan: &ExecutionPlan,
    historical_digest: &CommitteeHistoricalDigest,
    odds_brief: &OddsBrief,
    subject_profile: &CommitteeSubjectProfile,
) -> PositionPlan {
    let snapshot = &analysis
        .base_analysis
        .technical_context
        .stock_analysis
        .indicator_snapshot;
    let action_bias = analysis.resonance_context.action_bias.as_str();
    let resonance_score = analysis.resonance_context.resonance_score;
    let (base_starter, base_max) = base_position_limits_by_odds(&odds_brief.odds_grade);
    let confidence_penalty =
        confidence_penalty_pct(historical_digest.historical_confidence.as_str());
    let resonance_adjustment = if resonance_score >= 0.75 {
        0.02
    } else if resonance_score <= 0.45 {
        -0.02
    } else {
        0.0
    };
    let subject_cap = if subject_profile.asset_class == "etf" {
        0.28
    } else {
        0.35
    };

    let mut starter_position_pct = clamp_pct(
        base_starter + resonance_adjustment - confidence_penalty,
        0.0,
        subject_cap,
    );
    let mut max_position_pct = clamp_pct(
        base_max + resonance_adjustment - confidence_penalty,
        starter_position_pct,
        subject_cap,
    );

    let (position_action, entry_mode) = if action_bias == "reduce_or_exit" {
        starter_position_pct = 0.0;
        max_position_pct = 0.08;
        ("defensive_reduce".to_string(), "defensive_exit".to_string())
    } else if odds_brief.status != "available" || odds_brief.odds_grade == "pending_research" {
        starter_position_pct = starter_position_pct.min(0.04);
        max_position_pct = max_position_pct.min(0.08);
        ("pilot_only".to_string(), "research_pending".to_string())
    } else if odds_brief.odds_grade == "A" && action_bias == "add_on_strength" {
        (
            "build_on_strength".to_string(),
            "breakout_confirmation".to_string(),
        )
    } else if odds_brief.odds_grade == "B" || action_bias == "hold_and_confirm" {
        (
            "starter_then_confirm".to_string(),
            "breakout_confirmation".to_string(),
        )
    } else if odds_brief.odds_grade == "C" || action_bias == "watch_conflict" {
        starter_position_pct = starter_position_pct.min(0.06);
        max_position_pct = max_position_pct.min(0.12);
        ("pilot_only".to_string(), "range_confirmation".to_string())
    } else {
        starter_position_pct = 0.0;
        max_position_pct = max_position_pct.min(0.05);
        ("wait".to_string(), "wait_for_edge".to_string())
    };

    let liquidity_cap = classify_liquidity_cap(snapshot.volume_ratio_20, subject_profile);
    let position_risk_grade = classify_position_risk_grade(
        &odds_brief.odds_grade,
        historical_digest.historical_confidence.as_str(),
        resonance_score,
    );

    PositionPlan {
        position_action,
        entry_mode,
        starter_position_pct: round_ratio(starter_position_pct),
        max_position_pct: round_ratio(max_position_pct),
        add_on_trigger: format!(
            "仅在价格站上 {:.2} 且量比达到 {:.2} 后，按 {:.0}% 节奏追加仓位。",
            execution_plan.add_trigger_price,
            execution_plan.add_trigger_volume_ratio,
            execution_plan.add_position_pct * 100.0
        ),
        reduce_on_trigger: format!(
            "若价格跌破 {:.2}，先按 {:.0}% 节奏减仓并观察 {}。",
            execution_plan.reduce_trigger_price,
            execution_plan.reduce_position_pct * 100.0,
            execution_plan.rejection_zone
        ),
        hard_stop_trigger: format!(
            "若跌破 {:.2} 或 {:.2}，结束当前交易假设。",
            execution_plan.stop_loss_price, execution_plan.invalidation_price
        ),
        liquidity_cap,
        position_risk_grade,
        regime_adjustment: if resonance_score >= 0.7 {
            "当前共振偏强，可在确认后逐级上调仓位，但单次执行仍需分批。".to_string()
        } else {
            "当前共振强度一般或偏弱，应维持小仓位与更高复核频率。".to_string()
        },
        execution_notes: vec![
            format!(
                "当前 action_bias={}，因此仓位动作优先围绕 `{}` 执行。",
                action_bias, odds_brief.odds_grade
            ),
            format!(
                "执行层关键位来自 add {:.2} / reduce {:.2} / stop {:.2}。",
                execution_plan.add_trigger_price,
                execution_plan.reduce_trigger_price,
                execution_plan.stop_loss_price
            ),
        ],
        rationale: vec![
            format!(
                "仓位上限由赔率等级 {}、历史置信度 {} 与共振分数 {:.2} 共同决定。",
                odds_brief.odds_grade, historical_digest.historical_confidence, resonance_score
            ),
            format!(
                "当前给出 starter {:.0}% / max {:.0}% 的分档建议。",
                starter_position_pct * 100.0,
                max_position_pct * 100.0
            ),
        ],
    }
}

// 2026-04-02 CST: 这里补一个价格四舍五入助手，原因是 execution_plan 里的阈值是给上层直接阅读和执行的，不适合暴露过长浮点尾数；
// 目的：把技术层原始数值统一整理成更稳定的交易价位显示，同时不改变其指标来源。
fn round_price(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

// 2026-04-02 CST: 这里补一个比例四舍五入助手，原因是量比与仓位比例属于执行阈值，过长小数会降低可读性与可执行性；
// 目的：让 execution_plan 输出稳定、易读，又不丢掉核心方向性信息。
fn round_ratio(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

// 2026-04-08 CST: 这里集中定义赔率等级规则，原因是 V1 赔率系统先采用规则分档而不是复杂统计模型；
// 目的：让 A/B/C/D/E 等级具备稳定、可解释、可测试的判断口径。
fn classify_odds_grade(
    win_rate: Option<f64>,
    payoff_ratio: Option<f64>,
    expectancy: Option<f64>,
) -> &'static str {
    let Some(win_rate) = win_rate else {
        return "pending_research";
    };
    let Some(payoff_ratio) = payoff_ratio else {
        return "pending_research";
    };
    let Some(expectancy) = expectancy else {
        return "pending_research";
    };

    if win_rate >= 0.60 && payoff_ratio >= 1.50 && expectancy > 0.0 {
        "A"
    } else if win_rate >= 0.55 && payoff_ratio >= 1.20 && expectancy > 0.0 {
        "B"
    } else if win_rate >= 0.48 && expectancy > 0.0 {
        "C"
    } else if expectancy > -0.01 {
        "D"
    } else {
        "E"
    }
}

// 2026-04-08 CST: 这里集中把历史置信度映射成对外展示等级，原因是 briefing 需要把样本置信度转成更直观的消费者语义；
// 目的：让上层看到 `confidence_grade` 时不必再手工解释 sample_count 与 historical_confidence 的关系。
fn classify_confidence_grade(historical_confidence: &str, sample_count: usize) -> String {
    if sample_count >= 12 && historical_confidence == "high" {
        "high".to_string()
    } else if sample_count >= 6 && matches!(historical_confidence, "medium" | "high") {
        "medium".to_string()
    } else if sample_count > 0 {
        "low".to_string()
    } else {
        "unknown".to_string()
    }
}

// 2026-04-08 CST: 这里集中定义赔率等级对应的基础仓位分档，原因是仓位层需要先有一个稳定的基础值，再叠加共振与置信度修正；
// 目的：让 starter/max position 的来源可解释、可测试，并避免把阈值散落在多处分支中。
fn base_position_limits_by_odds(odds_grade: &str) -> (f64, f64) {
    match odds_grade {
        "A" => (0.12, 0.28),
        "B" => (0.08, 0.20),
        "C" => (0.05, 0.12),
        "D" => (0.02, 0.06),
        _ => (0.0, 0.04),
    }
}

// 2026-04-08 CST: 这里把历史置信度转换成仓位惩罚项，原因是样本可信度不足时不能只靠 action_bias 放大仓位；
// 目的：让低样本或未知研究状态自动触发降档，而不是继续依赖人工克制。
fn confidence_penalty_pct(historical_confidence: &str) -> f64 {
    match historical_confidence {
        "high" => 0.0,
        "medium" => 0.01,
        "low" => 0.03,
        _ => 0.05,
    }
}

// 2026-04-08 CST: 这里统一定义流动性上限文案，原因是仓位系统 V1 不做组合级成交冲击模型；
// 目的：先用量比和资产类别作为免费数据代理，明确单次执行应分几档推进。
fn classify_liquidity_cap(
    volume_ratio_20: f64,
    subject_profile: &CommitteeSubjectProfile,
) -> String {
    let cap: f64 = if volume_ratio_20 >= 1.20 {
        0.70
    } else if volume_ratio_20 >= 1.00 {
        0.50
    } else {
        0.30
    };
    if subject_profile.asset_class == "etf" {
        // 2026-04-08 CST: 这里给浮点常量补 f64 类型，原因是当前工具链下 `(cap + 0.10).min(0.80)` 出现字面量类型推断歧义，
        // 会阻断整个 crate 编译，连 foundation 元数据测试也无法运行。
        // 目的：只做最小编译修复，不改变原有仓位上限语义。
        format!(
            "单次执行不超过计划仓位的 {:.0}%",
            (cap + 0.10_f64).min(0.80_f64) * 100.0_f64
        )
    } else {
        format!("单次执行不超过计划仓位的 {:.0}%", cap * 100.0)
    }
}

// 2026-04-08 CST: 这里集中定义仓位风险等级，原因是仓位系统对外必须明确表达“建议仓位大不大”和“风险高不高”是两件事；
// 目的：让上层既能看到仓位比例，也能看到当前建议所对应的风险档位。
fn classify_position_risk_grade(
    odds_grade: &str,
    historical_confidence: &str,
    resonance_score: f64,
) -> String {
    if odds_grade == "A" && historical_confidence == "high" && resonance_score >= 0.70 {
        "medium".to_string()
    } else if matches!(odds_grade, "B" | "C")
        && matches!(historical_confidence, "medium" | "high")
        && resonance_score >= 0.55
    {
        "medium".to_string()
    } else {
        "high".to_string()
    }
}

// 2026-04-08 CST: 这里补一个仓位百分比夹紧助手，原因是 starter/max position 要受下限、上限与资产类别约束；
// 目的：避免规则分档和共振修正叠加后出现负仓位或超上限结果。
fn clamp_pct(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

// 2026-04-08 CST: 这里补一个可选百分比格式化助手，原因是赔率层 rationale 需要把 Option<f64> 转成对外可读文字；
// 目的：减少上层文案构造的重复判断，并保持 unavailable 状态下的表述稳定。
fn format_optional_pct(value: Option<f64>) -> String {
    value
        .map(|number| format!("{:.2}%", number * 100.0))
        .unwrap_or_else(|| "暂无".to_string())
}

// 2026-04-08 CST: 这里补一个可选倍率格式化助手，原因是赔率比在 rationale 中需要直接输出给用户阅读；
// 目的：让 payoff ratio 缺失时统一回落到“暂无”，避免出现 `null` 风格的对外文本。
fn format_optional_ratio(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.2}"))
        .unwrap_or_else(|| "暂无".to_string())
}

// 2026-04-02 CST: 这里先补 committee payload 的最小装配，原因是第一阶段虽然还不做投票引擎，但必须先让投决端拿到稳定事实入口；
// 目的：确保咨询模式和投决模式共享同一份 symbol / date / evidence_version / risk digest，而不是各自重拼底稿。
fn build_committee_payload(
    analysis: &SecurityAnalysisResonanceResult,
    analysis_date: &str,
    summary: &str,
    evidence_version: &str,
    execution_plan: &ExecutionPlan,
    subject_profile: &CommitteeSubjectProfile,
    historical_digest: &CommitteeHistoricalDigest,
    odds_digest: &OddsBrief,
    position_digest: &PositionPlan,
) -> CommitteePayload {
    // 2026-04-02 CST: 这里先把综合结论压成 recommendation digest，原因是投决会后续必须只消费 committee payload 而不能回头扫 briefing 细节，
    // 目的：让 chair 与各 reviewer 在同一份结构化推荐事实上表态，避免继续围绕扁平摘要重复解释。
    let recommendation_digest = CommitteeRecommendationDigest {
        final_stance: analysis.base_analysis.integrated_conclusion.stance.clone(),
        action_bias: analysis.resonance_context.action_bias.clone(),
        // 2026-04-08 CST: 这里按资产类别切换 recommendation 摘要来源，原因是 ETF 主链不能继续复用个股式 integrated headline。
        // 目的：让 committee payload 在最小改动下直接输出 ETF 可解释摘要，避免把指数/基金分析误说成公司基本面结论。
        summary: if subject_profile.asset_class == "etf" {
            analysis
                .base_analysis
                .technical_context
                .stock_analysis
                .consultation_conclusion
                .headline
                .clone()
        } else {
            analysis
                .base_analysis
                .integrated_conclusion
                .headline
                .clone()
        },
        confidence: analysis
            .base_analysis
            .technical_context
            .stock_analysis
            .consultation_conclusion
            .confidence
            .clone(),
    };
    // 2026-04-02 CST: 这里把 execution_plan 显式复制进 committee digest，原因是 execution reviewer 需要读到确定阈值而不是再次推导，
    // 目的：把“何时加减仓、何时失效”的边界固定为投票输入，后续 CLI/Skill/GUI 都复用同一份执行事实。
    let execution_digest = CommitteeExecutionDigest {
        add_trigger_price: execution_plan.add_trigger_price,
        add_trigger_volume_ratio: execution_plan.add_trigger_volume_ratio,
        add_position_pct: execution_plan.add_position_pct,
        reduce_trigger_price: execution_plan.reduce_trigger_price,
        reduce_position_pct: execution_plan.reduce_position_pct,
        stop_loss_price: execution_plan.stop_loss_price,
        invalidation_price: execution_plan.invalidation_price,
        rejection_zone: execution_plan.rejection_zone.clone(),
        watch_points: execution_plan.watch_points.clone(),
        explanation: execution_plan.explanation.clone(),
    };
    // 2026-04-02 CST: 这里把共振上下文压缩成角色可直接消费的摘要，原因是投决层更关心驱动项、负向扰动和事件覆盖而不是底层评估细节，
    // 目的：在不泄露整层内部结构的前提下，保留足够的驱动解释能力支撑固定角色投票。
    let resonance_digest = CommitteeResonanceDigest {
        resonance_score: analysis.resonance_context.resonance_score,
        action_bias: analysis.resonance_context.action_bias.clone(),
        top_positive_driver_names: analysis
            .resonance_context
            .top_positive_resonances
            .iter()
            .map(|driver| driver.display_name.clone())
            .collect(),
        top_negative_driver_names: analysis
            .resonance_context
            .top_negative_resonances
            .iter()
            .map(|driver| driver.display_name.clone())
            .collect(),
        event_override_titles: analysis
            .resonance_context
            .event_overrides
            .iter()
            .map(|event| event.title.clone())
            .collect(),
    };
    // 2026-04-02 CST: 这里把 readiness 状态显式固化出来，原因是 risk officer 与 chair 需要先判断“是否具备正式表决条件”，
    // 目的：把各层是否齐备从隐式推断变成确定布尔信号，减少不同角色各自猜测事实边界。
    let evidence_checks = CommitteeEvidenceChecks {
        // 2026-04-08 CST: 这里给 ETF 放开 fundamental_ready，原因是 ETF 天然不按单一公司财报口径投决。
        // 目的：避免 ETF 因“没有个股财报”被主链直接卡死，同时继续保留个股的硬门槛。
        fundamental_ready: subject_profile.asset_class == "etf"
            || analysis.base_analysis.fundamental_context.status == "available",
        technical_ready: true,
        resonance_ready: true,
        execution_ready: true,
        briefing_ready: true,
    };
    // 2026-04-08 CST: 这里先装配结构化风险合同，原因是标准收口版要求 committee_payload 以统一分类风险模型为主；
    // 目的：让旧 `key_risks` 摘要和后续 vote/Skill 扩展都从同一份 risk_breakdown 派生，而不是继续维护平行风险事实。
    let risk_breakdown = build_committee_risk_breakdown(analysis, &execution_plan, subject_profile);
    let key_risks = summarize_committee_key_risks(&risk_breakdown);

    CommitteePayload {
        symbol: analysis.symbol.clone(),
        analysis_date: analysis_date.to_string(),
        recommended_action: analysis.resonance_context.action_bias.clone(),
        confidence: analysis
            .base_analysis
            .technical_context
            .stock_analysis
            .consultation_conclusion
            .confidence
            .clone(),
        // 2026-04-08 CST: 这里显式写入 subject_profile，原因是后续 vote 层需要知道当前是 ETF 还是个股。
        // 目的：把资产类别判断从隐式猜测变成正式合同字段，减少后续继续散落写 symbol 前缀判断。
        subject_profile: subject_profile.clone(),
        risk_breakdown,
        key_risks,
        minority_objection_points: analysis
            .resonance_context
            .top_negative_resonances
            .iter()
            .take(2)
            .map(|driver| format!("{} 存在负向共振或背离风险", driver.display_name))
            .collect(),
        evidence_version: evidence_version.to_string(),
        briefing_digest: summary.to_string(),
        committee_schema_version: "committee-payload:v1".to_string(),
        recommendation_digest,
        execution_digest,
        resonance_digest,
        evidence_checks,
        historical_digest: historical_digest.clone(),
        odds_digest: odds_digest.clone(),
        position_digest: position_digest.clone(),
    }
}

// 2026-04-08 CST: 这里集中装配 committee 层的结构化风险合同，原因是标准收口版要求风险事实先分层、再派生摘要；
// 目的：把技术面、基本面、共振面、执行面的风险统一收口到同一 builder，避免 briefing 和 committee 各自再拼一套风险语义。
fn build_committee_risk_breakdown(
    analysis: &SecurityAnalysisResonanceResult,
    execution_plan: &ExecutionPlan,
    subject_profile: &CommitteeSubjectProfile,
) -> CommitteeRiskBreakdown {
    let technical = analysis
        .base_analysis
        .technical_context
        .stock_analysis
        .consultation_conclusion
        .risk_flags
        .iter()
        .take(2)
        .map(|risk| CommitteeRiskItem {
            category: "technical".to_string(),
            severity: "medium".to_string(),
            headline: risk.clone(),
            rationale: "技术面风险来自趋势、量价、关键位和时点信号的失效提示。".to_string(),
        })
        .collect();

    let fundamental = build_fundamental_risk_items(analysis, subject_profile);
    let resonance = build_resonance_risk_items(analysis);
    let execution = vec![CommitteeRiskItem {
        category: "execution".to_string(),
        severity: "medium".to_string(),
        headline: format!(
            "若价格跌破 {:.2} 或 {:.2}，当前交易假设将失效。",
            execution_plan.stop_loss_price, execution_plan.invalidation_price
        ),
        rationale: format!(
            "执行层当前以止损位 {:.2} 和失效位 {:.2} 作为交易假设边界，且需持续观察 {}。",
            execution_plan.stop_loss_price,
            execution_plan.invalidation_price,
            execution_plan.rejection_zone
        ),
    }];

    CommitteeRiskBreakdown {
        technical,
        fundamental,
        resonance,
        execution,
    }
}

// 2026-04-08 CST: 这里从结构化风险合同派生旧摘要字段，原因是当前 vote 规则与部分合同回归仍然依赖 `key_risks`；
// 目的：明确 `key_risks` 不再是独立事实源，而是 risk_breakdown 的摘要投影，避免双轨风险口径继续存在。
fn summarize_committee_key_risks(risk_breakdown: &CommitteeRiskBreakdown) -> Vec<String> {
    let mut key_risks = Vec::new();

    for items in [
        &risk_breakdown.technical,
        &risk_breakdown.fundamental,
        &risk_breakdown.resonance,
        &risk_breakdown.execution,
    ] {
        if let Some(item) = items.first() {
            key_risks.push(item.headline.clone());
        }
    }

    key_risks
}

// 2026-04-08 CST: 这里集中装配基本面风险项，原因是财报/公告可用性与同比风险需要作为独立类别进入结构化合同；
// 目的：让 committee 层能区分“基本面证据未就绪”和“已就绪但存在财报/公告风险”这两类不同语义。
fn build_fundamental_risk_items(
    analysis: &SecurityAnalysisResonanceResult,
    subject_profile: &CommitteeSubjectProfile,
) -> Vec<CommitteeRiskItem> {
    // 2026-04-08 CST: 这里先给 ETF 单独风险语义，原因是 ETF 的核心缺口不是“财报未就绪”，而是跟踪误差与指数研究未接入。
    // 目的：让 ETF 进入主链后暴露真实边界，而不是继续被错误标红为“缺财报”。
    if subject_profile.asset_class == "etf" {
        return vec![CommitteeRiskItem {
            category: "fundamental".to_string(),
            severity: "medium".to_string(),
            headline: "ETF 当前缺少跟踪误差、底层指数与申赎结构的专用研究。".to_string(),
            rationale: "ETF 投决重点不在单一公司财报，而在跟踪质量、指数结构、流动性与申赎机制；当前主链先把这类缺口显式暴露为专项研究待补。"
                .to_string(),
        }];
    }

    let context = &analysis.base_analysis.fundamental_context;
    if context.status != "available" {
        return vec![CommitteeRiskItem {
            category: "fundamental".to_string(),
            severity: "high".to_string(),
            headline: "基本面证据未就绪，财报与公告快照仍需补齐。".to_string(),
            rationale: "当前基本面子层未达到 available，投决时需要把信息缺口本身视为风险。"
                .to_string(),
        }];
    }

    context
        .risk_flags
        .iter()
        .take(2)
        .map(|risk| CommitteeRiskItem {
            category: "fundamental".to_string(),
            severity: "medium".to_string(),
            headline: risk.clone(),
            rationale: "基本面风险来自财报同比、盈利质量与公告披露的负向提示。".to_string(),
        })
        .collect()
}

// 2026-04-08 CST: 这里集中装配共振风险项，原因是负向共振和事件覆盖本来就不是技术面或执行面能表达清楚的风险；
// 目的：让 committee payload 能明确告诉上层哪些负向驱动正在拖累当前结论，而不是只混在摘要字符串里。
fn build_resonance_risk_items(
    analysis: &SecurityAnalysisResonanceResult,
) -> Vec<CommitteeRiskItem> {
    let mut items = analysis
        .resonance_context
        .top_negative_resonances
        .iter()
        .take(2)
        .map(|driver| CommitteeRiskItem {
            category: "resonance".to_string(),
            severity: "medium".to_string(),
            headline: format!("{} 存在负向共振或背离风险。", driver.display_name),
            rationale: format!(
                "{} 当前处于负向驱动列表，可能削弱综合结论的延续性。",
                driver.display_name
            ),
        })
        .collect::<Vec<_>>();

    if items.is_empty() {
        if let Some(event) = analysis.resonance_context.event_overrides.first() {
            items.push(CommitteeRiskItem {
                category: "resonance".to_string(),
                severity: "low".to_string(),
                headline: format!("事件覆盖项 `{}` 仍需持续跟踪。", event.title),
                rationale: "当前未出现显著负向共振时，事件覆盖项仍是需要持续监控的外生风险入口。"
                    .to_string(),
            });
        }
    }

    items
}

// 2026-04-02 CST: 这里把 briefing 默认投决建议正式收口成三种模式，原因是用户要求“默认就要投决会，并在报告里直接写出建议”，
// 目的：让个股报告、严格交易建议和已有持仓判断都能沿同一份 committee_payload 生成，而不是再由上层 Agent 手工改写。
#[allow(unreachable_code)]
fn build_committee_recommendations(
    committee_payload: &CommitteePayload,
    subject_profile: &CommitteeSubjectProfile,
) -> Result<CommitteeRecommendations, SecurityDecisionBriefingError> {
    // 2026-04-08 CST: 这里按 subject_profile 切换 report_focus 与场景文案，原因是 ETF 已进入同一投决主链，但说明口径不能再固定成个股。
    // 目的：先把 ETF/个股在 committee recommendation 层做最小语义分流，后续再继续扩展港股与海外资产。
    let (report_focus, standard_scenario, strict_scenario, advisory_scenario) =
        if subject_profile.asset_class == "etf" {
            (
                "etf_allocation_report",
                "ETF 常规配置与跟踪观察建议",
                "涉及新增仓位与仓位调整的 ETF 严格配置建议",
                "已有 ETF 持仓的跟踪与再平衡建议",
            )
        } else {
            (
                "stock_analysis_report",
                "个股分析报告默认投决会建议",
                "涉及金额与买卖动作的严格交易建议",
                "已有持仓判断与持仓处置建议",
            )
        };
    return Ok(CommitteeRecommendations {
        default_mode: "standard".to_string(),
        report_focus: report_focus.to_string(),
        standard: build_committee_recommendation_entry(
            committee_payload,
            "standard",
            standard_scenario,
        )?,
        strict: build_committee_recommendation_entry(committee_payload, "strict", strict_scenario)?,
        advisory: build_committee_recommendation_entry(
            committee_payload,
            "advisory",
            advisory_scenario,
        )?,
    });
    Ok(CommitteeRecommendations {
        default_mode: "standard".to_string(),
        report_focus: report_focus.to_string(),
        standard: build_committee_recommendation_entry(
            committee_payload,
            "standard",
            "个股分析报告默认投决会建议",
        )?,
        strict: build_committee_recommendation_entry(
            committee_payload,
            "strict",
            "涉及金额与买卖动作的严格交易建议",
        )?,
        advisory: build_committee_recommendation_entry(
            committee_payload,
            "advisory",
            "已有持仓判断与持仓处置建议",
        )?,
    })
}

// 2026-04-02 CST: 这里集中复用正式 vote Tool 生成 briefing 内嵌建议，原因是报告里的建议必须与独立 `security_committee_vote` 完全同口径，
// 目的：避免 report 层和 vote Tool 各自产出一份不同结论，重新引入用户刚刚明确反对的“双事实 / 双口径”问题。
fn build_committee_recommendation_entry(
    committee_payload: &CommitteePayload,
    committee_mode: &str,
    scenario: &str,
) -> Result<CommitteeRecommendationEntry, SecurityDecisionBriefingError> {
    let vote = security_committee_vote(&SecurityCommitteeVoteRequest {
        committee_payload: committee_payload.clone(),
        committee_mode: committee_mode.to_string(),
        meeting_id: Some(format!(
            "briefing-{}-{}",
            committee_payload.symbol, committee_mode
        )),
    })?;
    Ok(CommitteeRecommendationEntry {
        scenario: scenario.to_string(),
        vote,
    })
}

// 2026-04-02 CST: 这里把历史研究摘要正式接回 committee payload，原因是用户明确要求咨询和投决看到的信息必须一致，
// 不能一边说“有历史相似研究”，另一边 payload 还永远 unavailable；目的：让历史胜率、预期收益与回撤区间进入统一交付物。
fn build_historical_digest(
    symbol: &str,
    analysis_date: &str,
    subject_profile: &CommitteeSubjectProfile,
) -> CommitteeHistoricalDigest {
    // 2026-04-08 CST: 这里先让 ETF 历史研究显式 unavailable，原因是现有 study_key 仍是银行个股模板，硬套会产生误导。
    // 目的：先把 ETF 纳入正式主链，同时明确告诉上层“专用历史研究尚未接入”，避免伪精确。
    if subject_profile.asset_class == "etf" {
        return CommitteeHistoricalDigest {
            status: "unavailable".to_string(),
            historical_confidence: "unknown".to_string(),
            analog_sample_count: 0,
            analog_win_rate_10d: None,
            analog_loss_rate_10d: None,
            analog_flat_rate_10d: None,
            analog_avg_return_10d: None,
            analog_median_return_10d: None,
            analog_avg_win_return_10d: None,
            analog_avg_loss_return_10d: None,
            analog_payoff_ratio_10d: None,
            analog_expectancy_10d: None,
            expected_return_window: None,
            expected_drawdown_window: None,
            research_limitations: vec![
                "ETF 专用历史研究尚未接入，当前不复用个股 study_key。".to_string(),
            ],
        };
    }

    match signal_outcome_research_summary(&SignalOutcomeResearchSummaryRequest {
        symbol: symbol.to_string(),
        snapshot_date: Some(analysis_date.to_string()),
        study_key: "bank_resonance_core_technical_v1".to_string(),
    }) {
        Ok(summary) => CommitteeHistoricalDigest {
            status: summary.status,
            historical_confidence: summary.historical_confidence,
            analog_sample_count: summary.analog_sample_count,
            analog_win_rate_10d: summary.analog_win_rate_10d,
            analog_loss_rate_10d: summary.analog_loss_rate_10d,
            analog_flat_rate_10d: summary.analog_flat_rate_10d,
            analog_avg_return_10d: summary.analog_avg_return_10d,
            analog_median_return_10d: summary.analog_median_return_10d,
            analog_avg_win_return_10d: summary.analog_avg_win_return_10d,
            analog_avg_loss_return_10d: summary.analog_avg_loss_return_10d,
            analog_payoff_ratio_10d: summary.analog_payoff_ratio_10d,
            analog_expectancy_10d: summary.analog_expectancy_10d,
            expected_return_window: summary.expected_return_window,
            expected_drawdown_window: summary.expected_drawdown_window,
            research_limitations: summary.research_limitations,
        },
        Err(_) => CommitteeHistoricalDigest {
            status: "unavailable".to_string(),
            historical_confidence: "unknown".to_string(),
            analog_sample_count: 0,
            analog_win_rate_10d: None,
            analog_loss_rate_10d: None,
            analog_flat_rate_10d: None,
            analog_avg_return_10d: None,
            analog_median_return_10d: None,
            analog_avg_win_return_10d: None,
            analog_avg_loss_return_10d: None,
            analog_payoff_ratio_10d: None,
            analog_expectancy_10d: None,
            expected_return_window: None,
            expected_drawdown_window: None,
            research_limitations: vec![
                "历史研究摘要读取失败，当前按 unavailable 处理。".to_string(),
            ],
        },
    }
}

// 2026-04-02 CST: 这里先提供占位默认值函数，原因是请求合同在 assembler 和 dispatcher 接入前也需要可稳定反序列化，
// 目的：让合同层先独立成立，后续只做增量实现而不再回头拆字段默认规则。
fn default_lookback_days() -> usize {
    180
}

// 2026-04-02 CST: 这里把 factor lookback 的默认值先收口到 briefing 请求层，原因是 briefing 后续会统一协调 technical/fullstack/resonance 的观察窗口，
// 目的：避免调用方在正式接入前就必须显式传完整参数集合。
fn default_factor_lookback_days() -> usize {
    120
}

// 2026-04-02 CST: 这里把公告披露上限默认值也先钉在 briefing 请求层，原因是 briefing 未来要复用 fullstack 的信息面窗口而不是临时拼参数，
// 目的：让合同阶段就拥有稳定、可测试的默认行为。
fn default_disclosure_limit() -> usize {
    6
}

// 2026-04-08 CST: 这里补最小 subject_profile 推断，原因是当前 ETF 先只需要 A 股交易所代码前缀就能支撑主链分流。
// 目的：用最小规则先跑通 ETF 与个股的正式合同分流，后续再增量扩展港股、海外股票与海外 ETF。
fn infer_subject_profile(symbol: &str) -> CommitteeSubjectProfile {
    let normalized_symbol = symbol.trim().to_uppercase();
    let mut profile = CommitteeSubjectProfile::default();

    let is_etf = normalized_symbol
        .strip_suffix(".SZ")
        .map(|code| code.starts_with("15") || code.starts_with("16"))
        .unwrap_or(false)
        || normalized_symbol
            .strip_suffix(".SH")
            .map(|code| code.starts_with("51") || code.starts_with("56") || code.starts_with("58"))
            .unwrap_or(false);

    if is_etf {
        profile.asset_class = "etf".to_string();
        profile.market_scope = "china".to_string();
        profile.committee_focus = "fund_review".to_string();
    }

    profile
}

#[cfg(test)]
mod tests {
    use super::infer_subject_profile;

    #[test]
    fn infer_subject_profile_marks_a_share_etf_as_etf() {
        let profile = infer_subject_profile("159866.SZ");
        assert_eq!(profile.asset_class, "etf");
        assert_eq!(profile.committee_focus, "fund_review");
    }

    #[test]
    fn infer_subject_profile_keeps_bank_equity_as_equity() {
        let profile = infer_subject_profile("601998.SH");
        assert_eq!(profile.asset_class, "equity");
        assert_eq!(profile.committee_focus, "stock_review");
    }
}
