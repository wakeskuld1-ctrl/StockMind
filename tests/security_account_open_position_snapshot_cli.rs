mod common;

use chrono::{Duration, NaiveDate};
use excel_skill::ops::stock::security_execution_record::SecurityExecutionRecordDocument;
use excel_skill::runtime::security_corporate_action_store::{
    SecurityCorporateActionRow, SecurityCorporateActionStore,
};
use excel_skill::runtime::security_execution_store::SecurityExecutionStore;
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

// 2026-04-16 CST: Added because P0-1 needs a regression test that proves the formal
// open-position snapshot now exposes dividend-adjusted holding economics.
// Purpose: keep the first corporate-action integration locked from the public CLI surface.
fn seed_corporate_action(runtime_db_path: &Path, row: SecurityCorporateActionRow) {
    let corporate_action_db_path = runtime_db_path
        .parent()
        .expect("runtime db path should have parent")
        .join("security_corporate_action.db");
    let store = SecurityCorporateActionStore::new(corporate_action_db_path);
    store
        .upsert_rows(&[row])
        .expect("corporate action rows should persist");
}

// 2026-04-16 CST: Added because the existing execution_record CLI fixture path currently fails
// on an unrelated committee payload prerequisite.
// Purpose: seed one formal execution record directly so this regression test only validates the
// P0-1 open-position snapshot runtime reconstruction boundary.
fn seed_execution_record(runtime_db_path: &Path, record: &SecurityExecutionRecordDocument) {
    let execution_db_path = runtime_db_path
        .parent()
        .expect("runtime db path should have parent")
        .join("security_execution.db");
    let store = SecurityExecutionStore::new(execution_db_path);
    store
        .upsert_execution_record(record)
        .expect("execution record should persist");
}

fn execution_record_fixture(
    symbol: &str,
    account_id: &str,
    sector_tag: &str,
    actual_entry_date: &str,
    actual_entry_price: f64,
    current_position_pct: f64,
    position_state: &str,
) -> SecurityExecutionRecordDocument {
    SecurityExecutionRecordDocument {
        execution_record_id: format!("record-{symbol}-{position_state}"),
        contract_version: "security_execution_record.v1".to_string(),
        document_type: "security_execution_record".to_string(),
        generated_at: "2026-04-16T11:00:00+08:00".to_string(),
        symbol: symbol.to_string(),
        analysis_date: actual_entry_date.to_string(),
        account_id: Some(account_id.to_string()),
        sector_tag: Some(sector_tag.to_string()),
        position_state: position_state.to_string(),
        portfolio_position_plan_ref: None,
        execution_journal_ref: format!("journal-{symbol}"),
        position_plan_ref: format!("plan-{symbol}"),
        snapshot_ref: format!("snapshot-{symbol}"),
        outcome_ref: format!("outcome-{symbol}"),
        planned_entry_date: actual_entry_date.to_string(),
        planned_entry_price: actual_entry_price,
        planned_position_pct: 0.06,
        planned_max_position_pct: 0.15,
        actual_entry_date: actual_entry_date.to_string(),
        actual_entry_price,
        actual_position_pct: current_position_pct,
        current_position_pct,
        actual_exit_date: if position_state == "open" {
            String::new()
        } else {
            "2025-09-30".to_string()
        },
        actual_exit_price: if position_state == "open" {
            0.0
        } else {
            actual_entry_price * 1.03
        },
        exit_reason: if position_state == "open" {
            "position_still_open".to_string()
        } else {
            "target_hit".to_string()
        },
        holding_days: 5,
        planned_forward_return: 0.08,
        actual_return: 0.0,
        entry_slippage_pct: 0.0,
        position_size_gap_pct: current_position_pct - 0.06,
        planned_tranche_action: None,
        planned_tranche_pct: None,
        planned_peak_position_pct: None,
        actual_tranche_action: None,
        actual_tranche_pct: None,
        actual_peak_position_pct: None,
        tranche_count_drift: None,
        account_budget_alignment: None,
        execution_return_gap: -0.08,
        execution_quality: if position_state == "open" {
            "open_position_pending".to_string()
        } else {
            "aligned".to_string()
        },
        price_as_of_date: None,
        resolved_trade_date: None,
        current_price: None,
        share_adjustment_factor: None,
        cumulative_cash_dividend_per_share: None,
        dividend_adjusted_cost_basis: None,
        holding_total_return_pct: None,
        breakeven_price: None,
        corporate_action_summary: None,
        execution_record_notes: vec!["fixture".to_string()],
        attribution_summary: "fixture".to_string(),
    }
}

