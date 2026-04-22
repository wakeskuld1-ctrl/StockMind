# P14 Execution Request Enrichment Bridge Design

## Status

- Date: 2026-04-20
- Phase: `P14`
- Approved route: `Option 2 - execution request enrichment bridge`
- Upstream prerequisite:
  - `P10 -> P11 -> P12` portfolio-core chain is already implemented and guarded in this branch
  - the post-`P12` preview-only bridge is already implemented
  - `P13` formal request bridge is already implemented
- This document freezes the `P14` design and execution contract before implementation starts

## Intent

- Goal: add one formal `P14` bridge that enriches `P13` request-package rows into an execution-ready request bundle without calling runtime execution
- Scope:
  - consume only `SecurityPortfolioExecutionRequestPackageDocument`
  - emit one formal enriched execution request bundle document
  - preserve explicit lineage back to `P13`, preview, and `P12`
  - define the minimum enrichment fields needed before a later real execution apply bridge
  - keep the new bridge discoverable on the public stock tool surface
- Non-goals:
  - do not call `security_execution_record`
  - do not write execution runtime stores
  - do not persist execution facts
  - do not reopen `P10/P11/P12/P13` contracts
  - do not turn this phase into approval-first or audit-first packaging
- Success definition: the next implementation session can build `P14` without re-debating whether it should already execute trades or whether it is still only request preparation
- Delivery form: one design-freeze document that acts as the governing contract for the next implementation slice

## Single Source Of Truth

- Historical phase context:
  - `E:/SM/docs/handoff/AI_HANDOFF.md`
- Current branch health:
  - `E:/SM/docs/handoff/CURRENT_STATUS.md`
- Stable upstream contracts:
  - `E:/SM/src/ops/security_portfolio_execution_preview.rs`
  - `E:/SM/src/ops/security_portfolio_execution_request_package.rs`
  - `E:/SM/src/ops/security_execution_record.rs`
- Existing upstream acceptance:
  - `E:/SM/tests/security_portfolio_execution_preview_cli.rs`
  - `E:/SM/tests/security_portfolio_execution_request_package_cli.rs`
- Existing approved design baselines:
  - `E:/SM/docs/plans/2026-04-20-post-p12-portfolio-execution-preview-design.md`
  - `E:/SM/docs/plans/2026-04-20-post-p12-execution-request-preview-standardization-design.md`
  - `E:/SM/docs/plans/2026-04-20-p13-portfolio-execution-request-bridge-design.md`

## Two Approaches

### Option 1: Governed Execution Apply Bridge

- Core idea:
  - consume `P13` request rows
  - collect missing execution inputs
  - invoke the existing execution-record flow directly
- Pros:
  - shortest path to real execution closure
  - reduces the number of intermediate documents
- Cons:
  - crosses too many boundaries at once
  - mixes request preparation with runtime execution
- Risks:
  - easy to confuse request intent with actual execution fact

### Option 2: Execution Request Enrichment Bridge

- Core idea:
  - consume `P13` request rows
  - add the minimum execution-record-aligned enrichment fields needed for a later apply stage
  - stop before runtime execution
- Pros:
  - keeps the stage boundary explicit and low-risk
  - makes the gap between request packaging and real execution concrete
  - creates a cleaner handoff into a later execution apply bridge
- Cons:
  - still requires one later phase before true execution
  - adds one more formal document layer
- Risks:
  - if naming is weak, callers may treat enriched requests as already executable facts

## Decision

- Chosen approach: `Option 2 - execution request enrichment bridge`
- Why this route is the correct continuation:
  - `P13` intentionally stopped at request packaging
  - the existing `security_execution_record` contract is much richer and writes runtime state
  - therefore the safer next step is to freeze the enrichment gap first instead of collapsing preparation and execution into one slice
- Rejected alternative:
  - `Option 1 - Governed Execution Apply Bridge`
  - rejected for this phase because it would widen the implementation boundary too aggressively and blur the distinction between request intent and execution fact
- Known tradeoffs:
  - `P14` still will not execute trades
  - `P14` will enrich requests for later execution, not apply them

