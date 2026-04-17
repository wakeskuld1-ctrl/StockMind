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

// 2026-04-01 CST: 这里新增证券投决证据包 CLI 测试夹具，原因是方案 B 的第一步必须先把“研究输出能否冻结成统一证据包”锁成正式合同；
// 目的：确保后续无论 Skill 如何组织正反方辩论，读取到的都是同一份可审计、可回放的结构化证据，而不是临时拼接的自由文本。
fn create_stock_history_csv(prefix: &str, file_name: &str, rows: &[String]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_decision_evidence_bundle")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("security decision evidence fixture dir should exist");

    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n"))
        .expect("security decision evidence csv should be written");
    csv_path
}

// 2026-04-01 CST: 这里复用本地 HTTP 假服务，原因是证据包需要覆盖“外部基本面/公告可用”和“降级缺失”两种现实路径；
// 目的：让测试继续走真实 CLI 主链，同时稳定重放不同信息源状态，避免被外部网站波动影响回归结果。
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
fn tool_catalog_includes_security_decision_evidence_bundle() {
    let output = run_cli_with_json("");

    // 2026-04-01 CST: 这里先锁工具目录可发现性，原因是新 Tool 如果没进 catalog，Skill 和 EXE 都无法稳定调起；
    // 目的：避免只实现了业务逻辑，却遗漏 dispatcher/catalog 暴露，导致证券投决链“代码存在但产品不可用”。
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_decision_evidence_bundle")
    );
}

#[test]
fn security_decision_evidence_bundle_reports_analysis_date_and_data_gaps() {
    let runtime_db_path = create_test_runtime_db("security_decision_evidence_bundle_degraded");

    let stock_csv = create_stock_history_csv(
        "security_decision_evidence_bundle_degraded",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_decision_evidence_bundle_degraded",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_decision_evidence_bundle_degraded",
        "sector.csv",
        &build_choppy_history_rows(220),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 500 Internal Server Error",
            r#"{"error":"financial upstream failed"}"#,
            "application/json",
        ),
        (
            "/announcements",
            "HTTP/1.1 500 Internal Server Error",
            r#"{"error":"announcement upstream failed"}"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_decision_evidence_bundle",
        "args": {
            "symbol": "601916.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            // 2026-04-17 CST: Pin the evidence-bundle fixture to the local
            // history horizon, because this test validates degraded evidence
            // semantics rather than the "today + sync attempt" date-guard
            // path.
            // Purpose: keep the analysis_date assertion deterministic and stop
            // current-day sync behavior from drifting this regression case.
            "as_of_date": "2025-08-08"
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
                "EXCEL_SKILL_OFFICIAL_FINANCIAL_URL_BASE",
                format!("{server}/financials"),
            ),
            (
                "EXCEL_SKILL_SINA_FINANCIAL_URL_BASE",
                format!("{server}/financials"),
            ),
            (
                "EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE",
                format!("{server}/announcements"),
            ),
            (
                "EXCEL_SKILL_OFFICIAL_ANNOUNCEMENT_URL_BASE",
                format!("{server}/announcements"),
            ),
            (
                "EXCEL_SKILL_SINA_ANNOUNCEMENT_URL_BASE",
                format!("{server}/announcements"),
            ),
        ],
    );

    // 2026-04-01 CST: 这里锁“证据冻结 + 缺口显式披露”主路径，原因是投决会不能在信息缺失时继续假装自己拥有完整事实；
    // 目的：保证 analysis_date、evidence_hash、data_gaps、overall_status 这些投决层关键字段从第一版开始就是稳定合同。
    assert_eq!(output["status"], "ok");
    assert_eq!(output["data"]["symbol"], "601916.SH");
    assert_eq!(output["data"]["analysis_date"], "2025-08-08");
    assert_eq!(
        output["data"]["technical_context"]["contextual_conclusion"]["alignment"],
        "mixed"
    );
    assert_eq!(
        output["data"]["evidence_quality"]["overall_status"],
        "degraded"
    );
    assert_eq!(
        output["data"]["evidence_quality"]["fundamental_status"],
        "unavailable"
    );
    assert_eq!(
        output["data"]["evidence_quality"]["disclosure_status"],
        "unavailable"
    );
    assert!(
        output["data"]["data_gaps"]
            .as_array()
            .expect("data gaps should be array")
            .len()
            >= 2
    );
    assert!(
        output["data"]["evidence_hash"]
            .as_str()
            .expect("evidence hash should exist")
            .starts_with("sec-")
    );
}

// 2026-04-01 CST: 这里复用股票历史导入助手，原因是证据包必须建立在真实 stock_history_store 主链之上；
// 目的：避免测试直接伪造 fullstack 输出，导致后面接入投决会时无法覆盖真实运行路径。
fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_decision_evidence_bundle_fixture"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path.to_path_buf(),
        &[],
    );
    assert_eq!(output["status"], "ok");
}

// 2026-04-01 CST: 这里构造稳定上行并带末段突破的样本，原因是证据包测试需要覆盖“个股技术面明确偏强”的基础研究场景；
// 目的：让投决层后续判断关注点集中在信息缺口和风控，而不是被底层行情噪音干扰。
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

// 2026-04-01 CST: 这里补一个横盘等待样本，原因是证据包需要验证“行业或板块中性”时也能稳定产出分析日期和质量状态；
// 目的：避免所有测试都只覆盖强单边行情，后续在中性环境下出现合同缺口。
fn build_choppy_history_rows(day_count: usize) -> Vec<String> {
    let mut rows = vec!["trade_date,open,high,low,close,adj_close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("seed date should be valid");

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let wave = match offset % 6 {
            0 => -0.8,
            1 => 0.7,
            2 => -0.6,
            3 => 0.6,
            4 => -0.7,
            _ => 0.8,
        };
        let base = 100.0 + wave;
        let open = base - 0.15;
        let high = base + 0.85;
        let low = base - 0.85;
        let close = base + 0.1;
        let adj_close = close;
        let volume = 900_000 + (offset % 5) as i64 * 80_000;
        rows.push(format!(
            "{},{open:.2},{high:.2},{low:.2},{close:.2},{adj_close:.2},{volume}",
            trade_date.format("%Y-%m-%d")
        ));
    }

    rows
}
