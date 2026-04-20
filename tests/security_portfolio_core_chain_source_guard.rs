use std::fs;

const P10_SOURCE: &str = "src/ops/security_account_objective_contract.rs";
const P11_SOURCE: &str = "src/ops/security_portfolio_replacement_plan.rs";
const P12_SOURCE: &str = "src/ops/security_portfolio_allocation_decision.rs";
const CATALOG_SOURCE: &str = "src/tools/catalog.rs";
const DISPATCHER_SOURCE: &str = "src/tools/dispatcher.rs";
const ACCEPTANCE_DOC: &str = "docs/architecture/stockmind-acceptance-checklist.md";
const HANDOFF_DOC: &str = "docs/handoff/AI_HANDOFF.md";
const DECISION_LOG_DOC: &str = "docs/governance/decision_log.md";

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n")
}

fn read_required(path: &str) -> String {
    normalize_newlines(&fs::read_to_string(path).unwrap_or_else(|_| panic!("read {path}")))
}

fn struct_block(source: &str, marker: &str) -> String {
    // 2026-04-20 CST: Added because the dedicated portfolio-core chain guard
    // needs to inspect request shells instead of matching unrelated later fields.
    // Reason: whole-file substring checks would blur formal request boundaries with
    // downstream implementation details and create noisy false positives.
    // Purpose: isolate one struct block so the guard can assert exact upstream inputs.
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("missing struct marker `{marker}`"));
    let tail = &source[start..];
    let end = tail
        .find("\n}\n")
        .unwrap_or_else(|| panic!("missing struct terminator for `{marker}`"));
    tail[..end + 2].to_string()
}

fn assert_marker_order(source: &str, earlier: &str, later: &str, context: &str) {
    let earlier_index = source
        .find(earlier)
        .unwrap_or_else(|| panic!("missing `{earlier}` in {context}"));
    let later_index = source
        .find(later)
        .unwrap_or_else(|| panic!("missing `{later}` in {context}"));
    assert!(
        earlier_index < later_index,
        "Portfolio-core chain drift detected in {context}: expected `{earlier}` before `{later}`."
    );
}

#[test]
fn portfolio_core_request_shells_only_consume_formal_upstream_documents() {
    // 2026-04-20 CST: Added because the portfolio-core chain now has three
    // approved formal stages whose request shells must stay explicit.
    // Reason: once P12 exists, later sessions could silently widen P11/P12 back
    // to raw account fragments without tripping the broader manifest guard.
    // Purpose: freeze the exact formal upstream documents consumed by P11 and P12.
    let p10_source = read_required(P10_SOURCE);
    let p11_source = read_required(P11_SOURCE);
    let p12_source = read_required(P12_SOURCE);

    assert!(
        p10_source.contains("pub struct SecurityAccountObjectiveContractRequest"),
        "Portfolio-core chain drift detected in {P10_SOURCE}: the formal P10 request shell is missing."
    );

    let p11_request = struct_block(
        &p11_source,
        "pub struct SecurityPortfolioReplacementPlanRequest {",
    );
    assert!(
        p11_request.contains("pub account_objective_contract: SecurityAccountObjectiveContractDocument,"),
        "Portfolio-core chain drift detected in {P11_SOURCE}: P11 must still consume the formal account objective contract."
    );
    assert!(
        p11_request.contains("pub portfolio_candidate_set: SecurityPortfolioCandidateSet,"),
        "Portfolio-core chain drift detected in {P11_SOURCE}: P11 must still consume the governed portfolio candidate set."
    );
    assert!(
        !p11_request.contains("active_position_book")
            && !p11_request.contains("approved_candidates")
            && !p11_request.contains("monitoring_evidence_package"),
        "Portfolio-core chain drift detected in {P11_SOURCE}: P11 request shell widened beyond formal P10 outputs."
    );

    let p12_request = struct_block(
        &p12_source,
        "pub struct SecurityPortfolioAllocationDecisionRequest {",
    );
    assert!(
        p12_request.contains("pub account_objective_contract: SecurityAccountObjectiveContractDocument,"),
        "Portfolio-core chain drift detected in {P12_SOURCE}: P12 must still consume the formal account objective contract."
    );
    assert!(
        p12_request.contains("pub portfolio_candidate_set: SecurityPortfolioCandidateSet,"),
        "Portfolio-core chain drift detected in {P12_SOURCE}: P12 must still consume the governed portfolio candidate set."
    );
    assert!(
        p12_request.contains("pub portfolio_replacement_plan: SecurityPortfolioReplacementPlanDocument,"),
        "Portfolio-core chain drift detected in {P12_SOURCE}: P12 must still consume the formal P11 replacement plan."
    );
    assert!(
        !p12_request.contains("active_position_book")
            && !p12_request.contains("approved_candidates")
            && !p12_request.contains("position_contracts")
            && !p12_request.contains("monitoring_evidence_package"),
        "Portfolio-core chain drift detected in {P12_SOURCE}: P12 request shell widened beyond formal P10/P11 outputs."
    );
}

