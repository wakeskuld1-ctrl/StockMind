// 2026-04-16 CST: LEGACY_COMMITTEE_ADAPTER_FROZEN
// Reason: the user explicitly required the old committee chain to stop
// accumulating new business logic during the ongoing refactor.
// Purpose: this file is now a frozen legacy compatibility zone until the
// downstream consumers are migrated to the formal committee mainline.
// Rule:
// - DO NOT add new voting rules here.
// - DO NOT add new seat-agent protocol fields here.
// - DO NOT add new child-process behavior here.
// - DO NOT add new downstream business logic here.
// - Only explicit compatibility projection or approved retirement work is allowed.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Write;
use std::process::{Command, Stdio};
use thiserror::Error;

use crate::ops::stock::security_decision_card::{
    SecurityCommitteeMemberOpinion, SecurityCommitteeRiskVeto, SecurityCommitteeVoteTally,
    SecurityDecisionCard, SecurityDecisionThesis, build_security_decision_card,
    derive_exposure_side_from_action,
};
use crate::ops::stock::security_decision_evidence_bundle::{
    SecurityDecisionEvidenceBundleError, SecurityDecisionEvidenceBundleRequest,
    SecurityDecisionEvidenceBundleResult, SecurityExternalProxyInputs,
    security_decision_evidence_bundle,
};
use crate::ops::stock::security_risk_gates::{
    SecurityDecisionRiskProfile, SecurityRiskGateResult, evaluate_security_risk_gates,
};
use crate::tools::contracts::{ToolRequest, ToolResponse};

// 2026-04-01 CST: 这里定义证券投决会请求，原因是用户输入除了标的和环境代理，还会携带止损与目标收益约束；
// 目的：把“研究请求”和“裁决参数”收进同一个 Tool 合同，支持单次调用完成投决流程。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityDecisionCommitteeRequest {
    pub symbol: String,
    #[serde(default)]
    pub market_symbol: Option<String>,
    #[serde(default)]
    pub sector_symbol: Option<String>,
    #[serde(default)]
    pub market_profile: Option<String>,
    #[serde(default)]
    pub sector_profile: Option<String>,
    #[serde(default)]
    pub as_of_date: Option<String>,
    #[serde(default = "default_lookback_days")]
    pub lookback_days: usize,
    #[serde(default = "default_disclosure_limit")]
    pub disclosure_limit: usize,
    #[serde(default = "default_stop_loss_pct")]
    pub stop_loss_pct: f64,
    #[serde(default = "default_target_return_pct")]
    pub target_return_pct: f64,
    #[serde(default = "default_min_risk_reward_ratio")]
    pub min_risk_reward_ratio: f64,
    #[serde(default)]
    pub external_proxy_inputs: Option<SecurityExternalProxyInputs>,
}

// 2026-04-01 CST: 这里定义证券投决会结果，原因是顶层 Tool 需要同时返回证据、正反方、闸门和投决卡；
// 目的：让一次请求能拿到完整投决闭环，而不是外层再手工拼装多个中间结果。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityDecisionCommitteeResult {
    pub committee_engine: String,
    // 2026-04-09 CST: 这里新增正式 committee_session_ref，原因是 Task 1 要让主席线引用投委会线时不再手工拼接临时标识；
    // 目的：给 chair_resolution / package / verify 一个稳定的投委会正式引用锚点。
    pub committee_session_ref: String,
    pub symbol: String,
    pub analysis_date: String,
    pub market_profile: Option<String>,
    pub sector_profile: Option<String>,
    pub evidence_bundle: SecurityDecisionEvidenceBundleResult,
    pub member_opinions: Vec<SecurityCommitteeMemberOpinion>,
    pub vote_tally: SecurityCommitteeVoteTally,
    pub risk_veto: SecurityCommitteeRiskVeto,
    pub bull_case: SecurityDecisionThesis,
    pub bear_case: SecurityDecisionThesis,
    pub risk_gates: Vec<SecurityRiskGateResult>,
    pub decision_card: SecurityDecisionCard,
}

