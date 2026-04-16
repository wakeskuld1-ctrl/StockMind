mod common;

use serde_json::{Value, json};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::run_cli_with_json_runtime_and_envs;

fn create_test_approval_root(prefix: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let root = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_scorecard")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&root).expect("security scorecard approval root should exist");
    root
}

fn live_601916_runtime_db() -> PathBuf {
    PathBuf::from("tests/runtime_fixtures/local_memory/live_601916_20260408/stock_history.db")
}

fn spawn_http_route_server(routes: Vec<(&str, &str, &str, &str)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test http server should bind");
    let address = format!(
        "http://{}",
        listener
            .local_addr()
            .expect("test http server should have local addr")
    );
    let route_map = routes
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
        .collect::<std::collections::HashMap<String, (String, String, String)>>();

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
fn submit_approval_persists_formal_scorecard_even_without_model_artifact() {
    // 2026-04-09 CST: 这里先锁定评分卡正式对象合同红测，原因是用户明确要求评分卡不能再用手工主观分冒充正式结果；
    // 目的：要求主链即便拿不到训练模型，也必须落一份正式 scorecard 对象，并显式声明 model_unavailable，而不是继续沉默退化。
    let runtime_db_path = live_601916_runtime_db();
    let approval_root = create_test_approval_root("submit_approval_scorecard");
    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "<html><body>financials HTTP 406</body></html>",
            "text/html",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{
                "data":{
                    "list":[
                        {"notice_date":"2026-03-31","title":"浙商银行:浙商银行股份有限公司关于诉讼事项的进展公告","art_code":"AN202603301820871983","columns":[{"column_name":"诉讼仲裁"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_decision_submit_approval",
        "args": {
            "symbol": "601916.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "as_of_date": "2026-04-08",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12,
            "approval_runtime_root": approval_root.to_string_lossy(),
            "created_at": "2026-04-09T09:30:00+08:00"
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

    // 2026-04-09 CST: 这里先把评分卡最小合同钉死，原因是本轮先做“独立正式对象 + package 接线”，
    // 不是先去伪造一组主观分；目的：要求 scorecard 至少具备状态、原始特征快照、限制说明和 package 锚点。
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["scorecard"]["score_status"],
        "model_unavailable"
    );
    assert!(output["data"]["scorecard"]["raw_feature_snapshot"].is_object());
    assert!(output["data"]["scorecard"]["limitations"].is_array());
    assert!(
        output["data"]["scorecard_path"]
            .as_str()
            .expect("scorecard path should exist")
            .contains("scorecards")
    );
    assert_eq!(
        output["data"]["decision_package"]["object_graph"]["scorecard_path"],
        output["data"]["scorecard_path"]
    );
    assert!(
        output["data"]["decision_package"]["artifact_manifest"]
            .as_array()
            .expect("artifact manifest should be array")
            .iter()
            .any(|artifact| artifact["artifact_role"] == "security_scorecard")
    );

    let scorecard_path = PathBuf::from(
        output["data"]["scorecard_path"]
            .as_str()
            .expect("scorecard path should exist"),
    );
    let persisted_scorecard: Value = serde_json::from_slice(
        &fs::read(&scorecard_path).expect("persisted scorecard should be readable"),
    )
    .expect("persisted scorecard should be valid json");
    assert_eq!(persisted_scorecard["score_status"], "model_unavailable");
    assert!(persisted_scorecard["raw_feature_snapshot"].is_object());
}
