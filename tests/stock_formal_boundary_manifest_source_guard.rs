use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const MANIFEST_PLAN_DOC: &str =
    "docs/plans/2026-04-16-stock-formal-boundary-manifest-gate-design.md";
const SPLIT_MANIFEST_DOC: &str = "docs/plans/2026-04-15-stock-foundation-split-manifest-design.md";
const GATE_V2_PLAN_DOC: &str = "docs/plans/2026-04-15-stock-foundation-boundary-gate-v2-design.md";
const HANDOFF_DOC: &str = "docs/AI_HANDOFF.md";

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n")
}

fn collect_rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let entries = fs::read_dir(dir).unwrap_or_else(|_| panic!("read directory {}", dir.display()));
    for entry in entries {
        let entry = entry.unwrap_or_else(|_| panic!("read entry under {}", dir.display()));
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_rust_files(&path));
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    files.sort();
    files
}

fn declared_modules(source: &str, prefix: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix(prefix)
                .and_then(|value| value.strip_suffix(';'))
                .map(|value| value.trim().to_string())
        })
        .collect()
}

fn declared_module_name(line: &str) -> Option<String> {
    let trimmed = line.trim();
    trimmed
        .strip_prefix("pub mod ")
        .or_else(|| trimmed.strip_prefix("pub(crate) mod "))
        .and_then(|value| value.strip_suffix(';'))
        .map(|value| value.trim().to_string())
}

fn manifest_relative_paths(source: &str) -> BTreeSet<String> {
    // 2026-04-16 CST: Added because the helper-bridge scan must follow the formal
    // manifest itself instead of guessing ownership from filename prefixes.
    // Purpose: keep the gate aligned with `stock.rs` / `foundation.rs` so newly
    // approved modules do not need a second manual whitelist update.
    let mut approved_paths = BTreeSet::new();
    let mut pending_path_attr: Option<String> = None;

    for line in source.lines() {
        let trimmed = line.trim();

        if let Some(path) = trimmed
            .strip_prefix("#[path = \"")
            .and_then(|value| value.strip_suffix("\"]"))
        {
            pending_path_attr = Some(path.to_string());
            continue;
        }

        if let Some(module_name) = declared_module_name(trimmed) {
            approved_paths.insert(
                pending_path_attr
                    .take()
                    .unwrap_or_else(|| format!("{module_name}.rs")),
            );
            continue;
        }

        if !trimmed.is_empty() && !trimmed.starts_with("//") {
            pending_path_attr = None;
        }
    }

    approved_paths
}