// 2026-04-01 CST: 这里单独定义投决会错误边界，原因是顶层 Tool 需要用自己的语言描述“证据冻结失败”；
// 目的：给 dispatcher 和 Skill 一个稳定错误口径，不泄露太多内部实现细节。
#[derive(Debug, Error)]
pub enum SecurityDecisionCommitteeError {
    #[error("证券投决会证据准备失败: {0}")]
    Evidence(#[from] SecurityDecisionEvidenceBundleError),
    #[error("committee child process execution failed: {0}")]
    AgentExecution(String),
}

// 2026-04-01 CST: 这里实现证券投决会总入口，原因是我们要把研究、正反方、闸门和裁决收进一个可复用的 Tool；
// 目的：让同一对话能够通过“单次冻结证据 + 双立场独立生成 + 风控闸门裁决”拿到结构化投决结果。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCommitteeMemberAgentRequest {
    pub member_id: String,
    pub market_context: String,
    pub evidence_bundle: SecurityDecisionEvidenceBundleResult,
}

pub fn security_decision_committee(
    request: &SecurityDecisionCommitteeRequest,
) -> Result<SecurityDecisionCommitteeResult, SecurityDecisionCommitteeError> {
    let evidence_request = SecurityDecisionEvidenceBundleRequest {
        symbol: request.symbol.clone(),
        market_symbol: request.market_symbol.clone(),
        sector_symbol: request.sector_symbol.clone(),
        market_profile: request.market_profile.clone(),
        sector_profile: request.sector_profile.clone(),
        as_of_date: request.as_of_date.clone(),
        underlying_symbol: None,
        fx_symbol: None,
        lookback_days: request.lookback_days,
        disclosure_limit: request.disclosure_limit,
        external_proxy_inputs: request.external_proxy_inputs.clone(),
    };
    let evidence_bundle = security_decision_evidence_bundle(&evidence_request)?;

    // 2026-04-01 CST: 这里刻意让多头与空头只依赖冻结后的 evidence_bundle，原因是单对话内也要尽量保持双方初判独立；
    // 目的：避免后续把另一方的生成结果反向污染本方结论，退化成一边写一边改口的伪博弈。
    let risk_profile = SecurityDecisionRiskProfile {
        stop_loss_pct: request.stop_loss_pct,
        target_return_pct: request.target_return_pct,
        min_risk_reward_ratio: request.min_risk_reward_ratio,
    };
    let risk_gates = evaluate_security_risk_gates(&evidence_bundle, &risk_profile);
    let market_context = request
        .market_profile
        .as_deref()
        .unwrap_or("general_market");
    let member_opinions = build_member_opinions(&evidence_bundle, &risk_gates, market_context)?;
    let vote_tally = build_vote_tally(&member_opinions);
    let risk_veto = build_risk_veto(&evidence_bundle, &risk_gates, &member_opinions, &vote_tally);
    let bull_case = build_bull_case(&evidence_bundle);
    let bear_case = build_bear_case(&evidence_bundle);
    let mut decision_card = build_security_decision_card(
        &evidence_bundle,
        &bull_case,
        &bear_case,
        &risk_gates,
        &risk_profile,
    );
    apply_committee_vote_to_decision_card(&mut decision_card, &vote_tally);
    apply_risk_veto_to_decision_card(&mut decision_card, &risk_veto);
    apply_training_guardrail_to_decision_card(&mut decision_card);

    Ok(SecurityDecisionCommitteeResult {
        committee_engine: "seven_seat_committee_v3".to_string(),
        committee_session_ref: format!("committee-{}", decision_card.decision_id),
        symbol: evidence_bundle.symbol.clone(),
        analysis_date: evidence_bundle.analysis_date.clone(),
        market_profile: request.market_profile.clone(),
        sector_profile: request.sector_profile.clone(),
        evidence_bundle,
        member_opinions,
        vote_tally,
        risk_veto,
        bull_case,
        bear_case,
        risk_gates,
        decision_card,
    })
}

// 2026-04-07 CST: 这里新增七席委员会席位描述，原因是 V3 需要稳定的固定席位，但每席仍然读取完整证据而不是只看单一因子；
// 目的：把“固定席位 + 轻微倾向参数”显式化，为后续不同市场做小幅微调保留统一入口。
#[derive(Debug, Clone, Copy)]
struct CommitteeSeatProfile {
    member_id: &'static str,
    seat_name: &'static str,
    seat_kind: &'static str,
    tilt_label: &'static str,
    positive_bias: i32,
    caution_bias: i32,
}

