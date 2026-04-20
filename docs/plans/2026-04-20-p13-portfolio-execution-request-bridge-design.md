# P13 Portfolio Execution Request Bridge Design

## Status

- Date: 2026-04-20
- Phase: `P13`
- Approved route: `Option 1 - formal side-effect-free execution request bridge`
- Upstream prerequisite:
  - `P10 -> P11 -> P12` portfolio-core chain is already implemented and guarded in this branch
  - the post-`P12` preview-only bridge is already implemented
  - the preview rows already carry one nested request-aligned preview subset
- This document freezes the `P13` design and execution contract before implementation starts

## Intent

- Goal: add one formal `P13` bridge that upgrades governed post-`P12` preview output into a portfolio-level execution request package
- Scope:
  - consume only `SecurityPortfolioExecutionPreviewDocument`
  - emit one formal portfolio execution request package document
  - keep the current no-side-effect boundary
  - preserve explicit traceability back to `P12`
  - make the new bridge discoverable on the public stock tool surface
- Non-goals:
  - do not call `security_execution_record`
  - do not write execution runtime stores
  - do not persist real execution requests
  - do not reopen `P10/P11/P12` portfolio-core logic
  - do not replace the preview bridge with an approval or audit package
- Success definition: the next implementation session can build `P13` without re-debating whether the next stage should be approval-first, preview-only, or already-real execution
- Delivery form: one design-freeze document that acts as the governing contract for the next implementation slice

## Single Source Of Truth

- Historical phase context:
  - `E:/SM/docs/handoff/AI_HANDOFF.md`
- Current branch health:
  - `E:/SM/docs/handoff/CURRENT_STATUS.md`
- Stable upstream contracts:
  - `E:/SM/src/ops/security_portfolio_allocation_decision.rs`
  - `E:/SM/src/ops/security_portfolio_execution_preview.rs`
- Existing upstream acceptance:
  - `E:/SM/tests/security_portfolio_allocation_decision_cli.rs`
  - `E:/SM/tests/security_portfolio_execution_preview_cli.rs`
- Existing approved design baselines:
  - `E:/SM/docs/plans/2026-04-20-p12-governed-portfolio-allocation-decision-design.md`
  - `E:/SM/docs/plans/2026-04-20-post-p12-portfolio-execution-preview-design.md`
  - `E:/SM/docs/plans/2026-04-20-post-p12-execution-request-preview-standardization-design.md`

## Two Approaches

### Option 1: Formal Portfolio Execution Request Bridge

- Core idea:
  - consume the standardized preview document
  - emit one formal package that groups portfolio-level execution request rows plus readiness and blockers
- Pros:
  - directly continues the already approved preview-to-request trajectory
  - creates the cleanest contract for a later real execution bridge
  - keeps side effects out while still moving the mainline forward
- Cons:
  - adds one new formal document family and row contract
  - requires explicit separation between `ready_request` and non-executable `hold` rows
- Risks:
  - if naming is sloppy, consumers may confuse request packaging with already-executed facts

### Option 2: Portfolio Rebalance Approval / Audit Bundle

- Core idea:
  - wrap the preview output inside a richer approval and audit-oriented bundle
  - prioritize review semantics ahead of execution-request formalization
- Pros:
  - strong human-review and audit framing
  - could support later governance workflows
- Cons:
  - does not continue the already approved execution-bridge direction as directly
  - leaves the main execution-request contract still undefined
- Risks:
  - easy to drift off the current mainline and turn the next phase into a documentation-heavy detour

## Decision

- Chosen approach: `Option 1 - formal side-effect-free execution request bridge`
- Why this route is the correct continuation:
  - `P12` explicitly stopped before `P13+`
  - the first approved downstream step after `P12` was the preview-only bridge
  - the second approved downstream step standardized the preview rows to prepare a future execution bridge
  - therefore the next aligned step is a formal request bridge, not a different approval-first lane
- Rejected alternative:
  - `Option 2 - Portfolio Rebalance Approval / Audit Bundle`
  - rejected for this phase because it would widen the semantic surface while leaving the execution-request contract unfrozen
- Known tradeoffs:
  - `P13` still will not execute trades
  - `P13` will formalize request packaging, not persistence or runtime execution

## Execution Contract

- Chosen approach:
  - add one new formal tool downstream of the standardized preview document
  - consume only `SecurityPortfolioExecutionPreviewDocument`
  - emit a side-effect-free execution request package plus row-level request metadata
- Allowed change boundary:
  - create one new ops module for the request bridge
  - wire it into `stock.rs`, `stock_execution_and_position_management.rs`, catalog, and dispatcher
  - add one dedicated CLI test file
  - update governance / handoff truth only if implementation actually lands and changes accepted branch truth
- Explicit non-goals:
  - no runtime writes
  - no execution store integration
  - no real execution fact generation
  - no changes to `P12` or preview contracts in this round unless required by approved `P13` acceptance
- Best-practice path expected for this route:
  - `P13` must consume the formal preview document only
  - `P13` must preserve lineage back to the preview document and the governed allocation decision
  - `P13` must keep request rows explicit for `buy`, `sell`, and `hold`
  - `hold` must remain visible but must not be misrepresented as a ready executable request
  - `P13` must fail hard on malformed preview input instead of repairing it silently
- Acceptance checks for route conformance:
  - the public request shell contains only the preview document plus metadata
  - the output is a request package document, not an execution record
  - happy-path and hard-fail tests prove that `P13` packages governed preview rows instead of rebuilding from raw portfolio-core fragments

## Proposed Public Tool