## Execution Contract

- Chosen approach:
  - add one new formal tool downstream of the `P13` request package
  - consume only `SecurityPortfolioExecutionRequestPackageDocument`
  - emit a side-effect-free enriched execution request bundle
- Allowed change boundary:
  - create one new ops module for the enrichment bridge
  - wire it into `stock.rs`, `stock_execution_and_position_management.rs`, catalog, and dispatcher
  - add one dedicated CLI test file
  - update governance / handoff truth only if implementation actually lands and changes accepted branch truth
- Explicit non-goals:
  - no runtime writes
  - no execution store integration
  - no `security_execution_record` invocation
  - no direct creation of `SecurityExecutionRecordDocument`
- Best-practice path expected for this route:
  - `P14` must consume the formal `P13` request package only
  - `P14` must preserve lineage back to the preview and governed allocation decision
  - `P14` must enrich only `ready_request` rows into execution-record-aligned request candidates
  - `hold` rows must remain explicit and non-executable
  - `P14` must fail hard on malformed request-package input instead of repairing it silently
- Acceptance checks for route conformance:
  - the public request shell contains only the `P13` package plus metadata
  - the output is an enriched request bundle document, not an execution record
  - happy-path and hard-fail tests prove that `P14` enriches governed request rows instead of bypassing `P13`

## Proposed Public Tool

- Tool name: `security_portfolio_execution_request_enrichment`
- Stage meaning: `P14` execution request enrichment bridge
- Tool responsibility: convert the formal `P13` request package into one enriched execution-request bundle that is ready for a later governed execution apply bridge

## Contract

### Request Contract

- Proposed type: `SecurityPortfolioExecutionRequestEnrichmentRequest`
- Required fields:
  - `portfolio_execution_request_package: SecurityPortfolioExecutionRequestPackageDocument`
  - `analysis_date: String`
  - `created_at: String`
- Compatibility rule:
  - `P14` request must not accept raw preview rows, raw `P12` fragments, or direct runtime execution payloads
  - `P14` must treat the `P13` package as the only legal upstream source

### Output Contract

- Proposed document type: `security_portfolio_execution_request_enrichment`
- Proposed version: `security_portfolio_execution_request_enrichment.v1`
- Proposed primary document: `SecurityPortfolioExecutionRequestEnrichmentDocument`
- Proposed wrapper: `SecurityPortfolioExecutionRequestEnrichmentResult`

### Required Output Sections

- Identity and lineage:
  - `portfolio_execution_request_enrichment_id`
  - `contract_version`
  - `document_type`
  - `generated_at`
  - `analysis_date`
  - `account_id`
  - `portfolio_execution_request_package_ref`
  - `portfolio_execution_preview_ref`
  - `portfolio_allocation_decision_ref`
- Enriched request rows:
  - `enriched_request_rows`
  - `ready_for_apply_count`
  - `non_executable_hold_count`
  - `blocked_enrichment_count`
- Governance checks:
  - `readiness_status`
  - `blockers`
- Traceability and rationale:
  - `enrichment_rationale`
  - `enrichment_summary`

### Proposed Supporting Rows

- `SecurityPortfolioEnrichedExecutionRequestRow`
  - `symbol`
  - `request_action`
  - `requested_gross_pct`
  - `request_status`
  - `analysis_date`
  - `decision_ref`
  - `execution_action`
  - `execution_status`
  - `executed_gross_pct`
  - `execution_summary`
  - `enrichment_status`
  - `enrichment_summary`
  - `execution_apply_context`
- `SecurityExecutionApplyContext`
  - `as_of_date`
  - `market_symbol`
  - `sector_symbol`
  - `market_profile`
  - `sector_profile`
  - `market_regime`
  - `sector_template`
