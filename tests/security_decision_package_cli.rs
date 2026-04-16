mod common;

use chrono::{Duration, NaiveDate};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration as StdDuration, Instant, SystemTime, UNIX_EPOCH};

use crate::common::{
    create_test_runtime_db, run_cli_with_json, run_cli_with_json_runtime_and_envs,
};
use excel_skill::ops::stock::security_decision_package::sha256_for_json_value;

// 2026-04-16 CST: Reason=replace the retired package CLI fixture with a current-contract
// fixture root that can serve both direct package-builder assertions and package-path
// verification assertions.
// Purpose=keep this suite attached to the live public contract instead of the removed
// inline package/verify payload flow.
fn create_fixture_dir(prefix: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_decision_package")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("security decision package fixture dir should exist");
    fixture_dir
}

// 2026-04-16 CST: Reason=share simple JSON fixture persistence across the rebuilt
// package CLI suite after removing the old retired-flow helpers.
// Purpose=keep the live contract tests focused on builder and verification semantics.
fn create_json_fixture(prefix: &str, file_name: &str, value: &Value) -> PathBuf {
    let fixture_dir = create_fixture_dir(prefix);
    let json_path = fixture_dir.join(file_name);
    fs::write(
        &json_path,
        serde_json::to_vec_pretty(value).expect("json fixture should serialize"),
    )
    .expect("json fixture should be written");
    json_path
}

// 2026-04-09 CST: Keep the stock-history CSV helper in this suite because the
// submit-approval happy path still needs deterministic local market fixtures.
// Purpose=avoid external data drift while the package-path verification tests run.
fn create_stock_history_csv(prefix: &str, file_name: &str, rows: &[String]) -> PathBuf {
    let fixture_dir = create_fixture_dir(prefix);
    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n")).expect("security decision package csv should be written");
    csv_path
}

// 2026-04-09 CST: Keep the local HTTP stub because submit_approval still needs
// stable fundamentals and announcement responses in CLI tests.
// Purpose=hold the suite on governed package semantics instead of live network noise.
fn spawn_http_route_server(routes: Vec<(&str, &str, &str, &str)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test http server should bind");
    listener
        .set_nonblocking(true)
        .expect("test http server should become nonblocking");
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
        // 2026-04-16 CST: Keep the fixture server alive across submit_approval and
        // later post_meeting calls, because the rebuilt package tests now exercise
        // multiple governed tools against the same local provider stub.
        // Purpose=avoid false failures caused by the stub exiting after the first request pair.
        let idle_timeout = StdDuration::from_secs(2);
        let mut idle_started_at = Instant::now();
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    idle_started_at = Instant::now();
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
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if idle_started_at.elapsed() >= idle_timeout {
                        break;
                    }
                    thread::sleep(StdDuration::from_millis(10));
                }
                Err(_) => break,
            }
        }
    });

    address
}

#[test]
fn tool_catalog_includes_security_decision_package_chain() {
    let output = run_cli_with_json("");

    // 2026-04-16 CST: Reason=keep discovery coverage after retiring the old inline
    // package flow assertions in this suite.
    // Purpose=prove the formal package chain remains catalog-visible on the public surface.
    for tool_name in [
        "security_record_post_meeting_conclusion",
        "security_decision_package",
        "security_decision_verify_package",
        "security_decision_package_revision",
    ] {
        assert!(
            output["data"]["tool_catalog"]
                .as_array()
                .expect("tool catalog should be an array")
                .iter()
                .any(|tool| tool == tool_name),
            "tool catalog should include {tool_name}"
        );
    }
}

