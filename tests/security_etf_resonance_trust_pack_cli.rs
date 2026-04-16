mod common;

use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{
    create_test_runtime_db, run_cli_with_json, run_cli_with_json_runtime_and_envs,
};

// 2026-04-15 CST: Added because the new ETF trust-pack needs an isolated CSV fixture helper.
// Reason: the trust-pack replay test must control one deterministic ETF path without touching shared runtime data.
// Purpose: keep the replay sample and the current-verdict sample in one local fixture directory.
fn create_stock_history_csv(prefix: &str, file_name: &str, rows: &[String]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_etf_resonance_trust_pack")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("etf trust pack fixture dir should exist");

    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n")).expect("etf trust pack csv should be written");
    csv_path
}

#[test]
fn tool_catalog_includes_security_etf_resonance_trust_pack() {
    let output = run_cli_with_json("");

    // 2026-04-15 CST: Added because the trust-pack must be discoverable as a first-class stock tool.
    // Reason: if the catalog does not expose it, Skill and CLI cannot use the feature as an official chain step.
    // Purpose: lock public discovery before implementation details start changing.
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_etf_resonance_trust_pack")
    );
}

#[test]
fn security_etf_resonance_trust_pack_returns_current_verdict_and_replay_summary() {
    let runtime_db_path = create_test_runtime_db("security_etf_resonance_trust_pack_ready");
    let external_proxy_db_path = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("security_external_proxy.db");

    let etf_csv = create_stock_history_csv(
        "security_etf_resonance_trust_pack_ready",
        "159866.csv",
        &build_etf_rows(),
    );
    import_history_csv(&runtime_db_path, &etf_csv, "159866.SZ");

    let backfill_request = json!({
        "tool": "security_external_proxy_backfill",
        "args": {
            "batch_id": "etf-trust-pack-159866",
            "created_at": "2026-04-15T09:30:00+08:00",
            "records": [
                {
                    "symbol": "159866.SZ",
                    "as_of_date": "2026-04-01",
                    "instrument_subscope": "equity_etf",
                    "external_proxy_inputs": {
                        "premium_discount_proxy_status": "manual_bound",
                        "premium_discount_pct": 0.028,
                        "benchmark_relative_strength_status": "manual_bound",
                        "benchmark_relative_return_5d": 0.006,
                        "fx_proxy_status": "manual_bound",
                        "fx_return_5d": 0.002,
                        "overseas_market_proxy_status": "manual_bound",
                        "overseas_market_return_5d": 0.004
                    }
                },
                {
                    "symbol": "159866.SZ",
                    "as_of_date": "2026-04-02",
                    "instrument_subscope": "equity_etf",
                    "external_proxy_inputs": {
                        "premium_discount_proxy_status": "manual_bound",
                        "premium_discount_pct": 0.004,
                        "benchmark_relative_strength_status": "manual_bound",
                        "benchmark_relative_return_5d": 0.016,
                        "fx_proxy_status": "manual_bound",
                        "fx_return_5d": 0.008,
                        "overseas_market_proxy_status": "manual_bound",
                        "overseas_market_return_5d": 0.013
                    }
                },
                {
                    "symbol": "159866.SZ",
                    "as_of_date": "2026-04-03",
                    "instrument_subscope": "equity_etf",
                    "external_proxy_inputs": {
                        "premium_discount_proxy_status": "manual_bound",
                        "premium_discount_pct": 0.024,
                        "benchmark_relative_strength_status": "manual_bound",
                        "benchmark_relative_return_5d": 0.005,
                        "fx_proxy_status": "manual_bound",
                        "fx_return_5d": 0.001,
                        "overseas_market_proxy_status": "manual_bound",
                        "overseas_market_return_5d": 0.003
                    }
                },
                {
                    "symbol": "159866.SZ",
                    "as_of_date": "2026-04-10",
                    "instrument_subscope": "equity_etf",
                    "external_proxy_inputs": {
                        "premium_discount_proxy_status": "manual_bound",
                        "premium_discount_pct": 0.003,
                        "benchmark_relative_strength_status": "manual_bound",
                        "benchmark_relative_return_5d": 0.019,
                        "fx_proxy_status": "manual_bound",
                        "fx_return_5d": 0.009,
                        "overseas_market_proxy_status": "manual_bound",
                        "overseas_market_return_5d": 0.021
                    }
                }
            ]
        }
    });
    let backfill_output = run_cli_with_json_runtime_and_envs(
        &backfill_request.to_string(),
        &runtime_db_path,
        &[(
            "EXCEL_SKILL_EXTERNAL_PROXY_DB",
            external_proxy_db_path.to_string_lossy().to_string(),
        )],
    );
    assert_eq!(backfill_output["status"], "ok");

    let request = json!({
        "tool": "security_etf_resonance_trust_pack",
        "args": {
            "symbol": "159866.SZ",
            "as_of_date": "2026-04-12",
            "start_date": "2026-04-01",
            "end_date": "2026-04-03",
            "buy_premium_ceiling_pct": 0.01,
            "avoid_premium_ceiling_pct": 0.02,
            "benchmark_relative_return_floor_pct": 0.01,
            "fx_return_floor_pct": 0.005,
            "overseas_market_return_floor_pct": 0.01,
            "latest_case_limit": 3
        }
    });
    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &[(
            "EXCEL_SKILL_EXTERNAL_PROXY_DB",
            external_proxy_db_path.to_string_lossy().to_string(),
        )],
    );

    // 2026-04-15 CST: Added because the trust-pack must prove both "why now" and "whether it worked before".
    // Reason: the user explicitly rejected paragraph-style ETF advice without formal evidence and replay statistics.
    // Purpose: lock the minimal trust contract before we wire it into the wider decision chain.
    assert_eq!(
        output["status"], "ok",
        "unexpected trust-pack output: {output}"
    );
    assert_eq!(
        output["data"]["document_type"],
        "security_etf_resonance_trust_pack"
    );
    assert_eq!(output["data"]["current_analysis_date"], "2026-04-10");
    assert_eq!(
        output["data"]["current_proxy_snapshot"]["proxy_as_of_date"],
        "2026-04-10"
    );
    assert_eq!(
        output["data"]["underlying_market_assessment"]["verdict"],
        "supportive"
    );
    assert_eq!(output["data"]["fx_assessment"]["verdict"], "supportive");
    assert_eq!(output["data"]["premium_assessment"]["verdict"], "favorable");
    assert_eq!(
        output["data"]["current_resonance_verdict"]["status"],
        "triggered"
    );
    assert_eq!(
        output["data"]["current_resonance_verdict"]["gate_passed"],
        true
    );
    assert_eq!(output["data"]["replay_summary"]["sample_count"], 3);
    assert_eq!(output["data"]["replay_summary"]["eligible_sample_count"], 3);
    assert_eq!(
        output["data"]["replay_summary"]["triggered_sample_count"],
        1
    );
    assert_eq!(output["data"]["replay_summary"]["win_rate_5d"], json!(1.0));
    assert_eq!(output["data"]["replay_summary"]["win_rate_10d"], json!(1.0));
    assert_eq!(
        output["data"]["latest_triggered_cases"][0]["trade_date"],
        "2026-04-02"
    );
}

