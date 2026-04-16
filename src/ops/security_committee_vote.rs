use std::collections::BTreeMap;
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_decision_briefing::{CommitteePayload, CommitteeRiskItem};
use crate::tools::contracts::{ToolRequest, ToolResponse};

const COMMITTEE_ENGINE_VERSION: &str = "seven_seat_committee_v3";
const INTERNAL_SEAT_TOOL_NAME: &str = "security_committee_member_agent";
const CHILD_PROCESS_ENV_KEY: &str = "EXCEL_SKILL_COMMITTEE_CHILD";

// 2026-04-02 CST: 这里定义正式投决会请求合同，原因是方案 B 明确要求 vote Tool 只消费统一 committee payload；
// 目的：把 committee_mode 与 meeting_id 收口在强类型请求里，避免 dispatcher/Skill 在外层拼装第二套事实。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCommitteeVoteRequest {
    pub committee_payload: CommitteePayload,
    #[serde(default = "default_committee_mode")]
    pub committee_mode: String,
    #[serde(default)]
    pub meeting_id: Option<String>,
}

// 2026-04-08 CST: 这里定义席位级 agent 请求，原因是七席委员会需要通过子进程逐席执行，而不是在父进程内直接拼 votes；
// 目的：让每位委员都消费同一份 payload，但保留独立 seat_role 与 meeting_id 上下文，形成可审计的内部执行合同。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCommitteeMemberAgentRequest {
    pub committee_payload: CommitteePayload,
    pub committee_mode: String,
    pub seat_role: String,
    #[serde(default)]
    pub meeting_id: Option<String>,
}

// 2026-04-02 CST: 这里定义单个委员投票结构，原因是用户要求完整方案而不是最小返回值；
// 目的：保留角色、票型、信心、理由、关注点、阻断项与条件，后续无论 CLI 还是 GUI 都能直接解释。
// 2026-04-08 CST: 这里追加独立执行证明字段，原因是七席委员会需要证明每席是独立运行、独立留痕，而不是父进程伪造。
// 目的：把 member_id、seat_kind、execution_mode、execution_instance_id、process_id、evidence_version 固化进正式合同，便于审计与复盘。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CommitteeMemberVote {
    pub role: String,
    pub member_id: String,
    pub seat_kind: String,
    pub execution_mode: String,
    pub execution_instance_id: String,
    pub process_id: u64,
    pub evidence_version: String,
    pub vote: String,
    pub confidence: String,
    pub rationale: String,
    pub focus_points: Vec<String>,
    pub blockers: Vec<String>,
    pub conditions: Vec<String>,
}

// 2026-04-02 CST: 这里定义投决会结构化结果，原因是测试已经把最终聚合字段钉死成正式合同；
// 目的：让 catalog/dispatcher/Skill 与后续 review 都围绕同一份输出结构演进，而不是继续返回松散 JSON。
// 2026-04-08 CST: 这里补七席委员会引擎字段，原因是用户需要明确证明当前走的是七席委员会而不是旧五席实现。
// 目的：把 committee_engine、席位数量与多数派信息一起写入正式结果，支撑“独立执行证明”和审计摘要。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCommitteeVoteResult {
    pub symbol: String,
    pub analysis_date: String,
    pub evidence_version: String,
    pub committee_engine: String,
    pub committee_mode: String,
    pub deliberation_seat_count: usize,
    pub risk_seat_count: usize,
    pub majority_vote: String,
    pub majority_count: usize,
    pub final_decision: String,
    pub final_action: String,
    pub final_confidence: String,
    pub approval_ratio: f64,
    pub quorum_met: bool,
    pub veto_triggered: bool,
    pub veto_role: Option<String>,
    pub votes: Vec<CommitteeMemberVote>,
    pub conditions: Vec<String>,
    pub key_disagreements: Vec<String>,
    pub warnings: Vec<String>,
    pub meeting_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommitteeMode {
    Standard,
    Strict,
    Advisory,
}

#[derive(Debug, Clone, Copy)]
struct CommitteeSeatProfile {
    role: &'static str,
    member_id: &'static str,
    seat_kind: &'static str,
    leaning: &'static str,
    emphasis: &'static str,
}

#[derive(Debug, Clone)]
struct SeatOpinion {
    vote: String,
    confidence: String,
    rationale: String,
    focus_points: Vec<String>,
    blockers: Vec<String>,
    conditions: Vec<String>,
}

#[derive(Debug, Clone)]
enum MemberExecutionStrategy {
    ChildProcess(PathBuf),
    InProcessFallback,
}

#[derive(Debug, Error)]
pub enum SecurityCommitteeVoteError {
    #[error("committee_mode `{0}` 不受支持，仅支持 standard/strict/advisory")]
    UnsupportedCommitteeMode(String),
    #[error("committee_payload.evidence_version 不能为空")]
    MissingEvidenceVersion,
    #[error("committee_payload.committee_schema_version `{0}` 不受支持")]
    UnsupportedCommitteeSchemaVersion(String),
    #[error("committee_payload 不完整: {0}")]
    IncompleteCommitteePayload(String),
    #[error("seat_role `{0}` 不受支持")]
    UnsupportedCommitteeSeat(String),
    #[error("committee seat child process failed: {0}")]
    ChildProcessFailed(String),
    #[error("committee seat child process protocol invalid: {0}")]
    ChildProcessProtocol(String),
    #[error("committee tool serialization failed: {0}")]
    Serialization(String),
}