#[test]
fn security_decision_package_accepts_current_builder_contract() {
    let artifact_prefix = "security_decision_package_builder_contract";
    let decision_card_path = create_json_fixture(
        artifact_prefix,
        "decision_card.json",
        &json!({"decision_ref":"decision:601916.SH:2026-04-16"}),
    );
    let approval_request_path = create_json_fixture(
        artifact_prefix,
        "approval_request.json",
        &json!({"approval_ref":"approval:601916.SH:2026-04-16"}),
    );
    let position_plan_path = create_json_fixture(
        artifact_prefix,
        "position_plan.json",
        &json!({"plan_id":"position-plan:601916.SH:2026-04-16"}),
    );
    let approval_brief_path = create_json_fixture(
        artifact_prefix,
        "approval_brief.json",
        &json!({"brief_id":"approval-brief:601916.SH:2026-04-16"}),
    );
    let scorecard_path = create_json_fixture(
        artifact_prefix,
        "scorecard.json",
        &json!({"scorecard_id":"scorecard:601916.SH:2026-04-16"}),
    );
    let post_meeting_path = create_json_fixture(
        artifact_prefix,
        "post_meeting_conclusion.json",
        &json!({
            "post_meeting_conclusion_id": "post-meeting-601916-SH-2026-04-16",
            "contract_version": "security_post_meeting_conclusion.v1",
            "document_type": "security_post_meeting_conclusion",
            "generated_at": "2026-04-16T10:05:00+08:00",
            "symbol": "601916.SH",
            "analysis_date": "2026-04-16",
            "decision_id": "decision-601916-SH-2026-04-16",
            "chair_resolution_ref": "chair-decision-601916-SH-2026-04-16",
            "final_action": "buy",
            "chair_process_action": "approve_execution",
            "final_trading_stance": "bullish",
            "final_exposure_side": "long",
            "final_confidence": 0.82,
            "revision_required": false,
            "return_to_stage": null,
            "execution_notes": ["keep the plan"],
            "follow_up_actions": ["track next disclosure"],
            "conclusion_summary": "fixture post meeting"
        }),
    );
    let post_meeting_value: Value = serde_json::from_slice(
        &fs::read(&post_meeting_path).expect("post meeting fixture should be readable"),
    )
    .expect("post meeting fixture should be valid json");

    // 2026-04-16 CST: Build the current request in smaller values so the test
    // stays readable and avoids one giant recursive `json!` expansion.
    // Purpose=keep this builder-contract regression stable in the Rust test crate.
    let artifact_manifest = json!([{
        "artifact_role": "security_post_meeting_conclusion",
        "path": post_meeting_path.to_string_lossy(),
        "sha256": sha256_for_json_value(&post_meeting_value).expect("post meeting sha should compute"),
        "contract_version": "security_post_meeting_conclusion.v1",
        "required": false,
        "present": true
    }]);
    let args = json!({
        "created_at": "2026-04-16T10:10:00+08:00",
        "package_version": 1,
        "previous_package_path": null,
        "revision_reason": "initial_submission",
        "trigger_event_summary": "builder contract fixture",
        "scene_name": "security_review",
        "decision_id": "decision-601916-SH-2026-04-16",
        "decision_ref": "decision:601916.SH:2026-04-16",
        "approval_ref": "approval:601916.SH:2026-04-16",
        "symbol": "601916.SH",
        "analysis_date": "2026-04-16",
        "decision_status": "ready_for_review",
        "approval_status": "Pending",
        "model_grade_summary": null,
        "model_governance_summary": null,
        "lifecycle_governance_summary": null,
        "position_plan_ref": "position-plan:601916.SH:2026-04-16",
        "approval_brief_ref": "approval-brief:601916.SH:2026-04-16",
        "scorecard_ref": "scorecard:601916.SH:2026-04-16",
        "chair_resolution_ref": "chair-decision-601916-SH-2026-04-16",
        "condition_review_ref": null,
        "execution_record_ref": null,
        "post_trade_review_ref": null,
        "decision_card_path": decision_card_path.to_string_lossy(),
        "approval_request_path": approval_request_path.to_string_lossy(),
        "position_plan_path": position_plan_path.to_string_lossy(),
        "approval_brief_path": approval_brief_path.to_string_lossy(),
        "scorecard_path": scorecard_path.to_string_lossy(),
        "condition_review_path": null,
        "execution_record_path": null,
        "post_trade_review_path": null,
        "evidence_hash": "evidence-hash",
        "governance_hash": "governance-hash",
        "artifact_manifest": artifact_manifest
    });
    let request = json!({
        "tool": "security_decision_package",
        "args": args
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &create_test_runtime_db("security_decision_package_builder_contract"),
        &[],
    );

    // 2026-04-16 CST: Reason=replace the retired package-flow assertions with a
    // direct builder-contract regression on the live public request shape.
    // Purpose=prove the current package tool still builds the formal package document.
    assert_eq!(
        output["status"], "ok",
        "security_decision_package should accept the current builder contract, output={output}"
    );
    assert_eq!(output["data"]["package_version"], 1);
    assert_eq!(output["data"]["revision_reason"], "initial_submission");
    assert_eq!(
        output["data"]["trigger_event_summary"],
        "builder contract fixture"
    );
    assert_eq!(
        output["data"]["object_graph"]["chair_resolution_ref"],
        "chair-decision-601916-SH-2026-04-16"
    );
    assert!(output["data"]["artifact_manifest"]
        .as_array()
        .expect("artifact manifest should be an array")
        .iter()
        .any(|artifact| artifact["artifact_role"] == "security_post_meeting_conclusion"));
}

#[test]
fn security_decision_verify_package_accepts_submit_package_after_post_meeting_attachment() {
    let runtime_db_path = create_test_runtime_db("security_decision_package_verify_happy_path");
    let (package_path, server) = prepare_submit_package(
        &runtime_db_path,
        "security_decision_package_verify_happy_path",
        "2026-04-16T11:00:00+08:00",
    );
    attach_post_meeting_artifact(
        &runtime_db_path,
        &package_path,
        "security_decision_package_verify_happy_path",
        &server,
        "2026-04-16T12:00:00+08:00",
        false,
    );

    let verify_request = json!({
        "tool": "security_decision_verify_package",
        "args": {
            "package_path": package_path.to_string_lossy(),
            "approval_brief_signing_key_secret": "brief-secret-for-tests"
        }
    });
    let verify_output =
        run_cli_with_json_runtime_and_envs(&verify_request.to_string(), &runtime_db_path, &[]);

    // 2026-04-16 CST: Reason=keep one happy-path proof in this suite after the old
    // inline package verification contract was removed.
    // Purpose=prove the public chain still works when a governed post-meeting artifact
    // is attached to a real approval package on disk.
    assert_eq!(verify_output["status"], "ok");
    assert_eq!(verify_output["data"]["package_valid"], true);
    assert_eq!(
        verify_output["data"]["governance_checks"]["post_meeting_binding_consistent"],
        true
    );
    assert_eq!(
        verify_output["data"]["recommended_action"],
        "proceed_with_review"
    );
}

#[test]
fn security_decision_verify_package_flags_post_meeting_binding_misalignment_on_package_path() {
    let runtime_db_path =
        create_test_runtime_db("security_decision_package_verify_post_meeting_misaligned");
    let (package_path, server) = prepare_submit_package(
        &runtime_db_path,
        "security_decision_package_verify_post_meeting_misaligned",
        "2026-04-16T11:30:00+08:00",
    );
    attach_post_meeting_artifact(
        &runtime_db_path,
        &package_path,
        "security_decision_package_verify_post_meeting_misaligned",
        &server,
        "2026-04-16T12:30:00+08:00",
        true,
    );

    let verify_request = json!({
        "tool": "security_decision_verify_package",
        "args": {
            "package_path": package_path.to_string_lossy(),
            "approval_brief_signing_key_secret": "brief-secret-for-tests"
        }
    });
    let verify_output =
        run_cli_with_json_runtime_and_envs(&verify_request.to_string(), &runtime_db_path, &[]);

    // 2026-04-16 CST: Reason=replace the retired inline broken-package assertions with
    // the live package-path verification contract.
    // Purpose=prove verify_package still rejects post-meeting chair-binding drift on disk.
    assert_eq!(verify_output["status"], "ok");
    assert_eq!(verify_output["data"]["package_valid"], false);
    assert_eq!(
        verify_output["data"]["governance_checks"]["post_meeting_binding_consistent"],
        false
    );
    assert!(verify_output["data"]["issues"]
        .as_array()
        .expect("issues should be an array")
        .iter()
        .any(|item| item
            .as_str()
            .unwrap_or_default()
            .contains("security_post_meeting_conclusion")));
}

// 2026-04-16 CST: Reason=share a live submit_approval package fixture across the
// rebuilt package CLI tests instead of synthesizing the whole governed package by hand.
// Purpose=keep verification coverage tied to the real approval-package persistence path.
fn prepare_submit_package(
    runtime_db_path: &Path,
    prefix: &str,
    created_at: &str,
) -> (PathBuf, String) {
    let stock_csv = create_stock_history_csv(
        prefix,
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        prefix,
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        prefix,
        "sector.csv",
        &build_confirmed_breakout_rows(220, 950.0),
    );
    import_history_csv(runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(runtime_db_path, &sector_csv, "512800.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 200 OK",
            r#"[{"REPORT_DATE":"2025-12-31","NOTICE_DATE":"2026-03-28","TOTAL_OPERATE_INCOME":308227000000.0,"YSTZ":8.37,"PARENT_NETPROFIT":11117000000.0,"SJLTZ":9.31,"ROEJQ":14.8}]"#,
            "application/json",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{"data":{"list":[{"notice_date":"2026-03-28","title":"2025年度报告","art_code":"AN202603281234567890","columns":[{"column_name":"定期报告"}]}]}}"#,
            "application/json",
        ),
    ]);

    let approval_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scenes_runtime");
    let submit_request = json!({
        "tool": "security_decision_submit_approval",
        "args": {
            "symbol": "601916.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12,
            "approval_runtime_root": approval_root.to_string_lossy(),
            "created_at": created_at,
            "approval_brief_signing_key_id": "brief_signing_key_20260416",
            "approval_brief_signing_key_secret": "brief-secret-for-tests"
        }
    });
    let submit_output = run_cli_with_json_runtime_and_envs(
        &submit_request.to_string(),
        &runtime_db_path.to_path_buf(),
        &security_envs(&server),
    );
    assert_eq!(
        submit_output["status"], "ok",
        "security_decision_submit_approval should succeed, output={submit_output}"
    );

    (
        PathBuf::from(
            submit_output["data"]["decision_package_path"]
                .as_str()
                .expect("decision package path should exist"),
        ),
        server,
    )
}

