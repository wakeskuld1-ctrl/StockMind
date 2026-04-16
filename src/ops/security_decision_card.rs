use serde::{Deserialize, Serialize};

use crate::ops::stock::security_decision_evidence_bundle::SecurityDecisionEvidenceBundleResult;
use crate::ops::stock::security_risk_gates::{SecurityDecisionRiskProfile, SecurityRiskGateResult};

// 2026-04-01 CST: 这里定义投决立场摘要，原因是顶层 committee 需要把多头和空头的初判以结构化对象保留下来；
// 目的：让“独立立场”不再只是自由文本，而能被投决卡、Skill 和后续审阅层稳定消费。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityDecisionThesis {
    pub thesis_label: String,
    pub headline: String,
    pub confidence: String,
    pub thesis_points: Vec<String>,
    pub invalidation_conditions: Vec<String>,
    pub cited_risks: Vec<String>,
}

// 2026-04-07 CST: 这里新增七席委员会单席意见合同，原因是 V3 不再只有 bull/bear 两个摘要，必须把每个席位的独立投票与理由正式结构化保留下来；
// 目的：让 approval、audit、review 和后续复盘都能直接消费“谁投了什么票、基于什么理由、在什么条件下会改票”这类正式工件。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCommitteeMemberOpinion {
    pub member_id: String,
    pub seat_name: String,
    pub seat_kind: String,
    pub market_tilt_profile: String,
    pub vote: String,
    pub confidence: String,
    pub reasoning: String,
    pub supporting_points: Vec<String>,
    pub counter_points: Vec<String>,
    pub key_risks: Vec<String>,
    pub what_changes_my_mind: Vec<String>,
    pub execution_mode: String,
    // 2026-04-07 CST: 这里补充独立执行实例标识，原因是委员会已经升级为子进程级求解，
    // 仅保留 execution_mode 不足以向审批/复盘链证明“每席独立运行”；目的：沉淀可验证的独立执行证据。
    pub execution_instance_id: String,
    // 2026-04-07 CST: 这里补充子进程进程号，原因是投决会需要一个更硬的运行时证据，
    // 用来证明七席不是在同一执行上下文里串行改写同一个对象；目的：给独立性验证测试和审计说明直接取数。
    pub process_id: u32,
    pub evidence_hash: String,
}

// 2026-04-07 CST: 这里新增七席委员会计票摘要，原因是 V3 需要把“六名审议委员的多数票”和“风控席单独存在”明确写进正式输出；
// 目的：避免后续审批与复盘只能看到最终结论，看不到票型结构、席位数量与多数形成过程。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCommitteeVoteTally {
    pub deliberation_seat_count: usize,
    pub risk_seat_count: usize,
    pub buy_count: usize,
    pub hold_count: usize,
    pub reduce_count: usize,
    pub avoid_count: usize,
    pub abstain_count: usize,
    pub majority_vote: String,
    pub majority_count: usize,
}

// 2026-04-07 CST: 这里新增风控席有限否决摘要，原因是用户已经确认 V3 采用“多数制 + 风控席有限否决”而不是纯简单多数；
// 目的：让系统把风控席是否触发降级、降到什么状态、原因是什么，稳定地写进对外合同。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityCommitteeRiskVeto {
    pub seat_name: String,
    pub vote: String,
    pub status: String,
    pub reason: String,
}

// 2026-04-01 CST: 这里定义证券投决卡，原因是研究结论、正反方和闸门结果需要最终沉淀为一个统一对象；
// 目的：为后续审批、复核和用户输出提供单一裁决载体，而不是继续拼接多份分散结果。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityDecisionCard {
    pub decision_id: String,
    pub symbol: String,
    pub analysis_date: String,
    pub status: String,
    // 2026-04-09 CST: 这里新增正式动作语义，原因是旧 `direction` 直接跟随 stance，已被真实主链证明会与委员会多数票漂移；
    // 目的：把“最终建议做什么”从暴露方向里拆出来，避免再出现多数票 avoid 但方向仍 long 的错误输出。
    pub recommendation_action: String,
    // 2026-04-09 CST: 这里新增正式暴露方向语义，原因是用户明确要求动作与暴露方向分层，而不是继续混成一个旧字段；
    // 目的：为 approval_bridge、position_plan 和后续 scorecard 对齐提供稳定的 side 字段。
    pub exposure_side: String,
    pub direction: String,
    pub confidence_score: f64,
    pub expected_return_range: String,
    pub downside_risk: String,
    pub position_size_suggestion: String,
    pub required_next_actions: Vec<String>,
    pub final_recommendation: String,
}