pub fn security_committee_vote(
    request: &SecurityCommitteeVoteRequest,
) -> Result<SecurityCommitteeVoteResult, SecurityCommitteeVoteError> {
    let committee_mode = parse_committee_mode(&request.committee_mode)?;
    validate_committee_payload(&request.committee_payload)?;
    let warnings = build_vote_warnings(&request.committee_payload);
    let execution_strategy = resolve_member_execution_strategy();
    let votes = build_committee_votes(request, &execution_strategy)?;
    Ok(aggregate_committee_votes(
        &request.committee_payload,
        committee_mode,
        votes,
        warnings,
    ))
}

// 2026-04-08 CST: 这里暴露内部席位 agent，原因是七席委员会需要一个可被子进程直接调用的正式 seat-level 执行入口；
// 目的：确保每席都沿统一 payload -> vote 合同产出结果，而不是父进程在内存里直接模拟各委员意见。
pub fn security_committee_member_agent(
    request: &SecurityCommitteeMemberAgentRequest,
) -> Result<CommitteeMemberVote, SecurityCommitteeVoteError> {
    let committee_mode = parse_committee_mode(&request.committee_mode)?;
    validate_committee_payload(&request.committee_payload)?;
    let seat = resolve_seat_profile(&request.seat_role)?;
    let opinion = evaluate_seat_opinion(&request.committee_payload, committee_mode, seat);
    Ok(materialize_member_vote(
        &request.committee_payload,
        request.meeting_id.as_deref(),
        seat,
        opinion,
    ))
}

fn default_committee_mode() -> String {
    "standard".to_string()
}

// 2026-04-02 CST: 这里先把 mode 解析成内部枚举，原因是投决规则在 standard/strict/advisory 下差异明确；
// 目的：避免后续聚合逻辑继续散落字符串判断，降低规则扩展时的耦合与漏判风险。
fn parse_committee_mode(mode: &str) -> Result<CommitteeMode, SecurityCommitteeVoteError> {
    match mode.trim() {
        "standard" => Ok(CommitteeMode::Standard),
        "strict" => Ok(CommitteeMode::Strict),
        "advisory" => Ok(CommitteeMode::Advisory),
        other => Err(SecurityCommitteeVoteError::UnsupportedCommitteeMode(
            other.to_string(),
        )),
    }
}

// 2026-04-02 CST: 这里集中校验事实包完整性，原因是 vote Tool 不能在 payload 不完整时自行脑补或回溯；
// 目的：把所有“是否允许进入表决”的硬门槛收口到单点校验，确保 dispatcher 与上层 Skill 得到一致错误。
fn validate_committee_payload(
    payload: &CommitteePayload,
) -> Result<(), SecurityCommitteeVoteError> {
    if payload.evidence_version.trim().is_empty() {
        return Err(SecurityCommitteeVoteError::MissingEvidenceVersion);
    }
    if payload.committee_schema_version.trim() != "committee-payload:v1" {
        return Err(
            SecurityCommitteeVoteError::UnsupportedCommitteeSchemaVersion(
                payload.committee_schema_version.clone(),
            ),
        );
    }
    if payload.briefing_digest.trim().is_empty() {
        return Err(SecurityCommitteeVoteError::IncompleteCommitteePayload(
            "briefing_digest 不能为空".to_string(),
        ));
    }
    if payload.symbol.trim().is_empty() || payload.analysis_date.trim().is_empty() {
        return Err(SecurityCommitteeVoteError::IncompleteCommitteePayload(
            "symbol 与 analysis_date 不能为空".to_string(),
        ));
    }
    if payload.key_risks.is_empty() {
        return Err(SecurityCommitteeVoteError::IncompleteCommitteePayload(
            "key_risks 不能为空".to_string(),
        ));
    }
    // 2026-04-09 CST: Tighten the committee payload contract so malformed
    // risk_breakdown/category drift is rejected before any seat starts voting.
    validate_risk_breakdown_bucket("technical", &payload.risk_breakdown.technical)?;
    validate_risk_breakdown_bucket("fundamental", &payload.risk_breakdown.fundamental)?;
    validate_risk_breakdown_bucket("resonance", &payload.risk_breakdown.resonance)?;
    validate_risk_breakdown_bucket("execution", &payload.risk_breakdown.execution)?;
    let derived_key_risks = derive_committee_key_risks(payload);
    if payload.key_risks != derived_key_risks {
        return Err(SecurityCommitteeVoteError::IncompleteCommitteePayload(
            "key_risks 必须严格来自 risk_breakdown headline 派生".to_string(),
        ));
    }
    Ok(())
}

// 2026-04-09 CST: Keep risk_breakdown as the single source of truth so each
// bucket only carries its own category semantics.
fn validate_risk_breakdown_bucket(
    expected_category: &str,
    items: &[CommitteeRiskItem],
) -> Result<(), SecurityCommitteeVoteError> {
    for item in items {
        if item.category != expected_category {
            return Err(SecurityCommitteeVoteError::IncompleteCommitteePayload(
                format!(
                    "risk_breakdown.{expected_category} category mismatch: expected {expected_category}, got {}",
                    item.category
                ),
            ));
        }
    }
    Ok(())
}

// 2026-04-09 CST: Rebuild legacy key_risks from the structured contract to
// prevent manual summaries from drifting away from risk_breakdown.
fn derive_committee_key_risks(payload: &CommitteePayload) -> Vec<String> {
    let mut key_risks = Vec::new();
    for items in [
        &payload.risk_breakdown.technical,
        &payload.risk_breakdown.fundamental,
        &payload.risk_breakdown.resonance,
        &payload.risk_breakdown.execution,
    ] {
        if let Some(item) = items.first() {
            key_risks.push(item.headline.clone());
        }
    }
    key_risks
}

