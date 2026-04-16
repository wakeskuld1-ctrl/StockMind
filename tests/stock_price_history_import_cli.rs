mod common;

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::HashMap, fmt::Write as _};

use rusqlite::Connection;
use serde_json::json;

use crate::common::{
    create_test_runtime_db, run_cli_with_json, run_cli_with_json_and_runtime,
    run_cli_with_json_runtime_and_envs,
};

// 2026-03-28 CST: 这里新增股票历史 CSV 测试文件生成助手，原因是第一刀要先锁住“命令行 EXE + CSV -> SQLite”的真实入口；
// 目的：让红绿测试都围绕同一个外部导入合同展开，避免实现先行后再补测试导致合同漂移。
fn create_stock_history_csv(prefix: &str, file_name: &str, rows: &[&str]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("stock_history_csv")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("csv fixture directory should be created");

    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n")).expect("csv fixture should be written");
    csv_path
}

// 2026-03-28 CST: 这里补一个测试期望数据库路径助手，原因是股票历史库与 session runtime 共享同一 runtime 根目录更利于后续 Skill / Tool 串联；
// 目的：让测试能直接验证 SQLite 落盘结果，而不是只看 stdout JSON。
fn stock_history_db_path(runtime_db_path: &Path) -> PathBuf {
    runtime_db_path
        .parent()
        .expect("runtime db should always have parent directory")
        .join("stock_history.db")
}

// 2026-03-29 CST: 这里补最小本地 HTTP 假服务，原因是股票历史 HTTP 同步红测需要稳定重放腾讯/新浪返回体；
// 目的：把 provider 合同锁在测试内，不依赖真实第三方接口的当天状态。
fn spawn_http_server(status_line: &str, body: &str, content_type: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test http server should bind");
    let address = format!(
        "http://{}",
        listener
            .local_addr()
            .expect("test http server should have local addr")
    );
    let status = status_line.to_string();
    let response_body = body.to_string();
    let response_content_type = content_type.to_string();

    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buffer = [0_u8; 2048];
            let _ = stream.read(&mut buffer);
            let response = format!(
                "{status}\r\nContent-Type: {response_content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });

    address
}

// 2026-04-15 CST: Added because cross-border ETF data completion needs one test
// server that can return multiple FRED CSV payloads in a single run.
// Reason: JPYCNY.FX is derived from two source series rather than one endpoint.
// Purpose: keep the new red tests deterministic without touching the real network.
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
            let mut buffer = [0_u8; 2048];
            let _ = stream.read(&mut buffer);
            let request_text = String::from_utf8_lossy(&buffer);
            let request_path = request_text
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/")
                .split('?')
                .next()
                .unwrap_or("/");
            let (status, response_body, response_content_type) =
                route_map.get(request_path).cloned().unwrap_or_else(|| {
                    (
                        "HTTP/1.1 404 Not Found".to_string(),
                        "not found".to_string(),
                        "text/plain".to_string(),
                    )
                });
            let mut response = String::new();
            let _ = write!(
                &mut response,
                "{status}\r\nContent-Type: {response_content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });

    address
}

#[test]
fn tool_catalog_includes_import_stock_price_history() {
    let output = run_cli_with_json("");

    // 2026-03-28 CST: 这里先锁目录可发现性，原因是导入能力如果不进入 catalog，后续 Skill 和外部 EXE 调用都无法稳定发现；
    // 目的：防止只实现底层导入逻辑，却遗漏 catalog/dispatcher 暴露导致能力实际上不可用。
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "import_stock_price_history")
    );
}

