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

use crate::common::{run_cli_with_json_and_runtime, run_cli_with_json_runtime_and_envs};

// 2026-04-12 CST: Add a dedicated validation-slice fixture helper, because P9/P10
// need one repeatable end-to-end lifecycle bundle that can later be copied into
// operator-facing runtime validation storage.
// Purpose: keep all generated validation artifacts in one deterministic test-owned directory.
fn create_validation_fixture_dir(prefix: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_lifecycle_validation")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir)
        .expect("security lifecycle validation fixture dir should exist");
    fixture_dir
}

// 2026-04-12 CST: Add a local runtime-db helper, because this validation test
// needs all approval and lifecycle artifacts to land beside one shared manifest.
// Purpose: let the end-to-end slice be copied later without reconstructing paths.
fn create_validation_runtime_db(prefix: &str) -> PathBuf {
    create_validation_fixture_dir(prefix).join("runtime.db")
}

// 2026-04-12 CST: Reuse a local stock-history CSV helper, because the validation
// slice must generate real approval/package artifacts instead of relying on prebuilt fixtures.
// Purpose: keep the end-to-end lifecycle test self-contained and replayable.
fn create_stock_history_csv(fixture_dir: &Path, file_name: &str, rows: &[String]) -> PathBuf {
    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n"))
        .expect("security lifecycle validation csv should be written");
    csv_path
}

// 2026-04-12 CST: Persist generated lifecycle objects as standalone JSON fixtures,
// because P9 requires copyable validation artifacts rather than in-memory-only tool results.
// Purpose: make the same files consumable by package revision and later operator validation.
fn create_json_fixture(file_path: &Path, value: &Value) {
    fs::write(
        file_path,
        serde_json::to_vec_pretty(value).expect("json fixture should serialize"),
    )
    .expect("security lifecycle validation json should be written");
}

// 2026-04-12 CST: Reuse the local mock HTTP server pattern, because approval
// submission still depends on financial and announcement endpoints.
// Purpose: keep the validation slice deterministic and independent from live network drift.
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

// 2026-04-12 CST: Keep history import local to the validation test, because the
// lifecycle replay path still starts from formal price history already loaded in runtime.
// Purpose: avoid depending on unrelated preloaded test runtimes.
fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_lifecycle_validation_fixture"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path.to_path_buf(),
        &[],
    );
    assert_eq!(output["status"], "ok");
}

// 2026-04-12 CST: Reuse the confirmed-breakout generator, because approval
// submission still needs a stable market/stock setup to pass committee gating.
// Purpose: give the validation slice realistic but deterministic history inputs.
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
        rows.push(format!(
            "{},{open:.2},{high:.2},{low:.2},{next_close:.2},{next_close:.2},{volume}",
            trade_date.format("%Y-%m-%d")
        ));
        close = next_close;
    }

    rows
}

