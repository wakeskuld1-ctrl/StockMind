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
    create_test_runtime_db, run_cli_with_json, run_cli_with_json_runtime_and_envs,
};

// 2026-04-09 CST: 这里新增主席裁决 CLI 测试夹具，原因是 Task 1 需要先把“量化线 / 投委会线 / 主席线”
// 明确拆成正式对外契约；目的：先把最终正式决议只能由主席对象输出这一点锁进红测，再做最小实现。
fn create_stock_history_csv(prefix: &str, file_name: &str, rows: &[String]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_chair_resolution")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("security chair resolution fixture dir should exist");

    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n")).expect("security chair resolution csv should be written");
    csv_path
}

// 2026-04-09 CST: 这里复用本地 HTTP 假服务，原因是主席裁决测试仍然需要基于稳定的财报/公告输入构造同源证据；
// 目的：避免外部接口波动干扰“最终正式决议出口”这条主线回归测试。
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
fn tool_catalog_includes_security_chair_resolution() {
    let output = run_cli_with_json("");

    // 2026-04-09 CST: 这里先锁主席裁决 Tool 的可发现性，原因是如果 catalog 不暴露它，
    // 那么“主席才是唯一正式决议出口”就无法成为真正产品能力；目的：确保 CLI / Skill / 后续 package 都能稳定发现这条线。
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_chair_resolution")
    );
}

