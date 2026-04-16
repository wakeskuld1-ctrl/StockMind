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

fn approx_equal(left: f64, right: f64) {
    let diff = (left - right).abs();
    assert!(
        diff <= 1e-9,
        "expected values to be approximately equal, left={left}, right={right}, diff={diff}"
    );
}

// 2026-04-09 CST: 这里新增 execution journal CLI 测试夹具目录助手，原因是 P1 要先把多笔成交 journal 锁成正式 Tool；
// 目的：继续沿正式 runtime + 行情导入链路验证，而不是在测试里跳过真实分析上下文。
fn create_stock_history_csv(prefix: &str, file_name: &str, rows: &[String]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_execution_journal")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("security execution journal fixture dir should exist");

    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n"))
        .expect("security execution journal csv should be written");
    csv_path
}

// 2026-04-09 CST: 这里复用本地 HTTP 假服务，原因是 journal 底层仍要走正式财报与公告上下文；
// 目的：把失败点收敛在多笔成交聚合合同，而不是联网波动。
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

// 2026-04-09 CST: 这里复用正式 stock history 导入链，原因是 journal 聚合仍依赖 position_plan 和 forward_outcome 同源；
// 目的：确保测试继续建立在 SQLite 历史主链之上。
fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_execution_journal_fixture"
        }
    });

    let output = crate::common::run_cli_with_json_and_runtime(
        &request.to_string(),
        &runtime_db_path.to_path_buf(),
    );
    assert_eq!(output["status"], "ok");
}

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
fn tool_catalog_includes_security_execution_journal() {
    let output = run_cli_with_json("");

    // 2026-04-09 CST: 这里先锁 execution journal Tool 的可发现性，原因是 P1 不是给 execution_record 偷偷塞数组字段；
    // 目的：确保多笔成交 journal 自身就是正式一等对象。
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_execution_journal"),
        "tool catalog should include security_execution_journal"
    );
}

#[test]
fn security_execution_journal_outputs_formal_document_with_multi_trade_aggregation() {
    let runtime_db_path = create_test_runtime_db("security_execution_journal_ready");
    let server = prepare_security_environment(&runtime_db_path, "security_execution_journal_ready");
    let request = execution_journal_request();

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );

    // 2026-04-09 CST: 这里先锁多笔成交正式合同，原因是 P1 的目标是“journal 明细 + 聚合摘要”；
    // 目的：确保输出里既有 trades，也有峰值仓位、加权买卖价和已实现收益。
    assert_eq!(
        output["status"], "ok",
        "security_execution_journal should succeed, output={output}"
    );
    assert_eq!(
        output["data"]["execution_journal"]["document_type"],
        "security_execution_journal"
    );
    assert_eq!(output["data"]["execution_journal"]["trade_count"], json!(4));
    assert_eq!(
        output["data"]["execution_journal"]["entry_trade_count"],
        json!(2)
    );
    assert_eq!(
        output["data"]["execution_journal"]["exit_trade_count"],
        json!(2)
    );
    assert_eq!(
        output["data"]["execution_journal"]["peak_position_pct"],
        json!(0.12)
    );
    assert_eq!(
        output["data"]["execution_journal"]["final_position_pct"],
        json!(0.0)
    );
    assert_eq!(
        output["data"]["execution_journal"]["holding_start_date"],
        json!("2025-09-18")
    );
    assert_eq!(
        output["data"]["execution_journal"]["holding_end_date"],
        json!("2025-10-02")
    );
    approx_equal(
        output["data"]["execution_journal"]["weighted_entry_price"]
            .as_f64()
            .expect("weighted entry price should be numeric"),
        (62.20_f64 * 0.07_f64 + 62.90_f64 * 0.05_f64) / 0.12_f64,
    );
    approx_equal(
        output["data"]["execution_journal"]["weighted_exit_price"]
            .as_f64()
            .expect("weighted exit price should be numeric"),
        (65.80_f64 * 0.06_f64 + 66.40_f64 * 0.06_f64) / 0.12_f64,
    );
    assert!(
        output["data"]["execution_journal"]["trades"]
            .as_array()
            .expect("journal trades should be an array")
            .iter()
            .any(|item| item["resulting_position_pct"] == json!(0.12))
    );
}

#[test]
fn security_execution_journal_allows_open_position_snapshot_without_forced_flat_exit() {
    let runtime_db_path = create_test_runtime_db("security_execution_journal_open_position");
    let server =
        prepare_security_environment(&runtime_db_path, "security_execution_journal_open_position");
    let mut request = execution_journal_request();
    // 2026-04-10 CST: 这里补“未平仓快照”红测，原因是方案A要先让执行链支持连续状态，
    // 目的：锁住 execution_journal 不再强制要求当前批次必须卖出清零后才能形成正式对象。
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

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );

    assert_eq!(
        output["status"], "ok",
        "security_execution_journal should allow open position snapshot, output={output}"
    );
    assert_eq!(
        output["data"]["execution_journal"]["position_state"],
        json!("open")
    );
    assert_eq!(
        output["data"]["execution_journal"]["exit_trade_count"],
        json!(0)
    );
    assert_eq!(
        output["data"]["execution_journal"]["final_position_pct"],
        json!(0.12)
    );
    assert_eq!(
        output["data"]["execution_journal"]["holding_end_date"],
        json!("2025-09-19")
    );
    assert_eq!(
        output["data"]["execution_journal"]["weighted_exit_price"],
        json!(0.0)
    );
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

fn execution_journal_request() -> Value {
    json!({
        "tool": "security_execution_journal",
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
            "execution_trades": [
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
            ],
            "execution_journal_notes": [
                "采用分批建仓、分批止盈的执行策略"
            ],
            "created_at": "2026-04-09T15:00:00+08:00"
        }
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