- Tool name: `security_portfolio_execution_request_package`
- Stage meaning: `P13` portfolio execution request bridge
- Tool responsibility: convert the standardized preview-only post-`P12` output into one formal portfolio execution request package that is ready for later approval or runtime execution-bridge work

## Contract

### Request Contract

- Proposed type: `SecurityPortfolioExecutionRequestPackageRequest`
- Required fields:
  - `portfolio_execution_preview: SecurityPortfolioExecutionPreviewDocument`
  - `created_at: String`
- Compatibility rule:
  - `P13` request must not accept raw `P12` rows, raw candidate fragments, or direct runtime execution data
  - `P13` must treat the preview document as the only legal upstream source

### Output Contract

- Proposed document type: `security_portfolio_execution_request_package`
- Proposed version: `security_portfolio_execution_request_package.v1`
- Proposed primary document: `SecurityPortfolioExecutionRequestPackageDocument`
- Proposed wrapper: `SecurityPortfolioExecutionRequestPackageResult`

### Required Output Sections

- Identity and lineage:
  - `portfolio_execution_request_package_id`
  - `contract_version`
  - `document_type`
  - `generated_at`
  - `account_id`
  - `portfolio_execution_preview_ref`
  - `portfolio_allocation_decision_ref`
- Request rows:
  - `request_rows`
  - `ready_request_count`
  - `blocked_request_count`
  - `hold_request_count`
- Governance checks:
  - `readiness_status`
  - `blockers`
- Traceability and rationale:
  - `request_rationale`
  - `request_summary`

### Proposed Supporting Rows

- `SecurityPortfolioExecutionRequestRow`
  - `symbol`
  - `request_action`
  - `requested_gross_pct`
  - `request_status`
  - `request_summary`
  - `source_preview_action`
  - `source_weight_delta_pct`
- request-action rules:
  - `buy` preview -> `buy` request
  - `sell` preview -> `sell` request
  - `hold` preview -> `hold` request row with non-executable status
- request-status rules:
  - `buy` and `sell` default to `ready_request` when preview input is valid
  - `hold` defaults to `non_executable_hold`

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
  - `P13` only packages requests from the formal preview document
  - `P13` must not become an approval bundle or runtime execution layer
  - `P13` must keep `hold` visible without promoting it into a ready executable request
- Temporary assumptions:
  - the first `P13` version may derive request rows directly from `preview_rows`
  - richer batching, broker-routing, approval semantics, and scheduling remain deferred

## Rejection Boundary

- `P13` must hard-fail when `portfolio_execution_preview.account_id` is missing
- `P13` must hard-fail when `portfolio_execution_preview.portfolio_allocation_decision_ref` is missing
- `P13` must hard-fail when preview rows contain unsupported `preview_action`
- `P13` must hard-fail when `requested_gross_pct` cannot be mapped consistently from governed preview deltas
- `P13` must hard-fail when row counts and package summary counts do not reconcile
- `P13` must hard-fail when callers try to bypass the formal preview document by adding raw execution fragment fields to the request contract

## Readiness Semantics

- `readiness_status = ready` only when all executable request rows are valid and blockers are empty
- `readiness_status = blocked` when any package-level hard-governance check fails
- `hold` rows do not block the package by default, but they must remain explicitly non-executable

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
  - any proposal that bypasses the preview document
  - any proposal that turns `P13` into an approval-first bundle
  - any proposal that writes runtime execution facts

### Pre-Completion Gate For The Future Implementation

- catalog exposes `security_portfolio_execution_request_package`
- dispatcher accepts the request and returns the named result wrapper
- happy-path CLI test proves `P13` packages one governed preview document into one formal execution request package
- hard-fail tests prove rejection of:
  - malformed preview lineage
  - unsupported preview action drift
  - mismatched request summary counts
- branch truth and task journal are updated if implementation changes accepted health or next-step guidance

## Minimum Verification Entry For The Future Implementation

- focused:
  - `cargo test --test security_portfolio_execution_request_package_cli -- --nocapture`
- downstream chain confidence:
  - `cargo test --test security_portfolio_execution_preview_cli -- --nocapture`
- portfolio-core confidence:
  - `$env:CARGO_TARGET_DIR='E:\SM\target_p12_enhanced'; cargo test --test security_account_objective_contract_cli --test security_portfolio_replacement_plan_cli --test security_portfolio_allocation_decision_cli -- --nocapture`
- repository-level confidence after landing:
  - `$env:CARGO_TARGET_DIR='E:\SM\target_p12_enhanced'; cargo test -- --nocapture`

## Initial File Plan For The Future Implementation

- Create: `E:/SM/src/ops/security_portfolio_execution_request_package.rs`
- Modify: `E:/SM/src/ops/stock.rs`
- Modify: `E:/SM/src/ops/stock_execution_and_position_management.rs`
- Modify: `E:/SM/src/tools/catalog.rs`
- Modify: `E:/SM/src/tools/dispatcher.rs`
- Modify: `E:/SM/src/tools/dispatcher/stock_ops.rs`
- Create: `E:/SM/tests/security_portfolio_execution_request_package_cli.rs`
- Modify if implementation changes governance truth:
  - `E:/SM/docs/governance/contract_registry.md`
  - `E:/SM/docs/governance/decision_log.md`
  - `E:/SM/docs/handoff/CURRENT_STATUS.md`
  - `E:/SM/docs/handoff/HANDOFF_ISSUES.md`
  - `E:/SM/CHANGELOG_TASK.MD`

## Next Skill

- Immediate next skill after this design freeze: `writing-plans`
- Immediate next engineering workflow after planning: `test-driven-development`
