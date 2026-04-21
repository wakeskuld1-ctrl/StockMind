# StockMind Decision Log

## Fixed Decisions

| Date | Decision | Why it exists | Current status | Source |
| --- | --- | --- | --- | --- |
| 2026-04-15 | StockMind remains a stock-only standalone repo | keep the formal securities chain buildable without reviving the old foundation stack | active | `docs/architecture/stockmind-snapshot-manifest.md`, `README.md` |
| 2026-04-15 | `src/ops/stock.rs` is a frozen formal boundary | prevent the split repo from drifting back into a flat surface | active | `docs/AI_HANDOFF.md`, `docs/plans/2026-04-16-stock-formal-boundary-manifest-gate-design.md` |
| 2026-04-16 | legacy committee flow is compatibility-only | keep new governance work on `security_committee_vote -> security_chair_resolution` | active | `docs/AI_HANDOFF.md`, `docs/plans/2026-04-16-security-legacy-committee-governance-design.md` |
| 2026-04-17 | acceptance must be expressed as structure, formal mainline, and repository levels | replace memory-based acceptance with repeatable checks | active | `docs/architecture/stockmind-acceptance-checklist.md` |
| 2026-04-17 | README must explain how to run, verify, and judge the split repo | lower handoff and onboarding cost | active | `README.md`, `CHANGELOG_TASK.MD` |
| 2026-04-19 | post-open management starts from approved packets and governed evidence objects | keep pure-data and governance flow explicit | historically referenced; re-verify when those slices are touched again | `docs/handoff/AI_HANDOFF.md` |
| 2026-04-19 | P10 and P11 are implemented before P12 | avoid overstating portfolio-core completeness | active | `docs/handoff/AI_HANDOFF.md`, `src/tools/catalog.rs` |
| 2026-04-20 | P12 is an enhanced bounded refinement layer downstream of P11, not a second replacement solver | keep the final portfolio-core stage auditable while allowing residual-cash priority fill inside turnover slack and governed symbol caps | active | `docs/plans/2026-04-20-p12-governed-portfolio-allocation-decision-design.md`, `docs/plans/2026-04-20-p12-enhanced-allocation-refinement-design.md`, `src/ops/security_portfolio_allocation_decision.rs` |
| 2026-04-20 | portfolio-core chain requires a dedicated source guard after P12 | keep `P10 -> P11 -> P12` on formal document inputs, approved public-tool order, and explicit acceptance routing after the chain became code-complete | active | `tests/security_portfolio_core_chain_source_guard.rs`, `docs/architecture/stockmind-acceptance-checklist.md`, `docs/handoff/AI_HANDOFF.md` |
| 2026-04-20 | the first downstream step after P12 stays preview-only and may expose only a nested request-aligned preview subset | allow one explicit post-P12 consumer without silently turning allocation output into runtime execution or persistence while still preparing a future execution bridge | active | `docs/plans/2026-04-20-post-p12-portfolio-execution-preview-design.md`, `docs/plans/2026-04-20-post-p12-execution-request-preview-standardization-design.md`, `src/ops/security_portfolio_execution_preview.rs`, `tests/security_portfolio_execution_preview_cli.rs` |
| 2026-04-20 | P13 packages standardized preview into formal request documents without becoming real execution | continue the post-P12 execution-bridge mainline while keeping runtime writes, execution facts, and approval detours out of this slice | active | `docs/plans/2026-04-20-p13-portfolio-execution-request-bridge-design.md`, `src/ops/security_portfolio_execution_request_package.rs`, `tests/security_portfolio_execution_request_package_cli.rs` |
| 2026-04-21 | P14 enriches formal P13 request packages into execution-aligned request bundles without becoming real execution | keep the post-P12 execution bridge decomposed and auditable by inserting one explicit enrichment layer before any future apply/runtime stage | active | `docs/plans/2026-04-20-p14-execution-request-enrichment-bridge-design.md`, `src/ops/security_portfolio_execution_request_enrichment.rs`, `tests/security_portfolio_execution_request_enrichment_cli.rs` |
| 2026-04-21 | P15 applies governed P14 enriched request bundles through `security_execution_record` without becoming broker execution | keep the post-P12 execution path on the existing runtime-owned execution mainline while preserving explicit blocked/hold semantics and truthful non-atomicity boundaries | active | `docs/plans/2026-04-21-p15-portfolio-execution-apply-bridge-design.md`, `src/ops/security_portfolio_execution_apply_bridge.rs`, `tests/security_portfolio_execution_apply_bridge_cli.rs` |
| 2026-04-20 | branch-health truth must live in a dedicated current-status file | stop historical acceptance notes from being mistaken for current branch health | active | `docs/handoff/CURRENT_STATUS.md` |
| 2026-04-20 | P10/P11 closeout must cite the checked-in graph audit as a structural support artifact, not as a semantic proof replacement | keep phase closeout honest while still preserving a reusable repository-level audit bundle | active | `docs/handoff/P10_P11_AUDIT_CLOSEOUT.md`, `graphify-out/GRAPH_REPORT.md` |

## Current Assumptions

- the repository should optimize for minimal backport instead of broad structural churn
- runtime ownership stays governed and stock-only
- the checked-in graph audit is structural support evidence and remains AST-only in this branch
- acceptance claims must always be backed by commands run on the current branch

## Open Questions

- if a future phase needs stronger audit coverage, which non-code corpora should be added beyond the current AST-only graph bundle?
- should the branch-health snapshot be updated only on state change, or on every major task closeout?

## Decision Logging Rule

Add a new decision row when any of the following become true:

- a boundary that future contributors must not silently cross is introduced
- a contract or runtime rule becomes a release gate
- a historical assumption is replaced by a stricter repository rule

Do not use this file for transient bug notes; those belong in `docs/handoff/HANDOFF_ISSUES.md` and `CHANGELOG_TASK.MD`.
