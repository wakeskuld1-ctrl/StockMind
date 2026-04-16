mod common;

use serde_json::json;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{create_test_runtime_db, run_cli_with_json, run_cli_with_json_and_runtime};

// 2026-04-12 CST: Create a dedicated history fixture root, because the new
// governed stock fundamental backfill tool should prove it can persist outside
// the default runtime location before implementation lands.
// Purpose: keep persistence tests isolated and reproducible.
fn create_history_runtime_root(prefix: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_fundamental_history_backfill")
        .join(format!("{prefix}_{unique_suffix}"))
}

#[test]
fn tool_catalog_includes_security_fundamental_history_backfill() {
    let output = run_cli_with_json("");

    // 2026-04-12 CST: Lock tool discoverability first, because stock historical
    // fundamentals must become a first-class governed backfill path instead of
    // remaining implicit in one-off live fetches.
    // Purpose: ensure CLI and Skills can find the new tool once it exists.
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_fundamental_history_backfill")
    );
}

#[test]
fn security_fundamental_history_backfill_persists_latest_report_snapshot() {
    let runtime_db_path = create_test_runtime_db("security_fundamental_history_backfill");
    let history_root = create_history_runtime_root("security_fundamental_history_backfill");
    let request = json!({
        "tool": "security_fundamental_history_backfill",
        "args": {
            "batch_id": "fundamental-batch-2026-04-12-a",
            "created_at": "2026-04-12T22:10:00+08:00",
            "history_runtime_root": history_root.to_string_lossy(),
            "records": [
                {
                    "symbol": "601916.SH",
                    "report_period": "2025-12-31",
                    "notice_date": "2026-03-28",
                    "source": "eastmoney_financials",
                    "report_metrics": {
                        "revenue": 308227000000.0,
                        "revenue_yoy_pct": 8.37,
                        "net_profit": 11117000000.0,
                        "net_profit_yoy_pct": 9.31,
                        "roe_pct": 14.8
                    }
                }
            ]
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-12 CST: Lock the first governed stock fundamental-history
    // persistence contract, because historical financial context must become
    // replayable before validation and shadow governance can trust it.
    // Purpose: require a stable result document, storage path, and backfill path.
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["document_type"],
        "security_fundamental_history_backfill_result"
    );
    assert_eq!(output["data"]["imported_record_count"], 1);
    assert_eq!(output["data"]["covered_symbol_count"], 1);
    assert_eq!(
        output["data"]["covered_report_periods"]
            .as_array()
            .expect("covered report periods should be an array")
            .len(),
        1
    );
    assert!(
        output["data"]["storage_path"].as_str().is_some(),
        "storage path should be returned for audit"
    );
    assert!(
        output["data"]["backfill_result_path"].as_str().is_some(),
        "backfill result path should be returned for later validation linkage"
    );
}
