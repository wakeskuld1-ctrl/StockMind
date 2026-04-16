mod common;

use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

use crate::common::{create_test_runtime_db, run_cli_with_json, run_cli_with_json_and_runtime};

#[test]
fn tool_catalog_includes_security_history_expansion() {
    let output = run_cli_with_json("");

    // 2026-04-11 CST: Add a red test for the governed history-expansion tool,
    // because P5 starts by promoting historical proxy accumulation into a first-class
    // auditable capability instead of leaving it implicit in scattered runtime files.
    // Purpose: lock discoverability before implementation touches dispatcher wiring.
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_history_expansion")
    );
}

#[test]
fn security_history_expansion_persists_governed_record() {
    let runtime_db_path = create_test_runtime_db("security_history_expansion_ready");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("history_expansion_runtime");

    let request = json!({
        "tool": "security_history_expansion",
        "args": {
            "created_at": "2026-04-11T18:30:00+08:00",
            "history_runtime_root": runtime_root.to_string_lossy(),
            "market_scope": "A_SHARE",
            "instrument_scope": "ETF",
            "instrument_subscope": "treasury_etf",
            "proxy_fields": [
                "yield_curve_proxy_status",
                "yield_curve_slope_delta_bp_5d",
                "funding_liquidity_proxy_status",
                "funding_liquidity_spread_delta_bp_5d"
            ],
            "date_range": "2025-01-01..2025-12-31",
            "symbol_list": ["511010.SH", "511060.SH"],
            "coverage_summary": {
                "horizon_days": [10, 30, 60],
                "coverage_note": "backfilled treasury proxy history for promotion readiness"
            }
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-11 CST: Add a red test for the formal history-expansion record,
    // because P5 needs a deterministic governed document proving what historical
    // proxy coverage was expanded before any shadow/champion logic can trust it.
    // Purpose: require one persisted record with stable refs, paths, and coverage fields.
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["history_expansion"]["document_type"],
        "security_history_expansion"
    );
    assert_eq!(
        output["data"]["history_expansion"]["market_scope"],
        "A_SHARE"
    );
    assert_eq!(
        output["data"]["history_expansion"]["instrument_scope"],
        "ETF"
    );
    assert_eq!(
        output["data"]["history_expansion"]["instrument_subscope"],
        "treasury_etf"
    );
    assert_eq!(
        output["data"]["history_expansion"]["date_range"],
        "2025-01-01..2025-12-31"
    );
    assert_eq!(
        output["data"]["history_expansion"]["proxy_fields"]
            .as_array()
            .expect("proxy fields should be array")
            .len(),
        4
    );
    let record_path = PathBuf::from(
        output["data"]["history_expansion_path"]
            .as_str()
            .expect("history expansion path should exist"),
    );
    assert!(record_path.exists());

    let persisted: Value = serde_json::from_slice(
        &fs::read(&record_path).expect("history expansion record should be readable"),
    )
    .expect("history expansion record should be valid json");
    assert_eq!(persisted["document_type"], "security_history_expansion");
    assert_eq!(persisted["instrument_subscope"], "treasury_etf");
}

#[test]
fn security_history_expansion_exposes_standardized_readiness_coverage() {
    let runtime_db_path = create_test_runtime_db("security_history_expansion_standardized");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("history_expansion_runtime");

    let request = json!({
        "tool": "security_history_expansion",
        "args": {
            "created_at": "2026-04-11T21:10:00+08:00",
            "history_runtime_root": runtime_root.to_string_lossy(),
            "market_scope": "A_SHARE",
            "instrument_scope": "ETF",
            "instrument_subscope": "gold_etf",
            "proxy_fields": [
                "gold_spot_proxy_status",
                "gold_spot_return_5d",
                "usd_index_proxy_status",
                "usd_index_return_5d"
            ],
            "date_range": "2025-01-01..2025-12-31",
            "symbol_list": ["518880.SH"],
            "coverage_summary": {
                "horizon_days": [10, 30, 60],
                "coverage_note": "backfilled standardized gold proxy coverage"
            }
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-11 CST: Add a red test for standardized coverage output, because P6
    // needs history expansion to become a reusable readiness input instead of a
    // loose descriptive note.
    // Purpose: force the document to expose stable coverage-tier and proxy-coverage fields.
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["history_expansion"]["coverage_summary"]["coverage_tier"],
        "standardized_ready"
    );
    assert_eq!(
        output["data"]["history_expansion"]["coverage_summary"]["shadow_readiness_hint"],
        "shadow_coverage_ready"
    );
    assert_eq!(
        output["data"]["history_expansion"]["coverage_summary"]["champion_readiness_hint"],
        "champion_coverage_ready"
    );
    assert_eq!(
        output["data"]["history_expansion"]["coverage_summary"]["proxy_field_coverage"]
            .as_array()
            .expect("proxy field coverage should be array")
            .len(),
        4
    );
}

#[test]
fn security_history_expansion_consumes_governed_backfill_results() {
    let runtime_db_path = create_test_runtime_db("security_history_expansion_backfill_link");
    let backfill_request = json!({
        "tool": "security_external_proxy_backfill",
        "args": {
            "batch_id": "backfill-batch-2026-04-11-history-link",
            "created_at": "2026-04-11T22:10:00+08:00",
            "records": [
                {
                    "symbol": "511010.SH",
                    "as_of_date": "2026-04-09",
                    "instrument_subscope": "treasury_etf",
                    "external_proxy_inputs": {
                        "yield_curve_proxy_status": "manual_bound",
                        "yield_curve_slope_delta_bp_5d": -5.5
                    }
                },
                {
                    "symbol": "511010.SH",
                    "as_of_date": "2026-04-10",
                    "instrument_subscope": "treasury_etf",
                    "external_proxy_inputs": {
                        "yield_curve_proxy_status": "manual_bound",
                        "yield_curve_slope_delta_bp_5d": -6.0,
                        "funding_liquidity_proxy_status": "manual_bound",
                        "funding_liquidity_spread_delta_bp_5d": 2.5
                    }
                }
            ]
        }
    });
    let backfill_output =
        run_cli_with_json_and_runtime(&backfill_request.to_string(), &runtime_db_path);
    let backfill_result_path = backfill_output["data"]["backfill_result_path"]
        .as_str()
        .expect("backfill result path should exist");

    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("history_expansion_runtime");
    let request = json!({
        "tool": "security_history_expansion",
        "args": {
            "created_at": "2026-04-11T22:20:00+08:00",
            "history_runtime_root": runtime_root.to_string_lossy(),
            "market_scope": "A_SHARE",
            "instrument_scope": "ETF",
            "instrument_subscope": "treasury_etf",
            "proxy_fields": [
                "yield_curve_proxy_status",
                "yield_curve_slope_delta_bp_5d",
                "funding_liquidity_proxy_status",
                "funding_liquidity_spread_delta_bp_5d"
            ],
            "date_range": "2026-04-09..2026-04-10",
            "symbol_list": ["511010.SH"],
            "backfill_result_paths": [backfill_result_path],
            "coverage_summary": {
                "horizon_days": [10, 30],
                "coverage_note": "consume governed backfill result for history expansion"
            }
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-11 CST: Add a red test for governed backfill-result consumption,
    // because P7 needs history expansion to read actual imported proxy evidence
    // instead of relying only on free-form operator notes.
    // Purpose: force history expansion to retain batch refs, covered dates, and
    // imported record counts from formal backfill results.
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["history_expansion"]["coverage_summary"]["consumed_backfill_batch_refs"]
            .as_array()
            .expect("consumed backfill batch refs should be array")
            .len(),
        1
    );
    assert_eq!(
        output["data"]["history_expansion"]["coverage_summary"]["covered_dates"]
            .as_array()
            .expect("covered dates should be array")
            .len(),
        2
    );
    assert_eq!(
        output["data"]["history_expansion"]["coverage_summary"]["imported_record_count"],
        2
    );
}
