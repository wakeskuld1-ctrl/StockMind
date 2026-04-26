# P20A Lifecycle Closeout Readiness Design

## Intent
- Goal: add P20A as a side-effect-free lifecycle closeout readiness layer after P19E replay commit audit.
- Scope: consume one formal P19E `SecurityPortfolioExecutionReplayCommitAuditDocument`, classify each audited replay row as closeout-preflight eligible or blocked, preserve P19D/P19E non-atomic truth, and emit one readiness document for a future closeout writer.
- Non-goals: do not write runtime facts, do not call `security_execution_record`, do not call `security_post_trade_review`, do not write or depend on `security_closed_position_archive`, do not replay broker fills, do not create broker orders, do not materialize positions, do not close lifecycle, and do not treat P19E verification as broker-fill or closed-position truth.
- Success definition: callers can see which P19E rows are eligible for a future lifecycle closeout preflight, which rows are blocked, and why, without creating any runtime, post-trade, archive, position, or lifecycle mutation.
- Delivery form: design doc now; after approval, implementation plan, later Rust module, CLI tests, source guards, public stock boundary wiring, governance docs, handoff notes, and append-only task journal entry.

## Contract
- Tool name: `security_portfolio_execution_lifecycle_closeout_readiness`.
- Request contract: `SecurityPortfolioExecutionLifecycleCloseoutReadinessRequest`.
- Primary output contract: `SecurityPortfolioExecutionLifecycleCloseoutReadinessDocument` wrapped by `SecurityPortfolioExecutionLifecycleCloseoutReadinessResult`.
- Required input:
  - one P19E `SecurityPortfolioExecutionReplayCommitAuditDocument`
  - optional `created_at`
- Required P19E identity:
  - `document_type == "security_portfolio_execution_replay_commit_audit"`
  - `contract_version == "security_portfolio_execution_replay_commit_audit.v1"`
  - `runtime_write_count == 0`
  - P19E must expose source P19D ref, source P19C ref, and preserved source non-atomicity notice.
- Required row evidence:
  - every candidate row must carry `audit_status`
  - every eligible row must carry target execution record ref
  - every eligible row must carry commit idempotency key
  - every eligible row must carry canonical commit payload hash
  - every eligible row must carry source P19C ref
  - every eligible row must carry runtime replay idempotency key, payload hash, and source P19C ref
- Readiness row statuses:
  - `eligible_for_closeout_preflight`
  - `blocked_missing_runtime_record`
  - `blocked_metadata_mismatch`
  - `blocked_commit_failed`
  - `blocked_idempotency_conflict`
  - `blocked_no_commit_work`
  - `blocked_not_auditable`
  - `blocked_unknown_audit_status`
- Eligible source statuses:
  - P19E `verified`
  - P19E `already_committed_verified`
- Blocked source statuses:
  - P19E `missing_runtime_record`
  - P19E `metadata_mismatch`
  - P19E `commit_failed_preserved`
  - P19E `idempotency_conflict_confirmed`
  - P19E `skipped_no_commit_work_preserved`
  - P19E `not_auditable`
  - any unknown P19E row status
- Document statuses:
  - `no_closeout_candidates`
  - `closeout_preflight_ready`
  - `partial_closeout_preflight_ready`
  - `blocked`
  - `rejected`
- Output counts:
  - `readiness_row_count`
  - `eligible_for_closeout_preflight_count`
  - `blocked_missing_runtime_record_count`
  - `blocked_metadata_mismatch_count`
  - `blocked_commit_failed_count`
  - `blocked_idempotency_conflict_count`
  - `blocked_no_commit_work_count`
  - `blocked_not_auditable_count`
  - `blocked_unknown_audit_status_count`
  - `runtime_write_count`
- Runtime write count:
  - P20A must always emit `runtime_write_count = 0`.
- Rejection conditions:
  - unsupported or missing P19E document identity
  - unsupported P19E contract version
  - P19E `runtime_write_count != 0`
  - missing P19D source ref
  - missing P19C source ref
  - missing preserved non-atomicity notice
  - request or input attempts to provide lifecycle closeout, archive write, runtime write, broker-fill replay, or position materialization semantics
  - eligible row lacks target execution ref, commit key, payload hash, source P19C ref, or runtime replay metadata evidence
- Traceability requirements:
  - Preserve P19E audit document ref, P19D source ref, P19C source ref, P19E row audit status, target execution ref, idempotency key, payload hash, and runtime replay metadata for each row.
  - Preserve P19D/P19E non-atomic partial truth in the readiness document.
  - Preserve blocked row reasons explicitly; do not roll them up into a generic failure.
  - Do not use notes or free-text strings as lifecycle eligibility evidence.

## Hard Rejection Red Lines
- Do not call `security_execution_record` from P20A.
- Do not call `security_post_trade_review` from P20A because it can enter the execution-record write path.
- Do not write directly to SQLite, store sessions, repositories, or runtime mutation APIs from P20A.
- Do not write or call any `security_closed_position_archive` implementation from P20A.
- Do not assume `security_closed_position_archive` is available in the current worktree; local evidence only confirms historical handoff references.
- Do not treat P19E `verified` as lifecycle closed.
- Do not treat P19D replay runtime commit as broker fill.
- Do not treat P19E `verified_with_preserved_failures` as all rows eligible.
- Do not collapse blocked audit states into eligible rows.
- Do not close lifecycle, materialize positions, or produce closed-position archive documents in P20A.