#[test]
fn import_stock_price_history_imports_csv_into_sqlite() {
    let runtime_db_path = create_test_runtime_db("stock_price_history_import_ok");
    let csv_path = create_stock_history_csv(
        "stock_price_history_import_ok",
        "prices.csv",
        &[
            "trade_date,open,high,low,close,adj_close,volume",
            "2026-03-25,1500.10,1512.80,1496.20,1508.00,1508.00,2300000",
            "2026-03-26,1508.00,1520.00,1501.00,1516.80,1516.80,2500000",
            "2026-03-27,1516.80,1530.50,1510.00,1528.90,1528.90,2700000",
        ],
    );
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": "600519.SH",
            "source": "manual_csv"
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-03-28 CST: 这里先锁第一版成功导入合同，原因是后续技术面 Tool 会直接依赖这条历史数据主线；
    // 目的：确保导入后既有标准 JSON 回执，也真实写入 SQLite，而不是停留在内存计算或临时文件。
    assert_eq!(output["status"], "ok");
    assert_eq!(output["data"]["symbol"], "600519.SH");
    assert_eq!(output["data"]["imported_row_count"], 3);
    assert_eq!(output["data"]["date_range"]["start_date"], "2026-03-25");
    assert_eq!(output["data"]["date_range"]["end_date"], "2026-03-27");

    let database_path = stock_history_db_path(&runtime_db_path);
    let connection = Connection::open(database_path).expect("stock history db should exist");
    let row_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM stock_price_history WHERE symbol = '600519.SH'",
            [],
            |row| row.get(0),
        )
        .expect("row count query should succeed");
    let latest_close: f64 = connection
        .query_row(
            "SELECT close FROM stock_price_history WHERE symbol = '600519.SH' AND trade_date = '2026-03-27'",
            [],
            |row| row.get(0),
        )
        .expect("latest close query should succeed");

    assert_eq!(row_count, 3);
    assert!((latest_close - 1528.90).abs() < 0.0001);
}

#[test]
fn import_stock_price_history_defaults_adj_close_to_close_when_missing() {
    let runtime_db_path = create_test_runtime_db("stock_price_history_import_missing_adj_close");
    let csv_path = create_stock_history_csv(
        "stock_price_history_import_missing_adj_close",
        "prices_missing_adj_close.csv",
        &[
            "trade_date,open,high,low,close,volume",
            "2026-03-27,1516.80,1530.50,1510.00,1528.90,2700000",
        ],
    );
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": "600519.SH",
            "source": "manual_csv"
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-12 CST: Lock the missing-adj-close fallback, because validation
    // slices and lightweight fixtures often only provide close without a second
    // adjusted-close column.
    // Purpose: require the importer to reuse close as adj_close when the column is absent.
    assert_eq!(output["status"], "ok");
    assert_eq!(output["data"]["imported_row_count"], 1);

    let database_path = stock_history_db_path(&runtime_db_path);
    let connection = Connection::open(database_path).expect("stock history db should exist");
    let adj_close: f64 = connection
        .query_row(
            "SELECT adj_close FROM stock_price_history WHERE symbol = '600519.SH' AND trade_date = '2026-03-27'",
            [],
            |row| row.get(0),
        )
        .expect("adj close query should succeed");

    assert!((adj_close - 1528.90).abs() < 0.0001);
}

