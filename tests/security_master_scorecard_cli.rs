mod common;

use chrono::{Duration, NaiveDate};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{
    create_test_runtime_db, run_cli_with_json, run_cli_with_json_runtime_and_envs,
};

// 2026-04-11 CST: 这里新增 master_scorecard CLI 测试夹具，原因是方案 C 要先把“未来多期限赚钱效益总卡”锁成正式外部合同；
// 目的：先用红测固定 Tool 发现性、正式对象边界和多期限回放总卡最小字段，再去补实现，避免后续把设计稿和真实上线能力混在一起。
fn create_stock_history_csv(prefix: &str, file_name: &str, rows: &[String]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_master_scorecard")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("security master scorecard fixture dir should exist");

    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n")).expect("security master scorecard csv should be written");
    csv_path
}

// 2026-04-11 CST: 这里沿用本地 HTTP 假服务，原因是 master_scorecard 内部仍会走 committee/scorecard 链，而这些链会依赖财报与公告上下文；
// 目的：让测试聚焦在总卡聚合语义本身，而不是被外部 HTTP 波动打断。
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
fn tool_catalog_includes_security_master_scorecard() {
    let output = run_cli_with_json("");

    // 2026-04-11 CST: 这里先锁住 master_scorecard 的可发现性，原因是如果 catalog 不暴露它，后续 Skill 和 CLI 就无法把“未来赚钱效益总卡”当成正式能力使用；
    // 目的：确保它从第一天起就是一等 Tool，而不是隐藏在内部实现里的临时聚合函数。
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_master_scorecard")
    );
}

