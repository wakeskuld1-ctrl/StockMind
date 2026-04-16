mod common;

use chrono::{Duration, NaiveDate};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{create_test_runtime_db, run_cli_with_json_runtime_and_envs};

// 2026-04-12 CST: Create a validation root helper, because the governed
// validation-slice test should prove that slice-local stock history and stock
// information history are persisted together.
// Purpose: keep the validation-history test isolated and reproducible.
fn create_validation_runtime_root(prefix: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let root = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_stock_history_governance")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&root).expect("validation runtime root should exist");
    root
}

// 2026-04-12 CST: Reuse a deterministic route-based HTTP server, because the
// validation tool still fetches live-compatible payloads and this test should not
// depend on public network drift.
// Purpose: keep history-persistence regression stable across reruns.
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
        // 2026-04-12 CST: Allow more than one request per registered route,
        // because one validation slice now consumes repeated market/sector
        // price syncs before it fetches governed fundamental and disclosure data.
        // Purpose: keep the test server alive long enough for the full validation chain.
        for _ in 0..12 {
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

// 2026-04-12 CST: Build a stable Sina-style payload, because the validation
// slice still needs enough trading history to satisfy technical indicators while
// we test governed information-history persistence.
// Purpose: keep the price leg of the regression deterministic.
fn build_sina_kline_body(day_count: usize, start_close: f64) -> String {
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("seed date should be valid");
    let mut close = start_close;
    let mut rows = Vec::with_capacity(day_count);

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let next_close = if offset < day_count - 20 {
            close + 0.42
        } else {
            close + 0.88
        };
        let open = close;
        let high = next_close.max(open) + 0.65;
        let low = next_close.min(open) - 0.51;
        let volume = 1_000_000 + offset as i64 * 9_000;
        rows.push(format!(
            "{{\"day\":\"{}\",\"open\":\"{open:.2}\",\"high\":\"{high:.2}\",\"low\":\"{low:.2}\",\"close\":\"{next_close:.2}\",\"volume\":\"{volume}\"}}",
            trade_date.format("%Y-%m-%d")
        ));
        close = next_close;
    }

    format!("[{}]", rows.join(","))
}

// 2026-04-12 CST: Import one generated CSV into stock history through the formal
// tool path, because the governed fullstack-history regression should still use
// the same stock-history import contract as the rest of the mainline.
// Purpose: avoid bypassing the formal CLI chain in the test.
fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "provider": "fixture_csv"
        }
    });
    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path.to_path_buf(),
        &[],
    );
    assert_eq!(output["status"], "ok");
}

// 2026-04-12 CST: Generate a compact but valid CSV history fixture, because the
// governed fullstack-history regression only needs enough structure to satisfy
// the technical path before checking information-history precedence.
// Purpose: keep the new regression self-contained.
fn create_stock_history_csv(prefix: &str, rows: &[String]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_stock_history_governance")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("csv fixture dir should exist");
    let csv_path = fixture_dir.join("history.csv");
    fs::write(&csv_path, rows.join("\n")).expect("csv fixture should be written");
    csv_path
}

