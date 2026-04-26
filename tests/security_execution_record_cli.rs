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

use crate::common::{
    create_test_runtime_db, run_cli_with_json, run_cli_with_json_runtime_and_envs,
};

fn approx_equal(left: f64, right: f64) {
    let diff = (left - right).abs();
    assert!(
        diff <= 1e-9,
        "expected values to be approximately equal, left={left}, right={right}, diff={diff}"
    );
}

// 2026-04-09 CST: 这里新增 execution record CLI 测试夹具目录助手，原因是 Task 10 要先从外层合同锁定“真实执行对象”；
// 目的：确保后续实现沿正式 runtime + 行情导入链路回归，而不是在测试里直接拼伪对象。
fn create_stock_history_csv(prefix: &str, file_name: &str, rows: &[String]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_execution_record")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("security execution record fixture dir should exist");

    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n")).expect("security execution record csv should be written");
    csv_path
}

// 2026-04-09 CST: 这里复用本地 HTTP 假服务，原因是 execution record 底层仍复用财报与公告上下文；
// 目的：隔离联网噪声，让本轮失败点只落在真实执行合同和收益归因逻辑上。
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
        for _ in 0..route_map.len() + 6 {
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

// 2026-04-09 CST: 这里复用正式 stock history 导入链，原因是 execution record 归因依赖分析时点与未来窗口同源；
// 目的：让执行归因测试继续建立在 SQLite 历史主链上，而不是绕过正式数据入口。
fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_execution_record_fixture"
        }
    });

    let output = crate::common::run_cli_with_json_and_runtime(
        &request.to_string(),
        &runtime_db_path.to_path_buf(),
    );
    assert_eq!(output["status"], "ok");
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

// 2026-04-09 CST: 这里沿用“先震荡再上行”的稳定样本，原因是 Task 10 要验证真实执行收益归因而不是行情识别本身；
// 目的：让 entry/exit 价格与 forward outcome 都能落在同一条可解释路径上。
fn build_review_rows(day_count: usize, start_close: f64) -> Vec<String> {
    let mut rows = vec!["trade_date,open,high,low,close,adj_close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("seed date should be valid");
    let mut close = start_close;

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let (next_close, high, low, volume) = if offset < 260 {
            let next_close = close + 0.20;
            (
                next_close,
                next_close + 0.45,
                close - 0.30,
                820_000 + offset as i64 * 4_000,
            )
        } else {
            let phase = offset - 260;
            let next_close = match phase {
                0..=4 => close + 0.10,
                5..=9 => close - 0.05,
                10..=19 => close + 0.35,
                _ => close + 0.28,
            };
            (
                next_close,
                next_close + 0.40,
                next_close - 0.22,
                1_100_000 + phase as i64 * 18_000,
            )
        };
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

#[test]
fn tool_catalog_includes_security_execution_record() {
    let output = run_cli_with_json("");

    // 2026-04-09 CST: 这里先锁 execution record Tool 的可发现性，原因是 Task 10 不能只把真实执行对象藏在 review 内部；
    // 目的：让 CLI / Skill / 后续治理链都能显式发现独立 execution record 入口。
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_execution_record"),
        "tool catalog should include security_execution_record"
    );
}

#[test]
fn security_execution_record_outputs_formal_record_with_return_attribution() {
    let runtime_db_path = create_test_runtime_db("security_execution_record_ready");
    let server = prepare_security_environment(&runtime_db_path, "security_execution_record_ready");
    let request = execution_request();

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );

    // 2026-04-09 CST: 这里锁真实执行对象和收益归因合同，原因是 Task 10 的目标不是新增一个空壳 Tool，
    // 目的：确保它能把计划、快照、未来结果与真实成交绑成一份正式 execution record。
    assert_eq!(
        output["status"], "ok",
        "security_execution_record should succeed, output={output}"
    );
    assert_eq!(
        output["data"]["execution_record"]["document_type"],
        "security_execution_record"
    );
    assert_eq!(
        output["data"]["execution_journal"]["document_type"],
        "security_execution_journal"
    );
    assert_eq!(
        output["data"]["execution_record"]["execution_journal_ref"],
        output["data"]["execution_journal"]["execution_journal_id"]
    );
    assert_eq!(
        output["data"]["execution_record"]["position_plan_ref"],
        output["data"]["position_plan_result"]["position_plan_document"]["position_plan_id"]
    );
    assert_eq!(
        output["data"]["execution_record"]["snapshot_ref"],
        output["data"]["forward_outcome_result"]["snapshot"]["snapshot_id"]
    );
    assert_eq!(
        output["data"]["execution_record"]["outcome_ref"],
        output["data"]["forward_outcome_result"]["selected_outcome"]["outcome_id"]
    );
    assert_eq!(
        output["data"]["execution_record"]["actual_entry_price"],
        json!(62.40)
    );
    assert_eq!(
        output["data"]["execution_record"]["actual_exit_price"],
        json!(66.10)
    );
    assert_eq!(
        output["data"]["execution_record"]["actual_position_pct"],
        json!(0.12)
    );
    assert_eq!(
        output["data"]["execution_record"]["actual_return"],
        json!(66.10_f64 / 62.40_f64 - 1.0_f64)
    );
    assert_eq!(
        output["data"]["execution_record"]["holding_days"],
        json!(14)
    );
    approx_equal(
        output["data"]["execution_record"]["position_size_gap_pct"]
            .as_f64()
            .expect("position size gap should be numeric"),
        output["data"]["execution_record"]["actual_position_pct"]
            .as_f64()
            .expect("actual position pct should be numeric")
            - output["data"]["execution_record"]["planned_position_pct"]
                .as_f64()
                .expect("planned position pct should be numeric"),
    );
    approx_equal(
        output["data"]["execution_record"]["execution_return_gap"]
            .as_f64()
            .expect("execution return gap should be numeric"),
        output["data"]["execution_record"]["actual_return"]
            .as_f64()
            .expect("actual return should be numeric")
            - output["data"]["execution_record"]["planned_forward_return"]
                .as_f64()
                .expect("planned forward return should be numeric"),
    );
    assert!(
        output["data"]["execution_record"]["execution_quality"]
            .as_str()
            .expect("execution quality should be present")
            .len()
            > 0
    );
}

