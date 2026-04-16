mod common;

use chrono::{Duration, NaiveDate};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;
use serde_json::{Value, json};

use crate::common::{
    create_test_runtime_db, run_cli_with_json, run_cli_with_json_runtime_and_envs,
};

// 2026-04-12 CST: Add a dedicated validation-root helper, because the real-data
// validation backfill tool must prove it can land artifacts outside the default
// runtime root before implementation begins.
// Purpose: keep the CLI contract focused on one explicit validation slice directory.
fn create_validation_runtime_root(prefix: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let root = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_real_data_validation_backfill")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&root).expect("validation runtime root should exist");
    root
}

// 2026-04-12 CST: Create one pool-style proxy-history fixture beside the runtime root,
// because ETF validation slices must prove they can auto-discover and import governed
// pool proxy history before latest chair replay is considered trustworthy.
// Purpose: keep the red test aligned with the real pool-training directory layout.
fn create_pool_proxy_history_fixture(
    runtime_db_path: &PathBuf,
    pool_dir_name: &str,
    file_name: &str,
    body: &str,
) -> PathBuf {
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db path should always have a parent");
    let pool_root = runtime_root.join(pool_dir_name);
    fs::create_dir_all(&pool_root).expect("pool proxy root should exist");
    let file_path = pool_root.join(file_name);
    fs::write(&file_path, body).expect("pool proxy history fixture should be written");
    file_path
}

// 2026-04-12 CST: Reuse a small route-based mock server, because this test needs
// one deterministic entry point for price history plus public disclosure endpoints.
// Purpose: make the red test independent from live-provider drift.
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

// 2026-04-12 CST: Build a deterministic Sina-style K-line body, because the
// real-data validation slice still needs enough history to satisfy the technical
// chain inside security_analysis_fullstack.
// Purpose: keep the test provider payload large enough for 200-day indicators without live data.
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

#[test]
fn tool_catalog_includes_security_real_data_validation_backfill() {
    let output = run_cli_with_json("");

    // 2026-04-12 CST: Lock tool discoverability first, because the governed
    // validation-slice refresh must become a first-class public stock tool.
    // Purpose: prevent implementation-only drift where the tool exists but cannot be found.
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_real_data_validation_backfill")
    );
}

#[test]
fn security_real_data_validation_backfill_persists_price_history_context_and_manifest() {
    let runtime_db_path = create_test_runtime_db("security_real_data_validation_backfill");
    let validation_root = create_validation_runtime_root("security_real_data_validation_backfill");
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
            "slice_id": "601916_sh_real_validation",
            "symbol": "601916.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "start_date": "2025-01-01",
            "end_date": "2025-08-08",
            "providers": ["sina"],
            "validation_runtime_root": validation_root.to_string_lossy(),
            "created_at": "2026-04-12T20:30:00+08:00"
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

    // 2026-04-12 CST: Lock the first governed real-data validation contract,
    // because the goal is one tool that refreshes price history plus disclosure
    // context into a dedicated slice instead of a shell recipe.
    // Purpose: require a stable runtime DB, a persisted fullstack context, and a manifest.
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["document_type"],
        "security_real_data_validation_backfill_result"
    );
    assert_eq!(output["data"]["slice_id"], "601916_sh_real_validation");
    assert_eq!(
        output["data"]["validation_runtime_root"],
        validation_root.to_string_lossy().to_string()
    );

    let runtime_db_path = PathBuf::from(
        output["data"]["runtime_db_path"]
            .as_str()
            .expect("runtime db path should exist"),
    );
    let fullstack_context_path = PathBuf::from(
        output["data"]["fullstack_context_path"]
            .as_str()
            .expect("fullstack context path should exist"),
    );
    let manifest_path = PathBuf::from(
        output["data"]["manifest_path"]
            .as_str()
            .expect("manifest path should exist"),
    );

    assert!(runtime_db_path.exists(), "runtime db should be written");
    assert!(
        fullstack_context_path.exists(),
        "fullstack context should be persisted"
    );
    assert!(manifest_path.exists(), "manifest should be persisted");
    assert!(
        runtime_db_path.starts_with(&validation_root),
        "dedicated runtime db should live under the validation root"
    );

    let manifest: Value = serde_json::from_slice(
        &fs::read(&manifest_path).expect("manifest file should be readable"),
    )
    .expect("manifest json should parse");
    assert_eq!(manifest["slice_id"], "601916_sh_real_validation");
    assert_eq!(manifest["primary_symbol"], "601916.SH");
    assert_eq!(
        manifest["price_sync_summaries"]
            .as_array()
            .expect("price sync summaries should be an array")
            .len(),
        3
    );

    let connection = Connection::open(&runtime_db_path).expect("runtime db should open");
    let primary_rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM stock_price_history WHERE symbol = '601916.SH'",
            [],
            |row| row.get(0),
        )
        .expect("primary symbol rows should exist");
    let market_rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM stock_price_history WHERE symbol = '510300.SH'",
            [],
            |row| row.get(0),
        )
        .expect("market symbol rows should exist");
    let sector_rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM stock_price_history WHERE symbol = '512800.SH'",
            [],
            |row| row.get(0),
        )
        .expect("sector symbol rows should exist");

    assert_eq!(primary_rows, 220);
    assert_eq!(market_rows, 220);
    assert_eq!(sector_rows, 220);
}