// 2026-04-02 CST: 这里统一生成 warning，原因是历史研究 unavailable 与 readiness 边界应被显式暴露而不是默认吞掉；
// 目的：让投决会结果即使继续输出结论，也能把证据缺口和边界条件一并带给上层。
fn build_vote_warnings(payload: &CommitteePayload) -> Vec<String> {
    let mut warnings = Vec::new();
    if payload.historical_digest.status != "available" {
        push_unique_text(
            &mut warnings,
            format!(
                "历史研究层当前为 {}，委员会将基于现有 briefing 事实包继续表决。",
                payload.historical_digest.status
            ),
        );
    }
    for limitation in &payload.historical_digest.research_limitations {
        push_unique_text(&mut warnings, limitation.clone());
    }
    if !payload.evidence_checks.fundamental_ready {
        push_unique_text(
            &mut warnings,
            "财报/基本面证据尚未完全就绪，基本面相关席位会自动偏保守。".to_string(),
        );
    }
    if !payload.evidence_checks.briefing_ready {
        push_unique_text(
            &mut warnings,
            "briefing 尚未标记为 ready，委员会结果只能视作无效草案。".to_string(),
        );
    }
    warnings
}

// 2026-04-08 CST: 这里统一构建七席 roster，原因是用户已经把投决制度升级为“6 名审议委员 + 1 名风控委员”；
// 目的：让每次同一份 payload 都通过同一组席位独立执行，确保投票轨迹稳定、可审计、可复盘。
fn committee_seat_profiles() -> &'static [CommitteeSeatProfile] {
    &[
        CommitteeSeatProfile {
            role: "chair",
            member_id: "committee-chair-001",
            seat_kind: "deliberation",
            leaning: "综合协调、强调结论一致性",
            emphasis: "综合事实包形成主席总结意见",
        },
        CommitteeSeatProfile {
            role: "fundamental_reviewer",
            member_id: "committee-fundamental-001",
            seat_kind: "deliberation",
            leaning: "偏重基本面但不忽视执行风险",
            emphasis: "优先复核财报、公告、证据就绪度与核心风险",
        },
        CommitteeSeatProfile {
            role: "technical_reviewer",
            member_id: "committee-technical-001",
            seat_kind: "deliberation",
            leaning: "偏重趋势与共振，但接受基本面约束",
            emphasis: "优先检查趋势延续性、共振强度和交易质量",
        },
        CommitteeSeatProfile {
            role: "event_reviewer",
            member_id: "committee-event-001",
            seat_kind: "deliberation",
            leaning: "偏重事件冲击与少数异议",
            emphasis: "优先评估事件覆盖、负向驱动与盘中信息扰动",
        },
        CommitteeSeatProfile {
            role: "valuation_reviewer",
            member_id: "committee-valuation-001",
            seat_kind: "deliberation",
            leaning: "偏重赔率、历史样本与收益回撤比",
            emphasis: "优先检查历史研究层、样本胜率与回撤边界",
        },
        CommitteeSeatProfile {
            role: "execution_reviewer",
            member_id: "committee-execution-001",
            seat_kind: "deliberation",
            leaning: "偏重交易落地与阈值清晰度",
            emphasis: "优先检查加减仓门槛、止损位与监控点是否可执行",
        },
        CommitteeSeatProfile {
            role: "risk_officer",
            member_id: "committee-risk-001",
            seat_kind: "risk_control",
            leaning: "偏重否决权、先看失败边界",
            emphasis: "优先识别 readiness 缺口、红线风险与执行失效位",
        },
    ]
}

fn resolve_seat_profile(
    seat_role: &str,
) -> Result<CommitteeSeatProfile, SecurityCommitteeVoteError> {
    committee_seat_profiles()
        .iter()
        .copied()
        .find(|seat| seat.role == seat_role)
        .ok_or_else(|| SecurityCommitteeVoteError::UnsupportedCommitteeSeat(seat_role.to_string()))
}

fn resolve_member_execution_strategy() -> MemberExecutionStrategy {
    resolve_committee_binary_path()
        .map(MemberExecutionStrategy::ChildProcess)
        .unwrap_or(MemberExecutionStrategy::InProcessFallback)
}

fn resolve_committee_binary_path() -> Option<PathBuf> {
    if let Ok(current_exe) = env::current_exe() {
        if is_excel_skill_binary(&current_exe) {
            return Some(current_exe);
        }
        if let Some(nearby_binary) = resolve_nearby_excel_skill_binary(&current_exe) {
            return Some(nearby_binary);
        }
    }

    env::var_os("CARGO_BIN_EXE_excel_skill")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
}

fn is_excel_skill_binary(path: &Path) -> bool {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.eq_ignore_ascii_case("excel_skill"))
        .unwrap_or(false)
}

// 2026-04-08 CST: 这里补测试/工作树场景下的二进制定位，原因是 integration test 的 current_exe 往往指向 `target/.../deps/<test>.exe`；
// 目的：优先从当前测试 harness 的邻近目录回推 `target/debug/excel_skill(.exe)`，让直接函数测试也能复用真实 CLI 子进程路径。
fn resolve_nearby_excel_skill_binary(current_exe: &Path) -> Option<PathBuf> {
    let file_name = if cfg!(windows) {
        "excel_skill.exe"
    } else {
        "excel_skill"
    };
    let current_parent = current_exe.parent()?;
    let direct_candidate = current_parent.join(file_name);
    if direct_candidate.is_file() {
        return Some(direct_candidate);
    }

    let parent_candidate = current_parent.parent()?.join(file_name);
    if parent_candidate.is_file() {
        return Some(parent_candidate);
    }

    None
}

fn build_committee_votes(
    request: &SecurityCommitteeVoteRequest,
    execution_strategy: &MemberExecutionStrategy,
) -> Result<Vec<CommitteeMemberVote>, SecurityCommitteeVoteError> {
    committee_seat_profiles()
        .iter()
        .map(|seat| match execution_strategy {
            MemberExecutionStrategy::ChildProcess(binary_path) => {
                run_child_process_member_vote(binary_path, request, *seat)
            }
            MemberExecutionStrategy::InProcessFallback => {
                run_in_process_member_vote(request, *seat)
            }
        })
        .collect()
}