#[test]
fn security_master_scorecard_returns_formal_multi_horizon_profitability_summary() {
    let runtime_db_path = create_test_runtime_db("security_master_scorecard_ready");

    let stock_csv = create_stock_history_csv(
        "security_master_scorecard_ready",
        "stock.csv",
        &build_linear_growth_rows(420, 100.0, 1.0),
    );
    let market_csv = create_stock_history_csv(
        "security_master_scorecard_ready",
        "market.csv",
        &build_linear_growth_rows(420, 3200.0, 5.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_master_scorecard_ready",
        "sector.csv",
        &build_linear_growth_rows(420, 950.0, 2.0),
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

    let request = json!({
        "tool": "security_master_scorecard",
        "args": {
            "symbol": "601916.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "as_of_date": "2025-08-28",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12,
            "created_at": "2026-04-11T12:00:00+08:00"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
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

    // 2026-04-11 CST: 这里先锁住方案 C 的最小正式合同，原因是这轮要做的是“历史回放型大总卡”，不是先把训练版总卡一步做到头；
    // 目的：要求输出必须同时保留 committee 线、scorecard 线和 master_scorecard 正式对象，并且总卡要显式汇总 6 个 horizon 的赚钱效益结果。
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["scorecard"]["document_type"],
        "security_scorecard"
    );
    assert_eq!(
        output["data"]["master_scorecard"]["document_type"],
        "security_master_scorecard"
    );
    assert_eq!(
        output["data"]["master_scorecard"]["scorecard_status"],
        "model_unavailable"
    );
    assert_eq!(
        output["data"]["master_scorecard"]["aggregation_status"],
        "historical_replay_only"
    );
    assert_eq!(
        output["data"]["master_scorecard"]["committee_session_ref"],
        output["data"]["committee_result"]["committee_session_ref"]
    );
    assert_eq!(
        output["data"]["master_scorecard"]["scorecard_ref"],
        output["data"]["scorecard"]["scorecard_id"]
    );
    // 2026-04-16 CST: Added because plan A step 1 must freeze the public replay-mode
    // master-scorecard contract, not only the internal builder result.
    // Reason: a later refactor could keep unit tests green while silently dropping the
    // composite business artifact or governed committee payload from the CLI surface.
    // Purpose: require the formal tool output to carry both new bridge artifacts together.
    assert_eq!(
        output["data"]["composite_scorecard"]["document_type"],
        "security_composite_scorecard"
    );
    assert_eq!(
        output["data"]["committee_payload_adapter"]["committee_schema_version"],
        "committee-payload:v1"
    );
    assert_eq!(
        output["data"]["committee_payload_adapter"]["symbol"],
        output["data"]["master_scorecard"]["symbol"]
    );
    assert_eq!(
        output["data"]["committee_payload_adapter"]["recommended_action"],
        output["data"]["committee_result"]["decision_card"]["recommendation_action"]
    );
    assert_eq!(
        output["data"]["committee_payload_adapter"]["recommendation_digest"]["summary"]
            .as_str()
            .map(|value| value.contains("composite_score=")),
        Some(true)
    );

    let horizon_breakdown = output["data"]["master_scorecard"]["horizon_breakdown"]
        .as_array()
        .expect("horizon breakdown should be an array");
    assert_eq!(horizon_breakdown.len(), 6);
    assert_eq!(find_horizon(horizon_breakdown, 5)["positive_return"], true);
    assert_eq!(
        find_horizon(horizon_breakdown, 60)["hit_upside_first"],
        true
    );

    let master_score = output["data"]["master_scorecard"]["master_score"]
        .as_f64()
        .expect("master score should exist");
    assert!(
        master_score > 60.0,
        "master score should be meaningfully positive"
    );
    assert_eq!(
        output["data"]["master_scorecard"]["master_signal"],
        "historically_effective"
    );
}

#[test]
fn security_master_scorecard_preserves_partial_multi_head_summary_when_three_heads_are_available() {
    let runtime_db_path = create_test_runtime_db("security_master_scorecard_multi_head");

    let stock_csv = create_stock_history_csv(
        "security_master_scorecard_multi_head",
        "stock.csv",
        &build_linear_growth_rows(420, 100.0, 1.0),
    );
    let market_csv = create_stock_history_csv(
        "security_master_scorecard_multi_head",
        "market.csv",
        &build_linear_growth_rows(420, 3200.0, 5.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_master_scorecard_multi_head",
        "sector.csv",
        &build_linear_growth_rows(420, 950.0, 2.0),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let model_dir = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("multi_head_models");
    fs::create_dir_all(&model_dir).expect("multi head model dir should exist");
    let return_model_path = model_dir.join("return_head.json");
    let drawdown_model_path = model_dir.join("drawdown_head.json");
    let path_model_path = model_dir.join("path_quality_head.json");
    fs::write(
        &return_model_path,
        build_regression_head_artifact_json("return_head", 0.118, 0.135),
    )
    .expect("return head model should be written");
    fs::write(
        &drawdown_model_path,
        build_regression_head_artifact_json("drawdown_head", 0.038, 0.031),
    )
    .expect("drawdown head model should be written");
    fs::write(
        &path_model_path,
        build_regression_head_artifact_json("path_quality_head", 72.0, 81.0),
    )
    .expect("path quality head model should be written");

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
                        {"notice_date":"2026-03-28","title":"2025骞村害鎶ュ憡","art_code":"AN202603281234567890","columns":[{"column_name":"瀹氭湡鎶ュ憡"}]},
                        {"notice_date":"2026-03-28","title":"2025骞村害鍒╂鼎鍒嗛厤棰勬鍏憡","art_code":"AN202603281234567891","columns":[{"column_name":"鍏徃鍏憡"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_master_scorecard",
        "args": {
            "symbol": "601916.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "as_of_date": "2025-08-28",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12,
            "created_at": "2026-04-11T23:10:00+08:00",
            "return_head_model_path": return_model_path.to_string_lossy(),
            "drawdown_head_model_path": drawdown_model_path.to_string_lossy(),
            "path_quality_head_model_path": path_model_path.to_string_lossy()
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
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

    // 2026-04-11 CST: Lock the partial multi-head summary contract, because the
    // governed runtime now distinguishes between three-head partial context and the
    // full five-head path-event-ready state.
    // Purpose: keep the summary auditable when only return/drawdown/path heads are
    // attached, without pretending the full path-event context is ready.
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["master_scorecard"]["aggregation_status"],
        "historical_replay_only"
    );
    assert_eq!(
        output["data"]["master_scorecard"]["trained_head_summary"]["head_count"],
        3
    );
    assert_eq!(
        output["data"]["master_scorecard"]["trained_head_summary"]["availability_status"],
        "partial_multi_head"
    );
    assert!(
        output["data"]["master_scorecard"]["trained_head_summary"]["expected_return"].is_number(),
        "expected return should exist"
    );
    assert!(
        output["data"]["master_scorecard"]["trained_head_summary"]["expected_drawdown"].is_number(),
        "expected drawdown should exist"
    );
    assert!(
        output["data"]["master_scorecard"]["trained_head_summary"]["expected_path_quality"]
            .is_number(),
        "expected path quality should exist"
    );
}

#[test]
fn security_master_scorecard_attaches_path_event_context_when_available() {
    let runtime_db_path = create_test_runtime_db("security_master_scorecard_path_events");

    let stock_csv = create_stock_history_csv(
        "security_master_scorecard_path_events",
        "stock.csv",
        &build_linear_growth_rows(420, 100.0, 1.0),
    );
    let market_csv = create_stock_history_csv(
        "security_master_scorecard_path_events",
        "market.csv",
        &build_linear_growth_rows(420, 3200.0, 5.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_master_scorecard_path_events",
        "sector.csv",
        &build_linear_growth_rows(420, 950.0, 2.0),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let model_dir = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("path_event_models");
    fs::create_dir_all(&model_dir).expect("path event model dir should exist");
    let return_model_path = model_dir.join("return_head.json");
    let drawdown_model_path = model_dir.join("drawdown_head.json");
    let path_model_path = model_dir.join("path_quality_head.json");
    let upside_model_path = model_dir.join("upside_first_head.json");
    let stop_model_path = model_dir.join("stop_first_head.json");
    fs::write(
        &return_model_path,
        build_regression_head_artifact_json("return_head", 0.118, 0.135),
    )
    .expect("return head model should be written");
    fs::write(
        &drawdown_model_path,
        build_regression_head_artifact_json("drawdown_head", 0.038, 0.031),
    )
    .expect("drawdown head model should be written");
    fs::write(
        &path_model_path,
        build_regression_head_artifact_json("path_quality_head", 72.0, 81.0),
    )
    .expect("path quality head model should be written");
    fs::write(
        &upside_model_path,
        build_classification_head_artifact_json("upside_first_head", 0.62, 0.84),
    )
    .expect("upside-first head model should be written");
    fs::write(
        &stop_model_path,
        build_classification_head_artifact_json("stop_first_head", 0.31, 0.12),
    )
    .expect("stop-first head model should be written");

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

    let request = json!({
        "tool": "security_master_scorecard",
        "args": {
            "symbol": "601916.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "as_of_date": "2025-08-28",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12,
            "created_at": "2026-04-11T23:55:00+08:00",
            "return_head_model_path": return_model_path.to_string_lossy(),
            "drawdown_head_model_path": drawdown_model_path.to_string_lossy(),
            "path_quality_head_model_path": path_model_path.to_string_lossy(),
            "upside_first_head_model_path": upside_model_path.to_string_lossy(),
            "stop_first_head_model_path": stop_model_path.to_string_lossy()
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
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

    // 2026-04-11 CST: Add a path-event-context red test, because P4 must let the
    // governed master scorecard expose upside-first versus stop-first asymmetry
    // instead of leaving those event heads invisible to downstream decisions.
    // Purpose: lock auditable path-event summary fields before the chair starts
    // referencing them in final execution constraints.
    assert_eq!(output["status"], "ok");
    assert!(
        output["data"]["master_scorecard"]["trained_head_summary"]["expected_upside_first_probability"]
            .is_number(),
        "path-event summary should expose upside-first probability"
    );
    assert!(
        output["data"]["master_scorecard"]["trained_head_summary"]["expected_stop_first_probability"]
            .is_number(),
        "path-event summary should expose stop-first probability"
    );
    assert_eq!(
        output["data"]["master_scorecard"]["trained_head_summary"]["head_count"],
        5
    );
}

#[test]
fn security_master_scorecard_supports_prediction_mode_180d() {
    let runtime_db_path = create_test_runtime_db("security_master_scorecard_prediction_mode");

    let stock_csv = create_stock_history_csv(
        "security_master_scorecard_prediction_mode",
        "stock.csv",
        &build_linear_growth_rows(560, 100.0, 0.6),
    );
    let market_csv = create_stock_history_csv(
        "security_master_scorecard_prediction_mode",
        "market.csv",
        &build_linear_growth_rows(560, 3200.0, 2.5),
    );
    let sector_csv = create_stock_history_csv(
        "security_master_scorecard_prediction_mode",
        "sector.csv",
        &build_linear_growth_rows(560, 950.0, 1.2),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let model_dir = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("prediction_mode_models");
    fs::create_dir_all(&model_dir).expect("prediction mode model dir should exist");
    let direction_model_path = model_dir.join("direction_head.json");
    let return_model_path = model_dir.join("return_head.json");
    let drawdown_model_path = model_dir.join("drawdown_head.json");
    let path_model_path = model_dir.join("path_quality_head.json");
    let upside_model_path = model_dir.join("upside_first_head.json");
    let stop_model_path = model_dir.join("stop_first_head.json");
    fs::write(
        &direction_model_path,
        build_classification_head_artifact_json("direction_head", 0.55, 0.78),
    )
    .expect("direction head model should be written");
    fs::write(
        &return_model_path,
        build_regression_head_artifact_json("return_head", 0.082, 0.126),
    )
    .expect("return head model should be written");
    fs::write(
        &drawdown_model_path,
        build_regression_head_artifact_json("drawdown_head", 0.061, 0.034),
    )
    .expect("drawdown head model should be written");
    fs::write(
        &path_model_path,
        build_regression_head_artifact_json("path_quality_head", 58.0, 83.0),
    )
    .expect("path quality head model should be written");
    fs::write(
        &upside_model_path,
        build_classification_head_artifact_json("upside_first_head", 0.52, 0.81),
    )
    .expect("upside-first head model should be written");
    fs::write(
        &stop_model_path,
        build_classification_head_artifact_json("stop_first_head", 0.33, 0.14),
    )
    .expect("stop-first head model should be written");

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
                        {"notice_date":"2026-03-28","title":"2025骞村害鎶ュ憡","art_code":"AN202603281234567890","columns":[{"column_name":"瀹氭湡鎶ュ憡"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_master_scorecard",
        "args": {
            "symbol": "601916.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "as_of_date": "2026-04-12",
            "prediction_mode": "prediction",
            "prediction_horizon_days": 180,
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12,
            "created_at": "2026-04-12T12:00:00+08:00",
            "scorecard_model_path": direction_model_path.to_string_lossy(),
            "return_head_model_path": return_model_path.to_string_lossy(),
            "drawdown_head_model_path": drawdown_model_path.to_string_lossy(),
            "path_quality_head_model_path": path_model_path.to_string_lossy(),
            "upside_first_head_model_path": upside_model_path.to_string_lossy(),
            "stop_first_head_model_path": stop_model_path.to_string_lossy()
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
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

    // 2026-04-12 UTC+08: Lock the new future-looking 180d contract, because the
    // product requirement has shifted from historical replay to prediction from
    // the current analysis date.
    // Purpose: require master_scorecard to emit regression, risk, and cluster /
    // analog summaries even when no future replay rows exist yet.
    assert_eq!(
        output["status"], "ok",
        "unexpected prediction output: {output}"
    );
    assert_eq!(
        output["data"]["master_scorecard"]["aggregation_status"],
        "future_prediction_quant_context"
    );
    assert_eq!(
        output["data"]["master_scorecard"]["prediction_summary"]["prediction_mode"],
        "prediction"
    );
    assert_eq!(
        output["data"]["master_scorecard"]["prediction_summary"]["prediction_horizon_days"],
        180
    );
    // 2026-04-16 CST: Added because prediction mode must preserve the same public bridge
    // artifacts as replay mode after the adapter was attached to both branches.
    // Reason: the mainline helper now fans into two execution paths, and this test prevents
    // one branch from losing the composite or committee payload output during future edits.
    // Purpose: freeze branch parity at the external tool contract level.
    assert_eq!(
        output["data"]["composite_scorecard"]["document_type"],
        "security_composite_scorecard"
    );
    assert_eq!(
        output["data"]["committee_payload_adapter"]["committee_schema_version"],
        "committee-payload:v1"
    );
    assert_eq!(
        output["data"]["committee_payload_adapter"]["odds_digest"]["status"],
        "available"
    );
    assert_eq!(
        output["data"]["committee_payload_adapter"]["subject_profile"]["asset_class"],
        "equity"
    );
    assert_eq!(
        output["data"]["committee_payload_adapter"]["position_digest"]["regime_adjustment"],
        output["data"]["composite_scorecard"]["composite_actionability"]
    );
    assert!(
        output["data"]["master_scorecard"]["prediction_summary"]["regression_line"]["expected_return"]
            .is_number(),
        "prediction summary should expose regression expected return"
    );
    assert!(
        output["data"]["master_scorecard"]["prediction_summary"]["risk_line"]["expected_drawdown"]
            .is_number(),
        "prediction summary should expose risk expected drawdown"
    );
    assert!(
        output["data"]["master_scorecard"]["prediction_summary"]["cluster_line"]["regime_cluster_label"]
            .is_string(),
        "prediction summary should expose regime cluster label"
    );
    assert!(
        output["data"]["master_scorecard"]["prediction_summary"]["cluster_line"]["analog_sample_count"]
            .as_i64()
            .unwrap_or_default()
            > 0,
        "prediction summary should expose analog sample count"
    );
}

fn find_horizon<'a>(
    horizon_breakdown: &'a [serde_json::Value],
    horizon_days: i64,
) -> &'a serde_json::Value {
    horizon_breakdown
        .iter()
        .find(|item| item["horizon_days"].as_i64() == Some(horizon_days))
        .expect("requested horizon should exist")
}

fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_master_scorecard_fixture"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path.to_path_buf(),
        &[],
    );
    assert_eq!(output["status"], "ok");
}

fn build_regression_head_artifact_json(
    target_head: &str,
    baseline_value: f64,
    technical_only_value: f64,
) -> String {
    json!({
        "model_id": format!("a_share_equity_10d_{target_head}"),
        "model_version": "candidate_test",
        "label_definition": "security_forward_outcome.v1",
        "target_head": target_head,
        "prediction_mode": "regression",
        "prediction_baseline": baseline_value,
        "base_score": baseline_value,
        "training_window": "2025-03-01..2025-08-31",
        "oot_window": "2025-12-01..2026-01-31",
        "features": [
            {
                "feature_name": "integrated_stance",
                "group_name": "M",
                "bins": [
                    {
                        "bin_label": "technical_only",
                        "match_values": ["integrated_stance:technical_only"],
                        "predicted_value": technical_only_value,
                        "points": 0.0
                    }
                ]
            }
        ]
    })
    .to_string()
}

fn build_classification_head_artifact_json(
    target_head: &str,
    baseline_probability: f64,
    technical_only_probability: f64,
) -> String {
    let baseline_logit = (baseline_probability / (1.0 - baseline_probability)).ln();
    let technical_only_logit =
        (technical_only_probability / (1.0 - technical_only_probability)).ln();
    json!({
        "model_id": format!("a_share_equity_10d_{target_head}"),
        "model_version": "candidate_test",
        "label_definition": "security_forward_outcome.v1",
        "target_head": target_head,
        "prediction_mode": "classification",
        "base_score": 600.0,
        "training_window": "2025-03-01..2025-08-31",
        "oot_window": "2025-12-01..2026-01-31",
        "intercept": baseline_logit,
        "features": [
            {
                "feature_name": "integrated_stance",
                "group_name": "M",
                "bins": [
                    {
                        "bin_label": "technical_only",
                        "match_values": ["integrated_stance:technical_only"],
                        "woe": technical_only_logit - baseline_logit,
                        "logit_contribution": technical_only_logit - baseline_logit,
                        "points": 12.0
                    }
                ]
            }
        ]
    })
    .to_string()
}

// 2026-04-11 CST: 这里沿用单边上涨样本，原因是最小总卡测试的重点不是重新校验行情输入链，而是校验赚钱效益汇总对象；
// 目的：先用低噪声样本锁住“未来多期限正收益时，总卡应给出明显偏正历史效果”的正式合同。
fn build_linear_growth_rows(day_count: usize, start_close: f64, daily_step: f64) -> Vec<String> {
    let mut rows = vec!["trade_date,open,high,low,close,adj_close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("seed date should be valid");
    let mut close = start_close;

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let next_close = close + daily_step;
        let open = close;
        let high = next_close.max(open) + 0.8;
        let low = next_close.min(open) - 0.6;
        let volume = 900_000 + offset as i64 * 2_000;
        rows.push(format!(
            "{},{open:.2},{high:.2},{low:.2},{next_close:.2},{next_close:.2},{volume}",
            trade_date.format("%Y-%m-%d")
        ));
        close = next_close;
    }

    rows
}
