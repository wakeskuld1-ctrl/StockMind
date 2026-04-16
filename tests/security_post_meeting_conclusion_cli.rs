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

// 2026-04-08 CST: 这里新增 Task 3 的独立 CLI 测试夹具，原因是会后结论对象化属于新的正式治理入口，必须先有独立红测来锁定合同；
// 目的：避免后续把会后结论继续塞回 revision 摘要或 approval_request 状态字段里，导致对象边界再次退化。
fn create_stock_history_csv(prefix: &str, file_name: &str, rows: &[String]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_post_meeting_conclusion")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("security post meeting fixture dir should exist");

    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n")).expect("security post meeting csv should be written");
    csv_path
}

// 2026-04-08 CST: 这里复用本地 HTTP 假服务，原因是会后结论 Tool 的 happy path 仍然需要先走真实 submit approval 主链生成 package；
// 目的：把财报与公告依赖限制在本地可控夹具里，让这条新治理链的红绿灯不受外部接口波动影响。
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
fn tool_catalog_includes_security_record_post_meeting_conclusion() {
    let output = run_cli_with_json("");

    // 2026-04-08 CST: 这里先锁定 Task 3 新 Tool 的可发现性，原因是没有 catalog 入口就不算正式治理能力；
    // 目的：确保后续 Skill、CLI 和审批链都能稳定找到“记录会后结论”这个正式入口。
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_record_post_meeting_conclusion")
    );
}

#[test]
fn security_record_post_meeting_conclusion_creates_conclusion_and_revises_package() {
    let runtime_db_path = create_test_runtime_db("security_post_meeting_conclusion_happy_path");
    let approval_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scenes_runtime");

    let stock_csv = create_stock_history_csv(
        "security_post_meeting_conclusion_happy_path",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_post_meeting_conclusion_happy_path",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_post_meeting_conclusion_happy_path",
        "sector.csv",
        &build_confirmed_breakout_rows(220, 950.0),
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

    let submit_request = json!({
        "tool": "security_decision_submit_approval",
        "args": {
            "symbol": "601916.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12,
            "approval_runtime_root": approval_root.to_string_lossy(),
            "created_at": "2026-04-08T16:00:00+08:00",
            "approval_brief_signing_key_id": "brief_signing_key_20260408",
            "approval_brief_signing_key_secret": "brief-secret-for-tests"
        }
    });
    let submit_output = run_cli_with_json_runtime_and_envs(
        &submit_request.to_string(),
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

    let package_path = submit_output["data"]["decision_package_path"]
        .as_str()
        .expect("decision package path should exist")
        .to_string();

    let record_request = json!({
        "tool": "security_record_post_meeting_conclusion",
        "args": {
            "package_path": package_path,
            "final_disposition": "approve",
            "disposition_reason": "committee_adopted_majority",
            "key_reasons": ["risk_cleared", "thesis_accepted"],
            "required_follow_ups": ["track_post_approval_execution"],
            "reviewer_notes": "committee accepted the majority view",
            "reviewer": "pm_lead",
            "reviewer_role": "PortfolioManager",
            "revision_reason": "post_meeting_conclusion_recorded",
            "reverify_after_revision": true,
            "approval_brief_signing_key_secret": "brief-secret-for-tests"
        }
    });
    let record_output =
        run_cli_with_json_runtime_and_envs(&record_request.to_string(), &runtime_db_path, &[]);

    // 2026-04-08 CST: 这里先锁定 Task 3 新 Tool 的最小 happy path，原因是方案 C 要求“会后结论对象 + 独立入口 + package revision”三者同时成立；
    // 目的：确保这不是单纯补一个 JSON 文件，而是真正形成会后结论落盘并驱动 package 进入新版本的正式治理动作。
    assert_eq!(record_output["status"], "ok");
    assert_eq!(
        record_output["data"]["post_meeting_conclusion"]["final_disposition"],
        "approve"
    );
    assert_eq!(
        record_output["data"]["post_meeting_conclusion"]["document_type"],
        "security_post_meeting_conclusion"
    );
    assert_eq!(record_output["data"]["package_version"], 2);
    assert_eq!(
        record_output["data"]["revision_reason"],
        "post_meeting_conclusion_recorded"
    );
    assert!(
        record_output["data"]["post_meeting_conclusion_path"]
            .as_str()
            .expect("post meeting conclusion path should exist")
            .contains("post_meeting_conclusions")
    );
    assert!(
        record_output["data"]["decision_package_path"]
            .as_str()
            .expect("decision package path should exist")
            .contains("decision_packages")
    );
}

fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_post_meeting_conclusion_fixture"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path.to_path_buf(),
        &[],
    );
    assert_eq!(output["status"], "ok");
}

fn build_confirmed_breakout_rows(day_count: usize, start_close: f64) -> Vec<String> {
    let mut rows = vec!["trade_date,open,high,low,close,adj_close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("seed date should be valid");
    let mut close = start_close;

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let (next_close, volume): (f64, i64) = if offset < day_count - 20 {
            (close + 0.78, 880_000 + offset as i64 * 8_000)
        } else {
            let phase = offset - (day_count - 20);
            match phase % 4 {
                0 => (close + 1.35, 1_700_000 + phase as i64 * 26_000),
                1 => (close - 0.18, 420_000),
                2 => (close + 1.08, 1_540_000 + phase as i64 * 22_000),
                _ => (close + 0.42, 1_240_000),
            }
        };

        let open = close;
        let high = next_close.max(open) + 1.0;
        let low = next_close.min(open) - 0.86;
        let adj_close = next_close;
        rows.push(format!(
            "{},{open:.2},{high:.2},{low:.2},{next_close:.2},{adj_close:.2},{volume}",
            trade_date.format("%Y-%m-%d")
        ));
        close = next_close;
    }

    rows
}