// 2026-04-01 CST: 这里统一生成证券投决卡，原因是 committee 顶层应该专注组织流程，而不是内嵌裁决细节；
// 目的：把状态归类、仓位建议和最终话术收口到一个明确模块中。
pub fn build_security_decision_card(
    bundle: &SecurityDecisionEvidenceBundleResult,
    bull_case: &SecurityDecisionThesis,
    bear_case: &SecurityDecisionThesis,
    risk_gates: &[SecurityRiskGateResult],
    risk_profile: &SecurityDecisionRiskProfile,
) -> SecurityDecisionCard {
    let has_blocking_fail = risk_gates
        .iter()
        .any(|gate| gate.blocking && gate.result == "fail");
    let has_warn = risk_gates.iter().any(|gate| gate.result == "warn");

    let status = if has_blocking_fail {
        "blocked".to_string()
    } else if bundle.evidence_quality.overall_status != "complete" || has_warn {
        "needs_more_evidence".to_string()
    } else {
        "ready_for_review".to_string()
    };

    let recommendation_action = derive_preliminary_recommendation_action(bundle).to_string();
    let exposure_side = derive_exposure_side_from_action(&recommendation_action).to_string();
    let direction = exposure_side.clone();

    let confidence_score = score_confidence(bundle, risk_gates);
    let position_size_suggestion = match status.as_str() {
        "blocked" => "none".to_string(),
        "needs_more_evidence" => "pilot".to_string(),
        _ => "starter".to_string(),
    };

    let required_next_actions = collect_next_actions(risk_gates, bull_case, bear_case);
    let expected_return_range = format!(
        "{:.1}% - {:.1}%",
        risk_profile.target_return_pct * 100.0,
        risk_profile.target_return_pct * 100.0 * 1.5
    );
    let downside_risk = format!("{:.1}%", risk_profile.stop_loss_pct * 100.0);
    let final_recommendation =
        build_final_recommendation(&status, &position_size_suggestion, risk_profile, risk_gates);

    SecurityDecisionCard {
        decision_id: format!("{}-{}", bundle.symbol, bundle.analysis_date),
        symbol: bundle.symbol.clone(),
        analysis_date: bundle.analysis_date.clone(),
        status,
        recommendation_action,
        exposure_side,
        direction,
        confidence_score,
        expected_return_range,
        downside_risk,
        position_size_suggestion,
        required_next_actions,
        final_recommendation,
    }
}

// 2026-04-09 CST: 这里保留一个初步动作推导器，原因是 decision_card builder 在 committee 投票完成前仍需要先生成一版基础对象；
// 目的：让后续 committee 能在同一个正式对象上覆盖最终动作语义，而不是继续散落多个临时字段。
fn derive_preliminary_recommendation_action(
    bundle: &SecurityDecisionEvidenceBundleResult,
) -> &'static str {
    match bundle.integrated_conclusion.stance.as_str() {
        "negative" | "watchful_negative" | "bearish" => "avoid",
        "technical_only" | "neutral" => "hold",
        _ => "buy",
    }
}

// 2026-04-09 CST: 这里集中维护动作到暴露方向的映射，原因是主链现在需要同时服务 committee、scorecard、bridge 与 position_plan；
// 目的：避免多处重复写 if/else 并再次发生动作和方向语义不一致。
pub fn derive_exposure_side_from_action(action: &str) -> &'static str {
    match action {
        "buy" | "hold" | "reduce" => "long",
        "short" => "short",
        "hedge" => "hedge",
        _ => "neutral",
    }
}

// 2026-04-01 CST: 这里把多源信息压成一个简单置信分，原因是投决卡需要稳定的数值字段给后续 UI/审批使用；
// 目的：先提供可解释的 v1 分值，再为后续更复杂的打分模型预留位置。
fn score_confidence(
    bundle: &SecurityDecisionEvidenceBundleResult,
    risk_gates: &[SecurityRiskGateResult],
) -> f64 {
    let mut score = match bundle.integrated_conclusion.stance.as_str() {
        "positive" => 0.78,
        "watchful_positive" => 0.66,
        "neutral" => 0.52,
        _ => 0.35,
    };

    if bundle.technical_context.contextual_conclusion.alignment == "tailwind" {
        score += 0.08;
    }
    if bundle.evidence_quality.overall_status != "complete" {
        score -= 0.08;
    }
    score -= risk_gates
        .iter()
        .filter(|gate| gate.result == "warn")
        .count() as f64
        * 0.05;
    if risk_gates
        .iter()
        .any(|gate| gate.blocking && gate.result == "fail")
    {
        score -= 0.15;
    }

    score.clamp(0.0, 0.95)
}

// 2026-04-01 CST: 这里集中生成后续动作，原因是投决会输出需要告诉下一步该补什么而不是只给状态；
// 目的：让用户和后续 AI 在 blocked / needs_more_evidence 场景下有明确动作列表。
fn collect_next_actions(
    risk_gates: &[SecurityRiskGateResult],
    bull_case: &SecurityDecisionThesis,
    bear_case: &SecurityDecisionThesis,
) -> Vec<String> {
    let mut actions = Vec::new();
    for gate in risk_gates {
        if let Some(remediation) = gate.remediation.as_ref() {
            actions.push(remediation.clone());
        }
    }
    actions.push(format!(
        "继续跟踪多头失效条件：{}",
        bull_case.invalidation_conditions.join("；")
    ));
    actions.push(format!(
        "继续核对空头挑战点：{}",
        bear_case.thesis_points.join("；")
    ));
    dedupe_strings(&mut actions);
    actions
}

// 2026-04-01 CST: 这里集中生成最终裁决话术，原因是状态、仓位和风报比说明不应分散在顶层 Tool 里手写；
// 目的：保持 CLI、Skill 与后续 UI 输出的一致口径。
fn build_final_recommendation(
    status: &str,
    position_size_suggestion: &str,
    risk_profile: &SecurityDecisionRiskProfile,
    risk_gates: &[SecurityRiskGateResult],
) -> String {
    let ratio = if risk_profile.stop_loss_pct <= f64::EPSILON {
        0.0
    } else {
        risk_profile.target_return_pct / risk_profile.stop_loss_pct
    };
    match status {
        "blocked" => format!(
            "当前不建议进入执行建议，核心原因是风报比仅为 {:.2}，尚未达到投决会的最低要求。",
            ratio
        ),
        "needs_more_evidence" => format!(
            "当前仅建议以 {} 级别观察或试探，虽然风报比为 {:.2}，但仍有 {} 个闸门处于提醒状态。",
            position_size_suggestion,
            ratio,
            risk_gates
                .iter()
                .filter(|gate| gate.result == "warn")
                .count()
        ),
        _ => format!(
            "当前可进入审阅状态，建议以 {} 仓位方案启动，核心依据是风报比为 {:.2} 且主要闸门已通过。",
            position_size_suggestion, ratio
        ),
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
