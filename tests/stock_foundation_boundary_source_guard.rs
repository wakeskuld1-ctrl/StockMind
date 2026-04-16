use std::fs;
use std::path::{Path, PathBuf};

const SPLIT_PLAN_DOC: &str = "docs/plans/2026-04-15-stock-foundation-decoupling-design.md";
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

fn is_stock_business_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    name.starts_with("stock_")
        || name.starts_with("security_")
        || name == "technical_consultation_basic.rs"
}

#[test]
fn stock_business_modules_do_not_import_generic_foundation_analytics() {
    // 2026-04-15 CST: Added because the user explicitly wants Stock and generic
    // foundation analytics to become separable engineering blocks if there is no
    // real business-layer dependency between them.
    // Purpose: fail fast when stock business files start importing generic analysis
    // modules and silently recreate coupling that blocks future workspace split.
    let forbidden_imports = [
        "crate::ops::linear_regression",
        "crate::ops::logistic_regression",
        "crate::ops::stat_summary",
        "crate::ops::correlation_analysis",
        "crate::ops::trend_analysis",
        "crate::ops::cluster_kmeans",
        "crate::ops::decision_assistant",
        "crate::ops::foundation::",
    ];

    for path in collect_rust_files(Path::new("src/ops")) {
        if !is_stock_business_file(&path) {
            continue;
        }

        let source = fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("read source file {}", path.display()));
        let normalized = normalize_newlines(&source);
        for forbidden in forbidden_imports {
            assert!(
                !normalized.contains(forbidden),
                "Stock/foundation boundary drift detected in {}: generic foundation analytics import `{}` is forbidden for stock business files. Review {} and {} before coupling Stock back to the generic analytics block.",
                path.display(),
                forbidden,
                SPLIT_PLAN_DOC,
                HANDOFF_DOC,
            );
        }
    }
}

#[test]
fn stock_dispatcher_keeps_generic_foundation_analytics_outside_the_stock_bus() {
    // 2026-04-15 CST: Added because the future split needs one clear answer about
    // whether the stock tool bus owns generic analytics routes.
    // Purpose: fail fast when stock_ops starts routing generic analytics tools that
    // should stay outside the stock-domain engineering boundary.
    let source =
        fs::read_to_string("src/tools/dispatcher/stock_ops.rs").expect("read stock dispatcher");
    let normalized = normalize_newlines(&source);

    for forbidden_tool in [
        "\"linear_regression\"",
        "\"logistic_regression\"",
        "\"stat_summary\"",
        "\"correlation_analysis\"",
        "\"trend_analysis\"",
        "\"cluster_kmeans\"",
        "\"decision_assistant\"",
    ] {
        assert!(
            !normalized.contains(forbidden_tool),
            "Stock/foundation boundary drift detected in src/tools/dispatcher/stock_ops.rs: generic analytics tool route `{forbidden_tool}` should not enter the stock dispatcher bus. Review {SPLIT_PLAN_DOC} and {HANDOFF_DOC} before changing public routing ownership."
        );
    }
}

#[test]
fn stock_foundation_split_design_is_recorded_in_docs() {
    // 2026-04-15 CST: Added because the user wants the split rule written down so
    // later AI sessions do not rediscover or undo it from scratch.
    // Purpose: require one formal design baseline and one handoff marker before any
    // future split or coupling decision changes.
    let plan = fs::read_to_string(SPLIT_PLAN_DOC)
        .unwrap_or_else(|_| panic!("read split design doc {SPLIT_PLAN_DOC}"));
    let plan_normalized = normalize_newlines(&plan);
    assert!(
        plan_normalized
            .contains("Stock does not currently depend on generic foundation analytics."),
        "Split-baseline drift detected in {SPLIT_PLAN_DOC}: the formal dependency verdict marker is missing."
    );
    assert!(
        plan_normalized.contains("Recommended engineering split"),
        "Split-baseline drift detected in {SPLIT_PLAN_DOC}: the recommended engineering split section is missing."
    );

    let handoff = fs::read_to_string(HANDOFF_DOC).expect("read docs/AI_HANDOFF.md");
    let handoff_normalized = normalize_newlines(&handoff);
    assert!(
        handoff_normalized.contains("Stock/Foundation Decoupling Baseline"),
        "Split-baseline drift detected in {HANDOFF_DOC}: the stock/foundation decoupling handoff section is missing."
    );
}
