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

// 2026-04-09 CST: 这里新增投后复盘 CLI 测试夹具目录助手，原因是 Task 8 要先从外层合同锁定正式复盘 Tool；
// 目的：让测试走真实的行情导入、仓位计划、未来结果与复盘装配链路，而不是只在内存里拼对象。
fn create_stock_history_csv(prefix: &str, file_name: &str, rows: &[String]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_post_trade_review")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("security post trade review fixture dir should exist");

    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n"))
        .expect("security post trade review csv should be written");
    csv_path
}

// 2026-04-09 CST: 这里复用本地 HTTP 假服务，原因是复盘链路底层仍依赖财报与公告上下文；
// 目的：把失败点收敛在投后复盘合同本身，而不是网络波动。
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

// 2026-04-09 CST: 这里补历史导入测试助手，原因是投后复盘仍应建立在正式 stock history runtime 上；
// 目的：确保复盘 Tool 验证的是主链能力，不是手工伪造数据。
fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_post_trade_review_fixture"
        }
    });

    let output = crate::common::run_cli_with_json_and_runtime(
        &request.to_string(),
        &runtime_db_path.to_path_buf(),
    );
    assert_eq!(output["status"], "ok");
}

// 2026-04-09 CST: 这里构造“先震荡再上行”的样本，原因是复盘第一版需要同时验证收益、浮盈与 thesis 状态；
// 目的：让复盘 Tool 在正向兑现情形下有稳定、可解释的回归样本。
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
fn tool_catalog_includes_security_post_trade_review() {
    let output = run_cli_with_json("");

    // 2026-04-09 CST: 这里先锁投后复盘 Tool 的可发现性，原因是 Task 8 要正式补齐投后层；
    // 目的：避免后续继续靠手工把 position_plan 与 forward_outcome 拼接成“伪复盘”。
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_post_trade_review"),
        "tool catalog should include security_post_trade_review"
    );
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
fn security_post_trade_review_outputs_formal_review_document() {
    let runtime_db_path = create_test_runtime_db("security_post_trade_review_ready");
    let server = prepare_security_environment(&runtime_db_path, "security_post_trade_review_ready");
    let request = review_request();

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &security_envs(&server),
    );

    // 2026-04-09 CST: 这里锁最小正式投后复盘合同，原因是 Task 8 的目标是把投后复盘变成正式对象，
    // 目的：确保复盘文档与 position_plan、snapshot、forward_outcome 同源对齐，并显式给出 thesis 结论与调整提示。
    assert_eq!(
        output["status"], "ok",
        "security_post_trade_review should succeed, output={output}"
    );
    assert_eq!(
        output["data"]["post_trade_review"]["document_type"],
        "security_post_trade_review"
    );
    assert_eq!(
        output["data"]["post_trade_review"]["symbol"],
        output["data"]["position_plan_result"]["position_plan_document"]["symbol"]
    );
    assert_eq!(
        output["data"]["post_trade_review"]["snapshot_date"],
        output["data"]["forward_outcome_result"]["snapshot"]["as_of_date"]
    );
    assert_eq!(
        output["data"]["post_trade_review"]["position_plan_ref"],
        output["data"]["position_plan_result"]["position_plan_document"]["position_plan_id"]
    );
    assert_eq!(
        output["data"]["post_trade_review"]["snapshot_ref"],
        output["data"]["forward_outcome_result"]["snapshot"]["snapshot_id"]
    );
    assert_eq!(
        output["data"]["post_trade_review"]["execution_record_ref"],
        output["data"]["execution_record"]["execution_record_id"]
    );
    assert_eq!(
        output["data"]["post_trade_review"]["realized_return"],
        output["data"]["forward_outcome_result"]["selected_outcome"]["forward_return"]
    );
    assert_eq!(
        output["data"]["post_trade_review"]["max_drawdown_realized"],
        output["data"]["forward_outcome_result"]["selected_outcome"]["max_drawdown"]
    );
    assert_eq!(
        output["data"]["post_trade_review"]["max_runup_realized"],
        output["data"]["forward_outcome_result"]["selected_outcome"]["max_runup"]
    );
    assert_eq!(
        output["data"]["post_trade_review"]["planned_position"]["max_position_pct"],
        output["data"]["position_plan_result"]["position_plan_document"]["max_position_pct"]
    );
    assert_eq!(
        output["data"]["post_trade_review"]["executed_return"],
        output["data"]["execution_record"]["actual_return"]
    );
    assert_eq!(
        output["data"]["post_trade_review"]["execution_return_gap"],
        output["data"]["execution_record"]["execution_return_gap"]
    );
    assert_eq!(
        output["data"]["post_trade_review"]["thesis_status"],
        "validated"
    );
    assert_eq!(
        output["data"]["post_trade_review"]["execution_deviation"],
        output["data"]["execution_record"]["execution_quality"]
    );
}

#[test]
fn security_post_trade_review_summarizes_account_plan_drift_and_next_hint() {
    let runtime_db_path = create_test_runtime_db("security_post_trade_review_account_alignment");
    let server = prepare_security_environment(
        &runtime_db_path,
        "security_post_trade_review_account_alignment",
    );
    let mut request = review_request();
    // 2026-04-09 CST: 这里把账户级仓位计划一并送入复盘请求，原因是方案A-2要求投后层能直接解释账户偏差；
    // 目的：锁定 review 合同里的账户计划对齐结论，避免后续还要靠外层手工拼接偏差说明。
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

    // 2026-04-09 CST: 这里先补投后层账户偏差红测，原因是 execution record 记完事实后，review 还要给出正式治理结论；
    // 目的：锁定 account_plan_alignment / tranche_discipline / budget_drift_reason / next_account_adjustment_hint 的输出合同。
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["post_trade_review"]["account_plan_alignment"],
        "over_budget"
    );
    assert_eq!(
        output["data"]["post_trade_review"]["tranche_discipline"],
        "overfilled"
    );
    assert_eq!(
        output["data"]["post_trade_review"]["budget_drift_reason"],
        "executed_tranche_exceeded_account_budget"
    );
    assert_eq!(
        output["data"]["post_trade_review"]["next_account_adjustment_hint"],
        "下次同类机会先回到计划层数，未重新通过账户预算复核前不要继续追加强度。"
    );
}

#[test]
fn security_post_trade_review_marks_open_position_as_pending_closeout() {
    let runtime_db_path = create_test_runtime_db("security_post_trade_review_open_position");
    let server =
        prepare_security_environment(&runtime_db_path, "security_post_trade_review_open_position");
    let mut request = review_request();
    // 2026-04-10 CST: 这里补“未平仓 review”红测，原因是连续状态下复盘层至少要能识别当前仍在持仓，
    // 目的：锁住 post_trade_review 不再把未平仓快照误判成完整兑现后的正式结论。
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

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["execution_record"]["position_state"],
        json!("open")
    );
    assert_eq!(
        output["data"]["post_trade_review"]["review_status"],
        json!("open_position_pending")
    );
    assert_eq!(
        output["data"]["post_trade_review"]["execution_deviation"],
        json!("open_position_pending")
    );
    assert_eq!(
        output["data"]["post_trade_review"]["executed_return"],
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

fn review_request() -> serde_json::Value {
    json!({
        "tool": "security_post_trade_review",
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

fn portfolio_position_plan_document_fixture() -> serde_json::Value {
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