// 2026-04-10 CST: 这里新增账户 open snapshot CLI 测试文件，原因是方案B要求把“上一轮 execution_record -> 下一轮账户输入”正式对象化；
// 目的：先从外层合同锁住 runtime 自动读取 open execution_record 的行为，再补实现，避免继续停留在手工传参。

fn create_stock_history_csv(prefix: &str, file_name: &str, rows: &[String]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_account_open_position_snapshot")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir)
        .expect("security account open position snapshot fixture dir should exist");

    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n"))
        .expect("security account open position snapshot csv should be written");
    csv_path
}

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
        for _ in 0..route_map.len() + 10 {
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

fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_account_open_position_snapshot_fixture"
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
fn tool_catalog_includes_security_account_open_position_snapshot() {
    let output = run_cli_with_json("");
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_account_open_position_snapshot"),
        "tool catalog should include security_account_open_position_snapshot"
    );
}

// 2026-04-18 CST: Added because Task 3 needs the existing snapshot tool to
// expose one explicit active-position-book layer for later monitoring tasks.
// Reason: the approved design separates the compatibility snapshot shell from
// the live active-position semantics that downstream evaluation will consume.
// Purpose: freeze the first public `ActivePositionBook` projection at the CLI surface.
#[test]
fn security_account_open_position_snapshot_exposes_active_position_book_for_monitoring() {
    let runtime_db_path = create_test_runtime_db("security_active_position_book_single");
    let server =
        prepare_security_environment(&runtime_db_path, "security_active_position_book_single");
    let execution_db_path = runtime_db_path
        .parent()
        .expect("runtime db path should have parent")
        .join("security_execution.db");
    let envs = security_envs_with_execution_db(&server, &execution_db_path);
    seed_execution_record(
        &runtime_db_path,
        &execution_record_fixture(
            "601916.SH",
            "acct-demo-active-001",
            "bank",
            "2025-09-18",
            62.40,
            0.12,
            "open",
        ),
    );
    seed_corporate_action(
        &runtime_db_path,
        SecurityCorporateActionRow {
            symbol: "601916.SH".to_string(),
            effective_date: "2025-09-22".to_string(),
            action_type: "split".to_string(),
            cash_dividend_per_share: 0.0,
            split_ratio: 1.5,
            bonus_ratio: 0.0,
            source: "security_active_position_book_single_fixture".to_string(),
            payload_json: "{\"split_ratio\":1.5}".to_string(),
        },
    );
    seed_corporate_action(
        &runtime_db_path,
        SecurityCorporateActionRow {
            symbol: "601916.SH".to_string(),
            effective_date: "2025-09-25".to_string(),
            action_type: "cash_dividend".to_string(),
            cash_dividend_per_share: 0.18,
            split_ratio: 1.0,
            bonus_ratio: 0.0,
            source: "security_active_position_book_single_fixture".to_string(),
            payload_json: "{\"cash_dividend_per_share\":0.18}".to_string(),
        },
    );

    let snapshot_request = json!({
        "tool": "security_account_open_position_snapshot",
        "args": {
            "account_id": "acct-demo-active-001",
            "created_at": "2026-04-16T09:35:00+08:00"
        }
    });
    let snapshot_output =
        run_cli_with_json_runtime_and_envs(&snapshot_request.to_string(), &runtime_db_path, &envs);

    assert_eq!(snapshot_output["status"], "ok");
    assert_eq!(
        snapshot_output["data"]["active_position_book"]["document_type"],
        json!("security_active_position_book")
    );
    assert_eq!(
        snapshot_output["data"]["active_position_book"]["active_position_count"],
        json!(1)
    );
    assert_eq!(
        snapshot_output["data"]["active_position_book"]["active_positions"][0]["symbol"],
        json!("601916.SH")
    );
    assert_eq!(
        snapshot_output["data"]["active_position_book"]["active_positions"][0]["current_weight_pct"],
        json!(0.12)
    );
    assert_eq!(
        snapshot_output["data"]["active_position_book"]["active_positions"][0]["source_execution_record_ref"],
        json!("record-601916.SH-open")
    );
    assert_eq!(
        snapshot_output["data"]["active_position_book"]["active_positions"][0]["resolved_trade_date"],
        json!("2026-02-24")
    );
    assert!(
        snapshot_output["data"]["active_position_book"]["active_positions"][0]["holding_total_return_pct"]
            .as_f64()
            .expect("holding total return should exist")
            > 0.0
    );
    assert!(
        snapshot_output["data"]["active_position_book"]["active_positions"][0]["breakeven_price"]
            .as_f64()
            .expect("breakeven price should exist")
            < 41.50
    );
}

