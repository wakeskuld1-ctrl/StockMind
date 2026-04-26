mod common;

use chrono::{Duration, NaiveDate};
use rusqlite::Connection;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
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

#[test]
fn tool_catalog_includes_security_portfolio_execution_replay_commit_writer() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_portfolio_execution_replay_commit_writer"),
        "tool catalog should include P19D writer"
    );
}

#[test]
fn security_portfolio_execution_replay_commit_writer_emits_no_work_without_runtime_writes() {
    let request = writer_request(
        preflight_document(vec![], "no_commit_work"),
        "controlled_per_row",
    );

    let output = run_cli_with_json(&request.to_string());

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_replay_commit_writer"];
    assert_eq!(
        document["document_type"],
        "security_portfolio_execution_replay_commit_writer"
    );
    assert_eq!(document["commit_status"], "no_commit_work");
    assert_eq!(document["runtime_write_count"], 0);
    assert_eq!(document["commit_row_count"], 0);
    assert!(
        document["non_atomicity_notice"]
            .as_str()
            .unwrap_or_default()
            .contains("per-row")
    );
}

#[test]
fn security_portfolio_execution_replay_commit_writer_commits_one_ready_row_with_replay_metadata() {
    let runtime_db_path = create_test_runtime_db("p19d_commit_writer_happy_path");
    let server = prepare_security_environment(&runtime_db_path, "p19d_commit_writer_happy_path");
    let row = preflight_row("601916.SH", "buy", 0.12, "p19d-happy-key");
    let request = writer_request(
        preflight_document(vec![row.clone()], "commit_preflight_ready"),
        "controlled_per_row",
    );

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );

    // 2026-04-26 CST: Reason=P19D is the first approved runtime replay writer after P19C.
    // Purpose=prove it writes only one deterministic execution record with replay metadata.
    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_replay_commit_writer"];
    assert_eq!(document["commit_status"], "committed");
    assert_eq!(document["runtime_write_count"], 1);
    assert_eq!(document["committed_count"], 1);
    assert_eq!(document["already_committed_count"], 0);
    let commit_row = &document["commit_rows"][0];
    assert_eq!(commit_row["row_status"], "committed");
    assert_eq!(
        commit_row["target_execution_record_ref"],
        target_execution_record_ref(row["commit_idempotency_key"].as_str().unwrap())
    );

    let persisted = persisted_execution_record_json(
        &runtime_db_path,
        commit_row["target_execution_record_ref"].as_str().unwrap(),
    );
    assert_eq!(
        persisted["replay_commit_idempotency_key"],
        row["commit_idempotency_key"]
    );
    assert_eq!(
        persisted["replay_commit_payload_hash"],
        row["canonical_commit_payload_hash"]
    );
    assert_eq!(
        persisted["replay_commit_source_p19c_ref"],
        "portfolio-execution-replay-commit-preflight:acct-1:2026-04-26T10:00:00+08:00"
    );
}

#[test]
fn security_portfolio_execution_replay_commit_writer_rerun_reports_already_committed() {
    let runtime_db_path = create_test_runtime_db("p19d_commit_writer_already_committed");
    let server =
        prepare_security_environment(&runtime_db_path, "p19d_commit_writer_already_committed");
    let row = preflight_row("601916.SH", "buy", 0.12, "p19d-rerun-key");
    let request = writer_request(
        preflight_document(vec![row], "commit_preflight_ready"),
        "controlled_per_row",
    );

    let first_output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );
    assert_eq!(first_output["status"], "ok", "first output={first_output}");

    let second_output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );

    assert_eq!(
        second_output["status"], "ok",
        "second output={second_output}"
    );
    let document = &second_output["data"]["portfolio_execution_replay_commit_writer"];
    assert_eq!(
        document["commit_status"],
        "committed_with_already_committed"
    );
    assert_eq!(document["runtime_write_count"], 0);
    assert_eq!(document["committed_count"], 0);
    assert_eq!(document["already_committed_count"], 1);
    assert_eq!(
        document["commit_rows"][0]["row_status"],
        "already_committed"
    );
}

#[test]
fn security_portfolio_execution_replay_commit_writer_rejects_conflicting_runtime_target() {
    let runtime_db_path = create_test_runtime_db("p19d_commit_writer_conflict");
    let server = prepare_security_environment(&runtime_db_path, "p19d_commit_writer_conflict");
    let row = preflight_row("601916.SH", "buy", 0.12, "p19d-conflict-key");
    let target_ref = target_execution_record_ref(row["commit_idempotency_key"].as_str().unwrap());
    seed_conflicting_replay_record(&runtime_db_path, &server, &target_ref);
    let request = writer_request(
        preflight_document(vec![row], "commit_preflight_ready"),
        "controlled_per_row",
    );

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );

    assert_eq!(output["status"], "ok", "output={output}");
    let document = &output["data"]["portfolio_execution_replay_commit_writer"];
    assert_eq!(document["commit_status"], "rejected");
    assert_eq!(document["runtime_write_count"], 0);
    assert_eq!(document["idempotency_conflict_count"], 1);
    assert_eq!(
        document["commit_rows"][0]["row_status"],
        "idempotency_conflict"
    );

    let persisted = persisted_execution_record_json(&runtime_db_path, &target_ref);
    assert_eq!(
        persisted["replay_commit_idempotency_key"],
        "conflicting-seed-key"
    );
}

