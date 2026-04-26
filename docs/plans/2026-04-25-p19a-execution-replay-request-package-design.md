# P19A Execution Replay Request Package Design

## Intent
- Goal: add a side-effect-free P19A replay-request package downstream of the P18 repair-intent package.
- Scope: consume `SecurityPortfolioExecutionRepairPackageDocument`, select only `governed_retry_candidate` rows, and freeze a replay request artifact for a later executor design.
- Non-goals: do not write runtime facts, call `security_execution_record`, replay broker fills, materialize positions, retry execution, or close lifecycle.
- Success definition: P19A exposes a public stock-bus tool that turns retryable P18 repair intent into explicit replay request rows and excludes manual/governance rows without guessing.
- Delivery form: Rust module, CLI tests, stock boundary wiring, governance docs, handoff notes, and task journal entry.

## Contract
- Tool name: `security_portfolio_execution_replay_request_package`.
- Request contract: `SecurityPortfolioExecutionReplayRequestPackageRequest`.
- Primary output contract: `SecurityPortfolioExecutionReplayRequestPackageDocument` wrapped by `SecurityPortfolioExecutionReplayRequestPackageResult`.
- Input object: one `SecurityPortfolioExecutionRepairPackageDocument`.
- Output object: one side-effect-free replay request package.
- Core row object: `SecurityPortfolioExecutionReplayRequestRow`.
- Eligible rows: only P18 rows where `repair_class == "governed_retry_candidate"`.
- Excluded rows:
  - `manual_follow_up` must remain manual follow-up and must not become replay work.
  - `blocked_pending_decision` must remain governance-blocked and must not become replay work.
  - no-repair P18 documents must produce an empty replay request package.
- Required lineage:
  - `portfolio_execution_repair_package_id`
  - `portfolio_execution_reconciliation_bridge_ref`
  - `portfolio_execution_status_bridge_ref`
  - `portfolio_execution_apply_bridge_ref`
  - `portfolio_execution_request_enrichment_ref`
  - `portfolio_execution_request_package_ref`
  - `portfolio_execution_preview_ref`
  - `portfolio_allocation_decision_ref`
- Required replay evidence for each included row:
  - at least one of `execution_record_ref`, `execution_journal_ref`, or blocker text containing a retry/replay signal.
- Rejection conditions:
  - missing required lineage
  - unsupported P18 `repair_status`
  - P18 summary count drift
  - unknown P18 `repair_class`
  - `repair_status == "no_repair_required"` with non-empty repair rows
  - `repair_status == "repair_required"` with `repair_required_count` drift
  - retry candidate without enough replay evidence
- Traceability requirements: P19A must preserve P18, P17, P16, P15, P14, P13, preview, and P12 refs in the output document.
- Compatibility zones: `src/ops/stock.rs` remains the public stock module manifest; catalog and dispatcher ordering must keep P19A immediately after P18 in execution-and-position-management.

## Decision
- Chosen approach: A1 strict request package.
- Why: it adds the next formal bridge while keeping replay execution out of scope, matching the existing P13/P14/P16/P17/P18 side-effect-free pattern.
- Rejected alternative: A2 request package plus detailed preflight diagnostics. It is richer but risks leaking executor design into this phase.
- Rejected alternative: direct controlled retry executor. It creates runtime-write, rollback, and broker-fill semantics that require a separate P19B contract.
- Known tradeoff: P19A does not reduce unresolved execution state by itself; it only freezes replay intent for a future executor.
- Open question: whether P19B should consume only P19A replay request rows or also re-read P18 for defensive validation remains deferred.

## Acceptance
- Before implementation starts:
  - this design document exists under `docs/plans/`
  - an implementation plan exists under `docs/plans/`
  - P18 request/output fields and tests have been inspected as the upstream style source
- Before completion can be claimed:
  - P19A tests are written before production code and observed red for missing tool/module behavior
  - implementation makes focused P19A tests green
  - public stock boundary and grouping guards are updated and green
  - `cargo check` succeeds in an isolated target dir
  - governance docs and handoff notes record that P19A is not replay execution
  - `.trae/CHANGELOG_TASK.md` receives an append-only task entry
- Completion must be refused or softened if only focused tests pass and `cargo check` is not run.
