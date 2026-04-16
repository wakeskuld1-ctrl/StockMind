mod common;

use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{create_test_runtime_db, run_cli_with_json, run_cli_with_json_and_runtime};

// 2026-04-11 CST: Add a promotion fixture workspace, because P5 champion-governance
// tests need persisted evaluation documents rather than temporary inline JSON.
// Purpose: keep promotion decisions reproducible and isolated from other tests.
fn create_promotion_fixture_dir(prefix: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_model_promotion")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("promotion fixture dir should exist");
    fixture_dir
}

// 2026-04-11 CST: Write a minimal shadow-evaluation fixture, because P5 promotion
// should consume the governed evaluation output instead of ad-hoc flags.
// Purpose: lock promotion decisions to one explicit evaluation contract.
fn write_shadow_evaluation_fixture(
    fixture_dir: &Path,
    file_name: &str,
    recommended_model_grade: &str,
) -> PathBuf {
    let path = fixture_dir.join(file_name);
    let payload = json!({
        "shadow_evaluation_id": "shadow-evaluation:A_SHARE:ETF:treasury_etf:2026-04-11:v1",
        "contract_version": "security_shadow_evaluation.v1",
        "document_type": "security_shadow_evaluation",
        "created_at": "2026-04-11T19:50:00+08:00",
        "market_scope": "A_SHARE",
        "instrument_scope": "ETF",
        "instrument_subscope": "treasury_etf",
        "model_registry_ref": "registry-shadow-eval-20260411",
        "sample_readiness_status": "sample_ready",
        "class_balance_status": "class_balance_ready",
        "path_event_coverage_status": "path_event_ready",
        "proxy_coverage_status": "history_coverage_ready",
        "production_readiness": "shadow_candidate_ready",
        "recommended_model_grade": recommended_model_grade,
        "shadow_observation_count": 1,
        "shadow_consistency_status": "shadow_consistent",
        "shadow_window_count": 1,
        "oot_stability_status": "oot_thin",
        "window_consistency_status": "window_observation_thin",
        "promotion_blockers": [],
        "promotion_evidence_notes": [],
        "evaluation_notes": [
            "promotion evaluation fixture"
        ]
    });
    fs::write(
        &path,
        serde_json::to_vec_pretty(&payload).expect("shadow evaluation payload should serialize"),
    )
    .expect("shadow evaluation fixture should be written");
    path
}

// 2026-04-11 CST: Write a minimal registry fixture for promotion tests, because
// P5 promotion decisions should still anchor back to a governed model registry.
// Purpose: avoid treating promotion as a free-floating decision without model identity.
fn write_registry_fixture(fixture_dir: &Path, file_name: &str) -> PathBuf {
    let path = fixture_dir.join(file_name);
    let payload = json!({
        "registry_id": "registry-promotion-20260411",
        "contract_version": "security_scorecard_model_registry.v1",
        "document_type": "security_scorecard_model_registry",
        "model_id": "a_share_etf_treasury_etf_10d_direction_head",
        "market_scope": "A_SHARE",
        "instrument_scope": "ETF",
        "instrument_subscope": "treasury_etf",
        "horizon_days": 10,
        "target_head": "direction_head",
        "model_version": "candidate_20260411_shadow",
        "status": "candidate",
        "model_grade": "candidate",
        "grade_reason": "awaiting_shadow_evaluation",
        "training_window": "2024-01-01..2024-12-31",
        "validation_window": "2025-01-01..2025-06-30",
        "oot_window": "2025-07-01..2025-12-31",
        "artifact_path": "tests/runtime_fixtures/security_model_promotion/fake_artifact.json",
        "artifact_sha256": "fixture-sha",
        "metrics_summary_json": {
            "readiness_assessment": {
                "production_readiness": "shadow_candidate_ready"
            }
        },
        "published_at": "2026-04-11T20:00:00+08:00"
    });
    fs::write(
        &path,
        serde_json::to_vec_pretty(&payload).expect("registry payload should serialize"),
    )
    .expect("promotion registry fixture should be written");
    path
}

#[test]
fn tool_catalog_includes_security_model_promotion() {
    let output = run_cli_with_json("");

    // 2026-04-11 CST: Add a catalog red test for model promotion, because P5
    // should expose governed grade transitions as a first-class stock tool.
    // Purpose: keep promotion discoverable for CLI and Skill orchestration.
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_model_promotion")
    );
}