fn run_in_process_member_vote(
    request: &SecurityCommitteeVoteRequest,
    seat: CommitteeSeatProfile,
) -> Result<CommitteeMemberVote, SecurityCommitteeVoteError> {
    let agent_request = SecurityCommitteeMemberAgentRequest {
        committee_payload: request.committee_payload.clone(),
        committee_mode: request.committee_mode.clone(),
        seat_role: seat.role.to_string(),
        meeting_id: request.meeting_id.clone(),
    };
    security_committee_member_agent(&agent_request)
}

fn run_child_process_member_vote(
    binary_path: &Path,
    request: &SecurityCommitteeVoteRequest,
    seat: CommitteeSeatProfile,
) -> Result<CommitteeMemberVote, SecurityCommitteeVoteError> {
    let agent_request = SecurityCommitteeMemberAgentRequest {
        committee_payload: request.committee_payload.clone(),
        committee_mode: request.committee_mode.clone(),
        seat_role: seat.role.to_string(),
        meeting_id: request.meeting_id.clone(),
    };
    let tool_request = ToolRequest {
        tool: INTERNAL_SEAT_TOOL_NAME.to_string(),
        args: serde_json::to_value(&agent_request)
            .map_err(|error| SecurityCommitteeVoteError::Serialization(error.to_string()))?,
    };
    let request_payload = serde_json::to_vec(&tool_request)
        .map_err(|error| SecurityCommitteeVoteError::Serialization(error.to_string()))?;
    let mut child = Command::new(binary_path)
        .env(CHILD_PROCESS_ENV_KEY, "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| SecurityCommitteeVoteError::ChildProcessFailed(error.to_string()))?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(&request_payload)
            .map_err(|error| SecurityCommitteeVoteError::ChildProcessFailed(error.to_string()))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|error| SecurityCommitteeVoteError::ChildProcessFailed(error.to_string()))?;
    if !output.status.success() {
        let stderr_text = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout_text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Err(SecurityCommitteeVoteError::ChildProcessFailed(format!(
            "seat={} status={:?} stderr={} stdout={}",
            seat.role, output.status, stderr_text, stdout_text
        )));
    }

    let response: ToolResponse = serde_json::from_slice(&output.stdout).map_err(|error| {
        SecurityCommitteeVoteError::ChildProcessProtocol(format!(
            "seat={} parse tool response failed: {error}",
            seat.role
        ))
    })?;
    if response.status != "ok" {
        return Err(SecurityCommitteeVoteError::ChildProcessProtocol(format!(
            "seat={} returned status={} error={}",
            seat.role,
            response.status,
            response
                .error
                .unwrap_or_else(|| "unknown child error".to_string())
        )));
    }

    serde_json::from_value::<CommitteeMemberVote>(response.data).map_err(|error| {
        SecurityCommitteeVoteError::ChildProcessProtocol(format!(
            "seat={} parse vote payload failed: {error}",
            seat.role
        ))
    })
}

fn evaluate_seat_opinion(
    payload: &CommitteePayload,
    committee_mode: CommitteeMode,
    seat: CommitteeSeatProfile,
) -> SeatOpinion {
    match seat.role {
        "chair" => build_chair_opinion(payload),
        "fundamental_reviewer" => build_fundamental_opinion(payload),
        "technical_reviewer" => build_technical_opinion(payload),
        "event_reviewer" => build_event_opinion(payload, committee_mode),
        "valuation_reviewer" => build_valuation_opinion(payload, committee_mode),
        "execution_reviewer" => build_execution_opinion(payload, committee_mode),
        "risk_officer" => build_risk_opinion(payload, committee_mode),
        _ => SeatOpinion {
            vote: "defer".to_string(),
            confidence: "low".to_string(),
            rationale: format!("席位 {} 未配置分析逻辑，按保守原则自动 defer。", seat.role),
            focus_points: vec![seat.leaning.to_string(), seat.emphasis.to_string()],
            blockers: vec!["席位分析规则缺失".to_string()],
            conditions: Vec::new(),
        },
    }
}

fn materialize_member_vote(
    payload: &CommitteePayload,
    meeting_id: Option<&str>,
    seat: CommitteeSeatProfile,
    opinion: SeatOpinion,
) -> CommitteeMemberVote {
    let process_id = std::process::id() as u64;
    CommitteeMemberVote {
        role: seat.role.to_string(),
        member_id: seat.member_id.to_string(),
        seat_kind: seat.seat_kind.to_string(),
        execution_mode: resolve_execution_mode_label().to_string(),
        execution_instance_id: build_execution_instance_id(meeting_id, seat, process_id),
        process_id,
        evidence_version: payload.evidence_version.clone(),
        vote: opinion.vote,
        confidence: opinion.confidence,
        rationale: opinion.rationale,
        focus_points: normalize_focus_points(opinion.focus_points),
        blockers: opinion.blockers,
        conditions: opinion.conditions,
    }
}

fn resolve_execution_mode_label() -> &'static str {
    match env::var(CHILD_PROCESS_ENV_KEY).ok().as_deref() {
        Some("1") => "child_process",
        _ => "in_process_fallback",
    }
}

fn build_execution_instance_id(
    meeting_id: Option<&str>,
    seat: CommitteeSeatProfile,
    process_id: u64,
) -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!(
        "{}:{}:{}:{}",
        meeting_id.unwrap_or("committee-meeting"),
        seat.role,
        process_id,
        nonce
    )
}

