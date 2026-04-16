mod common;

use chrono::{Duration, NaiveDate};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{
    create_test_runtime_db, run_cli_with_json, run_cli_with_json_and_runtime,
    run_cli_with_json_runtime_and_envs,
};
use excel_skill::ops::stock::security_decision_package::{
    SecurityDecisionPackageArtifact, SecurityDecisionPackageBuildInput,
    build_security_decision_package,
};

// 2026-04-02 CST: 杩欓噷鏂板 package revision 娴嬭瘯澶瑰叿锛屽師鍥犳槸 P0-6 鐨勬牳蹇冩槸鈥滃鎵瑰寘璺熺潃瀹℃壒鍔ㄤ綔鐢熸垚鏂扮増鏈€濓紱
// 鐩殑锛氭妸 v1 package -> 鏇存柊瀹℃壒宸ヤ欢 -> 鐢熸垚 v2 package 鐨勬渶灏忛棴鐜攣杩涚嫭绔嬫祴璇曘€?
fn create_stock_history_csv(prefix: &str, file_name: &str, rows: &[String]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_decision_package_revision")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir)
        .expect("security decision package revision fixture dir should exist");

    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n"))
        .expect("security decision package revision csv should be written");
    csv_path
}

// 2026-04-12 CST: Add a reusable JSON fixture helper, because P8-4 needs
// lifecycle documents to be attached during package revision without hand-editing
// runtime artifacts inline.
// Purpose: keep lifecycle-package tests readable while preserving stable per-test files.
fn create_json_fixture(prefix: &str, file_name: &str, value: &Value) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_decision_package_revision")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir)
        .expect("security decision package revision json fixture dir should exist");

    let json_path = fixture_dir.join(file_name);
    fs::write(
        &json_path,
        serde_json::to_vec_pretty(value).expect("json fixture should serialize"),
    )
    .expect("security decision package revision json fixture should be written");
    json_path
}

// 2026-04-16 CST: Reason=extract the shared post-meeting chair fixture used by
// the growing revision-governance regression family.
// Purpose=reduce duplicated chair artifact construction so later edge-case tests
// can stay focused on the binding rule they are proving.
fn create_post_meeting_chair_fixture(
    prefix: &str,
    chair_resolution_ref: &str,
    conclusion_summary: &str,
) -> PathBuf {
    create_json_fixture(
        prefix,
        "post_meeting_conclusion.json",
        &json!({
            "post_meeting_conclusion_id": "post-meeting-decision-601916-SH-2026-04-15",
            "contract_version": "security_post_meeting_conclusion.v1",
            "document_type": "security_post_meeting_conclusion",
            "generated_at": "2026-04-15T09:45:00+08:00",
            "symbol": "601916.SH",
            "analysis_date": "2026-04-15",
            "decision_id": "decision-601916-SH-2026-04-15",
            "chair_resolution_ref": chair_resolution_ref,
            "final_action": "buy",
            "chair_process_action": "approve_execution",
            "final_trading_stance": "bullish",
            "final_exposure_side": "long",
            "final_confidence": 0.82,
            "revision_required": false,
            "return_to_stage": null,
            "execution_notes": ["keep the approved execution rhythm"],
            "follow_up_actions": ["track the next disclosure window"],
            "conclusion_summary": conclusion_summary
        }),
    )
}

// 2026-04-16 CST: Reason=extract the minimal package builder shared by the
// chair-binding revision regressions.
// Purpose=keep package identity stable while each test varies only the chair
// anchor source and post-meeting artifact presence.
fn build_chair_revision_fixture_package(
    package_chair_resolution_ref: Option<&str>,
    post_meeting_artifact: SecurityDecisionPackageArtifact,
) -> Value {
    serde_json::to_value(build_security_decision_package(
        SecurityDecisionPackageBuildInput {
            created_at: "2026-04-15T09:30:00+08:00".to_string(),
            package_version: 1,
            previous_package_path: None,
            revision_reason: "initial_submission".to_string(),
            trigger_event_summary: "initial package".to_string(),
            scene_name: "security_review".to_string(),
            decision_id: "decision-601916-SH-2026-04-15".to_string(),
            decision_ref: "decision:601916.SH:2026-04-15".to_string(),
            approval_ref: "approval:decision-601916-SH-2026-04-15".to_string(),
            symbol: "601916.SH".to_string(),
            analysis_date: "2026-04-15".to_string(),
            decision_status: "ready_for_review".to_string(),
            approval_status: "Pending".to_string(),
            model_grade_summary: None,
            model_governance_summary: None,
            lifecycle_governance_summary: None,
            position_plan_ref: "position-plan:601916.SH:2026-04-15".to_string(),
            approval_brief_ref: "approval-brief:601916.SH:2026-04-15".to_string(),
            scorecard_ref: "scorecard:601916.SH:2026-04-15".to_string(),
            chair_resolution_ref: package_chair_resolution_ref.map(|value| value.to_string()),
            condition_review_ref: None,
            execution_record_ref: None,
            post_trade_review_ref: None,
            decision_card_path: "artifacts/decision-card.json".to_string(),
            approval_request_path: "artifacts/approval-request.json".to_string(),
            position_plan_path: "artifacts/position-plan.json".to_string(),
            approval_brief_path: "artifacts/approval-brief.json".to_string(),
            scorecard_path: "artifacts/scorecard.json".to_string(),
            condition_review_path: None,
            execution_record_path: None,
            post_trade_review_path: None,
            evidence_hash: "evidence-hash".to_string(),
            governance_hash: "governance-hash".to_string(),
            artifact_manifest: vec![post_meeting_artifact],
        },
    ))
    .expect("chair revision fixture package should serialize")
}

