mod common;

use chrono::{Duration, NaiveDate};
use excel_skill::ops::stock::security_scorecard_training::{
    debug_build_weekly_anchor_dates, debug_build_weekly_price_feature_rows,
    debug_build_weekly_rolling_split_plan, debug_load_governed_weekly_observation_dates,
};
use excel_skill::runtime::stock_history_store::StockHistoryRow;
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

// 2026-04-09 CST: 这里新增 scorecard training CLI 测试夹具，原因是 Task 5 需要先把正式训练入口的契约锁进红测；
// 目的：先验证“训练产物 + refit_run + model_registry”一体化输出，再做最小实现，避免后续把训练入口做成临时脚本。
fn create_training_fixture_dir(prefix: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_scorecard_training")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("security scorecard training fixture dir should exist");
    fixture_dir
}

// 2026-04-09 CST: 这里复用本地 HTTP 假服务，原因是训练入口会沿用 feature_snapshot/forward_outcome，而上游仍依赖财报和公告上下文；
// 目的：让训练测试只关注训练主链本身，不被外部网络或线上接口波动干扰。
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
        // 2026-04-09 CST: 这里放宽测试 HTTP 服务的接入次数，原因是训练入口会对多个样本重复拉取财报和公告上下文；
        // 目的：确保测试夹具覆盖多样本训练场景时不会因为本地假服务提早关闭而误报失败。
        for _ in 0..256 {
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
fn tool_catalog_includes_security_scorecard_training() {
    let output = run_cli_with_json("");

    // 2026-04-09 CST: 这里先锁 training Tool 的可发现性，原因是如果 catalog 不正式暴露它，后续 Skill 与训练编排就没有一等入口；
    // 目的：确保证券评分卡训练入口能和 snapshot/forward_outcome/refit 一样被统一发现与路由。
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_scorecard_training")
    );
}

#[test]
fn security_scorecard_training_generates_artifact_and_registers_refit_outputs() {
    let runtime_db_path = create_test_runtime_db("security_scorecard_training_ready");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scorecard_training_runtime");
    let fixture_dir = create_training_fixture_dir("security_scorecard_training_ready");

    let stock_up_csv = fixture_dir.join("stock_up.csv");
    let stock_down_csv = fixture_dir.join("stock_down.csv");
    let market_csv = fixture_dir.join("market.csv");
    let sector_csv = fixture_dir.join("sector.csv");

    fs::write(
        &stock_up_csv,
        build_trend_rows(420, 100.0, 0.9, 1.0).join("\n"),
    )
    .expect("upward symbol csv should be written");
    fs::write(
        &stock_down_csv,
        build_trend_rows(420, 120.0, -0.7, 1.0).join("\n"),
    )
    .expect("downward symbol csv should be written");
    fs::write(
        &market_csv,
        build_trend_rows(420, 3200.0, 2.5, 5.0).join("\n"),
    )
    .expect("market csv should be written");
    fs::write(
        &sector_csv,
        build_trend_rows(420, 980.0, 1.4, 2.0).join("\n"),
    )
    .expect("sector csv should be written");

    import_history_csv(&runtime_db_path, &stock_up_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &stock_down_csv, "600000.SH");
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
        "tool": "security_scorecard_training",
        "args": {
            "created_at": "2026-04-09T17:30:00+08:00",
            "training_runtime_root": runtime_root.to_string_lossy(),
            "market_scope": "A_SHARE",
            "instrument_scope": "EQUITY",
            "symbol_list": ["601916.SH", "600000.SH"],
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "horizon_days": 10,
            "target_head": "direction_head",
            "train_range": "2025-03-01..2025-08-31",
            "valid_range": "2025-09-01..2025-11-30",
            "test_range": "2025-12-01..2026-01-31",
            "feature_set_version": "security_feature_snapshot.v1",
            "label_definition_version": "security_forward_outcome.v1"
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

    // 2026-04-09 CST: 这里锁定 Task 5 方案 B 的最小正式合同，原因是训练入口必须一次返回产物、治理对象和落盘路径；
    // 目的：确保后续回算、重估和 package 挂接都能直接消费统一输出，而不是再去拼接中间状态。
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["refit_run"]["document_type"],
        "security_scorecard_refit_run"
    );
    assert_eq!(
        output["data"]["model_registry"]["document_type"],
        "security_scorecard_model_registry"
    );
    assert_eq!(
        output["data"]["model_registry"]["target_head"],
        "direction_head"
    );
    assert_eq!(output["data"]["model_registry"]["horizon_days"], 10);
    // 2026-04-14 CST: Added because "training passed" is not enough for later operators; they
    // also need one stable observability contract showing whether the run produced a usable
    // sample split and metrics summary.
    // Purpose: lock the minimal "can inspect training health" view before the pipeline moves on
    // to real-data backfill and larger retraining jobs.
    // 2026-04-16 CST: Added because A-1a expands the first formal training sample field set.
    // Reason: the prior 18-field baseline still missed regime / industry / event-density / QV proxy fields.
    // Purpose: keep retraining observability aligned with the approved thicker sample contract.
    // 2026-04-17 CST: Added because disclosure events now enter training as weighted component
    // scores instead of only sparse boolean/risk-count hints.
    // Purpose: lock the first formal event-scoring feature family into the training contract.
    assert_eq!(output["data"]["metrics_summary_json"]["feature_count"], 19);
    assert!(
        output["data"]["metrics_summary_json"]["sample_count"]
            .as_u64()
            .expect("sample_count should be numeric")
            >= 8
    );
    assert!(
        output["data"]["metrics_summary_json"]["train"]["sample_count"]
            .as_u64()
            .expect("train sample_count should be numeric")
            >= 4
    );
    assert!(
        output["data"]["metrics_summary_json"]["valid"]["sample_count"]
            .as_u64()
            .expect("valid sample_count should be numeric")
            >= 2
    );
    assert!(
        output["data"]["metrics_summary_json"]["test"]["sample_count"]
            .as_u64()
            .expect("test sample_count should be numeric")
            >= 2
    );
    assert_eq!(
        output["data"]["metrics_summary_json"]["post_validation_holdout"]["sample_count"],
        output["data"]["metrics_summary_json"]["valid"]["sample_count"]
            .as_u64()
            .expect("valid sample_count should be numeric")
            + output["data"]["metrics_summary_json"]["test"]["sample_count"]
                .as_u64()
                .expect("test sample_count should be numeric")
    );
    assert_eq!(
        output["data"]["metrics_summary_json"]["train"]["accuracy"],
        1.0
    );
    assert_eq!(
        output["data"]["metrics_summary_json"]["valid"]["accuracy"],
        1.0
    );
    assert_eq!(
        output["data"]["metrics_summary_json"]["test"]["accuracy"],
        1.0
    );
    assert_eq!(
        output["data"]["metrics_summary_json"]["train"]["positive_rate"],
        0.5
    );
    assert_eq!(
        output["data"]["metrics_summary_json"]["valid"]["positive_rate"],
        0.5
    );
    assert_eq!(
        output["data"]["metrics_summary_json"]["test"]["positive_rate"],
        0.5
    );
    // 2026-04-17 CST: Added because the user explicitly asked to inspect process metrics instead
    // of only split accuracy.
    // Reason: the formal training contract must now expose a governed diagnostic surface showing
    // correlation, walk-forward stability, drift, and segment slices.
    // Purpose: lock the minimum P0 diagnostic report contract before implementing the builder.
    assert!(
        output["data"]["metrics_summary_json"]["diagnostics"].is_object(),
        "expected diagnostics summary to exist in metrics_summary_json, output={output}"
    );
    assert!(
        output["data"]["metrics_summary_json"]["diagnostics"]["correlation_summary"].is_object(),
        "expected correlation summary to exist in diagnostics, output={output}"
    );
    assert!(
        output["data"]["metrics_summary_json"]["diagnostics"]["walk_forward_summary"].is_object(),
        "expected walk-forward summary to exist in diagnostics, output={output}"
    );
    assert!(
        output["data"]["metrics_summary_json"]["diagnostics"]["segment_slice_summary"].is_object(),
        "expected segment slice summary to exist in diagnostics, output={output}"
    );
    assert!(
        output["data"]["metrics_summary_json"]["diagnostics"]["readiness_assessment"].is_object(),
        "expected readiness assessment to exist in diagnostics, output={output}"
    );

    let artifact_path = PathBuf::from(
        output["data"]["artifact_path"]
            .as_str()
            .expect("artifact path should exist"),
    );
    let refit_run_path = PathBuf::from(
        output["data"]["refit_run_path"]
            .as_str()
            .expect("refit run path should exist"),
    );
    let model_registry_path = PathBuf::from(
        output["data"]["model_registry_path"]
            .as_str()
            .expect("model registry path should exist"),
    );
    let training_diagnostic_report_path = PathBuf::from(
        output["data"]["training_diagnostic_report_path"]
            .as_str()
            .expect("training diagnostic report path should exist"),
    );

    assert!(artifact_path.exists());
    assert!(refit_run_path.exists());
    assert!(model_registry_path.exists());
    assert!(training_diagnostic_report_path.exists());

    let artifact_json: Value =
        serde_json::from_slice(&fs::read(&artifact_path).expect("artifact should be readable"))
            .expect("artifact should be valid json");
    assert_eq!(
        artifact_json["model_id"],
        "a_share_equity_10d_direction_head"
    );
    assert_eq!(
        artifact_json["label_definition"],
        "security_forward_outcome.v1"
    );
    assert_eq!(artifact_json["training_window"], "2025-03-01..2025-08-31");
    assert_eq!(artifact_json["oot_window"], "2025-12-01..2026-01-31");
    assert!(artifact_json["features"].is_array());
    assert!(
        artifact_json["features"]
            .as_array()
            .expect("features should be an array")
            .len()
            >= 1
    );
    // 2026-04-10 CST: 这里把第一阶段统一评分版真正要入模的字段锁进产物断言，原因是训练链已经不应再停留在 4 个占位特征；
    // 目的：确保技术面、基本面、消息面结构化因子都进入 artifact，后续顺丰/平安验证时才能输出像样的问题点。
    let feature_names = artifact_json["features"]
        .as_array()
        .expect("features should be an array")
        .iter()
        .filter_map(|feature| feature["feature_name"].as_str())
        .collect::<Vec<_>>();
    // 2026-04-21 CST: Updated because the approved follow-up step now removes the
    // integrated summary label itself from training after the non-index cleanup.
    // Purpose: lock the retained index-focused atomic feature surface in the artifact contract.
    for expected_feature in [
        "market_regime",
        "instrument_subscope",
        "technical_alignment",
        "trend_bias",
        "trend_strength",
        "volume_confirmation",
        "breakout_signal",
        "momentum_signal",
        "flow_status",
        "volume_ratio_20",
        "mfi_14",
        "macd_histogram",
        "data_gap_count",
        "risk_note_count",
        "bollinger_position_20d",
        "range_position_14d",
        "mean_reversion_deviation_20d",
        "rsi_14",
        "atr_ratio_14",
    ] {
        assert!(
            feature_names
                .iter()
                .any(|feature_name| feature_name == &expected_feature),
            "expected trained feature `{expected_feature}` to exist in artifact"
        );
    }
    for removed_feature in [
        "industry_bucket",
        "subindustry_bucket",
        "profit_signal",
        "fundamental_status",
        "disclosure_status",
        "announcement_count",
        "event_density_bucket",
        "disclosure_risk_keyword_count",
        "has_risk_warning_notice",
        "hard_risk_score",
        "negative_attention_score",
        "positive_support_score",
        "event_net_impact_score",
        "revenue_yoy_pct",
        "net_profit_yoy_pct",
        "roe_pct",
        "shareholder_return_status",
        "quality_bucket",
        "integrated_stance",
    ] {
        assert!(
            !feature_names
                .iter()
                .any(|feature_name| feature_name == &removed_feature),
            "expected removed Phase-A feature `{removed_feature}` to stay out of artifact"
        );
    }
    // 2026-04-21 CST: Added because the approved Nikkei retraining route replaces the older
    // CCI-derived mean-reversion enum with MA20 percentage-deviation bands.
    // Reason: the governed training contract must prove the new bucket exists and the old one
    // no longer participates in model fitting.
    // Purpose: lock the feature-set swap before the next real retraining run.
    assert!(
        !feature_names
            .iter()
            .any(|feature_name| *feature_name == "valuation_status"),
        "expected valuation_status to be removed from the governed training contract"
    );
    assert!(
        !feature_names
            .iter()
            .any(|feature_name| *feature_name == "mean_reversion_state_20d"),
        "expected mean_reversion_deviation_20d to replace the old CCI-derived mean_reversion_state_20d training field"
    );
    assert!(
        !feature_names
            .iter()
            .any(|feature_name| *feature_name == "fundamental_quality_bucket"),
        "expected quality_bucket to replace the old fundamental_quality_bucket training alias"
    );

    let persisted_refit_run: Value = serde_json::from_slice(
        &fs::read(&refit_run_path).expect("persisted refit run should be readable"),
    )
    .expect("persisted refit run should be valid json");
    assert_eq!(
        persisted_refit_run["candidate_artifact_path"],
        Value::String(artifact_path.to_string_lossy().to_string())
    );

    let persisted_model_registry: Value = serde_json::from_slice(
        &fs::read(&model_registry_path).expect("persisted model registry should be readable"),
    )
    .expect("persisted model registry should be valid json");
    assert_eq!(
        persisted_model_registry["artifact_path"],
        Value::String(artifact_path.to_string_lossy().to_string())
    );
    assert_eq!(persisted_model_registry["target_head"], "direction_head");
    // 2026-04-14 CST: Added because training observability should be inspectable from the
    // persisted registry as well, not only from the immediate CLI response.
    // Purpose: keep later review, handoff, and retraining follow-up flows able to read the same
    // summary without replaying the entire training run in memory.
    assert_eq!(
        persisted_model_registry["metrics_summary_json"]["feature_count"],
        19
    );
    assert!(
        persisted_model_registry["metrics_summary_json"]["sample_count"]
            .as_u64()
            .expect("persisted registry sample_count should be numeric")
            >= 8
    );
    assert!(
        persisted_model_registry["metrics_summary_json"]["train"]["sample_count"]
            .as_u64()
            .expect("persisted registry train sample_count should be numeric")
            >= 4
    );
    assert!(
        persisted_model_registry["metrics_summary_json"]["valid"]["sample_count"]
            .as_u64()
            .expect("persisted registry valid sample_count should be numeric")
            >= 2
    );
    assert_eq!(
        persisted_model_registry["metrics_summary_json"]["post_validation_holdout"]["sample_count"],
        persisted_model_registry["metrics_summary_json"]["valid"]["sample_count"]
            .as_u64()
            .expect("persisted registry valid sample_count should be numeric")
            + persisted_model_registry["metrics_summary_json"]["test"]["sample_count"]
                .as_u64()
                .expect("persisted registry test sample_count should be numeric")
    );
    assert!(
        persisted_model_registry["metrics_summary_json"]["diagnostics"].is_object(),
        "expected diagnostics summary in persisted registry"
    );

    let persisted_training_diagnostic_report: Value = serde_json::from_slice(
        &fs::read(&training_diagnostic_report_path)
            .expect("persisted training diagnostic report should be readable"),
    )
    .expect("persisted training diagnostic report should be valid json");
    assert_eq!(
        persisted_training_diagnostic_report["document_type"],
        "security_scorecard_training_diagnostic_report"
    );
    assert!(
        persisted_training_diagnostic_report["correlation_summary"].is_object(),
        "expected correlation summary in persisted training diagnostic report"
    );
    assert!(
        persisted_training_diagnostic_report["walk_forward_summary"].is_object(),
        "expected walk-forward summary in persisted training diagnostic report"
    );
    assert!(
        persisted_training_diagnostic_report["segment_slice_summary"].is_object(),
        "expected segment slice summary in persisted training diagnostic report"
    );
}

