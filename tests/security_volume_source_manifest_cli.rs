mod common;

use excel_skill::runtime::stock_history_store::{StockHistoryRow, StockHistoryStore};
use serde_json::json;

use crate::common::{create_test_runtime_db, run_cli_with_json, run_cli_with_json_and_runtime};

// 2026-04-25 CST: Added because Nikkei volume readiness now needs a formal manifest
// before model tuning can distinguish no-volume spot history from a short but usable proxy.
// Purpose: lock the public tool contract for governed volume-source inventory data.
#[test]
fn tool_catalog_includes_security_volume_source_manifest() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_volume_source_manifest")
    );
}

// 2026-04-25 CST: Added because the approved Scheme B requires a structured list of
// volume sources, not another implicit manual SQLite inspection step.
// Purpose: prove the manifest separates FRED no-volume spot rows from a short non-zero proxy.
#[test]
fn security_volume_source_manifest_reports_no_volume_and_short_proxy_sources() {
    let runtime_db_path = create_test_runtime_db("security_volume_source_manifest");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have a parent directory");
    let store = StockHistoryStore::new(runtime_root.join("stock_history.db"));

    store
        .import_rows(
            "NK225.IDX",
            "fred_nikkei225_manual_test",
            &build_rows(24, 0),
        )
        .expect("zero-volume spot rows should import");
    store
        .import_rows(
            "NK225_VOL.PROXY",
            "existing_yfinance_n225_volume_proxy_test",
            &build_rows(12, 1_800_000),
        )
        .expect("volume proxy rows should import");

    let request = json!({
        "tool": "security_volume_source_manifest",
        "args": {
            "instrument_symbol": "NK225.IDX",
            "volume_source_symbols": ["NK225.IDX", "NK225_VOL.PROXY", "NK225_MISSING.PROXY"],
            "minimum_effective_history_days": 20,
            "as_of_date": "2026-04-25"
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["document_type"],
        "security_volume_source_manifest"
    );
    assert_eq!(output["data"]["instrument_symbol"], "NK225.IDX");
    assert_eq!(
        output["data"]["summary"]["volume_source_count"], 3,
        "manifest should preserve all requested source symbols"
    );
    assert_eq!(output["data"]["summary"]["train_ready_source_count"], 0);
    assert_eq!(output["data"]["summary"]["usable_short_proxy_count"], 1);
    assert_eq!(output["data"]["summary"]["no_volume_source_count"], 1);
    assert_eq!(output["data"]["summary"]["missing_source_count"], 1);

    let sources = output["data"]["volume_sources"]
        .as_array()
        .expect("volume_sources should be an array");
    let spot = sources
        .iter()
        .find(|source| source["symbol"] == "NK225.IDX")
        .expect("spot source should exist");
    let proxy = sources
        .iter()
        .find(|source| source["symbol"] == "NK225_VOL.PROXY")
        .expect("proxy source should exist");
    let missing = sources
        .iter()
        .find(|source| source["symbol"] == "NK225_MISSING.PROXY")
        .expect("missing source should exist");

    assert_eq!(spot["coverage_status"], "no_volume");
    assert_eq!(spot["row_count"], 24);
    assert_eq!(spot["nonzero_volume_rows"], 0);
    assert_eq!(spot["nonzero_volume_ratio"], 0.0);
    assert_eq!(spot["source_names"][0], "fred_nikkei225_manual_test");

    assert_eq!(proxy["coverage_status"], "usable_short_proxy");
    assert_eq!(proxy["row_count"], 12);
    assert_eq!(proxy["nonzero_volume_rows"], 12);
    assert_eq!(proxy["zero_volume_rows"], 0);
    assert_eq!(proxy["nonzero_volume_ratio"], 1.0);
    assert_eq!(proxy["eligible_for_training"], false);
    assert_eq!(proxy["missing_days_to_effective_gate"], 8);
    assert_eq!(
        proxy["limitations"][0],
        "coverage_shorter_than_minimum_effective_history_days"
    );

    assert_eq!(missing["coverage_status"], "missing_history");
    assert_eq!(missing["row_count"], 0);
}

fn build_rows(day_count: usize, volume: i64) -> Vec<StockHistoryRow> {
    (0..day_count)
        .map(|day| StockHistoryRow {
            trade_date: format!("2026-01-{:02}", day + 1),
            open: 30_000.0 + day as f64,
            high: 30_050.0 + day as f64,
            low: 29_950.0 + day as f64,
            close: 30_020.0 + day as f64,
            adj_close: 30_020.0 + day as f64,
            volume,
        })
        .collect()
}
