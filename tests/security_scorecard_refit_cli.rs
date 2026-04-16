mod common;

use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{create_test_runtime_db, run_cli_with_json, run_cli_with_json_and_runtime};

// 2026-04-09 CST: 这里新增 scorecard refit CLI 测试夹具，原因是 Task 4 需要先把“离线重估主对象 + 注册表”的正式合同锁进红测；
// 目的：先验证 refit_run、candidate artifact 注册与 train/valid/test 窗口落盘，再做最小实现，避免后续训练入口反向改对象边界。
fn create_refit_fixture_dir(prefix: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_scorecard_refit")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("security scorecard refit fixture dir should exist");
    fixture_dir
}

// 2026-04-09 CST: 这里生成最小可用的 scorecard model artifact，原因是 refit 注册表必须对真实 artifact 路径和哈希做正式登记；
// 目的：让测试聚焦“治理对象登记”而不是训练过程本身，同时为后续 Task 5 训练入口复用同一类 artifact 合同打底。
fn write_scorecard_model_artifact(
    fixture_dir: &Path,
    file_name: &str,
    model_id: &str,
    model_version: &str,
) -> PathBuf {
    let artifact_path = fixture_dir.join(file_name);
    let payload = json!({
        "model_id": model_id,
        "model_version": model_version,
        "label_definition": "security_forward_outcome.v1",
        "training_window": "2022-01-01..2024-12-31",
        "oot_window": "2025-07-01..2025-12-31",
        "positive_label_definition": "positive_return_10d",
        "binning_version": "binning.v1",
        "coefficient_version": "coef.v1",
        "model_sha256": null,
        "intercept": 0.18,
        "base_score": 600.0,
        "features": [
            {
                "feature_name": "majority_count",
                "group_name": "Q",
                "bins": [
                    {
                        "bin_label": "majority_ge_4",
                        "min_inclusive": 4.0,
                        "max_exclusive": 8.0,
                        "woe": 0.42,
                        "logit_contribution": 0.15,
                        "points": 26.0
                    }
                ]
            }
        ]
    });
    fs::write(
        &artifact_path,
        serde_json::to_vec_pretty(&payload).expect("artifact payload should serialize"),
    )
    .expect("scorecard model artifact should be written");
    artifact_path
}

#[test]
fn tool_catalog_includes_security_scorecard_refit() {
    let output = run_cli_with_json("");

    // 2026-04-09 CST: 这里先锁 refit Tool 的可发现性，原因是如果 catalog 没有正式暴露它，后续训练治理链就没有一等入口；
    // 目的：确保 CLI / Skill / 后续自动化都能稳定发现“离线重估 + 注册”的正式主链能力。
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_scorecard_refit")
    );
}

