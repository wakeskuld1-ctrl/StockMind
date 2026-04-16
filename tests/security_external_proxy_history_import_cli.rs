mod common;

use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{create_test_runtime_db, run_cli_with_json, run_cli_with_json_and_runtime};

// 2026-04-12 CST: Create one fixture file root for proxy-history import tests,
// because the new formal import tool should prove it can consume real files
// before later external data batches are wired in.
// Purpose: keep import fixtures isolated and reproducible.
fn create_proxy_fixture_file(prefix: &str, file_name: &str, body: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let root = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_external_proxy_history_import")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&root).expect("proxy fixture root should exist");
    let file_path = root.join(file_name);
    fs::write(&file_path, body).expect("proxy fixture file should be written");
    file_path
}

#[test]
fn tool_catalog_includes_security_external_proxy_history_import() {
    let output = run_cli_with_json("");

    // 2026-04-12 CST: Lock discoverability for the governed proxy-history import tool,
    // because Historical Data Phase 1 should expose a public file-based bridge into
    // dated ETF proxy history before stronger live data feeds land.
    // Purpose: ensure CLI and Skills can find the new import capability.
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_external_proxy_history_import")
    );
}

#[test]
fn security_external_proxy_history_import_reads_csv_and_persists_records() {
    let runtime_db_path = create_test_runtime_db("security_external_proxy_history_import");
    let csv_path = create_proxy_fixture_file(
        "security_external_proxy_history_import",
        "proxy_history.csv",
        // 2026-04-12 CST: Keep each CSV row aligned to the shared header width,
        // because the formal import tool should validate a real, standards-shaped
        // batch instead of relying on ambiguous column shifting.
        // Purpose: prove governed proxy-history import works with reproducible files.
        "symbol,as_of_date,instrument_subscope,yield_curve_proxy_status,yield_curve_slope_delta_bp_5d,funding_liquidity_proxy_status,funding_liquidity_spread_delta_bp_5d,gold_spot_proxy_status,gold_spot_proxy_return_5d,usd_index_proxy_status,usd_index_proxy_return_5d,real_rate_proxy_status,real_rate_proxy_delta_bp_5d,fx_proxy_status,fx_return_5d,overseas_market_proxy_status,overseas_market_return_5d,market_session_gap_status,market_session_gap_days,etf_fund_flow_proxy_status,etf_fund_flow_5d,premium_discount_proxy_status,premium_discount_pct,benchmark_relative_strength_status,benchmark_relative_return_5d\n511010.SH,2026-04-10,treasury_etf,manual_bound,-6.5,manual_bound,3.0,,,,,,,,,,,,,,,,,,\n518880.SH,2026-04-10,gold_etf,,,,,manual_bound,0.024,manual_bound,-0.011,manual_bound,-4.0,,,,,,,,,,,\n513500.SH,2026-04-10,cross_border_etf,,,,,,,,,,,manual_bound,0.008,manual_bound,0.015,manual_bound,1.0,,,,,,\n512800.SH,2026-04-10,equity_etf,,,,,,,,,,,,,,,,,manual_bound,0.032,manual_bound,0.0015,manual_bound,0.006\n",
    );
    let request = json!({
        "tool": "security_external_proxy_history_import",
        "args": {
            "batch_id": "external-proxy-import-2026-04-12-a",
            "created_at": "2026-04-12T23:20:00+08:00",
            "file_path": csv_path.to_string_lossy()
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-12 CST: Lock the file-based proxy-history import contract,
    // because ETF external proxy history needs one governed bridge for real
    // batches before dedicated live crawlers are hardened.
    // Purpose: require the new tool to parse real files and persist all dated proxy rows.
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["document_type"],
        "security_external_proxy_history_import_result"
    );
    assert_eq!(output["data"]["imported_record_count"], 4);
    assert_eq!(output["data"]["covered_symbol_count"], 4);
    assert_eq!(
        output["data"]["source_file_path"],
        csv_path.to_string_lossy().to_string()
    );
}