fn build_chair_opinion(payload: &CommitteePayload) -> SeatOpinion {
    let mut conditions = standard_conditions(payload);
    let mut blockers = Vec::new();
    if !payload.evidence_checks.briefing_ready {
        blockers
            .push("briefing 尚未通过 ready 校验，主席席位拒绝将草案升格为正式决议。".to_string());
    }
    if payload.recommendation_digest.action_bias == "reduce_or_exit" {
        push_unique_text(
            &mut conditions,
            "综合结论已经转向 reduce_or_exit，需要重新确认是否还存在继续持有或加仓前提。"
                .to_string(),
        );
    }
    let vote = if !blockers.is_empty() {
        "defer"
    } else if conditions.is_empty() {
        "approve"
    } else {
        "conditional_approve"
    };

    SeatOpinion {
        vote: vote.to_string(),
        confidence: normalize_confidence(&payload.confidence).to_string(),
        rationale: format!(
            "主席席位综合 briefing 摘要、推荐动作与风险边界后认为当前建议为 {}，但仍需尊重条件约束。",
            payload.recommended_action
        ),
        focus_points: vec![
            payload.recommendation_digest.summary.clone(),
            payload.briefing_digest.clone(),
            format!("key_risks={}", payload.key_risks.len()),
        ],
        blockers,
        conditions,
    }
}

fn build_fundamental_opinion(payload: &CommitteePayload) -> SeatOpinion {
    let mut conditions = standard_conditions(payload);
    let mut blockers = Vec::new();
    let is_fund_review = payload.subject_profile.asset_class == "etf"
        || payload.subject_profile.committee_focus == "fund_review";
    let has_financial_risk = payload.key_risks.iter().any(|risk| {
        risk.contains("财报")
            || risk.contains("同比")
            || risk.contains("利润")
            || risk.contains("公告")
    });
    let vote = if is_fund_review {
        if !payload.evidence_checks.fundamental_ready {
            push_unique_text(
                &mut conditions,
                "ETF fund_review 资料尚未补齐，需继续核对跟踪误差、底层指数结构、流动性与申赎机制。"
                    .to_string(),
            );
        }
        if payload.recommendation_digest.action_bias == "reduce_or_exit" {
            push_unique_text(
                &mut conditions,
                "ETF 当前综合建议仍偏保守，基金审议席位要求等待新的跟踪质量确认信号。".to_string(),
            );
            "defer"
        } else {
            "conditional_approve"
        }
    } else if !payload.evidence_checks.fundamental_ready {
        blockers.push("基本面证据未就绪，需先补齐财报与公告快照。".to_string());
        "defer"
    } else if has_financial_risk {
        push_unique_text(
            &mut conditions,
            "财报与公告中的关键同比口径尚需再核验，放大仓位前必须完成复核。".to_string(),
        );
        "conditional_approve"
    } else if payload.recommendation_digest.action_bias == "reduce_or_exit" {
        push_unique_text(
            &mut conditions,
            "综合建议已偏保守，基本面席位要求等待新的经营确认信号。".to_string(),
        );
        "defer"
    } else {
        "approve"
    };

    SeatOpinion {
        vote: vote.to_string(),
        confidence: normalize_confidence(&payload.recommendation_digest.confidence).to_string(),
        rationale: if is_fund_review {
            "ETF 基本面/基金审议席位会把跟踪误差、底层指数结构、流动性与申赎机制作为 fund-review 语义，不会因为缺少个股财报就直接 defer。"
                .to_string()
        } else {
            "基本面席位会同时看推荐摘要、关键风险与历史研究可用性，不会只盯单一财报指标。"
                .to_string()
        },
        focus_points: vec![
            payload.recommendation_digest.summary.clone(),
            payload
                .key_risks
                .first()
                .cloned()
                .unwrap_or_else(|| "暂无关键风险".to_string()),
            format!("historical_status={}", payload.historical_digest.status),
        ],
        blockers,
        conditions,
    }
}

fn build_technical_opinion(payload: &CommitteePayload) -> SeatOpinion {
    let mut conditions = standard_conditions(payload);
    let mut blockers = Vec::new();
    let resonance_score = payload.resonance_digest.resonance_score;
    let vote = if !payload.evidence_checks.technical_ready {
        blockers.push("技术面证据未就绪，趋势延续性无法确认。".to_string());
        "defer"
    } else if payload.resonance_digest.action_bias == "reduce_or_exit" || resonance_score <= 0.42 {
        blockers.push("技术/共振层当前不支持继续执行积极动作。".to_string());
        "reject"
    } else if resonance_score >= 0.70 {
        if payload.recommendation_digest.action_bias == "hold_and_confirm" {
            push_unique_text(
                &mut conditions,
                "技术面要求等待放量确认后再执行 add_on_strength。".to_string(),
            );
            "conditional_approve"
        } else {
            "approve"
        }
    } else {
        push_unique_text(
            &mut conditions,
            "趋势确认仍不足，技术席位要求等待下一轮信号增强。".to_string(),
        );
        "defer"
    };

    SeatOpinion {
        vote: vote.to_string(),
        confidence: normalize_confidence(&payload.confidence).to_string(),
        rationale: format!(
            "技术席位综合共振分数 {:.2}、动作偏向 {} 与执行阈值后给出表态。",
            resonance_score, payload.resonance_digest.action_bias
        ),
        focus_points: vec![
            payload
                .resonance_digest
                .top_positive_driver_names
                .first()
                .cloned()
                .unwrap_or_else(|| "缺少正向驱动".to_string()),
            payload
                .resonance_digest
                .top_negative_driver_names
                .first()
                .cloned()
                .unwrap_or_else(|| "暂无明显负向驱动".to_string()),
            format!(
                "watch_points={}",
                payload.execution_digest.watch_points.len()
            ),
        ],
        blockers,
        conditions,
    }
}

