mod common;

use rusqlite::Connection;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{
    create_test_runtime_db, run_cli_with_json, run_cli_with_json_runtime_and_envs,
};

// 2026-04-14 CST: Added because the new batch backfill tool needs one isolated governed-history
// root in tests, and reusing shared runtime folders would blur whether this tool really controls
// its own disclosure/fundamental landing paths.
// Purpose: keep the batch data-thickening CLI test reproducible and side-effect free.
fn create_history_runtime_root(prefix: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let root = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("stock_training_data_backfill")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&root).expect("history runtime root should exist");
    root
}

// 2026-04-14 CST: Added because the batch backfill tool fans out into price, financial, and
// announcement providers, and the CLI test must lock all three sources behind one deterministic
// local server rather than drifting with public network state.
// Purpose: simulate the multi-route provider surface required by stock training-data thickening.
fn spawn_query_aware_server(
    route_map: HashMap<String, String>,
    expected_requests: usize,
) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test http server should bind");
    let address = format!(
        "http://{}",
        listener
            .local_addr()
            .expect("test http server should have local addr")
    );

    thread::spawn(move || {
        for _ in 0..expected_requests {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            let mut buffer = [0_u8; 4096];
            let _ = stream.read(&mut buffer);
            let request_text = String::from_utf8_lossy(&buffer);
            let request_uri = request_text
                .lines()
                .next()
                .unwrap_or_default()
                .split_whitespace()
                .nth(1)
                .unwrap_or("/")
                .to_string();
            let body = route_map
                .get(&request_uri)
                .cloned()
                .unwrap_or_else(|| "{\"error\":\"not found\"}".to_string());
            let status_line = if route_map.contains_key(&request_uri) {
                "HTTP/1.1 200 OK"
            } else {
                "HTTP/1.1 404 Not Found"
            };
            let response = format!(
                "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });

    address
}

// 2026-04-14 CST: Added because the batch backfill tool must prove it is publicly discoverable
// before we rely on it for retraining preparation.
// Purpose: prevent implementation-only drift where the batch tool exists but cannot be found by CLI or Skills.
#[test]
fn tool_catalog_includes_stock_training_data_backfill() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "stock_training_data_backfill")
    );
}

