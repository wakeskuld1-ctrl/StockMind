# P15 Portfolio Execution Apply Bridge Design

## Status

- Date: `2026-04-21`
- Phase: `P15`
- Approved route: `Option 1 - thin governed execution apply bridge`
- Upstream prerequisite:
  - `P10 -> P11 -> P12` portfolio-core chain is already implemented and guarded in this branch
  - the post-`P12` preview-only bridge is already implemented
  - `P13` formal request bridge is already implemented
  - `P14` request enrichment bridge is already implemented
  - latest branch-health truth currently records one verified full-suite green run on this branch
- This document freezes the `P15` design and execution contract before implementation starts

## Intent

- Goal: add one formal `P15` bridge that applies the governed `P14` enriched request bundle into runtime-backed execution records through the existing `security_execution_record` mainline
- Scope:
  - consume only `SecurityPortfolioExecutionRequestEnrichmentDocument`
  - pre-validate the enriched bundle before the first runtime write
  - apply only `ready_for_apply` rows through the formal `security_execution_record` path
  - keep `non_executable_hold` rows explicit and skipped
  - emit one batch-level apply document with row-level runtime refs and apply statuses
  - expose the new apply bridge on the public stock tool surface
- Non-goals:
  - do not call any external broker API
  - do not add approval workflow or operator sign-off in this phase
  - do not promise batch-level atomic rollback across multiple symbols
  - do not add partial-fill, cancel, retry, or execution-scheduling engines
  - do not reopen `P10/P11/P12/P13/P14` contracts unless required by approved `P15` acceptance
  - do not replace or rewrite `security_execution_record` runtime ownership
- Success definition: the next implementation session can build `P15` without re-debating whether this phase should become broker integration, approval workflow, or a second enrichment layer
- Delivery form: one design-freeze document that acts as the governing contract for the next implementation slice

## Single Source Of Truth

- Historical phase context:
  - `E:/SM/docs/handoff/AI_HANDOFF.md`
- Current branch health:
  - `E:/SM/docs/handoff/CURRENT_STATUS.md`
- Stable upstream contracts:
  - `E:/SM/src/ops/security_portfolio_execution_request_enrichment.rs`
  - `E:/SM/src/ops/security_execution_record.rs`
  - `E:/SM/src/runtime/formal_security_runtime_registry.rs`
- Existing upstream acceptance:
  - `E:/SM/tests/security_portfolio_execution_request_enrichment_cli.rs`
  - `E:/SM/tests/security_execution_record_cli.rs`
  - `E:/SM/tests/stock_formal_boundary_manifest_source_guard.rs`
- Existing approved design baselines:
  - `E:/SM/docs/plans/2026-04-20-p13-portfolio-execution-request-bridge-design.md`
  - `E:/SM/docs/plans/2026-04-20-p14-execution-request-enrichment-bridge-design.md`

## Two Approaches

### Option 1: Thin Governed Execution Apply Bridge

- Core idea:
  - consume the formal `P14` enrichment bundle
  - run one preflight validation pass across the whole bundle
  - map each `ready_for_apply` row into the existing `security_execution_record` contract
  - return one batch-level apply result with explicit row statuses and runtime refs
- Pros:
  - directly continues the already approved `P13 -> P14 -> P15` execution trajectory
  - reuses the existing runtime-owned execution mainline instead of creating a second execution runtime path
  - keeps `P15` bounded to apply orchestration rather than reopening execution semantics
- Cons:
  - still needs explicit handling for skipped holds and possible per-row runtime failures
  - batch semantics must be documented carefully so callers do not assume hidden atomicity
- Risks:
  - if the contract is vague, callers may misread `apply` as real broker execution instead of governed runtime execution recording
  - if preflight is weak, malformed rows could start partial runtime writes before the bridge rejects the bundle

### Option 2: Approval-Gated Apply Bridge

- Core idea:
  - extend `P15` to require approval/operator metadata before any apply work starts
  - combine request apply with approval and audit gating in one new stage
- Pros:
  - stronger governance framing
  - could support future approval and operator workflows more directly
- Cons:
  - widens the semantic surface beyond the currently approved minimal apply target
  - delays the formal apply bridge by coupling it to a separate approval lane
- Risks:
  - easy to turn `P15` into `P15 + later governance work` and lose the current mainline focus

## Decision

- Chosen approach: `Option 1 - thin governed execution apply bridge`
- Why this route is the correct continuation:
  - `P13` froze the request package
  - `P14` froze the enrichment gap without writing runtime execution facts
  - the next aligned step is therefore to apply only the approved enriched request bundle into the already existing execution-record mainline
- Rejected alternative:
  - `Option 2 - approval-gated apply bridge`
  - rejected for this phase because it would widen the contract into approval workflow while the minimal apply boundary is still not frozen
- Known tradeoffs:
  - `P15` will write runtime-backed execution records, so it is no longer side-effect free
  - `P15` still does not mean broker execution, fill lifecycle, or trade reconciliation
  - `P15` must state clearly that there is no new cross-symbol atomic rollback guarantee in this phase