// 2026-04-16 CST: Reason=attach one governed post-meeting artifact to a real approval
// package so this suite can exercise the current package-path verification contract.
// Purpose=cover both the valid and the intentionally misaligned chair-binding paths.
fn attach_post_meeting_artifact(
    runtime_db_path: &Path,
    package_path: &Path,
    prefix: &str,
    server: &str,
    created_at: &str,
    tamper_chair_ref: bool,
) {
    let mut package_json: Value = serde_json::from_slice(
        &fs::read(package_path).expect("decision package should be readable"),
    )
    .expect("decision package should be valid json");

    let package_symbol = package_json["symbol"]
        .as_str()
        .expect("symbol should exist");
    let package_analysis_date = package_json["analysis_date"]
        .as_str()
        .expect("analysis date should exist");
    let package_decision_id = package_json["decision_id"]
        .as_str()
        .expect("decision id should exist");
    let record_request = json!({
        "tool": "security_record_post_meeting_conclusion",
        "args": {
            "symbol": package_symbol,
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "as_of_date": package_analysis_date,
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12,
            "created_at": created_at,
            "execution_notes": ["keep the plan"],
            "follow_up_actions": ["track next disclosure"]
        }
    });
    let record_output = run_cli_with_json_runtime_and_envs(
        &record_request.to_string(),
        &runtime_db_path.to_path_buf(),
        &security_envs(server),
    );
    assert_eq!(
        record_output["status"], "ok",
        "security_record_post_meeting_conclusion should succeed, output={record_output}"
    );
    // 2026-04-16 CST: Switch this suite to the real post_meeting tool output after
    // review found that the previous helper hand-crafted a chair ref the runtime never emits.
    // Purpose=keep package-path verification attached to real downstream governance artifacts.
    let post_meeting_document = record_output["data"]["post_meeting_conclusion"].clone();
    assert_eq!(
        post_meeting_document["decision_id"].as_str(),
        Some(package_decision_id),
        "post meeting decision id should stay aligned with the package"
    );
    assert_eq!(
        post_meeting_document["symbol"].as_str(),
        Some(package_symbol),
        "post meeting symbol should stay aligned with the package"
    );
    assert_eq!(
        post_meeting_document["analysis_date"].as_str(),
        Some(package_analysis_date),
        "post meeting analysis date should stay aligned with the package"
    );
    let chair_resolution_ref = post_meeting_document["chair_resolution_ref"]
        .as_str()
        .expect("post meeting chair resolution ref should exist")
        .to_string();
    let post_meeting_path = create_json_fixture(
        prefix,
        "post_meeting_conclusion.json",
        &post_meeting_document,
    );

    let package_chair_resolution_ref = if tamper_chair_ref {
        format!("{chair_resolution_ref}::tampered")
    } else {
        chair_resolution_ref.clone()
    };
    package_json["object_graph"]["chair_resolution_ref"] =
        Value::String(package_chair_resolution_ref);
    package_json["artifact_manifest"]
        .as_array_mut()
        .expect("artifact manifest should be an array")
        .push(json!({
            "artifact_role": "security_post_meeting_conclusion",
            "path": post_meeting_path.to_string_lossy(),
            "sha256": sha256_for_json_value(&post_meeting_document).expect("post meeting sha should compute"),
            "contract_version": "security_post_meeting_conclusion.v1",
            "required": false,
            "present": true
        }));

    fs::write(
        package_path,
        serde_json::to_vec_pretty(&package_json).expect("package json should serialize"),
    )
    .expect("package should be rewritten");
}

fn security_envs(server: &str) -> [(&'static str, String); 6] {
    [
        (
            "EXCEL_SKILL_EASTMONEY_FINANCIAL_URL_BASE",
            format!("{server}/financials"),
        ),
        (
            "EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE",
            format!("{server}/announcements"),
        ),
        (
            "EXCEL_SKILL_OFFICIAL_FINANCIAL_URL_BASE",
            format!("{server}/financials"),
        ),
        (
            "EXCEL_SKILL_OFFICIAL_ANNOUNCEMENT_URL_BASE",
            format!("{server}/announcements"),
        ),
        (
            "EXCEL_SKILL_SINA_FINANCIAL_URL_BASE",
            format!("{server}/financials"),
        ),
        (
            "EXCEL_SKILL_SINA_ANNOUNCEMENT_URL_BASE",
            format!("{server}/announcements"),
        ),
    ]
}

fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_decision_package_fixture"
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