// 2026-04-16 CST: Reason=centralize package fixture persistence for the chair
// revision regressions after the suite grew past one-off inline setup.
// Purpose=reduce path-writing duplication and keep each regression centered on
// the governance assertion instead of file plumbing.
fn write_chair_revision_fixture_package(
    runtime_db_path: &Path,
    package_file_name: &str,
    package_json: &Value,
) -> PathBuf {
    let package_runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scenes_runtime");
    let package_dir = package_runtime_root.join("decision_packages");
    fs::create_dir_all(&package_dir).expect("decision package directory should exist");
    let package_path = package_dir.join(package_file_name);
    fs::write(
        &package_path,
        serde_json::to_vec_pretty(package_json).expect("package should serialize"),
    )
    .expect("package fixture should be written");
    package_path
}

// 2026-04-16 CST: Reason=share the revision invocation boilerplate across the
// chair-binding regressions while preserving each test's custom revision reason.
// Purpose=let the tests focus on the resulting package binding instead of
// repeatedly reconstructing the CLI call shape.
fn run_chair_revision(
    runtime_db_path: &PathBuf,
    package_path: &Path,
    revision_reason: &str,
) -> Value {
    let revision_request = json!({
        "tool": "security_decision_package_revision",
        "args": {
            "package_path": package_path.to_string_lossy(),
            "revision_reason": revision_reason,
            "reverify_after_revision": false
        }
    });
    run_cli_with_json_runtime_and_envs(&revision_request.to_string(), runtime_db_path, &[])
}

// 2026-04-02 CST: 杩欓噷澶嶇敤鏈湴 HTTP 鍋囨湇鍔★紝鍘熷洜鏄?revision 娴嬭瘯浠嶇劧闇€瑕佸厛鐢熸垚鐪熷疄瀹℃壒鍖咃紱
// 鐩殑锛氫繚璇佽储鎶ュ拰鍏憡璇佹嵁绋冲畾鍙噸鏀撅紝涓嶈澶栭儴鎺ュ彛娉㈠姩骞叉壈 package 鐗堟湰鍖栨祴璇曘€?
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
fn tool_catalog_includes_security_decision_package_revision() {
    let output = run_cli_with_json("");

    // 2026-04-02 CST: 杩欓噷鍏堥攣浣?revision Tool 鐨勫彲鍙戠幇鎬э紝鍘熷洜鏄鎵瑰寘鐗堟湰鍖栧鏋滀笉杩?catalog锛屽氨鏃犳硶鎴愪负姝ｅ紡涓婚摼鑳藉姏锛?
    // 鐩殑锛氱‘淇?CLI / Skill / 鍚庣画鑷姩鍖栭兘鑳界ǔ瀹氬彂鐜扳€滅敓鎴愪笅涓€涓?package 鐗堟湰鈥濈殑鍏ュ彛銆?
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_decision_package_revision")
    );
}