// 2026-04-18 CST: Added because Task 3 also needs the active-position-book
// refresh path to stay stable when one account carries multiple live positions.
// Reason: later per-position evaluation and account aggregation will depend on
// deterministic symbol/ref coverage across repeated refreshes.
// Purpose: lock the multi-position active book semantics before Task 4 starts.
#[test]
fn security_account_open_position_snapshot_refreshes_active_position_book_for_multiple_open_positions()
 {
    let runtime_db_path = create_test_runtime_db("security_active_position_book_multi");
    let server =
        prepare_security_environment(&runtime_db_path, "security_active_position_book_multi");
    let execution_db_path = runtime_db_path
        .parent()
        .expect("runtime db path should have parent")
        .join("security_execution.db");
    let envs = security_envs_with_execution_db(&server, &execution_db_path);
    seed_execution_record(
        &runtime_db_path,
        &execution_record_fixture(
            "601916.SH",
            "acct-demo-active-002",
            "bank",
            "2025-09-18",
            62.40,
            0.12,
            "open",
        ),
    );
    seed_execution_record(
        &runtime_db_path,
        &execution_record_fixture(
            "600919.SH",
            "acct-demo-active-002",
            "bank",
            "2025-09-18",
            11.20,
            0.08,
            "open",
        ),
    );

    let snapshot_request = json!({
        "tool": "security_account_open_position_snapshot",
        "args": {
            "account_id": "acct-demo-active-002",
            "created_at": "2026-04-16T10:05:00+08:00"
        }
    });
    let snapshot_output =
        run_cli_with_json_runtime_and_envs(&snapshot_request.to_string(), &runtime_db_path, &envs);

    assert_eq!(snapshot_output["status"], "ok");
    assert_eq!(
        snapshot_output["data"]["active_position_book"]["active_position_count"],
        json!(2)
    );
    assert_eq!(
        snapshot_output["data"]["active_position_book"]["source_execution_record_refs"],
        json!(["record-600919.SH-open", "record-601916.SH-open"])
    );
    assert_eq!(
        snapshot_output["data"]["active_position_book"]["active_positions"][0]["symbol"],
        json!("600919.SH")
    );
    assert_eq!(
        snapshot_output["data"]["active_position_book"]["active_positions"][1]["symbol"],
        json!("601916.SH")
    );
    assert_eq!(
        snapshot_output["data"]["active_position_book"]["active_positions"][0]["current_weight_pct"],
        json!(0.08)
    );
    assert_eq!(
        snapshot_output["data"]["active_position_book"]["active_positions"][1]["current_weight_pct"],
        json!(0.12)
    );
}

