#![recursion_limit = "256"]

use std::collections::BTreeMap;

use excel_skill::ops::stock::security_analysis_contextual::SecurityAnalysisContextualResult;
use excel_skill::ops::stock::security_chair_resolution::{
    SecurityChairResolutionResult, build_security_chair_resolution,
};
use excel_skill::ops::stock::security_decision_card::{
    SecurityCommitteeRiskVeto, SecurityCommitteeVoteTally, SecurityDecisionCard,
    SecurityDecisionThesis,
};
use excel_skill::ops::stock::security_decision_committee::SecurityDecisionCommitteeResult;
use excel_skill::ops::stock::security_decision_evidence_bundle::SecurityDecisionEvidenceBundleResult;
use excel_skill::ops::stock::security_independent_advice::{
    SecurityIndependentAdviceDocument, SecurityIndependentAdviceRequest,
    security_independent_advice,
};
use excel_skill::ops::stock::security_record_post_meeting_conclusion::{
    SecurityPostMeetingConclusionRequest, build_security_post_meeting_conclusion,
};
use excel_skill::ops::stock::security_risk_gates::SecurityRiskGateResult;
use excel_skill::ops::stock::security_scorecard::{
    SecurityScorecardDocument, SecurityScorecardModelBinding,
};
use serde_json::{from_value, json};

// 2026-04-13 CST: 这里新增主席 builder 级测试文件，原因是旧的 security_chair_resolution_cli 重链路在 Windows 上会挂住，
// 目的：先用纯构造级测试把主席仲裁最关键的 4 个分支锁住，不再依赖 runtime/HTTP/子进程。

// 2026-04-13 CST: 这里统一构造最小技术分析结果夹具，原因是 committee 结果要求带完整证据对象，
// 目的：通过纯内存 JSON 夹具满足类型要求，同时避免真实行情/同步链路进入测试。
fn fixture_technical_result(symbol: &str, analysis_date: &str) -> serde_json::Value {
    json!({
        "symbol": symbol,
        "as_of_date": analysis_date,
        "analysis_date": analysis_date,
        "evidence_version": format!("technical:{symbol}:{analysis_date}:v1"),
        "requested_as_of_date": analysis_date,
        "effective_analysis_date": analysis_date,
        "effective_trade_date": analysis_date,
        "local_data_last_date": analysis_date,
        "data_freshness_status": "local_exact_requested_date",
        "sync_attempted": false,
        "sync_result": null,
        "date_fallback_reason": null,
        "history_row_count": 260,
        "trend_bias": "bullish",
        "trend_strength": "moderate",
        "volume_confirmation": "confirmed",
        "money_flow_signal": "neutral",
        "mean_reversion_signal": "neutral",
        "range_position_signal": "mid_range",
        "bollinger_position_signal": "neutral",
        "bollinger_midline_signal": "midline_support_bias",
        "bollinger_bandwidth_signal": "normal",
        "breakout_signal": "confirmed_breakout",
        "divergence_signal": "none",
        "timing_signal": "constructive",
        "rsrs_signal": "supportive",
        "rsrs_status": "ready",
        "momentum_signal": "positive",
        "volatility_state": "normal",
        "consultation_conclusion": {
            "bias": "bullish",
            "confidence": "moderate",
            "headline": "趋势与量能保持一致",
            "rationale": ["趋势向上", "量能确认"],
            "risk_flags": []
        },
        "summary": "builder test fixture",
        "recommended_actions": ["继续跟踪突破确认"],
        "watch_points": ["观察量能延续"],
        "indicator_snapshot": {
            // 2026-04-21 CST: Added because TechnicalIndicatorSnapshot now requires
            // one explicit SMA20 field in addition to the older SMA50/SMA200 fields.
            // Reason: this builder fixture should track schema drift without changing
            // any chair-resolution branch semantics under test.
            // Purpose: keep the fixture aligned with the governed technical snapshot contract.
            "close": 10.5,
            "ema_10": 10.2,
            "sma_20": 10.0,
            "sma_50": 9.8,
            "sma_200": 8.9,
            "adx_14": 24.0,
            "plus_di_14": 28.0,
            "minus_di_14": 18.0,
            "obv": 1200000.0,
            "volume_sma_20": 850000.0,
            "volume_ratio_20": 1.2,
            "mfi_14": 56.0,
            "cci_20": 48.0,
            "williams_r_14": -42.0,
            "boll_width_ratio_20": 0.12,
            "macd": 0.8,
            "macd_signal": 0.6,
            "macd_histogram": 0.2,
            "rsi_14": 58.0,
            "k_9": 62.0,
            "d_9": 58.0,
            "j_9": 70.0,
            "rsrs_beta_18": 1.1,
            "rsrs_zscore_18_60": 0.7,
            "rsrs_status": "ready",
            "boll_upper": 11.2,
            "boll_middle": 10.0,
            "boll_lower": 8.8,
            "support_level_20": 9.6,
            "resistance_level_20": 11.1,
            "atr_14": 0.35
        },
        "data_window_summary": {
            "requested_lookback_days": 260,
            "loaded_row_count": 260,
            "start_date": "2025-04-01",
            "end_date": analysis_date
        }
    })
}