#[test]
fn security_real_data_validation_backfill_enriches_treasury_etf_peer_environment() {
    let runtime_db_path =
        create_test_runtime_db("security_real_data_validation_backfill_treasury_etf");
    let validation_root =
        create_validation_runtime_root("security_real_data_validation_backfill_treasury_etf");
    let external_proxy_db_path = runtime_db_path
        .parent()
        .expect("runtime db should live under a fixture root")
        .join("security_external_proxy.db");
    let sina_kline_body = build_sina_kline_body(220, 140.0);
    create_pool_proxy_history_fixture(
        &runtime_db_path,
        "pool_training_fix_treasury_validation",
        "treasury_pool_proxy_history.csv",
        "symbol,as_of_date,instrument_subscope,yield_curve_proxy_status,yield_curve_slope_delta_bp_5d,funding_liquidity_proxy_status,funding_liquidity_spread_delta_bp_5d,gold_spot_proxy_status,gold_spot_proxy_return_5d,usd_index_proxy_status,usd_index_proxy_return_5d,real_rate_proxy_status,real_rate_proxy_delta_bp_5d,fx_proxy_status,fx_return_5d,overseas_market_proxy_status,overseas_market_return_5d,market_session_gap_status,market_session_gap_days,etf_fund_flow_proxy_status,etf_fund_flow_5d,premium_discount_proxy_status,premium_discount_pct,benchmark_relative_strength_status,benchmark_relative_return_5d\n511010.SH,2025-08-08,treasury_etf,manual_bound,-3.0,manual_bound,7.0,,,,,,,,,,,,,,,,,,\n",
    );
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
            r#"{"data":{"list":[]}}"#,
            "application/json",
        ),
    ]);
    let request = json!({
        "tool": "security_real_data_validation_backfill",
        "args": {
            "slice_id": "511010_sh_treasury_real_validation",
            "symbol": "511010.SH",
            "market_profile": "a_share_core",
            "sector_profile": "treasury_etf",
            "start_date": "2025-01-01",
            "end_date": "2025-08-08",
            "providers": ["sina"],
            "validation_runtime_root": validation_root.to_string_lossy(),
            "created_at": "2026-04-12T21:00:00+08:00"
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
                "EXCEL_SKILL_EXTERNAL_PROXY_DB",
                external_proxy_db_path.to_string_lossy().to_string(),
            ),
        ],
    );

    // 2026-04-12 UTC+08: Add a treasury ETF slice regression here, because the
    // current governed validation slice for 511010.SH misses the treasury peer
    // environment symbol and therefore cannot replay the native ETF environment.
    // Purpose: require the slice builder to enrich treasury ETF slices with their
    // peer environment symbol instead of only syncing the primary ETF itself.
    assert_eq!(
        output["status"], "ok",
        "unexpected validation output: {output}"
    );
    assert_eq!(
        output["data"]["price_sync_summaries"]
            .as_array()
            .expect("sync summaries")
            .len(),
        3
    );
    assert!(
        output["data"]["price_sync_summaries"]
            .as_array()
            .expect("sync summaries should be an array")
            .iter()
            .any(|item| item["symbol"] == "511060.SH"),
        "treasury ETF slice should sync 511060.SH as the governed peer environment"
    );
}