const SEVEN_SEAT_ROSTER: [CommitteeSeatProfile; 7] = [
    CommitteeSeatProfile {
        member_id: "seat-1",
        seat_name: "fundamental_steady_seat",
        seat_kind: "deliberation",
        tilt_label: "fundamental_steady",
        positive_bias: 1,
        caution_bias: 1,
    },
    CommitteeSeatProfile {
        member_id: "seat-2",
        seat_name: "trend_confirmation_seat",
        seat_kind: "deliberation",
        tilt_label: "trend_confirmation",
        positive_bias: 2,
        caution_bias: 0,
    },
    CommitteeSeatProfile {
        member_id: "seat-3",
        seat_name: "event_sensitive_seat",
        seat_kind: "deliberation",
        tilt_label: "event_sensitive",
        positive_bias: 0,
        caution_bias: 2,
    },
    CommitteeSeatProfile {
        member_id: "seat-4",
        seat_name: "valuation_odds_seat",
        seat_kind: "deliberation",
        tilt_label: "valuation_odds",
        positive_bias: 1,
        caution_bias: 1,
    },
    CommitteeSeatProfile {
        member_id: "seat-5",
        seat_name: "macro_prudent_seat",
        seat_kind: "deliberation",
        tilt_label: "macro_prudent",
        positive_bias: 0,
        caution_bias: 2,
    },
    CommitteeSeatProfile {
        member_id: "seat-6",
        seat_name: "offensive_flex_seat",
        seat_kind: "deliberation",
        tilt_label: "offensive_flex",
        positive_bias: 2,
        caution_bias: 0,
    },
    CommitteeSeatProfile {
        member_id: "seat-7",
        seat_name: "risk_control_seat",
        seat_kind: "risk_control",
        tilt_label: "risk_control",
        positive_bias: 0,
        caution_bias: 3,
    },
];

// 2026-04-07 CST: 这里实现七席最小独立意见生成器，原因是 V3 第一阶段至少要保证每个固定席位都会在同一份证据包上形成单独的正式意见对象；
// 目的：先把“七席合同”立起来，同时保留现有 bull/bear 摘要给后续 bridge 兼容使用。
fn build_member_opinions(
    bundle: &SecurityDecisionEvidenceBundleResult,
    risk_gates: &[SecurityRiskGateResult],
    market_context: &str,
) -> Result<Vec<SecurityCommitteeMemberOpinion>, SecurityDecisionCommitteeError> {
    SEVEN_SEAT_ROSTER
        .iter()
        .map(|seat| run_child_process_member_opinion(bundle, risk_gates, market_context, seat))
        .collect()
}

