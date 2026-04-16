use std::fs;
use std::path::{Path, PathBuf};

const OPS_ROOT: &str = "src/ops";
const LEGACY_DIRECT_IMPORT: &str = "use crate::ops::stock::security_decision_committee::";
const COMPAT_IMPORT: &str = "use crate::ops::stock::security_legacy_committee_compat::";

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n")
}

fn collect_rs_files(root: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).expect("read src/ops directory");
    for entry in entries {
        let entry = entry.expect("read src/ops entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn normalize_rel_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[test]
fn legacy_committee_direct_dependency_is_confined_to_compat_adapter() {
    // 2026-04-16 CST: Added because the user explicitly required future cargo
    // test runs to intercept any new business-layer drift back into the frozen
    // legacy committee module.
    // Purpose: keep the compatibility owner singular while later migrations move
    // downstream modules toward the formal committee mainline.
    let mut rs_files = Vec::new();
    collect_rs_files(Path::new(OPS_ROOT), &mut rs_files);

    let mut offenders = Vec::new();
    for path in rs_files {
        let normalized_path = normalize_rel_path(&path);
        let source = normalize_newlines(
            &fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {normalized_path}")),
        );
        if source.contains(LEGACY_DIRECT_IMPORT)
            && normalized_path != "src/ops/security_legacy_committee_compat.rs"
        {
            offenders.push(normalized_path);
        }
    }

    assert!(
        offenders.is_empty(),
        "Legacy committee governance drift detected: only src/ops/security_legacy_committee_compat.rs may directly import security_decision_committee. Offenders: {offenders:?}"
    );
}

#[test]
fn key_business_callers_must_depend_on_compat_adapter() {
    // 2026-04-16 CST: Added because the governance round should not only forbid
    // new drift, but also prove that the first key business callers were really
    // moved behind the compatibility owner.
    // Purpose: lock the first dependency-shrinking step so later edits cannot
    // silently slide chair, submit, or master_scorecard back to direct legacy use.
    for path in [
        "src/ops/security_chair_resolution.rs",
        "src/ops/security_decision_submit_approval.rs",
        "src/ops/security_master_scorecard.rs",
    ] {
        let source =
            normalize_newlines(&fs::read_to_string(path).unwrap_or_else(|_| panic!("read {path}")));
        assert!(
            source.contains(COMPAT_IMPORT),
            "Legacy committee governance drift detected: {path} must import security_legacy_committee_compat."
        );
    }
}