// 2026-04-18 CST: Added because the active-position-book semantics must keep
// closed or fully reduced positions out of the monitoring surface.
// Reason: Task 4 and Task 5 should consume only live holdings instead of
// re-filtering zero-weight rows every time.
// Purpose: freeze the current-position-based inclusion rule at the CLI boundary.
#[test]
fn security_account_open_position_snapshot_excludes_closed_or_zero_weight_positions_from_active_book()
 {
    let runtime_db_path = create_test_runtime_db("security_active_position_book_filters_closed");
    let server = prepare_security_environment(
        &runtime_db_path,
        "security_active_position_book_filters_closed",
    );
    let execution_db_path = runtime_db_path
        .parent()
        .expect("runtime db path should have parent")
        .join("security_execution.db");
    let envs = security_envs_with_execution_db(&server, &execution_db_path);
    seed_execution_record(
        &runtime_db_path,
        &execution_record_fixture(
            "601916.SH",
            "acct-demo-active-003",
            "bank",
            "2025-09-18",
            62.40,
            0.12,
            "open",
        ),
    );
    seed_execution_record(
        &runtime_db_path,
        &execution_record_fixture(
            "600919.SH",
            "acct-demo-active-003",
            "bank",
            "2025-09-18",
            11.20,
            0.00,
            "closed",
        ),
    );

    let snapshot_request = json!({
        "tool": "security_account_open_position_snapshot",
        "args": {
            "account_id": "acct-demo-active-003",
            "created_at": "2026-04-16T10:15:00+08:00"
        }
    });
    let snapshot_output =
        run_cli_with_json_runtime_and_envs(&snapshot_request.to_string(), &runtime_db_path, &envs);

    assert_eq!(snapshot_output["status"], "ok");
    assert_eq!(
        snapshot_output["data"]["active_position_book"]["active_position_count"],
        json!(1)
    );
    assert_eq!(
        snapshot_output["data"]["active_position_book"]["active_positions"][0]["symbol"],
        json!("601916.SH")
    );
    assert_eq!(
        snapshot_output["data"]["active_position_book"]["source_execution_record_refs"],
        json!(["record-601916.SH-open"])
    );
}

// 2026-04-18 CST: Added because later monitoring aggregation must tolerate an
// account that currently has no live holdings.
// Reason: the pure-data loop should return one explicit empty active book
// instead of making downstream callers infer emptiness from errors.
// Purpose: lock the empty-account active-position-book contract before Task 4 starts.
#[test]
fn security_account_open_position_snapshot_returns_empty_active_book_for_account_without_open_positions()
 {
    let runtime_db_path = create_test_runtime_db("security_active_position_book_empty_account");
    let server = prepare_security_environment(
        &runtime_db_path,
        "security_active_position_book_empty_account",
    );
    let execution_db_path = runtime_db_path
        .parent()
        .expect("runtime db path should have parent")
        .join("security_execution.db");
    let envs = security_envs_with_execution_db(&server, &execution_db_path);

    let snapshot_request = json!({
        "tool": "security_account_open_position_snapshot",
        "args": {
            "account_id": "acct-demo-active-empty",
            "created_at": "2026-04-16T10:25:00+08:00"
        }
    });
    let snapshot_output =
        run_cli_with_json_runtime_and_envs(&snapshot_request.to_string(), &runtime_db_path, &envs);

    assert_eq!(snapshot_output["status"], "ok");
    assert_eq!(
        snapshot_output["data"]["active_position_book"]["account_id"],
        json!("acct-demo-active-empty")
    );
    assert_eq!(
        snapshot_output["data"]["active_position_book"]["active_position_count"],
        json!(0)
    );
    assert_eq!(
        snapshot_output["data"]["active_position_book"]["active_positions"],
        json!([])
    );
    assert_eq!(
        snapshot_output["data"]["active_position_book"]["source_execution_record_refs"],
        json!([])
    );
}