fn build_event_opinion(payload: &CommitteePayload, committee_mode: CommitteeMode) -> SeatOpinion {
    let mut conditions = standard_conditions(payload);
    let mut blockers = Vec::new();
    let event_count = payload.resonance_digest.event_override_titles.len();
    let objection_count = payload.minority_objection_points.len();
    let vote = if event_count == 0 && objection_count == 0 {
        if payload.key_risks.len() <= 2 {
            "approve"
        } else {
            push_unique_text(
                &mut conditions,
                "事件面当前平静，但关键风险尚未完全消化，需持续跟踪盘中扰动。".to_string(),
            );
            "conditional_approve"
        }
    } else if matches!(committee_mode, CommitteeMode::Strict) && event_count >= 2 {
        blockers
            .push("严格模式下事件覆盖项过多，事件席位要求延后到新公告确认后再表决。".to_string());
        "defer"
    } else {
        push_unique_text(
            &mut conditions,
            "事件覆盖与少数异议点需要纳入盘中复核清单，若出现反转应立即重新开会。".to_string(),
        );
        "conditional_approve"
    };

    SeatOpinion {
        vote: vote.to_string(),
        confidence: if event_count > 0 { "medium" } else { "low" }.to_string(),
        rationale: "事件席位会看同一份 payload 中的事件覆盖、负向驱动、少数异议与执行 watch points，再判断信息扰动是否足以改写原结论。"
            .to_string(),
        focus_points: vec![
            payload
                .resonance_digest
                .event_override_titles
                .first()
                .cloned()
                .unwrap_or_else(|| "暂无事件覆盖项".to_string()),
            payload
                .minority_objection_points
                .first()
                .cloned()
                .unwrap_or_else(|| "暂无少数异议".to_string()),
            format!(
                "negative_drivers={}",
                payload.resonance_digest.top_negative_driver_names.len()
            ),
        ],
        blockers,
        conditions,
    }
}

fn build_valuation_opinion(
    payload: &CommitteePayload,
    committee_mode: CommitteeMode,
) -> SeatOpinion {
    let mut conditions = standard_conditions(payload);
    let mut blockers = Vec::new();
    let analog_win_rate = payload.historical_digest.analog_win_rate_10d.unwrap_or(0.0);
    let vote = if payload.historical_digest.status != "available" {
        push_unique_text(
            &mut conditions,
            "历史研究层未 fully available，估值/赔率席位只能给出附条件意见。".to_string(),
        );
        if matches!(committee_mode, CommitteeMode::Strict) {
            "defer"
        } else {
            "conditional_approve"
        }
    } else if analog_win_rate >= 0.55 {
        "approve"
    } else if analog_win_rate > 0.0 {
        push_unique_text(
            &mut conditions,
            "历史样本胜率不高，估值席位要求缩小仓位或降低盈利预期。".to_string(),
        );
        "conditional_approve"
    } else {
        blockers.push(
            "历史研究层可用但未形成有效胜率样本，估值席位拒绝把赔率结论说得过满。".to_string(),
        );
        "defer"
    };

    SeatOpinion {
        vote: vote.to_string(),
        confidence: if analog_win_rate >= 0.55 {
            "medium".to_string()
        } else {
            "low".to_string()
        },
        rationale: "估值/赔率席位同样看同一份事实包，但会把历史研究层、预期收益窗口与预期回撤窗口放在更高权重。"
            .to_string(),
        focus_points: vec![
            format!("historical_status={}", payload.historical_digest.status),
            format!("analog_win_rate_10d={analog_win_rate:.2}"),
            payload
                .historical_digest
                .expected_return_window
                .clone()
                .unwrap_or_else(|| "缺少 expected_return_window".to_string()),
        ],
        blockers,
        conditions,
    }
}

fn build_execution_opinion(
    payload: &CommitteePayload,
    committee_mode: CommitteeMode,
) -> SeatOpinion {
    let mut conditions = Vec::new();
    let mut blockers = Vec::new();
    let execution_invalid = has_execution_red_flag(payload);
    let watch_points = payload.execution_digest.watch_points.len();
    let vote = if !payload.evidence_checks.execution_ready || execution_invalid {
        blockers.push("执行阈值存在红线冲突，当前无法形成可执行交易计划。".to_string());
        "reject"
    } else if matches!(committee_mode, CommitteeMode::Strict) && watch_points < 2 {
        blockers.push("严格模式下执行观察点不足，交易脚本不可直接落地。".to_string());
        "defer"
    } else if watch_points == 0 {
        push_unique_text(
            &mut conditions,
            "补齐 execution watch points 后再执行。".to_string(),
        );
        "conditional_approve"
    } else if payload.historical_digest.status != "available" {
        push_unique_text(
            &mut conditions,
            "先按小仓位试单，待历史研究层补齐后再决定是否扩大执行。".to_string(),
        );
        "conditional_approve"
    } else {
        "approve"
    };

    SeatOpinion {
        vote: vote.to_string(),
        confidence: "high".to_string(),
        rationale: "执行席位不会只看价格阈值，也会同时检查事件风险、历史样本可用性和推荐动作是否能被实际执行。"
            .to_string(),
        focus_points: vec![
            format!("add_trigger={:.2}", payload.execution_digest.add_trigger_price),
            format!("reduce_trigger={:.2}", payload.execution_digest.reduce_trigger_price),
            format!("stop_loss={:.2}", payload.execution_digest.stop_loss_price),
        ],
        blockers,
        conditions,
    }
}