#[test]
fn security_decision_package_revision_builds_v2_package_after_approval_update() {
    let runtime_db_path = create_test_runtime_db("security_decision_package_revision_v2");
    let approval_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scenes_runtime");

    let stock_csv = create_stock_history_csv(
        "security_decision_package_revision_v2",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_decision_package_revision_v2",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_decision_package_revision_v2",
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
            "created_at": "2026-04-02T18:00:00+08:00",
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

    let package_path = PathBuf::from(
        submit_output["data"]["decision_package_path"]
            .as_str()
            .expect("decision package path should exist"),
    );
    let approval_request_path = PathBuf::from(
        submit_output["data"]["approval_request_path"]
            .as_str()
            .expect("approval request path should exist"),
    );
    let approval_events_path = PathBuf::from(
        submit_output["data"]["approval_events_path"]
            .as_str()
            .expect("approval events path should exist"),
    );
    let audit_log_path = PathBuf::from(
        submit_output["data"]["audit_log_path"]
            .as_str()
            .expect("audit log path should exist"),
    );

    let mut approval_request: Value = serde_json::from_slice(
        &fs::read(&approval_request_path).expect("approval request should be readable"),
    )
    .expect("approval request should be valid json");
    approval_request["status"] = Value::String("Approved".to_string());
    approval_request["approved_reviewers"] = json!(["risk_officer", "pm_lead"]);
    approval_request["approved_signatures"] = json!([
        {
            "reviewer": "risk_officer",
            "reviewer_role": "RiskOfficer",
            "timestamp": "2026-04-02T18:20:00+08:00"
        },
        {
            "reviewer": "pm_lead",
            "reviewer_role": "PortfolioManager",
            "timestamp": "2026-04-02T18:22:00+08:00"
        }
    ]);
    fs::write(
        &approval_request_path,
        serde_json::to_vec_pretty(&approval_request).expect("approval request should serialize"),
    )
    .expect("approval request should be updated");

    let approval_events = json!([
        {
            "approval_id": approval_request["approval_ref"],
            "decision_id": approval_request["decision_id"],
            "reviewer": "risk_officer",
            "reviewer_role": "RiskOfficer",
            "action": "Approve",
            "timestamp": "2026-04-02T18:20:00+08:00",
            "notes": "risk cleared",
            "override_reason": null,
            "decision_version": 1
        },
        {
            "approval_id": approval_request["approval_ref"],
            "decision_id": approval_request["decision_id"],
            "reviewer": "pm_lead",
            "reviewer_role": "PortfolioManager",
            "action": "Approve",
            "timestamp": "2026-04-02T18:22:00+08:00",
            "notes": "pm approved",
            "override_reason": null,
            "decision_version": 2
        }
    ]);
    fs::write(
        &approval_events_path,
        serde_json::to_vec_pretty(&approval_events).expect("approval events should serialize"),
    )
    .expect("approval events should be updated");

    let mut audit_lines =
        fs::read_to_string(&audit_log_path).expect("audit log should be readable");
    audit_lines.push_str(
        "{\"event_type\":\"approval_action_applied\",\"timestamp\":\"2026-04-02T18:22:00+08:00\",\"decision_id\":\"");
    audit_lines.push_str(
        approval_request["decision_id"]
            .as_str()
            .expect("decision id should exist"),
    );
    audit_lines.push_str("\",\"decision_ref\":");
    audit_lines.push_str(&approval_request["decision_ref"].to_string());
    audit_lines.push_str(",\"approval_ref\":\"");
    audit_lines.push_str(
        approval_request["approval_ref"]
            .as_str()
            .expect("approval ref should exist"),
    );
    audit_lines.push_str("\",\"evidence_hash\":");
    audit_lines.push_str(&approval_request["evidence_hash"].to_string());
    audit_lines.push_str(",\"governance_hash\":");
    audit_lines.push_str(&approval_request["governance_hash"].to_string());
    audit_lines.push_str(",\"decision_status\":\"Approved\",\"approval_status\":\"Approved\",\"reviewer\":\"pm_lead\",\"reviewer_role\":\"PortfolioManager\",\"approval_action\":\"Approve\",\"notes\":\"pm approved\",\"override_reason\":null,\"decision_version\":2,\"signature_key_id\":null,\"signature_algorithm\":null,\"signature_path\":null,\"signed_payload_sha256\":null,\"signed_contract_version\":null,\"prev_hash\":null,\"record_hash\":null}\n");
    fs::write(&audit_log_path, audit_lines).expect("audit log should be updated");

    let revision_request = json!({
        "tool": "security_decision_package_revision",
        "args": {
            "package_path": package_path.to_string_lossy(),
            "revision_reason": "approval_event_applied",
            "reverify_after_revision": true,
            "approval_brief_signing_key_secret": "brief-secret-for-tests"
        }
    });
    let revision_output =
        run_cli_with_json_runtime_and_envs(&revision_request.to_string(), &runtime_db_path, &[]);

    // 2026-04-02 CST: 杩欓噷閿佷綇瀹℃壒鍔ㄤ綔鍚庣殑 v2 package 涓昏矾寰勶紝鍘熷洜鏄?P0-6 鐨勭洰鏍囧氨鏄瀹℃壒鍖呭紑濮嬪叿澶囨寮忕増鏈彶锛?
    // 鐩殑锛氱‘淇濇洿鏂板悗鐨勫鎵瑰伐浠朵細椹卞姩鏂?package 鐗堟湰鐢熸垚锛屽苟涓旇兘甯︿笂鍓嶇増鏈紩鐢ㄣ€佽Е鍙戞憳瑕佸拰鏂扮殑 verification report銆?
    assert_eq!(
        revision_output["status"], "ok",
        "security_decision_package_revision lifecycle output={revision_output}"
    );
    assert_eq!(revision_output["data"]["package_version"], 2);
    assert_eq!(
        revision_output["data"]["revision_reason"],
        "approval_event_applied"
    );
    assert_eq!(
        revision_output["data"]["previous_package_path"],
        Value::String(package_path.to_string_lossy().to_string())
    );
    assert_eq!(
        revision_output["data"]["decision_package"]["package_status"],
        "approved_bundle_ready"
    );
    // 2026-04-08 CST: 杩欓噷琛ュ厖 v2 package 瀵硅薄鍥炬柇瑷€锛屽師鍥犳槸 Task 1 鏂板鐨勬寮忓璞″浘涓嶈兘鍙瓨鍦ㄤ簬 v1 鍒濆鍖呴噷锛?
    // 鐩殑锛氱‘淇?revision 涔嬪悗鐨?package 浠嶄繚鐣?position_plan / approval_brief 鐨勬寮忓紩鐢紝涓嶄細鍦ㄧ増鏈寲鏃堕€€鍥為殣寮忓叧绯汇€?
    assert_eq!(
        revision_output["data"]["decision_package"]["object_graph"]["position_plan_ref"],
        submit_output["data"]["position_plan"]["plan_id"]
    );
    assert_eq!(
        revision_output["data"]["decision_package"]["object_graph"]["approval_brief_ref"],
        submit_output["data"]["approval_brief"]["brief_id"]
    );
    // 2026-04-08 CST: 杩欓噷琛?revision 鍚?approval_request 缁粦鏂█锛屽師鍥犳槸 Task 2 涓嶅彧瑕佹眰 v1 package 鍦ㄦ彁浜ゆ椂鎸備笂 binding锛岃繕瑕佹眰鐗堟湰鍖栧悗杩欐潯閾句笉涓㈠け锛?
    // 鐩殑锛氱‘淇濆鎵圭姸鎬佹洿鏂扮敓鎴?v2 package 鏃讹紝approval_request 閲岀殑 position_plan_binding 浠嶇劧鍜?object_graph / position_plan 鎸囧悜鍚屼竴姝ｅ紡瀵硅薄銆?
    let revised_approval_request: Value = serde_json::from_slice(
        &fs::read(&approval_request_path).expect("revised approval request should be readable"),
    )
    .expect("revised approval request should be valid json");
    assert_eq!(
        revised_approval_request["position_plan_binding"]["position_plan_ref"],
        submit_output["data"]["position_plan"]["plan_id"]
    );
    assert_eq!(
        revised_approval_request["position_plan_binding"]["position_plan_path"],
        submit_output["data"]["position_plan_path"]
    );
    assert_eq!(
        revised_approval_request["position_plan_binding"]["position_plan_contract_version"],
        "security_position_plan.v2"
    );
    assert_eq!(
        revision_output["data"]["decision_package"]["object_graph"]["position_plan_ref"],
        revised_approval_request["position_plan_binding"]["position_plan_ref"]
    );
    assert_eq!(
        revision_output["data"]["decision_package"]["object_graph"]["position_plan_path"],
        revised_approval_request["position_plan_binding"]["position_plan_path"]
    );
    assert!(
        revision_output["data"]["trigger_event_summary"]
            .as_str()
            .expect("trigger event summary should exist")
            .contains("pm_lead")
    );
    assert!(
        revision_output["data"]["verification_report_path"]
            .as_str()
            .expect("verification report path should exist")
            .contains("decision_packages_verification")
    );

    let revised_package_path = PathBuf::from(
        revision_output["data"]["decision_package_path"]
            .as_str()
            .expect("decision package path should exist"),
    );
    assert!(revised_package_path.exists());

    let revised_package: Value = serde_json::from_slice(
        &fs::read(&revised_package_path).expect("revised package should be readable"),
    )
    .expect("revised package should be valid json");
    assert_eq!(revised_package["package_version"], 2);
    assert_eq!(revised_package["revision_reason"], "approval_event_applied");
    assert_eq!(
        revised_package["previous_package_path"],
        package_path.to_string_lossy().to_string()
    );
    assert_eq!(revised_package["package_status"], "approved_bundle_ready");
    assert_eq!(
        revised_package["object_graph"]["position_plan_ref"],
        revised_approval_request["position_plan_binding"]["position_plan_ref"]
    );
    assert_eq!(
        revised_package["object_graph"]["position_plan_path"],
        revised_approval_request["position_plan_binding"]["position_plan_path"]
    );
    assert!(
        revised_package["artifact_manifest"]
            .as_array()
            .expect("artifact manifest should be array")
            .iter()
            .any(|artifact| artifact["artifact_role"] == "approval_events")
    );
}

#[test]
fn security_decision_package_revision_preserves_chair_resolution_ref_from_post_meeting_artifact() {
    let runtime_db_path = create_test_runtime_db("security_decision_package_revision_chair_ref");
    let chair_resolution_ref = "chair-resolution:601916.SH:2026-04-15:committee:v1";
    let post_meeting_path = create_post_meeting_chair_fixture(
        "security_decision_package_revision_chair_ref",
        chair_resolution_ref,
        "post meeting conclusion binds the package to the chair resolution",
    );
    let package_json = build_chair_revision_fixture_package(
        None,
        SecurityDecisionPackageArtifact {
            // 2026-04-16 CST: Reason=seed the minimal governed artifact needed by the new revision regression.
            // Purpose=let the test exercise runtime chair-ref recovery from post-meeting evidence without depending on submit_approval.
            artifact_role: "security_post_meeting_conclusion".to_string(),
            path: post_meeting_path.to_string_lossy().to_string(),
            sha256: String::new(),
            contract_version: "security_post_meeting_conclusion.v1".to_string(),
            required: false,
            present: true,
        },
    );
    let package_path = write_chair_revision_fixture_package(
        &runtime_db_path,
        "decision-601916-SH-2026-04-15.json",
        &package_json,
    );
    let revision_output = run_chair_revision(
        &runtime_db_path,
        &package_path,
        "recover_chair_resolution_binding",
    );

    // 2026-04-16 CST: Reason=lock the newest explicit chair node into the runtime revision path.
    // Purpose=prove v2 packages keep governance-visible chair binding even when v1 only carries it through post-meeting artifacts.
    assert_eq!(
        revision_output["status"], "ok",
        "security_decision_package_revision should preserve chair binding, output={revision_output}"
    );
    assert_eq!(
        revision_output["data"]["decision_package"]["object_graph"]["chair_resolution_ref"],
        chair_resolution_ref
    );

    let revised_package_path = PathBuf::from(
        revision_output["data"]["decision_package_path"]
            .as_str()
            .expect("revised package path should exist"),
    );
    let revised_package: Value = serde_json::from_slice(
        &fs::read(&revised_package_path).expect("revised package should be readable"),
    )
    .expect("revised package should be valid json");
    assert_eq!(
        revised_package["object_graph"]["chair_resolution_ref"],
        chair_resolution_ref
    );
    assert!(
        revised_package["artifact_manifest"]
            .as_array()
            .expect("artifact manifest should be array")
            .iter()
            .any(|artifact| artifact["artifact_role"] == "security_post_meeting_conclusion")
    );
}

#[test]
fn security_decision_package_revision_prefers_existing_chair_ref_over_tampered_post_meeting_artifact()
 {
    let runtime_db_path =
        create_test_runtime_db("security_decision_package_revision_existing_chair_ref");
    let expected_chair_resolution_ref = "chair-resolution:601916.SH:2026-04-15:committee:v1";
    let tampered_chair_resolution_ref =
        "chair-resolution:601916.SH:2026-04-15:committee:v1::tampered";
    let post_meeting_path = create_post_meeting_chair_fixture(
        "security_decision_package_revision_existing_chair_ref",
        tampered_chair_resolution_ref,
        "post meeting artifact was tampered after the package object graph had already frozen the chair binding",
    );
    let package_json = build_chair_revision_fixture_package(
        Some(expected_chair_resolution_ref),
        SecurityDecisionPackageArtifact {
            // 2026-04-16 CST: Reason=lock the revision boundary against a post-meeting
            // artifact that drifts after the package object graph has already frozen.
            // Purpose=prove revision keeps the formal package anchor instead of
            // re-importing a tampered downstream chair reference.
            artifact_role: "security_post_meeting_conclusion".to_string(),
            path: post_meeting_path.to_string_lossy().to_string(),
            sha256: String::new(),
            contract_version: "security_post_meeting_conclusion.v1".to_string(),
            required: false,
            present: true,
        },
    );
    let package_path = write_chair_revision_fixture_package(
        &runtime_db_path,
        "decision-601916-SH-2026-04-15-existing-chair.json",
        &package_json,
    );
    let revision_output = run_chair_revision(
        &runtime_db_path,
        &package_path,
        "preserve_frozen_chair_resolution_binding",
    );

    // 2026-04-16 CST: Reason=add the paired regression for the already-frozen
    // package anchor after we locked recovery-from-artifact behavior.
    // Purpose=prove revision never lets a later post-meeting drift override the
    // formal chair binding that v1 had already frozen into object_graph.
    assert_eq!(
        revision_output["status"], "ok",
        "security_decision_package_revision should keep the frozen chair binding, output={revision_output}"
    );
    assert_eq!(
        revision_output["data"]["decision_package"]["object_graph"]["chair_resolution_ref"],
        expected_chair_resolution_ref
    );

    let revised_package_path = PathBuf::from(
        revision_output["data"]["decision_package_path"]
            .as_str()
            .expect("revised package path should exist"),
    );
    let revised_package: Value = serde_json::from_slice(
        &fs::read(&revised_package_path).expect("revised package should be readable"),
    )
    .expect("revised package should be valid json");
    assert_eq!(
        revised_package["object_graph"]["chair_resolution_ref"],
        expected_chair_resolution_ref
    );
    assert_ne!(
        revised_package["object_graph"]["chair_resolution_ref"],
        tampered_chair_resolution_ref
    );
}

#[test]
fn security_decision_package_revision_preserves_existing_chair_ref_without_post_meeting_artifact() {
    let runtime_db_path = create_test_runtime_db(
        "security_decision_package_revision_existing_chair_without_artifact",
    );
    let expected_chair_resolution_ref = "chair-resolution:601916.SH:2026-04-15:committee:v1";
    let package_json = build_chair_revision_fixture_package(
        Some(expected_chair_resolution_ref),
        SecurityDecisionPackageArtifact {
            // 2026-04-16 CST: Reason=cover the frozen-chair path when the
            // downstream post-meeting artifact is absent during revision.
            // Purpose=prove revision keeps the package anchor even without a
            // readable post-meeting artifact to recover from.
            artifact_role: "security_post_meeting_conclusion".to_string(),
            path: String::new(),
            sha256: String::new(),
            contract_version: "security_post_meeting_conclusion.v1".to_string(),
            required: false,
            present: false,
        },
    );
    let package_path = write_chair_revision_fixture_package(
        &runtime_db_path,
        "decision-601916-SH-2026-04-15-existing-chair-no-artifact.json",
        &package_json,
    );
    let revision_output = run_chair_revision(
        &runtime_db_path,
        &package_path,
        "preserve_frozen_chair_without_post_meeting_artifact",
    );

    // 2026-04-16 CST: Reason=complete the frozen-chair regression family with
    // the missing-artifact case after we locked recovery and anti-override paths.
    // Purpose=prove revision preserves the explicit package anchor even when
    // there is no post-meeting artifact available in the manifest.
    assert_eq!(
        revision_output["status"], "ok",
        "security_decision_package_revision should preserve the frozen chair binding without a post-meeting artifact, output={revision_output}"
    );
    assert_eq!(
        revision_output["data"]["decision_package"]["object_graph"]["chair_resolution_ref"],
        expected_chair_resolution_ref
    );

    let revised_package_path = PathBuf::from(
        revision_output["data"]["decision_package_path"]
            .as_str()
            .expect("revised package path should exist"),
    );
    let revised_package: Value = serde_json::from_slice(
        &fs::read(&revised_package_path).expect("revised package should be readable"),
    )
    .expect("revised package should be valid json");
    assert_eq!(
        revised_package["object_graph"]["chair_resolution_ref"],
        expected_chair_resolution_ref
    );
    assert!(
        revised_package["artifact_manifest"]
            .as_array()
            .expect("artifact manifest should be array")
            .iter()
            .any(|artifact| {
                artifact["artifact_role"] == "security_post_meeting_conclusion"
                    && artifact["present"] == false
            })
    );
}

#[test]
fn security_decision_package_revision_can_attach_lifecycle_refs_and_feedback_summary() {
    let runtime_db_path = create_test_runtime_db("security_decision_package_revision_lifecycle");
    let approval_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scenes_runtime");

    let stock_csv = create_stock_history_csv(
        "security_decision_package_revision_lifecycle",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_decision_package_revision_lifecycle",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_decision_package_revision_lifecycle",
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
                        {"notice_date":"2026-03-28","title":"2025骞村勾搴︽姤鍛?,"art_code":"AN202603281234567890","columns":[{"column_name":"瀹氭湡鎶ュ憡"}]}
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
            "created_at": "2026-04-12T10:00:00+08:00"
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

    let package_path = PathBuf::from(
        submit_output["data"]["decision_package_path"]
            .as_str()
            .expect("decision package path should exist"),
    );
    let package_document: Value = serde_json::from_slice(
        &fs::read(&package_path).expect("decision package should be readable"),
    )
    .expect("decision package should be valid json");
    let decision_ref = submit_output["data"]["decision_ref"]
        .as_str()
        .expect("decision ref should exist");
    let approval_ref = submit_output["data"]["approval_ref"]
        .as_str()
        .expect("approval ref should exist");
    let package_analysis_date = package_document["analysis_date"]
        .as_str()
        .expect("package analysis date should exist");
    let position_plan_ref = submit_output["data"]["position_plan"]["plan_id"]
        .as_str()
        .expect("position plan ref should exist");

    let condition_review_request = json!({
        "tool": "security_condition_review",
        "args": {
            // 2026-04-16 CST: Reason=replace the last handwritten lifecycle attachment with the
            // formal condition-review tool output so this revision test stops depending on a stale schema.
            // Purpose=keep all lifecycle refs in the package-revision flow aligned with governed mainline documents.
            "symbol": "601916.SH",
            "analysis_date": "2026-04-12",
            "decision_ref": decision_ref,
            "approval_ref": approval_ref,
            "position_plan_ref": position_plan_ref,
            "decision_package_path": package_path.to_string_lossy(),
            "review_trigger_type": "manual_review",
            "review_trigger_summary": "build-day lifecycle review fixture",
            "created_at": "2026-04-12T10:05:00+08:00"
        }
    });
    let condition_review_output =
        run_cli_with_json_and_runtime(&condition_review_request.to_string(), &runtime_db_path);
    assert_eq!(
        condition_review_output["status"], "ok",
        "condition_review output={condition_review_output}"
    );
    let condition_review_ref =
        condition_review_output["data"]["condition_review"]["condition_review_id"]
            .as_str()
            .expect("condition review ref should exist");
    let condition_review_path = create_json_fixture(
        "security_decision_package_revision_lifecycle",
        "condition_review.json",
        &condition_review_output["data"]["condition_review"].clone(),
    );
    let execution_record_request = json!({
        "tool": "security_execution_record",
        "args": {
            // 2026-04-16 CST: Reason=source execution lifecycle fixtures from the formal tool instead of
            // preserving a stale handwritten JSON contract in this revision test.
            // Purpose=keep lifecycle package revision coverage aligned with the live execution document shape.
            "symbol": "601916.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "market_regime": "a_share",
            "sector_template": "bank",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "as_of_date": "2025-08-08",
            "condition_review_ref": condition_review_ref,
            "execution_action": "build",
            "execution_status": "filled",
            "executed_gross_pct": 0.06,
            "execution_trades": [
                {
                    "trade_date": "2025-08-08",
                    "side": "buy",
                    "price": 62.40,
                    "position_pct_delta": 0.06,
                    "reason": "breakout_entry",
                    "notes": ["formal lifecycle execution fixture"]
                }
            ],
            "execution_summary": "build starter position",
            "created_at": "2026-04-12T10:15:00+08:00"
        }
    });
    let execution_record_output =
        run_cli_with_json_and_runtime(&execution_record_request.to_string(), &runtime_db_path);
    assert_eq!(
        execution_record_output["status"], "ok",
        "execution_record output={execution_record_output}"
    );
    let execution_record_ref =
        execution_record_output["data"]["execution_record"]["execution_record_id"]
            .as_str()
            .expect("execution record ref should exist");
    let mut execution_record_document = execution_record_output["data"]["execution_record"].clone();
    // 2026-04-16 CST: Reason=align the formal execution artifact with the existing approval package
    // identity, because the live execution tool currently rebuilds a fresh position-plan lineage.
    // Purpose=keep this revision test focused on package attachment semantics instead of unrelated
    // planner identity drift.
    execution_record_document["analysis_date"] = Value::String(package_analysis_date.to_string());
    execution_record_document["position_plan_ref"] = Value::String(position_plan_ref.to_string());
    let execution_record_path = create_json_fixture(
        "security_decision_package_revision_lifecycle",
        "execution_record.json",
        &execution_record_document,
    );
    let post_trade_review_request = json!({
        "tool": "security_post_trade_review",
        "args": {
            // 2026-04-16 CST: Reason=generate the review attachment from the formal tool instead of
            // maintaining a second outdated JSON fixture in the revision test.
            // Purpose=let lifecycle package revision consume governed review artifacts from the mainline.
            "symbol": "601916.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "market_regime": "a_share",
            "sector_template": "bank",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "as_of_date": "2025-08-08",
            "execution_trades": [
                {
                    "trade_date": "2025-08-08",
                    "side": "buy",
                    "price": 62.40,
                    "position_pct_delta": 0.06,
                    "reason": "breakout_entry",
                    "notes": ["formal lifecycle review fixture"]
                }
            ],
            "created_at": "2026-04-12T10:30:00+08:00"
        }
    });
    let post_trade_review_output =
        run_cli_with_json_and_runtime(&post_trade_review_request.to_string(), &runtime_db_path);
    assert_eq!(
        post_trade_review_output["status"], "ok",
        "post_trade_review output={post_trade_review_output}"
    );
    let post_trade_review_ref = post_trade_review_output["data"]["post_trade_review"]["review_id"]
        .as_str()
        .expect("post trade review ref should exist");
    let mut post_trade_review_document =
        post_trade_review_output["data"]["post_trade_review"].clone();
    // 2026-04-16 CST: Reason=align the formal review artifact with the package-bound execution and
    // position-plan refs before revision validation checks package object-graph consistency.
    // Purpose=preserve formal review structure while keeping this regression scoped to revision wiring.
    post_trade_review_document["analysis_date"] = Value::String(package_analysis_date.to_string());
    post_trade_review_document["position_plan_ref"] = Value::String(position_plan_ref.to_string());
    post_trade_review_document["execution_record_ref"] =
        Value::String(execution_record_ref.to_string());
    let post_trade_review_path = create_json_fixture(
        "security_decision_package_revision_lifecycle",
        "post_trade_review.json",
        &post_trade_review_document,
    );

    let revision_request = json!({
        "tool": "security_decision_package_revision",
        "args": {
            "package_path": package_path.to_string_lossy(),
            "revision_reason": "attach_lifecycle_records",
            "reverify_after_revision": false,
            "condition_review_path": condition_review_path.to_string_lossy(),
            "execution_record_path": execution_record_path.to_string_lossy(),
            "post_trade_review_path": post_trade_review_path.to_string_lossy()
        }
    });
    let revision_output =
        run_cli_with_json_runtime_and_envs(&revision_request.to_string(), &runtime_db_path, &[]);

    // 2026-04-12 CST: Lock lifecycle attachment into the package revision flow,
    // because P8-4/P8-5 require the formal condition/execution/post-trade objects
    // to become first-class package references instead of external loose files.
    // Purpose: prove object-graph wiring, manifest persistence, and feedback summary generation in one governed revision path.
    assert_eq!(
        revision_output["status"], "ok",
        "security_decision_package_revision lifecycle output={revision_output}"
    );
    assert_eq!(
        revision_output["data"]["decision_package"]["object_graph"]["condition_review_ref"],
        condition_review_ref
    );
    assert_eq!(
        revision_output["data"]["decision_package"]["object_graph"]["execution_record_ref"],
        execution_record_ref
    );
    assert_eq!(
        revision_output["data"]["decision_package"]["object_graph"]["post_trade_review_ref"],
        post_trade_review_ref
    );
    assert_eq!(
        revision_output["data"]["decision_package"]["lifecycle_governance_summary"]["recommended_governance_action"],
        post_trade_review_output["data"]["post_trade_review"]["next_adjustment_hint"]
    );
    assert_eq!(
        revision_output["data"]["decision_package"]["lifecycle_governance_summary"]["attribution_layers"],
        json!([])
    );
    assert!(
        revision_output["data"]["decision_package"]["artifact_manifest"]
            .as_array()
            .expect("artifact manifest should be array")
            .iter()
            .any(|artifact| artifact["artifact_role"] == "post_trade_review")
    );
}

fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_decision_package_revision_fixture"
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
