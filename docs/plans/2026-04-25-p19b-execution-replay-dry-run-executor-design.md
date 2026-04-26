# P19B Execution Replay Dry-Run Executor Design

## Intent
- Goal: add P19B as a controlled replay executor boundary that initially supports dry-run validation only.
- Scope: consume `SecurityPortfolioExecutionReplayRequestPackageDocument`, validate replay executor eligibility, freeze deterministic dry-run execution truth, and expose the result as a public stock-bus tool.
- Non-goals: do not write runtime facts, call `security_execution_record`, replay broker fills, materialize positions, retry live execution, mutate P15/P16/P17/P18/P19A artifacts, or close lifecycle.
- Success definition: callers can prove which P19A replay request rows are eligible for a future commit executor and why, with stable idempotency keys and rejection semantics.
- Delivery form: design doc, implementation plan, later Rust module, CLI tests, stock boundary wiring, governance docs, handoff notes, and task journal entry.

## Contract
- Tool name: `security_portfolio_execution_replay_executor`.
- Request contract: `SecurityPortfolioExecutionReplayExecutorRequest`.
- Primary output contract: `SecurityPortfolioExecutionReplayExecutorDocument` wrapped by `SecurityPortfolioExecutionReplayExecutorResult`.
- Input object: one `SecurityPortfolioExecutionReplayRequestPackageDocument`.
- Required request field: `execution_mode`.
- Supported mode in this phase: `dry_run`.
- Explicitly rejected mode in this phase: `commit`.
- Core row object: `SecurityPortfolioExecutionReplayExecutorRow`.
- Required lineage:
  - `portfolio_execution_replay_request_package_id`
  - `portfolio_execution_repair_package_ref`
  - `portfolio_execution_reconciliation_bridge_ref`
  - `portfolio_execution_status_bridge_ref`
  - `portfolio_execution_apply_bridge_ref`
  - `portfolio_execution_request_enrichment_ref`
  - `portfolio_execution_request_package_ref`
  - `portfolio_execution_preview_ref`
  - `portfolio_allocation_decision_ref`
- Required row evidence:
  - `replay_request_status == "ready_for_replay_request"`
  - `repair_class == "governed_retry_candidate"`
  - at least one `replay_evidence_refs` entry
  - deterministic idempotency key derived from account, analysis date, symbol, action, requested gross pct, P19A package ref, and evidence refs
- Output behavior:
  - no rows with `replay_request_status == "no_replay_requested"` produce `dry_run_status = "no_replay_work"`
  - eligible rows produce `dry_run_status = "validated_for_dry_run"`
  - no runtime refs are created; any output ref is a planned/dry-run identifier only
- Rejection conditions:
  - missing required lineage
  - unsupported `execution_mode`
  - `execution_mode == "commit"` in this phase
  - unsupported P19A `replay_request_status`
  - P19A summary count drift
  - row `repair_class` is not `governed_retry_candidate`
  - row `replay_request_status` is not `ready_for_replay_request`
  - row has no replay evidence refs
  - duplicate deterministic idempotency key inside one document
- Traceability requirements: P19B must preserve P19A, P18, P17, P16, P15, P14, P13, preview, and P12 refs in the output document.
- Compatibility zones: `src/ops/stock.rs` remains the public stock module manifest; catalog and dispatcher ordering must keep P19B immediately after P19A in execution-and-position-management.

## Decision
- Chosen approach: B1 dry-run-first executor.
- Why: P19B is the first executor-shaped boundary after side-effect-free request packages. Dry-run-first freezes executor validation, idempotency, and rejection semantics before any runtime write semantics are added.
- Rejected alternative: direct controlled commit executor. It would require rollback, duplicate execution prevention, partial commit semantics, runtime ownership rules, and failure recovery in the same step.
- Rejected alternative: P19B as another request package. P19A already owns request freezing; P19B should start executor validation instead of adding another passive bridge.
- Known tradeoff: P19B dry-run does not reduce unresolved runtime state by itself.
- Open question: whether a future commit phase should extend this same tool with `execution_mode = "commit"` or add a separate `security_portfolio_execution_replay_commit_executor` remains deferred.

## Acceptance
- Before implementation starts:
  - this design document exists under `docs/plans/`
  - an implementation plan exists under `docs/plans/`
  - P19A output fields and tests have been inspected as the upstream style source
  - repository-wide verification after P19A has passed in the current worktree
- Before completion can be claimed:
  - P19B tests are written before production code and observed red for missing tool/module behavior
  - implementation makes focused P19B tests green
  - tests prove `commit` mode is rejected in this phase
  - tests prove duplicate idempotency keys are rejected
  - tests prove dry-run output creates no runtime refs
  - public stock boundary and grouping guards are updated and green
  - `cargo check` succeeds in an isolated target dir
  - governance docs and handoff notes record that P19B is dry-run-only and not runtime replay
  - `.trae/CHANGELOG_TASK.md` receives an append-only task entry
- Completion must be refused or softened if only focused tests pass and `cargo check` is not run.
