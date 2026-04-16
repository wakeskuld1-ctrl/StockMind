use std::fs;
use std::path::{Path, PathBuf};

const LEGACY_PLAN_DOC: &str =
    "docs/plans/2026-04-16-security-legacy-committee-governance-design.md";
const HANDOFF_DOC: &str = "docs/AI_HANDOFF.md";
const GROUPED_GATEWAY_FILE: &str = "src/ops/stock_governance_and_positioning.rs";
const DISPATCHER_FILE: &str = "src/tools/dispatcher/stock_ops.rs";
const LEGACY_IMPORT_MARKER: &str = "security_decision_committee::{";

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n")
}

fn normalize_rel_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn collect_rs_files(root: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).expect("read directory");
    for entry in entries {
        let entry = entry.expect("read entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn legacy_committee_application_surface_stays_explicitly_labeled() {
    // 2026-04-16 CST: Added because the dependency gate already freezes direct
    // business imports inside src/ops, but the application surface can still
    // accidentally normalize the legacy committee route unless it stays clearly
    // labeled at grouped-gateway and dispatcher level.
    // Reason: later sessions may only look at the public application surface and
    // miss the lower-level compatibility rule.
    // Purpose: fail fast when the remaining legacy export path stops carrying
    // explicit legacy wording.
    let grouped_gateway = normalize_newlines(
        &fs::read_to_string(GROUPED_GATEWAY_FILE)
            .expect("read src/ops/stock_governance_and_positioning.rs"),
    );
    let dispatcher = normalize_newlines(
        &fs::read_to_string(DISPATCHER_FILE).expect("read src/tools/dispatcher/stock_ops.rs"),
    );
    let handoff =
        normalize_newlines(&fs::read_to_string(HANDOFF_DOC).expect("read docs/AI_HANDOFF.md"));

    assert!(
        grouped_gateway.contains("legacy committee chain still needs one"),
        "Application-surface governance drift detected in {GROUPED_GATEWAY_FILE}: grouped gateway must keep the legacy committee export explicitly documented. Review {LEGACY_PLAN_DOC} and {HANDOFF_DOC} before changing this surface."
    );
    assert!(
        dispatcher.contains("controlled legacy"),
        "Application-surface governance drift detected in {DISPATCHER_FILE}: dispatcher must keep the legacy committee route explicitly documented. Review {LEGACY_PLAN_DOC} and {HANDOFF_DOC} before changing this surface."
    );
    assert!(
        dispatcher.contains("LegacySecurityCommitteeMemberAgentRequest"),
        "Application-surface governance drift detected in {DISPATCHER_FILE}: legacy committee request aliases must stay explicitly named. Review {LEGACY_PLAN_DOC} and {HANDOFF_DOC} before changing dispatcher wiring."
    );
    assert!(
        dispatcher.contains("legacy_security_committee_member_agent"),
        "Application-surface governance drift detected in {DISPATCHER_FILE}: legacy committee member-agent route must stay explicitly named. Review {LEGACY_PLAN_DOC} and {HANDOFF_DOC} before changing dispatcher wiring."
    );
    assert!(
        handoff.contains("legacy committee application surface"),
        "Application-surface governance drift detected in {HANDOFF_DOC}: missing legacy application-surface rule. Review {LEGACY_PLAN_DOC} before changing grouped gateway or dispatcher exposure."
    );
}

#[test]
fn legacy_committee_dispatcher_import_is_confined_to_one_explicit_file() {
    // 2026-04-16 CST: Added because the next likely drift is not a new business
    // import under src/ops, but another application file quietly wiring the
    // frozen legacy committee route through a second dispatcher or shell.
    // Reason: once that happens, retirement complexity expands again even if the
    // old business dependency gate still passes.
    // Purpose: keep one single application-surface owner for the remaining legacy
    // committee dispatcher import.
    let mut rs_files = Vec::new();
    collect_rs_files(Path::new("src/tools"), &mut rs_files);

    let mut offenders = Vec::new();
    for path in rs_files {
        let normalized_path = normalize_rel_path(&path);
        let source = normalize_newlines(
            &fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {normalized_path}")),
        );
        if source.contains(LEGACY_IMPORT_MARKER) && normalized_path != DISPATCHER_FILE {
            offenders.push(normalized_path);
        }
    }

    assert!(
        offenders.is_empty(),
        "Application-surface governance drift detected: only {DISPATCHER_FILE} may import the grouped legacy committee dispatcher route. Offenders: {offenders:?}. Review {LEGACY_PLAN_DOC} and {HANDOFF_DOC} before widening legacy exposure."
    );
}