#[test]
fn security_model_promotion_emits_shadow_grade_decision_document() {
    let runtime_db_path = create_test_runtime_db("security_model_promotion_shadow");
    let fixture_dir = create_promotion_fixture_dir("security_model_promotion_shadow");
    let registry_path = write_registry_fixture(&fixture_dir, "registry.json");
    let shadow_evaluation_path =
        write_shadow_evaluation_fixture(&fixture_dir, "shadow_evaluation.json", "shadow");

    let request = json!({
        "tool": "security_model_promotion",
        "args": {
            "created_at": "2026-04-11T20:00:00+08:00",
            "model_registry_path": registry_path.to_string_lossy(),
            "shadow_evaluation_path": shadow_evaluation_path.to_string_lossy(),
            "requested_model_grade": "shadow"
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-11 CST: Add a red test for governed model-promotion decisions,
    // because P5 needs one persistent promotion document that can be attached to
    // approval and package governance later.
    // Purpose: force promotion results to publish stable grade and rationale fields.
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["promotion"]["document_type"],
        "security_model_promotion"
    );
    assert_eq!(
        output["data"]["promotion"]["approved_model_grade"],
        "shadow"
    );
    assert_eq!(
        output["data"]["promotion"]["promotion_decision"],
        "promote_to_shadow"
    );
    assert!(
        output["data"]["promotion_path"].as_str().is_some(),
        "promotion path should be returned"
    );
}

#[test]
fn security_model_promotion_only_upgrades_to_champion_after_stable_shadow_observations() {
    let runtime_db_path = create_test_runtime_db("security_model_promotion_champion");
    let fixture_dir = create_promotion_fixture_dir("security_model_promotion_champion");
    let registry_path = write_registry_fixture(&fixture_dir, "registry.json");
    let champion_shadow_evaluation_path = fixture_dir.join("shadow_evaluation_champion.json");
    let payload = json!({
        "shadow_evaluation_id": "shadow-evaluation:A_SHARE:ETF:treasury_etf:2026-04-11:v1",
        "contract_version": "security_shadow_evaluation.v1",
        "document_type": "security_shadow_evaluation",
        "created_at": "2026-04-11T21:30:00+08:00",
        "market_scope": "A_SHARE",
        "instrument_scope": "ETF",
        "instrument_subscope": "treasury_etf",
        "model_registry_ref": "registry-promotion-20260411",
        "sample_readiness_status": "sample_ready",
        "class_balance_status": "class_balance_ready",
        "path_event_coverage_status": "path_event_ready",
        "proxy_coverage_status": "history_coverage_ready",
        "production_readiness": "champion_candidate_ready",
        "recommended_model_grade": "champion",
        "shadow_observation_count": 3,
        "shadow_consistency_status": "shadow_consistent",
        "shadow_window_count": 3,
        "oot_stability_status": "oot_stable",
        "window_consistency_status": "window_consistent",
        "promotion_blockers": [],
        "promotion_evidence_notes": [],
        "evaluation_notes": [
            "champion promotion evaluation fixture"
        ]
    });
    fs::write(
        &champion_shadow_evaluation_path,
        serde_json::to_vec_pretty(&payload)
            .expect("champion shadow evaluation payload should serialize"),
    )
    .expect("champion shadow evaluation fixture should be written");

    let request = json!({
        "tool": "security_model_promotion",
        "args": {
            "created_at": "2026-04-11T21:30:00+08:00",
            "model_registry_path": registry_path.to_string_lossy(),
            "shadow_evaluation_path": champion_shadow_evaluation_path.to_string_lossy(),
            "requested_model_grade": "champion"
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-11 CST: Add a red test for the hardened champion gate, because P6
    // should only allow champion when repeated shadow observations already proved
    // stable and blocker-free.
    // Purpose: prevent champion from being granted off a single shadow-ready snapshot.
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["promotion"]["approved_model_grade"],
        "champion"
    );
    assert_eq!(
        output["data"]["promotion"]["promotion_decision"],
        "promote_to_champion"
    );
}

#[test]
fn security_model_promotion_rejects_champion_when_shadow_observations_are_still_thin() {
    let runtime_db_path = create_test_runtime_db("security_model_promotion_champion_blocked");
    let fixture_dir = create_promotion_fixture_dir("security_model_promotion_champion_blocked");
    let registry_path = write_registry_fixture(&fixture_dir, "registry.json");
    let blocked_shadow_evaluation_path = fixture_dir.join("shadow_evaluation_blocked.json");
    let payload = json!({
        "shadow_evaluation_id": "shadow-evaluation:A_SHARE:ETF:treasury_etf:2026-04-11:v1",
        "contract_version": "security_shadow_evaluation.v1",
        "document_type": "security_shadow_evaluation",
        "created_at": "2026-04-11T21:35:00+08:00",
        "market_scope": "A_SHARE",
        "instrument_scope": "ETF",
        "instrument_subscope": "treasury_etf",
        "model_registry_ref": "registry-promotion-20260411",
        "sample_readiness_status": "sample_ready",
        "class_balance_status": "class_balance_ready",
        "path_event_coverage_status": "path_event_ready",
        "proxy_coverage_status": "history_coverage_ready",
        "production_readiness": "champion_candidate_ready",
        "recommended_model_grade": "champion",
        "shadow_observation_count": 1,
        "shadow_consistency_status": "shadow_observation_thin",
        "shadow_window_count": 1,
        "oot_stability_status": "oot_thin",
        "window_consistency_status": "window_observation_thin",
        "promotion_blockers": [
            "champion gate requires at least three governed shadow observations"
        ],
        "promotion_evidence_notes": [
            "champion gate requires at least two stable comparison windows"
        ],
        "evaluation_notes": [
            "blocked champion promotion fixture"
        ]
    });
    fs::write(
        &blocked_shadow_evaluation_path,
        serde_json::to_vec_pretty(&payload)
            .expect("blocked shadow evaluation payload should serialize"),
    )
    .expect("blocked shadow evaluation fixture should be written");

    let request = json!({
        "tool": "security_model_promotion",
        "args": {
            "created_at": "2026-04-11T21:35:00+08:00",
            "model_registry_path": registry_path.to_string_lossy(),
            "shadow_evaluation_path": blocked_shadow_evaluation_path.to_string_lossy(),
            "requested_model_grade": "champion"
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-11 CST: Add a negative red test for hardened champion gating,
    // because P6 should block champion when shadow evidence is still thin even if
    // the requested grade says champion.
    // Purpose: lock observation-count and blocker checks into promotion logic.
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["promotion"]["approved_model_grade"],
        "candidate"
    );
    assert_eq!(
        output["data"]["promotion"]["promotion_decision"],
        "retain_current_grade"
    );
}

#[test]
fn security_model_promotion_rejects_champion_when_oot_window_evidence_is_still_thin() {
    let runtime_db_path = create_test_runtime_db("security_model_promotion_oot_blocked");
    let fixture_dir = create_promotion_fixture_dir("security_model_promotion_oot_blocked");
    let registry_path = write_registry_fixture(&fixture_dir, "registry.json");
    let blocked_shadow_evaluation_path = fixture_dir.join("shadow_evaluation_oot_blocked.json");
    let payload = json!({
        "shadow_evaluation_id": "shadow-evaluation:A_SHARE:ETF:treasury_etf:2026-04-11:v1",
        "contract_version": "security_shadow_evaluation.v1",
        "document_type": "security_shadow_evaluation",
        "created_at": "2026-04-11T22:45:00+08:00",
        "market_scope": "A_SHARE",
        "instrument_scope": "ETF",
        "instrument_subscope": "treasury_etf",
        "model_registry_ref": "registry-promotion-20260411",
        "sample_readiness_status": "sample_ready",
        "class_balance_status": "class_balance_ready",
        "path_event_coverage_status": "path_event_ready",
        "proxy_coverage_status": "history_coverage_ready",
        "production_readiness": "champion_candidate_ready",
        "recommended_model_grade": "champion",
        "shadow_observation_count": 3,
        "shadow_consistency_status": "shadow_consistent",
        "shadow_window_count": 1,
        "oot_stability_status": "oot_thin",
        "window_consistency_status": "window_observation_thin",
        "promotion_blockers": [],
        "promotion_evidence_notes": [
            "champion gate requires at least two stable comparison windows"
        ],
        "evaluation_notes": [
            "blocked champion promotion fixture due to thin oot windows"
        ]
    });
    fs::write(
        &blocked_shadow_evaluation_path,
        serde_json::to_vec_pretty(&payload)
            .expect("oot blocked shadow evaluation payload should serialize"),
    )
    .expect("oot blocked shadow evaluation fixture should be written");

    let request = json!({
        "tool": "security_model_promotion",
        "args": {
            "created_at": "2026-04-11T22:45:00+08:00",
            "model_registry_path": registry_path.to_string_lossy(),
            "shadow_evaluation_path": blocked_shadow_evaluation_path.to_string_lossy(),
            "requested_model_grade": "champion"
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-11 CST: Add a red test for OOT/window gating, because P7 should
    // block champion when repeated shadow observations exist but comparison-window
    // evidence is still too thin.
    // Purpose: lock the stronger champion gate to multi-window evidence.
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["promotion"]["approved_model_grade"],
        "candidate"
    );
    assert_eq!(
        output["data"]["promotion"]["promotion_decision"],
        "retain_current_grade"
    );
}
