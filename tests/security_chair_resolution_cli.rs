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

const FIXTURE_AS_OF_DATE: &str = "2025-08-08";

// 2026-04-09 CST: 杩欓噷鏂板涓诲腑瑁佸喅 CLI 娴嬭瘯澶瑰叿锛屽師鍥犳槸 Task 1 闇€瑕佸厛鎶娾€滈噺鍖栫嚎 / 鎶曞浼氱嚎 / 涓诲腑绾库€?// 鏄庣‘鎷嗘垚姝ｅ紡瀵瑰濂戠害锛涚洰鐨勶細鍏堟妸鏈€缁堟寮忓喅璁彧鑳界敱涓诲腑瀵硅薄杈撳嚭杩欎竴鐐归攣杩涚孩娴嬶紝鍐嶅仛鏈€灏忓疄鐜般€?
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

// 2026-04-09 CST: 杩欓噷澶嶇敤鏈湴 HTTP 鍋囨湇鍔★紝鍘熷洜鏄富甯鍐虫祴璇曚粛鐒堕渶瑕佸熀浜庣ǔ瀹氱殑璐㈡姤/鍏憡杈撳叆鏋勯€犲悓婧愯瘉鎹紱
// 鐩殑锛氶伩鍏嶅閮ㄦ帴鍙ｆ尝鍔ㄥ共鎵扳€滄渶缁堟寮忓喅璁嚭鍙ｂ€濊繖鏉′富绾垮洖褰掓祴璇曘€?
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

    // 2026-04-09 CST: 杩欓噷鍏堥攣涓诲腑瑁佸喅 Tool 鐨勫彲鍙戠幇鎬э紝鍘熷洜鏄鏋?catalog 涓嶆毚闇插畠锛?    // 閭ｄ箞鈥滀富甯墠鏄敮涓€姝ｅ紡鍐宠鍑哄彛鈥濆氨鏃犳硶鎴愪负鐪熸浜у搧鑳藉姏锛涚洰鐨勶細纭繚 CLI / Skill / 鍚庣画 package 閮借兘绋冲畾鍙戠幇杩欐潯绾裤€?
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
                        {"notice_date":"2026-03-28","title":"2025骞村勾搴︽姤鍛?,"art_code":"AN202603281234567890","columns":[{"column_name":"瀹氭湡鎶ュ憡"}]},
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
            // 2026-04-17 CST: Added because this fixture-backed chair test should
            // stay on the governed local sample instead of drifting into live sync.
            // Purpose: keep the final-action contract assertion deterministic.
            "as_of_date": FIXTURE_AS_OF_DATE,
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

    // 2026-04-09 CST: 杩欓噷鍏堥攣涓夌嚎寮洪殧绂荤殑鏈€灏忔寮忓绾︼紝鍘熷洜鏄湰杞笉鏄户缁妸鎶曞浼氱粨鏋滅洿鎺ュ綋鏈€缁堝缓璁緭鍑猴紝
    // 鑰屾槸瑕佹眰涓诲腑鍗曠嫭璇诲叆閲忓寲绾垮拰鎶曞浼氱嚎鍚庡舰鎴愬敮涓€姝ｅ紡鍔ㄤ綔锛涚洰鐨勶細纭繚 chair_resolution 鎴愪负姝ｅ紡鏈€缁堝喅璁璞★紝
    // 鍚屾椂 committee 鍜?scorecard 浠嶇劧鍚勮嚜淇濈暀涓虹嫭绔嬭緭鍏ョ嚎銆?
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
            .expect("scorecard ref should exist")
            .starts_with("scorecard-"),
        "chair resolution should point to the formal scorecard object in the current contract"
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
    assert_eq!(
        output["data"]["committee_result"]["evidence_bundle"]["data_gaps"]
            .as_array()
            .expect("data gaps should exist")
            .len(),
        0,
        "gold ETF evidence should no longer carry formal data gaps once proxy history is complete: {output}"
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
    assert_ne!(
        output["data"]["scorecard"]["score_status"],
        "cross_section_invalid"
    );
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
    assert_ne!(
        output["data"]["scorecard"]["score_status"],
        "cross_section_invalid"
    );
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
    assert_ne!(
        output["data"]["scorecard"]["score_status"],
        "cross_section_invalid"
    );
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
    // 2026-04-11 CST: 杩欓噷鍏堣ˉ鈥滄棤璁粌妯″瀷鏃朵富甯笉寰楃洿鎺ョ粰杩涙敾鍔ㄤ綔鈥濈殑绾㈡祴锛屽師鍥犳槸鐢ㄦ埛瑕佹眰杩愯鏃朵篃瑕佸己鍒剁害鏉熼珮纭畾鎬у缓璁紱
    // 鐩殑锛氶攣浣?chair 鍦?`model_unavailable` 鍦烘櫙涓嬪繀椤绘妸鏈€缁堝姩浣滈檷绾т负闈炴墽琛屽瀷缁撹锛屽苟鏄惧紡杞负涓€ф毚闇层€?
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
                        {"notice_date":"2026-03-28","title":"2025骞村勾搴︽姤鍛?,"art_code":"AN202603281234567890","columns":[{"column_name":"瀹氭湡鎶ュ憡"}]},
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
            // 2026-04-17 CST: Added because the model-unavailable downgrade should
            // be evaluated against the local fixture window, not current live data.
            // Purpose: isolate chair downgrade behavior from date-driven sync drift.
            "as_of_date": FIXTURE_AS_OF_DATE,
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
    assert_eq!(output["data"]["chair_resolution"]["final_action"], "defer");
    assert_eq!(output["data"]["chair_resolution"]["final_stance"], "observe");
    assert!(
        output["data"]["chair_resolution"]["chair_reasoning"]
            .as_str()
            .expect("chair reasoning should exist")
            .contains("model_unavailable")
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
                        {"notice_date":"2026-03-28","title":"2025楠炴潙瀹抽幎銉ユ啞","art_code":"AN202603281234567890","columns":[{"column_name":"鐎规碍婀￠幎銉ユ啞"}]},
                        {"notice_date":"2026-03-28","title":"2025楠炴潙瀹抽崚鈺傞紟閸掑棝鍘ゆ０鍕攳閸忣剙鎲?,"art_code":"AN202603281234567891","columns":[{"column_name":"閸忣剙寰冮崗顒€鎲?}]}
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
            // 2026-04-17 CST: Added because the three-head fixture should remain
            // pinned to the governed local sample rather than sync to today's tape.
            // Purpose: keep partial multi-head constraint assertions stable.
            "as_of_date": FIXTURE_AS_OF_DATE,
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
                .contains("风险否决状态")),
        "execution constraints should keep the formal chair-side governance guard"
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
                        {"notice_date":"2026-03-28","title":"2025楠炴潙瀹抽幎銉ユ啞","art_code":"AN202603281234567890","columns":[{"column_name":"鐎规碍婀￠幎銉ユ啞"}]},
                        {"notice_date":"2026-03-28","title":"2025楠炴潙瀹抽崚鈺傞紟閸掑棝鍘ゆ０鍕攳閸忣剙鎲?,"art_code":"AN202603281234567891","columns":[{"column_name":"閸忣剙寰冮崗顒€鎲?}]}
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
            // 2026-04-17 CST: Added because path-event wording assertions depend on
            // the synthetic local sample, not on whatever live sync returns today.
            // Purpose: freeze the reasoning language against the intended fixture.
            "as_of_date": FIXTURE_AS_OF_DATE,
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
            .contains("冲突等级"),
        "chair reasoning should still expose the formal arbitration summary"
    );
    assert!(
        output["data"]["chair_resolution"]["execution_constraints"]
            .as_array()
            .expect("execution constraints should exist")
            .iter()
            .any(|item| item
                .as_str()
                .expect("constraint should be string")
                .contains("最终流程动作")),
        "execution constraints should keep the formal chair action guard"
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
                        {"notice_date":"2026-03-28","title":"2025楠炴潙瀹抽幎銉ユ啞","art_code":"AN202603281234567890","columns":[{"column_name":"鐎规碍婀￠幎銉ユ啞"}]}
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
    assert_eq!(
        output["data"]["chair_resolution"]["analysis_date"],
        "2026-04-12"
    );
    assert!(
        output["data"]["chair_resolution"]["chair_reasoning"]
            .as_str()
            .expect("chair reasoning should exist")
            .contains("量化线状态"),
        "chair reasoning should keep the current quant-status summary"
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
    let mut features = vec![
        // 2026-04-17 CST: Added because ETF runtime guard now requires a model to
        // carry at least one ETF-wide differentiating family marker in addition to
        // any subscope-specific proxy family.
        // Purpose: keep ETF chair fixtures aligned with the runtime binding contract.
        json!({
            "feature_name": "etf_context_status",
            "group_name": "X",
            "bins": [{"bin_label":"present","match_values":["__other__"],"points":2.0}],
        }),
        json!({
            "feature_name": "etf_asset_scope",
            "group_name": "X",
            "bins": [{"bin_label":"present","match_values":["__other__"],"points":2.0}],
        }),
    ];

    features.extend(feature_bins
        .into_iter()
        .map(|(feature_name, bins): (&str, Value)| {
            json!({
                "feature_name": feature_name,
                "group_name": if feature_name.ends_with("_status") { "X" } else { "T" },
                "bins": bins,
            })
        })
        .collect::<Vec<_>>());

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

// 2026-04-09 CST: 杩欓噷娌跨敤绋冲畾涓婅涓旀湯娈电獊鐮寸殑鏍锋湰锛屽師鍥犳槸涓诲腑瑁佸喅娴嬭瘯鐨勭洰鏍囦笉鏄噸鏂伴獙璇佽鎯呰緭鍏ラ摼锛?// 鑰屾槸楠岃瘉鈥滀笁绾胯緭鍏?-> 姝ｅ紡鏈€缁堝喅璁緭鍑衡€濈殑瀵硅薄杈圭晫锛涚洰鐨勶細鎶婂櫔澹板敖閲忓帇浣庯紝璁╁け璐ョ偣钀藉湪 chair_resolution 濂戠害鏈韩銆?
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
        // 2026-04-17 CST: Updated because the stricter chair-side technical chain
        // now evaluates resistance/support with wick-derived key levels.
        // Purpose: keep this fixture decisively beyond prior key levels instead of
        // hiding the move under oversized shadows that collapse to `range_wait`.
        let high = next_close.max(open)
            + if offset < day_count - 20 { 0.28 } else { 0.14 };
        let low = next_close.min(open)
            - if offset < day_count - 20 { 0.24 } else { 0.12 };
        let adj_close = next_close;
        rows.push(format!(
            "{},{open:.2},{high:.2},{low:.2},{next_close:.2},{adj_close:.2},{volume}",
            trade_date.format("%Y-%m-%d")
        ));
        close = next_close;
    }

    rows
}