## Current Design Blocker

- the current `P14` enriched row does not yet carry enough execution-context fields to build a valid `SecurityExecutionRecordRequest` on its own
- concrete missing context relative to the current execution-record / execution-journal path includes:
  - market and sector lookup anchors such as `market_symbol` and `sector_symbol`
  - feature / outcome anchor fields such as `as_of_date`, `market_regime`, and `sector_template`
  - lifecycle-context refs such as `position_plan_ref` or equivalent binding data if the apply path wants to use the lifecycle compatibility route
- therefore `Option 1` remains the approved phase direction, but the implementation route still needs one explicit context-sourcing decision before code starts
- bounded remediation options:
  - extend the upstream `P14` document so the apply bridge receives a legal execution-request subset plus the missing context directly
  - add one `P15` context resolver that reconstructs the missing execution-request context from approved upstream runtime or package sources before invoking `security_execution_record`

## Execution Contract

- Chosen approach:
  - add one new formal tool downstream of the `P14` enrichment bundle
  - consume only `SecurityPortfolioExecutionRequestEnrichmentDocument`
  - preflight-validate the whole bundle before the first runtime write
  - apply only `ready_for_apply` rows by invoking the existing `security_execution_record`
  - emit one batch-level apply document with explicit runtime refs, skipped holds, and failure surfaces
- Allowed change boundary:
  - create one new ops module for the apply bridge
  - wire it into `stock.rs`, `stock_execution_and_position_management.rs`, catalog, and dispatcher
  - add one dedicated CLI test file
  - update `tests/stock_formal_boundary_manifest_source_guard.rs`
  - update governance / handoff truth only if implementation actually lands and changes accepted branch truth
- Explicit non-goals:
  - no external broker routing
  - no execution retry queue
  - no approval bundle
  - no rollback layer that rewrites or deletes already persisted execution facts
  - no bypass of `security_execution_record`
- Best-practice path expected for this route:
  - `P15` must consume the formal `P14` bundle only
  - `P15` must preserve lineage back to `P14`, `P13`, preview, and governed allocation decision
  - `P15` must hard-reject blocked bundles before the first runtime write
  - `non_executable_hold` rows must remain visible and must not write execution runtime facts
  - `P15` must use the existing `security_execution_record` mainline instead of creating an ad hoc runtime write path
  - `P15` must report batch-level non-atomicity truthfully instead of implying hidden rollback
- Acceptance checks for route conformance:
  - the public request shell contains only the `P14` enrichment document plus metadata
  - the output is an apply-bridge document, not a broker execution report
  - happy-path and hard-fail tests prove that `P15` applies governed enriched rows through `security_execution_record` instead of bypassing it

## Proposed Public Tool

- Tool name: `security_portfolio_execution_apply_bridge`
- Stage meaning: `P15` governed execution apply bridge
- Tool responsibility: convert the formal `P14` enriched request bundle into runtime-backed execution records through the existing execution-record mainline while preserving explicit batch traceability

## Contract

### Request Contract

- Proposed type: `SecurityPortfolioExecutionApplyBridgeRequest`
- Required fields:
  - `portfolio_execution_request_enrichment: SecurityPortfolioExecutionRequestEnrichmentDocument`
  - `created_at: String`
- Compatibility rule:
  - `P15` request must not accept raw `P13` request rows, raw preview rows, direct broker payloads, or hand-built execution fact fragments
  - `P15` must treat the `P14` enrichment document as the only legal upstream source

### Output Contract

- Proposed document type: `security_portfolio_execution_apply_bridge`
- Proposed version: `security_portfolio_execution_apply_bridge.v1`
- Proposed primary document: `SecurityPortfolioExecutionApplyBridgeDocument`
- Proposed wrapper: `SecurityPortfolioExecutionApplyBridgeResult`

### Required Output Sections

- Identity and lineage:
  - `portfolio_execution_apply_bridge_id`
  - `contract_version`
  - `document_type`
  - `generated_at`
  - `analysis_date`
  - `account_id`
  - `portfolio_execution_request_enrichment_ref`
  - `portfolio_execution_request_package_ref`
  - `portfolio_execution_preview_ref`
  - `portfolio_allocation_decision_ref`
- Apply rows:
  - `apply_rows`
  - `applied_count`
  - `skipped_hold_count`
  - `failed_apply_count`
- Governance checks:
  - `apply_status`
  - `blockers`
  - `non_atomicity_notice`
- Traceability and rationale:
  - `apply_rationale`
  - `apply_summary`

### Proposed Supporting Rows

- `SecurityPortfolioExecutionApplyRow`
  - `symbol`
  - `request_action`
  - `requested_gross_pct`
  - `enrichment_status`
  - `apply_status`
  - `execution_record_ref`
  - `execution_journal_ref`
  - `apply_summary`
- apply-status rules:
  - `ready_for_apply` row -> `applied` only after `security_execution_record` succeeds
  - `non_executable_hold` row -> `skipped_non_executable_hold`
  - bundle with any `blocked` row -> reject before apply starts
  - runtime failure after apply starts -> batch status must surface `partial_apply_failure` without claiming rollback