#[test]
fn security_real_data_validation_backfill_preserves_equity_etf_native_profile_semantics() {
    let runtime_db_path =
        create_test_runtime_db("security_real_data_validation_backfill_equity_etf");
    let validation_root =
        create_validation_runtime_root("security_real_data_validation_backfill_equity_etf");
    let external_proxy_db_path = runtime_db_path
        .parent()
        .expect("runtime db should live under a fixture root")
        .join("security_external_proxy.db");
    let sina_kline_body = build_sina_kline_body(220, 1.40);
    create_pool_proxy_history_fixture(
        &runtime_db_path,
        "pool_training_fix_equity_validation",
        "equity_pool_proxy_history.csv",
        "symbol,as_of_date,instrument_subscope,yield_curve_proxy_status,yield_curve_slope_delta_bp_5d,funding_liquidity_proxy_status,funding_liquidity_spread_delta_bp_5d,gold_spot_proxy_status,gold_spot_proxy_return_5d,usd_index_proxy_status,usd_index_proxy_return_5d,real_rate_proxy_status,real_rate_proxy_delta_bp_5d,fx_proxy_status,fx_return_5d,overseas_market_proxy_status,overseas_market_return_5d,market_session_gap_status,market_session_gap_days,etf_fund_flow_proxy_status,etf_fund_flow_5d,premium_discount_proxy_status,premium_discount_pct,benchmark_relative_strength_status,benchmark_relative_return_5d\n512800.SH,2025-08-08,equity_etf,,,,,,,,,,,,,,,,,manual_bound,0.018,manual_bound,-0.0012,manual_bound,0.0041\n",
    );
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
            r#"{"data":{"list":[]}}"#,
            "application/json",
        ),
    ]);
    let request = json!({
        "tool": "security_real_data_validation_backfill",
        "args": {
            "slice_id": "512800_sh_equity_etf_real_validation",
            "symbol": "512800.SH",
            "market_profile": "a_share_core",
            "sector_profile": "equity_etf_peer",
            "start_date": "2025-01-01",
            "end_date": "2025-08-08",
            "providers": ["sina"],
            "validation_runtime_root": validation_root.to_string_lossy(),
            "created_at": "2026-04-12T21:10:00+08:00"
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
                "EXCEL_SKILL_EXTERNAL_PROXY_DB",
                external_proxy_db_path.to_string_lossy().to_string(),
            ),
        ],
    );

    // 2026-04-12 UTC+08: Add an ETF-native slice regression here, because the
    // current 512800.SH validation flow still falls back to an industry profile
    // instead of preserving equity ETF semantics all the way into the manifest.
    // Purpose: lock that governed validation slices can stay ETF-native without
    // forcing callers back to `a_share_bank`.
    assert_eq!(
        output["status"], "ok",
        "unexpected validation output: {output}"
    );
    let manifest_path = PathBuf::from(
        output["data"]["manifest_path"]
            .as_str()
            .expect("manifest path should exist"),
    );
    let manifest: Value = serde_json::from_slice(
        &fs::read(&manifest_path).expect("manifest file should be readable"),
    )
    .expect("manifest json should parse");
    assert_eq!(manifest["sector_profile"], "equity_etf_peer");
    assert_eq!(manifest["sector_symbol"], "512800.SH");
}

