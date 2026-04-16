mod common;

use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{create_test_runtime_db, run_cli_with_json, run_cli_with_json_and_runtime};

// 2026-04-11 CST: Add a dedicated shadow-evaluation fixture workspace, because P5
// needs governed evaluation documents to read persisted registry and history
// expansion records instead of relying on in-memory ad-hoc stubs.
// Purpose: keep shadow-governance tests isolated and reproducible across runs.
fn create_shadow_fixture_dir(prefix: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_shadow_evaluation")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("shadow evaluation fixture dir should exist");
    fixture_dir
}

// 2026-04-11 CST: Write a minimal registry fixture for shadow-governance tests,
// because P5 evaluation should consume the same persisted registry contract that
// refit and approval already use.
// Purpose: lock the evaluation tool to governed registry files instead of loose JSON.
fn write_registry_fixture(
    fixture_dir: &Path,
    file_name: &str,
    model_grade: &str,
    production_readiness: &str,
    test_auc: f64,
    test_accuracy: f64,
) -> PathBuf {
    let path = fixture_dir.join(file_name);
    let payload = json!({
        "registry_id": "registry-shadow-eval-20260411",
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
        "model_grade": model_grade,
        "grade_reason": "promoted_by_refit_decision",
        "training_window": "2024-01-01..2024-12-31",
        "validation_window": "2025-01-01..2025-06-30",
        "oot_window": "2025-07-01..2025-12-31",
        "artifact_path": "tests/runtime_fixtures/security_shadow_evaluation/fake_artifact.json",
        "artifact_sha256": "fixture-sha",
        "metrics_summary_json": {
            "readiness_assessment": {
                "minimum_sample_status": "sample_ready",
                "class_balance_status": "class_balance_ready",
                "path_event_coverage_status": "path_event_ready",
                "production_readiness": production_readiness,
                "notes": []
            },
            "test": {
                "auc": test_auc,
                "accuracy": test_accuracy
            }
        },
        "published_at": "2026-04-11T19:20:00+08:00"
    });
    fs::write(
        &path,
        serde_json::to_vec_pretty(&payload).expect("registry payload should serialize"),
    )
    .expect("registry fixture should be written");
    path
}

// 2026-04-11 CST: Write a governed history-expansion fixture, because P5 shadow
// evaluation should explicitly account for proxy-history coverage before promoting
// a model beyond plain candidate usage.
// Purpose: ensure the evaluation contract consumes auditable history-expansion files.
fn write_history_expansion_fixture(fixture_dir: &Path, file_name: &str) -> PathBuf {
    let path = fixture_dir.join(file_name);
    let payload = json!({
        "history_expansion_id": "history-expansion:A_SHARE:ETF:treasury_etf:2025-01-01_2025-12-31:v1",
        "contract_version": "security_history_expansion.v1",
        "document_type": "security_history_expansion",
        "created_at": "2026-04-11T19:30:00+08:00",
        "market_scope": "A_SHARE",
        "instrument_scope": "ETF",
        "instrument_subscope": "treasury_etf",
        "date_range": "2025-01-01..2025-12-31",
        "proxy_fields": [
            "yield_curve_proxy_status",
            "yield_curve_slope_delta_bp_5d",
            "funding_liquidity_proxy_status",
            "funding_liquidity_spread_delta_bp_5d"
        ],
        "symbol_list": ["511010.SH", "511060.SH"],
        "coverage_summary": {
            "horizon_days": [10, 30, 60],
            "coverage_note": "treasury history backfill available for shadow evaluation",
            "coverage_tier": "standardized_ready",
            "shadow_readiness_hint": "shadow_coverage_ready",
            "champion_readiness_hint": "champion_coverage_ready",
            "proxy_field_coverage": [
                {
                    "proxy_field": "yield_curve_proxy_status",
                    "coverage_status": "covered_in_expansion",
                    "covered_horizons": [10, 30, 60]
                }
            ]
        }
    });
    fs::write(
        &path,
        serde_json::to_vec_pretty(&payload).expect("history expansion payload should serialize"),
    )
    .expect("history expansion fixture should be written");
    path
}