#[test]
fn security_execution_record_replay_control_forces_id_and_machine_metadata() {
    let runtime_db_path = create_test_runtime_db("security_execution_record_replay_control");
    let server =
        prepare_security_environment(&runtime_db_path, "security_execution_record_replay_control");
    let mut request = execution_request();
    request["args"]["replay_commit_control"] = json!({
        "target_execution_record_ref": "execution-record-replay:test-stable-target",
        "commit_idempotency_key": "p19d-idempotency:test-stable-target",
        "canonical_commit_payload_hash": "sha256:test-stable-payload",
        "source_p19c_ref": "security_portfolio_execution_replay_commit_preflight:test-source"
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );

    // 2026-04-26 CST: Reason=P19D needs deterministic replay writes through this existing tool.
    // Purpose=lock replay-control identity and machine-readable evidence before adding P19D.
    assert_eq!(output["status"], "ok", "output={output}");
    let record = &output["data"]["execution_record"];
    assert_eq!(
        record["execution_record_id"],
        json!("execution-record-replay:test-stable-target")
    );
    assert_eq!(
        record["replay_commit_idempotency_key"],
        json!("p19d-idempotency:test-stable-target")
    );
    assert_eq!(
        record["replay_commit_payload_hash"],
        json!("sha256:test-stable-payload")
    );
    assert_eq!(
        record["replay_commit_source_p19c_ref"],
        json!("security_portfolio_execution_replay_commit_preflight:test-source")
    );

    let persisted = persisted_execution_record_json(
        &runtime_db_path,
        "execution-record-replay:test-stable-target",
    );
    assert_eq!(
        persisted["replay_commit_idempotency_key"],
        json!("p19d-idempotency:test-stable-target")
    );
    assert_eq!(
        persisted["replay_commit_payload_hash"],
        json!("sha256:test-stable-payload")
    );
}

#[test]
fn security_execution_record_replay_control_rejects_conflicting_target_without_overwrite() {
    let runtime_db_path = create_test_runtime_db("security_execution_record_replay_conflict");
    let server = prepare_security_environment(
        &runtime_db_path,
        "security_execution_record_replay_conflict",
    );
    let target_ref = "execution-record-replay:test-conflict-target";

    let mut first_request = execution_request();
    first_request["args"]["replay_commit_control"] = json!({
        "target_execution_record_ref": target_ref,
        "commit_idempotency_key": "p19d-idempotency:first",
        "canonical_commit_payload_hash": "sha256:first-payload",
        "source_p19c_ref": "security_portfolio_execution_replay_commit_preflight:first"
    });
    let first_output = run_cli_with_json_runtime_and_envs(
        &first_request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );
    assert_eq!(first_output["status"], "ok", "first output={first_output}");

    let mut conflicting_request = execution_request();
    conflicting_request["args"]["replay_commit_control"] = json!({
        "target_execution_record_ref": target_ref,
        "commit_idempotency_key": "p19d-idempotency:second",
        "canonical_commit_payload_hash": "sha256:second-payload",
        "source_p19c_ref": "security_portfolio_execution_replay_commit_preflight:second"
    });
    let conflicting_output = run_cli_with_json_runtime_and_envs(
        &conflicting_request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );

    // 2026-04-26 CST: Reason=the repository upsert updates on conflict by default.
    // Purpose=prove replay-control detects conflicting evidence inside the write path first.
    assert_eq!(
        conflicting_output["status"], "error",
        "conflicting output={conflicting_output}"
    );
    assert!(
        conflicting_output["error"]
            .as_str()
            .unwrap_or_default()
            .contains("replay commit conflict"),
        "conflicting output should explain replay conflict, output={conflicting_output}"
    );

    let persisted = persisted_execution_record_json(&runtime_db_path, target_ref);
    assert_eq!(
        persisted["replay_commit_idempotency_key"],
        json!("p19d-idempotency:first")
    );
    assert_eq!(
        persisted["replay_commit_payload_hash"],
        json!("sha256:first-payload")
    );
}

#[test]
fn security_execution_record_aggregates_multi_trade_journal_into_formal_record() {
    let runtime_db_path = create_test_runtime_db("security_execution_record_journal_ready");
    let server =
        prepare_security_environment(&runtime_db_path, "security_execution_record_journal_ready");
    let mut request = execution_request();
    request["args"]["execution_trades"] = json!([
        {
            "trade_date": "2025-09-18",
            "side": "buy",
            "price": 62.20,
            "position_pct_delta": 0.07,
            "reason": "breakout_entry",
            "notes": ["首次突破后建仓"]
        },
        {
            "trade_date": "2025-09-19",
            "side": "buy",
            "price": 62.90,
            "position_pct_delta": 0.05,
            "reason": "pullback_add",
            "notes": ["回踩确认后补仓"]
        },
        {
            "trade_date": "2025-09-29",
            "side": "sell",
            "price": 65.80,
            "position_pct_delta": 0.06,
            "reason": "partial_take_profit",
            "notes": ["先兑现一半利润"]
        },
        {
            "trade_date": "2025-10-02",
            "side": "sell",
            "price": 66.40,
            "position_pct_delta": 0.06,
            "reason": "full_exit",
            "notes": ["目标位附近全部退出"]
        }
    ]);
    request["args"]["execution_journal_notes"] = json!(["采用分批建仓与分批止盈"]);

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );

    // 2026-04-09 CST: 这里新增 journal -> record 聚合红测，原因是 P1 不是只新增一个 journal Tool；
    // 目的：确保 execution_record 真正复用多笔成交聚合结果，而不是仍旧只看单次进出字段。
    assert_eq!(output["status"], "ok");
    assert_eq!(output["data"]["execution_journal"]["trade_count"], json!(4));
    assert_eq!(
        output["data"]["execution_record"]["actual_entry_price"],
        output["data"]["execution_journal"]["weighted_entry_price"]
    );
    assert_eq!(
        output["data"]["execution_record"]["actual_exit_price"],
        output["data"]["execution_journal"]["weighted_exit_price"]
    );
    assert_eq!(
        output["data"]["execution_record"]["actual_position_pct"],
        output["data"]["execution_journal"]["peak_position_pct"]
    );
}