#[test]
fn security_real_data_validation_backfill_auto_imports_cross_border_pool_proxy_history() {
    let runtime_db_path =
        create_test_runtime_db("security_real_data_validation_backfill_cross_border_proxy_import");
    let validation_root = create_validation_runtime_root(
        "security_real_data_validation_backfill_cross_border_proxy_import",
    );
    let external_proxy_db_path = runtime_db_path
        .parent()
        .expect("runtime db should live under a fixture root")
        .join("security_external_proxy.db");
    let sina_kline_body = build_sina_kline_body(220, 7.30);
    create_pool_proxy_history_fixture(
        &runtime_db_path,
        "pool_training_fix_cross_border_validation",
        "cross_border_pool_proxy_history.csv",
        "symbol,as_of_date,instrument_subscope,yield_curve_proxy_status,yield_curve_slope_delta_bp_5d,funding_liquidity_proxy_status,funding_liquidity_spread_delta_bp_5d,gold_spot_proxy_status,gold_spot_proxy_return_5d,usd_index_proxy_status,usd_index_proxy_return_5d,real_rate_proxy_status,real_rate_proxy_delta_bp_5d,fx_proxy_status,fx_return_5d,overseas_market_proxy_status,overseas_market_return_5d,market_session_gap_status,market_session_gap_days,etf_fund_flow_proxy_status,etf_fund_flow_5d,premium_discount_proxy_status,premium_discount_pct,benchmark_relative_strength_status,benchmark_relative_return_5d\n513180.SH,2025-08-08,cross_border_etf,,,,,,,,,,,manual_bound,-0.0038,manual_bound,0.0120,manual_bound,0,,,,,,\n",
    );
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
            r#"{"data":{"list":[]}}"#,
            "application/json",
        ),
    ]);
    let request = json!({
        "tool": "security_real_data_validation_backfill",
        "args": {
            "slice_id": "513180_sh_cross_border_real_validation",
            "symbol": "513180.SH",
            "market_profile": "a_share_core",
            "sector_profile": "cross_border_etf",
            "start_date": "2025-01-01",
            "end_date": "2025-08-08",
            "providers": ["sina"],
            "validation_runtime_root": validation_root.to_string_lossy(),
            "created_at": "2026-04-12T21:20:00+08:00"
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
                "EXCEL_SKILL_EXTERNAL_PROXY_DB",
                external_proxy_db_path.to_string_lossy().to_string(),
            ),
        ],
    );

    // 2026-04-12 CST: Add an auto-import regression here, because the latest
    // pool-validation flow currently leaves cross-border ETF proxy rows inside
    // sidecar CSV files instead of governed external-proxy storage.
    // Purpose: require validation slices to hydrate the governed proxy store
    // before latest chair replay consumes the slice.
    assert_eq!(
        output["status"], "ok",
        "unexpected validation output: {output}"
    );
    assert_eq!(
        output["data"]["external_proxy_import_result_paths"]
            .as_array()
            .expect("external proxy import results should be an array")
            .len(),
        1
    );
    let connection =
        Connection::open(&external_proxy_db_path).expect("external proxy db should open");
    let imported_rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM security_external_proxy_history WHERE symbol = '513180.SH' AND as_of_date = '2025-08-08'",
            [],
            |row| row.get(0),
        )
        .expect("external proxy rows should be queryable");
    assert_eq!(imported_rows, 1);
}

#[test]
fn security_real_data_validation_backfill_rejects_etf_slice_without_pool_proxy_history() {
    let runtime_db_path =
        create_test_runtime_db("security_real_data_validation_backfill_missing_etf_proxy");
    let validation_root =
        create_validation_runtime_root("security_real_data_validation_backfill_missing_etf_proxy");
    let external_proxy_db_path = runtime_db_path
        .parent()
        .expect("runtime db should live under a fixture root")
        .join("security_external_proxy.db");
    let sina_kline_body = build_sina_kline_body(220, 7.30);
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
            r#"{"data":{"list":[]}}"#,
            "application/json",
        ),
    ]);
    let request = json!({
        "tool": "security_real_data_validation_backfill",
        "args": {
            "slice_id": "515790_sh_equity_etf_missing_proxy",
            "symbol": "515790.SH",
            "market_profile": "a_share_core",
            "sector_profile": "equity_etf_peer",
            "start_date": "2025-01-01",
            "end_date": "2025-08-08",
            "providers": ["sina"],
            "validation_runtime_root": validation_root.to_string_lossy(),
            "created_at": "2026-04-12T21:30:00+08:00"
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
                "EXCEL_SKILL_EXTERNAL_PROXY_DB",
                external_proxy_db_path.to_string_lossy().to_string(),
            ),
        ],
    );

    // 2026-04-12 CST: Add a missing-history failure regression here, because the
    // current validation flow can produce a misleadingly successful ETF slice even
    // when no governed proxy rows exist for latest scoring.
    // Purpose: fail fast instead of letting later latest-chair reruns drift into
    // placeholder_unbound and fake summary success.
    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error message should exist")
            .contains("missing required etf proxy history"),
        "unexpected error payload: {output}"
    );
}
