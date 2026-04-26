mod common;

use excel_skill::runtime::stock_history_store::{StockHistoryRow, StockHistoryStore};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{create_test_runtime_db, run_cli_with_json, run_cli_with_json_and_runtime};

// 2026-04-25 CST: Added because Nikkei official turnover import must be discoverable
// before operators can use it as the free-source receiver for long-horizon volume proxy data.
// Purpose: lock the public tool catalog contract for official turnover ingestion.
#[test]
fn tool_catalog_includes_security_nikkei_turnover_import() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_nikkei_turnover_import")
    );
}

// 2026-04-25 CST: Added because the approved Scheme B receiver must convert official
// Total Trading Value rows into a governed turnover proxy without overwriting FRED prices.
// Purpose: prove import alignment, scaling, and manifest compatibility end to end.
#[test]
fn security_nikkei_turnover_import_builds_manifest_usable_turnover_proxy() {
    let runtime_db_path = create_test_runtime_db("security_nikkei_turnover_import");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have a parent directory");
    let store = StockHistoryStore::new(runtime_root.join("stock_history.db"));
    store
        .import_rows(
            "NK225.IDX",
            "fred_nikkei225_manual_test",
            &build_price_rows(),
        )
        .expect("price rows should import");

    let source_path = create_turnover_fixture(
        "security_nikkei_turnover_import",
        "Date,Total Trading Value(Tril.Yen)\nApr/01/2026,4.125\nApr/02/2026,3.875\nApr/03/2026,4.500\n",
    );

    let import_request = json!({
        "tool": "security_nikkei_turnover_import",
        "args": {
            "source_path": source_path.to_string_lossy(),
            "price_symbol": "NK225.IDX",
            "turnover_symbol": "NK225_TURNOVER.NIKKEI",
            "source": "nikkei_official_total_trading_value_fixture"
        }
    });

    let import_output =
        run_cli_with_json_and_runtime(&import_request.to_string(), &runtime_db_path);

    assert_eq!(import_output["status"], "ok", "output={import_output}");
    assert_eq!(
        import_output["data"]["document_type"],
        "security_nikkei_turnover_import_result"
    );
    assert_eq!(import_output["data"]["imported_row_count"], 3);
    assert_eq!(import_output["data"]["skipped_missing_price_count"], 0);
    assert_eq!(
        import_output["data"]["unit"],
        "total_trading_value_trillion_yen_scaled_1e6"
    );

    let imported_rows = store
        .load_rows_in_range("NK225_TURNOVER.NIKKEI", "2026-04-01", "2026-04-03")
        .expect("turnover proxy rows should load");
    assert_eq!(imported_rows.len(), 3);
    assert_eq!(imported_rows[0].close, 40_000.0);
    assert_eq!(imported_rows[0].volume, 4_125_000);
    assert_eq!(imported_rows[2].volume, 4_500_000);

    let manifest_request = json!({
        "tool": "security_volume_source_manifest",
        "args": {
            "instrument_symbol": "NK225.IDX",
            "volume_source_symbols": ["NK225_TURNOVER.NIKKEI"],
            "minimum_effective_history_days": 3,
            "as_of_date": "2026-04-25"
        }
    });
    let manifest_output =
        run_cli_with_json_and_runtime(&manifest_request.to_string(), &runtime_db_path);

    assert_eq!(manifest_output["status"], "ok", "output={manifest_output}");
    assert_eq!(
        manifest_output["data"]["volume_sources"][0]["coverage_status"],
        "train_ready_volume_proxy"
    );
    assert_eq!(
        manifest_output["data"]["volume_sources"][0]["source_names"][0],
        "nikkei_official_total_trading_value_fixture"
    );
}

fn create_turnover_fixture(prefix: &str, body: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_nikkei_turnover_import")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("turnover fixture directory should exist");
    let source_path = fixture_dir.join("nikkei_turnover.csv");
    fs::write(&source_path, body).expect("turnover fixture should be written");
    source_path
}

fn build_price_rows() -> Vec<StockHistoryRow> {
    ["2026-04-01", "2026-04-02", "2026-04-03"]
        .into_iter()
        .enumerate()
        .map(|(index, trade_date)| StockHistoryRow {
            trade_date: trade_date.to_string(),
            open: 39_900.0 + index as f64,
            high: 40_100.0 + index as f64,
            low: 39_800.0 + index as f64,
            close: 40_000.0 + index as f64,
            adj_close: 40_000.0 + index as f64,
            volume: 0,
        })
        .collect()
}