fn build_risk_opinion(payload: &CommitteePayload, committee_mode: CommitteeMode) -> SeatOpinion {
    let mut conditions = standard_conditions(payload);
    let mut blockers = Vec::new();
    let execution_invalid = has_execution_red_flag(payload);
    let vote = if !payload.evidence_checks.briefing_ready || execution_invalid {
        blockers.push("事实包 readiness 或执行阈值存在硬缺口，风控席位直接否决。".to_string());
        "reject"
    } else if matches!(committee_mode, CommitteeMode::Strict)
        && payload.historical_digest.status != "available"
    {
        blockers.push("严格模式要求历史研究层 available，当前条件不足。".to_string());
        "defer"
    } else if payload.key_risks.len() >= 4
        || payload.recommendation_digest.action_bias == "reduce_or_exit"
    {
        blockers.push("关键风险过多或综合建议已转向 reduce_or_exit，触发风控否决。".to_string());
        "reject"
    } else if conditions.is_empty() {
        "approve"
    } else {
        push_unique_text(
            &mut conditions,
            "风控席位要求严格遵守执行阈值，不得在条件未满足前擅自扩大仓位。".to_string(),
        );
        "conditional_approve"
    };

    SeatOpinion {
        vote: vote.to_string(),
        confidence: "high".to_string(),
        rationale: "风控席位虽然更看重失败边界，但同样会基于同一份 payload 综合核查风险、执行、历史研究与推荐动作。"
            .to_string(),
        focus_points: vec![
            format!("key_risks={}", payload.key_risks.len()),
            format!("historical_status={}", payload.historical_digest.status),
            format!("briefing_ready={}", payload.evidence_checks.briefing_ready),
        ],
        blockers,
        conditions,
    }
}

fn normalize_focus_points(points: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for point in points {
        push_unique_text(&mut normalized, point);
        if normalized.len() >= 4 {
            break;
        }
    }
    normalized
}

// 2026-04-02 CST: 这里集中做表决聚合，原因是标准/严格/咨询模式下的 quorum、veto 与最终结论必须保持单点真值；
// 目的：让测试、CLI 与 Skill 都以同一套规则产出 final_decision / approval_ratio / warnings 等正式字段。
fn aggregate_committee_votes(
    payload: &CommitteePayload,
    committee_mode: CommitteeMode,
    votes: Vec<CommitteeMemberVote>,
    warnings: Vec<String>,
) -> SecurityCommitteeVoteResult {
    let total_votes = votes.len().max(1) as f64;
    let approval_votes = votes
        .iter()
        .filter(|vote| matches!(vote.vote.as_str(), "approve" | "conditional_approve"))
        .count();
    let conditional_votes = votes
        .iter()
        .filter(|vote| vote.vote == "conditional_approve")
        .count();
    let reject_votes = votes.iter().filter(|vote| vote.vote == "reject").count();
    let quorum_met = votes.len() >= committee_seat_profiles().len();
    let veto_role = resolve_veto_role(&votes, committee_mode);
    let veto_triggered = veto_role.is_some();
    let approval_ratio = round_ratio(approval_votes as f64 / total_votes);
    let conditions = collect_conditions(&votes);
    let key_disagreements = collect_key_disagreements(payload, &votes, &warnings);
    let (majority_vote, majority_count) = resolve_majority_vote(&votes);
    let final_decision = determine_final_decision(
        committee_mode,
        quorum_met,
        veto_triggered,
        approval_votes,
        conditional_votes,
        reject_votes,
    );
    let final_action = determine_final_action(payload, &final_decision);
    let final_confidence = determine_final_confidence(payload, &final_decision, &warnings);
    let meeting_digest = build_meeting_digest(
        payload,
        committee_mode,
        &final_decision,
        approval_ratio,
        veto_triggered,
        &majority_vote,
        majority_count,
    );

    SecurityCommitteeVoteResult {
        symbol: payload.symbol.clone(),
        analysis_date: payload.analysis_date.clone(),
        evidence_version: payload.evidence_version.clone(),
        committee_engine: COMMITTEE_ENGINE_VERSION.to_string(),
        committee_mode: committee_mode_label(committee_mode).to_string(),
        deliberation_seat_count: 6,
        risk_seat_count: 1,
        majority_vote,
        majority_count,
        final_decision,
        final_action,
        final_confidence,
        approval_ratio,
        quorum_met,
        veto_triggered,
        veto_role,
        votes,
        conditions,
        key_disagreements,
        warnings,
        meeting_digest,
    }
}

fn resolve_veto_role(
    votes: &[CommitteeMemberVote],
    committee_mode: CommitteeMode,
) -> Option<String> {
    match committee_mode {
        CommitteeMode::Advisory => None,
        CommitteeMode::Standard | CommitteeMode::Strict => votes
            .iter()
            .find(|vote| vote.role == "risk_officer" && vote.vote == "reject")
            .map(|vote| vote.role.clone()),
    }
}

fn resolve_majority_vote(votes: &[CommitteeMemberVote]) -> (String, usize) {
    let mut vote_counter = BTreeMap::new();
    for vote in votes {
        *vote_counter.entry(vote.vote.clone()).or_insert(0_usize) += 1;
    }

    let mut winning_vote = "split".to_string();
    let mut winning_count = 0_usize;
    let mut tie = false;
    for (vote, count) in vote_counter {
        if count > winning_count {
            winning_vote = vote;
            winning_count = count;
            tie = false;
        } else if count == winning_count {
            tie = true;
        }
    }

    if tie {
        ("split".to_string(), winning_count)
    } else {
        (winning_vote, winning_count)
    }
}