// 2026-04-11 CST: Persist prior shadow evaluations, because P6 needs champion
// readiness to depend on repeated governed shadow observations rather than a
// single passing snapshot.
// Purpose: let the evaluation tool compute observation count and consistency status.
fn write_prior_shadow_evaluation_fixture(
    fixture_dir: &Path,
    file_name: &str,
    consistency_status: &str,
) -> PathBuf {
    let path = fixture_dir.join(file_name);
    let payload = json!({
        "shadow_evaluation_id": format!("shadow-evaluation-prior-{file_name}"),
        "contract_version": "security_shadow_evaluation.v1",
        "document_type": "security_shadow_evaluation",
        "created_at": "2026-04-10T10:00:00+08:00",
        "market_scope": "A_SHARE",
        "instrument_scope": "ETF",
        "instrument_subscope": "treasury_etf",
        "model_registry_ref": "registry-shadow-eval-20260411",
        "sample_readiness_status": "sample_ready",
        "class_balance_status": "class_balance_ready",
        "path_event_coverage_status": "path_event_ready",
        "proxy_coverage_status": "history_coverage_ready",
        "production_readiness": "champion_candidate_ready",
        "recommended_model_grade": "shadow",
        "shadow_observation_count": 1,
        "shadow_consistency_status": consistency_status,
        "promotion_blockers": [],
        "evaluation_notes": [
            "prior shadow observation fixture"
        ]
    });
    fs::write(
        &path,
        serde_json::to_vec_pretty(&payload)
            .expect("prior shadow evaluation payload should serialize"),
    )
    .expect("prior shadow evaluation fixture should be written");
    path
}

#[test]
fn tool_catalog_includes_security_shadow_evaluation() {
    let output = run_cli_with_json("");

    // 2026-04-11 CST: Add a catalog red test for shadow evaluation, because P5
    // governance should expose promotion-readiness review as a first-class stock tool.
    // Purpose: make CLI and Skill discovery stable before implementation lands.
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_shadow_evaluation")
    );
}

