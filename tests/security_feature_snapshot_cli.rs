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

// 2026-04-09 CST: 这里新增特征快照 CLI 测试夹具，原因是 Task 2 要先把“分析时点可见特征冻结”变成正式对象，
// 目的：锁住 feature_snapshot 的最小正式契约，再去补最小实现，避免后续继续临场读取漂移数据。
fn create_stock_history_csv(prefix: &str, file_name: &str, rows: &[String]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_feature_snapshot")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("security feature snapshot fixture dir should exist");

    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n")).expect("security feature snapshot csv should be written");
    csv_path
}

// 2026-04-09 CST: 这里复用本地 HTTP 假服务，原因是特征快照仍需要在稳定财报/公告上下文下冻结当时可见信息；
// 目的：让测试聚焦在 snapshot contract，而不是受外部数据接口抖动影响。
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
fn tool_catalog_includes_security_feature_snapshot() {
    let output = run_cli_with_json("");

    // 2026-04-09 CST: 这里先锁特征快照 Tool 的可发现性，原因是如果 catalog 里没有它，
    // 那么后续训练/回算链就没有正式入口；目的：确保 feature_snapshot 是一等能力，而不是内部临时辅助函数。
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_feature_snapshot")
    );
}

#[test]
fn security_feature_snapshot_freezes_raw_and_group_features_with_hash() {
    let runtime_db_path = create_test_runtime_db("security_feature_snapshot_ready");

    let stock_csv = create_stock_history_csv(
        "security_feature_snapshot_ready",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_feature_snapshot_ready",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_feature_snapshot_ready",
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
                        {"notice_date":"2026-03-28","title":"2025年年度报告","art_code":"AN202603281234567890","columns":[{"column_name":"定期报告"}]},
                        {"notice_date":"2026-03-28","title":"2025年度利润分配预案公告","art_code":"AN202603281234567891","columns":[{"column_name":"公司公告"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_feature_snapshot",
        "args": {
            "symbol": "601916.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
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
            (
                "EXCEL_SKILL_ETF_FACTS_URL_BASE",
                format!("{server}/etf-facts"),
            ),
        ],
    );

    // 2026-04-09 CST: 这里先锁 feature_snapshot 最小正式契约，原因是 Task 2 的目标是把“当时可见特征”冻结成可回放对象，
    // 目的：要求它必须同时输出 snapshot_id、原子特征、因子分组特征和 snapshot_hash，避免后续训练阶段再回头补口径。
    assert_eq!(output["status"], "ok");
    assert_eq!(output["data"]["document_type"], "security_feature_snapshot");
    assert!(
        output["data"]["snapshot_id"]
            .as_str()
            .expect("snapshot id should exist")
            .starts_with("snapshot-")
    );
    assert!(output["data"]["raw_features_json"].is_object());
    assert!(output["data"]["group_features_json"].is_object());
    assert!(output["data"]["data_quality_flags"].is_array());
    assert!(
        output["data"]["snapshot_hash"]
            .as_str()
            .expect("snapshot hash should exist")
            .starts_with("snapshot-")
    );
    // 2026-04-10 CST: 这里先把统一评分版第一阶段要消费的关键原子特征锁进测试，原因是后续训练和评分结论都依赖这些字段稳定存在；
    // 目的：防止特征快照只保留“有对象”但缺少真正可训练、可解释的消息面与基本面字段，导致下游又回到手工拼接。
    let raw_features = output["data"]["raw_features_json"]
        .as_object()
        .expect("raw features should be an object");
    assert_eq!(raw_features["profit_signal"], "positive");
    assert_eq!(raw_features["announcement_count"], 2);
    assert_eq!(raw_features["has_annual_report_notice"], true);
    assert_eq!(raw_features["has_dividend_notice"], true);
    assert_eq!(raw_features["has_buyback_or_increase_notice"], false);
    assert_eq!(raw_features["disclosure_positive_keyword_count"], 2);
    assert_eq!(raw_features["disclosure_risk_keyword_count"], 0);
    assert!(raw_features.get("trend_bias").is_some());
    assert!(raw_features.get("momentum_signal").is_some());
    // 2026-04-16 CST: Added because A-1a approved the first formal training-field thickening pass.
    // Reason: training was still missing stable regime / industry / event-density segmentation fields.
    // Purpose: freeze these derived fields into the canonical snapshot before retraining starts consuming them.
    assert_eq!(raw_features["market_profile"], "a_share_core");
    assert_eq!(raw_features["sector_profile"], "a_share_bank");
    assert_eq!(raw_features["market_regime"], "a_share");
    assert_eq!(raw_features["industry_bucket"], "bank");
    assert_eq!(raw_features["instrument_subscope"], "equity");
    assert_eq!(raw_features["event_density_bucket"], "moderate");
    let group_features = output["data"]["group_features_json"]
        .as_object()
        .expect("group features should be an object");
    assert_eq!(group_features["M"]["market_regime"], "a_share");
    assert_eq!(group_features["M"]["industry_bucket"], "bank");
    assert_eq!(group_features["M"]["instrument_subscope"], "equity");
    assert_ne!(group_features["Q"]["flow_status"], "not_populated_v1");
    assert_eq!(group_features["Q"]["event_density_bucket"], "moderate");
    assert_ne!(group_features["V"]["valuation_status"], "not_populated_v1");
}

#[test]
fn security_feature_snapshot_freezes_etf_fact_features_into_x_group() {
    let runtime_db_path = create_test_runtime_db("security_feature_snapshot_etf_context");

    let etf_csv = create_stock_history_csv(
        "security_feature_snapshot_etf_context",
        "etf.csv",
        &build_confirmed_breakout_rows(220, 1.22),
    );
    let market_csv = create_stock_history_csv(
        "security_feature_snapshot_etf_context",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_feature_snapshot_etf_context",
        "sector.csv",
        &build_confirmed_breakout_rows(220, 1100.0),
    );
    import_history_csv(&runtime_db_path, &etf_csv, "159866.SZ");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "513520.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 500 Internal Server Error",
            "{\"error\":\"etf no financials\"}",
            "application/json",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{
                "data":{
                    "list":[
                        {"notice_date":"2026-04-08","title":"基金定期报告提示公告","art_code":"ETF2026040802","columns":[{"column_name":"基金公告"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
        (
            "/etf-facts",
            "HTTP/1.1 200 OK",
            r#"{
                "fund_name":"日经ETF工银",
                "benchmark":"日经225指数",
                "asset_scope":"跨境股票ETF",
                "latest_scale":42.6,
                "latest_share":31.2,
                "premium_discount_rate_pct":1.75,
                "structure_risk_flags":["QDII额度约束需持续跟踪"],
                "research_gaps":["前十大权重需补专项研究"]
            }"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_feature_snapshot",
        "args": {
            "symbol": "159866.SZ",
            "market_symbol": "510300.SH",
            "sector_symbol": "513520.SH",
            "market_profile": "a_share_core",
            "sector_profile": "cross_border_etf"
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
            (
                "EXCEL_SKILL_ETF_FACTS_URL_BASE",
                format!("{server}/etf-facts"),
            ),
        ],
    );

    // 2026-04-13 CST: 这里补 ETF 快照冻结红测，原因是 ETF 厚度不能只停留在 fullstack，必须继续下沉到 snapshot。
    // 目的：锁定 ETF 结构事实会被冻结为 raw feature 和 X 组特征，供训练与回放直接复用。
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["raw_features_json"]["subject_asset_class"],
        "etf"
    );
    assert_eq!(
        output["data"]["raw_features_json"]["etf_context_status"],
        "available"
    );
    assert_eq!(
        output["data"]["raw_features_json"]["etf_benchmark_available"],
        true
    );
    assert_eq!(
        output["data"]["raw_features_json"]["etf_scale_available"],
        true
    );
    assert_eq!(
        output["data"]["group_features_json"]["X"]["etf_context_status"],
        "available"
    );
    assert_eq!(
        output["data"]["group_features_json"]["X"]["etf_structure_risk_count"],
        1
    );
    assert_eq!(
        output["data"]["group_features_json"]["M"]["subject_asset_class"],
        "etf"
    );
}

#[test]
fn security_feature_snapshot_exposes_etf_specific_numeric_features() {
    let runtime_db_path = create_test_runtime_db("security_feature_snapshot_etf");

    let etf_csv = create_stock_history_csv(
        "security_feature_snapshot_etf",
        "etf.csv",
        &build_confirmed_breakout_rows(260, 101.0),
    );
    let market_csv = create_stock_history_csv(
        "security_feature_snapshot_etf",
        "market.csv",
        &build_confirmed_breakout_rows(260, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_feature_snapshot_etf",
        "sector.csv",
        &build_confirmed_breakout_rows(260, 110.0),
    );
    import_history_csv(&runtime_db_path, &etf_csv, "511010.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "511060.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "<html><body>financials unavailable for etf fixture</body></html>",
            "text/html",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{"data":{"list":[]}}"#,
            "application/json",
        ),
        (
            "/etf-facts",
            "HTTP/1.1 200 OK",
            r#"{
                "fund_name":"Nikkei Cross Border ETF Fixture",
                "benchmark":"Nikkei 225",
                "asset_scope":"cross_border_equity_etf",
                "latest_scale":18.2,
                "latest_share":9.6,
                "premium_discount_rate_pct":0.8,
                "structure_risk_flags":["cross_border_t_plus_gap"],
                "research_gaps":[]
            }"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_feature_snapshot",
        "args": {
            "symbol": "511010.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "511060.SH",
            "market_profile": "a_share_core",
            "sector_profile": "bond_etf_peer",
            "stop_loss_pct": 0.01,
            "target_return_pct": 0.015
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
            (
                "EXCEL_SKILL_ETF_FACTS_URL_BASE",
                format!("{server}/etf-facts"),
            ),
        ],
    );

    // 2026-04-11 CST: Lock the ETF raw snapshot feature family on the public feature
    // snapshot contract, because ETF training/runtime now depend on these numeric
    // differentiators instead of the old coarse-only seed.
    // Purpose: prove the ETF-specific fields are emitted through the formal tool path,
    // not only through internal helpers.
    assert_eq!(output["status"], "ok", "feature snapshot output: {output}");
    assert_eq!(output["data"]["instrument_type"], "ETF");
    assert!(output["data"]["raw_features_json"]["close_vs_sma50"].is_number());
    assert!(output["data"]["raw_features_json"]["close_vs_sma200"].is_number());
    assert!(output["data"]["raw_features_json"]["volume_ratio_20"].is_number());
    assert!(output["data"]["raw_features_json"]["mfi_14"].is_number());
    assert!(output["data"]["raw_features_json"]["rsrs_zscore_18_60"].is_number());
    assert!(output["data"]["raw_features_json"]["support_gap_pct_20"].is_number());
    assert!(output["data"]["raw_features_json"]["resistance_gap_pct_20"].is_number());
}

#[test]
fn security_feature_snapshot_preserves_gold_etf_manual_proxy_inputs() {
    let runtime_db_path = create_test_runtime_db("security_feature_snapshot_gold_proxy_inputs");

    let etf_csv = create_stock_history_csv(
        "security_feature_snapshot_gold_proxy_inputs",
        "gold_etf.csv",
        &build_confirmed_breakout_rows(260, 101.0),
    );
    let market_csv = create_stock_history_csv(
        "security_feature_snapshot_gold_proxy_inputs",
        "market.csv",
        &build_confirmed_breakout_rows(260, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_feature_snapshot_gold_proxy_inputs",
        "sector.csv",
        &build_confirmed_breakout_rows(260, 99.0),
    );
    import_history_csv(&runtime_db_path, &etf_csv, "518880.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "518800.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "<html><body>financials unavailable for gold etf fixture</body></html>",
            "text/html",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{"data":{"list":[]}}"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_feature_snapshot",
        "args": {
            "symbol": "518880.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "518800.SH",
            "market_profile": "a_share_core",
            "sector_profile": "gold_etf_peer",
            "external_proxy_inputs": {
                "gold_spot_proxy_status": "manual_bound",
                "gold_spot_proxy_return_5d": 0.024,
                "usd_index_proxy_status": "manual_bound",
                "usd_index_proxy_return_5d": -0.013,
                "real_rate_proxy_status": "manual_bound",
                "real_rate_proxy_delta_bp_5d": -8.5
            }
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
            (
                "EXCEL_SKILL_ETF_FACTS_URL_BASE",
                format!("{server}/etf-facts"),
            ),
        ],
    );

    // 2026-04-11 CST: Add a red snapshot regression for gold ETF manual proxy inputs,
    // reason: Scheme B now needs live gold/FX/rate proxy values to enter the formal
    // feature snapshot instead of staying outside the governed raw-feature contract.
    // Purpose: prove the public snapshot tool can freeze supplied gold ETF proxy
    // inputs for downstream scorecard, committee, and approval consumers.
    assert_eq!(output["status"], "ok", "feature snapshot output: {output}");
    assert_eq!(
        output["data"]["raw_features_json"]["gold_spot_proxy_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["raw_features_json"]["gold_spot_proxy_return_5d"],
        json!(0.024)
    );
    assert_eq!(
        output["data"]["raw_features_json"]["usd_index_proxy_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["raw_features_json"]["usd_index_proxy_return_5d"],
        json!(-0.013)
    );
    assert_eq!(
        output["data"]["raw_features_json"]["real_rate_proxy_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["raw_features_json"]["real_rate_proxy_delta_bp_5d"],
        json!(-8.5)
    );
    assert_eq!(
        output["data"]["group_features_json"]["X"]["gold_spot_proxy_status"],
        "manual_bound"
    );
}

#[test]
fn security_feature_snapshot_preserves_treasury_etf_manual_proxy_inputs() {
    let runtime_db_path = create_test_runtime_db("security_feature_snapshot_treasury_proxy_inputs");

    let etf_csv = create_stock_history_csv(
        "security_feature_snapshot_treasury_proxy_inputs",
        "treasury_etf.csv",
        &build_confirmed_breakout_rows(260, 101.0),
    );
    let market_csv = create_stock_history_csv(
        "security_feature_snapshot_treasury_proxy_inputs",
        "market.csv",
        &build_confirmed_breakout_rows(260, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_feature_snapshot_treasury_proxy_inputs",
        "sector.csv",
        &build_confirmed_breakout_rows(260, 110.0),
    );
    import_history_csv(&runtime_db_path, &etf_csv, "511010.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "511060.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "<html><body>financials unavailable for treasury etf fixture</body></html>",
            "text/html",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{"data":{"list":[]}}"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_feature_snapshot",
        "args": {
            "symbol": "511010.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "511060.SH",
            "market_profile": "a_share_core",
            "sector_profile": "bond_etf_peer",
            "external_proxy_inputs": {
                "yield_curve_proxy_status": "manual_bound",
                "yield_curve_slope_delta_bp_5d": -6.0,
                "funding_liquidity_proxy_status": "manual_bound",
                "funding_liquidity_spread_delta_bp_5d": -12.5
            }
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
            (
                "EXCEL_SKILL_ETF_FACTS_URL_BASE",
                format!("{server}/etf-facts"),
            ),
        ],
    );

    // 2026-04-11 CST: Add a red snapshot regression for treasury ETF manual proxy
    // inputs, reason: Scheme B now needs live yield-curve and funding-liquidity
    // proxies to enter the governed feature snapshot instead of staying outside it.
    // Purpose: prove the public snapshot tool can freeze supplied treasury ETF
    // proxy inputs for downstream scorecard and approval consumers.
    assert_eq!(output["status"], "ok", "feature snapshot output: {output}");
    assert_eq!(
        output["data"]["raw_features_json"]["yield_curve_proxy_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["raw_features_json"]["yield_curve_slope_delta_bp_5d"],
        json!(-6.0)
    );
    assert_eq!(
        output["data"]["raw_features_json"]["funding_liquidity_proxy_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["raw_features_json"]["funding_liquidity_spread_delta_bp_5d"],
        json!(-12.5)
    );
    assert_eq!(
        output["data"]["group_features_json"]["X"]["yield_curve_proxy_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["group_features_json"]["X"]["yield_curve_slope_delta_bp_5d"],
        json!(-6.0)
    );
    assert_eq!(
        output["data"]["group_features_json"]["X"]["funding_liquidity_proxy_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["group_features_json"]["X"]["funding_liquidity_spread_delta_bp_5d"],
        json!(-12.5)
    );
}

#[test]
fn security_feature_snapshot_hydrates_historical_proxy_backfill_for_treasury_etf() {
    let runtime_db_path =
        create_test_runtime_db("security_feature_snapshot_historical_treasury_proxy");

    let etf_csv = create_stock_history_csv(
        "security_feature_snapshot_historical_treasury_proxy",
        "treasury_etf.csv",
        &build_confirmed_breakout_rows(260, 101.0),
    );
    let market_csv = create_stock_history_csv(
        "security_feature_snapshot_historical_treasury_proxy",
        "market.csv",
        &build_confirmed_breakout_rows(260, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_feature_snapshot_historical_treasury_proxy",
        "sector.csv",
        &build_confirmed_breakout_rows(260, 110.0),
    );
    import_history_csv(&runtime_db_path, &etf_csv, "511010.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "511060.SH");

    let backfill_request = json!({
        "tool": "security_external_proxy_backfill",
        "args": {
            "batch_id": "snapshot-historical-treasury-batch",
            "created_at": "2026-04-11T23:58:00+08:00",
            "records": [{
                "symbol": "511010.SH",
                "as_of_date": "2025-09-17",
                "instrument_subscope": "treasury_etf",
                "external_proxy_inputs": {
                    "yield_curve_proxy_status": "manual_bound",
                    "yield_curve_slope_delta_bp_5d": -7.25,
                    "funding_liquidity_proxy_status": "manual_bound",
                    "funding_liquidity_spread_delta_bp_5d": 2.75
                }
            }]
        }
    });
    let backfill_output =
        run_cli_with_json_runtime_and_envs(&backfill_request.to_string(), &runtime_db_path, &[]);
    assert_eq!(backfill_output["status"], "ok");

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
                        {"notice_date":"2026-03-28","title":"2025年度报告","art_code":"AN202603281234567890","columns":[{"column_name":"定期报告"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_feature_snapshot",
        "args": {
            "symbol": "511010.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "511060.SH",
            "market_profile": "a_share_core",
            "sector_profile": "treasury_etf",
            "as_of_date": "2025-09-17"
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
            (
                "EXCEL_SKILL_ETF_FACTS_URL_BASE",
                format!("{server}/etf-facts"),
            ),
        ],
    );

    // 2026-04-11 CST: Add a historical-proxy snapshot regression, because P4 needs
    // dated proxy backfill to rehydrate formal feature snapshots for replay and
    // training instead of only supporting same-request manual proxy overrides.
    // Purpose: prove the governed snapshot path can hydrate stored treasury inputs.
    assert_eq!(output["status"], "ok", "feature snapshot output: {output}");
    assert_eq!(
        output["data"]["raw_features_json"]["yield_curve_proxy_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["raw_features_json"]["yield_curve_slope_delta_bp_5d"],
        json!(-7.25)
    );
    assert_eq!(
        output["data"]["group_features_json"]["X"]["funding_liquidity_spread_delta_bp_5d"],
        json!(2.75)
    );
}

#[test]
fn security_feature_snapshot_preserves_cross_border_etf_manual_proxy_inputs() {
    let runtime_db_path =
        create_test_runtime_db("security_feature_snapshot_cross_border_proxy_inputs");

    let etf_csv = create_stock_history_csv(
        "security_feature_snapshot_cross_border_proxy_inputs",
        "cross_border_etf.csv",
        &build_confirmed_breakout_rows(260, 101.0),
    );
    let market_csv = create_stock_history_csv(
        "security_feature_snapshot_cross_border_proxy_inputs",
        "market.csv",
        &build_confirmed_breakout_rows(260, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_feature_snapshot_cross_border_proxy_inputs",
        "sector.csv",
        &build_confirmed_breakout_rows(260, 110.0),
    );
    let fx_csv = create_stock_history_csv(
        "security_feature_snapshot_cross_border_proxy_inputs",
        "fx.csv",
        &build_confirmed_breakout_rows(260, 7.1),
    );
    import_history_csv(&runtime_db_path, &etf_csv, "159866.SZ");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "1326.T");
    import_history_csv(&runtime_db_path, &fx_csv, "JPYCNY.FX");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "<html><body>financials unavailable for cross-border etf fixture</body></html>",
            "text/html",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{"data":{"list":[]}}"#,
            "application/json",
        ),
        (
            "/etf-facts",
            "HTTP/1.1 200 OK",
            r#"{
                "fund_name":"Nikkei Cross Border ETF Fixture",
                "benchmark":"Nikkei 225",
                "asset_scope":"cross_border_equity_etf",
                "latest_scale":18.2,
                "latest_share":9.6,
                "premium_discount_rate_pct":0.8,
                "structure_risk_flags":["cross_border_t_plus_gap"],
                "research_gaps":[]
            }"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_feature_snapshot",
        "args": {
            "symbol": "159866.SZ",
            "market_symbol": "510300.SH",
            "sector_symbol": "1326.T",
            "market_profile": "a_share_core",
            "sector_profile": "nikkei_qdii_cross_border_peer",
            "underlying_symbol": "1326.T",
            "fx_symbol": "JPYCNY.FX",
            "external_proxy_inputs": {
                "fx_proxy_status": "manual_bound",
                "fx_return_5d": 0.011,
                "overseas_market_proxy_status": "manual_bound",
                "overseas_market_return_5d": -0.018,
                "market_session_gap_status": "manual_bound",
                "market_session_gap_days": 1.0
            }
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
            (
                "EXCEL_SKILL_ETF_FACTS_URL_BASE",
                format!("{server}/etf-facts"),
            ),
        ],
    );

    // 2026-04-11 CST: Add a red snapshot regression for cross-border ETF manual
    // proxy inputs, reason: Scheme B now needs FX, overseas-market, and session-gap
    // inputs to enter the governed feature snapshot instead of staying outside it.
    // Purpose: prove the public snapshot tool can freeze supplied cross-border ETF
    // proxy inputs for downstream scorecard and approval consumers.
    assert_eq!(output["status"], "ok", "feature snapshot output: {output}");
    assert_eq!(
        output["data"]["raw_features_json"]["fx_proxy_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["raw_features_json"]["fx_return_5d"],
        json!(0.011)
    );
    assert_eq!(
        output["data"]["raw_features_json"]["overseas_market_proxy_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["raw_features_json"]["overseas_market_return_5d"],
        json!(-0.018)
    );
    assert_eq!(
        output["data"]["raw_features_json"]["market_session_gap_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["raw_features_json"]["market_session_gap_days"],
        json!(1.0)
    );
    assert_eq!(
        output["data"]["group_features_json"]["X"]["fx_proxy_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["raw_features_json"]["cross_border_underlying_symbol"], "1326.T",
        "feature snapshot output: {output}"
    );
    assert_eq!(
        output["data"]["raw_features_json"]["cross_border_fx_symbol"],
        "JPYCNY.FX"
    );
    assert_eq!(
        output["data"]["group_features_json"]["X"]["cross_border_context_status"],
        "available"
    );
    assert_eq!(
        output["data"]["group_features_json"]["X"]["cross_border_analysis_method"],
        "underlying_first_cross_border_etf_v1"
    );
    assert_eq!(
        output["data"]["group_features_json"]["X"]["market_session_gap_days"],
        json!(1.0)
    );
}

#[test]
fn security_feature_snapshot_preserves_equity_etf_manual_proxy_inputs() {
    let runtime_db_path = create_test_runtime_db("security_feature_snapshot_equity_proxy_inputs");
    let etf_csv = create_stock_history_csv(
        "security_feature_snapshot_equity_proxy_inputs",
        "equity_etf.csv",
        &build_confirmed_breakout_rows(260, 1.25),
    );
    let market_csv = create_stock_history_csv(
        "security_feature_snapshot_equity_proxy_inputs",
        "market.csv",
        &build_confirmed_breakout_rows(260, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_feature_snapshot_equity_proxy_inputs",
        "sector.csv",
        &build_confirmed_breakout_rows(260, 980.0),
    );
    import_history_csv(&runtime_db_path, &etf_csv, "512880.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "<html><body>financials unavailable for equity etf fixture</body></html>",
            "text/html",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{"data":{"list":[]}}"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_feature_snapshot",
        "args": {
            "symbol": "512880.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "market_profile": "a_share_core",
            "sector_profile": "equity_etf_peer",
            "external_proxy_inputs": {
                "etf_fund_flow_proxy_status": "manual_bound",
                "etf_fund_flow_5d": 0.067,
                "premium_discount_proxy_status": "manual_bound",
                "premium_discount_pct": 0.0042,
                "benchmark_relative_strength_status": "manual_bound",
                "benchmark_relative_return_5d": 0.013
            }
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

    // 2026-04-11 CST: Add a red snapshot regression for equity ETF manual proxy
    // inputs, reason: Scheme B now needs fund-flow, premium-discount, and
    // benchmark-relative inputs to enter the governed feature snapshot.
    // Purpose: prove the public snapshot tool can freeze supplied equity ETF
    // proxy inputs for downstream scorecard and approval consumers.
    assert_eq!(output["status"], "ok", "feature snapshot output: {output}");
    assert_eq!(
        output["data"]["raw_features_json"]["etf_fund_flow_proxy_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["raw_features_json"]["etf_fund_flow_5d"],
        json!(0.067)
    );
    assert_eq!(
        output["data"]["raw_features_json"]["premium_discount_proxy_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["raw_features_json"]["premium_discount_pct"],
        json!(0.0042)
    );
    assert_eq!(
        output["data"]["raw_features_json"]["benchmark_relative_strength_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["raw_features_json"]["benchmark_relative_return_5d"],
        json!(0.013)
    );
    assert_eq!(
        output["data"]["group_features_json"]["X"]["etf_fund_flow_proxy_status"],
        "manual_bound"
    );
    assert_eq!(
        output["data"]["group_features_json"]["X"]["benchmark_relative_return_5d"],
        json!(0.013)
    );
}

fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_feature_snapshot_fixture"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path.to_path_buf(),
        &[],
    );
    assert_eq!(output["status"], "ok");
}

// 2026-04-09 CST: 这里沿用稳定上行样本，原因是本测试只关心可见特征冻结对象本身，
// 目的：降低行情噪声，让失败点聚焦在 snapshot_id / raw_features / group_features / hash 契约。
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