- enrichment rules:
  - `ready_request` rows become `ready_for_apply`
  - `non_executable_hold` rows stay `non_executable_hold`
  - `blocked_request` rows remain blocked and must not be upgraded
  - `ready_for_apply` rows must carry one minimum `execution_apply_context` sufficient for the later `P15` bridge to build a legal `SecurityExecutionRecordRequest` without introducing a second hidden context resolver
  - when governed taxonomy coverage is missing for an A-share symbol, `P14` may supply one bounded default execution anchor:
    - `market_symbol = 510300.SH`
    - `market_profile = a_share_core_v1`
    - `sector_symbol` falls back to the same market anchor when no narrower routed sector exists
    - `sector_profile` falls back to the same market profile when no narrower routed sector profile exists
  - this fallback is temporary, execution-context-only, and must not be widened into generic silent repair for non-A-share symbols or unrelated contract drift

## Rule Layer Separation

- Universal rules:
  - no cross-account contamination
  - no hidden repair of malformed upstream contracts
  - no execution facts without execution evidence
- Project rules:
  - portfolio-core stages must stay decomposed as `P10`, `P11`, `P12`
  - downstream execution stages must remain explicit and auditable
  - current branch truth belongs to `CURRENT_STATUS.md`, not historical summaries
- Task rules for this slice:
  - `P14` only enriches the formal `P13` package
  - `P14` must not invoke runtime execution
  - `P14` must keep hold rows visible and non-executable
- Temporary assumptions:
  - the first `P14` version may derive enrichment fields deterministically from `P13` rows plus one required `analysis_date`
  - the first `P14` version may also derive one minimum `execution_apply_context` from governed routing metadata plus the approved `analysis_date`, including the bounded A-share fallback defined above when taxonomy coverage is absent
  - richer broker fields, approval refs, and actual trade-result fields remain deferred

## Rejection Boundary

- `P14` must hard-fail when `portfolio_execution_request_package.account_id` is missing
- `P14` must hard-fail when `portfolio_execution_request_package.portfolio_execution_preview_ref` is missing
- `P14` must hard-fail when `portfolio_execution_request_package.portfolio_allocation_decision_ref` is missing
- `P14` must hard-fail when request rows contain unsupported `request_action` or `request_status`
- `P14` must hard-fail when `analysis_date` is blank
- `P14` must hard-fail when row counts and bundle summary counts do not reconcile
- `P14` must hard-fail when callers try to bypass the formal `P13` package by adding raw execution fragment fields to the request contract

## Readiness Semantics

- `readiness_status = ready` only when all enrichable request rows are valid and blockers are empty
- `readiness_status = blocked` when any package-level hard-governance check fails
- `non_executable_hold` rows do not block the bundle by default, but must remain explicit

## Acceptance

### Pre-Implementation Gate

- this document must explicitly define:
  - public tool name
  - legal request input
  - legal output sections
  - rejection conditions
  - non-goals
  - minimum verification commands
- invalid work to reject before implementation:
  - any proposal that bypasses the `P13` package
  - any proposal that invokes `security_execution_record`
  - any proposal that writes runtime execution facts

### Pre-Completion Gate For The Future Implementation

- catalog exposes `security_portfolio_execution_request_enrichment`
- dispatcher accepts the request and returns the named result wrapper
- happy-path CLI test proves `P14` enriches one governed `P13` package into one formal enriched request bundle
- hard-fail tests prove rejection of:
  - malformed lineage refs
  - unsupported request action/status drift
  - blank `analysis_date`
  - mismatched bundle summary counts
- branch truth and task journal are updated if implementation changes accepted health or next-step guidance

## Minimum Verification Entry For The Future Implementation

- focused:
  - `cargo test --test security_portfolio_execution_request_enrichment_cli -- --nocapture`
- downstream chain confidence:
  - `cargo test --test security_portfolio_execution_request_package_cli -- --nocapture`
  - `cargo test --test security_portfolio_execution_preview_cli -- --nocapture`
- portfolio-core confidence:
  - `$env:CARGO_TARGET_DIR='E:\SM\target_p12_enhanced'; cargo test --test security_account_objective_contract_cli --test security_portfolio_replacement_plan_cli --test security_portfolio_allocation_decision_cli -- --nocapture`

## Next Skill

- Immediate next skill after this design freeze: `writing-plans`
- Immediate next engineering workflow after planning: `test-driven-development`