fn build_member_opinion(
    bundle: &SecurityDecisionEvidenceBundleResult,
    risk_gates: &[SecurityRiskGateResult],
    market_context: &str,
    seat: &CommitteeSeatProfile,
) -> SecurityCommitteeMemberOpinion {
    // 2026-04-07 CST: 这里在意见对象构建时捕获当前进程号，原因是 V3 已进入子进程级独立执行阶段；
    // 目的：把“谁在算这张票”沉淀进正式合同，后续可直接用于独立性证明、审计和复盘。
    let process_id = std::process::id();
    // 2026-04-07 CST: 这里生成执行实例标识，原因是单纯的 seat_id 只能证明席位身份，
    // 不能证明本次输出来自独立运行实例；目的：把席位身份和运行实例绑定，避免后续只剩静态席位标签。
    let execution_instance_id = format!("{}-{process_id}", seat.member_id);
    let base_score = base_committee_score(bundle, risk_gates);
    let seat_score = base_score + seat.positive_bias - seat.caution_bias;
    let warn_count = risk_gates
        .iter()
        .filter(|gate| gate.result == "warn")
        .count();
    let has_blocking_fail = risk_gates
        .iter()
        .any(|gate| gate.blocking && gate.result == "fail");
    let evidence_incomplete = bundle.evidence_quality.overall_status != "complete";
    let vote = if seat.seat_kind == "risk_control" {
        if has_blocking_fail {
            "avoid"
        } else if evidence_incomplete || !bundle.data_gaps.is_empty() || warn_count > 0 {
            "abstain"
        } else if seat_score >= 2 {
            "hold"
        } else {
            "reduce"
        }
    } else if has_blocking_fail {
        if seat_score >= 3 { "reduce" } else { "avoid" }
    } else if evidence_incomplete && seat.caution_bias >= 2 {
        "abstain"
    } else if seat_score >= 4 {
        "buy"
    } else if seat_score >= 2 {
        "hold"
    } else if seat_score >= 0 {
        "reduce"
    } else {
        "avoid"
    };

    let mut supporting_points = vec![
        bundle.integrated_conclusion.headline.clone(),
        bundle
            .technical_context
            .contextual_conclusion
            .headline
            .clone(),
    ];
    if bundle.fundamental_context.status == "available" {
        supporting_points.push(bundle.fundamental_context.headline.clone());
    }
    if bundle.disclosure_context.status == "available" {
        supporting_points.push(bundle.disclosure_context.headline.clone());
    }
    dedupe_strings(&mut supporting_points);

    let mut counter_points = bundle.data_gaps.clone();
    counter_points.extend(bundle.risk_notes.iter().take(3).cloned());
    if counter_points.is_empty() {
        counter_points
            .push("当前没有出现足以直接推翻研究主线的新增反证，但仍需持续复核。".to_string());
    }
    dedupe_strings(&mut counter_points);

    let mut what_changes_my_mind = vec![
        "若最新正式分红方案与当前证据口径不一致，需要重新审议。".to_string(),
        "若市场与板块从顺风切换为明显逆风，需要重新评估仓位级别。".to_string(),
    ];
    what_changes_my_mind.extend(
        bundle
            .technical_context
            .stock_analysis
            .consultation_conclusion
            .risk_flags
            .iter()
            .take(2)
            .cloned(),
    );
    dedupe_strings(&mut what_changes_my_mind);

    SecurityCommitteeMemberOpinion {
        member_id: seat.member_id.to_string(),
        seat_name: seat.seat_name.to_string(),
        seat_kind: seat.seat_kind.to_string(),
        market_tilt_profile: format!("{market_context}:{}", seat.tilt_label),
        vote: vote.to_string(),
        confidence: classify_member_confidence(seat_score, has_blocking_fail, evidence_incomplete)
            .to_string(),
        reasoning: build_member_reasoning(bundle, seat, vote, warn_count, evidence_incomplete),
        supporting_points,
        counter_points: counter_points.clone(),
        key_risks: counter_points,
        what_changes_my_mind,
        execution_mode: "in_process".to_string(),
        execution_instance_id,
        process_id,
        evidence_hash: bundle.evidence_hash.clone(),
    }
}

pub fn security_committee_member_agent(
    request: &SecurityCommitteeMemberAgentRequest,
) -> Result<SecurityCommitteeMemberOpinion, SecurityDecisionCommitteeError> {
    let Some(seat) = resolve_seat_profile(&request.member_id) else {
        return Err(SecurityDecisionCommitteeError::AgentExecution(format!(
            "unknown committee seat: {}",
            request.member_id
        )));
    };
    let risk_profile = SecurityDecisionRiskProfile {
        stop_loss_pct: default_stop_loss_pct(),
        target_return_pct: default_target_return_pct(),
        min_risk_reward_ratio: default_min_risk_reward_ratio(),
    };
    let risk_gates = evaluate_security_risk_gates(&request.evidence_bundle, &risk_profile);
    let mut opinion = build_member_opinion(
        &request.evidence_bundle,
        &risk_gates,
        &request.market_context,
        seat,
    );
    opinion.execution_mode = "child_process".to_string();
    Ok(opinion)
}