#[test]
fn security_lifecycle_validation_slice_round_trips_formal_tools() {
    let runtime_db_path = create_validation_runtime_db("security_lifecycle_validation_slice");
    let fixture_dir = runtime_db_path
        .parent()
        .expect("runtime db should have a parent directory")
        .to_path_buf();
    let approval_root = fixture_dir.join("scenes_runtime");

    let stock_csv = create_stock_history_csv(
        &fixture_dir,
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        &fixture_dir,
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        &fixture_dir,
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
            "created_at": "2026-04-12T14:00:00+08:00"
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
    assert_eq!(submit_output["status"], "ok");

    let decision_ref = submit_output["data"]["decision_ref"]
        .as_str()
        .expect("decision ref should exist");
    let approval_ref = submit_output["data"]["approval_ref"]
        .as_str()
        .expect("approval ref should exist");
    let position_plan_ref = submit_output["data"]["position_plan"]["plan_id"]
        .as_str()
        .expect("position plan ref should exist");
    let package_path = submit_output["data"]["decision_package_path"]
        .as_str()
        .expect("decision package path should exist");

    let condition_review_request = json!({
        "tool": "security_condition_review",
        "args": {
            "symbol": "601916.SH",
            "analysis_date": "2026-04-12",
            "decision_ref": decision_ref,
            "approval_ref": approval_ref,
            "position_plan_ref": position_plan_ref,
            "decision_package_path": package_path,
            "review_trigger_type": "manual_review",
            "review_trigger_summary": "盘中人工复核，维持原计划",
            "created_at": "2026-04-12T14:05:00+08:00"
        }
    });
    let condition_review_output =
        run_cli_with_json_and_runtime(&condition_review_request.to_string(), &runtime_db_path);
    assert_eq!(condition_review_output["status"], "ok");
    let condition_review_document = condition_review_output["data"]["condition_review"].clone();
    let condition_review_ref = condition_review_document["condition_review_id"]
        .as_str()
        .expect("condition review ref should exist");
    let condition_review_path = fixture_dir.join("condition_review.json");
    create_json_fixture(&condition_review_path, &condition_review_document);

    let execution_record_request = json!({
        "tool": "security_execution_record",
        "args": {
            "symbol": "601916.SH",
            "analysis_date": "2026-04-12",
            "decision_ref": decision_ref,
            "approval_ref": approval_ref,
            "position_plan_ref": position_plan_ref,
            "condition_review_ref": condition_review_ref,
            "execution_action": "build",
            "execution_status": "filled",
            "executed_gross_pct": 0.06,
            "execution_summary": "按计划建立首仓",
            "created_at": "2026-04-12T14:15:00+08:00"
        }
    });
    let execution_record_output =
        run_cli_with_json_and_runtime(&execution_record_request.to_string(), &runtime_db_path);
    assert_eq!(execution_record_output["status"], "ok");
    let execution_record_document = execution_record_output["data"]["execution_record"].clone();
    let execution_record_ref = execution_record_document["execution_record_id"]
        .as_str()
        .expect("execution record ref should exist");
    let execution_record_path = fixture_dir.join("execution_record.json");
    create_json_fixture(&execution_record_path, &execution_record_document);

    let post_trade_review_request = json!({
        "tool": "security_post_trade_review",
        "args": {
            "symbol": "601916.SH",
            "analysis_date": "2026-04-12",
            "decision_ref": decision_ref,
            "approval_ref": approval_ref,
            "position_plan_ref": position_plan_ref,
            "execution_record_ref": execution_record_ref,
            "review_status": "completed",
            "review_summary": "执行后确认继续以 shadow 方式跟踪量化上下文",
            "attribution": {
                "data_issue": false,
                "model_issue": true,
                "governance_issue": true,
                "execution_issue": false
            },
            "recommended_governance_action": "continue_shadow",
            "created_at": "2026-04-12T14:30:00+08:00"
        }
    });
    let post_trade_review_output =
        run_cli_with_json_and_runtime(&post_trade_review_request.to_string(), &runtime_db_path);
    assert_eq!(post_trade_review_output["status"], "ok");
    let post_trade_review_document = post_trade_review_output["data"]["post_trade_review"].clone();
    let post_trade_review_path = fixture_dir.join("post_trade_review.json");
    create_json_fixture(&post_trade_review_path, &post_trade_review_document);

    let revision_request = json!({
        "tool": "security_decision_package_revision",
        "args": {
            "package_path": package_path,
            "revision_reason": "attach_lifecycle_validation_slice",
            "reverify_after_revision": false,
            "condition_review_path": condition_review_path.to_string_lossy(),
            "execution_record_path": execution_record_path.to_string_lossy(),
            "post_trade_review_path": post_trade_review_path.to_string_lossy()
        }
    });
    let revision_output =
        run_cli_with_json_runtime_and_envs(&revision_request.to_string(), &runtime_db_path, &[]);

    // 2026-04-12 CST: Lock a full validation slice that uses the formal lifecycle
    // tools instead of hand-authored JSON, because P9/P10 need one replayable
    // end-to-end sample before operator data backfill begins.
    // Purpose: prove approval, condition review, execution, post-trade review, and package revision round-trip under one governed fixture bundle.
    assert_eq!(revision_output["status"], "ok");
    assert_eq!(
        revision_output["data"]["decision_package"]["object_graph"]["condition_review_ref"],
        condition_review_ref
    );
    assert_eq!(
        revision_output["data"]["decision_package"]["object_graph"]["execution_record_ref"],
        execution_record_ref
    );
    assert_eq!(
        revision_output["data"]["decision_package"]["lifecycle_governance_summary"]["recommended_governance_action"],
        "continue_shadow"
    );
    assert!(
        revision_output["data"]["decision_package"]["artifact_manifest"]
            .as_array()
            .expect("artifact manifest should be array")
            .iter()
            .any(|artifact| artifact["artifact_role"] == "post_trade_review")
    );

    let manifest_path = fixture_dir.join("validation_slice_manifest.json");
    create_json_fixture(
        &manifest_path,
        &json!({
            "document_type": "security_lifecycle_validation_slice",
            "symbol": "601916.SH",
            "analysis_date": "2026-04-12",
            "decision_ref": decision_ref,
            "approval_ref": approval_ref,
            "position_plan_ref": position_plan_ref,
            "condition_review_ref": condition_review_ref,
            "execution_record_ref": execution_record_ref,
            "post_trade_review_ref": post_trade_review_document["post_trade_review_id"],
            "runtime_root": approval_root.to_string_lossy(),
            "condition_review_path": condition_review_path.to_string_lossy(),
            "execution_record_path": execution_record_path.to_string_lossy(),
            "post_trade_review_path": post_trade_review_path.to_string_lossy(),
            "decision_package_path": revision_output["data"]["decision_package_path"]
        }),
    );
    assert!(manifest_path.exists());
}
