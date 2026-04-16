mod common;

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

// 2026-04-12 CST: Create an isolated disclosure-history root for live backfill tests,
// because the new tool should prove it can land governed multi-page announcements
// outside the shared runtime before implementation lands.
// Purpose: keep persistence tests deterministic and side-effect free.
fn create_history_runtime_root(prefix: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let root = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_disclosure_history_live_backfill")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&root).expect("history runtime root should exist");
    root
}

// 2026-04-12 CST: Keep one query-aware HTTP fixture for paged announcement payloads,
// because the live disclosure tool must prove it can aggregate multiple pages before
// persisting governed history.
// Purpose: lock page-wise provider behavior without relying on public network responses.
fn spawn_query_aware_server(route_map: HashMap<String, String>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test http server should bind");
    let address = format!(
        "http://{}",
        listener
            .local_addr()
            .expect("test http server should have local addr")
    );

    thread::spawn(move || {
        for _ in 0..3 {
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

#[test]
fn tool_catalog_includes_security_disclosure_history_live_backfill() {
    let output = run_cli_with_json("");

    // 2026-04-12 CST: Lock discoverability for the live governed disclosure-history tool,
    // because Historical Data Phase 1 should expose a public path for paged announcement imports.
    // Purpose: ensure CLI and Skills can find the new live disclosure backfill capability.
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_disclosure_history_live_backfill")
    );
}

#[test]
fn security_disclosure_history_live_backfill_fetches_multiple_pages() {
    let runtime_db_path = create_test_runtime_db("security_disclosure_history_live_backfill");
    let history_root = create_history_runtime_root("security_disclosure_history_live_backfill");
    let server = spawn_query_aware_server(HashMap::from([
        (
            "/announcements?sr=-1&page_size=2&page_index=1&ann_type=A&stock_list=601916".to_string(),
            r#"{"data":{"list":[{"notice_date":"2026-03-28","title":"2025 Annual Report","art_code":"AN202603281234567890","columns":[{"column_name":"Periodic Report"}]},{"notice_date":"2026-03-28","title":"2025 Profit Distribution Plan","art_code":"AN202603281234567891","columns":[{"column_name":"Company Notice"}]}]}}"#.to_string(),
        ),
        (
            "/announcements?sr=-1&page_size=2&page_index=2&ann_type=A&stock_list=601916".to_string(),
            r#"{"data":{"list":[{"notice_date":"2026-02-14","title":"Shareholder Meeting Resolution","art_code":"AN202602141234567892","columns":[{"column_name":"Company Notice"}]}]}}"#.to_string(),
        ),
    ]));
    let request = json!({
        "tool": "security_disclosure_history_live_backfill",
        "args": {
            "symbol": "601916.SH",
            "batch_id": "disclosure-live-2026-04-12-a",
            "created_at": "2026-04-12T23:15:00+08:00",
            "history_runtime_root": history_root.to_string_lossy(),
            "page_size": 2,
            "max_pages": 2
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &[(
            "EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE",
            format!("{server}/announcements"),
        )],
    );

    // 2026-04-12 CST: Lock the multi-page live disclosure backfill contract,
    // because stock governed event history should aggregate paged announcements
    // instead of freezing only the latest single response page.
    // Purpose: require the new tool to persist all fetched announcements into governed storage.
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["document_type"],
        "security_disclosure_history_live_backfill_result"
    );
    assert_eq!(output["data"]["symbol"], "601916.SH");
    assert_eq!(output["data"]["fetched_record_count"], 3);
    assert_eq!(output["data"]["imported_record_count"], 3);
    assert_eq!(output["data"]["covered_symbol_count"], 1);
}
