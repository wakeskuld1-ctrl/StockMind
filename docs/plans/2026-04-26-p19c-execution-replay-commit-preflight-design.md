# P19C Execution Replay Commit Preflight Design

## Intent
- Goal: add P19C as a side-effect-free replay commit preflight boundary after P19B dry-run validation.
- Scope: consume the P19B `SecurityPortfolioExecutionReplayExecutorDocument` and the matching P14 `SecurityPortfolioExecutionRequestEnrichmentDocument`, verify that replay rows can be mapped to structured execution-record request inputs, freeze canonical commit payload hashes, and define durable idempotency candidates for a later P19D runtime writer.
- Non-goals: do not write runtime facts, call `security_execution_record`, create a replay commit ledger, submit broker orders, replay broker fills, materialize positions, close lifecycle, mutate P15/P16/P17/P18/P19A/P19B artifacts, or claim bundle rollback semantics.
- Success definition: callers can prove which P19B dry-run rows have enough structured commit input to become future runtime writes, which rows are blocked, and what canonical payload hash/idempotency key P19D must honor.
- Delivery form: design doc, implementation plan, later Rust module, CLI tests, stock boundary wiring, governance docs, handoff notes, and append-only task journal entry.

## Contract
- Tool name: `security_portfolio_execution_replay_commit_preflight`.
- Request contract: `SecurityPortfolioExecutionReplayCommitPreflightRequest`.
- Primary output contract: `SecurityPortfolioExecutionReplayCommitPreflightDocument` wrapped by `SecurityPortfolioExecutionReplayCommitPreflightResult`.
- Required inputs:
  - one P19B `SecurityPortfolioExecutionReplayExecutorDocument`
  - one P14 `SecurityPortfolioExecutionRequestEnrichmentDocument`
  - `preflight_mode = "commit_preflight_only"`
- Required lineage:
  - P19B `portfolio_execution_replay_request_package_ref`
  - P19B `portfolio_execution_request_enrichment_ref`
  - P19B `portfolio_execution_request_package_ref`
  - P19B `portfolio_execution_preview_ref`
  - P19B `portfolio_allocation_decision_ref`
  - P14 `portfolio_execution_request_enrichment_id`
  - P14 `portfolio_execution_request_package_ref`
  - P14 `portfolio_execution_preview_ref`
  - P14 `portfolio_allocation_decision_ref`
- P19C must verify P19B and P14 lineage equality before inspecting rows.
- P19C must only accept P19B documents with:
  - `document_type == "security_portfolio_execution_replay_executor"`
  - `execution_mode == "dry_run"`
  - `runtime_write_count == 0`
  - no `runtime_execution_record_ref` on any executor row
  - `dry_run_status` in `no_replay_work` or `validated_for_dry_run`
- P19C must reject `dry_run_status = "no_replay_work"` when P19B still contains executor rows.
- P19C must only preflight executor rows with `dry_run_status == "validated_for_dry_run"`.
- P19C must match each executor row to exactly one P14 enriched row by account, analysis date, symbol, request action, and requested/executed gross percent.
- P19C must reject matched P14 rows unless `enrichment_status == "ready_for_apply"`.
- P19C must reject rows with missing `execution_apply_context`, missing `as_of_date`, missing `execution_action`, non-positive executed gross percent, or missing replay evidence.
- P19C must build a canonical commit payload preview from the structured P14 enriched row and P19B replay evidence. The preview may mirror the future `SecurityExecutionRecordRequest`, but it must remain a document payload only and must not call `security_execution_record`.
- P19C must derive:
  - `commit_idempotency_key`
  - `canonical_commit_payload_hash`
  - `planned_execution_record_ref`
  - `source_replay_executor_row_key`
- P19C must inherit and verify each P19B deterministic row idempotency key as a source input, but it must not treat the P19B key as durable runtime idempotency.
- P19C must reject duplicate `commit_idempotency_key` values inside one preflight document.
- P19C must hard-fail if the same `commit_idempotency_key` appears with different canonical payload hashes inside one preflight document.
- P19C output statuses:
  - `no_commit_work`
  - `commit_preflight_ready`
  - `commit_preflight_blocked`
- Row statuses:
  - `preflight_ready`
  - `preflight_blocked`
- Rejection conditions:
  - unsupported `preflight_mode`
  - any request or document field that tries to authorize `execution_mode = "commit"`
  - missing or mismatched lineage
  - P19B is not dry-run-only
  - P19B already contains runtime refs
  - unsupported P19B document or row status
  - P19B `no_replay_work` status with non-empty executor rows
  - P14 bundle is blocked or has summary drift
  - no exact P14 row match for a P19B executor row
  - multiple P14 row matches for one executor row
  - matched P14 row is not `ready_for_apply`
  - missing structured apply context
  - missing replay evidence
  - duplicate or conflicting commit idempotency keys
- Traceability requirements: P19C must preserve P19B, P19A, P18, P17, P16, P15, P14, P13, preview, and P12 refs in the output document.
- Compatibility zones: P19C must be a new public stock-bus tool placed after P19B in execution-and-position-management ordering. It must not extend P19B to accept `commit`.