// 2026-04-12 CST: Build one gently rising fixture, because the technical chain
// only needs valid history and not a specific research stance for this regression.
// Purpose: minimize non-history noise in the new tests.
fn build_history_rows(day_count: usize, seed_close: f64) -> Vec<String> {
    let mut rows = vec!["date,open,high,low,close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("seed date should be valid");
    let mut close = seed_close;
    for offset in 0..day_count {
        let date = start_date + Duration::days(offset as i64);
        let next_close = close + 0.18;
        let open = close;
        let high = next_close + 0.24;
        let low = open - 0.21;
        let volume = 600_000 + offset as i64 * 6_000;
        rows.push(format!(
            "{},{open:.2},{high:.2},{low:.2},{next_close:.2},{}",
            date.format("%Y-%m-%d"),
            volume
        ));
        close = next_close;
    }
    rows
}

#[test]
fn security_analysis_fullstack_prefers_governed_stock_history_before_live_fetch() {
    let runtime_db_path = create_test_runtime_db("security_stock_history_governed_fullstack");
    let stock_csv = create_stock_history_csv(
        "security_stock_history_governed_fullstack_stock",
        &build_history_rows(220, 3.21),
    );
    let market_csv = create_stock_history_csv(
        "security_stock_history_governed_fullstack_market",
        &build_history_rows(220, 5.88),
    );
    let sector_csv = create_stock_history_csv(
        "security_stock_history_governed_fullstack_sector",
        &build_history_rows(220, 1.77),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let fundamental_backfill_request = json!({
        "tool": "security_fundamental_history_backfill",
        "args": {
            "batch_id": "fundamental-governed-20260412",
            "created_at": "2026-04-12T22:25:00+08:00",
            "records": [
                {
                    "symbol": "601916.SH",
                    "report_period": "2025-12-31",
                    "notice_date": "2026-03-28",
                    "source": "eastmoney_financials",
                    "report_metrics": {
                        "revenue": 308227000000.0,
                        "revenue_yoy_pct": 8.37,
                        "net_profit": 11117000000.0,
                        "net_profit_yoy_pct": 9.31,
                        "roe_pct": 14.8
                    }
                }
            ]
        }
    });
    let disclosure_backfill_request = json!({
        "tool": "security_disclosure_history_backfill",
        "args": {
            "batch_id": "disclosure-governed-20260412",
            "created_at": "2026-04-12T22:26:00+08:00",
            "records": [
                {
                    "symbol": "601916.SH",
                    "published_at": "2026-03-28",
                    "title": "2025 Annual Report",
                    "article_code": "AN202603281234567890",
                    "category": "Periodic Report",
                    "source": "eastmoney_announcements"
                },
                {
                    "symbol": "601916.SH",
                    "published_at": "2026-03-28",
                    "title": "2025 Profit Distribution Plan",
                    "article_code": "AN202603281234567891",
                    "category": "Company Notice",
                    "source": "eastmoney_announcements"
                }
            ]
        }
    });
    let _ = run_cli_with_json_runtime_and_envs(
        &fundamental_backfill_request.to_string(),
        &runtime_db_path,
        &[],
    );
    let _ = run_cli_with_json_runtime_and_envs(
        &disclosure_backfill_request.to_string(),
        &runtime_db_path,
        &[],
    );

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 500 Internal Server Error",
            r#"{"error":"financial upstream failed"}"#,
            "application/json",
        ),
        (
            "/announcements",
            "HTTP/1.1 500 Internal Server Error",
            r#"{"error":"announcement upstream failed"}"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_analysis_fullstack",
        "args": {
            "symbol": "601916.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "as_of_date": "2026-04-10",
            "disclosure_limit": 2
        }
    });
    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &[
            ("EXCEL_SKILL_EASTMONEY_DAILY_LIMIT", "0".to_string()),
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

    // 2026-04-12 CST: Lock governed history precedence before implementation,
    // because stock validation and replay should stop depending on live fetches
    // once governed historical fundamentals and disclosures exist.
    // Purpose: force fullstack to return available contexts from history even if live fetch fails.
    assert_eq!(output["status"], "ok");
    assert_eq!(output["data"]["fundamental_context"]["status"], "available");
    assert_eq!(
        output["data"]["fundamental_context"]["source"],
        "governed_fundamental_history"
    );
    assert_eq!(output["data"]["disclosure_context"]["status"], "available");
    assert_eq!(
        output["data"]["disclosure_context"]["source"],
        "governed_disclosure_history"
    );
}

#[test]
fn security_real_data_validation_backfill_persists_slice_local_stock_information_history() {
    let runtime_db_path = create_test_runtime_db("security_stock_history_validation_slice");
    let validation_root = create_validation_runtime_root("security_stock_history_validation_slice");
    let sina_kline_body = build_sina_kline_body(220, 88.1);
    let server = spawn_http_route_server(vec![
        (
            "/sina-kline",
            "HTTP/1.1 200 OK",
            &sina_kline_body,
            "application/json",
        ),
        (
            "/financials",
            "HTTP/1.1 200 OK",
            r#"[{"REPORT_DATE":"2025-12-31","NOTICE_DATE":"2026-03-28","TOTAL_OPERATE_INCOME":308227000000.0,"YSTZ":8.37,"PARENT_NETPROFIT":11117000000.0,"SJLTZ":9.31,"ROEJQ":14.8}]"#,
            "application/json",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{"data":{"list":[{"notice_date":"2026-03-28","title":"2025 Annual Report","art_code":"AN202603281234567890","columns":[{"column_name":"Periodic Report"}]},{"notice_date":"2026-03-28","title":"2025 Profit Distribution Plan","art_code":"AN202603281234567891","columns":[{"column_name":"Company Notice"}]}]}}"#,
            "application/json",
        ),
    ]);
    let request = json!({
        "tool": "security_real_data_validation_backfill",
        "args": {
            "slice_id": "601916_sh_real_validation_history",
            "symbol": "601916.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "start_date": "2025-01-01",
            "end_date": "2025-08-08",
            "providers": ["sina"],
            "validation_runtime_root": validation_root.to_string_lossy(),
            "created_at": "2026-04-12T22:40:00+08:00"
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

    // 2026-04-12 CST: Lock slice-local stock information history persistence,
    // because validation slices should not stop at price history once governed
    // stock fundamental/disclosure history exists.
    // Purpose: require validation refresh to persist replayable non-price evidence.
    assert_eq!(output["status"], "ok");
    let fundamental_history_db_path = PathBuf::from(
        output["data"]["fundamental_history_db_path"]
            .as_str()
            .expect("fundamental history db path should exist"),
    );
    let disclosure_history_db_path = PathBuf::from(
        output["data"]["disclosure_history_db_path"]
            .as_str()
            .expect("disclosure history db path should exist"),
    );
    assert!(fundamental_history_db_path.exists());
    assert!(disclosure_history_db_path.exists());

    let fundamental_connection =
        Connection::open(&fundamental_history_db_path).expect("fundamental db should open");
    let disclosure_connection =
        Connection::open(&disclosure_history_db_path).expect("disclosure db should open");
    let fundamental_rows: i64 = fundamental_connection
        .query_row(
            "SELECT COUNT(*) FROM security_fundamental_history WHERE symbol = '601916.SH'",
            [],
            |row| row.get(0),
        )
        .expect("fundamental rows should exist");
    let disclosure_rows: i64 = disclosure_connection
        .query_row(
            "SELECT COUNT(*) FROM security_disclosure_history WHERE symbol = '601916.SH'",
            [],
            |row| row.get(0),
        )
        .expect("disclosure rows should exist");

    assert_eq!(fundamental_rows, 1);
    assert_eq!(disclosure_rows, 2);

    let manifest_path = PathBuf::from(
        output["data"]["manifest_path"]
            .as_str()
            .expect("manifest path should exist"),
    );
    let manifest: Value = serde_json::from_slice(
        &fs::read(&manifest_path).expect("manifest file should be readable"),
    )
    .expect("manifest json should parse");
    assert_eq!(
        manifest["fundamental_history_db_path"],
        fundamental_history_db_path.to_string_lossy().to_string()
    );
    assert_eq!(
        manifest["disclosure_history_db_path"],
        disclosure_history_db_path.to_string_lossy().to_string()
    );
}