#[test]
fn security_chair_resolution_outputs_formal_final_action_separate_from_committee_and_scorecard() {
    let runtime_db_path = create_test_runtime_db("security_chair_resolution_ready");

    let stock_csv = create_stock_history_csv(
        "security_chair_resolution_ready",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_chair_resolution_ready",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_chair_resolution_ready",
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

    let request = json!({
        "tool": "security_chair_resolution",
        "args": {
            "symbol": "601916.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12,
            "created_at": "2026-04-09T12:00:00+08:00"
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

    // 2026-04-09 CST: 这里先锁三线强隔离的最小正式契约，原因是本轮不是继续把投委会结果直接当最终建议输出，
    // 而是要求主席单独读入量化线和投委会线后形成唯一正式动作；目的：确保 chair_resolution 成为正式最终决议对象，
    // 同时 committee 和 scorecard 仍然各自保留为独立输入线。
    assert_eq!(output["status"], "ok", "unexpected chair output: {output}");
    assert_eq!(
        output["data"]["scorecard"]["document_type"],
        "security_scorecard"
    );
    assert_eq!(
        output["data"]["chair_resolution"]["document_type"],
        "security_chair_resolution"
    );
    assert_eq!(
        output["data"]["chair_resolution"]["selected_action"],
        output["data"]["committee_result"]["decision_card"]["recommendation_action"]
    );
    assert_eq!(
        output["data"]["chair_resolution"]["selected_exposure_side"],
        output["data"]["committee_result"]["decision_card"]["exposure_side"]
    );
    assert!(
        output["data"]["chair_resolution"]["master_scorecard_ref"]
            .as_str()
            .expect("master scorecard ref should exist")
            .starts_with("master-scorecard-"),
        "chair resolution should point to the formal master scorecard object"
    );
    assert!(
        output["data"]["chair_resolution"]["committee_session_ref"]
            .as_str()
            .expect("committee session ref should exist")
            .starts_with("committee-")
    );
}

#[test]
fn security_chair_resolution_hydrates_historical_proxy_backfill_for_gold_etf() {
    let runtime_db_path = create_test_runtime_db("security_chair_resolution_gold_historical_proxy");
    let external_proxy_db_path = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("security_external_proxy.db");

    let etf_csv = create_stock_history_csv(
        "security_chair_resolution_gold_historical_proxy",
        "gold_etf.csv",
        &build_confirmed_breakout_rows(260, 101.0),
    );
    let market_csv = create_stock_history_csv(
        "security_chair_resolution_gold_historical_proxy",
        "market.csv",
        &build_confirmed_breakout_rows(260, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_chair_resolution_gold_historical_proxy",
        "sector.csv",
        &build_confirmed_breakout_rows(260, 99.0),
    );
    import_history_csv(&runtime_db_path, &etf_csv, "518880.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "518800.SH");

    let backfill_request = json!({
        "tool": "security_external_proxy_backfill",
        "args": {
            "batch_id": "chair-gold-historical-proxy",
            "created_at": "2026-04-12T09:00:00+08:00",
            "records": [{
                "symbol": "518880.SH",
                "as_of_date": "2025-08-08",
                "instrument_subscope": "gold_etf",
                "external_proxy_inputs": {
                    "gold_spot_proxy_status": "manual_bound",
                    "gold_spot_proxy_return_5d": 0.021019,
                    "usd_index_proxy_status": "manual_bound",
                    "usd_index_proxy_return_5d": -0.003841,
                    "real_rate_proxy_status": "manual_bound",
                    "real_rate_proxy_delta_bp_5d": -2.0
                }
            }]
        }
    });
    let backfill_output = run_cli_with_json_runtime_and_envs(
        &backfill_request.to_string(),
        &runtime_db_path,
        &[(
            "EXCEL_SKILL_EXTERNAL_PROXY_DB",
            external_proxy_db_path.to_string_lossy().to_string(),
        )],
    );
    assert_eq!(backfill_output["status"], "ok");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "<html><body>financials unavailable for gold etf fixture</body></html>",
            "text/html",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{"data":{"list":[]}}"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_chair_resolution",
        "args": {
            "symbol": "518880.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "518800.SH",
            "market_profile": "a_share_core",
            "sector_profile": "gold_etf_peer",
            "as_of_date": "2025-08-08",
            "created_at": "2026-04-12T09:05:00+08:00"
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
            (
                "EXCEL_SKILL_EXTERNAL_PROXY_DB",
                external_proxy_db_path.to_string_lossy().to_string(),
            ),
        ],
    );

    // 2026-04-12 UTC+08: Add a deep-chain ETF proxy regression here, because the
    // current governed gold ETF history can already hydrate feature snapshots but
    // still drops out before the final chair path reads the scorecard snapshot.
    // Purpose: lock that historical ETF proxy fields must survive through
    // committee, scorecard, master scorecard, and chair resolution.
    assert_eq!(output["status"], "ok", "unexpected chair output: {output}");
    assert_eq!(
        output["data"]["scorecard"]["raw_feature_snapshot"]["gold_spot_proxy_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["scorecard"]["raw_feature_snapshot"]["usd_index_proxy_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["scorecard"]["raw_feature_snapshot"]["real_rate_proxy_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["scorecard"]["raw_feature_snapshot"]["gold_spot_proxy_return_5d"],
        json!(0.021019)
    );
}

#[test]
fn security_chair_resolution_does_not_require_stock_only_information_for_gold_etf_when_proxy_history_is_complete()
 {
    let runtime_db_path =
        create_test_runtime_db("security_chair_resolution_gold_etf_info_complete");
    let external_proxy_db_path = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("security_external_proxy.db");

    let etf_csv = create_stock_history_csv(
        "security_chair_resolution_gold_etf_info_complete",
        "gold_etf.csv",
        &build_confirmed_breakout_rows(260, 101.0),
    );
    let market_csv = create_stock_history_csv(
        "security_chair_resolution_gold_etf_info_complete",
        "market.csv",
        &build_confirmed_breakout_rows(260, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_chair_resolution_gold_etf_info_complete",
        "sector.csv",
        &build_confirmed_breakout_rows(260, 99.0),
    );
    import_history_csv(&runtime_db_path, &etf_csv, "518880.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "518800.SH");

    let backfill_request = json!({
        "tool": "security_external_proxy_backfill",
        "args": {
            "batch_id": "chair-gold-etf-info-complete",
            "created_at": "2026-04-13T10:05:00+08:00",
            "records": [{
                "symbol": "518880.SH",
                "as_of_date": "2025-08-08",
                "instrument_subscope": "gold_etf",
                "external_proxy_inputs": {
                    "gold_spot_proxy_status": "manual_bound",
                    "gold_spot_proxy_return_5d": 0.021019,
                    "usd_index_proxy_status": "manual_bound",
                    "usd_index_proxy_return_5d": -0.003841,
                    "real_rate_proxy_status": "manual_bound",
                    "real_rate_proxy_delta_bp_5d": -2.0
                }
            }]
        }
    });
    let backfill_output = run_cli_with_json_runtime_and_envs(
        &backfill_request.to_string(),
        &runtime_db_path,
        &[(
            "EXCEL_SKILL_EXTERNAL_PROXY_DB",
            external_proxy_db_path.to_string_lossy().to_string(),
        )],
    );
    assert_eq!(backfill_output["status"], "ok");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "<html><body>financials unavailable for ETF fixture</body></html>",
            "text/html",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{"data":{"list":[]}}"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_chair_resolution",
        "args": {
            "symbol": "518880.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "518800.SH",
            "market_profile": "a_share_core",
            "sector_profile": "gold_etf_peer",
            "as_of_date": "2025-08-08",
            "created_at": "2026-04-13T10:10:00+08:00"
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
            (
                "EXCEL_SKILL_EXTERNAL_PROXY_DB",
                external_proxy_db_path.to_string_lossy().to_string(),
            ),
        ],
    );

    // 2026-04-13 UTC+08: Add a final-chair ETF governance assertion here, because
    // the current stack must prove the chair path now consumes ETF-native
    // governed evidence instead of requiring stock-only information contracts.
    // Purpose: assert against the real committee/chair fields that carry ETF
    // completeness and risk-veto semantics after the governed proxy fix.
    assert_eq!(
        output["status"], "ok",
        "unexpected gold ETF chair output: {output}"
    );
    assert_eq!(
        output["data"]["committee_result"]["evidence_bundle"]["evidence_quality"]["overall_status"],
        "complete"
    );
    assert_eq!(
        output["data"]["committee_result"]["risk_veto"]["status"],
        "none"
    );
    assert!(
        !output["data"]["chair_resolution"]["execution_constraints"]
            .as_array()
            .expect("execution constraints should exist")
            .iter()
            .filter_map(|item| item.as_str())
            .any(|item| item.contains("基本面") || item.contains("公告")),
        "gold ETF chair output should no longer request stock-only information backfill: {output}"
    );
}

#[test]
fn security_chair_resolution_accepts_treasury_etf_subscope_artifact() {
    let runtime_db_path = create_test_runtime_db("security_chair_resolution_treasury_artifact");
    let external_proxy_db_path = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("security_external_proxy.db");

    let etf_csv = create_stock_history_csv(
        "security_chair_resolution_treasury_artifact",
        "treasury_etf.csv",
        &build_confirmed_breakout_rows(260, 101.0),
    );
    let market_csv = create_stock_history_csv(
        "security_chair_resolution_treasury_artifact",
        "market.csv",
        &build_confirmed_breakout_rows(260, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_chair_resolution_treasury_artifact",
        "sector.csv",
        &build_confirmed_breakout_rows(260, 110.0),
    );
    import_history_csv(&runtime_db_path, &etf_csv, "511010.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "511060.SH");

    let backfill_request = json!({
        "tool": "security_external_proxy_backfill",
        "args": {
            "batch_id": "chair-treasury-historical-proxy",
            "created_at": "2026-04-12T09:10:00+08:00",
            "records": [{
                "symbol": "511010.SH",
                "as_of_date": "2025-08-08",
                "instrument_subscope": "treasury_etf",
                "external_proxy_inputs": {
                    "yield_curve_proxy_status": "manual_bound",
                    "yield_curve_slope_delta_bp_5d": -3.0,
                    "funding_liquidity_proxy_status": "manual_bound",
                    "funding_liquidity_spread_delta_bp_5d": 7.0
                }
            }]
        }
    });
    let backfill_output = run_cli_with_json_runtime_and_envs(
        &backfill_request.to_string(),
        &runtime_db_path,
        &[(
            "EXCEL_SKILL_EXTERNAL_PROXY_DB",
            external_proxy_db_path.to_string_lossy().to_string(),
        )],
    );
    assert_eq!(backfill_output["status"], "ok");

    let model_dir = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("etf_models");
    fs::create_dir_all(&model_dir).expect("etf model dir should exist");
    let scorecard_model_path = model_dir.join("treasury_direction_head.json");
    fs::write(
        &scorecard_model_path,
        build_etf_direction_artifact_json(
            "treasury_etf",
            vec![
                ("close_vs_sma200", json!([{"bin_label":"positive","min_inclusive":-10.0,"max_exclusive":10.0,"points":12.0}])),
                ("boll_width_ratio_20", json!([{"bin_label":"stable","min_inclusive":0.0,"max_exclusive":10.0,"points":8.0}])),
                ("atr_14", json!([{"bin_label":"normal","min_inclusive":0.0,"max_exclusive":10.0,"points":8.0}])),
                ("rsrs_zscore_18_60", json!([{"bin_label":"balanced","min_inclusive":-10.0,"max_exclusive":10.0,"points":8.0}])),
                ("yield_curve_proxy_status", json!([{"bin_label":"bound","match_values":["manual_bound"],"points":10.0}])),
                ("yield_curve_slope_delta_bp_5d", json!([{"bin_label":"delta","min_inclusive":-100.0,"max_exclusive":100.0,"points":10.0}])),
                ("funding_liquidity_proxy_status", json!([{"bin_label":"bound","match_values":["manual_bound"],"points":10.0}])),
                ("funding_liquidity_spread_delta_bp_5d", json!([{"bin_label":"spread","min_inclusive":-100.0,"max_exclusive":100.0,"points":10.0}]))
            ],
        ),
    )
    .expect("treasury etf scorecard model should be written");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "<html><body>financials unavailable for treasury etf fixture</body></html>",
            "text/html",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{"data":{"list":[]}}"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_chair_resolution",
        "args": {
            "symbol": "511010.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "511060.SH",
            "market_profile": "a_share_core",
            "sector_profile": "treasury_etf",
            "as_of_date": "2025-08-08",
            "created_at": "2026-04-12T09:15:00+08:00",
            "scorecard_model_path": scorecard_model_path.to_string_lossy()
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
            (
                "EXCEL_SKILL_EXTERNAL_PROXY_DB",
                external_proxy_db_path.to_string_lossy().to_string(),
            ),
        ],
    );

    // 2026-04-12 UTC+08: Add a treasury ETF artifact-consumption regression here,
    // because the final chair path currently stops at cross_section_invalid when
    // the ETF sub-pool model binding is wrong or missing.
    // Purpose: lock one governed treasury ETF artifact that the scorecard runtime
    // can accept as a structurally valid final-chain binding.
    assert_eq!(output["status"], "ok", "unexpected chair output: {output}");
    assert_eq!(output["data"]["scorecard"]["score_status"], "ready");
    assert_eq!(
        output["data"]["scorecard"]["model_binding"]["instrument_subscope"],
        "treasury_etf"
    );
    assert_eq!(
        output["data"]["scorecard"]["raw_feature_snapshot"]["yield_curve_proxy_status"],
        "manual_bound"
    );
}

#[test]
fn security_chair_resolution_uses_latest_bound_treasury_proxy_on_non_trading_date() {
    let runtime_db_path =
        create_test_runtime_db("security_chair_resolution_treasury_non_trading_proxy");
    let external_proxy_db_path = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("security_external_proxy.db");

    let etf_csv = create_stock_history_csv(
        "security_chair_resolution_treasury_non_trading_proxy",
        "treasury_etf.csv",
        &build_confirmed_breakout_rows(548, 101.0),
    );
    let market_csv = create_stock_history_csv(
        "security_chair_resolution_treasury_non_trading_proxy",
        "market.csv",
        &build_confirmed_breakout_rows(548, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_chair_resolution_treasury_non_trading_proxy",
        "sector.csv",
        &build_confirmed_breakout_rows(548, 110.0),
    );
    import_history_csv(&runtime_db_path, &etf_csv, "511010.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "511060.SH");

    let backfill_request = json!({
        "tool": "security_external_proxy_backfill",
        "args": {
            "batch_id": "chair-treasury-non-trading-proxy",
            "created_at": "2026-04-12T23:50:00+08:00",
            "records": [{
                "symbol": "511010.SH",
                "as_of_date": "2026-04-10",
                "instrument_subscope": "treasury_etf",
                "external_proxy_inputs": {
                    "yield_curve_proxy_status": "manual_bound",
                    "yield_curve_slope_delta_bp_5d": -3.0,
                    "funding_liquidity_proxy_status": "manual_bound",
                    "funding_liquidity_spread_delta_bp_5d": 7.0
                }
            }]
        }
    });
    let backfill_output = run_cli_with_json_runtime_and_envs(
        &backfill_request.to_string(),
        &runtime_db_path,
        &[(
            "EXCEL_SKILL_EXTERNAL_PROXY_DB",
            external_proxy_db_path.to_string_lossy().to_string(),
        )],
    );
    assert_eq!(backfill_output["status"], "ok");

    let model_dir = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("etf_models_non_trading");
    fs::create_dir_all(&model_dir).expect("etf model dir should exist");
    let scorecard_model_path = model_dir.join("treasury_direction_head.json");
    fs::write(
        &scorecard_model_path,
        build_etf_direction_artifact_json(
            "treasury_etf",
            vec![
                ("close_vs_sma200", json!([{"bin_label":"positive","min_inclusive":-10.0,"max_exclusive":10.0,"points":12.0}])),
                ("boll_width_ratio_20", json!([{"bin_label":"stable","min_inclusive":0.0,"max_exclusive":10.0,"points":8.0}])),
                ("atr_14", json!([{"bin_label":"normal","min_inclusive":0.0,"max_exclusive":10.0,"points":8.0}])),
                ("rsrs_zscore_18_60", json!([{"bin_label":"balanced","min_inclusive":-10.0,"max_exclusive":10.0,"points":8.0}])),
                ("yield_curve_proxy_status", json!([{"bin_label":"bound","match_values":["manual_bound"],"points":10.0}])),
                ("yield_curve_slope_delta_bp_5d", json!([{"bin_label":"delta","min_inclusive":-100.0,"max_exclusive":100.0,"points":10.0}])),
                ("funding_liquidity_proxy_status", json!([{"bin_label":"bound","match_values":["manual_bound"],"points":10.0}])),
                ("funding_liquidity_spread_delta_bp_5d", json!([{"bin_label":"spread","min_inclusive":-100.0,"max_exclusive":100.0,"points":10.0}]))
            ],
        ),
    )
    .expect("treasury etf scorecard model should be written");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "<html><body>financials unavailable for treasury etf fixture</body></html>",
            "text/html",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{"data":{"list":[]}}"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_chair_resolution",
        "args": {
            "symbol": "511010.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "511060.SH",
            "market_profile": "a_share_core",
            "sector_profile": "treasury_etf",
            "as_of_date": "2026-04-12",
            "created_at": "2026-04-12T23:55:00+08:00",
            "scorecard_model_path": scorecard_model_path.to_string_lossy()
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
            (
                "EXCEL_SKILL_EXTERNAL_PROXY_DB",
                external_proxy_db_path.to_string_lossy().to_string(),
            ),
        ],
    );

    // 2026-04-12 UTC+08: Add a non-trading ETF proxy fallback red test here,
    // because live 180d prediction requests use `2026-04-12` while the governed
    // treasury proxy history is dated `2026-04-10`.
    // Purpose: force the final chair chain to reuse the latest available ETF proxy
    // snapshot on or before the requested date instead of dropping back to stock-style
    // unavailable information semantics on weekends.
    assert_eq!(output["status"], "ok", "unexpected chair output: {output}");
    assert_eq!(output["data"]["scorecard"]["score_status"], "ready");
    assert_eq!(
        output["data"]["scorecard"]["raw_feature_snapshot"]["yield_curve_proxy_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["committee_result"]["evidence_bundle"]["fundamental_context"]["source"],
        "governed_etf_proxy_information"
    );
    assert_eq!(
        output["data"]["committee_result"]["evidence_bundle"]["disclosure_context"]["source"],
        "governed_etf_proxy_information"
    );
}

#[test]
fn security_chair_resolution_hydrates_latest_equity_etf_proxy_without_explicit_as_of_date() {
    let runtime_db_path =
        create_test_runtime_db("security_chair_resolution_equity_latest_proxy_without_as_of_date");
    let external_proxy_db_path = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("security_external_proxy.db");

    let etf_csv = create_stock_history_csv(
        "security_chair_resolution_equity_latest_proxy_without_as_of_date",
        "equity_etf.csv",
        &build_confirmed_breakout_rows(465, 101.0),
    );
    let market_csv = create_stock_history_csv(
        "security_chair_resolution_equity_latest_proxy_without_as_of_date",
        "market.csv",
        &build_confirmed_breakout_rows(465, 3200.0),
    );
    import_history_csv(&runtime_db_path, &etf_csv, "515790.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");

    let backfill_request = json!({
        "tool": "security_external_proxy_backfill",
        "args": {
            "batch_id": "chair-equity-latest-proxy-without-as-of-date",
            "created_at": "2026-04-12T23:58:00+08:00",
            "records": [{
                "symbol": "515790.SH",
                "as_of_date": "2026-04-10",
                "instrument_subscope": "equity_etf",
                "external_proxy_inputs": {
                    "etf_fund_flow_proxy_status": "manual_bound",
                    "etf_fund_flow_5d": 0.004019,
                    "premium_discount_proxy_status": "manual_bound",
                    "premium_discount_pct": -0.004321,
                    "benchmark_relative_strength_status": "manual_bound",
                    "benchmark_relative_return_5d": -0.004292
                }
            }]
        }
    });
    let backfill_output = run_cli_with_json_runtime_and_envs(
        &backfill_request.to_string(),
        &runtime_db_path,
        &[(
            "EXCEL_SKILL_EXTERNAL_PROXY_DB",
            external_proxy_db_path.to_string_lossy().to_string(),
        )],
    );
    assert_eq!(backfill_output["status"], "ok");

    let model_dir = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("etf_models_equity_latest_proxy");
    fs::create_dir_all(&model_dir).expect("equity etf model dir should exist");
    let scorecard_model_path = model_dir.join("equity_direction_head.json");
    // 2026-04-12 UTC+08: Add a focused latest-proxy regression artifact here, because
    // this bug is about the final chair path forgetting to hydrate X-group ETF proxy
    // status fields when the request omits `as_of_date`.
    // Purpose: keep the test narrowly aimed at latest-date proxy fallback instead of
    // mixing in unrelated pooled-training artifacts.
    fs::write(
        &scorecard_model_path,
        build_etf_direction_artifact_json(
            "equity_etf",
            vec![
                ("close_vs_sma50", json!([{"bin_label":"neutral","min_inclusive":-10.0,"max_exclusive":10.0,"points":12.0}])),
                ("close_vs_sma200", json!([{"bin_label":"neutral","min_inclusive":-10.0,"max_exclusive":10.0,"points":12.0}])),
                ("volume_ratio_20", json!([{"bin_label":"normal","min_inclusive":0.0,"max_exclusive":10.0,"points":8.0}])),
                ("support_gap_pct_20", json!([{"bin_label":"support","min_inclusive":-10.0,"max_exclusive":10.0,"points":8.0}])),
                ("resistance_gap_pct_20", json!([{"bin_label":"resistance","min_inclusive":-10.0,"max_exclusive":10.0,"points":8.0}])),
                ("rsrs_zscore_18_60", json!([{"bin_label":"balanced","min_inclusive":-10.0,"max_exclusive":10.0,"points":8.0}])),
                ("etf_fund_flow_proxy_status", json!([{"bin_label":"bound","match_values":["manual_bound"],"points":10.0}])),
                ("etf_fund_flow_5d", json!([{"bin_label":"flow","min_inclusive":-10.0,"max_exclusive":10.0,"points":8.0}])),
                ("premium_discount_proxy_status", json!([{"bin_label":"bound","match_values":["manual_bound"],"points":10.0}])),
                ("premium_discount_pct", json!([{"bin_label":"discount","min_inclusive":-10.0,"max_exclusive":10.0,"points":8.0}])),
                ("benchmark_relative_strength_status", json!([{"bin_label":"bound","match_values":["manual_bound"],"points":10.0}])),
                ("benchmark_relative_return_5d", json!([{"bin_label":"relative","min_inclusive":-10.0,"max_exclusive":10.0,"points":8.0}]))
            ],
        ),
    )
    .expect("equity etf scorecard model should be written");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "<html><body>financials unavailable for equity etf fixture</body></html>",
            "text/html",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{"data":{"list":[]}}"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_chair_resolution",
        "args": {
            "symbol": "515790.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "515790.SH",
            "market_profile": "a_share_core",
            "sector_profile": "equity_etf_peer",
            "created_at": "2026-04-12T23:59:00+08:00",
            "scorecard_model_path": scorecard_model_path.to_string_lossy()
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
            (
                "EXCEL_SKILL_EXTERNAL_PROXY_DB",
                external_proxy_db_path.to_string_lossy().to_string(),
            ),
        ],
    );

    // 2026-04-12 UTC+08: Lock the latest-date ETF proxy fallback here, because the
    // live pooled holdout repair showed that omitting `as_of_date` in latest runs
    // still dropped X-group status fields back to `placeholder_unbound`.
    // Purpose: guarantee that final chair calls inherit the resolved analysis date
    // when hydrating governed ETF proxy history.
    assert_eq!(
        output["status"], "ok",
        "unexpected equity ETF chair output: {output}"
    );
    assert_eq!(
        output["data"]["chair_resolution"]["analysis_date"],
        "2026-04-10"
    );
    assert_eq!(output["data"]["scorecard"]["score_status"], "ready");
    assert_eq!(
        output["data"]["scorecard"]["raw_feature_snapshot"]["etf_fund_flow_proxy_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["scorecard"]["raw_feature_snapshot"]["premium_discount_proxy_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["scorecard"]["raw_feature_snapshot"]["benchmark_relative_strength_status"],
        "manual_bound"
    );
}

#[test]
fn security_chair_resolution_downgrades_to_abstain_when_scorecard_model_is_unavailable() {
    // 2026-04-11 CST: 这里先补“无训练模型时主席不得直接给进攻动作”的红测，原因是用户要求运行时也要强制约束高确定性建议；
    // 目的：锁住 chair 在 `model_unavailable` 场景下必须把最终动作降级为非执行型结论，并显式转为中性暴露。
    let runtime_db_path = create_test_runtime_db("security_chair_resolution_no_model");

    let stock_csv = create_stock_history_csv(
        "security_chair_resolution_no_model",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_chair_resolution_no_model",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_chair_resolution_no_model",
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

    let request = json!({
        "tool": "security_chair_resolution",
        "args": {
            "symbol": "601916.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12,
            "created_at": "2026-04-11T17:00:00+08:00"
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

    assert_eq!(output["status"], "ok", "unexpected chair output: {output}");
    assert_eq!(
        output["data"]["scorecard"]["score_status"],
        "model_unavailable"
    );
    assert_eq!(
        output["data"]["chair_resolution"]["selected_action"],
        "abstain"
    );
    assert_eq!(
        output["data"]["chair_resolution"]["selected_exposure_side"],
        "neutral"
    );
    // 2026-04-13 CST: Add the chair-side entry-layer regression, because the
    // first-stage governed entry signal must be readable from the final chair
    // object as well as the position-plan object.
    // Purpose: enforce the "position_plan + chair" dual-anchor contract.
    assert_eq!(output["data"]["chair_resolution"]["entry_grade"], "watch");
    assert_eq!(output["data"]["chair_resolution"]["target_gross_pct"], 0.01);
    assert_eq!(
        output["data"]["chair_resolution"]["sizing_grade"],
        "watch_probe"
    );
    assert!(
        output["data"]["chair_resolution"]["entry_reason"]
            .as_str()
            .expect("entry reason should exist")
            .contains("scorecard")
    );
    assert!(
        output["data"]["chair_resolution"]["chair_reasoning"]
            .as_str()
            .expect("chair reasoning should exist")
            .contains("训练")
    );
}

#[test]
fn security_chair_resolution_preserves_partial_multi_head_constraints_when_three_heads_are_available()
 {
    let runtime_db_path = create_test_runtime_db("security_chair_resolution_multi_head");

    let stock_csv = create_stock_history_csv(
        "security_chair_resolution_multi_head",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_chair_resolution_multi_head",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_chair_resolution_multi_head",
        "sector.csv",
        &build_confirmed_breakout_rows(220, 950.0),
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
        build_regression_head_artifact_json("return_head", 0.10, 0.14),
    )
    .expect("return head model should be written");
    fs::write(
        &drawdown_model_path,
        build_regression_head_artifact_json("drawdown_head", 0.04, 0.03),
    )
    .expect("drawdown head model should be written");
    fs::write(
        &path_model_path,
        build_regression_head_artifact_json("path_quality_head", 70.0, 84.0),
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
        "tool": "security_chair_resolution",
        "args": {
            "symbol": "601916.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12,
            "created_at": "2026-04-11T23:20:00+08:00",
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

    // 2026-04-11 CST: Lock chair-level partial multi-head consumption, because the
    // governed runtime now reserves the full multi-head wording for the five-head
    // path-event-ready case while three-head context still needs to survive.
    // Purpose: make the final chair object keep drawdown/path constraints even
    // before upside-first and stop-first are attached.
    assert_eq!(output["status"], "ok", "unexpected chair output: {output}");
    assert!(
        output["data"]["chair_resolution"]["execution_constraints"]
            .as_array()
            .expect("execution constraints should exist")
            .iter()
            .any(|item| item
                .as_str()
                .expect("constraint should be string")
                .contains("expected drawdown")),
        "execution constraints should reference trained drawdown context"
    );
}

#[test]
fn security_chair_resolution_references_path_event_asymmetry_when_available() {
    let runtime_db_path = create_test_runtime_db("security_chair_resolution_path_events");

    let stock_csv = create_stock_history_csv(
        "security_chair_resolution_path_events",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_chair_resolution_path_events",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_chair_resolution_path_events",
        "sector.csv",
        &build_confirmed_breakout_rows(220, 950.0),
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
        build_regression_head_artifact_json("return_head", 0.10, 0.14),
    )
    .expect("return head model should be written");
    fs::write(
        &drawdown_model_path,
        build_regression_head_artifact_json("drawdown_head", 0.04, 0.03),
    )
    .expect("drawdown head model should be written");
    fs::write(
        &path_model_path,
        build_regression_head_artifact_json("path_quality_head", 70.0, 84.0),
    )
    .expect("path quality head model should be written");
    fs::write(
        &upside_model_path,
        build_classification_head_artifact_json("upside_first_head", 0.58, 0.83),
    )
    .expect("upside-first head model should be written");
    fs::write(
        &stop_model_path,
        build_classification_head_artifact_json("stop_first_head", 0.27, 0.11),
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
                        {"notice_date":"2026-03-28","title":"2025骞村害鎶ュ憡","art_code":"AN202603281234567890","columns":[{"column_name":"瀹氭湡鎶ュ憡"}]},
                        {"notice_date":"2026-03-28","title":"2025骞村害鍒╂鼎鍒嗛厤棰勬鍏憡","art_code":"AN202603281234567891","columns":[{"column_name":"鍏徃鍏憡"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_chair_resolution",
        "args": {
            "symbol": "601916.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
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

    // 2026-04-11 CST: Add a path-event chair red test, because P4 must prove the
    // final formal decision can read upside-first versus stop-first asymmetry from
    // the governed master scorecard instead of ignoring the new path-event heads.
    // Purpose: lock the execution-constraint language before we claim path-event
    // context is part of the live decision chain.
    assert_eq!(output["status"], "ok", "unexpected chair output: {output}");
    assert!(
        output["data"]["chair_resolution"]["chair_reasoning"]
            .as_str()
            .expect("chair reasoning should exist")
            .contains("upside-first probability"),
        "chair reasoning should mention upside-first probability"
    );
    assert!(
        output["data"]["chair_resolution"]["execution_constraints"]
            .as_array()
            .expect("execution constraints should exist")
            .iter()
            .any(|item| item
                .as_str()
                .expect("constraint should be string")
                .contains("path-event asymmetry")),
        "execution constraints should include the path-event asymmetry guard"
    );
}

#[test]
fn security_chair_resolution_reads_prediction_mode_180d_context() {
    let runtime_db_path = create_test_runtime_db("security_chair_resolution_prediction_mode");

    let stock_csv = create_stock_history_csv(
        "security_chair_resolution_prediction_mode",
        "stock.csv",
        &build_confirmed_breakout_rows(560, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_chair_resolution_prediction_mode",
        "market.csv",
        &build_confirmed_breakout_rows(560, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_chair_resolution_prediction_mode",
        "sector.csv",
        &build_confirmed_breakout_rows(560, 950.0),
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
        build_classification_head_artifact_json("direction_head", 0.54, 0.79),
    )
    .expect("direction head model should be written");
    fs::write(
        &return_model_path,
        build_regression_head_artifact_json("return_head", 0.081, 0.133),
    )
    .expect("return head model should be written");
    fs::write(
        &drawdown_model_path,
        build_regression_head_artifact_json("drawdown_head", 0.062, 0.036),
    )
    .expect("drawdown head model should be written");
    fs::write(
        &path_model_path,
        build_regression_head_artifact_json("path_quality_head", 57.0, 86.0),
    )
    .expect("path quality head model should be written");
    fs::write(
        &upside_model_path,
        build_classification_head_artifact_json("upside_first_head", 0.53, 0.83),
    )
    .expect("upside-first head model should be written");
    fs::write(
        &stop_model_path,
        build_classification_head_artifact_json("stop_first_head", 0.31, 0.13),
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
        "tool": "security_chair_resolution",
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
            "created_at": "2026-04-12T12:20:00+08:00",
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

    // 2026-04-12 UTC+08: Lock chair-level future prediction semantics, because
    // the chair must explain 180d forward-looking regression, risk, and cluster
    // evidence instead of falling back to replay-only wording.
    // Purpose: make the last formal decision object consume prediction-mode
    // context explicitly before we claim the stack supports future 180d analysis.
    assert_eq!(
        output["status"], "ok",
        "unexpected prediction chair output: {output}"
    );
    assert!(
        output["data"]["chair_resolution"]["chair_reasoning"]
            .as_str()
            .expect("chair reasoning should exist")
            .contains("prediction-mode quant context"),
        "chair reasoning should mention prediction-mode quant context"
    );
    assert!(
        output["data"]["chair_resolution"]["execution_constraints"]
            .as_array()
            .expect("execution constraints should exist")
            .iter()
            .any(|item| item
                .as_str()
                .expect("constraint should be string")
                .contains("regime cluster")),
        "execution constraints should reference regime cluster context"
    );
}

fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_chair_resolution_fixture"
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

fn build_etf_direction_artifact_json(
    instrument_subscope: &str,
    feature_bins: Vec<(&str, Value)>,
) -> String {
    let features = feature_bins
        .into_iter()
        .map(|(feature_name, bins): (&str, Value)| {
            json!({
                "feature_name": feature_name,
                "group_name": if feature_name.ends_with("_status") { "X" } else { "T" },
                "bins": bins,
            })
        })
        .collect::<Vec<_>>();

    json!({
        "model_id": format!("a_share_etf_{}_10d_direction_head", instrument_subscope),
        "model_version": "candidate_test",
        "label_definition": "security_forward_outcome.v1",
        "target_head": "direction_head",
        "base_score": 620.0,
        "training_window": "2025-01-22..2025-02-13",
        "oot_window": "2025-03-04..2025-03-11",
        "instrument_subscope": instrument_subscope,
        "intercept": 0.0,
        "features": features
    })
    .to_string()
}

// 2026-04-09 CST: 这里沿用稳定上行且末段突破的样本，原因是主席裁决测试的目标不是重新验证行情输入链，
// 而是验证“三线输入 -> 正式最终决议输出”的对象边界；目的：把噪声尽量压低，让失败点落在 chair_resolution 契约本身。
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
