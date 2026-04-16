mod common;

use serde_json::json;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{create_test_runtime_db, run_cli_with_json, run_cli_with_json_and_runtime};

// 2026-04-12 CST: Create a dedicated disclosure-history root, because the new
// governed announcement backfill tool must prove it can persist into an isolated
// runtime location before implementation begins.
// Purpose: keep persistence tests deterministic across reruns.
fn create_history_runtime_root(prefix: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_disclosure_history_backfill")
        .join(format!("{prefix}_{unique_suffix}"))
}

#[test]
fn tool_catalog_includes_security_disclosure_history_backfill() {
    let output = run_cli_with_json("");

    // 2026-04-12 CST: Lock announcement-history tool discovery first, because
    // stock disclosure history must become a public governed capability before
    // validation and replay can rely on it.
    // Purpose: ensure CLI and Skills can find the new backfill tool.
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_disclosure_history_backfill")
    );
}

#[test]
fn security_disclosure_history_backfill_persists_recent_announcements() {
    let runtime_db_path = create_test_runtime_db("security_disclosure_history_backfill");
    let history_root = create_history_runtime_root("security_disclosure_history_backfill");
    let request = json!({
        "tool": "security_disclosure_history_backfill",
        "args": {
            "batch_id": "disclosure-batch-2026-04-12-a",
            "created_at": "2026-04-12T22:15:00+08:00",
            "history_runtime_root": history_root.to_string_lossy(),
            "records": [
                {
                    "symbol": "601916.SH",
                    "published_at": "2026-03-28",
                    "title": "2025 Annual Report",
                    "article_code": "AN202603281234567890",
                    "category": "Periodic Report",
                    "source": "eastmoney_announcements"
                },
                {
                    "symbol": "601916.SH",
                    "published_at": "2026-03-28",
                    "title": "2025 Profit Distribution Plan",
                    "article_code": "AN202603281234567891",
                    "category": "Company Notice",
                    "source": "eastmoney_announcements"
                }
            ]
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-12 CST: Lock the first governed disclosure-history persistence
    // contract, because replayable information context must stop depending on
    // one-off live announcement fetches.
    // Purpose: require a stable result document and auditable persistence paths.
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["document_type"],
        "security_disclosure_history_backfill_result"
    );
    assert_eq!(output["data"]["imported_record_count"], 2);
    assert_eq!(output["data"]["covered_symbol_count"], 1);
    assert_eq!(
        output["data"]["covered_published_dates"]
            .as_array()
            .expect("covered published dates should be an array")
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