// 2026-04-13 CST: 这里统一构造最小 contextual 夹具，原因是证据包 builder 需要完整 technical_context，
// 目的：让 chair builder 测试只关心仲裁分支，不被环境分析链拖住。
fn fixture_contextual_result(
    symbol: &str,
    analysis_date: &str,
) -> SecurityAnalysisContextualResult {
    from_value(json!({
        "symbol": symbol,
        "market_symbol": "510300.SH",
        "sector_symbol": "512800.SH",
        "analysis_date": analysis_date,
        "evidence_version": format!("contextual:{symbol}:{analysis_date}:v1"),
        "analysis_date_guard": {
            "requested_as_of_date": analysis_date,
            "effective_analysis_date": analysis_date,
            "effective_trade_date": analysis_date,
            "local_data_last_date": analysis_date,
            "data_freshness_status": "local_exact_requested_date",
            "sync_attempted": false,
            "sync_result": null,
            "date_fallback_reason": null
        },
        "stock_analysis": fixture_technical_result(symbol, analysis_date),
        "market_analysis": fixture_technical_result("510300.SH", analysis_date),
        "sector_analysis": fixture_technical_result("512800.SH", analysis_date),
        "contextual_conclusion": {
            "alignment": "tailwind",
            "headline": "个股与环境共振",
            "rationale": ["大盘同向", "板块同向"],
            "risk_flags": []
        }
    }))
    .expect("contextual fixture should deserialize")
}