// 2026-04-15 CST: Added because the trust-pack replay needs a deterministic forward path.
// Reason: one handcrafted ETF close series is enough to prove replay math and current-date fallback in one test.
// Purpose: avoid hidden dependence on live market data during contract verification.
fn build_etf_rows() -> Vec<String> {
    [
        ("2026-03-30", 1.000),
        ("2026-03-31", 1.010),
        ("2026-04-01", 1.015),
        ("2026-04-02", 1.020),
        ("2026-04-03", 1.030),
        ("2026-04-06", 1.040),
        ("2026-04-07", 1.050),
        ("2026-04-08", 1.060),
        ("2026-04-09", 1.070),
        ("2026-04-10", 1.080),
        ("2026-04-13", 1.090),
        ("2026-04-14", 1.100),
        ("2026-04-15", 1.110),
        ("2026-04-16", 1.120),
        ("2026-04-17", 1.130),
    ]
    .into_iter()
    .map(|(trade_date, close)| {
        format!(
            "trade_date,open,high,low,close,adj_close,volume\n{trade_date},{close:.3},{close:.3},{close:.3},{close:.3},{close:.3},1000000"
        )
    })
    .enumerate()
    .flat_map(|(index, row)| {
        if index == 0 {
            row.lines().map(|line| line.to_string()).collect::<Vec<_>>()
        } else {
            row.lines().skip(1).map(|line| line.to_string()).collect::<Vec<_>>()
        }
    })
    .collect()
}

// 2026-04-15 CST: Added because the new test should reuse the same official stock import tool.
// Reason: importing through the public CLI keeps the trust-pack test aligned with real runtime behavior.
// Purpose: verify the feature on the formal path rather than by writing SQLite rows directly.
fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_etf_resonance_trust_pack_fixture"
        }
    });

    let output = crate::common::run_cli_with_json_and_runtime(
        &request.to_string(),
        &runtime_db_path.to_path_buf(),
    );
    assert_eq!(output["status"], "ok");
}