#[test]
fn security_execution_record_captures_account_plan_alignment_for_tranche_execution() {
    let runtime_db_path = create_test_runtime_db("security_execution_record_account_alignment");
    let server = prepare_security_environment(
        &runtime_db_path,
        "security_execution_record_account_alignment",
    );
    let mut request = execution_request();
    // 2026-04-09 CST: 这里显式注入账户级仓位计划，原因是方案A-2要把“账户层建议”正式回写到 execution record；
    // 目的：先从 CLI 合同锁定 planned tranche / actual tranche / 预算对齐状态，避免实现时退回成只看单票仓位。
    request["args"]["portfolio_position_plan_document"] =
        portfolio_position_plan_document_fixture();
    request["args"]["execution_trades"] = json!([
        {
            "trade_date": "2025-09-18",
            "side": "buy",
            "price": 62.20,
            "position_pct_delta": 0.07,
            "reason": "breakout_entry",
            "notes": ["首次突破后加仓"]
        },
        {
            "trade_date": "2025-09-19",
            "side": "buy",
            "price": 62.90,
            "position_pct_delta": 0.05,
            "reason": "pullback_add",
            "notes": ["回踩确认后补仓"]
        },
        {
            "trade_date": "2025-09-29",
            "side": "sell",
            "price": 65.80,
            "position_pct_delta": 0.06,
            "reason": "partial_take_profit",
            "notes": ["先兑现一半利润"]
        },
        {
            "trade_date": "2025-10-02",
            "side": "sell",
            "price": 66.40,
            "position_pct_delta": 0.06,
            "reason": "full_exit",
            "notes": ["目标位附近全部退出"]
        }
    ]);

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );

    // 2026-04-09 CST: 这里先补账户级偏差回写红测，原因是方案A-2不只是记录真实成交，还要能对上账户层仓位建议；
    // 目的：锁定 execution_record 对“计划层 vs 实际层”的正式输出，后续 review/package 才能稳定复用。
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["execution_record"]["portfolio_position_plan_ref"],
        "portfolio-position-plan-acct-main-2026-04-09"
    );
    assert_eq!(
        output["data"]["execution_record"]["planned_tranche_action"],
        "add_tranche"
    );
    assert_eq!(
        output["data"]["execution_record"]["planned_tranche_pct"],
        json!(0.08)
    );
    assert_eq!(
        output["data"]["execution_record"]["planned_peak_position_pct"],
        json!(0.12)
    );
    assert_eq!(
        output["data"]["execution_record"]["actual_tranche_action"],
        "add_tranche"
    );
    assert_eq!(
        output["data"]["execution_record"]["actual_tranche_pct"],
        json!(0.12)
    );
    assert_eq!(
        output["data"]["execution_record"]["actual_peak_position_pct"],
        json!(0.16)
    );
    assert_eq!(
        output["data"]["execution_record"]["account_budget_alignment"],
        "over_budget"
    );
    assert_eq!(
        output["data"]["execution_record"]["tranche_count_drift"],
        json!(1)
    );
}