fn run_child_process_member_opinion(
    bundle: &SecurityDecisionEvidenceBundleResult,
    risk_gates: &[SecurityRiskGateResult],
    market_context: &str,
    seat: &CommitteeSeatProfile,
) -> Result<SecurityCommitteeMemberOpinion, SecurityDecisionCommitteeError> {
    let request = ToolRequest {
        tool: "security_committee_member_agent".to_string(),
        args: json!(SecurityCommitteeMemberAgentRequest {
            member_id: seat.member_id.to_string(),
            market_context: market_context.to_string(),
            evidence_bundle: bundle.clone(),
        }),
    };
    let mut opinion = run_child_process_agent(request)?;
    opinion.execution_mode = "child_process".to_string();
    if opinion.key_risks.is_empty() {
        opinion.key_risks = risk_gates
            .iter()
            .filter(|gate| gate.result != "pass")
            .map(|gate| gate.reason.clone())
            .collect();
    }
    Ok(opinion)
}

fn run_child_process_agent(
    request: ToolRequest,
) -> Result<SecurityCommitteeMemberOpinion, SecurityDecisionCommitteeError> {
    let current_exe = std::env::current_exe().map_err(|error| {
        SecurityDecisionCommitteeError::AgentExecution(format!(
            "failed to locate current exe: {error}"
        ))
    })?;
    let input = serde_json::to_vec(&request).map_err(|error| {
        SecurityDecisionCommitteeError::AgentExecution(format!(
            "failed to serialize child process request: {error}"
        ))
    })?;

    let mut child = Command::new(current_exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            SecurityDecisionCommitteeError::AgentExecution(format!(
                "failed to spawn child process: {error}"
            ))
        })?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(&input).map_err(|error| {
            SecurityDecisionCommitteeError::AgentExecution(format!(
                "failed to write child process stdin: {error}"
            ))
        })?;
    }

    let output = child.wait_with_output().map_err(|error| {
        SecurityDecisionCommitteeError::AgentExecution(format!(
            "failed while waiting for child process: {error}"
        ))
    })?;

    if !output.status.success() {
        return Err(SecurityDecisionCommitteeError::AgentExecution(format!(
            "child process exited abnormally: {}",
            output.status
        )));
    }

    let stdout = String::from_utf8(output.stdout).map_err(|error| {
        SecurityDecisionCommitteeError::AgentExecution(format!(
            "child process stdout is not valid utf-8 json: {error}"
        ))
    })?;
    let response = serde_json::from_str::<ToolResponse>(&stdout).map_err(|error| {
        SecurityDecisionCommitteeError::AgentExecution(format!(
            "failed to parse child process response: {error}"
        ))
    })?;

    if response.status != "ok" {
        return Err(SecurityDecisionCommitteeError::AgentExecution(
            response
                .error
                .unwrap_or_else(|| "child process returned non-ok status".to_string()),
        ));
    }

    serde_json::from_value::<SecurityCommitteeMemberOpinion>(response.data).map_err(|error| {
        SecurityDecisionCommitteeError::AgentExecution(format!(
            "failed to parse child process opinion: {error}"
        ))
    })
}

fn resolve_seat_profile(member_id: &str) -> Option<&'static CommitteeSeatProfile> {
    SEVEN_SEAT_ROSTER
        .iter()
        .find(|seat| seat.member_id == member_id)
}

fn base_committee_score(
    bundle: &SecurityDecisionEvidenceBundleResult,
    risk_gates: &[SecurityRiskGateResult],
) -> i32 {
    let mut score = match bundle.integrated_conclusion.stance.as_str() {
        "positive" => 3,
        "watchful_positive" => 2,
        "neutral" => 1,
        "watchful_negative" => -1,
        _ => -2,
    };
    if bundle.technical_context.contextual_conclusion.alignment == "tailwind" {
        score += 1;
    }
    if bundle.fundamental_context.status == "available" {
        score += 1;
    }
    if bundle.disclosure_context.status == "available" {
        score += 1;
    }
    score -= bundle.data_gaps.len().min(2) as i32;
    score -= risk_gates
        .iter()
        .filter(|gate| gate.result == "warn")
        .count()
        .min(2) as i32;
    if risk_gates
        .iter()
        .any(|gate| gate.blocking && gate.result == "fail")
    {
        score -= 2;
    }
    score
}