## Rule Layer Separation

- Universal rules:
  - no cross-account contamination
  - no hidden repair of malformed upstream contracts
  - no execution facts without execution evidence
- Project rules:
  - portfolio-core stages must stay decomposed as `P10`, `P11`, `P12`
  - downstream execution stages must remain explicit and auditable
  - current branch truth belongs to `CURRENT_STATUS.md`, not historical summaries
  - runtime writes must stay on the existing formal stock runtime path
- Task rules for this slice:
  - `P15` only applies the formal `P14` enrichment document
  - `P15` must not become broker execution, approval workflow, or retry scheduling
  - `P15` must skip `non_executable_hold` rows explicitly
  - `P15` must preflight the bundle before the first write
- Temporary assumptions:
  - the first `P15` version may derive one legal `SecurityExecutionRecordRequest` per ready row only after the missing execution-context sourcing route is explicitly frozen
  - richer operator metadata, approval refs, and external execution-routing data remain deferred

## Rejection Boundary

- `P15` must hard-fail when `portfolio_execution_request_enrichment.account_id` is missing
- `P15` must hard-fail when any lineage ref required by `P14` is missing
- `P15` must hard-fail when `portfolio_execution_request_enrichment.readiness_status` is incompatible with apply because blocked rows exist
- `P15` must hard-fail when `ready_for_apply_count`, `non_executable_hold_count`, or `blocked_enrichment_count` do not reconcile with row observations
- `P15` must hard-fail when apply rows contain unsupported `enrichment_status`
- `P15` must hard-fail when callers try to bypass the formal `P14` bundle by adding raw execution fragment fields to the request contract

## Apply Semantics

- `apply_status = applied` only when all ready rows completed through `security_execution_record` and failed count is zero
- `apply_status = partial_apply_failure` when preflight passed, at least one runtime-backed apply succeeded, and a later ready row failed during apply
- `apply_status = rejected` when bundle-level validation fails before the first runtime write
- `apply_status = applied_with_skipped_holds` when all ready rows applied and one or more hold rows were explicitly skipped
- `non_atomicity_notice` must state that this phase does not introduce cross-symbol rollback semantics

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
  - any proposal that bypasses the `P14` enrichment bundle
  - any proposal that writes runtime facts without `security_execution_record`
  - any proposal that silently upgrades hold rows into executable rows
  - any proposal that claims batch atomic rollback without implementing and proving it

### Pre-Completion Gate For The Future Implementation

- catalog exposes `security_portfolio_execution_apply_bridge`
- dispatcher accepts the request and returns the named result wrapper
- happy-path CLI test proves `P15` applies one governed `P14` bundle into runtime-backed execution records
- hold-semantics CLI test proves `non_executable_hold` rows are skipped and do not create execution records
- hard-fail tests prove rejection of:
  - malformed enrichment lineage
  - blocked bundle apply
  - mismatched apply summary counts
  - unsupported enrichment status drift
- source-guard coverage proves the approved `P15` module was added to the frozen stock boundary
- branch truth and task journal are updated if implementation changes accepted health or next-step guidance

## Minimum Verification Entry For The Future Implementation

- focused:
  - `cargo test --test security_portfolio_execution_apply_bridge_cli -- --nocapture`
- downstream chain confidence:
  - `cargo test --test security_portfolio_execution_request_enrichment_cli -- --nocapture`
  - `cargo test --test security_execution_record_cli -- --nocapture`
- boundary confidence:
  - `cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture`
- portfolio-core confidence:
  - `$env:CARGO_TARGET_DIR='E:\SM\target_p12_enhanced'; cargo test --test security_account_objective_contract_cli --test security_portfolio_replacement_plan_cli --test security_portfolio_allocation_decision_cli -- --nocapture`

## Initial File Plan For The Future Implementation

- Create: `E:/SM/src/ops/security_portfolio_execution_apply_bridge.rs`
- Modify: `E:/SM/src/ops/stock.rs`
- Modify: `E:/SM/src/ops/stock_execution_and_position_management.rs`
- Modify: `E:/SM/src/tools/catalog.rs`
- Modify: `E:/SM/src/tools/dispatcher.rs`
- Modify: `E:/SM/src/tools/dispatcher/stock_ops.rs`
- Create: `E:/SM/tests/security_portfolio_execution_apply_bridge_cli.rs`
- Modify: `E:/SM/tests/stock_formal_boundary_manifest_source_guard.rs`
- Modify if implementation changes governance truth:
  - `E:/SM/docs/governance/contract_registry.md`
  - `E:/SM/docs/governance/decision_log.md`
  - `E:/SM/docs/handoff/CURRENT_STATUS.md`
  - `E:/SM/docs/handoff/HANDOFF_ISSUES.md`
  - `E:/SM/CHANGELOG_TASK.MD`

## Next Skill

- Immediate next skill after this design freeze: `writing-plans`
- Immediate next engineering workflow after planning: `test-driven-development`
