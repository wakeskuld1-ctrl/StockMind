mod common;

use chrono::{Duration, NaiveDate};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{
    create_test_runtime_db, run_cli_with_json, run_cli_with_json_runtime_and_envs,
};
use excel_skill::ops::stock::security_decision_approval_bridge::{
    PersistedApprovalPositionPlanBinding, PersistedApprovalRequest, PersistedApprovalState,
    PersistedApprovalStatus, PersistedDecisionCard, PersistedDecisionDirection,
    PersistedDecisionStatus, PersistedDownsideRisk, PersistedExpectedReturnRange,
    PersistedPortfolioImpact, PersistedPositionSizeSuggestion,
};
use excel_skill::ops::stock::security_decision_approval_brief::{
    SecurityApprovalBriefPackageBinding, SecurityApprovalBriefPositionPlanSummary,
    SecurityDecisionApprovalBrief,
};
use excel_skill::ops::stock::security_decision_package::{
    SecurityDecisionPackageArtifact, SecurityDecisionPackageBuildInput,
    build_security_decision_package, sha256_for_json_value,
};
use excel_skill::ops::stock::security_position_plan::{
    PositionAddPlan, PositionEntryPlan, PositionReducePlan, PositionStopLossPlan,
    PositionTakeProfitPlan, SecurityPositionPlan, SecurityPositionPlanApprovalBinding,
};
use excel_skill::ops::stock::security_record_post_meeting_conclusion::SecurityPostMeetingConclusionDocument;
use excel_skill::ops::stock::security_scorecard::{
    SecurityScoreFeatureContribution, SecurityScoreGroupBreakdown, SecurityScorecardDocument,
    SecurityScorecardModelBinding,
};

// 2026-04-02 CST: 这里新增独立的证券 package 校验测试夹具，原因是 P0-5 需要验证“先提交审批包，再回头校验”的完整往返路径；
// 目的：把 package verify 的 happy path 和篡改路径都锁在独立测试里，避免和 submit_approval 测试耦得过重。
fn create_stock_history_csv(prefix: &str, file_name: &str, rows: &[String]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_decision_verify_package")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("security decision verify fixture dir should exist");

    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n")).expect("security decision verify csv should be written");
    csv_path
}

// 2026-04-16 CST: Reason=add a local JSON fixture helper for verify-package governance
// regressions that must avoid the unstable submit mainline while still writing
// formal artifacts to disk.
// Purpose=keep the new chair-binding tamper test isolated to package verification.
fn create_json_fixture(prefix: &str, file_name: &str, value: &Value) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_decision_verify_package")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir)
        .expect("security decision verify json fixture dir should exist");

    let json_path = fixture_dir.join(file_name);
    fs::write(
        &json_path,
        serde_json::to_vec_pretty(value).expect("json fixture should serialize"),
    )
    .expect("security decision verify json fixture should be written");
    json_path
}