// 2026-04-13 CST: 这里统一构造最小证据包夹具，原因是 committee 结果类型包含 evidence_bundle，
// 目的：保留完整正式对象边界，但避免真实 fullstack 研究链进入 builder 测试。
fn fixture_evidence_bundle(
    symbol: &str,
    analysis_date: &str,
) -> SecurityDecisionEvidenceBundleResult {
    from_value(json!({
        "symbol": symbol,
        "analysis_date": analysis_date,
        "technical_context": fixture_contextual_result(symbol, analysis_date),
        "fundamental_context": {
            "status": "available",
            "source": "fixture",
            "latest_report_period": "2025-12-31",
            "report_notice_date": "2026-03-28",
            "headline": "盈利保持稳定",
            "profit_signal": "positive",
            "report_metrics": {
                "revenue": 1000000000.0,
                "revenue_yoy_pct": 8.5,
                "net_profit": 100000000.0,
                "net_profit_yoy_pct": 10.2,
                "roe_pct": 12.1
            },
            "narrative": ["盈利改善"],
            "risk_flags": []
        },
        "disclosure_context": {
            "status": "available",
            "source": "fixture",
            "announcement_count": 1,
            "headline": "公告面平稳",
            "keyword_summary": ["年报"],
            "recent_announcements": [{
                "published_at": "2026-03-28",
                "title": "2025年年度报告",
                "article_code": "AN202603280001",
                "category": "定期报告"
            }],
            "risk_flags": []
        },
        // 2026-04-16 CST: Added because the evidence bundle contract now always
        // includes ETF facts.
        // Reason: this pure builder test should keep validating chair branches
        // instead of failing on schema drift.
        // Purpose: provide one minimal non-ETF fixture object.
        "etf_context": {
            "status": "not_applicable",
            "source": "fixture",
            "fund_name": null,
            "benchmark": null,
            "asset_scope": null,
            "latest_scale": null,
            "latest_share": null,
            "premium_discount_rate_pct": null,
            "headline": "not applicable for equity",
            "structure_risk_flags": [],
            "research_gaps": []
        },
        // 2026-04-16 CST: Added because cross-border ETF evidence is now part of
        // the canonical bundle contract.
        // Reason: omitting this object causes serde to fail before the builder
        // branch assertions can run.
        // Purpose: mark the equity fixture as explicitly outside the cross-border path.
        "cross_border_context": {
            "status": "not_applicable",
            "analysis_method": "underlying_first_cross_border_etf_v1",
            "underlying_market": {
                "status": "not_applicable",
                "symbol": null,
                "bias": null,
                "confidence": null,
                "headline": "not applicable for equity",
                "support_level_20": null,
                "resistance_level_20": null,
                "rationale": [],
                "risk_flags": []
            },
            "fx_market": {
                "status": "not_applicable",
                "symbol": null,
                "bias": null,
                "confidence": null,
                "headline": "not applicable for equity",
                "support_level_20": null,
                "resistance_level_20": null,
                "rationale": [],
                "risk_flags": []
            },
            "premium_assessment": {
                "status": "not_applicable",
                "premium_discount_rate_pct": null,
                "verdict": "not_applicable",
                "headline": "not applicable for equity",
                "risk_flags": []
            },
            "resonance_verdict": "not_applicable",
            "headline": "not applicable for equity",
            "rationale": [],
            "risk_flags": []
        },
        "industry_context": {
            "sector_symbol": "512800.SH",
            "proxy_bias": "positive",
            "headline": "行业跟随上行",
            "rationale": ["行业代理强势"],
            "risk_flags": []
        },
        "integrated_conclusion": {
            "stance": "positive",
            "headline": "技术与环境协同",
            "rationale": ["技术偏强", "环境顺风"],
            "risk_flags": []
        },
        "evidence_quality": {
            "technical_status": "available",
            "fundamental_status": "available",
            "disclosure_status": "available",
            "overall_status": "complete",
            "risk_flags": []
        },
        "risk_notes": [],
        "data_gaps": [],
        "evidence_hash": "fixture-evidence-hash"
    }))
    .expect("evidence bundle fixture should deserialize")
}