// 2026-04-14 CST: Added because plan A+ needs one formal CLI contract that thickens stock
// training data across price, financial history, and disclosure history in one run.
// Purpose: lock the minimal happy path for stock retraining preparation without pulling ETF into the batch.
#[test]
fn stock_training_data_backfill_syncs_price_and_backfills_information_history_for_equities() {
    let runtime_db_path = create_test_runtime_db("stock_training_data_backfill");
    let history_root = create_history_runtime_root("stock_training_data_backfill");
    let server = spawn_query_aware_server(
        HashMap::from([
            (
                "/sina-kline".to_string(),
                r#"[{"day":"2026-03-27","open":"3.10","high":"3.18","low":"3.05","close":"3.16","volume":"1200000"},{"day":"2026-03-28","open":"3.16","high":"3.22","low":"3.11","close":"3.20","volume":"1360000"}]"#.to_string(),
            ),
            (
                // 2026-04-14 CST: Updated because live governed fundamental backfill appends
                // Eastmoney-style query parameters to the configured base URL before issuing
                // the request.
                // Purpose: keep the CLI fixture aligned with the real production URL builder so
                // the batch backfill test fails only on business regressions, not on route drift.
                "/financials?type=1&code=SH601916".to_string(),
                r#"[{"REPORT_DATE":"2025-12-31","NOTICE_DATE":"2026-03-28","TOTAL_OPERATE_INCOME":308227000000.0,"YSTZ":8.37,"PARENT_NETPROFIT":11117000000.0,"SJLTZ":9.31,"ROEJQ":14.8},{"REPORT_DATE":"2024-12-31","NOTICE_DATE":"2025-03-29","TOTAL_OPERATE_INCOME":284100000000.0,"YSTZ":6.12,"PARENT_NETPROFIT":10122000000.0,"SJLTZ":7.04,"ROEJQ":13.9}]"#.to_string(),
            ),
            (
                "/announcements?sr=-1&page_size=2&page_index=1&ann_type=A&stock_list=601916".to_string(),
                r#"{"data":{"list":[{"notice_date":"2026-03-28","title":"2025 Annual Report","art_code":"AN202603281234567890","columns":[{"column_name":"Periodic Report"}]},{"notice_date":"2026-03-28","title":"2025 Profit Distribution Plan","art_code":"AN202603281234567891","columns":[{"column_name":"Company Notice"}]}]}}"#.to_string(),
            ),
            (
                "/announcements?sr=-1&page_size=2&page_index=2&ann_type=A&stock_list=601916".to_string(),
                r#"{"data":{"list":[{"notice_date":"2026-02-14","title":"Shareholder Meeting Resolution","art_code":"AN202602141234567892","columns":[{"column_name":"Company Notice"}]}]}}"#.to_string(),
            ),
        ]),
        20,
    );
    let request = json!({
        "tool": "stock_training_data_backfill",
        "args": {
            "equity_symbols": ["601916.SH"],
            "market_symbols": ["510300.SH"],
            "sector_symbols": ["512800.SH"],
            "start_date": "2026-03-27",
            "end_date": "2026-03-28",
            "adjustment": "qfq",
            "providers": ["sina"],
            "batch_id": "stock-training-data-backfill-2026-04-14-a",
            "created_at": "2026-04-14T21:30:00+08:00",
            "history_runtime_root": history_root.to_string_lossy(),
            "disclosure_page_size": 2,
            "disclosure_max_pages": 2,
            "backfill_fundamentals": true,
            "backfill_disclosures": true
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &[
            ("EXCEL_SKILL_SINA_KLINE_URL", format!("{server}/sina-kline")),
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

    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["document_type"],
        "stock_training_data_backfill_result"
    );
    assert_eq!(
        output["data"]["price_sync_results"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        output["data"]["fundamental_backfill_results"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        output["data"]["disclosure_backfill_results"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        output["data"]["known_gaps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "corporate_action_history_not_implemented")
    );

    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should always have a parent directory");
    let price_connection = Connection::open(runtime_root.join("stock_history.db"))
        .expect("stock history db should exist");
    let imported_price_rows: i64 = price_connection
        .query_row("SELECT COUNT(*) FROM stock_price_history", [], |row| {
            row.get(0)
        })
        .expect("price row count query should succeed");
    let imported_symbols: i64 = price_connection
        .query_row(
            "SELECT COUNT(DISTINCT symbol) FROM stock_price_history",
            [],
            |row| row.get(0),
        )
        .expect("price symbol count query should succeed");

    assert_eq!(imported_price_rows, 6);
    assert_eq!(imported_symbols, 3);

    let fundamental_connection =
        Connection::open(history_root.join("security_fundamental_history.db"))
            .expect("fundamental history db should exist");
    let imported_fundamental_rows: i64 = fundamental_connection
        .query_row(
            "SELECT COUNT(*) FROM security_fundamental_history WHERE symbol = '601916.SH'",
            [],
            |row| row.get(0),
        )
        .expect("fundamental row count query should succeed");
    assert_eq!(imported_fundamental_rows, 2);

    let disclosure_connection =
        Connection::open(history_root.join("security_disclosure_history.db"))
            .expect("disclosure history db should exist");
    let imported_disclosure_rows: i64 = disclosure_connection
        .query_row(
            "SELECT COUNT(*) FROM security_disclosure_history WHERE symbol = '601916.SH'",
            [],
            |row| row.get(0),
        )
        .expect("disclosure row count query should succeed");
    assert_eq!(imported_disclosure_rows, 3);
}

// 2026-04-14 CST: Added because the first real stock-first backfill probe showed that Eastmoney
// financial TLS failures can block the whole batch even though Sina still has usable free data.
// Purpose: reproduce the current real-world failure mode and lock the fallback path before fixing it.
#[test]
fn stock_training_data_backfill_falls_back_to_sina_financial_and_announcement_history_when_eastmoney_fails()
 {
    let runtime_db_path = create_test_runtime_db("stock_training_data_backfill_fallback");
    let history_root = create_history_runtime_root("stock_training_data_backfill_fallback");
    let server = spawn_query_aware_server(
        HashMap::from([
            (
                "/sina-kline".to_string(),
                r#"[{"day":"2026-03-27","open":"3.10","high":"3.18","low":"3.05","close":"3.16","volume":"1200000"},{"day":"2026-03-28","open":"3.16","high":"3.22","low":"3.11","close":"3.20","volume":"1360000"}]"#.to_string(),
            ),
            (
                "/financials?type=1&code=SH601916".to_string(),
                r#"{"error":"tls failed"}"#.to_string(),
            ),
            (
                "/sina-financial?symbol=601916.SH&stockid=601916".to_string(),
                r#"
                <table>
                  <tr>
                    <th>报告日期</th>
                    <th>2025-12-31</th>
                    <th>2024-12-31</th>
                  </tr>
                  <tr>
                    <td>营业总收入(元)</td>
                    <td>308227000000</td>
                    <td>284100000000</td>
                  </tr>
                  <tr>
                    <td>营业总收入增长率(%)</td>
                    <td>8.37</td>
                    <td>6.12</td>
                  </tr>
                  <tr>
                    <td>归母净利润(元)</td>
                    <td>11117000000</td>
                    <td>10122000000</td>
                  </tr>
                  <tr>
                    <td>归母净利润增长率(%)</td>
                    <td>9.31</td>
                    <td>7.04</td>
                  </tr>
                  <tr>
                    <td>加权净资产收益率(%)</td>
                    <td>14.8</td>
                    <td>13.9</td>
                  </tr>
                </table>
                "#.to_string(),
            ),
            (
                "/announcements?sr=-1&page_size=2&page_index=1&ann_type=A&stock_list=601916".to_string(),
                r#"{"error":"eastmoney unavailable"}"#.to_string(),
            ),
            (
                "/sina-announcements?symbol=601916.SH&stockid=601916".to_string(),
                r#"
                2026-03-28&nbsp;<a href="/notice?id=AN202603281234567890">2025 Annual Report</a>
                2026-03-28&nbsp;<a href="/notice?id=AN202603281234567891">2025 Profit Distribution Plan</a>
                2026-02-14&nbsp;<a href="/notice?id=AN202602141234567892">Shareholder Meeting Resolution</a>
                "#.to_string(),
            ),
        ]),
        20,
    );
    let request = json!({
        "tool": "stock_training_data_backfill",
        "args": {
            "equity_symbols": ["601916.SH"],
            "market_symbols": ["510300.SH"],
            "sector_symbols": ["512800.SH"],
            "start_date": "2026-03-27",
            "end_date": "2026-03-28",
            "adjustment": "qfq",
            "providers": ["sina"],
            "batch_id": "stock-training-data-backfill-2026-04-14-fallback",
            "created_at": "2026-04-14T21:45:00+08:00",
            "history_runtime_root": history_root.to_string_lossy(),
            "disclosure_page_size": 2,
            "disclosure_max_pages": 2,
            "backfill_fundamentals": true,
            "backfill_disclosures": true
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &[
            ("EXCEL_SKILL_SINA_KLINE_URL", format!("{server}/sina-kline")),
            (
                "EXCEL_SKILL_EASTMONEY_FINANCIAL_URL_BASE",
                format!("{server}/financials"),
            ),
            (
                "EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE",
                format!("{server}/announcements"),
            ),
            (
                "EXCEL_SKILL_SINA_FINANCIAL_URL_BASE",
                format!("{server}/sina-financial"),
            ),
            (
                "EXCEL_SKILL_SINA_ANNOUNCEMENT_URL_BASE",
                format!("{server}/sina-announcements"),
            ),
        ],
    );

    assert_eq!(output["status"], "ok", "fallback output: {}", output);
    assert_eq!(
        output["data"]["fundamental_backfill_results"][0]["fetched_record_count"],
        2
    );
    assert_eq!(
        output["data"]["disclosure_backfill_results"][0]["fetched_record_count"],
        3
    );

    let fundamental_connection =
        Connection::open(history_root.join("security_fundamental_history.db"))
            .expect("fundamental history db should exist");
    let imported_fundamental_rows: i64 = fundamental_connection
        .query_row(
            "SELECT COUNT(*) FROM security_fundamental_history WHERE symbol = '601916.SH'",
            [],
            |row| row.get(0),
        )
        .expect("fundamental row count query should succeed");
    assert_eq!(imported_fundamental_rows, 2);

    let disclosure_connection =
        Connection::open(history_root.join("security_disclosure_history.db"))
            .expect("disclosure history db should exist");
    let imported_disclosure_rows: i64 = disclosure_connection
        .query_row(
            "SELECT COUNT(*) FROM security_disclosure_history WHERE symbol = '601916.SH'",
            [],
            |row| row.get(0),
        )
        .expect("disclosure row count query should succeed");
    assert_eq!(imported_disclosure_rows, 3);
}