#[test]
fn security_execution_record_supports_open_position_snapshot() {
    let runtime_db_path = create_test_runtime_db("security_execution_record_open_position");
    let server =
        prepare_security_environment(&runtime_db_path, "security_execution_record_open_position");
    let mut request = execution_request();
    // 2026-04-10 CST: 这里补“未平仓 execution_record”红测，原因是账户层连续状态不能要求每次都先完整平仓，
    // 目的：锁住 execution_record 能把当前仍在持仓的执行快照正式沉淀出来。
    request["args"]["execution_trades"] = json!([
        {
            "trade_date": "2025-09-18",
            "side": "buy",
            "price": 62.20,
            "position_pct_delta": 0.07,
            "reason": "breakout_entry",
            "notes": ["首次突破后建仓"]
        },
        {
            "trade_date": "2025-09-19",
            "side": "buy",
            "price": 62.90,
            "position_pct_delta": 0.05,
            "reason": "pullback_add",
            "notes": ["回踩确认后补仓"]
        }
    ]);
    request["args"]["execution_record_notes"] = json!(["当前仍在持仓，先记录执行快照"]);

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["execution_record"]["position_state"],
        json!("open")
    );
    // 2026-04-10 CST: 这里锁账户绑定字段，原因是后续 runtime 自动回接 open snapshot 必须按账户维度读取 execution_record；
    // 目的：防止 execution_record 继续只是一笔孤立执行摘要，无法进入账户层连续状态链。
    assert_eq!(
        output["data"]["execution_record"]["account_id"],
        json!("acct-demo-001")
    );
    assert_eq!(
        output["data"]["execution_record"]["sector_tag"],
        json!("bank")
    );
    assert_eq!(
        output["data"]["execution_record"]["actual_exit_date"],
        json!("")
    );
    assert_eq!(
        output["data"]["execution_record"]["actual_exit_price"],
        json!(0.0)
    );
    assert_eq!(
        output["data"]["execution_record"]["exit_reason"],
        json!("position_still_open")
    );
    assert_eq!(
        output["data"]["execution_record"]["actual_position_pct"],
        json!(0.12)
    );
    // 2026-04-10 CST: 这里锁定 current_position_pct，原因是账户层下一轮要消费“当前还剩多少仓位”而不是峰值仓位；
    // 目的：防止 execution_record 继续只暴露 peak position，导致 open snapshot 无法正式回接到账户输入。
    assert_eq!(
        output["data"]["execution_record"]["current_position_pct"],
        json!(0.12)
    );
    assert_eq!(output["data"]["execution_record"]["holding_days"], json!(1));
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
            r#"[
                {
                    "REPORT_DATE":"2025-12-31",
                    "NOTICE_DATE":"2026-03-28",
                    "TOTAL_OPERATE_INCOME":258000000000.0,
                    "YSTZ":5.20,
                    "PARENT_NETPROFIT":9500000000.0,
                    "SJLTZ":4.10,
                    "ROEJQ":11.20
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
                        {"notice_date":"2026-03-28","title":"2025年度利润分配预案公告","art_code":"AN202603281010101010","columns":[{"column_name":"公司公告"}]},
                        {"notice_date":"2026-03-20","title":"关于回购进展的公告","art_code":"AN202603201010101011","columns":[{"column_name":"公司公告"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ])
}

fn execution_request() -> Value {
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
            "account_id": "acct-demo-001",
            "sector_tag": "bank",
            "actual_entry_date": "2025-09-18",
            "actual_entry_price": 62.40,
            "actual_position_pct": 0.12,
            "actual_exit_date": "2025-10-02",
            "actual_exit_price": 66.10,
            "exit_reason": "take_profit_partial",
            "execution_record_notes": [
                "突破后次日回踩确认介入",
                "提前在目标位附近落袋一部分收益"
            ],
            "created_at": "2026-04-09T12:00:00+08:00"
        }
    })
}