fn classify_member_confidence(
    seat_score: i32,
    has_blocking_fail: bool,
    evidence_incomplete: bool,
) -> &'static str {
    if has_blocking_fail {
        "guarded"
    } else if evidence_incomplete {
        "medium"
    } else if seat_score >= 4 {
        "high"
    } else if seat_score >= 1 {
        "medium"
    } else {
        "guarded"
    }
}

fn build_member_reasoning(
    bundle: &SecurityDecisionEvidenceBundleResult,
    seat: &CommitteeSeatProfile,
    vote: &str,
    warn_count: usize,
    evidence_incomplete: bool,
) -> String {
    let evidence_status = if evidence_incomplete {
        "证据尚未完全齐备"
    } else {
        "证据主干相对完整"
    };
    format!(
        "{} 在 {} 场景下读取同一份完整证据后，给出 {} 票；当前 {}，综合结论为“{}”，提醒闸门数量为 {}。",
        seat.seat_name,
        seat.tilt_label,
        vote,
        evidence_status,
        bundle.integrated_conclusion.headline,
        warn_count
    )
}

// 2026-04-07 CST: 这里新增七席委员会计票逻辑，原因是 V3 需要把 6 名审议席和 1 名风控席的角色区别明确写入合同；
// 目的：让后续审批简报、仓位计划与审计链可以直接读取投票结构，而不是只能猜最终结论是怎么来的。
fn build_vote_tally(opinions: &[SecurityCommitteeMemberOpinion]) -> SecurityCommitteeVoteTally {
    let deliberation_votes: Vec<&str> = opinions
        .iter()
        .filter(|opinion| opinion.seat_kind == "deliberation")
        .map(|opinion| opinion.vote.as_str())
        .collect();
    let risk_seat_count = opinions
        .iter()
        .filter(|opinion| opinion.seat_kind == "risk_control")
        .count();
    let buy_count = deliberation_votes
        .iter()
        .filter(|vote| **vote == "buy")
        .count();
    let hold_count = deliberation_votes
        .iter()
        .filter(|vote| **vote == "hold")
        .count();
    let reduce_count = deliberation_votes
        .iter()
        .filter(|vote| **vote == "reduce")
        .count();
    let avoid_count = deliberation_votes
        .iter()
        .filter(|vote| **vote == "avoid")
        .count();
    let abstain_count = deliberation_votes
        .iter()
        .filter(|vote| **vote == "abstain")
        .count();

    let counts = [
        ("buy", buy_count),
        ("hold", hold_count),
        ("reduce", reduce_count),
        ("avoid", avoid_count),
        ("abstain", abstain_count),
    ];
    let max_count = counts.iter().map(|(_, count)| *count).max().unwrap_or(0);
    let tied_winners: Vec<&str> = counts
        .iter()
        .filter(|(_, count)| *count == max_count && *count > 0)
        .map(|(vote, _)| *vote)
        .collect();
    let majority_vote = if tied_winners.len() == 1 {
        tied_winners[0].to_string()
    } else {
        "split".to_string()
    };

    SecurityCommitteeVoteTally {
        deliberation_seat_count: deliberation_votes.len(),
        risk_seat_count,
        buy_count,
        hold_count,
        reduce_count,
        avoid_count,
        abstain_count,
        majority_vote,
        majority_count: max_count,
    }
}

fn build_risk_veto(
    bundle: &SecurityDecisionEvidenceBundleResult,
    risk_gates: &[SecurityRiskGateResult],
    opinions: &[SecurityCommitteeMemberOpinion],
    vote_tally: &SecurityCommitteeVoteTally,
) -> SecurityCommitteeRiskVeto {
    let risk_opinion = opinions
        .iter()
        .find(|opinion| opinion.seat_kind == "risk_control");
    let has_blocking_fail = risk_gates
        .iter()
        .any(|gate| gate.blocking && gate.result == "fail");
    let has_warn = risk_gates.iter().any(|gate| gate.result == "warn");

    let (status, reason) = if has_blocking_fail {
        (
            "blocked".to_string(),
            "风控席确认存在阻断型风险闸门失败，委员会结论必须降级为 blocked。".to_string(),
        )
    } else if bundle.evidence_quality.overall_status != "complete"
        || !bundle.data_gaps.is_empty()
        || (vote_tally.majority_vote == "buy" && has_warn)
    {
        (
            "needs_more_evidence".to_string(),
            "风控席认为证据完整度或风险揭示仍不足，当前只能降级为 needs_more_evidence。"
                .to_string(),
        )
    } else {
        (
            "none".to_string(),
            "风控席未触发额外否决，委员会多数意见可按正常流程进入后续治理。".to_string(),
        )
    };

    SecurityCommitteeRiskVeto {
        seat_name: risk_opinion
            .map(|opinion| opinion.seat_name.clone())
            .unwrap_or_else(|| "risk_control_seat".to_string()),
        vote: risk_opinion
            .map(|opinion| opinion.vote.clone())
            .unwrap_or_else(|| "abstain".to_string()),
        status,
        reason,
    }
}