#[test]
fn security_scorecard_training_persists_explicit_instrument_subscope_into_artifact_and_registry() {
    // 2026-04-20 CST: Added because Task 1 must first prove the trainer preserves an explicit
    // non-equity identity contract before we widen the real Nikkei and gold subject slices.
    // Purpose: force artifact and registry outputs to stop dropping instrument_subscope on the floor.
    let runtime_db_path = create_test_runtime_db("security_scorecard_training_identity_contract");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scorecard_training_runtime");
    let fixture_dir = create_training_fixture_dir("security_scorecard_training_identity_contract");

    let stock_up_csv = fixture_dir.join("stock_up.csv");
    let stock_down_csv = fixture_dir.join("stock_down.csv");
    let market_csv = fixture_dir.join("market.csv");
    let sector_csv = fixture_dir.join("sector.csv");

    fs::write(
        &stock_up_csv,
        build_trend_rows(420, 100.0, 0.9, 1.0).join("\n"),
    )
    .expect("upward symbol csv should be written");
    fs::write(
        &stock_down_csv,
        // 2026-04-20 CST: Adjusted because the new downward-head contract test needs
        // sustained negative forward returns inside the train window instead of a path
        // that hits the floor too early and collapses all labels to one class.
        // Purpose: keep this fixture focused on target-head semantics, not floor artifacts.
        build_trend_rows(420, 260.0, -0.3, 1.0).join("\n"),
    )
    .expect("downward symbol csv should be written");
    fs::write(
        &market_csv,
        build_trend_rows(420, 3200.0, 2.5, 5.0).join("\n"),
    )
    .expect("market csv should be written");
    fs::write(
        &sector_csv,
        build_trend_rows(420, 980.0, 1.4, 2.0).join("\n"),
    )
    .expect("sector csv should be written");

    import_history_csv(&runtime_db_path, &stock_up_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &stock_down_csv, "600000.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 200 OK",
            r#"[{"REPORT_DATE":"2025-12-31","NOTICE_DATE":"2026-03-28","TOTAL_OPERATE_INCOME":308227000000.0,"YSTZ":8.37,"PARENT_NETPROFIT":11117000000.0,"SJLTZ":9.31,"ROEJQ":14.8}]"#,
            "application/json",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{"data":{"list":[{"notice_date":"2026-03-28","title":"2025 annual report","art_code":"AN202603281234567890","columns":[{"column_name":"periodic_report"}]}]}}"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_scorecard_training",
        "args": {
            "created_at": "2026-04-20T11:00:00+08:00",
            "training_runtime_root": runtime_root.to_string_lossy(),
            "market_scope": "GLOBAL",
            "instrument_scope": "INDEX",
            "instrument_subscope": "nikkei_index",
            "symbol_list": ["601916.SH", "600000.SH"],
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "horizon_days": 10,
            "target_head": "direction_head",
            "train_range": "2025-03-01..2025-08-31",
            "valid_range": "2025-09-01..2025-11-30",
            "test_range": "2025-12-01..2026-01-31",
            "feature_set_version": "security_feature_snapshot.v1",
            "label_definition_version": "security_forward_outcome.v1"
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

    assert_eq!(output["status"], "ok", "output={output}");
    let artifact_path = PathBuf::from(
        output["data"]["artifact_path"]
            .as_str()
            .expect("artifact path should exist"),
    );
    let artifact_json: Value =
        serde_json::from_slice(&fs::read(&artifact_path).expect("artifact should be readable"))
            .expect("artifact should be valid json");
    assert_eq!(artifact_json["instrument_subscope"], "nikkei_index");
    assert_eq!(
        output["data"]["model_registry"]["instrument_subscope"],
        "nikkei_index"
    );
}

#[test]
fn security_scorecard_training_supports_direction_down_head_contract() {
    // 2026-04-20 CST: Added because the approved trainer-contract refactor must
    // expose a clean downward target head before any retraining resumes.
    // Reason: the old contract reused positive_label_definition for every direction.
    // Purpose: lock the downward artifact contract at the CLI boundary.
    let runtime_db_path = create_test_runtime_db("security_scorecard_training_direction_down");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scorecard_training_runtime");
    let fixture_dir = create_training_fixture_dir("security_scorecard_training_direction_down");
    let nikkei_csv = fixture_dir.join("nikkei_decade.csv");

    fs::write(
        &nikkei_csv,
        build_nikkei_decade_rows(3900, 16800.0).join("\n"),
    )
    .expect("nikkei csv should be written");
    import_history_csv(&runtime_db_path, &nikkei_csv, "NK225.IDX");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "<html><body>financials unavailable for downward head index fixture</body></html>",
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
        "tool": "security_scorecard_training",
        "args": {
            "created_at": "2026-04-20T22:00:00+08:00",
            "training_runtime_root": runtime_root.to_string_lossy(),
            "market_scope": "GLOBAL",
            "instrument_scope": "INDEX",
            "instrument_subscope": "nikkei_index",
            "symbol_list": ["NK225.IDX"],
            "market_symbol": "NK225.IDX",
            "sector_symbol": "NK225.IDX",
            "horizon_days": 10,
            "target_head": "direction_down_head",
            "train_range": "2016-04-20..2025-09-30",
            "valid_range": "2025-10-01..2025-12-31",
            "test_range": "2026-01-01..2026-04-20",
            "feature_set_version": "security_feature_snapshot.v1",
            "label_definition_version": "security_forward_outcome.v1"
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

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["artifact"]["target_head"],
        "direction_down_head"
    );
    assert_eq!(
        output["data"]["artifact"]["target_label_definition"],
        "negative_return_10d"
    );
    assert_eq!(
        output["data"]["artifact"]["positive_label_definition"],
        Value::Null
    );
    assert_eq!(
        output["data"]["model_registry"]["target_head"],
        "direction_down_head"
    );

    let artifact_path = PathBuf::from(
        output["data"]["artifact_path"]
            .as_str()
            .expect("artifact path should exist"),
    );
    let artifact_json: Value =
        serde_json::from_slice(&fs::read(&artifact_path).expect("artifact should be readable"))
            .expect("artifact should be valid json");
    assert_eq!(artifact_json["target_head"], "direction_down_head");
    assert_eq!(
        artifact_json["target_label_definition"],
        "negative_return_10d"
    );
    assert_eq!(artifact_json["positive_label_definition"], Value::Null);
}

#[test]
fn security_scorecard_training_supports_repair_stable_head_contract() {
    // 2026-04-21 CST: Added because the approved Nikkei retraining route now pivots
    // from plain direction to oversold-repair stability.
    // Reason: the user explicitly accepted a new binary head instead of reusing direction_up/down.
    // Purpose: lock the repair-stable artifact contract before the new head reaches real training.
    let runtime_db_path = create_test_runtime_db("security_scorecard_training_repair_stable");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scorecard_training_runtime");
    let fixture_dir = create_training_fixture_dir("security_scorecard_training_repair_stable");
    let nikkei_csv = fixture_dir.join("nikkei_decade.csv");

    fs::write(
        &nikkei_csv,
        build_nikkei_decade_rows(3900, 16800.0).join("\n"),
    )
    .expect("nikkei csv should be written");
    import_history_csv(&runtime_db_path, &nikkei_csv, "NK225.IDX");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "<html><body>financials unavailable for repair-stable index fixture</body></html>",
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
        "tool": "security_scorecard_training",
        "args": {
            "created_at": "2026-04-21T10:20:00+08:00",
            "training_runtime_root": runtime_root.to_string_lossy(),
            "market_scope": "GLOBAL",
            "instrument_scope": "INDEX",
            "instrument_subscope": "nikkei_index",
            "symbol_list": ["NK225.IDX"],
            "market_symbol": "NK225.IDX",
            "sector_symbol": "NK225.IDX",
            "horizon_days": 10,
            "target_head": "repair_stable_head",
            "train_range": "2016-04-20..2025-09-30",
            "valid_range": "2025-10-01..2025-12-31",
            "test_range": "2026-01-01..2026-04-20",
            "feature_set_version": "security_feature_snapshot.v1",
            "label_definition_version": "security_forward_outcome.v1"
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

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["artifact"]["target_head"],
        "repair_stable_head"
    );
    assert_eq!(
        output["data"]["artifact"]["target_label_definition"],
        "repair_stable_10d"
    );
    assert_eq!(
        output["data"]["artifact"]["positive_label_definition"],
        "repair_stable_10d"
    );
    assert_eq!(
        output["data"]["model_registry"]["target_head"],
        "repair_stable_head"
    );
    assert!(
        output["data"]["metrics_summary_json"]["sample_count"]
            .as_u64()
            .expect("sample_count should be numeric")
            >= 3
    );

    let artifact_path = PathBuf::from(
        output["data"]["artifact_path"]
            .as_str()
            .expect("artifact path should exist"),
    );
    let artifact_json: Value =
        serde_json::from_slice(&fs::read(&artifact_path).expect("artifact should be readable"))
            .expect("artifact should be valid json");
    assert_eq!(artifact_json["target_head"], "repair_stable_head");
    assert_eq!(
        artifact_json["target_label_definition"],
        "repair_stable_10d"
    );
    assert_eq!(
        artifact_json["positive_label_definition"],
        "repair_stable_10d"
    );
}

#[test]
fn security_scorecard_training_nikkei_repair_contract_uses_futures_factors_and_drops_zero_variance_fields()
 {
    let runtime_db_path = create_test_runtime_db("security_scorecard_training_nikkei_futures");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scorecard_training_runtime");
    let fixture_dir = create_training_fixture_dir("security_scorecard_training_nikkei_futures");
    let nikkei_csv = fixture_dir.join("nikkei_decade.csv");
    let futures_csv = fixture_dir.join("nikkei_futures_decade.csv");

    fs::write(
        &nikkei_csv,
        build_nikkei_decade_rows(3900, 16800.0).join("\n"),
    )
    .expect("nikkei csv should be written");
    fs::write(
        &futures_csv,
        build_nikkei_futures_decade_rows(3900, 16860.0).join("\n"),
    )
    .expect("nikkei futures csv should be written");
    import_history_csv(&runtime_db_path, &nikkei_csv, "NK225.IDX");
    import_history_csv(&runtime_db_path, &futures_csv, "NK225_F1.FUT");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "<html><body>financials unavailable for nikkei futures fixture</body></html>",
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
        "tool": "security_scorecard_training",
        "args": {
            "created_at": "2026-04-21T22:10:00+08:00",
            "training_runtime_root": runtime_root.to_string_lossy(),
            "market_scope": "GLOBAL",
            "instrument_scope": "INDEX",
            "instrument_subscope": "nikkei_index",
            "symbol_list": ["NK225.IDX"],
            "market_symbol": "NK225.IDX",
            "sector_symbol": "NK225.IDX",
            "futures_symbol": "NK225_F1.FUT",
            "horizon_days": 10,
            "target_head": "repair_stable_head",
            "train_range": "2016-04-20..2025-09-30",
            "valid_range": "2025-10-01..2025-12-31",
            "test_range": "2026-01-01..2026-04-20",
            "feature_set_version": "security_feature_snapshot.v1",
            "label_definition_version": "security_forward_outcome.v1"
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

    assert_eq!(output["status"], "ok", "output={output}");
    let artifact_path = PathBuf::from(
        output["data"]["artifact_path"]
            .as_str()
            .expect("artifact path should exist"),
    );
    let artifact_json: Value =
        serde_json::from_slice(&fs::read(&artifact_path).expect("artifact should be readable"))
            .expect("artifact should be valid json");
    let feature_names = artifact_json["features"]
        .as_array()
        .expect("features should be an array")
        .iter()
        .filter_map(|feature| feature["feature_name"].as_str())
        .collect::<Vec<_>>();

    for expected_feature in [
        "futures_return_1d",
        "futures_spot_basis_pct",
        "futures_return_3d",
        "spot_return_3d",
        "futures_relative_strength_3d",
    ] {
        assert!(
            feature_names
                .iter()
                .any(|feature_name| feature_name == &expected_feature),
            "expected Nikkei futures feature `{expected_feature}` to exist in artifact"
        );
    }
    for removed_feature in [
        "instrument_subscope",
        "volume_confirmation",
        "flow_status",
        "volume_ratio_20",
        "mfi_14",
        "data_gap_count",
        "risk_note_count",
        "futures_lead_strength_3d",
    ] {
        assert!(
            !feature_names
                .iter()
                .any(|feature_name| feature_name == &removed_feature),
            "expected zero-information Nikkei feature `{removed_feature}` to stay out of artifact"
        );
    }
}

#[test]
fn security_scorecard_training_generates_nikkei_index_artifact() {
    // 2026-04-20 CST: Added because Task 2 must prove the governed trainer can
    // complete one end-to-end Nikkei index slice before any macro augmentation is considered.
    // Purpose: lock the baseline `NK225.IDX` path on artifact identity, label contract, and sample volume.
    let runtime_db_path = create_test_runtime_db("security_scorecard_training_nikkei_index");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scorecard_training_runtime");
    let fixture_dir = create_training_fixture_dir("security_scorecard_training_nikkei_index");
    let nikkei_csv = fixture_dir.join("nikkei.csv");

    fs::write(
        &nikkei_csv,
        build_nikkei_mixed_regime_rows(420, 32500.0).join("\n"),
    )
    .expect("nikkei csv should be written");
    import_history_csv(&runtime_db_path, &nikkei_csv, "NK225.IDX");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "<html><body>financials unavailable for index fixture</body></html>",
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
        "tool": "security_scorecard_training",
        "args": {
            "created_at": "2026-04-20T16:00:00+08:00",
            "training_runtime_root": runtime_root.to_string_lossy(),
            "market_scope": "GLOBAL",
            "instrument_scope": "INDEX",
            "instrument_subscope": "nikkei_index",
            "symbol_list": ["NK225.IDX"],
            "market_symbol": "NK225.IDX",
            "sector_symbol": "NK225.IDX",
            "horizon_days": 10,
            "target_head": "direction_head",
            "train_range": "2025-03-01..2025-08-31",
            "valid_range": "2025-09-01..2025-11-30",
            "test_range": "2025-12-01..2026-01-31",
            "feature_set_version": "security_feature_snapshot.v1",
            "label_definition_version": "security_forward_outcome.v1"
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

    assert_eq!(output["status"], "ok", "output={output}");
    let artifact_path = PathBuf::from(
        output["data"]["artifact_path"]
            .as_str()
            .expect("artifact path should exist"),
    );
    let artifact_json: Value =
        serde_json::from_slice(&fs::read(&artifact_path).expect("artifact should be readable"))
            .expect("artifact should be valid json");
    let feature_names = artifact_json["features"]
        .as_array()
        .expect("features should be an array")
        .iter()
        .filter_map(|feature| feature["feature_name"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        artifact_json["model_id"],
        "global_index_nikkei_index_1w_direction_head"
    );
    assert_eq!(artifact_json["instrument_subscope"], "nikkei_index");
    assert_eq!(
        artifact_json["label_definition"],
        "security_forward_outcome.v1"
    );
    assert!(
        output["data"]["metrics_summary_json"]["sample_count"]
            .as_u64()
            .expect("sample_count should be numeric")
            >= 3
    );
    assert!(
        !feature_names
            .iter()
            .any(|feature_name| *feature_name == "risk_note_count"),
        "expected Nikkei index artifact to drop risk_note_count because it is not a governed index risk factor"
    );
}

#[test]
fn security_scorecard_training_supports_decade_nikkei_training_and_post_2025_10_holdout() {
    // 2026-04-20 CST: Added because the user explicitly rejected the thin Nikkei baseline
    // and approved a 10-year governed training slice with a post-2025-10 holdout check.
    // Purpose: force the trainer to use materially denser decade-scale samples and expose holdout accuracy after the cutoff.
    let runtime_db_path = create_test_runtime_db("security_scorecard_training_nikkei_decade");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scorecard_training_runtime");
    let fixture_dir = create_training_fixture_dir("security_scorecard_training_nikkei_decade");
    let nikkei_csv = fixture_dir.join("nikkei_decade.csv");

    fs::write(
        &nikkei_csv,
        build_nikkei_decade_rows(3900, 16800.0).join("\n"),
    )
    .expect("nikkei decade csv should be written");
    import_history_csv(&runtime_db_path, &nikkei_csv, "NK225.IDX");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "<html><body>financials unavailable for decade index fixture</body></html>",
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
        "tool": "security_scorecard_training",
        "args": {
            "created_at": "2026-04-20T20:00:00+08:00",
            "training_runtime_root": runtime_root.to_string_lossy(),
            "market_scope": "GLOBAL",
            "instrument_scope": "INDEX",
            "instrument_subscope": "nikkei_index",
            "symbol_list": ["NK225.IDX"],
            "market_symbol": "NK225.IDX",
            "sector_symbol": "NK225.IDX",
            "horizon_days": 10,
            "target_head": "direction_head",
            "train_range": "2016-04-20..2025-09-30",
            "valid_range": "2025-10-01..2025-12-31",
            "test_range": "2026-01-01..2026-04-20",
            "feature_set_version": "security_feature_snapshot.v1",
            "label_definition_version": "security_forward_outcome.v1"
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

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["artifact"]["model_id"],
        "global_index_nikkei_index_1w_direction_head"
    );
    assert!(
        output["data"]["metrics_summary_json"]["train"]["sample_count"]
            .as_u64()
            .expect("train sample count should be numeric")
            >= 60
    );
    assert!(
        output["data"]["metrics_summary_json"]["sample_count"]
            .as_u64()
            .expect("sample_count should be numeric")
            >= 70
    );
    assert_eq!(
        output["data"]["metrics_summary_json"]["post_validation_holdout"]["cutoff_date"],
        "2025-10-01"
    );
    let feature_names = output["data"]["artifact"]["features"]
        .as_array()
        .expect("artifact features should be an array")
        .iter()
        .filter_map(|feature| feature["feature_name"].as_str())
        .collect::<Vec<_>>();
    assert!(
        feature_names
            .iter()
            .any(|feature_name| *feature_name == "weekly_spot_return_p50"),
        "nikkei weekly training artifact should expose weekly spot features"
    );
    assert!(
        output["data"]["metrics_summary_json"]["post_validation_holdout"]["sample_count"]
            .as_u64()
            .expect("holdout sample count should be numeric")
            >= 6
    );
    assert!(
        output["data"]["metrics_summary_json"]["post_validation_holdout"]["accuracy"]
            .as_f64()
            .expect("holdout accuracy should be numeric")
            >= 0.0
    );
}

#[test]
fn security_scorecard_training_nikkei_weekly_route_emits_weekly_artifact_features() {
    let runtime_db_path = create_test_runtime_db("security_scorecard_training_nikkei_weekly_route");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scorecard_training_runtime");
    let fixture_dir =
        create_training_fixture_dir("security_scorecard_training_nikkei_weekly_route");
    let nikkei_csv = fixture_dir.join("nikkei_weekly_route.csv");
    let futures_csv = fixture_dir.join("nikkei_futures_weekly_route.csv");

    fs::write(
        &nikkei_csv,
        build_nikkei_decade_rows(420, 16800.0).join("\n"),
    )
    .expect("nikkei weekly route csv should be written");
    fs::write(
        &futures_csv,
        build_nikkei_futures_decade_rows(420, 16840.0).join("\n"),
    )
    .expect("nikkei futures weekly route csv should be written");
    import_history_csv(&runtime_db_path, &nikkei_csv, "NK225.IDX");
    import_history_csv(&runtime_db_path, &futures_csv, "NK225_F1.FUT");
    seed_capital_flow_history_for_training(
        &runtime_db_path,
        NaiveDate::from_ymd_opt(2015, 8, 14).expect("seed date should be valid"),
        80,
    );

    let request = json!({
        "tool": "security_scorecard_training",
        "args": {
            "created_at": "2026-04-23T15:00:00+08:00",
            "artifact_runtime_root": runtime_root.to_string_lossy(),
            "capital_flow_runtime_root": runtime_db_path.parent().expect("runtime db should have parent").to_string_lossy(),
            "market_scope": "GLOBAL",
            "instrument_scope": "INDEX",
            "instrument_subscope": "nikkei_index",
            "symbol_list": ["NK225.IDX"],
            "market_symbol": "NK225.IDX",
            "sector_symbol": "NK225.IDX",
            "futures_symbol": "NK225_F1.FUT",
            "horizon_days": 10,
            "target_head": "direction_head",
            "train_range": "2016-03-01..2016-08-31",
            "valid_range": "2016-09-01..2016-09-14",
            "test_range": "2016-09-15..2016-09-30",
            "feature_set_version": "security_feature_snapshot.v1",
            "label_definition_version": "security_forward_outcome.v1",
            "capital_source_feature_mode": "nikkei_jpx_mof_v1"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(&request.to_string(), &runtime_db_path, &[]);

    assert_eq!(output["status"], "ok", "output={output}");
    let feature_names = output["data"]["artifact"]["features"]
        .as_array()
        .expect("artifact features should be an array")
        .iter()
        .filter_map(|feature| feature["feature_name"].as_str())
        .collect::<Vec<_>>();
    assert!(
        feature_names
            .iter()
            .any(|feature_name| *feature_name == "weekly_spot_return_p50"),
        "weekly route should emit weekly spot feature names"
    );
    assert!(
        feature_names
            .iter()
            .any(|feature_name| *feature_name == "weekly_basis_pct_p50"),
        "weekly route should emit weekly basis feature names"
    );
    assert!(
        output["data"]["metrics_summary_json"]["train"]["sample_count"]
            .as_u64()
            .expect("train sample count should be numeric")
            >= 24
    );
    assert!(
        output["data"]["metrics_summary_json"]["train"]["sample_count"]
            .as_u64()
            .expect("train sample count should be numeric")
            <= 28,
        "weekly rolling route should deduplicate repeated train anchors instead of replaying the same week in every rolling window"
    );
    assert!(
        output["data"]["metrics_summary_json"]["valid"]["sample_count"]
            .as_u64()
            .expect("valid sample count should be numeric")
            >= 4
    );
    assert!(
        output["data"]["metrics_summary_json"]["test"]["sample_count"]
            .as_u64()
            .expect("test sample count should be numeric")
            >= 4
    );
    assert_eq!(
        output["data"]["metrics_summary_json"]["valid"]["sample_count"],
        output["data"]["metrics_summary_json"]["test"]["sample_count"],
        "weekly rolling route should score one valid week and one test week per rolling window"
    );
    assert!(
        output["data"]["metrics_summary_json"]["rolling_window_count"]
            .as_u64()
            .expect("rolling window count should be numeric")
            >= 4
    );
    assert_eq!(
        output["data"]["metrics_summary_json"]["valid"]["sample_count"],
        output["data"]["metrics_summary_json"]["rolling_window_count"],
        "weekly rolling route should aggregate validation by executed windows instead of flattening all weeks into one global pool"
    );
    assert_eq!(
        output["data"]["metrics_summary_json"]["test"]["sample_count"],
        output["data"]["metrics_summary_json"]["rolling_window_count"],
        "weekly rolling route should aggregate test by executed windows instead of flattening all weeks into one global pool"
    );
}

#[test]
fn security_scorecard_training_keeps_capital_source_metrics_as_observation_only_in_nikkei_run() {
    // 2026-04-24 CST: Updated because the approved route now keeps capital-source
    // metrics as observation-only output instead of training features.
    // Reason: the user explicitly asked to observe funding metrics first before
    // allowing them to influence weekly Nikkei training.
    // Purpose: lock the new boundary between training features and observation output.
    let runtime_db_path =
        create_test_runtime_db("security_scorecard_training_nikkei_capital_source_ab");
    let capital_flow_runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .to_path_buf();
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scorecard_training_runtime");
    let fixture_dir =
        create_training_fixture_dir("security_scorecard_training_nikkei_capital_source_ab");
    let nikkei_csv = fixture_dir.join("nikkei_decade.csv");

    fs::write(
        &nikkei_csv,
        build_nikkei_decade_rows(3900, 16800.0).join("\n"),
    )
    .expect("nikkei decade csv should be written");
    import_history_csv(&runtime_db_path, &nikkei_csv, "NK225.IDX");
    seed_capital_flow_history_for_training(
        &runtime_db_path,
        NaiveDate::from_ymd_opt(2015, 8, 14).expect("seed date should be valid"),
        560,
    );

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "<html><body>financials unavailable for capital source ab fixture</body></html>",
            "text/html",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{"data":{"list":[]}}"#,
            "application/json",
        ),
    ]);

    let baseline_request = json!({
        "tool": "security_scorecard_training",
        "args": {
            "created_at": "2026-04-22T13:40:00+08:00",
            "artifact_runtime_root": runtime_root.to_string_lossy(),
            "market_scope": "GLOBAL",
            "instrument_scope": "INDEX",
            "instrument_subscope": "nikkei_index",
            "symbol_list": ["NK225.IDX"],
            "market_symbol": "NK225.IDX",
            "sector_symbol": "NK225.IDX",
            "horizon_days": 10,
            "target_head": "direction_head",
            "train_range": "2016-04-20..2025-09-30",
            "valid_range": "2025-10-01..2025-12-31",
            "test_range": "2026-01-01..2026-04-20",
            "feature_set_version": "security_feature_snapshot.v1",
            "label_definition_version": "security_forward_outcome.v1"
        }
    });
    let enhanced_request = json!({
        "tool": "security_scorecard_training",
        "args": {
            "created_at": "2026-04-22T13:41:00+08:00",
            "artifact_runtime_root": runtime_root.to_string_lossy(),
            "capital_flow_runtime_root": capital_flow_runtime_root.to_string_lossy(),
            "market_scope": "GLOBAL",
            "instrument_scope": "INDEX",
            "instrument_subscope": "nikkei_index",
            "symbol_list": ["NK225.IDX"],
            "market_symbol": "NK225.IDX",
            "sector_symbol": "NK225.IDX",
            "horizon_days": 10,
            "target_head": "direction_head",
            "train_range": "2016-04-20..2025-09-30",
            "valid_range": "2025-10-01..2025-12-31",
            "test_range": "2026-01-01..2026-04-20",
            "feature_set_version": "security_feature_snapshot.v1",
            "label_definition_version": "security_forward_outcome.v1",
            "capital_source_feature_mode": "nikkei_jpx_mof_v1"
        }
    });

    let baseline_output = run_cli_with_json_runtime_and_envs(
        &baseline_request.to_string(),
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
    let enhanced_output = run_cli_with_json_runtime_and_envs(
        &enhanced_request.to_string(),
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

    assert_eq!(baseline_output["status"], "ok", "output={baseline_output}");
    assert_eq!(enhanced_output["status"], "ok", "output={enhanced_output}");
    assert_eq!(
        baseline_output["data"]["metrics_summary_json"]["sample_count"],
        enhanced_output["data"]["metrics_summary_json"]["sample_count"]
    );

    let baseline_artifact_path = PathBuf::from(
        baseline_output["data"]["artifact_path"]
            .as_str()
            .expect("baseline artifact path should exist"),
    );
    let enhanced_artifact_path = PathBuf::from(
        enhanced_output["data"]["artifact_path"]
            .as_str()
            .expect("enhanced artifact path should exist"),
    );
    let baseline_artifact_json: Value = serde_json::from_slice(
        &fs::read(&baseline_artifact_path).expect("baseline artifact should be readable"),
    )
    .expect("baseline artifact should be valid json");
    let enhanced_artifact_json: Value = serde_json::from_slice(
        &fs::read(&enhanced_artifact_path).expect("enhanced artifact should be readable"),
    )
    .expect("enhanced artifact should be valid json");

    let baseline_feature_names = baseline_artifact_json["features"]
        .as_array()
        .expect("baseline features should be an array")
        .iter()
        .filter_map(|feature| feature["feature_name"].as_str())
        .collect::<Vec<_>>();
    let enhanced_feature_names = enhanced_artifact_json["features"]
        .as_array()
        .expect("enhanced features should be an array")
        .iter()
        .filter_map(|feature| feature["feature_name"].as_str())
        .collect::<Vec<_>>();

    for forbidden_feature in [
        "overseas_flow_persistence_4w",
        "domestic_flow_persistence_4w",
        "overseas_vs_domestic_spread",
        "foreign_net_buy_ratio_ma2_vs_prev2",
        "investment_trust_net_buy_ratio_wow_1w",
        "mof_foreign_japan_equity_net_ma2_vs_prev2",
        "recent_up_move_foreign_inflow_share",
        "recent_up_move_domestic_inflow_share",
        "recent_down_move_foreign_outflow_share",
        "recent_down_move_domestic_outflow_share",
    ] {
        assert!(
            !baseline_feature_names
                .iter()
                .any(|feature_name| feature_name == &forbidden_feature),
            "baseline run should not expose observation-only capital-source metric `{forbidden_feature}`"
        );
        assert!(
            !enhanced_feature_names
                .iter()
                .any(|feature_name| feature_name == &forbidden_feature),
            "enhanced run should keep observation-only capital-source metric `{forbidden_feature}` out of training"
        );
    }
    for removed_feature in [
        "foreign_net_buy_ratio_1w",
        "foreign_net_buy_ratio_wow_1w",
        "investment_trust_net_buy_ratio_1w",
        "mof_foreign_japan_equity_net_4w",
        "mof_foreign_japan_equity_net_wow_1w",
    ] {
        assert!(
            !enhanced_feature_names
                .iter()
                .any(|feature_name| feature_name == &removed_feature),
            "enhanced run should stop exposing stale capital-source feature `{removed_feature}`"
        );
    }
    assert!(
        baseline_output["data"]["metrics_summary_json"]["capital_source_observation"].is_null(),
        "baseline run should not emit capital-source observation summary"
    );
    assert!(
        enhanced_output["data"]["metrics_summary_json"]["capital_source_observation"].is_object(),
        "enhanced run should emit capital-source observation summary"
    );
    assert_eq!(
        enhanced_output["data"]["metrics_summary_json"]["capital_source_observation"]["mode"],
        "observation_only"
    );
    assert!(
        enhanced_output["data"]["metrics_summary_json"]["capital_source_observation"]["factor_count"]
            .as_u64()
            .expect("capital-source observation factor count should be numeric")
            >= 10
    );
    assert!(
        enhanced_output["data"]["metrics_summary_json"]["capital_source_observation"]["latest_values"]
            ["overseas_flow_persistence_4w"]
            .is_number(),
        "enhanced run should retain latest observation value for overseas flow persistence"
    );
    for observation_key in [
        "total_net_flow_ratio_4w",
        "total_net_flow_ratio_13w",
        "total_net_flow_ratio_26w",
        "total_net_flow_ratio_52w",
        "total_positive_flow_share_13w",
        "total_positive_flow_share_26w",
        "total_positive_flow_share_52w",
        "total_net_flow_ratio_13w_vs_prev13w",
        "total_net_flow_ratio_26w_vs_prev26w",
    ] {
        assert!(
            enhanced_output["data"]["metrics_summary_json"]["capital_source_observation"]["latest_values"]
                [observation_key]
                .is_number(),
            "enhanced run should expose total-flow observation metric `{observation_key}`"
        );
        assert!(
            enhanced_output["data"]["metrics_summary_json"]["capital_source_observation"]["factor_stats"]
                [observation_key]
                .is_object(),
            "enhanced run should expose factor stats for total-flow observation metric `{observation_key}`"
        );
    }

    let enhanced_diagnostic_path = PathBuf::from(
        enhanced_output["data"]["training_diagnostic_report_path"]
            .as_str()
            .expect("enhanced diagnostic path should exist"),
    );
    let enhanced_diagnostic_json: Value = serde_json::from_slice(
        &fs::read(&enhanced_diagnostic_path).expect("enhanced diagnostic should be readable"),
    )
    .expect("enhanced diagnostic should be valid json");
    assert!(
        !enhanced_diagnostic_json["feature_coverage_summary"]["features"]
            .as_array()
            .expect("feature coverage should be an array")
            .iter()
            .any(|feature| feature["feature_name"] == "overseas_flow_persistence_4w"),
        "enhanced diagnostics should keep observation-only capital-source metrics out of training coverage"
    );
    let zero_variance_features =
        enhanced_diagnostic_json["correlation_summary"]["zero_variance_features"]
            .as_array()
            .expect("zero variance features should be an array")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>();
    assert!(
        !zero_variance_features
            .iter()
            .any(|feature_name| *feature_name == "overseas_flow_persistence_4w"),
        "observation-only capital-source metrics should stay out of training diagnostics"
    );
}

#[test]
fn security_scorecard_training_rejects_capital_source_run_without_explicit_capital_flow_runtime_root()
 {
    // 2026-04-22 CST: Added because scheme 2 splits artifact output paths from
    // capital-flow data roots and must fail closed when the data root is omitted.
    // Purpose: prevent the trainer from guessing that artifact runtime paths also hold source data.
    let runtime_db_path =
        create_test_runtime_db("security_scorecard_training_missing_capital_flow_runtime_root");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scorecard_training_runtime");

    let request = json!({
        "tool": "security_scorecard_training",
        "args": {
            "created_at": "2026-04-22T16:00:00+08:00",
            "artifact_runtime_root": runtime_root.to_string_lossy(),
            "market_scope": "GLOBAL",
            "instrument_scope": "INDEX",
            "instrument_subscope": "nikkei_index",
            "symbol_list": ["NK225.IDX"],
            "market_symbol": "NK225.IDX",
            "sector_symbol": "NK225.IDX",
            "horizon_days": 10,
            "target_head": "direction_head",
            "train_range": "2016-04-20..2025-09-30",
            "valid_range": "2025-10-01..2025-12-31",
            "test_range": "2026-01-01..2026-04-20",
            "feature_set_version": "security_feature_snapshot.v1",
            "label_definition_version": "security_forward_outcome.v1",
            "capital_source_feature_mode": "nikkei_jpx_mof_v1"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(&request.to_string(), &runtime_db_path, &[]);

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .expect("error should be a string")
            .contains("capital_flow_runtime_root"),
        "output={output}"
    );
}

#[test]
fn security_scorecard_training_drops_fundamental_and_disclosure_features_in_phase_a_contract() {
    // 2026-04-10 CST: 这里先补真实训练失败的复现测试，原因是当前真实训练在基本面数值缺失时会把 null 透传进训练特征，
    // 直接触发 `revenue_yoy_pct` 等 numeric feature 构建失败。
    // 目的：先锁住“证据层输出给训练的 numeric feature 必须始终保持 numeric 合同”这个统一标准，再修正式实现。
    let runtime_db_path =
        create_test_runtime_db("security_scorecard_training_missing_numeric_metrics");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scorecard_training_runtime");
    let fixture_dir =
        create_training_fixture_dir("security_scorecard_training_missing_numeric_metrics");

    let stock_up_csv = fixture_dir.join("stock_up.csv");
    let stock_down_csv = fixture_dir.join("stock_down.csv");
    let market_csv = fixture_dir.join("market.csv");
    let sector_csv = fixture_dir.join("sector.csv");

    fs::write(
        &stock_up_csv,
        build_trend_rows(420, 100.0, 0.9, 1.0).join("\n"),
    )
    .expect("upward symbol csv should be written");
    fs::write(
        &stock_down_csv,
        build_trend_rows(420, 120.0, -0.7, 1.0).join("\n"),
    )
    .expect("downward symbol csv should be written");
    fs::write(
        &market_csv,
        build_trend_rows(420, 3200.0, 2.5, 5.0).join("\n"),
    )
    .expect("market csv should be written");
    fs::write(
        &sector_csv,
        build_trend_rows(420, 980.0, 1.4, 2.0).join("\n"),
    )
    .expect("sector csv should be written");

    import_history_csv(&runtime_db_path, &stock_up_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &stock_down_csv, "600000.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 200 OK",
            r#"[]"#,
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
        "tool": "security_scorecard_training",
        "args": {
            "created_at": "2026-04-10T18:00:00+08:00",
            "training_runtime_root": runtime_root.to_string_lossy(),
            "market_scope": "A_SHARE",
            "instrument_scope": "EQUITY",
            "symbol_list": ["601916.SH", "600000.SH"],
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "horizon_days": 10,
            "target_head": "direction_head",
            "train_range": "2025-03-01..2025-08-31",
            "valid_range": "2025-09-01..2025-11-30",
            "test_range": "2025-12-01..2026-01-31",
            "feature_set_version": "security_feature_snapshot.v1",
            "label_definition_version": "security_forward_outcome.v1"
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

    assert_eq!(output["status"], "ok", "output={output}");
    let artifact_path = PathBuf::from(
        output["data"]["artifact_path"]
            .as_str()
            .expect("artifact path should exist"),
    );
    let artifact_json: Value =
        serde_json::from_slice(&fs::read(&artifact_path).expect("artifact should be readable"))
            .expect("artifact should be valid json");
    // 2026-04-21 CST: Updated because Phase A removes the whole company-data family
    // instead of keeping null-normalized placeholders inside index-focused training.
    // Purpose: make the new contract fail if removed equity-only fields leak back in.
    for feature_name in [
        "profit_signal",
        "fundamental_status",
        "disclosure_status",
        "announcement_count",
        "disclosure_risk_keyword_count",
        "has_risk_warning_notice",
        "hard_risk_score",
        "negative_attention_score",
        "positive_support_score",
        "event_net_impact_score",
        "revenue_yoy_pct",
        "net_profit_yoy_pct",
        "roe_pct",
        "shareholder_return_status",
        "quality_bucket",
    ] {
        assert!(
            !artifact_json["features"]
                .as_array()
                .expect("features should be an array")
                .iter()
                .any(|feature| feature["feature_name"] == feature_name),
            "artifact should exclude removed Phase-A feature `{feature_name}`"
        );
    }
}

#[test]
fn security_scorecard_training_tolerates_unseen_categorical_values_in_diagnostic_splits() {
    // 2026-04-17 CST: Added because the real 40-name rerun now re-encodes train/valid/test during
    // diagnostics, and the previous implementation crashed once test-only disclosure states first
    // appeared after train-only binning.
    // Purpose: lock the formal regression where `has_risk_warning_notice=true` exists only in the
    // test split, while the training run must still complete and expose an explicit fallback bin.
    let runtime_db_path =
        create_test_runtime_db("security_scorecard_training_unseen_categorical_diagnostics");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scorecard_training_runtime");
    let fixture_dir =
        create_training_fixture_dir("security_scorecard_training_unseen_categorical_diagnostics");

    let stock_up_csv = fixture_dir.join("stock_up.csv");
    let stock_down_csv = fixture_dir.join("stock_down.csv");
    let market_csv = fixture_dir.join("market.csv");
    let sector_csv = fixture_dir.join("sector.csv");

    fs::write(
        &stock_up_csv,
        build_trend_rows(420, 100.0, 0.9, 1.0).join("\n"),
    )
    .expect("upward symbol csv should be written");
    fs::write(
        &stock_down_csv,
        build_trend_rows(420, 120.0, -0.7, 1.0).join("\n"),
    )
    .expect("downward symbol csv should be written");
    fs::write(
        &market_csv,
        build_trend_rows(420, 3200.0, 2.5, 5.0).join("\n"),
    )
    .expect("market csv should be written");
    fs::write(
        &sector_csv,
        build_trend_rows(420, 980.0, 1.4, 2.0).join("\n"),
    )
    .expect("sector csv should be written");

    import_history_csv(&runtime_db_path, &stock_up_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &stock_down_csv, "600000.SH");
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
                    "list":[]
                }
            }"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_scorecard_training",
        "args": {
            "created_at": "2026-04-17T18:10:00+08:00",
            "training_runtime_root": runtime_root.to_string_lossy(),
            "market_scope": "A_SHARE",
            "instrument_scope": "EQUITY",
            "symbol_list": ["601916.SH", "600000.SH"],
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "horizon_days": 10,
            "target_head": "direction_head",
            "train_range": "2025-03-01..2025-08-31",
            "valid_range": "2025-09-01..2025-11-30",
            "test_range": "2025-12-01..2026-01-31",
            "feature_set_version": "security_feature_snapshot.v1",
            "label_definition_version": "security_forward_outcome.v1"
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

    assert_eq!(output["status"], "ok", "output={output}");
    let artifact_path = PathBuf::from(
        output["data"]["artifact_path"]
            .as_str()
            .expect("artifact path should exist"),
    );
    let artifact_json: Value =
        serde_json::from_slice(&fs::read(&artifact_path).expect("artifact should be readable"))
            .expect("artifact should be valid json");
    // 2026-04-21 CST: Updated because the next cleanup step removes integrated_stance too,
    // while the unseen-category guard itself must remain alive on a retained categorical feature.
    // Purpose: keep the diagnostic fallback contract independent from removed summary fields.
    let market_regime_feature = artifact_json["features"]
        .as_array()
        .expect("features should be an array")
        .iter()
        .find(|feature| feature["feature_name"] == "market_regime")
        .expect("market regime feature should exist");
    assert!(
        market_regime_feature["bins"]
            .as_array()
            .expect("market regime bins should be an array")
            .iter()
            .any(|bin| bin["bin_label"] == "__unseen__"),
        "artifact should expose the governed unseen categorical fallback bin"
    );
}

fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_scorecard_training_fixture"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path.to_path_buf(),
        &[],
    );
    assert_eq!(output["status"], "ok");
}

fn seed_capital_flow_history_for_training(
    runtime_db_path: &Path,
    start_date: NaiveDate,
    week_count: usize,
) {
    // 2026-04-22 CST: Added because the Nikkei capital-source A/B training test
    // needs governed weekly JPX/MOF history before the enhanced contract can expose new features.
    // Purpose: build one deterministic long-span weekly flow history in the same runtime as training.
    let mut records = Vec::new();
    for week_offset in 0..week_count {
        let metric_date = (start_date + Duration::days((week_offset * 7) as i64))
            .format("%Y-%m-%d")
            .to_string();
        // 2026-04-23 CST: Updated because the enhanced Nikkei capital-source
        // training contract now consumes 4-week aggregated persistence factors.
        // Reason: the old even/odd fixture made every rolling 4-week aggregate
        // constant and falsely downgraded the new factors to zero-variance.
        // Purpose: keep the seeded weekly flow history deterministic while still
        // producing non-constant 4-week capital-source features for diagnostics.
        let cycle = week_offset % 6;
        let structural_shift = (week_offset / 12) as f64 * 0.45;
        let foreign_value = match cycle {
            0 => 18.0,
            1 => 9.5,
            2 => 22.0,
            3 => 7.0,
            4 => 15.0,
            _ => 11.5,
        } + structural_shift;
        let trust_bank_value = match cycle {
            0 => 4.0,
            1 => 7.5,
            2 => 5.5,
            3 => 8.0,
            4 => 6.0,
            _ => 9.0,
        } + structural_shift * 0.35;
        let investment_trust_value = match cycle {
            0 => 2.0,
            1 => 5.0,
            2 => 3.0,
            3 => 6.5,
            4 => 4.0,
            _ => 7.0,
        } + structural_shift * 0.25;
        let individual_value = -match cycle {
            0 => 7.0,
            1 => 12.5,
            2 => 8.5,
            3 => 14.0,
            4 => 9.5,
            _ => 11.0,
        } - structural_shift * 0.2;
        let mof_value = match cycle {
            0 => 24.0,
            1 => 12.0,
            2 => 28.0,
            3 => 10.0,
            4 => 19.0,
            _ => 14.0,
        } + structural_shift * 0.6;

        for (series_key, value) in [
            ("foreign_net_buy", foreign_value),
            ("trust_bank_net_buy", trust_bank_value),
            ("investment_trust_net_buy", investment_trust_value),
            ("individual_net_buy", individual_value),
        ] {
            records.push(json!({
                "dataset_id": "jpx_weekly_investor_type",
                "frequency": "weekly",
                "metric_date": metric_date,
                "series_key": series_key,
                "value": value,
                "source": "training_fixture_capital_flow",
                "payload_json": {
                    "reason": "nikkei_capital_source_ab_test",
                    "week_offset": week_offset
                }
            }));
        }
        records.push(json!({
            "dataset_id": "mof_weekly_cross_border",
            "frequency": "weekly",
            "metric_date": metric_date,
            "series_key": "foreign_japan_equity_net",
            "value": mof_value,
            "source": "training_fixture_capital_flow",
            "payload_json": {
                "reason": "nikkei_capital_source_ab_test",
                "week_offset": week_offset
            }
        }));
    }

    let request = json!({
        "tool": "security_capital_flow_backfill",
        "args": {
            "batch_id": "security_scorecard_training_nikkei_capital_source_seed",
            "created_at": "2026-04-22T13:30:00+08:00",
            "records": records
        }
    });
    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path.to_path_buf(),
        &[],
    );
    assert_eq!(output["status"], "ok", "output={output}");
}

// 2026-04-09 CST: 这里构造可控趋势样本，原因是训练测试需要同时覆盖正负标签，但不希望把失败点散到复杂行情生成上；
// 目的：用可手算的上升/下降路径稳定生成 direction_head 样本，便于后续训练、回归与调试。
fn build_trend_rows(
    day_count: usize,
    start_close: f64,
    daily_drift: f64,
    intraday_padding: f64,
) -> Vec<String> {
    let mut rows = vec!["trade_date,open,high,low,close,adj_close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("seed date should be valid");
    let mut close = start_close;

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let next_close = (close + daily_drift).max(1.0);
        let open = close;
        let high = open.max(next_close) + intraday_padding;
        // 2026-04-09 CST: 这里把训练夹具的 low 下限改成“动态正数底”，原因是固定 0.10 会让长下跌样本后段低点完全失去波动，
        // 目的：保留 CSV 夹具在极端下跌段的低点变化，避免 RSRS 窗口被测试数据人为压成分母为 0 的假退化形态。
        let dynamic_low_floor = (start_close * 0.01).max(0.05) + offset as f64 * 0.001;
        let low = (open.min(next_close) - intraday_padding).max(dynamic_low_floor);
        let volume = 900_000 + offset as i64 * 8_000;
        rows.push(format!(
            "{},{open:.2},{high:.2},{low:.2},{next_close:.2},{next_close:.2},{volume}",
            trade_date.format("%Y-%m-%d")
        ));
        close = next_close;
    }

    rows
}

fn build_nikkei_mixed_regime_rows(day_count: usize, start_close: f64) -> Vec<String> {
    // 2026-04-20 CST: Added because Task 2 needs one deterministic Nikkei-like
    // index fixture with both positive and negative future windows.
    // Purpose: keep the red test focused on governed index training instead of random label scarcity.
    let mut rows = vec!["trade_date,open,high,low,close,adj_close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("seed date should be valid");
    let mut close = start_close;

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let daily_drift = match offset {
            0..=219 => -118.0,
            _ => 136.0,
        };
        let next_close = (close + daily_drift).max(12000.0);
        let open = close;
        let high = open.max(next_close) + 48.0 + (offset % 5) as f64 * 3.0;
        let low = open.min(next_close) - 52.0 - (offset % 7) as f64 * 2.0;
        let volume = 1_200_000 + (offset % 9) as i64 * 35_000;
        rows.push(format!(
            "{},{open:.2},{high:.2},{low:.2},{next_close:.2},{next_close:.2},{volume}",
            trade_date.format("%Y-%m-%d")
        ));
        close = next_close;
    }

    rows
}

fn build_nikkei_decade_rows(day_count: usize, start_close: f64) -> Vec<String> {
    // 2026-04-20 CST: Added because the 10-year Nikkei contract needs one deterministic
    // regime-switching fixture with enough density for monthly-like rolling sampling.
    // Purpose: keep decade-scale sample growth and post-cutoff holdout checks reproducible in CI.
    let mut rows = vec!["trade_date,open,high,low,close,adj_close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2015, 8, 8).expect("seed date should be valid");
    let mut close = start_close;

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let phase = (offset / 63) % 6;
        let daily_drift = match phase {
            0 => 52.0,
            1 => -41.0,
            2 => 33.0,
            3 => -56.0,
            4 => 61.0,
            _ => -29.0,
        } + ((offset % 5) as f64 - 2.0) * 3.5;
        let next_close = (close + daily_drift).max(9000.0);
        let open = close;
        let high = open.max(next_close) + 44.0 + (offset % 7) as f64 * 2.0;
        let low = open.min(next_close) - 46.0 - (offset % 11) as f64 * 1.5;
        let volume = 1_100_000 + (offset % 13) as i64 * 28_000;
        rows.push(format!(
            "{},{open:.2},{high:.2},{low:.2},{next_close:.2},{next_close:.2},{volume}",
            trade_date.format("%Y-%m-%d")
        ));
        close = next_close;
    }

    rows
}

fn build_nikkei_futures_decade_rows(day_count: usize, start_close: f64) -> Vec<String> {
    let mut rows = vec!["trade_date,open,high,low,close,adj_close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2015, 8, 8).expect("seed date should be valid");
    let mut close = start_close;

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let phase = (offset / 63) % 6;
        let daily_drift = match phase {
            0 => 58.0,
            1 => -36.0,
            2 => 44.0,
            3 => -61.0,
            4 => 69.0,
            _ => -24.0,
        } + ((offset % 7) as f64 - 3.0) * 4.0;
        let basis_drift = match offset % 44 {
            0..=14 => 22.0,
            15..=29 => -12.0,
            _ => 16.0,
        };
        let next_close = (close + daily_drift + basis_drift).max(9000.0);
        let open = close;
        let high = open.max(next_close) + 62.0 + (offset % 9) as f64 * 2.5;
        let low = open.min(next_close) - 64.0 - (offset % 13) as f64 * 2.0;
        let volume = 970_000 + (offset % 17) as i64 * 24_000;
        rows.push(format!(
            "{},{open:.2},{high:.2},{low:.2},{next_close:.2},{next_close:.2},{volume}",
            trade_date.format("%Y-%m-%d")
        ));
        close = next_close;
    }

    rows
}

fn parse_fixture_rows(csv_rows: &[String]) -> Vec<StockHistoryRow> {
    csv_rows
        .iter()
        .skip(1)
        .map(|line| {
            let columns = line.split(',').collect::<Vec<_>>();
            StockHistoryRow {
                trade_date: columns[0].to_string(),
                open: columns[1].parse().expect("open should parse"),
                high: columns[2].parse().expect("high should parse"),
                low: columns[3].parse().expect("low should parse"),
                close: columns[4].parse().expect("close should parse"),
                adj_close: columns[5].parse().expect("adj close should parse"),
                volume: columns[6].parse().expect("volume should parse"),
            }
        })
        .collect()
}

#[test]
fn build_trend_rows_keeps_low_series_variable_in_downtrend_fixture() {
    // 2026-04-09 CST: 这里先补训练夹具退化根因的失败测试，原因是 Task 5 的真实问题不是训练器本身，
    // 而是下跌样本在构造 CSV 时把 low 长时间钉死到同一楼板价，进一步把 RSRS 窗口压成分母为 0。
    // 目的：先锁住“下跌夹具也必须保留低点变化”这个约束，避免后续再用表面通过的训练结果掩盖数据构造缺陷。
    let rows = build_trend_rows(420, 120.0, -0.7, 1.0);
    let collapsed_low_count = rows
        .iter()
        .skip(1)
        .filter(|line| line.split(',').nth(3) == Some("0.10"))
        .count();

    assert_eq!(
        collapsed_low_count, 0,
        "下跌夹具不应该把 low 压成重复的 0.10 楼板价"
    );
}

#[test]
fn weekly_price_aggregation_emits_distribution_quantiles_for_nikkei_training() {
    let spot_rows = parse_fixture_rows(&build_nikkei_decade_rows(220, 16800.0));
    let futures_rows = parse_fixture_rows(&build_nikkei_futures_decade_rows(220, 16840.0));
    let weekly_rows = debug_build_weekly_price_feature_rows(&spot_rows, Some(&futures_rows))
        .expect("weekly aggregation should succeed");
    let feature_names = weekly_rows
        .first()
        .expect("weekly feature rows should not be empty")
        .feature_values
        .keys()
        .cloned()
        .collect::<Vec<_>>();

    assert!(
        feature_names
            .iter()
            .any(|name| name == &"weekly_spot_return_min"),
        "weekly spot quantile features should exist in artifact"
    );
    assert!(
        feature_names
            .iter()
            .any(|name| name == &"weekly_spot_return_p10"),
        "weekly spot p10 feature should exist in artifact"
    );
    assert!(
        feature_names
            .iter()
            .any(|name| name == &"weekly_spot_return_p25"),
        "weekly spot p25 feature should exist in artifact"
    );
    assert!(
        feature_names
            .iter()
            .any(|name| name == &"weekly_spot_return_p50"),
        "weekly spot p50 feature should exist in artifact"
    );
    assert!(
        feature_names
            .iter()
            .any(|name| name == &"weekly_spot_return_p75"),
        "weekly spot p75 feature should exist in artifact"
    );
    assert!(
        feature_names
            .iter()
            .any(|name| name == &"weekly_spot_return_p90"),
        "weekly spot p90 feature should exist in artifact"
    );
    assert!(
        feature_names
            .iter()
            .any(|name| name == &"weekly_spot_return_max"),
        "weekly spot max feature should exist in artifact"
    );
    assert!(
        feature_names
            .iter()
            .any(|name| name == &"weekly_futures_return_p50"),
        "weekly futures median feature should exist in weekly row"
    );
    assert!(
        feature_names
            .iter()
            .any(|name| name == &"weekly_basis_pct_p50"),
        "weekly basis median feature should exist in weekly row"
    );
    assert!(
        feature_names
            .iter()
            .any(|name| name == &"weekly_futures_relative_strength_p50"),
        "weekly relative strength median feature should exist in weekly row"
    );
    assert!(
        feature_names
            .iter()
            .any(|name| name == &"weekly_spot_close_position"),
        "weekly close position path feature should exist in weekly row"
    );
    assert!(
        feature_names
            .iter()
            .any(|name| name == &"weekly_spot_drawdown"),
        "weekly drawdown path feature should exist in weekly row"
    );
    assert!(
        feature_names
            .iter()
            .any(|name| name == &"weekly_spot_rebound"),
        "weekly rebound path feature should exist in weekly row"
    );
    assert!(
        feature_names
            .iter()
            .any(|name| name == &"weekly_volume_ratio_4w"),
        "weekly volume ratio feature should exist in weekly row"
    );
    assert!(
        feature_names
            .iter()
            .any(|name| name == &"weekly_up_day_volume_share"),
        "weekly up-day volume share feature should exist in weekly row"
    );
    assert!(
        feature_names
            .iter()
            .any(|name| name == &"weekly_down_day_volume_share"),
        "weekly down-day volume share feature should exist in weekly row"
    );
    assert!(
        feature_names
            .iter()
            .any(|name| name == &"weekly_volume_price_confirmation"),
        "weekly volume-price confirmation feature should exist in weekly row"
    );
    let volume_ratio_values = weekly_rows
        .iter()
        .filter_map(|row| row.feature_values.get("weekly_volume_ratio_4w").copied())
        .collect::<Vec<_>>();
    let up_share_values = weekly_rows
        .iter()
        .filter_map(|row| row.feature_values.get("weekly_up_day_volume_share").copied())
        .collect::<Vec<_>>();
    let confirmation_values = weekly_rows
        .iter()
        .filter_map(|row| row.feature_values.get("weekly_volume_price_confirmation").copied())
        .collect::<Vec<_>>();
    assert!(
        volume_ratio_values
            .windows(2)
            .any(|pair| (pair[0] - pair[1]).abs() > f64::EPSILON),
        "weekly volume ratio should vary when futures volume varies even if index spot volume is unavailable"
    );
    assert!(
        up_share_values
            .windows(2)
            .any(|pair| (pair[0] - pair[1]).abs() > f64::EPSILON),
        "weekly up-day volume share should vary when futures volume varies"
    );
    assert!(
        confirmation_values
            .iter()
            .any(|value| value.abs() > f64::EPSILON),
        "weekly volume-price confirmation should produce non-zero states when futures volume confirms a move"
    );
}

#[test]
fn weekly_anchor_calendar_uses_governed_weekly_dates_and_respects_cutoff() {
    let runtime_db_path = create_test_runtime_db("security_scorecard_training_weekly_anchor");
    seed_capital_flow_history_for_training(
        &runtime_db_path,
        NaiveDate::from_ymd_opt(2015, 8, 14).expect("seed date should be valid"),
        80,
    );
    let capital_flow_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .to_string_lossy()
        .to_string();
    let governed_dates = debug_load_governed_weekly_observation_dates(
        &capital_flow_root,
        "2015-08-14",
        "2016-02-28",
    )
    .expect("governed weekly dates should load");
    let spot_rows = parse_fixture_rows(&build_nikkei_decade_rows(260, 16800.0));
    let anchors =
        debug_build_weekly_anchor_dates(&spot_rows, &governed_dates, "2015-08-14", "2016-02-28")
            .expect("weekly anchors should be built");

    assert!(!anchors.is_empty(), "weekly anchors should not be empty");
    assert_eq!(
        anchors.first().expect("first anchor should exist"),
        "2015-08-14",
        "weekly anchors should start from governed weekly dates"
    );
    assert!(
        anchors.windows(2).all(|window| window[0] < window[1]),
        "weekly anchors should stay ordered"
    );
    assert!(
        anchors
            .iter()
            .all(|date: &String| date.as_str() <= "2016-02-28"),
        "weekly anchors should stop at the approved cutoff"
    );
}

#[test]
fn weekly_rolling_split_plan_uses_24w_1w_1w_stride_contract() {
    let anchors = (0..30)
        .map(|offset| {
            (NaiveDate::from_ymd_opt(2025, 1, 3).expect("seed date should be valid")
                + Duration::days((offset * 7) as i64))
            .format("%Y-%m-%d")
            .to_string()
        })
        .collect::<Vec<_>>();

    let plan = debug_build_weekly_rolling_split_plan(&anchors, 24, 1, 1, 1)
        .expect("weekly rolling split plan should build");

    assert_eq!(
        plan.len(),
        5,
        "30 weekly anchors should yield 5 rolling windows"
    );
    let first_window = plan.first().expect("first rolling window should exist");
    assert_eq!(
        first_window.train_anchor_dates.len(),
        24,
        "first rolling window should keep exactly 24 training weeks"
    );
    assert_eq!(
        first_window.valid_anchor_dates,
        vec!["2025-06-20".to_string()],
        "the first validation slice should consume one week after the 24 training weeks"
    );
    assert_eq!(
        first_window.test_anchor_dates,
        vec!["2025-06-27".to_string()],
        "the first test slice should consume the next week"
    );
    let second_window = plan.get(1).expect("second rolling window should exist");
    assert_eq!(
        second_window
            .train_anchor_dates
            .first()
            .expect("second train window should have first date"),
        "2025-01-10",
        "rolling stride should advance by exactly one week"
    );
}