#[test]
fn portfolio_core_public_tool_surface_keeps_formal_order() {
    // 2026-04-20 CST: Added because the public catalog and dispatcher should
    // expose the portfolio-core chain in the same approved stage order.
    // Reason: if the tool surface drifts, later sessions may wire around the
    // formal chain even when request shells still look correct in isolation.
    // Purpose: freeze P10 -> P11 -> P12 ordering on the public CLI bus.
    let catalog_source = read_required(CATALOG_SOURCE);
    let dispatcher_source = read_required(DISPATCHER_SOURCE);

    assert_marker_order(
        &catalog_source,
        "\"security_account_objective_contract\"",
        "\"security_portfolio_replacement_plan\"",
        CATALOG_SOURCE,
    );
    assert_marker_order(
        &catalog_source,
        "\"security_portfolio_replacement_plan\"",
        "\"security_portfolio_allocation_decision\"",
        CATALOG_SOURCE,
    );
    assert_marker_order(
        &dispatcher_source,
        "\"security_account_objective_contract\" =>",
        "\"security_portfolio_replacement_plan\" =>",
        DISPATCHER_SOURCE,
    );
    assert_marker_order(
        &dispatcher_source,
        "\"security_portfolio_replacement_plan\" =>",
        "\"security_portfolio_allocation_decision\" =>",
        DISPATCHER_SOURCE,
    );
}

#[test]
fn portfolio_core_chain_guard_is_recorded_in_docs() {
    // 2026-04-20 CST: Added because the dedicated chain guard should be visible
    // in acceptance, decision, and handoff truth before later sessions rely on it.
    // Reason: a hidden guard is easy to bypass socially even if the test exists in code.
    // Purpose: require one stable acceptance marker, one handoff marker, and one
    // closed decision marker for the portfolio-core chain guard.
    let acceptance = read_required(ACCEPTANCE_DOC);
    let handoff = read_required(HANDOFF_DOC);
    let decision_log = read_required(DECISION_LOG_DOC);

    assert!(
        acceptance.contains("security_portfolio_core_chain_source_guard"),
        "Portfolio-core chain drift detected in {ACCEPTANCE_DOC}: the dedicated guard command is missing from the acceptance map."
    );
    assert!(
        acceptance.contains("portfolio-core chain"),
        "Portfolio-core chain drift detected in {ACCEPTANCE_DOC}: the formal portfolio-core chain proof text is missing."
    );
    assert!(
        handoff.contains("security_portfolio_core_chain_source_guard"),
        "Portfolio-core chain drift detected in {HANDOFF_DOC}: the dedicated guard is missing from handoff truth."
    );
    assert!(
        decision_log.contains("portfolio-core chain requires a dedicated source guard after P12"),
        "Portfolio-core chain drift detected in {DECISION_LOG_DOC}: the fixed decision for the dedicated chain guard is missing."
    );
    assert!(
        !decision_log.contains(
            "after P12 landed, should future portfolio-core closeout require a dedicated chain-level source guard beyond the existing CLI and manifest coverage?"
        ),
        "Portfolio-core chain drift detected in {DECISION_LOG_DOC}: the old open question still treats the dedicated chain guard as unresolved."
    );
}