#[test]
fn security_portfolio_execution_replay_commit_writer_source_guard_blocks_direct_runtime_writes() {
    let source_path = Path::new("src")
        .join("ops")
        .join("security_portfolio_execution_replay_commit_writer.rs");
    let source =
        fs::read_to_string(&source_path).expect("P19D writer source should exist for source guard");

    assert!(source.contains("security_execution_record("));
    assert!(source.contains("load_execution_record("));
    assert!(!source.contains("upsert_execution_record("));
    assert!(!source.contains(".execute("));
    assert!(!source.contains("open_session("));
    assert!(!source.contains("INSERT INTO security_execution_records"));
    assert!(source.contains("non_atomicity_notice"));
    assert!(!source.contains("bundle atomic"));
}

fn writer_request(preflight: Value, commit_mode: &str) -> Value {
    json!({
        "tool": "security_portfolio_execution_replay_commit_writer",
        "args": {
            "portfolio_execution_replay_commit_preflight": preflight,
            "commit_mode": commit_mode,
            "created_at": "2026-04-26T11:00:00+08:00"
        }
    })
}

fn preflight_document(rows: Vec<Value>, status: &str) -> Value {
    json!({
        "portfolio_execution_replay_commit_preflight_id": "portfolio-execution-replay-commit-preflight:acct-1:2026-04-26T10:00:00+08:00",
        "contract_version": "security_portfolio_execution_replay_commit_preflight.v1",
        "document_type": "security_portfolio_execution_replay_commit_preflight",
        "generated_at": "2026-04-26T10:00:00+08:00",
        "analysis_date": "2025-09-17",
        "account_id": "acct-1",
        "preflight_mode": "commit_preflight_only",
        "portfolio_execution_replay_executor_ref": "portfolio-execution-replay-executor:acct-1:2026-04-26T09:00:00+08:00",
        "portfolio_execution_replay_request_package_ref": "portfolio-execution-replay-request-package:acct-1:2026-04-26T08:00:00+08:00",
        "portfolio_execution_request_enrichment_ref": "portfolio-execution-request-enrichment:acct-1:2026-04-25T15:00:00+08:00",
        "portfolio_execution_request_package_ref": "portfolio-execution-request-package:acct-1:2026-04-25T14:00:00+08:00",
        "portfolio_execution_preview_ref": "portfolio-execution-preview:acct-1:2026-04-25T13:00:00+08:00",
        "portfolio_allocation_decision_ref": "portfolio-allocation-decision:acct-1:2026-04-25T12:00:00+08:00",
        "preflight_rows": rows,
        "preflight_row_count": rows.len(),
        "runtime_write_count": 0,
        "preflight_status": status,
        "blockers": [],
        "preflight_rationale": ["fixture"],
        "preflight_summary": "fixture"
    })
}

fn preflight_row(symbol: &str, action: &str, gross_pct: f64, key_suffix: &str) -> Value {
    let commit_idempotency_key =
        format!("p19c|acct-1|2025-09-17|{symbol}|{action}|{gross_pct}|{key_suffix}");
    let payload = json!({
        "symbol": symbol,
        "analysis_date": "2025-09-17",
        "decision_ref": "decision-ref-p19d-fixture",
        "execution_action": action,
        "execution_status": "preflight_ready",
        "executed_gross_pct": gross_pct,
        "account_id": "acct-1",
        "as_of_date": "2025-09-17",
        "market_symbol": "510300.SH",
        "sector_symbol": "512800.SH",
        "market_regime": "a_share",
        "sector_template": "bank",
        "market_profile": "a_share_core",
        "sector_profile": "a_share_bank",
        "replay_evidence_refs": ["execution_record_ref:seed"],
        "source_p19b_idempotency_key": format!("p19b|acct-1|{symbol}|{key_suffix}")
    });
    let hash = hash_payload_preview(&payload);
    json!({
        "symbol": symbol,
        "request_action": action,
        "requested_gross_pct": gross_pct,
        "preflight_status": "preflight_ready",
        "source_p19b_idempotency_key": format!("p19b|acct-1|{symbol}|{key_suffix}"),
        "commit_idempotency_key": commit_idempotency_key,
        "canonical_commit_payload_hash": hash,
        "planned_execution_record_ref": format!("preflight:{key_suffix}"),
        "runtime_execution_record_ref": Value::Null,
        "commit_payload_preview": payload,
        "preflight_summary": "fixture row"
    })
}