#[test]
fn security_account_open_position_snapshot_reads_runtime_and_feeds_portfolio_plan() {
    let runtime_db_path = create_test_runtime_db("security_account_open_position_snapshot_ready");
    let server = prepare_security_environment(
        &runtime_db_path,
        "security_account_open_position_snapshot_ready",
    );
    let execution_db_path = runtime_db_path
        .parent()
        .expect("runtime db path should have parent")
        .join("security_execution.db");
    let envs = security_envs_with_execution_db(&server, &execution_db_path);
    seed_execution_record(
        &runtime_db_path,
        &execution_record_fixture(
            "601916.SH",
            "acct-demo-001",
            "bank",
            "2025-09-18",
            62.40,
            0.12,
            "open",
        ),
    );
    seed_execution_record(
        &runtime_db_path,
        &execution_record_fixture(
            "600919.SH",
            "acct-demo-001",
            "bank",
            "2025-09-18",
            11.20,
            0.00,
            "closed",
        ),
    );
    seed_corporate_action(
        &runtime_db_path,
        SecurityCorporateActionRow {
            symbol: "601916.SH".to_string(),
            effective_date: "2025-09-22".to_string(),
            action_type: "split".to_string(),
            cash_dividend_per_share: 0.0,
            split_ratio: 1.5,
            bonus_ratio: 0.0,
            source: "security_account_open_position_snapshot_fixture".to_string(),
            payload_json: "{\"split_ratio\":1.5}".to_string(),
        },
    );
    seed_corporate_action(
        &runtime_db_path,
        SecurityCorporateActionRow {
            symbol: "601916.SH".to_string(),
            effective_date: "2025-09-25".to_string(),
            action_type: "cash_dividend".to_string(),
            cash_dividend_per_share: 0.12,
            split_ratio: 1.0,
            bonus_ratio: 0.0,
            source: "security_account_open_position_snapshot_fixture".to_string(),
            payload_json: "{}".to_string(),
        },
    );

    let snapshot_request = json!({
        "tool": "security_account_open_position_snapshot",
        "args": {
            "account_id": "acct-demo-001",
            "created_at": "2026-04-10T16:00:00+08:00"
        }
    });
    let snapshot_output =
        run_cli_with_json_runtime_and_envs(&snapshot_request.to_string(), &runtime_db_path, &envs);
    assert_eq!(
        snapshot_output["status"], "ok",
        "snapshot_output={snapshot_output}"
    );
    // 2026-04-10 CST: 这里锁 runtime 自动读取结果，原因是方案B的目标就是收掉手工传 open_position_snapshots；
    // 目的：确保系统只带回当前仍 open 的 execution_record，并能给账户计划直接消费。
    assert_eq!(
        snapshot_output["data"]["account_open_position_snapshot"]["account_id"],
        json!("acct-demo-001")
    );
    assert_eq!(
        snapshot_output["data"]["account_open_position_snapshot"]["open_position_snapshots"]
            .as_array()
            .expect("open position snapshots should be array")
            .len(),
        1
    );
    assert_eq!(
        snapshot_output["data"]["account_open_position_snapshot"]["open_position_snapshots"][0]["symbol"],
        json!("601916.SH")
    );
    assert_eq!(
        snapshot_output["data"]["account_open_position_snapshot"]["open_position_snapshots"][0]["current_position_pct"],
        json!(0.12)
    );
    assert_eq!(
        snapshot_output["data"]["account_open_position_snapshot"]["open_position_snapshots"][0]["cumulative_cash_dividend_per_share"],
        json!(0.18)
    );
    assert_eq!(
        snapshot_output["data"]["account_open_position_snapshot"]["open_position_snapshots"][0]["resolved_trade_date"],
        json!("2026-02-24")
    );
    assert_eq!(
        snapshot_output["data"]["account_open_position_snapshot"]["open_position_snapshots"][0]["share_adjustment_factor"],
        json!(1.5)
    );
    assert!(
        snapshot_output["data"]["account_open_position_snapshot"]["open_position_snapshots"][0]["breakeven_price"]
            .as_f64()
            .expect("breakeven price should exist")
            < 41.50
    );

    let portfolio_request = json!({
        "tool": "security_portfolio_position_plan",
        "args": {
            "account_id": "acct-demo-001",
            "total_equity": 100000.0,
            "available_cash": 30000.0,
            "min_cash_reserve_pct": 0.20,
            "max_single_position_pct": 0.20,
            "max_sector_exposure_pct": 0.35,
            "max_portfolio_risk_budget_pct": 0.05,
            "current_portfolio_risk_budget_pct": 0.02,
            "max_single_trade_risk_budget_pct": 0.02,
            "holdings": [],
            "account_open_position_snapshot_document": snapshot_output["data"]["account_open_position_snapshot"].clone(),
            "candidates": [portfolio_candidate_fixture()],
            "created_at": "2026-04-10T16:05:00+08:00"
        }
    });
    let portfolio_output = run_cli_with_json(&portfolio_request.to_string());
    assert_eq!(
        portfolio_output["status"], "ok",
        "portfolio_output={portfolio_output}"
    );
    assert_eq!(
        portfolio_output["data"]["portfolio_position_plan"]["allocations"][0]["current_position_pct"],
        json!(0.12)
    );
    assert_eq!(
        portfolio_output["data"]["portfolio_position_plan"]["allocations"][0]["recommended_trade_amount"],
        json!(3000.0)
    );
}