// 2026-04-13 CST: 这里统一构造 committee 结果夹具，原因是主席 builder 只需要稳定读取正式 committee 对象，
// 目的：把 vote/risk/card 口径收在一个小夹具里，避免每条测试都手写散乱字段。
fn fixture_committee_result(
    selected_action: &str,
    confidence_score: f64,
    majority_vote: &str,
    risk_veto_status: &str,
    risk_veto_reason: &str,
) -> SecurityDecisionCommitteeResult {
    let symbol = "601916.SH";
    let analysis_date = "2026-04-13";
    let vote_tally: SecurityCommitteeVoteTally = from_value(json!({
        "deliberation_seat_count": 6,
        "risk_seat_count": 1,
        "buy_count": if majority_vote == "buy" { 4 } else { 0 },
        "hold_count": if majority_vote == "hold" { 4 } else { 0 },
        "reduce_count": 0,
        "avoid_count": 0,
        "abstain_count": if selected_action == "abstain" { 6 } else { 0 },
        "majority_vote": majority_vote,
        "majority_count": 4
    }))
    .expect("vote tally fixture should deserialize");
    let risk_veto: SecurityCommitteeRiskVeto = from_value(json!({
        "seat_name": "risk_control_seat",
        "vote": "hold",
        "status": risk_veto_status,
        "reason": risk_veto_reason
    }))
    .expect("risk veto fixture should deserialize");
    let bull_case: SecurityDecisionThesis = from_value(json!({
        "thesis_label": "bull_case",
        "headline": "上行逻辑成立",
        "confidence": "moderate",
        "thesis_points": ["趋势延续"],
        "invalidation_conditions": ["跌破支撑"],
        "cited_risks": ["量能回落"]
    }))
    .expect("bull thesis fixture should deserialize");
    let bear_case: SecurityDecisionThesis = from_value(json!({
        "thesis_label": "bear_case",
        "headline": "下行风险存在",
        "confidence": "moderate",
        "thesis_points": ["环境逆转"],
        "invalidation_conditions": ["重新站回阻力位"],
        "cited_risks": ["公告扰动"]
    }))
    .expect("bear thesis fixture should deserialize");
    let decision_card: SecurityDecisionCard = from_value(json!({
        "decision_id": format!("{symbol}-{analysis_date}"),
        "symbol": symbol,
        "analysis_date": analysis_date,
        "status": if risk_veto_status == "needs_more_evidence" { "needs_more_evidence" } else { "ready_for_review" },
        "recommendation_action": selected_action,
        "exposure_side": if selected_action == "buy" || selected_action == "hold" || selected_action == "reduce" { "long" } else { "neutral" },
        "direction": if selected_action == "buy" || selected_action == "hold" || selected_action == "reduce" { "long" } else { "neutral" },
        "confidence_score": confidence_score,
        "expected_return_range": "12.0% - 18.0%",
        "downside_risk": "5.0%",
        "position_size_suggestion": "starter",
        "required_next_actions": ["继续跟踪风控状态", "核对量化分数"],
        "final_recommendation": "builder fixture"
    }))
    .expect("decision card fixture should deserialize");

    SecurityDecisionCommitteeResult {
        committee_engine: "builder_fixture".to_string(),
        committee_session_ref: format!("committee-{symbol}-{analysis_date}"),
        symbol: symbol.to_string(),
        analysis_date: analysis_date.to_string(),
        market_profile: None,
        sector_profile: None,
        evidence_bundle: fixture_evidence_bundle(symbol, analysis_date),
        member_opinions: Vec::new(),
        vote_tally,
        risk_veto,
        bull_case,
        bear_case,
        // 2026-04-13 CST: 这里保留一个最小 gate，用于维持正式对象完整性；
        // 目的：不让 builder 测试退化成与真实合同完全脱节的裸字段拼装。
        risk_gates: vec![SecurityRiskGateResult {
            gate_name: "analysis_date_gate".to_string(),
            result: "pass".to_string(),
            blocking: false,
            reason: "fixture".to_string(),
            metric_snapshot: vec!["analysis_date=2026-04-13".to_string()],
            remediation: None,
        }],
        decision_card,
    }
}