fn apply_risk_veto_to_decision_card(
    decision_card: &mut SecurityDecisionCard,
    risk_veto: &SecurityCommitteeRiskVeto,
) {
    if risk_veto.status == "blocked" {
        decision_card.status = "blocked".to_string();
        decision_card.position_size_suggestion = "none".to_string();
        decision_card.final_recommendation = format!(
            "{} 当前被风控席正式否决，执行层不得放行。",
            risk_veto.reason
        );
    } else if risk_veto.status == "needs_more_evidence"
        && decision_card.status == "ready_for_review"
    {
        decision_card.status = "needs_more_evidence".to_string();
        decision_card.position_size_suggestion = "pilot".to_string();
        decision_card.final_recommendation = format!(
            "{} 当前只能以补证据而非直接放行的方式继续推进。",
            risk_veto.reason
        );
    }
    sync_legacy_direction_alias(decision_card);
}

// 2026-04-11 CST: Add non-executable fallback when the committee result is not
// ready_for_review, reason: the user required runtime enforcement so incomplete
// evidence cannot keep a high-conviction action label.
// Purpose: force blocked / needs_more_evidence states to carry neutral,
// non-executable actions before the result reaches chair or approval stages.
fn apply_training_guardrail_to_decision_card(decision_card: &mut SecurityDecisionCard) {
    match decision_card.status.as_str() {
        "blocked" => {
            decision_card.recommendation_action = "avoid".to_string();
            decision_card.exposure_side = "neutral".to_string();
            decision_card.direction = "neutral".to_string();
        }
        "needs_more_evidence" => {
            decision_card.recommendation_action = "abstain".to_string();
            decision_card.exposure_side = "neutral".to_string();
            decision_card.direction = "neutral".to_string();
            decision_card.confidence_score = decision_card.confidence_score.min(0.49);
            if !decision_card
                .required_next_actions
                .iter()
                .any(|item| item.contains("训练"))
            {
                decision_card
                    .required_next_actions
                    .push("补齐训练样本或正式模型后再进入可执行审阅".to_string());
            }
            if !decision_card.final_recommendation.contains("训练支撑") {
                decision_card.final_recommendation = format!(
                    "{} 当前仍缺训练支撑，不进入高确定性执行建议。",
                    decision_card.final_recommendation
                );
            }
        }
        _ => {}
    }
}

// 2026-04-09 CST: 这里把委员会多数票写回 decision_card，原因是真实 bug 的根因就是 decision_card 没有吸收七席最终动作；
// 目的：先以正式 recommendation_action / exposure_side 表达最终裁决，再让旧字段 direction 作为兼容别名跟随更新。
fn apply_committee_vote_to_decision_card(
    decision_card: &mut SecurityDecisionCard,
    vote_tally: &SecurityCommitteeVoteTally,
) {
    let recommendation_action = normalize_committee_action(&vote_tally.majority_vote);
    decision_card.recommendation_action = recommendation_action.clone();
    decision_card.exposure_side =
        derive_exposure_side_from_action(&recommendation_action).to_string();
    sync_legacy_direction_alias(decision_card);
}

fn normalize_committee_action(value: &str) -> String {
    match value {
        "buy" | "hold" | "reduce" | "avoid" | "abstain" => value.to_string(),
        _ => "abstain".to_string(),
    }
}

fn sync_legacy_direction_alias(decision_card: &mut SecurityDecisionCard) {
    decision_card.direction = decision_card.exposure_side.clone();
}