fn portfolio_position_plan_document_fixture() -> Value {
    json!({
        "portfolio_position_plan_id": "portfolio-position-plan-acct-main-2026-04-09",
        "contract_version": "security_portfolio_position_plan.v1",
        "document_type": "security_portfolio_position_plan",
        "generated_at": "2026-04-09T11:30:00+08:00",
        "account_id": "acct-main",
        "total_equity": 100000.0,
        "available_cash": 28000.0,
        "current_cash_pct": 0.28,
        "min_cash_reserve_pct": 0.10,
        "deployable_cash_amount": 18000.0,
        "deployable_cash_pct": 0.18,
        "current_invested_pct": 0.72,
        "max_portfolio_risk_budget_pct": 0.12,
        "current_portfolio_risk_budget_pct": 0.05,
        "remaining_portfolio_risk_budget_pct": 0.07,
        "max_single_trade_risk_budget_pct": 0.03,
        "estimated_new_risk_budget_pct": 0.02,
        "total_portfolio_risk_budget_pct": 0.07,
        "concentration_warnings": [],
        "risk_budget_warnings": [],
        "allocations": [
            {
                "symbol": "601916.SH",
                "action": "add",
                "sector_tag": "bank",
                "current_position_pct": 0.04,
                "target_position_pct": 0.12,
                "incremental_position_pct": 0.08,
                "recommended_trade_amount": 8000.0,
                "estimated_risk_budget_pct": 0.02,
                "suggested_tranche_action": "add_tranche",
                "suggested_tranche_pct": 0.08,
                "remaining_tranche_count": 1,
                "priority_score": 82,
                "constraint_flags": [],
                "rationale": [
                    "账户层允许继续加仓",
                    "本次建议走第二层加仓"
                ]
            }
        ],
        "portfolio_summary": "账户层建议对 601916.SH 继续执行一层加仓。"
    })
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