// 2026-04-13 CST: 这里统一构造 scorecard 夹具，原因是主席仲裁会读取 score_status / probability / limitations，
// 目的：让每个测试只改分支相关字段，不重复铺整份评分卡结构。
fn fixture_scorecard(score_status: &str) -> SecurityScorecardDocument {
    SecurityScorecardDocument {
        scorecard_id: "scorecard-601916.SH-2026-04-13".to_string(),
        contract_version: "security_scorecard.v1".to_string(),
        document_type: "security_scorecard".to_string(),
        generated_at: "2026-04-13T10:00:00+08:00".to_string(),
        symbol: "601916.SH".to_string(),
        analysis_date: "2026-04-13".to_string(),
        decision_id: "601916.SH-2026-04-13".to_string(),
        decision_ref: "decision_ref:601916.SH:2026-04-13:v1".to_string(),
        approval_ref: "approval_ref:601916.SH:2026-04-13:v1".to_string(),
        score_status: score_status.to_string(),
        label_definition: "horizon_10d_stop_5pct_target_10pct".to_string(),
        model_binding: SecurityScorecardModelBinding {
            model_id: Some("model-001".to_string()),
            model_version: Some("v1".to_string()),
            training_window: Some("2024-01-01..2025-12-31".to_string()),
            oot_window: Some("2026-01-01..2026-03-31".to_string()),
            target_label_definition: Some("positive_return_10d".to_string()),
            positive_label_definition: Some("10d_profit".to_string()),
            // 2026-04-16 CST: Added because the governed scorecard model binding
            // contract now carries instrument_subscope and builder fixtures must
            // stay aligned with the formal document shape.
            // Purpose: keep this pure chair builder test compiling against the
            // current scorecard contract without widening the fixture scope.
            instrument_subscope: None,
            binning_version: Some("bin-v1".to_string()),
            coefficient_version: Some("coef-v1".to_string()),
            model_sha256: Some("sha256-fixture".to_string()),
        },
        raw_feature_snapshot: BTreeMap::new(),
        feature_contributions: Vec::new(),
        group_breakdown: Vec::new(),
        base_score: Some(600.0),
        total_score: Some(680.0),
        success_probability: Some(0.71),
        quant_signal: "supportive".to_string(),
        quant_stance: "build".to_string(),
        recommendation_action: "buy".to_string(),
        exposure_side: "long".to_string(),
        score_summary: "builder fixture scorecard".to_string(),
        limitations: if score_status == "ready" {
            vec![]
        } else {
            vec!["量化模型暂不可用".to_string()]
        },
    }
}

// 2026-04-13 CST: 这里统一构造独立建议文档，原因是主席现在可以消费标准化独立建议 Tool 产物，
// 目的：在 builder 测试里直接验证标准对象冲突时的仲裁分支。
fn fixture_independent_advice(suggested_stance: &str) -> SecurityIndependentAdviceDocument {
    security_independent_advice(&SecurityIndependentAdviceRequest {
        symbol: "601916.SH".to_string(),
        analysis_date: "2026-04-13".to_string(),
        source_type: "llm_independent_review".to_string(),
        suggested_stance: suggested_stance.to_string(),
        confidence: Some(0.88),
        rationale: Some("独立建议与主链存在需要解释的差异".to_string()),
        key_risks: vec!["结论冲突".to_string()],
        evidence_basis: vec!["committee_package_v1".to_string()],
        generated_at: "2026-04-13T09:30:00+08:00".to_string(),
    })
}

#[test]
fn chair_builder_returns_for_revision_when_scorecard_not_ready() {
    let committee = fixture_committee_result("buy", 0.82, "buy", "pass", "no veto");
    let scorecard = fixture_scorecard("model_unavailable");

    let document =
        build_security_chair_resolution(&committee, &scorecard, None, "2026-04-13T10:30:00+08:00");

    // 2026-04-13 CST: 这里锁 scorecard not ready 分支，原因是用户明确要求这类情况优先修正而不是回避；
    // 目的：确保主席会退回量化补件，而不是误放行。
    assert_eq!(document.final_action, "return_for_revision");
    assert_eq!(
        document.return_to_stage.as_deref(),
        Some("security_scorecard")
    );
    assert!(document.revision_required);
    assert_eq!(document.evidence_sufficiency, "partial");
    assert_eq!(
        document.override_reason.as_deref(),
        Some("scorecard_not_ready")
    );
    assert!(
        document
            .required_materials
            .iter()
            .any(|item: &String| item.contains("补充量化评分卡"))
    );
}