fn determine_final_decision(
    committee_mode: CommitteeMode,
    quorum_met: bool,
    veto_triggered: bool,
    approval_votes: usize,
    conditional_votes: usize,
    reject_votes: usize,
) -> String {
    if !quorum_met {
        return "deferred".to_string();
    }
    match committee_mode {
        CommitteeMode::Standard => {
            if veto_triggered {
                "rejected".to_string()
            } else if approval_votes >= 4 {
                if conditional_votes == 0 {
                    "approved".to_string()
                } else {
                    "approved_with_conditions".to_string()
                }
            } else if reject_votes >= 4 {
                "rejected".to_string()
            } else {
                "deferred".to_string()
            }
        }
        CommitteeMode::Strict => {
            if veto_triggered {
                "rejected".to_string()
            } else if approval_votes >= 5 {
                if conditional_votes == 0 {
                    "approved".to_string()
                } else {
                    "approved_with_conditions".to_string()
                }
            } else if reject_votes >= 3 {
                "rejected".to_string()
            } else {
                "deferred".to_string()
            }
        }
        CommitteeMode::Advisory => {
            if approval_votes > reject_votes {
                if conditional_votes > 0 {
                    "approved_with_conditions".to_string()
                } else {
                    "approved".to_string()
                }
            } else if reject_votes > approval_votes {
                "rejected".to_string()
            } else {
                "deferred".to_string()
            }
        }
    }
}

fn determine_final_action(payload: &CommitteePayload, final_decision: &str) -> String {
    match final_decision {
        "approved" => payload.recommended_action.clone(),
        "approved_with_conditions" => format!("{}_with_conditions", payload.recommended_action),
        "deferred" => "wait_for_next_review".to_string(),
        "rejected" => "do_not_execute".to_string(),
        _ => "wait_for_next_review".to_string(),
    }
}

fn determine_final_confidence(
    payload: &CommitteePayload,
    final_decision: &str,
    warnings: &[String],
) -> String {
    match final_decision {
        "approved" if warnings.is_empty() => normalize_confidence(&payload.confidence).to_string(),
        "approved" | "approved_with_conditions" => {
            downgrade_confidence(&payload.recommendation_digest.confidence).to_string()
        }
        "deferred" => "low".to_string(),
        "rejected" => "high".to_string(),
        _ => "low".to_string(),
    }
}

fn build_meeting_digest(
    payload: &CommitteePayload,
    committee_mode: CommitteeMode,
    final_decision: &str,
    approval_ratio: f64,
    veto_triggered: bool,
    majority_vote: &str,
    majority_count: usize,
) -> String {
    format!(
        "{} 在 {} 模式下形成 {}，committee_engine={}，approval_ratio={:.2}，majority={}({})，veto_triggered={}，建议动作为 {}。",
        payload.symbol,
        committee_mode_label(committee_mode),
        final_decision,
        COMMITTEE_ENGINE_VERSION,
        approval_ratio,
        majority_vote,
        majority_count,
        veto_triggered,
        payload.recommended_action
    )
}

fn collect_conditions(votes: &[CommitteeMemberVote]) -> Vec<String> {
    let mut conditions = Vec::new();
    for vote in votes {
        for condition in &vote.conditions {
            push_unique_text(&mut conditions, condition.clone());
        }
    }
    conditions
}

fn collect_key_disagreements(
    payload: &CommitteePayload,
    votes: &[CommitteeMemberVote],
    warnings: &[String],
) -> Vec<String> {
    let mut disagreements = Vec::new();
    for point in &payload.minority_objection_points {
        push_unique_text(&mut disagreements, point.clone());
    }
    for vote in votes {
        if matches!(
            vote.vote.as_str(),
            "reject" | "defer" | "conditional_approve"
        ) {
            push_unique_text(
                &mut disagreements,
                format!("{}: {}", vote.role, vote.rationale),
            );
        }
        for blocker in &vote.blockers {
            push_unique_text(
                &mut disagreements,
                format!("{} blocker: {}", vote.role, blocker),
            );
        }
    }
    if disagreements.is_empty() {
        for warning in warnings {
            push_unique_text(&mut disagreements, warning.clone());
        }
    }
    disagreements
}

fn standard_conditions(payload: &CommitteePayload) -> Vec<String> {
    let mut conditions = Vec::new();
    if payload.historical_digest.status != "available" {
        push_unique_text(
            &mut conditions,
            "历史研究层未接入前，应按较小仓位与更短复核节奏执行。".to_string(),
        );
    }
    if !payload.key_risks.is_empty() {
        push_unique_text(
            &mut conditions,
            "严格遵守 execution_digest 中的加仓、减仓与止损阈值。".to_string(),
        );
    }
    if !payload.resonance_digest.event_override_titles.is_empty() {
        push_unique_text(
            &mut conditions,
            "若事件覆盖项发生反转，需要立即重新召开投决会。".to_string(),
        );
    }
    conditions
}

fn has_execution_red_flag(payload: &CommitteePayload) -> bool {
    payload.execution_digest.add_trigger_price <= 0.0
        || payload.execution_digest.stop_loss_price <= 0.0
        || payload.execution_digest.add_trigger_price <= payload.execution_digest.stop_loss_price
        || payload.execution_digest.invalidation_price >= payload.execution_digest.add_trigger_price
}

fn normalize_confidence(confidence: &str) -> &str {
    match confidence {
        "high" | "medium" | "low" => confidence,
        _ => "medium",
    }
}

fn downgrade_confidence(confidence: &str) -> &str {
    match normalize_confidence(confidence) {
        "high" => "medium",
        "medium" => "low",
        _ => "low",
    }
}

fn committee_mode_label(mode: CommitteeMode) -> &'static str {
    match mode {
        CommitteeMode::Standard => "standard",
        CommitteeMode::Strict => "strict",
        CommitteeMode::Advisory => "advisory",
    }
}

fn round_ratio(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn push_unique_text(target: &mut Vec<String>, candidate: String) {
    if candidate.trim().is_empty() {
        return;
    }
    if !target.iter().any(|existing| existing == &candidate) {
        target.push(candidate);
    }
}