// 2026-04-16 CST: Reason=seed a minimal governed package fixture for the new
// post-meeting chair tamper regression without routing through submit_approval.
// Purpose=let verify_package focus on chair-binding integrity instead of upstream
// approval-chain drift that is being handled in parallel.
fn create_minimal_verifiable_package_fixture(
    runtime_db_path: &Path,
    prefix: &str,
) -> (PathBuf, String) {
    let decision_id = "decision-601916-SH-2026-04-16".to_string();
    // 2026-04-16 CST: Match the current production chair identifier instead of the
    // retired synthetic ref shape that an earlier weak rewrite invented here.
    // Purpose=keep minimal verify fixtures aligned with the real chair-resolution contract.
    let chair_resolution_ref = format!("chair-{decision_id}");
    let decision_ref = "decision:601916.SH:2026-04-16".to_string();
    let approval_ref = "approval:decision-601916-SH-2026-04-16".to_string();
    let position_plan_ref = "position-plan:601916.SH:2026-04-16".to_string();
    let approval_brief_ref = "brief-decision-601916-SH-2026-04-16".to_string();
    let scorecard_ref = "scorecard-601916.SH-2026-04-16".to_string();

    let decision_card = PersistedDecisionCard {
        decision_ref: decision_ref.clone(),
        decision_id: decision_id.clone(),
        scene_name: "security_review".to_string(),
        asset_id: "601916.SH".to_string(),
        instrument_type: "equity".to_string(),
        strategy_type: "security_decision_committee".to_string(),
        horizon: "swing_10d".to_string(),
        recommendation_action: "buy".to_string(),
        exposure_side: "long".to_string(),
        direction: PersistedDecisionDirection::Long,
        status: PersistedDecisionStatus::ReadyForReview,
        confidence_score: 0.82,
        expected_return_range: PersistedExpectedReturnRange {
            low: 0.08,
            base: 0.12,
            high: 0.18,
        },
        downside_risk: PersistedDownsideRisk {
            soft_stop: 0.03,
            hard_stop: 0.05,
            tail_risk_note: Some("fixture tail risk".to_string()),
        },
        position_size_suggestion: PersistedPositionSizeSuggestion {
            gross_pct: 0.06,
            max_pct: 0.12,
            sizing_basis: "fixture".to_string(),
        },
        key_supporting_points: vec!["trend intact".to_string()],
        key_risks: vec!["earnings volatility".to_string()],
        invalidation_conditions: vec!["break below support".to_string()],
        evidence_refs: vec!["evidence-v1".to_string()],
        portfolio_impact: PersistedPortfolioImpact {
            sector_exposure_delta: Some(0.02),
            factor_exposure_note: Some("bank beta up".to_string()),
            liquidity_class: Some("liquid".to_string()),
        },
        approval: PersistedApprovalState {
            required: true,
            approval_state: PersistedApprovalStatus::Pending,
            approval_ref: Some(approval_ref.clone()),
        },
    };
    let decision_card_value =
        serde_json::to_value(&decision_card).expect("decision card should serialize");
    let decision_card_path =
        create_json_fixture(prefix, "decision_card.json", &decision_card_value);

    let position_plan = SecurityPositionPlan {
        contract_version: "security_position_plan.v1".to_string(),
        document_type: "security_position_plan".to_string(),
        plan_id: position_plan_ref.clone(),
        decision_id: decision_id.clone(),
        decision_ref: decision_ref.clone(),
        approval_ref: approval_ref.clone(),
        symbol: "601916.SH".to_string(),
        analysis_date: "2026-04-16".to_string(),
        plan_direction: "Long".to_string(),
        plan_status: "ready".to_string(),
        risk_budget_pct: 0.02,
        suggested_gross_pct: 0.06,
        starter_gross_pct: 0.04,
        max_gross_pct: 0.12,
        entry_grade: "A".to_string(),
        entry_reason: "fixture entry".to_string(),
        entry_blockers: Vec::new(),
        target_gross_pct: 0.10,
        sizing_grade: "A".to_string(),
        sizing_reason: "fixture sizing".to_string(),
        sizing_risk_flags: Vec::new(),
        entry_plan: PositionEntryPlan {
            entry_mode: "breakout".to_string(),
            trigger_condition: "price above breakout".to_string(),
            starter_gross_pct: 0.04,
            notes: "fixture".to_string(),
        },
        add_plan: PositionAddPlan {
            allow_add: true,
            trigger_condition: "retest holds".to_string(),
            max_gross_pct: 0.12,
            notes: "fixture".to_string(),
        },
        reduce_plan: PositionReducePlan {
            allow_reduce: true,
            trigger_condition: "momentum weakens".to_string(),
            target_gross_pct: 0.05,
            notes: "fixture".to_string(),
        },
        stop_loss_plan: PositionStopLossPlan {
            stop_loss_pct: 0.05,
            hard_stop_condition: "close below invalidation".to_string(),
            notes: "fixture".to_string(),
        },
        take_profit_plan: PositionTakeProfitPlan {
            first_target_pct: 0.10,
            second_target_pct: 0.18,
            partial_exit_rule: "take one third".to_string(),
            notes: "fixture".to_string(),
        },
        cancel_conditions: vec!["macro break".to_string()],
        sizing_rationale: vec!["fixture rationale".to_string()],
        approval_binding: SecurityPositionPlanApprovalBinding {
            decision_ref: decision_ref.clone(),
            approval_ref: approval_ref.clone(),
            approval_request_ref: approval_ref.clone(),
            package_scope: "security_decision_submit_approval".to_string(),
            binding_status: "bound".to_string(),
        },
    };
    let position_plan_value =
        serde_json::to_value(&position_plan).expect("position plan should serialize");
    let position_plan_path =
        create_json_fixture(prefix, "position_plan.json", &position_plan_value);
    let position_plan_sha256 =
        sha256_for_json_value(&position_plan_value).expect("position plan sha should compute");

    let approval_request = PersistedApprovalRequest {
        approval_ref: approval_ref.clone(),
        decision_id: decision_id.clone(),
        scene_name: "security_review".to_string(),
        status: PersistedApprovalStatus::Pending,
        created_at: "2026-04-16T09:00:00+08:00".to_string(),
        decision_ref: Some(decision_ref.clone()),
        evidence_hash: Some("evidence-hash".to_string()),
        governance_hash: Some("governance-hash".to_string()),
        min_approvals: 2,
        approved_reviewers: Vec::new(),
        approved_signatures: Vec::new(),
        enforce_role_matrix: true,
        require_risk_signoff: true,
        auto_reject_recommended: false,
        auto_reject_reason: None,
        auto_reject_gate_names: Vec::new(),
        recovery_action_required: false,
        recovery_actions: Vec::new(),
        position_plan_binding: Some(PersistedApprovalPositionPlanBinding {
            position_plan_ref: position_plan_ref.clone(),
            position_plan_path: position_plan_path.to_string_lossy().to_string(),
            position_plan_contract_version: "security_position_plan.v1".to_string(),
            position_plan_sha256,
            plan_status: "ready".to_string(),
            plan_direction: "Long".to_string(),
            gross_limit_summary: "starter 4%, max 12%".to_string(),
        }),
    };
    let approval_request_value =
        serde_json::to_value(&approval_request).expect("approval request should serialize");
    let approval_request_path =
        create_json_fixture(prefix, "approval_request.json", &approval_request_value);

    let approval_brief = SecurityDecisionApprovalBrief {
        brief_id: approval_brief_ref.clone(),
        contract_version: "security_approval_brief.v1".to_string(),
        document_type: "security_approval_brief".to_string(),
        generated_at: "2026-04-16T09:05:00+08:00".to_string(),
        scene_name: "security_review".to_string(),
        decision_id: decision_id.clone(),
        decision_ref: decision_ref.clone(),
        approval_ref: approval_ref.clone(),
        symbol: "601916.SH".to_string(),
        analysis_date: "2026-04-16".to_string(),
        decision_status: "ready_for_review".to_string(),
        approval_status: "Pending".to_string(),
        committee_status: "ready_for_review".to_string(),
        direction: "Long".to_string(),
        confidence_score: 0.82,
        confidence_band: "high".to_string(),
        executive_summary: "fixture summary".to_string(),
        bull_summary: vec!["trend supportive".to_string()],
        bear_summary: vec!["macro risk".to_string()],
        core_supporting_points: vec!["supporting point".to_string()],
        core_risks: vec!["core risk".to_string()],
        gate_summary: vec!["analysis_date_gate:pass:fixture".to_string()],
        gate_outcome_summary: vec!["analysis_date_gate -> pass".to_string()],
        position_summary: "starter long".to_string(),
        risk_budget_summary: "risk 2%".to_string(),
        entry_summary: "enter on breakout".to_string(),
        add_summary: "add on retest".to_string(),
        stop_loss_summary: "stop 5%".to_string(),
        take_profit_summary: "targets 10/18%".to_string(),
        cancel_summary: "cancel on macro break".to_string(),
        position_plan_summary: SecurityApprovalBriefPositionPlanSummary {
            position_plan_status: "ready".to_string(),
            risk_budget_summary: "risk 2%".to_string(),
            entry_summary: "enter on breakout".to_string(),
            add_summary: "add on retest".to_string(),
            stop_loss_summary: "stop 5%".to_string(),
            take_profit_summary: "targets 10/18%".to_string(),
            cancel_summary: "cancel on macro break".to_string(),
        },
        required_next_actions: vec!["await reviewers".to_string()],
        final_recommendation: "buy".to_string(),
        recommended_review_action: "proceed_with_review".to_string(),
        master_scorecard_summary: None,
        model_grade_summary: None,
        model_governance_summary: None,
        evidence_hash: "evidence-hash".to_string(),
        governance_hash: "governance-hash".to_string(),
        package_binding: SecurityApprovalBriefPackageBinding {
            artifact_role: "approval_brief".to_string(),
            brief_contract_version: "security_approval_brief.v1".to_string(),
            decision_ref: decision_ref.clone(),
            approval_ref: approval_ref.clone(),
            decision_id: decision_id.clone(),
        },
    };
    let approval_brief_value =
        serde_json::to_value(&approval_brief).expect("approval brief should serialize");
    let approval_brief_path =
        create_json_fixture(prefix, "approval_brief.json", &approval_brief_value);

    let scorecard = SecurityScorecardDocument {
        scorecard_id: scorecard_ref.clone(),
        contract_version: "security_scorecard.v1".to_string(),
        document_type: "security_scorecard".to_string(),
        generated_at: "2026-04-16T09:10:00+08:00".to_string(),
        symbol: "601916.SH".to_string(),
        analysis_date: "2026-04-16".to_string(),
        decision_id: decision_id.clone(),
        decision_ref: decision_ref.clone(),
        approval_ref: approval_ref.clone(),
        score_status: "ready".to_string(),
        label_definition: "horizon_10d_stop_5pct_target_10pct".to_string(),
        model_binding: SecurityScorecardModelBinding {
            model_id: Some("model-001".to_string()),
            model_version: Some("v1".to_string()),
            training_window: Some("2024-01-01..2025-12-31".to_string()),
            oot_window: Some("2026-01-01..2026-03-31".to_string()),
            target_label_definition: Some("positive_return_10d".to_string()),
            positive_label_definition: Some("10d_profit".to_string()),
            instrument_subscope: Some("a_share_bank".to_string()),
            binning_version: Some("bin-v1".to_string()),
            coefficient_version: Some("coef-v1".to_string()),
            model_sha256: Some("sha256-fixture".to_string()),
        },
        raw_feature_snapshot: BTreeMap::from([("trend_strength".to_string(), json!(0.82))]),
        feature_contributions: Vec::<SecurityScoreFeatureContribution>::new(),
        group_breakdown: Vec::<SecurityScoreGroupBreakdown>::new(),
        base_score: Some(600.0),
        total_score: Some(680.0),
        success_probability: Some(0.71),
        quant_signal: "supportive".to_string(),
        quant_stance: "build".to_string(),
        recommendation_action: "buy".to_string(),
        exposure_side: "long".to_string(),
        score_summary: "fixture scorecard".to_string(),
        limitations: Vec::new(),
    };
    let scorecard_value = serde_json::to_value(&scorecard).expect("scorecard should serialize");
    let scorecard_path = create_json_fixture(prefix, "scorecard.json", &scorecard_value);

    let post_meeting = SecurityPostMeetingConclusionDocument {
        post_meeting_conclusion_id: "post-meeting-decision-601916-SH-2026-04-16".to_string(),
        contract_version: "security_post_meeting_conclusion.v1".to_string(),
        document_type: "security_post_meeting_conclusion".to_string(),
        generated_at: "2026-04-16T09:20:00+08:00".to_string(),
        symbol: "601916.SH".to_string(),
        analysis_date: "2026-04-16".to_string(),
        decision_id: decision_id.clone(),
        chair_resolution_ref: chair_resolution_ref.clone(),
        final_action: "buy".to_string(),
        chair_process_action: "approve_execution".to_string(),
        final_trading_stance: "bullish".to_string(),
        final_exposure_side: "long".to_string(),
        final_confidence: 0.82,
        revision_required: false,
        return_to_stage: None,
        execution_notes: vec!["keep the plan".to_string()],
        follow_up_actions: vec!["track next disclosure".to_string()],
        conclusion_summary: "fixture post meeting".to_string(),
    };
    let post_meeting_value =
        serde_json::to_value(&post_meeting).expect("post meeting should serialize");
    let post_meeting_path =
        create_json_fixture(prefix, "post_meeting_conclusion.json", &post_meeting_value);

    let artifact_manifest = vec![
        build_fixture_artifact("decision_card", &decision_card_path, &decision_card_value),
        build_fixture_artifact(
            "approval_request",
            &approval_request_path,
            &approval_request_value,
        ),
        build_fixture_artifact("position_plan", &position_plan_path, &position_plan_value),
        build_fixture_artifact(
            "approval_brief",
            &approval_brief_path,
            &approval_brief_value,
        ),
        build_fixture_artifact("security_scorecard", &scorecard_path, &scorecard_value),
        build_fixture_artifact(
            "security_post_meeting_conclusion",
            &post_meeting_path,
            &post_meeting_value,
        ),
    ];

    let package = build_security_decision_package(SecurityDecisionPackageBuildInput {
        created_at: "2026-04-16T09:30:00+08:00".to_string(),
        package_version: 1,
        previous_package_path: None,
        revision_reason: "initial_submission".to_string(),
        trigger_event_summary: "fixture package".to_string(),
        scene_name: "security_review".to_string(),
        decision_id,
        decision_ref,
        approval_ref,
        symbol: "601916.SH".to_string(),
        analysis_date: "2026-04-16".to_string(),
        decision_status: "ready_for_review".to_string(),
        approval_status: "Pending".to_string(),
        model_grade_summary: None,
        model_governance_summary: None,
        lifecycle_governance_summary: None,
        position_plan_ref,
        approval_brief_ref,
        scorecard_ref,
        chair_resolution_ref: Some(chair_resolution_ref.clone()),
        condition_review_ref: None,
        execution_record_ref: None,
        post_trade_review_ref: None,
        decision_card_path: decision_card_path.to_string_lossy().to_string(),
        approval_request_path: approval_request_path.to_string_lossy().to_string(),
        position_plan_path: position_plan_path.to_string_lossy().to_string(),
        approval_brief_path: approval_brief_path.to_string_lossy().to_string(),
        scorecard_path: scorecard_path.to_string_lossy().to_string(),
        condition_review_path: None,
        execution_record_path: None,
        post_trade_review_path: None,
        evidence_hash: "evidence-hash".to_string(),
        governance_hash: "governance-hash".to_string(),
        artifact_manifest,
    });
    let package_runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scenes_runtime");
    let package_dir = package_runtime_root.join("decision_packages");
    fs::create_dir_all(&package_dir).expect("decision package directory should exist");
    let package_path = package_dir.join("decision-601916-SH-2026-04-16.json");
    fs::write(
        &package_path,
        serde_json::to_vec_pretty(&package).expect("package should serialize"),
    )
    .expect("package fixture should be written");

    (package_path, chair_resolution_ref)
}

