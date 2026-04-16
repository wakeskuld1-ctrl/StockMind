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

// 2026-04-09 CST: 这里新增 security_position_plan CLI 测试夹具目录助手，原因是 Task 7 需要先从外层合同锁定独立仓位计划 Tool；
// 目的：让测试走真实的 CSV -> SQLite -> briefing/position_plan 链路，而不是只在内存里拼 JSON 自证。
fn create_stock_history_csv(prefix: &str, file_name: &str, rows: &[String]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_position_plan")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("security position plan fixture dir should exist");

    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n")).expect("security position plan csv should be written");
    csv_path
}

// 2026-04-09 CST: 这里复用本地 HTTP 假服务，原因是 security_decision_briefing 仍依赖财报与公告信息面；
// 目的：隔离外部接口波动，把失败点锁在仓位计划合同，而不是网络抓数。
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
        for _ in 0..route_map.len() + 4 {
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

// 2026-04-09 CST: 这里补股票历史导入测试助手，原因是仓位计划 Tool 不应绕过正式行情导入入口；
// 目的：确认外层 Tool 合同建立在真实 runtime 数据准备链上。
fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_position_plan_fixture"
        }
    });

    let output = crate::common::run_cli_with_json_and_runtime(
        &request.to_string(),
        &runtime_db_path.to_path_buf(),
    );
    assert_eq!(
        output["status"], "ok",
        "security_position_plan should succeed, output={output}"
    );
}

