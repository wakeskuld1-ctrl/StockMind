mod common;

use chrono::{Duration, NaiveDate};
use serde_json::json;
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

// 2026-04-09 CST: 这里新增 forward_outcome CLI 测试夹具，原因是 Task 3 要先把“未来多期限标签回填”的正式合同锁进红测；
// 目的：先验证 snapshot 绑定、多期限标签和关键字段输出，再做最小实现，避免后续训练主链反复改对象边界。
fn create_stock_history_csv(prefix: &str, file_name: &str, rows: &[String]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_forward_outcome")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("security forward outcome fixture dir should exist");

    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n")).expect("security forward outcome csv should be written");
    csv_path
}

// 2026-04-09 CST: 这里复用本地 HTTP 假服务，原因是 forward_outcome 方案 B 需要绑定 feature_snapshot，而 snapshot 当前仍依赖财报/公告上下文；
// 目的：让测试聚焦“未来标签对象合同”，不被外部 HTTP 波动影响。
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
fn tool_catalog_includes_security_forward_outcome() {
    let output = run_cli_with_json("");

    // 2026-04-09 CST: 这里先锁 forward_outcome Tool 的可发现性，原因是如果 catalog 没有正式暴露它，后续训练/回算主链就没有一等入口；
    // 目的：确保 CLI / Skill / 训练流水线都能稳定发现这条标签回填能力。
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_forward_outcome")
    );
}

#[test]
fn security_forward_outcome_returns_snapshot_bound_multi_horizon_labels() {
    let runtime_db_path = create_test_runtime_db("security_forward_outcome_ready");

    let stock_csv = create_stock_history_csv(
        "security_forward_outcome_ready",
        "stock.csv",
        &build_linear_growth_rows(420, 100.0, 1.0),
    );
    let market_csv = create_stock_history_csv(
        "security_forward_outcome_ready",
        "market.csv",
        &build_linear_growth_rows(420, 3200.0, 5.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_forward_outcome_ready",
        "sector.csv",
        &build_linear_growth_rows(420, 950.0, 2.0),
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
                        {"notice_date":"2026-03-28","title":"2025年度报告","art_code":"AN202603281234567890","columns":[{"column_name":"定期报告"}]},
                        {"notice_date":"2026-03-28","title":"2025年度利润分配预案公告","art_code":"AN202603281234567891","columns":[{"column_name":"公司公告"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_forward_outcome",
        "args": {
            "symbol": "601916.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "as_of_date": "2025-08-28",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12
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

    // 2026-04-09 CST: 这里先锁方案 B 的最小正式合同，原因是 Task 3 目标不是临时算几个数字，而是生成能直接喂给训练/回算的正式标签对象；
    // 目的：要求输出同时包含绑定 snapshot、固定 6 个期限、关键收益/回撤/事件标签和 label_definition_version。
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["snapshot"]["document_type"],
        "security_feature_snapshot"
    );
    assert_eq!(
        output["data"]["snapshot"]["snapshot_id"],
        "snapshot-601916.SH-2025-08-28"
    );

    let forward_outcomes = output["data"]["forward_outcomes"]
        .as_array()
        .expect("forward outcomes should be an array");
    assert_eq!(forward_outcomes.len(), 6);

    let horizon_5 = find_horizon(forward_outcomes, 5);
    assert_eq!(horizon_5["document_type"], "security_forward_outcome");
    assert_eq!(horizon_5["snapshot_id"], "snapshot-601916.SH-2025-08-28");
    assert_eq!(horizon_5["positive_return"], true);
    assert_eq!(horizon_5["hit_upside_first"], false);
    assert_eq!(horizon_5["hit_stop_first"], false);
    assert_float_eq(
        horizon_5["forward_return"].as_f64().unwrap(),
        5.0 / 340.0,
        1e-9,
    );
    assert_float_eq(horizon_5["max_drawdown"].as_f64().unwrap(), 0.0, 1e-9);
    assert_float_eq(horizon_5["max_runup"].as_f64().unwrap(), 5.0 / 340.0, 1e-9);

    let horizon_20 = find_horizon(forward_outcomes, 20);
    assert_eq!(horizon_20["hit_upside_first"], false);
    assert_eq!(horizon_20["hit_stop_first"], false);
    assert_eq!(
        horizon_20["label_definition_version"],
        "security_forward_outcome.v1"
    );
    assert_float_eq(
        horizon_20["forward_return"].as_f64().unwrap(),
        20.0 / 340.0,
        1e-9,
    );

    let horizon_60 = find_horizon(forward_outcomes, 60);
    assert_eq!(horizon_60["hit_upside_first"], true);
    assert_eq!(horizon_60["hit_stop_first"], false);
    assert_float_eq(
        horizon_60["forward_return"].as_f64().unwrap(),
        60.0 / 340.0,
        1e-9,
    );

    let horizon_180 = find_horizon(forward_outcomes, 180);
    assert_eq!(horizon_180["positive_return"], true);
    assert_float_eq(
        horizon_180["forward_return"].as_f64().unwrap(),
        180.0 / 340.0,
        1e-9,
    );
}

fn find_horizon<'a>(
    forward_outcomes: &'a [serde_json::Value],
    horizon_days: i64,
) -> &'a serde_json::Value {
    forward_outcomes
        .iter()
        .find(|item| item["horizon_days"].as_i64() == Some(horizon_days))
        .expect("requested horizon should exist")
}

fn assert_float_eq(actual: f64, expected: f64, tolerance: f64) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tolerance,
        "expected {expected}, got {actual}, diff {diff}, tolerance {tolerance}"
    );
}

fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_forward_outcome_fixture"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path.to_path_buf(),
        &[],
    );
    assert_eq!(output["status"], "ok");
}

// 2026-04-09 CST: 这里构造线性上涨样本，原因是 Task 3 只需要先锁定 forward_return / runup / drawdown / 事件标签的计算合同；
// 目的：用可手算的价格路径降低噪声，让失败点聚焦在标签生成逻辑本身。
fn build_linear_growth_rows(day_count: usize, start_close: f64, daily_step: f64) -> Vec<String> {
    let mut rows = vec!["trade_date,open,high,low,close,adj_close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("seed date should be valid");
    let mut close = start_close;

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let next_close = close + daily_step;
        let open = close;
        let high = next_close + 0.5;
        let low = open.min(next_close) - 0.5;
        let adj_close = next_close;
        let volume = 1_000_000 + offset as i64 * 10_000;
        rows.push(format!(
            "{},{open:.2},{high:.2},{low:.2},{next_close:.2},{adj_close:.2},{volume}",
            trade_date.format("%Y-%m-%d")
        ));
        close = next_close;
    }

    rows
}