#[test]
fn security_scorecard_refit_records_run_and_registers_candidate_artifact() {
    let runtime_db_path = create_test_runtime_db("security_scorecard_refit_ready");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scorecard_runtime");
    let fixture_dir = create_refit_fixture_dir("security_scorecard_refit_ready");
    let artifact_path = write_scorecard_model_artifact(
        &fixture_dir,
        "candidate_model.json",
        "a_share_equity_10d_direction",
        "candidate_20260409",
    );

    let request = json!({
        "tool": "security_scorecard_refit",
        "args": {
            "created_at": "2026-04-09T15:00:00+08:00",
            "refit_runtime_root": runtime_root.to_string_lossy(),
            "market_scope": "A_SHARE",
            "instrument_scope": "EQUITY",
            "feature_set_version": "security_feature_snapshot.v1",
            "label_definition_version": "security_forward_outcome.v1",
            "train_range": "2022-01-01..2024-12-31",
            "valid_range": "2025-01-01..2025-06-30",
            "test_range": "2025-07-01..2025-12-31",
            "candidate_artifact": {
                "model_id": "a_share_equity_10d_direction",
                "model_version": "candidate_20260409",
                "horizon_days": 10,
                "target_head": "direction_head",
                "status": "candidate",
                "artifact_path": artifact_path.to_string_lossy(),
                "metrics_summary_json": {
                    "auc": 0.61,
                    "brier_score": 0.18,
                    "top_bottom_spread": 0.07
                }
            }
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-09 CST: 这里锁定方案 B 的最小正式合同，原因是 Task 4 的目标不是训练模型，而是把 refit 治理对象正式落盘；
    // 目的：要求输出同时包含 refit_run、model_registry、窗口信息、artifact 哈希与持久化路径，确保后续 Task 5 能直接承接。
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["refit_run"]["document_type"],
        "security_scorecard_refit_run"
    );
    assert_eq!(output["data"]["refit_run"]["market_scope"], "A_SHARE");
    assert_eq!(output["data"]["refit_run"]["instrument_scope"], "EQUITY");
    assert_eq!(
        output["data"]["refit_run"]["feature_set_version"],
        "security_feature_snapshot.v1"
    );
    assert_eq!(
        output["data"]["refit_run"]["label_definition_version"],
        "security_forward_outcome.v1"
    );
    assert_eq!(
        output["data"]["refit_run"]["train_range"],
        "2022-01-01..2024-12-31"
    );
    assert_eq!(
        output["data"]["refit_run"]["valid_range"],
        "2025-01-01..2025-06-30"
    );
    assert_eq!(
        output["data"]["refit_run"]["test_range"],
        "2025-07-01..2025-12-31"
    );
    assert_eq!(
        output["data"]["refit_run"]["candidate_artifact_path"],
        Value::String(artifact_path.to_string_lossy().to_string())
    );
    assert!(
        output["data"]["refit_run"]["refit_run_id"]
            .as_str()
            .expect("refit run id should exist")
            .starts_with("refit-")
    );

    assert_eq!(
        output["data"]["model_registry"]["document_type"],
        "security_scorecard_model_registry"
    );
    assert_eq!(
        output["data"]["model_registry"]["model_id"],
        "a_share_equity_10d_direction"
    );
    assert_eq!(
        output["data"]["model_registry"]["model_version"],
        "candidate_20260409"
    );
    assert_eq!(output["data"]["model_registry"]["status"], "candidate");
    assert_eq!(output["data"]["model_registry"]["horizon_days"], 10);
    assert_eq!(
        output["data"]["model_registry"]["target_head"],
        "direction_head"
    );
    assert_eq!(
        output["data"]["model_registry"]["training_window"],
        "2022-01-01..2024-12-31"
    );
    assert_eq!(
        output["data"]["model_registry"]["validation_window"],
        "2025-01-01..2025-06-30"
    );
    assert_eq!(
        output["data"]["model_registry"]["oot_window"],
        "2025-07-01..2025-12-31"
    );
    assert_eq!(
        output["data"]["model_registry"]["artifact_path"],
        Value::String(artifact_path.to_string_lossy().to_string())
    );
    assert!(
        output["data"]["model_registry"]["artifact_sha256"]
            .as_str()
            .expect("artifact sha256 should exist")
            .len()
            >= 32
    );
    assert_eq!(
        output["data"]["model_registry"]["metrics_summary_json"]["auc"],
        0.61
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
    assert!(refit_run_path.exists());
    assert!(model_registry_path.exists());

    let persisted_refit_run: Value = serde_json::from_slice(
        &fs::read(&refit_run_path).expect("persisted refit run should be readable"),
    )
    .expect("persisted refit run should be valid json");
    assert_eq!(
        persisted_refit_run["document_type"],
        "security_scorecard_refit_run"
    );
    assert_eq!(
        persisted_refit_run["candidate_artifact_path"],
        Value::String(artifact_path.to_string_lossy().to_string())
    );

    let persisted_model_registry: Value = serde_json::from_slice(
        &fs::read(&model_registry_path).expect("persisted model registry should be readable"),
    )
    .expect("persisted model registry should be valid json");
    assert_eq!(
        persisted_model_registry["document_type"],
        "security_scorecard_model_registry"
    );
    assert_eq!(persisted_model_registry["status"], "candidate");
    assert_eq!(
        persisted_model_registry["artifact_path"],
        Value::String(artifact_path.to_string_lossy().to_string())
    );
}

#[test]
fn security_scorecard_refit_exposes_shadow_grade_when_promotion_decision_requests_it() {
    let runtime_db_path = create_test_runtime_db("security_scorecard_refit_shadow_grade");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("scorecard_runtime");
    let fixture_dir = create_refit_fixture_dir("security_scorecard_refit_shadow_grade");
    let artifact_path = write_scorecard_model_artifact(
        &fixture_dir,
        "candidate_model.json",
        "a_share_etf_treasury_etf_10d_direction",
        "candidate_20260411_shadow",
    );

    let request = json!({
        "tool": "security_scorecard_refit",
        "args": {
            "created_at": "2026-04-11T19:10:00+08:00",
            "refit_runtime_root": runtime_root.to_string_lossy(),
            "market_scope": "A_SHARE",
            "instrument_scope": "ETF",
            "feature_set_version": "security_feature_snapshot.v1",
            "label_definition_version": "security_forward_outcome.v1",
            "train_range": "2024-01-01..2024-12-31",
            "valid_range": "2025-01-01..2025-06-30",
            "test_range": "2025-07-01..2025-12-31",
            "candidate_artifact": {
                "model_id": "a_share_etf_treasury_etf_10d_direction",
                "model_version": "candidate_20260411_shadow",
                "horizon_days": 10,
                "target_head": "direction_head",
                "status": "candidate",
                "artifact_path": artifact_path.to_string_lossy(),
                "metrics_summary_json": {
                    "auc": 0.73,
                    "readiness_assessment": {
                        "minimum_sample_status": "sample_ready",
                        "class_balance_status": "class_balance_ready",
                        "path_event_coverage_status": "path_event_ready",
                        "production_readiness": "shadow_ready"
                    }
                }
            },
            "promotion_decision": "shadow"
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-11 CST: Add a red test for explicit model-grade semantics,
    // because P5 needs refit and registry outputs to distinguish research
    // candidates from shadow-grade artifacts before approval consumes them.
    // Purpose: force refit outputs to publish a governed grade instead of only a raw status.
    assert_eq!(output["status"], "ok");
    assert_eq!(output["data"]["refit_run"]["model_grade"], "shadow");
    assert_eq!(output["data"]["model_registry"]["model_grade"], "shadow");
    assert_eq!(
        output["data"]["model_registry"]["grade_reason"],
        "promoted_by_refit_decision"
    );
}