// 2026-04-09 CST: 这里提供最小向上突破样本，原因是 security_decision_briefing 需要稳定的技术面输入；
// 目的：为独立仓位计划 Tool 提供可复现、低维护成本的基础行情夹具。
fn build_confirmed_breakout_rows(day_count: usize, start_close: f64) -> Vec<String> {
    let mut rows = vec!["trade_date,open,high,low,close,adj_close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("seed date should be valid");
    let mut close = start_close;

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let (next_close, volume): (f64, i64) = if offset < day_count - 20 {
            (close + 0.38, 860_000 + offset as i64 * 5_200)
        } else {
            let phase = offset - (day_count - 20);
            match phase % 4 {
                0 => (close + 0.92, 1_620_000 + phase as i64 * 24_000),
                1 => (close - 0.12, 560_000),
                2 => (close + 0.66, 1_420_000 + phase as i64 * 20_000),
                _ => (close + 0.28, 1_050_000),
            }
        };

        let open = close;
        let high = next_close.max(open) + 0.84;
        let low = next_close.min(open) - 0.76;
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
fn tool_catalog_includes_security_position_plan() {
    let output = run_cli_with_json("");

    // 2026-04-09 CST: 这里先锁 security_position_plan 的可发现性，原因是如果 catalog 不暴露独立仓位计划 Tool，
    // 目的：那上层 Skill 仍会退回去手工从 briefing 拆字段，无法形成正式主链。
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_position_plan"),
        "tool catalog should include security_position_plan"
    );
}

#[test]
fn security_position_plan_outputs_formal_document_aligned_with_briefing() {
    let runtime_db_path = create_test_runtime_db("security_position_plan_ready");
    let server = prepare_security_environment(&runtime_db_path, "security_position_plan_ready");
    let request = position_plan_request();

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );

    // 2026-04-09 CST: 这里锁独立仓位计划 Tool 的正式对象合同，原因是 Task 7 的目标不是再造一套仓位算法，
    // 目的：而是证明外层文档完全对齐 briefing 内部同源的 position_plan / odds_brief / committee_payload。
    assert_eq!(
        output["status"], "ok",
        "security_position_plan should succeed, output={output}"
    );
    assert_eq!(
        output["data"]["position_plan_document"]["document_type"],
        "security_position_plan"
    );
    assert_eq!(
        output["data"]["position_plan_document"]["symbol"],
        output["data"]["briefing_core"]["symbol"]
    );
    assert_eq!(
        output["data"]["position_plan_document"]["analysis_date"],
        output["data"]["briefing_core"]["analysis_date"]
    );
    assert_eq!(
        output["data"]["position_plan_document"]["analysis_date_guard"],
        output["data"]["briefing_core"]["analysis_date_guard"]
    );
    assert_eq!(
        output["data"]["position_plan_document"]["analysis_date_guard"]["effective_trade_date"],
        output["data"]["position_plan_document"]["analysis_date"]
    );
    assert_eq!(
        output["data"]["position_plan_document"]["recommended_action"],
        output["data"]["briefing_core"]["committee_payload"]["recommended_action"]
    );
    assert_eq!(
        output["data"]["position_plan_document"]["confidence"],
        output["data"]["briefing_core"]["committee_payload"]["confidence"]
    );
    assert_eq!(
        output["data"]["position_plan_document"]["odds_grade"],
        output["data"]["briefing_core"]["odds_brief"]["odds_grade"]
    );
    assert_eq!(
        output["data"]["position_plan_document"]["historical_confidence"],
        output["data"]["briefing_core"]["odds_brief"]["historical_confidence"]
    );
    assert_eq!(
        output["data"]["position_plan_document"]["position_action"],
        output["data"]["briefing_core"]["position_plan"]["position_action"]
    );
    assert_eq!(
        output["data"]["position_plan_document"]["entry_mode"],
        output["data"]["briefing_core"]["position_plan"]["entry_mode"]
    );
    assert_eq!(
        output["data"]["position_plan_document"]["starter_position_pct"],
        output["data"]["briefing_core"]["position_plan"]["starter_position_pct"]
    );
    assert_eq!(
        output["data"]["position_plan_document"]["max_position_pct"],
        output["data"]["briefing_core"]["position_plan"]["max_position_pct"]
    );
    assert_eq!(
        output["data"]["position_plan_document"]["entry_tranche_pct"],
        output["data"]["briefing_core"]["position_plan"]["starter_position_pct"]
    );
    assert_eq!(
        output["data"]["position_plan_document"]["add_tranche_pct"],
        output["data"]["briefing_core"]["execution_plan"]["add_position_pct"]
    );
    assert_eq!(
        output["data"]["position_plan_document"]["reduce_tranche_pct"],
        output["data"]["briefing_core"]["execution_plan"]["reduce_position_pct"]
    );
    assert_eq!(
        output["data"]["position_plan_document"]["tranche_template"],
        "starter_plus_adds"
    );
    assert!(
        output["data"]["position_plan_document"]["max_tranche_count"].is_number(),
        "max tranche count should be exposed as numeric field"
    );
    assert!(
        output["data"]["position_plan_document"]["tranche_trigger_rules"]
            .as_array()
            .expect("tranche trigger rules should be array")
            .len()
            >= 2,
        "tranche trigger rules should expose layered execution guidance"
    );
    assert_eq!(
        output["data"]["position_plan_document"]["execution_notes"],
        output["data"]["briefing_core"]["position_plan"]["execution_notes"]
    );
    assert_eq!(
        output["data"]["position_plan_document"]["rationale"],
        output["data"]["briefing_core"]["position_plan"]["rationale"]
    );
}

fn prepare_security_environment(runtime_db_path: &Path, prefix: &str) -> String {
    let stock_csv = create_stock_history_csv(
        prefix,
        "stock.csv",
        &build_confirmed_breakout_rows(220, 18.20),
    );
    let market_csv = create_stock_history_csv(
        prefix,
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        prefix,
        "sector.csv",
        &build_confirmed_breakout_rows(220, 980.0),
    );
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

fn position_plan_request() -> serde_json::Value {
    json!({
        "tool": "security_position_plan",
        "args": {
            "symbol": "601916.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "market_regime": "a_share",
            "sector_template": "bank",
            "lookback_days": 180,
            "factor_lookback_days": 120,
            "disclosure_limit": 6,
            "created_at": "2026-04-09T10:00:00+08:00"
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