#[test]
fn security_shadow_evaluation_builds_governed_shadow_readiness_document() {
    let runtime_db_path = create_test_runtime_db("security_shadow_evaluation_ready");
    let fixture_dir = create_shadow_fixture_dir("security_shadow_evaluation_ready");
    let registry_path = write_registry_fixture(
        &fixture_dir,
        "registry.json",
        "shadow",
        "shadow_ready",
        0.71,
        0.68,
    );
    let history_expansion_path =
        write_history_expansion_fixture(&fixture_dir, "history_expansion.json");

    let request = json!({
        "tool": "security_shadow_evaluation",
        "args": {
            "created_at": "2026-04-11T19:40:00+08:00",
            "market_scope": "A_SHARE",
            "instrument_scope": "ETF",
            "instrument_subscope": "treasury_etf",
            "model_registry_path": registry_path.to_string_lossy(),
            "history_expansion_paths": [history_expansion_path.to_string_lossy()],
            "evaluation_notes": [
                "shadow governance review for treasury ETF candidate"
            ]
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-11 CST: Add a red test for governed shadow evaluation, because P5
    // needs one persistent document that summarizes readiness, proxy coverage, and
    // the next eligible grade.
    // Purpose: force the new tool to output a stable evaluation record before promotion.
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["shadow_evaluation"]["document_type"],
        "security_shadow_evaluation"
    );
    assert_eq!(
        output["data"]["shadow_evaluation"]["recommended_model_grade"],
        "shadow"
    );
    assert_eq!(
        output["data"]["shadow_evaluation"]["proxy_coverage_status"],
        "history_coverage_ready"
    );
    assert!(
        output["data"]["shadow_evaluation_path"].as_str().is_some(),
        "shadow evaluation path should be returned"
    );
}

#[test]
fn security_shadow_evaluation_tracks_repeated_shadow_observations_for_champion_readiness() {
    let runtime_db_path = create_test_runtime_db("security_shadow_evaluation_champion");
    let fixture_dir = create_shadow_fixture_dir("security_shadow_evaluation_champion");
    let registry_path = write_registry_fixture(
        &fixture_dir,
        "registry.json",
        "shadow",
        "champion_candidate_ready",
        0.82,
        0.79,
    );
    let comparison_one = write_registry_fixture(
        &fixture_dir,
        "registry_window_one.json",
        "shadow",
        "champion_candidate_ready",
        0.79,
        0.73,
    );
    let comparison_two = write_registry_fixture(
        &fixture_dir,
        "registry_window_two.json",
        "shadow",
        "champion_candidate_ready",
        0.80,
        0.75,
    );
    let history_expansion_path =
        write_history_expansion_fixture(&fixture_dir, "history_expansion.json");
    let prior_one = write_prior_shadow_evaluation_fixture(
        &fixture_dir,
        "prior_shadow_one.json",
        "shadow_consistent",
    );
    let prior_two = write_prior_shadow_evaluation_fixture(
        &fixture_dir,
        "prior_shadow_two.json",
        "shadow_consistent",
    );

    let request = json!({
        "tool": "security_shadow_evaluation",
        "args": {
            "created_at": "2026-04-11T21:20:00+08:00",
            "market_scope": "A_SHARE",
            "instrument_scope": "ETF",
            "instrument_subscope": "treasury_etf",
            "model_registry_path": registry_path.to_string_lossy(),
            "comparison_model_registry_paths": [
                comparison_one.to_string_lossy(),
                comparison_two.to_string_lossy()
            ],
            "history_expansion_paths": [history_expansion_path.to_string_lossy()],
            "prior_shadow_evaluation_paths": [
                prior_one.to_string_lossy(),
                prior_two.to_string_lossy()
            ],
            "evaluation_notes": [
                "champion readiness review for repeated stable shadow observations"
            ]
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-11 CST: Add a red test for repeated shadow-observation governance,
    // because P6 should only recommend champion when the model stayed stable across
    // more than one governed shadow evaluation.
    // Purpose: force observation count, consistency, and blocker semantics into the document.
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["shadow_evaluation"]["shadow_observation_count"],
        3
    );
    assert_eq!(
        output["data"]["shadow_evaluation"]["shadow_consistency_status"],
        "shadow_consistent"
    );
    assert_eq!(
        output["data"]["shadow_evaluation"]["recommended_model_grade"],
        "champion"
    );
    assert_eq!(
        output["data"]["shadow_evaluation"]["shadow_window_count"],
        3
    );
    assert_eq!(
        output["data"]["shadow_evaluation"]["promotion_blockers"]
            .as_array()
            .expect("promotion blockers should be array")
            .len(),
        0
    );
    assert_eq!(
        output["data"]["shadow_evaluation"]["promotion_evidence_notes"]
            .as_array()
            .expect("promotion evidence notes should be array")
            .len(),
        0
    );
}

#[test]
fn security_shadow_evaluation_tracks_window_and_oot_stability_for_champion_readiness() {
    let runtime_db_path = create_test_runtime_db("security_shadow_evaluation_window_stability");
    let fixture_dir = create_shadow_fixture_dir("security_shadow_evaluation_window_stability");
    let registry_path = write_registry_fixture(
        &fixture_dir,
        "registry_current.json",
        "shadow",
        "champion_candidate_ready",
        0.81,
        0.76,
    );
    let comparison_one = write_registry_fixture(
        &fixture_dir,
        "registry_window_one.json",
        "shadow",
        "champion_candidate_ready",
        0.78,
        0.72,
    );
    let comparison_two = write_registry_fixture(
        &fixture_dir,
        "registry_window_two.json",
        "shadow",
        "champion_candidate_ready",
        0.80,
        0.74,
    );
    let history_expansion_path =
        write_history_expansion_fixture(&fixture_dir, "history_expansion.json");
    let prior_one = write_prior_shadow_evaluation_fixture(
        &fixture_dir,
        "prior_shadow_one.json",
        "shadow_consistent",
    );
    let prior_two = write_prior_shadow_evaluation_fixture(
        &fixture_dir,
        "prior_shadow_two.json",
        "shadow_consistent",
    );

    let request = json!({
        "tool": "security_shadow_evaluation",
        "args": {
            "created_at": "2026-04-11T22:40:00+08:00",
            "market_scope": "A_SHARE",
            "instrument_scope": "ETF",
            "instrument_subscope": "treasury_etf",
            "model_registry_path": registry_path.to_string_lossy(),
            "comparison_model_registry_paths": [
                comparison_one.to_string_lossy(),
                comparison_two.to_string_lossy()
            ],
            "history_expansion_paths": [history_expansion_path.to_string_lossy()],
            "prior_shadow_evaluation_paths": [
                prior_one.to_string_lossy(),
                prior_two.to_string_lossy()
            ]
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-11 CST: Add a red test for window/OOT promotion evidence, because
    // P7 needs champion-readiness to depend on repeated stable windows instead of
    // only a repeated shadow-observation count.
    // Purpose: force shadow evaluation to publish multi-window stability fields.
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["shadow_evaluation"]["shadow_window_count"],
        3
    );
    assert_eq!(
        output["data"]["shadow_evaluation"]["oot_stability_status"],
        "oot_stable"
    );
    assert_eq!(
        output["data"]["shadow_evaluation"]["window_consistency_status"],
        "window_consistent"
    );
    assert_eq!(
        output["data"]["shadow_evaluation"]["promotion_evidence_notes"]
            .as_array()
            .expect("promotion evidence notes should be array")
            .len(),
        0
    );
}