fn prepare_security_environment(runtime_db_path: &Path, prefix: &str) -> String {
    let stock_csv = create_stock_history_csv(prefix, "stock.csv", &build_review_rows(420, 12.0));
    let market_csv =
        create_stock_history_csv(prefix, "market.csv", &build_review_rows(420, 3200.0));
    let sector_csv = create_stock_history_csv(prefix, "sector.csv", &build_review_rows(420, 960.0));
    let stock_csv_other =
        create_stock_history_csv(prefix, "stock_other.csv", &build_review_rows(420, 11.5));
    import_history_csv(runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(runtime_db_path, &stock_csv_other, "600919.SH");
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

fn portfolio_candidate_fixture() -> Value {
    json!({
        "symbol": "601916.SH",
        "sector_tag": "bank",
        "position_plan_document": {
            "position_plan_id": "position-plan-601916.SH-2025-10-15",
            "contract_version": "security_position_plan.v1",
            "document_type": "security_position_plan",
            "generated_at": "2026-04-10T12:00:00+08:00",
            "symbol": "601916.SH",
            "analysis_date": "2025-10-15",
            "analysis_date_guard": {
                "requested_as_of_date": "2025-10-15",
                "effective_analysis_date": "2025-10-15",
                "effective_trade_date": "2025-10-15",
                "local_data_last_date": "2025-10-15",
                "data_freshness_status": "local_exact_requested_date",
                "sync_attempted": false,
                "sync_result": null,
                "date_fallback_reason": null
            },
            "evidence_version": "evidence-v1",
            "briefing_ref": "evidence-v1",
            "committee_payload_ref": "committee-payload:601916.SH:2025-10-15",
            "recommended_action": "buy",
            "confidence": "high",
            "odds_grade": "favorable",
            "historical_confidence": "high",
            "confidence_grade": "strong",
            "position_action": "build",
            "entry_mode": "breakout_confirmation",
            "starter_position_pct": 0.06,
            "max_position_pct": 0.15,
            "add_on_trigger": "volume_up",
            "reduce_on_trigger": "break_support",
            "hard_stop_trigger": "close_below_stop",
            "liquidity_cap": "单次执行不超过计划仓位的 30%",
            "position_risk_grade": "medium",
            "regime_adjustment": "normal",
            "execution_notes": ["只在确认后加仓"],
            "rationale": ["赔率较优"]
        }
    })
}

fn security_envs_with_execution_db(
    server: &str,
    execution_db_path: &Path,
) -> Vec<(&'static str, String)> {
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
        (
            "EXCEL_SKILL_SECURITY_EXECUTION_DB",
            execution_db_path.to_string_lossy().to_string(),
        ),
    ]
}
