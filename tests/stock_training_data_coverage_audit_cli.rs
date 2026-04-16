mod common;

use excel_skill::runtime::stock_history_store::{StockHistoryRow, StockHistoryStore};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{create_test_runtime_db, run_cli_with_json, run_cli_with_json_and_runtime};

// 2026-04-14 CST: Added because the new coverage audit needs one isolated config fixture
// and reusing the repository config would make assertions depend on unrelated symbols.
// Purpose: keep the coverage-audit CLI test deterministic and focused on readiness logic.
fn create_pool_config_fixture(prefix: &str, body: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("stock_training_data_coverage_audit")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("coverage config fixture directory should exist");
    let config_path = fixture_dir.join("pool.json");
    fs::write(&config_path, body).expect("coverage config fixture should be written");
    config_path
}

// 2026-04-14 CST: Added because the coverage audit contract must prove it is discoverable
// before later Skills rely on it for real-trading readiness checks.
// Purpose: prevent implementation-only drift where the audit exists but the tool catalog hides it.
#[test]
fn tool_catalog_includes_stock_training_data_coverage_audit() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "stock_training_data_coverage_audit")
    );
}

// 2026-04-14 CST: Added because stock-first real-trading readiness needs one formal verdict on
// which symbols are train-ready, which still need backfill, and which are fully missing.
// Purpose: lock the minimal end-to-end CLI contract for stock history coverage gating.
#[test]
fn stock_training_data_coverage_audit_reports_train_ready_backfill_needed_and_missing_symbols() {
    let runtime_db_path = create_test_runtime_db("stock_training_data_coverage_audit");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should always have a parent directory");
    let store = StockHistoryStore::new(runtime_root.join("stock_history.db"));

    let ready_rows = (0..760)
        .map(|day| StockHistoryRow {
            trade_date: format!("2023-01-{:04}", day + 1),
            open: 10.0,
            high: 10.5,
            low: 9.8,
            close: 10.2,
            adj_close: 10.2,
            volume: 1_000_000 + day as i64,
        })
        .collect::<Vec<_>>();
    let backfill_needed_rows = (0..320)
        .map(|day| StockHistoryRow {
            trade_date: format!("2024-02-{:04}", day + 1),
            open: 20.0,
            high: 20.5,
            low: 19.8,
            close: 20.1,
            adj_close: 20.1,
            volume: 2_000_000 + day as i64,
        })
        .collect::<Vec<_>>();

    store
        .import_rows("600000.SH", "fixture", &ready_rows)
        .expect("ready rows should import");
    store
        .import_rows("600036.SH", "fixture", &backfill_needed_rows)
        .expect("backfill-needed rows should import");

    let pool_config_path = create_pool_config_fixture(
        "stock_training_data_coverage_audit",
        r#"{
  "meta": { "version": "test_pool_v1" },
  "market_scope": "A_SHARE",
  "instrument_scope": "EQUITY",
  "readiness_gates": {
    "minimum_symbol_count": 1,
    "minimum_industry_bucket_count": 1,
    "minimum_effective_history_days_per_symbol": 750,
    "hard_floor_history_days_per_symbol": 200
  },
  "pools": [
    {
      "pool_id": "bank_core",
      "sector_proxy_symbol": "512800.SH",
      "equity_symbols": ["600000.SH", "600036.SH", "601398.SH"]
    }
  ]
}"#,
    );

    let request = json!({
        "tool": "stock_training_data_coverage_audit",
        "args": {
            "pool_config_path": pool_config_path.to_string_lossy(),
            "as_of_date": "2026-04-14"
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["document_type"],
        "stock_training_data_coverage_audit_result"
    );
    assert_eq!(output["data"]["pool_version"], "test_pool_v1");
    assert_eq!(output["data"]["summary"]["total_symbols"], 3);
    assert_eq!(output["data"]["summary"]["training_ready_symbols"], 1);
    assert_eq!(output["data"]["summary"]["hard_floor_pass_symbols"], 2);
    assert_eq!(output["data"]["summary"]["missing_history_symbols"], 1);
    assert_eq!(output["data"]["summary"]["training_pool_ready"], true);

    let coverage = output["data"]["symbol_coverage"]
        .as_array()
        .expect("symbol coverage should be an array");
    let ready_symbol = coverage
        .iter()
        .find(|item| item["symbol"] == "600000.SH")
        .expect("ready symbol should exist");
    let backfill_symbol = coverage
        .iter()
        .find(|item| item["symbol"] == "600036.SH")
        .expect("backfill-needed symbol should exist");
    let missing_symbol = coverage
        .iter()
        .find(|item| item["symbol"] == "601398.SH")
        .expect("missing symbol should exist");

    assert_eq!(ready_symbol["coverage_status"], "train_ready");
    assert_eq!(ready_symbol["eligible_for_training"], true);
    assert_eq!(ready_symbol["history_days"], 760);

    assert_eq!(backfill_symbol["coverage_status"], "backfill_needed");
    assert_eq!(backfill_symbol["eligible_for_training"], false);
    assert_eq!(backfill_symbol["meets_hard_floor_history_gate"], true);
    assert_eq!(backfill_symbol["missing_days_to_effective_gate"], 430);

    assert_eq!(missing_symbol["coverage_status"], "missing_history");
    assert_eq!(missing_symbol["history_days"], 0);
    assert_eq!(missing_symbol["missing_days_to_hard_floor_gate"], 200);
}
