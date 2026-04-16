mod common;

use serde_json::json;

use crate::common::{create_test_runtime_db, run_cli_with_json, run_cli_with_json_and_runtime};

#[test]
fn tool_catalog_includes_security_external_proxy_backfill() {
    let output = run_cli_with_json("");

    // 2026-04-11 CST: Add a catalog red test for the governed historical proxy
    // backfill tool, because P4 must expose dated proxy import as a first-class
    // stock tool instead of another hidden runtime helper.
    // Purpose: lock discoverability before implementation so later Skill and CLI
    // flows can rely on one formal entry point for dated proxy history.
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_external_proxy_backfill")
    );
}

#[test]
fn security_external_proxy_backfill_persists_dated_records_for_all_etf_subscopes() {
    let runtime_db_path = create_test_runtime_db("security_external_proxy_backfill");
    let request = json!({
        "tool": "security_external_proxy_backfill",
        "args": {
            "batch_id": "backfill-batch-2026-04-11-a",
            "created_at": "2026-04-11T23:50:00+08:00",
            "records": [
                {
                    "symbol": "511010.SH",
                    "as_of_date": "2026-04-10",
                    "instrument_subscope": "treasury_etf",
                    "external_proxy_inputs": {
                        "yield_curve_proxy_status": "manual_bound",
                        "yield_curve_slope_delta_bp_5d": -6.5,
                        "funding_liquidity_proxy_status": "manual_bound",
                        "funding_liquidity_spread_delta_bp_5d": 3.0
                    }
                },
                {
                    "symbol": "518880.SH",
                    "as_of_date": "2026-04-10",
                    "instrument_subscope": "gold_etf",
                    "external_proxy_inputs": {
                        "gold_spot_proxy_status": "manual_bound",
                        "gold_spot_proxy_return_5d": 0.024,
                        "usd_index_proxy_status": "manual_bound",
                        "usd_index_proxy_return_5d": -0.011
                    }
                },
                {
                    "symbol": "513800.SH",
                    "as_of_date": "2026-04-10",
                    "instrument_subscope": "cross_border_etf",
                    "external_proxy_inputs": {
                        "fx_proxy_status": "manual_bound",
                        "fx_return_5d": 0.008,
                        "overseas_market_proxy_status": "manual_bound",
                        "overseas_market_return_5d": 0.015,
                        "market_session_gap_status": "manual_bound",
                        "market_session_gap_days": 1.0
                    }
                },
                {
                    "symbol": "512800.SH",
                    "as_of_date": "2026-04-10",
                    "instrument_subscope": "equity_etf",
                    "external_proxy_inputs": {
                        "etf_fund_flow_proxy_status": "manual_bound",
                        "etf_fund_flow_5d": 0.032,
                        "premium_discount_proxy_status": "manual_bound",
                        "premium_discount_pct": 0.0015,
                        "benchmark_relative_strength_status": "manual_bound",
                        "benchmark_relative_return_5d": 0.006
                    }
                }
            ]
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-11 CST: Add a red test for dated proxy-record persistence across all
    // ETF sub-pools, because P4 must stop relying only on current live manual proxy
    // inputs when training samples look back by symbol and date.
    // Purpose: force the new tool to persist auditable per-date proxy records with
    // a deterministic batch ref before sample-join implementation begins.
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["document_type"],
        "security_external_proxy_backfill_result"
    );
    assert_eq!(output["data"]["imported_record_count"], 4);
    assert_eq!(
        output["data"]["batch_ref"],
        "external-proxy-backfill:backfill-batch-2026-04-11-a"
    );
    assert_eq!(output["data"]["covered_symbol_count"], 4);
    assert_eq!(output["data"]["coverage_tier"], "governed_backfill_ready");
    assert_eq!(
        output["data"]["covered_dates"]
            .as_array()
            .expect("covered dates should be array")
            .len(),
        1
    );
    assert_eq!(
        output["data"]["covered_proxy_fields"]
            .as_array()
            .expect("covered proxy fields should be array")
            .len(),
        20
    );
    assert!(
        output["data"]["storage_path"].as_str().is_some(),
        "storage path should be returned for audit"
    );
    assert!(
        output["data"]["backfill_result_path"].as_str().is_some(),
        "backfill result path should be returned for later history expansion linkage"
    );
}