## Hard Rejection Red Lines
- Do not modify P19B so `execution_mode = "commit"` stops hard-failing.
- Do not call or wrap `security_execution_record`.
- Do not write `SecurityExecutionStore`, runtime ledger rows, execution records, position records, broker fills, or lifecycle artifacts.
- Do not output non-empty `runtime_execution_record_ref`.
- Do not rename planned/preflight refs as runtime refs.
- Do not implement rollback, partial commit, retry execution, broker response handling, position materialization, or lifecycle closeout.
- Do not consume P19A/P18 directly and skip P19B dry-run truth.
- Do not claim durable idempotency; P19C only freezes candidate keys and payload hashes for P19D.
- Do not claim completion from happy-path P19C tests without boundary, catalog, dispatcher, governance, and handoff synchronization.

## Decision
- Chosen approach: P19C as commit-preflight-only boundary, with actual runtime write deferred to P19D.
- Why: P19B proves dry-run eligibility but does not carry enough structured runtime-write input or durable idempotency state. P19C freezes the missing commit contract before any runtime write exists.
- Rejected alternative: directly enabling `execution_mode = "commit"` in P19B. It would mix dry-run validation, runtime writes, durable idempotency, and partial failure semantics into one already-approved dry-run tool.
- Rejected alternative: P19C per-row partial commit through `security_execution_record`. It is faster but unsafe before durable idempotency, canonical payload hashing, already-committed detection, and failure semantics are explicit.
- Rejected alternative: P19C bundle-atomic commit. Existing `security_execution_record` owns its own session and commit; bundle rollback would require a separate batch writer or refactor that is out of scope for P19C.
- Known tradeoff: P19C still does not reduce unresolved runtime state; it only turns dry-run eligibility into commit readiness evidence.
- Open question deferred to P19D: whether runtime write should be per-row partial commit with durable ledger statuses or a true bundle-atomic writer with one transaction boundary.

## Acceptance
- Before implementation starts:
  - this design document exists under `docs/plans/`
  - an implementation plan exists under `docs/plans/`
  - P19B and P14 source structures have been inspected as upstream style and field sources
  - P19B follow-up repository-wide regression has passed in the current worktree
  - Independent Risk Pass has been completed and incorporated
- Before completion can be claimed:
  - P19C tests are written before production code and observed red for missing tool/module behavior
  - implementation makes focused P19C tests green
  - tests prove P19C refuses to call or emulate `security_execution_record`
  - tests prove P19C rejects P19B documents with runtime refs or non-dry-run mode
  - tests prove P19C rejects missing or ambiguous P14 matches
  - tests prove P19C derives stable commit idempotency keys and canonical payload hashes
  - tests prove duplicate/conflicting idempotency keys are rejected
  - public stock boundary and grouping guards are updated and green
  - `cargo check` succeeds in an isolated target dir
  - governance docs and handoff notes record that P19C is preflight-only and not runtime replay
  - `.trae/CHANGELOG_TASK.md` receives an append-only task entry
- Completion must be refused or softened if only focused tests pass and `cargo check` or source guards are not run.

## Cross-Artifact Contract

| Boundary Item | Source Of Truth | Runtime Entrypoints | Frozen/Derived Artifacts | Guard Tests | Required Sync |
|---|---|---|---|---|---|
| `security_portfolio_execution_replay_commit_preflight` | this design, implementation module, P19C CLI tests | `src/ops/stock.rs`, `src/ops/stock_execution_and_position_management.rs`, `src/tools/catalog.rs`, `src/tools/dispatcher.rs`, `src/tools/dispatcher/stock_ops.rs` | `tests/stock_formal_boundary_manifest_source_guard.rs`, contract registry, decision log, handoff status/issues, task journal | P19C CLI test, stock formal boundary guard, catalog grouping guard, dispatcher grouping guard, `cargo check`, repository regression after P19C if approved | add module, group export, catalog entry, dispatcher route, frozen manifest entry, governance rows, handoff notes |
| P19C preflight contract fields | `src/ops/security_portfolio_execution_replay_commit_preflight.rs` | CLI dispatcher request/response | tests and docs that freeze preflight-only semantics | P19C CLI tests | request/result/document/row/error structs, rejection tests, non-runtime-write tests |

## Independent Risk Pass
- Mode: `user-approved-subagent`
- Trigger: P19C touches a future commit boundary, public stock-bus routing, idempotency semantics, and the boundary between side-effect-free artifacts and runtime writes.
- Fresh-context question: What artifact, runtime fact, or source guard can drift if P19C turns dry-run replay eligibility into commit readiness evidence?
- Findings:
  - P19C naming must include `preflight`; a name such as `replay_commit_executor` would imply runtime write authority and is rejected for this phase.
  - P19B document-local idempotency is not durable runtime idempotency.
  - P19B rows do not contain all structured fields needed to safely call `security_execution_record`.
  - `security_execution_record` owns its own session and commit, so P19C must not claim bundle rollback.
  - `planned_execution_record_ref` is not a runtime ref and must not be treated as already written.
  - A new public P19C tool must update stock boundary, grouping, catalog, dispatcher, frozen manifest, governance docs, and handoff together.
- Blocking gaps:
  - Durable replay commit ledger schema is not defined in P19C and must be deferred to P19D.
  - Already-committed detection across process restarts is not defined in P19C and must be deferred to P19D.
  - Runtime write failure semantics are not implemented in P19C and must remain a P19D design choice.

## Next Skill
- Use `writing-plans` to create a P19C implementation plan.
- Do not use `test-driven-development` or edit production code until the design and plan are approved.