#[test]
fn chair_builder_defers_when_committee_abstains_without_independent_advice() {
    let committee = fixture_committee_result("abstain", 0.68, "split", "pass", "committee split");
    let scorecard = fixture_scorecard("model_unavailable");

    let document =
        build_security_chair_resolution(&committee, &scorecard, None, "2026-04-13T10:35:00+08:00");

    // 2026-04-13 CST: 这里锁 defer 分支，原因是“委员会 abstain + 量化未就绪 + 无独立建议”是典型暂缓签发场景；
    // 目的：确保主席输出 defer，而不是错误走 reject 或 approve。
    assert_eq!(document.final_action, "defer");
    assert_eq!(
        document.return_to_stage.as_deref(),
        Some("security_decision_committee")
    );
    assert!(!document.revision_required);
    assert_eq!(
        document.override_reason.as_deref(),
        Some("committee_abstain")
    );
    assert!(
        document
            .blocking_reasons
            .iter()
            .any(|item: &String| item.contains("主席暂缓签发"))
    );
}

#[test]
fn chair_builder_returns_for_revision_when_independent_advice_conflicts() {
    let committee = fixture_committee_result("buy", 0.84, "buy", "pass", "no veto");
    let scorecard = fixture_scorecard("ready");
    let advice = fixture_independent_advice("avoid");

    let document = build_security_chair_resolution(
        &committee,
        &scorecard,
        Some(&advice),
        "2026-04-13T10:40:00+08:00",
    );

    // 2026-04-13 CST: 这里锁独立建议冲突分支，原因是本轮用户已明确“大模型独立建议线”必须进入主席正式仲裁；
    // 目的：确保冲突时走 return_for_revision，并保留独立建议映射痕迹。
    assert_eq!(document.final_action, "return_for_revision");
    assert_eq!(
        document.return_to_stage.as_deref(),
        Some("security_decision_committee")
    );
    assert!(document.revision_required);
    assert_eq!(document.conflict_level, "high_conflict");
    assert_eq!(
        document.override_reason.as_deref(),
        Some("independent_advice_conflict")
    );
    assert!(
        document
            .evidence_mapping
            .iter()
            .any(|item| item.source.starts_with("independent_advice:"))
    );
}

#[test]
fn post_meeting_conclusion_builder_keeps_chair_process_fields_consistent() {
    let committee = fixture_committee_result("buy", 0.84, "buy", "pass", "no veto");
    let scorecard = fixture_scorecard("ready");
    let chair =
        build_security_chair_resolution(&committee, &scorecard, None, "2026-04-13T10:45:00+08:00");
    let chair_result = SecurityChairResolutionResult {
        committee_result: committee,
        scorecard,
        chair_resolution: chair.clone(),
    };
    let request = SecurityPostMeetingConclusionRequest {
        symbol: "601916.SH".to_string(),
        market_symbol: None,
        sector_symbol: None,
        market_profile: None,
        sector_profile: None,
        as_of_date: Some("2026-04-13".to_string()),
        lookback_days: 260,
        disclosure_limit: 8,
        stop_loss_pct: 0.05,
        target_return_pct: 0.12,
        min_risk_reward_ratio: 2.0,
        created_at: "2026-04-13T10:50:00+08:00".to_string(),
        scorecard_model_path: Some("artifacts/security_scorecard/model.json".to_string()),
        execution_notes: vec!["按主席要求推进".to_string()],
        follow_up_actions: vec!["继续跟踪公告".to_string()],
    };

    let post_meeting = build_security_post_meeting_conclusion(&chair_result, &request);

    // 2026-04-13 CST: 这里锁会后结论字段一致性，原因是用户明确要求 post meeting conclusion 与主席流程字段不能漂移；
    // 目的：确保 chair_process_action / final_action / final_stance / revision_required / return_to_stage 同源一致。
    assert_eq!(
        post_meeting.document_type,
        "security_post_meeting_conclusion"
    );
    assert_eq!(post_meeting.chair_resolution_ref, chair.chair_resolution_id);
    assert_eq!(post_meeting.final_action, chair.selected_action);
    assert_eq!(post_meeting.chair_process_action, chair.final_action);
    assert_eq!(post_meeting.final_trading_stance, chair.final_stance);
    assert_eq!(
        post_meeting.final_exposure_side,
        chair.selected_exposure_side
    );
    assert_eq!(post_meeting.revision_required, chair.revision_required);
    assert_eq!(post_meeting.return_to_stage, chair.return_to_stage);
}