## Decision
- Chosen approach: P20A as lifecycle closeout readiness / preflight eligibility, not a writer.
- Why: P19E proves runtime replay metadata consistency, but it does not prove broker fills, closed execution records, or lifecycle completion. P20A creates the next governed fact without crossing into mutation.
- Rejected alternative: direct P20 lifecycle closeout writer after P19E. It is premature because current local evidence does not show an available closed-position archive implementation or frozen state machine.
- Rejected alternative: calling `security_post_trade_review` to enrich readiness. Its current code path calls `security_execution_record`, which would violate P20A side-effect-free scope.
- Rejected alternative: treating all P19E `verified_with_preserved_failures` documents as batch-ready. Non-atomic partial truth means only row-level verified rows can be eligible.
- Known tradeoff: P20A adds another preflight layer, but it prevents P19E audit truth from being misused as lifecycle closure.
- Open question resolved for this design: P20A may emit partial readiness when some rows are eligible and some are blocked; it must not require all rows to be eligible before producing row-level readiness.

## Acceptance
- Before implementation starts:
  - this design document exists under `docs/plans/`
  - an implementation plan exists under `docs/plans/`
  - P19E repository-wide green verification is recorded in `docs/handoff/CURRENT_STATUS.md`
  - the public-boundary sync list names every file that must move with P20A
  - the independent risk subprocess findings are incorporated
- Before completion can be claimed:
  - P20A tests are written before production code and observed red for missing tool/module behavior
  - tests prove wrong P19E identity and contract version are rejected
  - tests prove P19E `runtime_write_count != 0` is rejected
  - tests prove missing P19D/P19C refs and missing non-atomicity notice are rejected
  - tests prove P19E `verified` rows become `eligible_for_closeout_preflight`
  - tests prove P19E `already_committed_verified` rows become `eligible_for_closeout_preflight`
  - tests prove P19E `missing_runtime_record` rows become `blocked_missing_runtime_record`
  - tests prove P19E `metadata_mismatch` rows become `blocked_metadata_mismatch`
  - tests prove P19E `commit_failed_preserved` rows become `blocked_commit_failed`
  - tests prove P19E `idempotency_conflict_confirmed` rows become `blocked_idempotency_conflict`
  - tests prove unknown P19E row statuses become `blocked_unknown_audit_status`
  - tests prove partial eligible and blocked rows produce `partial_closeout_preflight_ready`
  - tests prove `runtime_write_count == 0`
  - source guard proves P20A does not call `security_execution_record`, `security_post_trade_review`, `security_closed_position_archive`, SQLite `.execute`, `open_session`, `upsert_execution_record`, or runtime mutation APIs
  - boundary, catalog, dispatcher, frozen manifest, contract registry, decision log, handoff, and task journal are synchronized
  - focused P20A tests, adjacent P19E tests, boundary guards, `cargo check`, and repository-wide regression pass

## Cross-Artifact Contract

| Boundary Item | Source Of Truth | Runtime Entrypoints | Frozen/Derived Artifacts | Guard Tests | Required Sync |
|---|---|---|---|---|---|
| `security_portfolio_execution_lifecycle_closeout_readiness` | this design, implementation module, P20A CLI tests | `src/ops/stock.rs`, `src/ops/stock_execution_and_position_management.rs`, `src/tools/catalog.rs`, `src/tools/dispatcher.rs`, `src/tools/dispatcher/stock_ops.rs` | `tests/stock_formal_boundary_manifest_source_guard.rs`, contract registry, decision log, current status, handoff issues, task journal | P20A CLI tests, stock formal boundary guard, catalog grouping guard, dispatcher grouping guard, cargo check, full regression | add module, grouped export, catalog entry, dispatcher route, frozen manifest entry, governance rows, handoff notes |
| P20A side-effect-free readiness boundary | P20A design and P19E audit contract | no runtime write entrypoint; P20A should not need runtime reads if P19E evidence is complete | P20A source guard and readiness document rows | P20A source guard, P20A row-status tests, adjacent P19E tests | forbid `security_execution_record`, `security_post_trade_review`, closed archive calls, direct writes, `open_session`, and store mutation APIs |
| lifecycle closeout eligibility | P19E audit row machine-readable replay metadata | none | P20A readiness row evidence | eligible/blocked row tests and partial-readiness test | map only `verified` and `already_committed_verified` to eligibility; preserve all other statuses as blockers |

## Independent Risk Pass
- Mode: `user-approved-subagent`.
- Trigger: P20A adds a new public tool after P19E and sits next to lifecycle/closed-archive semantics, so boundary drift and semantic overclaim risk are high.
- Fresh-context question: Can P20A safely consume P19E audit and define lifecycle closeout readiness without writing runtime facts or claiming lifecycle closure?
- Findings:
  - No frozen P20/P20A/readiness contract exists yet.
  - P20A should be side-effect-free readiness/preflight.
  - P20A must not call `security_execution_record`, `security_post_trade_review`, closed archive writers, runtime write APIs, or position materialization paths.
  - P19E `verified` proves replay metadata consistency only; it does not prove broker fill, closed execution record, or lifecycle closure.
  - P19D replay commit records currently remain open-position shaped (`actual_exit_date` empty and `exit_reason = position_still_open`), so P20A cannot infer closed-position archive eligibility.
  - `security_closed_position_archive` has historical handoff boundary notes, but the current local `src/ops`, catalog, and dispatcher search did not find an implementation or route. P20A may reference that boundary as future work but must not depend on it.
- Blocking gaps:
  - P20B closeout writer remains blocked until lifecycle state machine, archive implementation availability, closeout criteria, partial-row semantics, and closed-position archive boundaries are frozen.
  - P20A has no current design blocker after this contract, but implementation must not start until the implementation plan is saved and approved.

## Next Skill
- Use `writing-plans` to create the P20A implementation plan after user approval.
- Do not use `test-driven-development` or edit production code until the design and plan are approved.