// 2026-04-01 CST: 这里生成多头立场摘要，原因是投决会需要一个只论证“为什么可以做”的独立对象；
// 目的：把研究链里的支持证据提炼成结构化 thesis，而不是直接给最终买卖建议。
fn build_bull_case(bundle: &SecurityDecisionEvidenceBundleResult) -> SecurityDecisionThesis {
    let stock_conclusion = &bundle
        .technical_context
        .stock_analysis
        .consultation_conclusion;
    let mut thesis_points = vec![
        stock_conclusion.headline.clone(),
        bundle
            .technical_context
            .contextual_conclusion
            .headline
            .clone(),
        bundle.integrated_conclusion.headline.clone(),
    ];
    if bundle.fundamental_context.status == "available" {
        thesis_points.push(bundle.fundamental_context.headline.clone());
    }
    if bundle.disclosure_context.status == "available" {
        thesis_points.push(bundle.disclosure_context.headline.clone());
    }
    dedupe_strings(&mut thesis_points);

    let mut invalidation_conditions = stock_conclusion.risk_flags.clone();
    invalidation_conditions.extend(
        bundle
            .technical_context
            .contextual_conclusion
            .risk_flags
            .clone(),
    );
    if invalidation_conditions.is_empty() {
        invalidation_conditions.push("个股失去技术面确认或环境共振时，原多头论证失效".to_string());
    }
    dedupe_strings(&mut invalidation_conditions);

    SecurityDecisionThesis {
        thesis_label: "bullish_thesis".to_string(),
        headline: format!(
            "{}，当前更接近“有条件通过研究审阅”的多头论证",
            bundle.integrated_conclusion.headline
        ),
        confidence: match bundle.integrated_conclusion.stance.as_str() {
            "positive" => "high".to_string(),
            "watchful_positive" => "medium".to_string(),
            _ => "guarded".to_string(),
        },
        thesis_points,
        invalidation_conditions,
        cited_risks: bundle.risk_notes.clone(),
    }
}

// 2026-04-01 CST: 这里生成空头挑战摘要，原因是投决会必须有一个专门挑错、找失效条件的对立对象；
// 目的：把单边乐观结论拉回到“有哪些证据不足或风险被低估”这一层。
fn build_bear_case(bundle: &SecurityDecisionEvidenceBundleResult) -> SecurityDecisionThesis {
    let mut thesis_points = Vec::new();
    if !bundle.data_gaps.is_empty() {
        thesis_points.extend(bundle.data_gaps.clone());
    }
    thesis_points.extend(bundle.risk_notes.iter().take(4).cloned());
    if thesis_points.is_empty() {
        thesis_points
            .push("当前未发现足以直接否决的强空头证据，但仍需防止研究结论过度乐观".to_string());
    }
    dedupe_strings(&mut thesis_points);

    let invalidation_conditions = vec![
        "如果后续基本面与公告持续确认且环境维持顺风，则本轮空头挑战权重下降".to_string(),
        "如果个股回踩后仍守住关键支撑并延续量价确认，则不宜继续按高强度反对处理".to_string(),
    ];

    SecurityDecisionThesis {
        thesis_label: "bearish_challenge".to_string(),
        headline: "当前需要重点核查证据缺口、事件风险与环境变化，而不是直接把研究偏强等同于可执行"
            .to_string(),
        confidence: if bundle.data_gaps.is_empty() {
            "medium".to_string()
        } else {
            "high".to_string()
        },
        thesis_points,
        invalidation_conditions,
        cited_risks: bundle.risk_notes.clone(),
    }
}

fn dedupe_strings(values: &mut Vec<String>) {
    let mut deduped = Vec::new();
    for value in values.drain(..) {
        if !deduped.contains(&value) {
            deduped.push(value);
        }
    }
    *values = deduped;
}

fn default_lookback_days() -> usize {
    260
}

fn default_disclosure_limit() -> usize {
    8
}

fn default_stop_loss_pct() -> f64 {
    0.05
}

fn default_target_return_pct() -> f64 {
    0.12
}

fn default_min_risk_reward_ratio() -> f64 {
    2.0
}