// 2026-04-16 CST: Reason=keep the fixture manifest sha generation aligned with
// the package builder hash semantics.
// Purpose=avoid introducing unrelated hash-noise into a chair-binding tamper test.
fn build_fixture_artifact(
    artifact_role: &str,
    path: &Path,
    value: &Value,
) -> SecurityDecisionPackageArtifact {
    SecurityDecisionPackageArtifact {
        artifact_role: artifact_role.to_string(),
        path: path.to_string_lossy().to_string(),
        sha256: sha256_for_json_value(value).expect("fixture artifact sha should compute"),
        contract_version: value["contract_version"]
            .as_str()
            .unwrap_or("fixture.v1")
            .to_string(),
        required: true,
        present: true,
    }
}

// 2026-04-02 CST: 这里复用本地 HTTP 假服务，原因是 verify 测试仍然需要先跑 submit_approval 生成真实 package；
// 目的：把财报和公告依赖继续限制在本地可控夹具里，保证 package 验证测试稳定可重放。
fn spawn_http_route_server(routes: Vec<(&str, &str, &str, &str)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test http server should bind");
    let address = format!(
        "http://{}",
        listener
            .local_addr()
            .expect("test http server should have local addr")
    );
    let route_map: HashMap<String, (String, String, String)> = routes
        .into_iter()
        .map(|(path, status_line, body, content_type)| {
            (
                path.to_string(),
                (
                    status_line.to_string(),
                    body.to_string(),
                    content_type.to_string(),
                ),
            )
        })
        .collect();

    thread::spawn(move || {
        for _ in 0..route_map.len() {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            let mut buffer = [0_u8; 4096];
            let _ = stream.read(&mut buffer);
            let request_text = String::from_utf8_lossy(&buffer);
            let request_line = request_text.lines().next().unwrap_or_default();
            let request_path = request_line
                .split_whitespace()
                .nth(1)
                .unwrap_or("/")
                .split('?')
                .next()
                .unwrap_or("/");
            let (status_line, body, content_type) =
                route_map.get(request_path).cloned().unwrap_or_else(|| {
                    (
                        "HTTP/1.1 404 Not Found".to_string(),
                        "{\"error\":\"not found\"}".to_string(),
                        "application/json".to_string(),
                    )
                });
            let response = format!(
                "{status_line}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });

    address
}

#[test]
fn tool_catalog_includes_security_decision_verify_package() {
    let output = run_cli_with_json("");

    // 2026-04-02 CST: 这里先锁住 verify Tool 的可发现性，原因是 package 校验如果不进 catalog，就无法进入正式产品主链；
    // 目的：确保 CLI / Skill / 后续自动化都能稳定发现“审批包校验”入口。
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_decision_verify_package")
    );
}

#[test]
fn security_decision_verify_package_accepts_signed_package_and_writes_report() {
    let runtime_db_path = create_test_runtime_db("security_decision_verify_package_signed");
    let approval_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scenes_runtime");

    let stock_csv = create_stock_history_csv(
        "security_decision_verify_package_signed",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_decision_verify_package_signed",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_decision_verify_package_signed",
        "sector.csv",
        &build_confirmed_breakout_rows(220, 950.0),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 200 OK",
            r#"[
                {
                    "REPORT_DATE":"2025-12-31",
                    "NOTICE_DATE":"2026-03-28",
                    "TOTAL_OPERATE_INCOME":308227000000.0,
                    "YSTZ":8.37,
                    "PARENT_NETPROFIT":11117000000.0,
                    "SJLTZ":9.31,
                    "ROEJQ":14.8
                }
            ]"#,
            "application/json",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{
                "data":{
                    "list":[
                        {"notice_date":"2026-03-28","title":"2025年年度报告","art_code":"AN202603281234567890","columns":[{"column_name":"定期报告"}]},
                        {"notice_date":"2026-03-28","title":"2025年度利润分配预案公告","art_code":"AN202603281234567891","columns":[{"column_name":"公司公告"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ]);

    let submit_request = json!({
        "tool": "security_decision_submit_approval",
        "args": {
            "symbol": "601916.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12,
            "approval_runtime_root": approval_root.to_string_lossy(),
            "created_at": "2026-04-02T16:30:00+08:00",
            "approval_brief_signing_key_id": "brief_signing_key_20260402",
            "approval_brief_signing_key_secret": "brief-secret-for-tests"
        }
    });

    let submit_output = run_cli_with_json_runtime_and_envs(
        &submit_request.to_string(),
        &runtime_db_path,
        &[
            (
                "EXCEL_SKILL_EASTMONEY_FINANCIAL_URL_BASE",
                format!("{server}/financials"),
            ),
            (
                "EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE",
                format!("{server}/announcements"),
            ),
        ],
    );
    let package_path = submit_output["data"]["decision_package_path"]
        .as_str()
        .expect("decision package path should exist")
        .to_string();

    let verify_request = json!({
        "tool": "security_decision_verify_package",
        "args": {
            "package_path": package_path,
            "approval_brief_signing_key_secret": "brief-secret-for-tests"
        }
    });
    let verify_output =
        run_cli_with_json_runtime_and_envs(&verify_request.to_string(), &runtime_db_path, &[]);

    // 2026-04-02 CST: 这里锁住 signed package 的 happy path，原因是 P0-5 的核心就是证明正式审批包已可系统校验；
    // 目的：确保 manifest、detached signature 和治理绑定同时通过，verification report 也能落盘。
    assert_eq!(verify_output["status"], "ok");
    assert_eq!(verify_output["data"]["package_valid"], true);
    assert_eq!(
        verify_output["data"]["recommended_action"],
        "proceed_with_review"
    );
    assert!(
        verify_output["data"]["verification_report_path"]
            .as_str()
            .expect("verification report path should exist")
            .contains("decision_packages_verification")
    );
    assert!(
        verify_output["data"]["artifact_checks"]
            .as_array()
            .expect("artifact checks should be array")
            .iter()
            .all(|item| item["exists_on_disk"] == true)
    );
    assert!(
        verify_output["data"]["hash_checks"]
            .as_array()
            .expect("hash checks should be array")
            .iter()
            .all(|item| item["matched"] == true)
    );
    assert!(
        verify_output["data"]["signature_checks"]
            .as_array()
            .expect("signature checks should be array")
            .iter()
            .any(|item| item["signature_valid"] == true)
    );
    // 2026-04-08 CST: 这里先锁定 verify 输出中的对象图一致性结果，原因是 Task 1 不仅要写入 object_graph，还要把它纳入正式校验；
    // 目的：确保后续 package 就算文件还在，也不能在对象引用漂移时被误判为有效。
    assert_eq!(
        verify_output["data"]["governance_checks"]["object_graph_consistent"],
        true
    );
    // 2026-04-08 CST: 这里先锁定仓位计划正式挂入审批链后的校验输出，原因是 Task 2 不仅要落盘 binding，还要让 verify 对其进行正式约束；
    // 目的：确保后续审批链对仓位计划的引用、完整性和方向一致性都能被稳定验证，而不是只验证文件存在。
    assert_eq!(
        verify_output["data"]["governance_checks"]["position_plan_binding_consistent"],
        true
    );
    assert_eq!(
        verify_output["data"]["governance_checks"]["position_plan_complete"],
        true
    );
    assert_eq!(
        verify_output["data"]["governance_checks"]["position_plan_direction_aligned"],
        true
    );
    // 2026-04-09 CST: 这里补锁 scorecard 治理校验的 happy path，原因是本轮不只是把评分卡落盘，还要求它正式进入 package / verify 主链；
    // 目的：确保后续只要评分卡引用、完整性或动作语义漂移，verify 就能第一时间拦截，而不是只验证文件存在。
    assert_eq!(
        verify_output["data"]["governance_checks"]["scorecard_binding_consistent"],
        true
    );
    assert_eq!(
        verify_output["data"]["governance_checks"]["scorecard_complete"],
        true
    );
    assert_eq!(
        verify_output["data"]["governance_checks"]["scorecard_action_aligned"],
        true
    );

    let report_path = PathBuf::from(
        verify_output["data"]["verification_report_path"]
            .as_str()
            .expect("verification report path should exist"),
    );
    assert!(report_path.exists());
}

#[test]
fn security_decision_verify_package_fails_after_approval_brief_is_tampered() {
    let runtime_db_path = create_test_runtime_db("security_decision_verify_package_tampered");
    let approval_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scenes_runtime");

    let stock_csv = create_stock_history_csv(
        "security_decision_verify_package_tampered",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_decision_verify_package_tampered",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_decision_verify_package_tampered",
        "sector.csv",
        &build_confirmed_breakout_rows(220, 950.0),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 200 OK",
            r#"[
                {
                    "REPORT_DATE":"2025-12-31",
                    "NOTICE_DATE":"2026-03-28",
                    "TOTAL_OPERATE_INCOME":308227000000.0,
                    "YSTZ":8.37,
                    "PARENT_NETPROFIT":11117000000.0,
                    "SJLTZ":9.31,
                    "ROEJQ":14.8
                }
            ]"#,
            "application/json",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{
                "data":{
                    "list":[
                        {"notice_date":"2026-03-28","title":"2025年年度报告","art_code":"AN202603281234567890","columns":[{"column_name":"定期报告"}]},
                        {"notice_date":"2026-03-28","title":"2025年度利润分配预案公告","art_code":"AN202603281234567891","columns":[{"column_name":"公司公告"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ]);

    let submit_request = json!({
        "tool": "security_decision_submit_approval",
        "args": {
            "symbol": "601916.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12,
            "approval_runtime_root": approval_root.to_string_lossy(),
            "created_at": "2026-04-02T16:45:00+08:00",
            "approval_brief_signing_key_id": "brief_signing_key_20260402",
            "approval_brief_signing_key_secret": "brief-secret-for-tests"
        }
    });

    let submit_output = run_cli_with_json_runtime_and_envs(
        &submit_request.to_string(),
        &runtime_db_path,
        &[
            (
                "EXCEL_SKILL_EASTMONEY_FINANCIAL_URL_BASE",
                format!("{server}/financials"),
            ),
            (
                "EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE",
                format!("{server}/announcements"),
            ),
        ],
    );
    let package_path = submit_output["data"]["decision_package_path"]
        .as_str()
        .expect("decision package path should exist")
        .to_string();
    let approval_brief_path = submit_output["data"]["approval_brief_path"]
        .as_str()
        .expect("approval brief path should exist");

    let mut approval_brief: Value = serde_json::from_slice(
        &fs::read(approval_brief_path).expect("approval brief should be readable"),
    )
    .expect("approval brief should be valid json");
    approval_brief["executive_summary"] = Value::String("tampered-summary".to_string());
    fs::write(
        approval_brief_path,
        serde_json::to_vec_pretty(&approval_brief).expect("tampered brief should serialize"),
    )
    .expect("tampered approval brief should be written");

    let verify_request = json!({
        "tool": "security_decision_verify_package",
        "args": {
            "package_path": package_path,
            "approval_brief_signing_key_secret": "brief-secret-for-tests"
        }
    });
    let verify_output =
        run_cli_with_json_runtime_and_envs(&verify_request.to_string(), &runtime_db_path, &[]);

    // 2026-04-02 CST: 这里锁住篡改失败路径，原因是 package 校验不能只会处理“正常工件”，还必须能识别审批简报被改写；
    // 目的：确保 manifest 哈希和 detached signature 至少有一条会报警，从而阻断带毒审批包继续流转。
    assert_eq!(verify_output["status"], "ok");
    assert_eq!(verify_output["data"]["package_valid"], false);
    assert_eq!(
        verify_output["data"]["recommended_action"],
        "quarantine_and_rebuild"
    );
    assert!(
        verify_output["data"]["issues"]
            .as_array()
            .expect("issues should be array")
            .len()
            >= 1
    );
    assert!(
        verify_output["data"]["hash_checks"]
            .as_array()
            .expect("hash checks should be array")
            .iter()
            .any(|item| item["artifact_role"] == "approval_brief" && item["matched"] == false)
    );
}

#[test]
fn security_decision_verify_package_fails_after_object_graph_is_tampered() {
    let runtime_db_path =
        create_test_runtime_db("security_decision_verify_package_object_graph_tampered");
    let approval_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scenes_runtime");

    let stock_csv = create_stock_history_csv(
        "security_decision_verify_package_object_graph_tampered",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_decision_verify_package_object_graph_tampered",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_decision_verify_package_object_graph_tampered",
        "sector.csv",
        &build_confirmed_breakout_rows(220, 950.0),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 200 OK",
            r#"[
                {
                    "REPORT_DATE":"2025-12-31",
                    "NOTICE_DATE":"2026-03-28",
                    "TOTAL_OPERATE_INCOME":308227000000.0,
                    "YSTZ":8.37,
                    "PARENT_NETPROFIT":11117000000.0,
                    "SJLTZ":9.31,
                    "ROEJQ":14.8
                }
            ]"#,
            "application/json",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{
                "data":{
                    "list":[
                        {"notice_date":"2026-03-28","title":"2025骞村勾搴︽姤鍛?,"art_code":"AN202603281234567890","columns":[{"column_name":"瀹氭湡鎶ュ憡"}]},
                        {"notice_date":"2026-03-28","title":"2025骞村害鍒╂鼎鍒嗛厤棰勬鍏憡","art_code":"AN202603281234567891","columns":[{"column_name":"鍏徃鍏憡"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ]);

    let submit_request = json!({
        "tool": "security_decision_submit_approval",
        "args": {
            "symbol": "601916.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12,
            "approval_runtime_root": approval_root.to_string_lossy(),
            "created_at": "2026-04-08T10:10:00+08:00",
            "approval_brief_signing_key_id": "brief_signing_key_20260408",
            "approval_brief_signing_key_secret": "brief-secret-for-tests"
        }
    });
    let submit_output = run_cli_with_json_runtime_and_envs(
        &submit_request.to_string(),
        &runtime_db_path,
        &[
            (
                "EXCEL_SKILL_EASTMONEY_FINANCIAL_URL_BASE",
                format!("{server}/financials"),
            ),
            (
                "EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE",
                format!("{server}/announcements"),
            ),
        ],
    );

    let package_path = submit_output["data"]["decision_package_path"]
        .as_str()
        .expect("decision package path should exist");
    rewrite_package_json(Path::new(package_path), |package_json| {
        package_json["object_graph"]["approval_brief_path"] =
            Value::String("tampered/approval_brief.json".to_string());
    });

    let verify_request = json!({
        "tool": "security_decision_verify_package",
        "args": {
            "package_path": package_path,
            "approval_brief_signing_key_secret": "brief-secret-for-tests"
        }
    });
    let verify_output =
        run_cli_with_json_runtime_and_envs(&verify_request.to_string(), &runtime_db_path, &[]);

    // 2026-04-08 CST: 这里锁定 object_graph 被篡改后的失败路径，原因是 Task 1 的核心就是“对象图本身也属于正式合同”；
    // 目的：确保 package 即便文件和 hash 仍可读，只要对象图路径与真实 artifact 清单不一致，也会被 verify 明确判为无效。
    assert_eq!(verify_output["status"], "ok");
    assert_eq!(verify_output["data"]["package_valid"], false);
    assert_eq!(
        verify_output["data"]["governance_checks"]["object_graph_consistent"],
        false
    );
    assert!(
        verify_output["data"]["issues"]
            .as_array()
            .expect("issues should be array")
            .iter()
            .any(|item| item
                .as_str()
                .expect("issue should be string")
                .contains("object_graph"))
    );
}

#[test]
fn security_decision_verify_package_fails_after_position_plan_binding_is_tampered() {
    let runtime_db_path =
        create_test_runtime_db("security_decision_verify_package_position_binding_tampered");
    let approval_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scenes_runtime");

    let stock_csv = create_stock_history_csv(
        "security_decision_verify_package_position_binding_tampered",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_decision_verify_package_position_binding_tampered",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_decision_verify_package_position_binding_tampered",
        "sector.csv",
        &build_confirmed_breakout_rows(220, 950.0),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 200 OK",
            r#"[
                {
                    "REPORT_DATE":"2025-12-31",
                    "NOTICE_DATE":"2026-03-28",
                    "TOTAL_OPERATE_INCOME":308227000000.0,
                    "YSTZ":8.37,
                    "PARENT_NETPROFIT":11117000000.0,
                    "SJLTZ":9.31,
                    "ROEJQ":14.8
                }
            ]"#,
            "application/json",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{
                "data":{
                    "list":[
                        {"notice_date":"2026-03-28","title":"2025年度报告","art_code":"AN202603281234567890","columns":[{"column_name":"定期报告"}]},
                        {"notice_date":"2026-03-28","title":"2025年度利润分配预案公告","art_code":"AN202603281234567891","columns":[{"column_name":"公司公告"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ]);

    let submit_request = json!({
        "tool": "security_decision_submit_approval",
        "args": {
            "symbol": "601916.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12,
            "approval_runtime_root": approval_root.to_string_lossy(),
            "created_at": "2026-04-08T10:20:00+08:00",
            "approval_brief_signing_key_id": "brief_signing_key_20260408",
            "approval_brief_signing_key_secret": "brief-secret-for-tests"
        }
    });
    let submit_output = run_cli_with_json_runtime_and_envs(
        &submit_request.to_string(),
        &runtime_db_path,
        &[
            (
                "EXCEL_SKILL_EASTMONEY_FINANCIAL_URL_BASE",
                format!("{server}/financials"),
            ),
            (
                "EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE",
                format!("{server}/announcements"),
            ),
        ],
    );

    let package_path = submit_output["data"]["decision_package_path"]
        .as_str()
        .expect("decision package path should exist");
    let approval_request_path = submit_output["data"]["approval_request_path"]
        .as_str()
        .expect("approval request path should exist");
    let mut approval_request: Value = serde_json::from_slice(
        &fs::read(approval_request_path).expect("approval request should be readable"),
    )
    .expect("approval request should be valid json");
    // 2026-04-08 CST: 这里先补 binding 被篡改的红测，原因是 Task 2 的核心不是“有 position_plan 文件”而是“审批请求明确绑定哪个计划”；
    // 目的：确保只要 approval_request 对 plan 的正式引用漂移，verify 就会把整条审批链判为无效，而不是继续放行。
    approval_request["position_plan_binding"]["position_plan_ref"] =
        Value::String("tampered-plan-ref".to_string());
    fs::write(
        approval_request_path,
        serde_json::to_vec_pretty(&approval_request)
            .expect("tampered approval request should serialize"),
    )
    .expect("tampered approval request should be written");

    let verify_request = json!({
        "tool": "security_decision_verify_package",
        "args": {
            "package_path": package_path,
            "approval_brief_signing_key_secret": "brief-secret-for-tests"
        }
    });
    let verify_output =
        run_cli_with_json_runtime_and_envs(&verify_request.to_string(), &runtime_db_path, &[]);

    assert_eq!(verify_output["status"], "ok");
    assert_eq!(verify_output["data"]["package_valid"], false);
    assert_eq!(
        verify_output["data"]["governance_checks"]["position_plan_binding_consistent"],
        false
    );
    assert!(
        verify_output["data"]["issues"]
            .as_array()
            .expect("issues should be array")
            .iter()
            .any(|item| item
                .as_str()
                .expect("issue should be string")
                .contains("position_plan_binding"))
    );
}

#[test]
fn security_decision_verify_package_fails_after_position_plan_direction_is_tampered() {
    let runtime_db_path =
        create_test_runtime_db("security_decision_verify_package_position_direction_tampered");
    let approval_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scenes_runtime");

    let stock_csv = create_stock_history_csv(
        "security_decision_verify_package_position_direction_tampered",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_decision_verify_package_position_direction_tampered",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_decision_verify_package_position_direction_tampered",
        "sector.csv",
        &build_confirmed_breakout_rows(220, 950.0),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 200 OK",
            r#"[
                {
                    "REPORT_DATE":"2025-12-31",
                    "NOTICE_DATE":"2026-03-28",
                    "TOTAL_OPERATE_INCOME":308227000000.0,
                    "YSTZ":8.37,
                    "PARENT_NETPROFIT":11117000000.0,
                    "SJLTZ":9.31,
                    "ROEJQ":14.8
                }
            ]"#,
            "application/json",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{
                "data":{
                    "list":[
                        {"notice_date":"2026-03-28","title":"2025年度报告","art_code":"AN202603281234567890","columns":[{"column_name":"定期报告"}]},
                        {"notice_date":"2026-03-28","title":"2025年度利润分配预案公告","art_code":"AN202603281234567891","columns":[{"column_name":"公司公告"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ]);

    let submit_request = json!({
        "tool": "security_decision_submit_approval",
        "args": {
            "symbol": "601916.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12,
            "approval_runtime_root": approval_root.to_string_lossy(),
            "created_at": "2026-04-08T10:30:00+08:00",
            "approval_brief_signing_key_id": "brief_signing_key_20260408",
            "approval_brief_signing_key_secret": "brief-secret-for-tests"
        }
    });
    let submit_output = run_cli_with_json_runtime_and_envs(
        &submit_request.to_string(),
        &runtime_db_path,
        &[
            (
                "EXCEL_SKILL_EASTMONEY_FINANCIAL_URL_BASE",
                format!("{server}/financials"),
            ),
            (
                "EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE",
                format!("{server}/announcements"),
            ),
        ],
    );

    let package_path = submit_output["data"]["decision_package_path"]
        .as_str()
        .expect("decision package path should exist");
    let position_plan_path = submit_output["data"]["position_plan_path"]
        .as_str()
        .expect("position plan path should exist");
    let mut position_plan: Value = serde_json::from_slice(
        &fs::read(position_plan_path).expect("position plan should be readable"),
    )
    .expect("position plan should be valid json");
    // 2026-04-08 CST: 这里补 direction 被篡改的红测，原因是 Task 2 除了绑定 plan 本体，还要求 plan 方向与投决方向显式对齐；
    // 目的：确保即便 position_plan 文件仍存在，只要方向被改写，verify 也会稳定打回而不是误判为可继续审议。
    position_plan["plan_direction"] = Value::String("Short".to_string());
    fs::write(
        position_plan_path,
        serde_json::to_vec_pretty(&position_plan).expect("tampered position plan should serialize"),
    )
    .expect("tampered position plan should be written");

    let verify_request = json!({
        "tool": "security_decision_verify_package",
        "args": {
            "package_path": package_path,
            "approval_brief_signing_key_secret": "brief-secret-for-tests"
        }
    });
    let verify_output =
        run_cli_with_json_runtime_and_envs(&verify_request.to_string(), &runtime_db_path, &[]);

    assert_eq!(verify_output["status"], "ok");
    assert_eq!(verify_output["data"]["package_valid"], false);
    assert_eq!(
        verify_output["data"]["governance_checks"]["position_plan_direction_aligned"],
        false
    );
    assert!(
        verify_output["data"]["issues"]
            .as_array()
            .expect("issues should be array")
            .iter()
            .any(|item| item
                .as_str()
                .expect("issue should be string")
                .contains("position_plan direction"))
    );
}

#[test]
fn security_decision_verify_package_fails_after_scorecard_action_is_tampered() {
    let runtime_db_path =
        create_test_runtime_db("security_decision_verify_package_scorecard_action_tampered");
    let approval_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scenes_runtime");

    let stock_csv = create_stock_history_csv(
        "security_decision_verify_package_scorecard_action_tampered",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_decision_verify_package_scorecard_action_tampered",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_decision_verify_package_scorecard_action_tampered",
        "sector.csv",
        &build_confirmed_breakout_rows(220, 950.0),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 200 OK",
            r#"[
                {
                    "REPORT_DATE":"2025-12-31",
                    "NOTICE_DATE":"2026-03-28",
                    "TOTAL_OPERATE_INCOME":308227000000.0,
                    "YSTZ":8.37,
                    "PARENT_NETPROFIT":11117000000.0,
                    "SJLTZ":9.31,
                    "ROEJQ":14.8
                }
            ]"#,
            "application/json",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{
                "data":{
                    "list":[
                        {"notice_date":"2026-03-28","title":"2025年年度报告","art_code":"AN202603281234567890","columns":[{"column_name":"定期报告"}]},
                        {"notice_date":"2026-03-28","title":"2025年度利润分配预案公告","art_code":"AN202603281234567891","columns":[{"column_name":"公司公告"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ]);

    let submit_request = json!({
        "tool": "security_decision_submit_approval",
        "args": {
            "symbol": "601916.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12,
            "approval_runtime_root": approval_root.to_string_lossy(),
            "created_at": "2026-04-09T11:10:00+08:00",
            "approval_brief_signing_key_id": "brief_signing_key_20260409",
            "approval_brief_signing_key_secret": "brief-secret-for-tests"
        }
    });
    let submit_output = run_cli_with_json_runtime_and_envs(
        &submit_request.to_string(),
        &runtime_db_path,
        &[
            (
                "EXCEL_SKILL_EASTMONEY_FINANCIAL_URL_BASE",
                format!("{server}/financials"),
            ),
            (
                "EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE",
                format!("{server}/announcements"),
            ),
        ],
    );

    let package_path = submit_output["data"]["decision_package_path"]
        .as_str()
        .expect("decision package path should exist");
    let scorecard_path = submit_output["data"]["scorecard_path"]
        .as_str()
        .expect("scorecard path should exist");
    let mut scorecard: Value =
        serde_json::from_slice(&fs::read(scorecard_path).expect("scorecard should be readable"))
            .expect("scorecard should be valid json");
    // 2026-04-09 CST: 这里补 scorecard 动作被篡改的红测，原因是评分卡进入正式治理链后，必须和 decision_card 的动作语义保持一致；
    // 目的：确保就算篡改者不动 decision_card，只改评分卡，也会被 verify 明确识别并打回。
    scorecard["recommendation_action"] = Value::String("__tampered__".to_string());
    scorecard["exposure_side"] = Value::String("neutral".to_string());
    fs::write(
        scorecard_path,
        serde_json::to_vec_pretty(&scorecard).expect("tampered scorecard should serialize"),
    )
    .expect("tampered scorecard should be written");

    let verify_request = json!({
        "tool": "security_decision_verify_package",
        "args": {
            "package_path": package_path,
            "approval_brief_signing_key_secret": "brief-secret-for-tests"
        }
    });
    let verify_output =
        run_cli_with_json_runtime_and_envs(&verify_request.to_string(), &runtime_db_path, &[]);

    assert_eq!(verify_output["status"], "ok");
    assert_eq!(verify_output["data"]["package_valid"], false);
    assert_eq!(
        verify_output["data"]["governance_checks"]["scorecard_binding_consistent"],
        true
    );
    assert_eq!(
        verify_output["data"]["governance_checks"]["scorecard_action_aligned"],
        false
    );
    assert!(
        verify_output["data"]["issues"]
            .as_array()
            .expect("issues should be array")
            .iter()
            .any(|item| item
                .as_str()
                .expect("issue should be string")
                .contains("security_scorecard action"))
    );
}

#[test]
fn security_decision_verify_package_fails_after_scorecard_identity_binding_is_tampered() {
    let runtime_db_path =
        create_test_runtime_db("security_decision_verify_package_scorecard_identity_tampered");
    let approval_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scenes_runtime");

    let stock_csv = create_stock_history_csv(
        "security_decision_verify_package_scorecard_identity_tampered",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_decision_verify_package_scorecard_identity_tampered",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_decision_verify_package_scorecard_identity_tampered",
        "sector.csv",
        &build_confirmed_breakout_rows(220, 950.0),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 200 OK",
            r#"[
                {
                    "REPORT_DATE":"2025-12-31",
                    "NOTICE_DATE":"2026-03-28",
                    "TOTAL_OPERATE_INCOME":308227000000.0,
                    "YSTZ":8.37,
                    "PARENT_NETPROFIT":11117000000.0,
                    "SJLTZ":9.31,
                    "ROEJQ":14.8
                }
            ]"#,
            "application/json",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{
                "data":{
                    "list":[
                        {"notice_date":"2026-03-28","title":"2025骞村勾搴︽姤鍛?,"art_code":"AN202603281234567890","columns":[{"column_name":"瀹氭湡鎶ュ憡"}]},
                        {"notice_date":"2026-03-28","title":"2025骞村害鍒╂鼎鍒嗛厤棰勬鍏憡","art_code":"AN202603281234567891","columns":[{"column_name":"鍏徃鍏憡"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ]);

    let submit_request = json!({
        "tool": "security_decision_submit_approval",
        "args": {
            "symbol": "601916.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12,
            "approval_runtime_root": approval_root.to_string_lossy(),
            "created_at": "2026-04-09T11:40:00+08:00",
            "approval_brief_signing_key_id": "brief_signing_key_20260409",
            "approval_brief_signing_key_secret": "brief-secret-for-tests"
        }
    });
    let submit_output = run_cli_with_json_runtime_and_envs(
        &submit_request.to_string(),
        &runtime_db_path,
        &[
            (
                "EXCEL_SKILL_EASTMONEY_FINANCIAL_URL_BASE",
                format!("{server}/financials"),
            ),
            (
                "EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE",
                format!("{server}/announcements"),
            ),
        ],
    );

    let package_path = submit_output["data"]["decision_package_path"]
        .as_str()
        .expect("decision package path should exist");
    let scorecard_path = submit_output["data"]["scorecard_path"]
        .as_str()
        .expect("scorecard path should exist");
    let mut scorecard: Value =
        serde_json::from_slice(&fs::read(scorecard_path).expect("scorecard should be readable"))
            .expect("scorecard should be valid json");
    // 2026-04-16 CST: Reason=lock the freshly reviewed governance blind spot where
    // verify_package previously ignored scorecard identity metadata drift.
    // Purpose=prove tampering scorecard symbol/date/decision_id now invalidates package verification.
    scorecard["symbol"] = Value::String("000001.SH".to_string());
    scorecard["analysis_date"] = Value::String("2026-04-17".to_string());
    scorecard["decision_id"] = Value::String("decision-000001-SH-2026-04-17".to_string());
    fs::write(
        scorecard_path,
        serde_json::to_vec_pretty(&scorecard).expect("tampered scorecard should serialize"),
    )
    .expect("tampered scorecard should be written");

    let verify_request = json!({
        "tool": "security_decision_verify_package",
        "args": {
            "package_path": package_path,
            "approval_brief_signing_key_secret": "brief-secret-for-tests"
        }
    });
    let verify_output =
        run_cli_with_json_runtime_and_envs(&verify_request.to_string(), &runtime_db_path, &[]);

    assert_eq!(verify_output["status"], "ok");
    assert_eq!(verify_output["data"]["package_valid"], false);
    assert_eq!(
        verify_output["data"]["governance_checks"]["scorecard_binding_consistent"],
        false
    );
    assert!(
        verify_output["data"]["issues"]
            .as_array()
            .expect("issues should be array")
            .iter()
            .any(|item| item
                .as_str()
                .expect("issue should be string")
                .contains("security_scorecard binding mismatch"))
    );
}

#[test]
fn security_decision_verify_package_fails_after_chair_resolution_ref_is_tampered() {
    let runtime_db_path =
        create_test_runtime_db("security_decision_verify_package_chair_ref_tampered");
    let (package_path, chair_resolution_ref) = create_minimal_verifiable_package_fixture(
        &runtime_db_path,
        "security_decision_verify_package_chair_ref_tampered",
    );

    // 2026-04-16 CST: Reason=add the smallest possible tamper regression on the
    // explicit chair node after the user approved the conservative governance path.
    // Purpose=prove verify_package rejects post-meeting artifacts whose chair anchor
    // no longer matches the package object graph.
    rewrite_package_json(&package_path, |package_json| {
        package_json["object_graph"]["chair_resolution_ref"] =
            Value::String(format!("{chair_resolution_ref}::tampered"));
    });

    let verify_request = json!({
        "tool": "security_decision_verify_package",
        "args": {
            "package_path": package_path.to_string_lossy()
        }
    });
    let verify_output =
        run_cli_with_json_runtime_and_envs(&verify_request.to_string(), &runtime_db_path, &[]);

    assert_eq!(verify_output["status"], "ok");
    assert_eq!(verify_output["data"]["package_valid"], false);
    assert_eq!(
        verify_output["data"]["governance_checks"]["post_meeting_binding_consistent"],
        false
    );
    assert_eq!(
        verify_output["data"]["governance_checks"]["object_graph_consistent"],
        true
    );
    assert!(
        verify_output["data"]["issues"]
            .as_array()
            .expect("issues should be array")
            .iter()
            .any(|item| item
                .as_str()
                .expect("issue should be string")
                .contains("security_post_meeting_conclusion binding mismatch"))
    );
}

#[test]
fn security_decision_verify_package_fails_after_post_meeting_chair_ref_is_tampered() {
    let runtime_db_path =
        create_test_runtime_db("security_decision_verify_package_post_meeting_chair_tampered");
    let (package_path, chair_resolution_ref) = create_minimal_verifiable_package_fixture(
        &runtime_db_path,
        "security_decision_verify_package_post_meeting_chair_tampered",
    );

    let post_meeting_path = resolve_post_meeting_artifact_path(&package_path);
    // 2026-04-16 CST: Reason=add the paired tamper regression on the persisted
    // post-meeting document after locking the package-side chair anchor case.
    // Purpose=prove verify_package also catches downstream chair-binding drift
    // when the artifact content changes but the package object graph stays intact.
    rewrite_post_meeting_artifact(&post_meeting_path, |post_meeting_json| {
        post_meeting_json["chair_resolution_ref"] =
            Value::String(format!("{chair_resolution_ref}::tampered"));
    });

    let verify_request = json!({
        "tool": "security_decision_verify_package",
        "args": {
            "package_path": package_path.to_string_lossy()
        }
    });
    let verify_output =
        run_cli_with_json_runtime_and_envs(&verify_request.to_string(), &runtime_db_path, &[]);

    assert_eq!(verify_output["status"], "ok");
    assert_eq!(verify_output["data"]["package_valid"], false);
    assert_eq!(
        verify_output["data"]["governance_checks"]["post_meeting_binding_consistent"],
        false
    );
    assert_eq!(
        verify_output["data"]["governance_checks"]["object_graph_consistent"],
        true
    );
    assert!(
        verify_output["data"]["issues"]
            .as_array()
            .expect("issues should be array")
            .iter()
            .any(|item| item
                .as_str()
                .expect("issue should be string")
                .contains("security_post_meeting_conclusion binding mismatch"))
    );
}

#[test]
fn security_decision_verify_package_fails_after_post_meeting_chair_ref_is_cleared() {
    let runtime_db_path =
        create_test_runtime_db("security_decision_verify_package_post_meeting_chair_cleared");
    let (package_path, _) = create_minimal_verifiable_package_fixture(
        &runtime_db_path,
        "security_decision_verify_package_post_meeting_chair_cleared",
    );

    let post_meeting_path = resolve_post_meeting_artifact_path(&package_path);
    // 2026-04-16 CST: Reason=cover the empty-chair branch in the same verify
    // governance boundary after locking mismatch-style post-meeting regressions.
    // Purpose=prove verify_package rejects a post-meeting artifact whose chair
    // binding was silently cleared instead of rewritten to another value.
    rewrite_post_meeting_artifact(&post_meeting_path, |post_meeting_json| {
        post_meeting_json["chair_resolution_ref"] = Value::String(String::new());
    });

    let verify_request = json!({
        "tool": "security_decision_verify_package",
        "args": {
            "package_path": package_path.to_string_lossy()
        }
    });
    let verify_output =
        run_cli_with_json_runtime_and_envs(&verify_request.to_string(), &runtime_db_path, &[]);

    assert_eq!(verify_output["status"], "ok");
    assert_eq!(verify_output["data"]["package_valid"], false);
    assert_eq!(
        verify_output["data"]["governance_checks"]["post_meeting_binding_consistent"],
        false
    );
    assert_eq!(
        verify_output["data"]["governance_checks"]["object_graph_consistent"],
        true
    );
    assert!(
        verify_output["data"]["issues"]
            .as_array()
            .expect("issues should be array")
            .iter()
            .any(|item| item
                .as_str()
                .expect("issue should be string")
                .contains("security_post_meeting_conclusion binding mismatch"))
    );
}

#[test]
fn security_decision_verify_package_fails_after_post_meeting_artifact_file_is_missing() {
    let runtime_db_path =
        create_test_runtime_db("security_decision_verify_package_post_meeting_file_missing");
    let (package_path, _) = create_minimal_verifiable_package_fixture(
        &runtime_db_path,
        "security_decision_verify_package_post_meeting_file_missing",
    );

    let post_meeting_path = resolve_post_meeting_artifact_path(&package_path);
    // 2026-04-16 CST: Reason=cover the missing-file branch for the governed
    // post-meeting artifact after locking content-level chair binding failures.
    // Purpose=prove verify_package rejects a package whose manifest still points
    // at post-meeting governance output that no longer exists on disk.
    fs::remove_file(&post_meeting_path).expect("post meeting artifact should be removable");

    let verify_request = json!({
        "tool": "security_decision_verify_package",
        "args": {
            "package_path": package_path.to_string_lossy()
        }
    });
    let verify_output =
        run_cli_with_json_runtime_and_envs(&verify_request.to_string(), &runtime_db_path, &[]);

    assert_eq!(verify_output["status"], "ok");
    assert_eq!(verify_output["data"]["package_valid"], false);
    assert_eq!(
        verify_output["data"]["recommended_action"],
        "quarantine_and_rebuild"
    );
    assert!(
        verify_output["data"]["artifact_checks"]
            .as_array()
            .expect("artifact checks should be array")
            .iter()
            .any(|item| {
                item["artifact_role"] == "security_post_meeting_conclusion"
                    && item["exists_on_disk"] == false
            })
    );
    assert!(
        verify_output["data"]["issues"]
            .as_array()
            .expect("issues should be array")
            .iter()
            .any(|item| item
                .as_str()
                .expect("issue should be string")
                .contains("security_post_meeting_conclusion"))
    );
}

fn resolve_post_meeting_artifact_path(package_path: &Path) -> PathBuf {
    let package_json: Value = serde_json::from_slice(
        &fs::read(package_path).expect("decision package should be readable"),
    )
    .expect("decision package should be valid json");
    PathBuf::from(
        package_json["artifact_manifest"]
            .as_array()
            .expect("artifact manifest should be array")
            .iter()
            .find(|artifact| artifact["artifact_role"] == "security_post_meeting_conclusion")
            .and_then(|artifact| artifact["path"].as_str())
            .expect("post meeting artifact path should exist"),
    )
}

fn rewrite_post_meeting_artifact(post_meeting_path: &Path, mutate: impl FnOnce(&mut Value)) {
    let mut post_meeting_json: Value = serde_json::from_slice(
        &fs::read(post_meeting_path).expect("post meeting artifact should be readable"),
    )
    .expect("post meeting artifact should be valid json");
    mutate(&mut post_meeting_json);
    fs::write(
        post_meeting_path,
        serde_json::to_vec_pretty(&post_meeting_json)
            .expect("post meeting artifact should serialize"),
    )
    .expect("post meeting artifact should be written");
}

fn rewrite_package_json(package_path: &Path, mutate: impl FnOnce(&mut Value)) {
    let mut package_json: Value = serde_json::from_slice(
        &fs::read(package_path).expect("decision package should be readable"),
    )
    .expect("decision package should be valid json");
    mutate(&mut package_json);
    fs::write(
        package_path,
        serde_json::to_vec_pretty(&package_json).expect("decision package should serialize"),
    )
    .expect("decision package should be written");
}

fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_decision_verify_package_fixture"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path.to_path_buf(),
        &[],
    );
    assert_eq!(output["status"], "ok");
}

fn build_confirmed_breakout_rows(day_count: usize, start_close: f64) -> Vec<String> {
    let mut rows = vec!["trade_date,open,high,low,close,adj_close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("seed date should be valid");
    let mut close = start_close;

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let (next_close, volume): (f64, i64) = if offset < day_count - 20 {
            (close + 0.78, 880_000 + offset as i64 * 8_000)
        } else {
            let phase = offset - (day_count - 20);
            match phase % 4 {
                0 => (close + 1.35, 1_700_000 + phase as i64 * 26_000),
                1 => (close - 0.18, 420_000),
                2 => (close + 1.08, 1_540_000 + phase as i64 * 22_000),
                _ => (close + 0.42, 1_240_000),
            }
        };

        let open = close;
        let high = next_close.max(open) + 1.0;
        let low = next_close.min(open) - 0.86;
        let adj_close = next_close;
        rows.push(format!(
            "{},{open:.2},{high:.2},{low:.2},{next_close:.2},{adj_close:.2},{volume}",
            trade_date.format("%Y-%m-%d")
        ));
        close = next_close;
    }

    rows
}
