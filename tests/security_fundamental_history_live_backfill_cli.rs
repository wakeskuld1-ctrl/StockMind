mod common;

use serde_json::json;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{
    create_test_runtime_db, run_cli_with_json, run_cli_with_json_runtime_and_envs,
};

// 2026-04-12 CST: Add an isolated history root helper for live financial-history backfill,
// because the new tool should prove it can persist governed multi-period snapshots outside
// the default runtime root before implementation lands.
// Purpose: keep CLI persistence tests reproducible and side-effect free.
fn create_history_runtime_root(prefix: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let root = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_fundamental_history_live_backfill")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&root).expect("history runtime root should exist");
    root
}

// 2026-04-12 CST: Keep one deterministic mock server for multi-period financial payloads,
// because the live backfill tool should be verified without depending on network drift.
// Purpose: let RED/GREEN tests lock provider parsing through a stable local route.
fn spawn_http_route_server(route_path: &str, body: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test http server should bind");
    let address = format!(
        "http://{}",
        listener
            .local_addr()
            .expect("test http server should have local addr")
    );
    let route_path = route_path.to_string();
    let response_body = body.to_string();

    thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let mut buffer = [0_u8; 4096];
        let _ = stream.read(&mut buffer);
        let request_text = String::from_utf8_lossy(&buffer);
        let request_path = request_text
            .lines()
            .next()
            .unwrap_or_default()
            .split_whitespace()
            .nth(1)
            .unwrap_or("/")
            .split('?')
            .next()
            .unwrap_or("/");
        let (status_line, body) = if request_path == route_path {
            ("HTTP/1.1 200 OK", response_body)
        } else {
            (
                "HTTP/1.1 404 Not Found",
                "{\"error\":\"not found\"}".to_string(),
            )
        };
        let response = format!(
            "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    });

    address
}

#[test]
fn tool_catalog_includes_security_fundamental_history_live_backfill() {
    let output = run_cli_with_json("");

    // 2026-04-12 CST: Lock discoverability for the live governed financial-history tool,
    // because Historical Data Phase 1 should expose a public path for multi-period imports.
    // Purpose: ensure CLI and Skills can find the new live backfill capability.
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_fundamental_history_live_backfill")
    );
}

#[test]
fn security_fundamental_history_live_backfill_fetches_multiple_report_periods() {
    let runtime_db_path = create_test_runtime_db("security_fundamental_history_live_backfill");
    let history_root = create_history_runtime_root("security_fundamental_history_live_backfill");
    let server = spawn_http_route_server(
        "/financials",
        r#"[{"REPORT_DATE":"2025-12-31","NOTICE_DATE":"2026-03-28","TOTAL_OPERATE_INCOME":308227000000.0,"YSTZ":8.37,"PARENT_NETPROFIT":11117000000.0,"SJLTZ":9.31,"ROEJQ":14.8},{"REPORT_DATE":"2024-12-31","NOTICE_DATE":"2025-03-29","TOTAL_OPERATE_INCOME":284100000000.0,"YSTZ":6.12,"PARENT_NETPROFIT":10122000000.0,"SJLTZ":7.04,"ROEJQ":13.9}]"#,
    );
    let request = json!({
        "tool": "security_fundamental_history_live_backfill",
        "args": {
            "symbol": "601916.SH",
            "batch_id": "fundamental-live-2026-04-12-a",
            "created_at": "2026-04-12T23:10:00+08:00",
            "history_runtime_root": history_root.to_string_lossy()
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &[(
            "EXCEL_SKILL_EASTMONEY_FINANCIAL_URL_BASE",
            format!("{server}/financials"),
        )],
    );

    // 2026-04-12 CST: Lock the multi-period live financial backfill contract,
    // because stock governed history should stop at latest-snapshot-only imports.
    // Purpose: require the new tool to persist all provider periods into governed storage.
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["document_type"],
        "security_fundamental_history_live_backfill_result"
    );
    assert_eq!(output["data"]["symbol"], "601916.SH");
    assert_eq!(output["data"]["fetched_record_count"], 2);
    assert_eq!(output["data"]["imported_record_count"], 2);
    assert_eq!(output["data"]["covered_symbol_count"], 1);
}