#[test]
fn import_stock_price_history_replaces_existing_symbol_trade_date_rows() {
    let runtime_db_path = create_test_runtime_db("stock_price_history_import_upsert");
    let first_csv_path = create_stock_history_csv(
        "stock_price_history_import_upsert_first",
        "prices_first.csv",
        &[
            "trade_date,open,high,low,close,adj_close,volume",
            "2026-03-27,1516.80,1530.50,1510.00,1528.90,1528.90,2700000",
        ],
    );
    let second_csv_path = create_stock_history_csv(
        "stock_price_history_import_upsert_second",
        "prices_second.csv",
        &[
            "trade_date,open,high,low,close,adj_close,volume",
            "2026-03-27,1516.80,1530.50,1510.00,1533.30,1533.30,3100000",
        ],
    );

    let first_request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": first_csv_path.to_string_lossy(),
            "symbol": "600519.SH",
            "source": "manual_csv"
        }
    });
    let second_request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": second_csv_path.to_string_lossy(),
            "symbol": "600519.SH",
            "source": "manual_csv"
        }
    });

    run_cli_with_json_and_runtime(&first_request.to_string(), &runtime_db_path);
    let output = run_cli_with_json_and_runtime(&second_request.to_string(), &runtime_db_path);

    // 2026-03-28 CST: 这里先锁“同一 symbol + trade_date 覆盖更新”的规则，原因是历史行情重导入和补数是高频场景；
    // 目的：避免后续 SQLite 层产生重复交易日记录，影响技术指标和咨询结论。
    assert_eq!(output["status"], "ok");
    assert_eq!(output["data"]["imported_row_count"], 1);

    let database_path = stock_history_db_path(&runtime_db_path);
    let connection = Connection::open(database_path).expect("stock history db should exist");
    let row_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM stock_price_history WHERE symbol = '600519.SH' AND trade_date = '2026-03-27'",
            [],
            |row| row.get(0),
        )
        .expect("duplicate count query should succeed");
    let close_price: f64 = connection
        .query_row(
            "SELECT close FROM stock_price_history WHERE symbol = '600519.SH' AND trade_date = '2026-03-27'",
            [],
            |row| row.get(0),
        )
        .expect("close query should succeed");
    let volume: i64 = connection
        .query_row(
            "SELECT volume FROM stock_price_history WHERE symbol = '600519.SH' AND trade_date = '2026-03-27'",
            [],
            |row| row.get(0),
        )
        .expect("volume query should succeed");

    assert_eq!(row_count, 1);
    assert!((close_price - 1533.30).abs() < 0.0001);
    assert_eq!(volume, 3100000);
}

#[test]
fn import_stock_price_history_rejects_csv_missing_required_columns() {
    let runtime_db_path = create_test_runtime_db("stock_price_history_import_missing_columns");
    let csv_path = create_stock_history_csv(
        "stock_price_history_import_missing_columns",
        "missing_columns.csv",
        &[
            "trade_date,open,high,low,close,adj_close",
            "2026-03-27,1516.80,1530.50,1510.00,1528.90,1528.90",
        ],
    );
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": "600519.SH",
            "source": "manual_csv"
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-03-28 CST: 这里先锁缺列报错合同，原因是 CSV 输入最容易出问题的就是列头不齐或口径不对；
    // 目的：让后续 Skill 或人工导入时能拿到明确错误，而不是悄悄导入脏数据。
    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error should exist")
            .contains("volume")
    );
}

#[test]
fn tool_catalog_includes_sync_stock_price_history() {
    let output = run_cli_with_json("");

    // 2026-03-29 CST: 这里先锁 HTTP 股票同步 Tool 的可发现性，原因是如果没进 catalog，后续 CLI / Skill 根本无法稳定调用；
    // 目的：先把“能力可发现”钉成合同，再补具体 provider 行为。
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "sync_stock_price_history")
    );
}