fn hash_payload_preview(payload: &Value) -> String {
    let canonical = payload.to_string();
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn target_execution_record_ref(commit_idempotency_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(commit_idempotency_key.as_bytes());
    format!("execution-record-replay:{:x}", hasher.finalize())
}

fn seed_conflicting_replay_record(runtime_db_path: &PathBuf, server: &str, target_ref: &str) {
    let mut request = execution_record_seed_request();
    request["args"]["replay_commit_control"] = json!({
        "target_execution_record_ref": target_ref,
        "commit_idempotency_key": "conflicting-seed-key",
        "canonical_commit_payload_hash": "sha256:conflicting-seed-payload",
        "source_p19c_ref": "security_portfolio_execution_replay_commit_preflight:conflict-seed"
    });
    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        runtime_db_path,
        &security_envs(server),
    );
    assert_eq!(output["status"], "ok", "seed output={output}");
}

fn execution_record_seed_request() -> Value {
    json!({
        "tool": "security_execution_record",
        "args": {
            "symbol": "601916.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "market_regime": "a_share",
            "sector_template": "bank",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "as_of_date": "2025-09-17",
            "review_horizon_days": 20,
            "lookback_days": 260,
            "factor_lookback_days": 120,
            "disclosure_limit": 6,
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12,
            "account_id": "acct-1",
            "sector_tag": "bank",
            "actual_entry_date": "2025-09-18",
            "actual_entry_price": 62.40,
            "actual_position_pct": 0.12,
            "actual_exit_date": "",
            "actual_exit_price": 0.0,
            "exit_reason": "position_still_open",
            "execution_record_notes": ["p19d conflict seed"],
            "created_at": "2026-04-26T10:30:00+08:00"
        }
    })
}

fn persisted_execution_record_json(runtime_db_path: &Path, execution_record_id: &str) -> Value {
    let execution_db_path = runtime_db_path
        .parent()
        .map(|parent| parent.join("security_execution.db"))
        .filter(|path| path.exists())
        .unwrap_or_else(|| runtime_db_path.to_path_buf());
    let connection = Connection::open(execution_db_path).expect("runtime db should open");
    let payload: String = connection
        .query_row(
            "SELECT payload_json FROM security_execution_records WHERE execution_record_id = ?1",
            [execution_record_id],
            |row| row.get(0),
        )
        .expect("persisted execution record should load");
    serde_json::from_str(&payload).expect("persisted execution record payload should parse")
}

fn create_stock_history_csv(prefix: &str, file_name: &str, rows: &[String]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_portfolio_execution_replay_commit_writer")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("fixture dir should exist");

    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n")).expect("csv should be written");
    csv_path
}

fn build_review_rows(day_count: usize, start_close: f64) -> Vec<String> {
    let mut rows = vec!["trade_date,open,high,low,close,adj_close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("seed date should be valid");
    let mut close = start_close;

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let next_close = close + 0.20;
        let high = next_close + 0.45;
        let low = close - 0.30;
        let volume = 820_000 + offset as i64 * 4_000;
        let open = close;
        let adj_close = next_close;
        rows.push(format!(
            "{},{open:.2},{high:.2},{low:.2},{next_close:.2},{adj_close:.2},{volume}",
            trade_date.format("%Y-%m-%d")
        ));
        close = next_close;
    }

    rows
}

fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "p19d_commit_writer_fixture"
        }
    });

    let output = crate::common::run_cli_with_json_and_runtime(
        &request.to_string(),
        &runtime_db_path.to_path_buf(),
    );
    assert_eq!(output["status"], "ok");
}

fn prepare_security_environment(runtime_db_path: &Path, prefix: &str) -> String {
    let stock_csv = create_stock_history_csv(prefix, "stock.csv", &build_review_rows(420, 12.0));
    let market_csv =
        create_stock_history_csv(prefix, "market.csv", &build_review_rows(420, 3200.0));
    let sector_csv = create_stock_history_csv(prefix, "sector.csv", &build_review_rows(420, 960.0));
    import_history_csv(runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(runtime_db_path, &sector_csv, "512800.SH");

    spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 200 OK",
            r#"{"data":{"report_date":"2025-12-31","revenue":258000000000.0,"revenue_yoy_pct":5.2,"net_profit":9500000000.0,"net_profit_yoy_pct":4.1,"roe_pct":11.2}}"#,
            "application/json",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{"data":{"list":[{"notice_date":"2026-03-28","title":"annual profit distribution","art_code":"AN1","columns":[{"column_name":"announcement"}]}]}}"#,
            "application/json",
        ),
    ])
}

fn spawn_http_route_server(routes: Vec<(&str, &str, &str, &str)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test http server should bind");
    let address = format!("http://{}", listener.local_addr().expect("local addr"));
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
        for _ in 0..route_map.len() + 10 {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
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

fn security_envs(server: &str) -> Vec<(&'static str, String)> {
    vec![
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
    ]
}