#[test]
fn stock_root_keeps_only_the_frozen_module_manifest() {
    // 2026-04-16 CST: Added because gate v2 still protected shell usage more than
    // the formal stock boundary declaration itself.
    // Purpose: freeze the approved `stock.rs` module manifest so later AI sessions
    // cannot silently add one new module into the formal stock surface.
    let source = fs::read_to_string("src/ops/stock.rs").expect("read src/ops/stock.rs");
    let normalized = normalize_newlines(&source);

    let expected_public_modules: BTreeSet<String> = [
        "stock_data_pipeline",
        "stock_pre_trade",
        "stock_governance_and_positioning",
        "stock_execution_and_position_management",
        "stock_post_trade",
        "stock_modeling_and_training",
        "stock_research_sidecar",
        "stock_data_readiness_entry",
        "stock_investment_case_entry",
        "stock_governed_action_entry",
        "stock_position_management_entry",
        "stock_post_trade_learning_entry",
        "stock_research_sidecar_entry",
        "import_stock_price_history",
        "security_analysis_contextual",
        "security_analysis_fullstack",
        "security_independent_advice",
        "security_position_plan",
        "security_portfolio_position_plan",
        "security_post_trade_review",
        "security_execution_record",
        "security_execution_journal",
        "security_account_open_position_snapshot",
        "stock_analysis_data_guard",
        "technical_consultation_basic",
        "security_decision_evidence_bundle",
        "security_etf_resonance_trust_pack",
        "security_risk_gates",
        "security_decision_card",
        "security_decision_committee",
        "security_scorecard",
        "security_composite_scorecard",
        // 2026-04-16 CST: Updated because the approved composite bridge now has its own
        // frozen design baseline and formal handoff closure.
        // Reason: the boundary audit proved the adapter is no longer an accidental drift but
        // an already-landed approved module on the stock mainline.
        // Purpose: align the manifest guard with the current approved formal stock surface.
        "security_composite_committee_payload_adapter",
        "security_master_scorecard",
        "security_model_promotion",
        "security_shadow_evaluation",
        "security_chair_resolution",
        "security_record_post_meeting_conclusion",
        "security_fundamental_history_backfill",
        "security_disclosure_history_backfill",
        "security_fundamental_history_live_backfill",
        "security_disclosure_history_live_backfill",
        "security_post_meeting_conclusion",
        "security_decision_package",
        "security_decision_verify_package",
        "security_decision_package_revision",
        "security_feature_snapshot",
        "security_forward_outcome",
        "security_external_proxy_backfill",
        "security_external_proxy_history_import",
        "security_history_expansion",
        "security_scorecard_model_registry",
        "security_scorecard_refit_run",
        "security_scorecard_training",
        "security_approval_brief_signature",
        "security_decision_approval_bridge",
        "security_decision_approval_brief",
        "security_condition_review",
        "security_decision_submit_approval",
        "security_decision_briefing",
        "security_position_plan_record",
        "security_record_position_adjustment",
        "security_committee_vote",
        "security_analysis_resonance",
        "sync_template_resonance_factors",
        "signal_outcome_research",
        "sync_stock_price_history",
        "stock_training_data_backfill",
        "stock_training_data_coverage_audit",
        "security_real_data_validation_backfill",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let expected_internal_modules: BTreeSet<String> = [
        "security_execution_record_assembler",
        "security_execution_account_binding_resolver",
        "security_account_open_position_snapshot_assembler",
        // 2026-04-16 CST: Updated because the current formal stock boundary already
        // froze the legacy compatibility adapter and the open-position corporate-action
        // summary as internal supporting modules.
        // Reason: the boundary audit proved these internals are real approved support
        // slices, not accidental drift.
        // Purpose: align the internal manifest guard with the current approved stock internals.
        "security_open_position_corporate_action_summary",
        "security_post_trade_review_assembler",
        "security_post_trade_review_policy",
        "security_legacy_committee_compat",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let actual_public_modules = declared_modules(&normalized, "pub mod ");
    let actual_internal_modules = declared_modules(&normalized, "pub(crate) mod ");

    assert_eq!(
        actual_public_modules, expected_public_modules,
        "Formal stock-boundary drift detected in src/ops/stock.rs: the public module manifest changed. Review {MANIFEST_PLAN_DOC}, {SPLIT_MANIFEST_DOC}, and {HANDOFF_DOC} before changing the formal stock boundary."
    );
    assert_eq!(
        actual_internal_modules, expected_internal_modules,
        "Formal stock-boundary drift detected in src/ops/stock.rs: the internal module manifest changed. Review {MANIFEST_PLAN_DOC}, {GATE_V2_PLAN_DOC}, and {HANDOFF_DOC} before changing supporting stock-boundary internals."
    );

    for forbidden_name in ["adapter", "helper", "foundation"] {
        assert!(
            !actual_public_modules
                .iter()
                .chain(actual_internal_modules.iter())
                .any(|value| {
                    value.contains(forbidden_name)
                        && value != "security_composite_committee_payload_adapter"
                }),
            "Formal stock-boundary drift detected in src/ops/stock.rs: module names containing `{forbidden_name}` are forbidden until an explicit exception process exists. Review {MANIFEST_PLAN_DOC} and {HANDOFF_DOC} before introducing this kind of boundary module."
        );
    }
}

#[test]
fn ops_root_keeps_only_foundation_and_stock_as_formal_boundaries() {
    // 2026-04-16 CST: Added because hidden boundary drift can also happen one level
    // above stock.rs if src/ops/mod.rs starts exposing a third formal business root.
    // Purpose: keep `crate::ops::*` limited to the two already-approved top-level
    // boundaries instead of reintroducing an ungoverned middle surface.
    let source = fs::read_to_string("src/ops/mod.rs").expect("read src/ops/mod.rs");
    let normalized = normalize_newlines(&source);
    let top_level_modules = declared_modules(&normalized, "pub mod ");

    let expected_top_level_modules: BTreeSet<String> = ["foundation", "stock"]
        .into_iter()
        .map(String::from)
        .collect();

    assert_eq!(
        top_level_modules, expected_top_level_modules,
        "Ops-root drift detected in src/ops/mod.rs: only `foundation` and `stock` may remain as formal top-level boundaries. Review {MANIFEST_PLAN_DOC}, {SPLIT_MANIFEST_DOC}, and {HANDOFF_DOC} before exposing another ops root."
    );
    assert!(
        normalized.contains("pub use foundation::"),
        "Ops-root drift detected in src/ops/mod.rs: foundation re-export surface marker is missing. Review {SPLIT_MANIFEST_DOC} before redesigning the top-level ops root."
    );
    assert!(
        !normalized.contains("pub use stock::"),
        "Ops-root drift detected in src/ops/mod.rs: stock modules must stay behind crate::ops::stock::* instead of returning to crate::ops::* re-exports. Review {MANIFEST_PLAN_DOC} and {HANDOFF_DOC} before changing this rule."
    );
}

#[test]
fn unscoped_ops_files_do_not_hide_stock_to_foundation_or_hold_zone_bridges() {
    // 2026-04-16 CST: Added because a future session could avoid the stock_ /
    // security_ naming rules by introducing a neutral-looking helper file under
    // src/ops and using it as an indirect bridge.
    // Purpose: fail fast when an unscoped ops file starts mixing stock-domain
    // references with foundation analytics or shared/runtime hold-zone references.
    // 2026-04-16 CST: Added because the formal-boundary manifest gate should use
    // the real approved surfaces as its source of truth.
    // Purpose: only scan files that are outside both frozen manifests.
    let mut approved_stock_paths = manifest_relative_paths(&normalize_newlines(
        &fs::read_to_string("src/ops/stock.rs").expect("read src/ops/stock.rs"),
    ));
    approved_stock_paths.insert("stock.rs".to_string());

    let mut approved_foundation_paths = manifest_relative_paths(&normalize_newlines(
        &fs::read_to_string("src/ops/foundation.rs").expect("read src/ops/foundation.rs"),
    ));
    approved_foundation_paths.insert("foundation.rs".to_string());

    let stock_markers = [
        "crate::ops::stock::",
        "super::stock_",
        "pub use super::stock_",
    ];
    let forbidden_bridge_markers = [
        "crate::ops::foundation::",
        "crate::ops::linear_regression",
        "crate::ops::logistic_regression",
        "crate::ops::stat_summary",
        "crate::ops::correlation_analysis",
        "crate::ops::trend_analysis",
        "crate::ops::cluster_kmeans",
        "crate::ops::decision_assistant",
        "crate::tools::catalog",
        "crate::tools::dispatcher",
        "crate::tools::contracts",
        "crate::runtime::",
    ];

    for path in collect_rust_files(Path::new("src/ops")) {
        let relative_path = path
            .strip_prefix("src/ops")
            .unwrap_or_else(|_| panic!("strip prefix for {}", path.display()))
            .to_string_lossy()
            .replace('\\', "/");
        if relative_path == "mod.rs"
            || approved_stock_paths.contains(&relative_path)
            || approved_foundation_paths.contains(&relative_path)
        {
            continue;
        }

        let source = fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("read source file {}", path.display()));
        let normalized = normalize_newlines(&source);
        let references_stock = stock_markers
            .iter()
            .any(|marker| normalized.contains(marker));
        let crosses_forbidden_boundary = forbidden_bridge_markers
            .iter()
            .any(|marker| normalized.contains(marker));

        assert!(
            !(references_stock && crosses_forbidden_boundary),
            "Hidden-bridge drift detected in {}: unscoped ops files must not mix stock references with foundation/shared/runtime markers. Review {MANIFEST_PLAN_DOC}, {GATE_V2_PLAN_DOC}, and {HANDOFF_DOC} before introducing a helper bridge here.",
            path.display(),
        );
    }
}

#[test]
fn formal_boundary_manifest_gate_is_recorded_in_docs() {
    // 2026-04-16 CST: Added because the new manifest gate should be discoverable
    // in both design docs and handoff before later sessions change boundary rules.
    // Purpose: require one formal design marker and one handoff marker for the
    // stock formal-boundary manifest gate.
    let plan = normalize_newlines(
        &fs::read_to_string(MANIFEST_PLAN_DOC)
            .unwrap_or_else(|_| panic!("read {MANIFEST_PLAN_DOC}")),
    );
    assert!(
        plan.contains("Use `Option B`."),
        "Formal-boundary manifest drift detected in {MANIFEST_PLAN_DOC}: the approved option marker is missing."
    );
    assert!(
        plan.contains("Gate 1 - stock root manifest freeze"),
        "Formal-boundary manifest drift detected in {MANIFEST_PLAN_DOC}: the stock root manifest section is missing."
    );

    let handoff =
        normalize_newlines(&fs::read_to_string(HANDOFF_DOC).expect("read docs/AI_HANDOFF.md"));
    assert!(
        handoff.contains("Stock Formal Boundary Manifest Gate"),
        "Formal-boundary manifest drift detected in {HANDOFF_DOC}: the formal-boundary manifest handoff section is missing."
    );
}