#[test]
fn sync_stock_price_history_imports_tencent_daily_history_into_sqlite() {
    let runtime_db_path = create_test_runtime_db("sync_stock_price_history_tencent_ok");
    let tencent_url = spawn_http_server(
        "HTTP/1.1 200 OK",
        r#"{"code":0,"msg":"","data":{"sh600519":{"qfqday":[["2026-03-25","1410.110","1410.270","1417.870","1401.010","2609346.000"],["2026-03-26","1409.000","1401.180","1413.900","1400.300","2309289.000"],["2026-03-27","1400.000","1416.020","1426.000","1396.660","3008700.000"]]}}}"#,
        "application/json",
    );
    let request = json!({
        "tool": "sync_stock_price_history",
        "args": {
            "symbol": "600519.SH",
            "start_date": "2026-03-25",
            "end_date": "2026-03-27",
            "adjustment": "qfq",
            "providers": ["tencent", "sina"]
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &[
            ("EXCEL_SKILL_TENCENT_KLINE_URL", tencent_url),
            (
                "EXCEL_SKILL_SINA_KLINE_URL",
                "http://127.0.0.1:9".to_string(),
            ),
        ],
    );

    // 2026-03-29 CST: 这里先锁腾讯主路径成功合同，原因是方案 2+3 已确认腾讯应作为第一优先 provider；
    // 目的：确保 HTTP 同步不仅返回 JSON 成功，还真实写入现有 stock_history SQLite 主表。
    assert_eq!(output["status"], "ok");
    assert_eq!(output["data"]["symbol"], "600519.SH");
    assert_eq!(output["data"]["provider_used"], "tencent");
    assert_eq!(output["data"]["imported_row_count"], 3);

    let database_path = stock_history_db_path(&runtime_db_path);
    let connection = Connection::open(database_path).expect("stock history db should exist");
    let row_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM stock_price_history WHERE symbol = '600519.SH'",
            [],
            |row| row.get(0),
        )
        .expect("row count query should succeed");
    assert_eq!(row_count, 3);
}

#[test]
fn sync_stock_price_history_falls_back_to_sina_when_tencent_fails() {
    let runtime_db_path = create_test_runtime_db("sync_stock_price_history_sina_fallback");
    let tencent_url = spawn_http_server(
        "HTTP/1.1 500 Internal Server Error",
        r#"{"code":1,"msg":"upstream failed"}"#,
        "application/json",
    );
    let sina_url = spawn_http_server(
        "HTTP/1.1 200 OK",
        r#"[{"day":"2026-03-25","open":"1410.110","high":"1417.870","low":"1401.010","close":"1410.270","volume":"2609346"},{"day":"2026-03-26","open":"1409.000","high":"1413.900","low":"1400.300","close":"1401.180","volume":"2309289"},{"day":"2026-03-27","open":"1400.000","high":"1426.000","low":"1396.660","close":"1416.020","volume":"3008700"}]"#,
        "application/json",
    );
    let request = json!({
        "tool": "sync_stock_price_history",
        "args": {
            "symbol": "600519.SH",
            "start_date": "2026-03-25",
            "end_date": "2026-03-27",
            "adjustment": "qfq",
            "providers": ["tencent", "sina"]
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &[
            ("EXCEL_SKILL_TENCENT_KLINE_URL", tencent_url),
            ("EXCEL_SKILL_SINA_KLINE_URL", sina_url),
        ],
    );

    // 2026-03-29 CST: 这里先锁新浪降级合同，原因是用户已经明确要求腾讯 + 新浪双源而不是只做单源；
    // 目的：确保腾讯失败时，系统会继续尝试新浪而不是直接中断整条同步链路。
    assert_eq!(output["status"], "ok");
    assert_eq!(output["data"]["provider_used"], "sina");
    assert_eq!(output["data"]["imported_row_count"], 3);
}

#[test]
fn sync_stock_price_history_reports_error_when_all_providers_fail() {
    let runtime_db_path = create_test_runtime_db("sync_stock_price_history_all_fail");
    let tencent_url = spawn_http_server(
        "HTTP/1.1 500 Internal Server Error",
        r#"{"code":1,"msg":"upstream failed"}"#,
        "application/json",
    );
    let sina_url = spawn_http_server(
        "HTTP/1.1 500 Internal Server Error",
        r#"{"error":"forbidden"}"#,
        "application/json",
    );
    let request = json!({
        "tool": "sync_stock_price_history",
        "args": {
            "symbol": "600519.SH",
            "start_date": "2026-03-25",
            "end_date": "2026-03-27",
            "adjustment": "qfq",
            "providers": ["tencent", "sina"]
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &[
            ("EXCEL_SKILL_TENCENT_KLINE_URL", tencent_url),
            ("EXCEL_SKILL_SINA_KLINE_URL", sina_url),
        ],
    );

    // 2026-03-29 CST: 这里先锁双源都失败时的错误合同，原因是老接口可用性本身就不稳定；
    // 目的：确保调用方拿到明确中文错误，而不是误以为已经写入 SQLite。
    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error should exist")
            .contains("provider")
    );
}

#[test]
fn sync_stock_price_history_imports_fred_index_history_into_sqlite() {
    let runtime_db_path = create_test_runtime_db("sync_stock_price_history_fred_index");
    let server = spawn_http_route_server(vec![(
        "/fred",
        "HTTP/1.1 200 OK",
        "DATE,VALUE\n2026-04-13,57541.23\n2026-04-14,57877.39\n",
        "text/csv",
    )]);
    let request = json!({
        "tool": "sync_stock_price_history",
        "args": {
            "symbol": "NK225.IDX",
            "start_date": "2026-04-13",
            "end_date": "2026-04-14",
            "adjustment": "qfq",
            "providers": ["fred"]
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &[("EXCEL_SKILL_FRED_CSV_URL_BASE", format!("{server}/fred"))],
    );

    // 2026-04-15 CST: Added because scheme B-2 must formally let non-A-share
    // penetrated index history enter the official stock history store.
    // Purpose: prove NK225.IDX can be synced without inventing a second ETF-only store.
    assert_eq!(output["status"], "ok", "fred output: {output}");
    assert_eq!(output["data"]["provider_used"], "fred");
    assert_eq!(output["data"]["imported_row_count"], 2);

    let database_path = stock_history_db_path(&runtime_db_path);
    let connection = Connection::open(database_path).expect("stock history db should exist");
    let latest_close: f64 = connection
        .query_row(
            "SELECT close FROM stock_price_history WHERE symbol = 'NK225.IDX' AND trade_date = '2026-04-14'",
            [],
            |row| row.get(0),
        )
        .expect("latest close query should succeed");
    assert!((latest_close - 57877.39).abs() < 0.0001);
}

#[test]
fn sync_stock_price_history_derives_jpycny_history_from_fred_cross_rates() {
    let runtime_db_path = create_test_runtime_db("sync_stock_price_history_jpycny");
    let server = spawn_http_route_server(vec![
        (
            "/fred",
            "HTTP/1.1 200 OK",
            "DATE,VALUE\n2026-04-10,159.22\n",
            "text/csv",
        ),
        (
            "/fred-cny",
            "HTTP/1.1 200 OK",
            "DATE,VALUE\n2026-04-10,6.8278\n",
            "text/csv",
        ),
    ]);
    let request = json!({
        "tool": "sync_stock_price_history",
        "args": {
            "symbol": "JPYCNY.FX",
            "start_date": "2026-04-10",
            "end_date": "2026-04-10",
            "adjustment": "qfq",
            "providers": ["fred"]
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &[
            ("EXCEL_SKILL_FRED_CSV_URL_BASE", format!("{server}/fred")),
            (
                "EXCEL_SKILL_FRED_DERIVED_DEXCHUS_URL_BASE",
                format!("{server}/fred-cny"),
            ),
        ],
    );

    // 2026-04-15 CST: Added because the approved cross-border ETF chain must
    // support a CNY execution-layer FX series instead of stopping at USDJPY.FX.
    // Purpose: lock the derived JPYCNY.FX write path on the formal sync tool.
    assert_eq!(output["status"], "ok", "derived fx output: {output}");
    assert_eq!(output["data"]["provider_used"], "fred");
    assert_eq!(output["data"]["imported_row_count"], 1);

    let database_path = stock_history_db_path(&runtime_db_path);
    let connection = Connection::open(database_path).expect("stock history db should exist");
    let latest_close: f64 = connection
        .query_row(
            "SELECT close FROM stock_price_history WHERE symbol = 'JPYCNY.FX' AND trade_date = '2026-04-10'",
            [],
            |row| row.get(0),
        )
        .expect("latest close query should succeed");
    let expected = 6.8278_f64 / 159.22_f64;
    assert!((latest_close - expected).abs() < 0.0000001);
}
