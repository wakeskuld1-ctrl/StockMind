# P19E Replay Commit Audit Design

## Intent
- Goal: add P19E as a read-only replay commit audit layer after the P19D controlled replay commit writer.
- Scope: consume one formal P19D `SecurityPortfolioExecutionReplayCommitWriterDocument`, read back runtime execution records, verify replay commit metadata, and emit one audit document that preserves row-level truth.
- Non-goals: do not write runtime facts, do not call `security_execution_record`, do not replay broker fills, do not create broker orders, do not materialize positions, do not close lifecycle, do not change P19B dry-run semantics, do not change P19C preflight-only semantics, and do not change P19D commit authority.
- Success definition: callers can prove whether each P19D committed or already-committed row has a matching runtime record with the expected replay idempotency key, payload hash, source P19C ref, and target execution ref, while preserving missing, failed, and conflict states explicitly.
- Delivery form: design doc, implementation plan, later Rust module, CLI tests, source guards, public stock boundary wiring, governance docs, handoff notes, and append-only task journal entry.

## Contract
- Tool name: `security_portfolio_execution_replay_commit_audit`.
- Request contract: `SecurityPortfolioExecutionReplayCommitAuditRequest`.
- Primary output contract: `SecurityPortfolioExecutionReplayCommitAuditDocument` wrapped by `SecurityPortfolioExecutionReplayCommitAuditResult`.
- Required input:
  - one P19D `SecurityPortfolioExecutionReplayCommitWriterDocument`
  - optional `created_at`
- Required P19D identity:
  - `document_type == "security_portfolio_execution_replay_commit_writer"`
  - `contract_version == "security_portfolio_execution_replay_commit_writer.v1"`
  - P19D source lineage must include the source P19C commit-preflight ref when P19D exposes it.
  - P19D `non_atomicity_notice` must be present and preserved in the audit output.
- Runtime read boundary:
  - P19E may read runtime execution records through the existing runtime store read API.
  - P19E must not write runtime facts.
  - P19E must not call `security_execution_record`.
  - P19E must not call store/session/repository mutation APIs, SQLite `execute`, or `open_session`.
- Audit fields per row:
  - source P19D row status
  - source P19D row ref or stable row identity
  - source P19C ref
  - target execution record ref
  - P19D commit idempotency key
  - P19D canonical commit payload hash
  - runtime record present flag
  - runtime replay idempotency key
  - runtime replay payload hash
  - runtime replay source P19C ref
  - audit status
  - blockers
- Audit row statuses:
  - `verified`
  - `already_committed_verified`
  - `missing_runtime_record`
  - `metadata_mismatch`
  - `idempotency_conflict_confirmed`
  - `commit_failed_preserved`
  - `skipped_no_commit_work_preserved`
  - `not_auditable`
- Audit document statuses:
  - `no_commit_work`
  - `verified`
  - `verified_with_preserved_failures`
  - `partial_audit_failure`
  - `rejected`
- Output counts:
  - `audit_row_count`
  - `verified_count`
  - `already_committed_verified_count`
  - `missing_runtime_record_count`
  - `metadata_mismatch_count`
  - `idempotency_conflict_confirmed_count`
  - `commit_failed_preserved_count`
  - `not_auditable_count`
  - `runtime_write_count`
- Runtime write count:
  - P19E must always emit `runtime_write_count = 0`.
- Rejection conditions:
  - unsupported or missing P19D document identity
  - unsupported P19D contract version
  - missing P19D `non_atomicity_notice`
  - missing target execution record ref for a row that P19D marks `committed` or `already_committed`
  - missing commit idempotency key or canonical payload hash for a row that should be runtime-verifiable
  - request attempts to provide or request lifecycle closeout semantics
  - any implementation path attempts runtime writes
- Traceability requirements:
  - Preserve P19D document ref, P19C source ref, target runtime ref, idempotency key, and payload hash in each auditable row.
  - Preserve P19D non-atomicity truth in the P19E document.
  - Preserve P19D failed and conflict rows as audit facts instead of hiding them behind a successful document status.
  - Do not parse free-text notes as the source of truth for replay metadata; use machine-readable replay metadata fields only.

## Hard Rejection Red Lines
- Do not modify P19B so `execution_mode = "commit"` stops hard-failing.
- Do not modify P19C so it writes runtime facts.
- Do not modify P19D to become lifecycle closeout.
- Do not call `security_execution_record` from P19E.
- Do not write directly to SQLite, store sessions, repositories, or runtime mutation APIs from P19E.
- Do not treat a P19D `committed` row as lifecycle complete.
- Do not treat runtime record presence as broker fill truth.
- Do not treat notes-only evidence as replay metadata truth.
- Do not collapse `committed`, `already_committed`, `commit_failed`, and `idempotency_conflict` into one generic audit state.

## Decision
- Chosen approach: Scheme A, read-only P19E commit audit / runtime replay verification.
- Why: P19D is the first replay phase that writes runtime facts, so the next layer should verify the write result before lifecycle closeout or position materialization is designed.
- Rejected alternative: direct P20 lifecycle closeout after P19D. It lacks a frozen lifecycle state machine, closeout criteria, partial-commit semantics, and boundary with existing closed-position archive objects.
- Rejected alternative: extending P19D to self-audit as a final closeout step. It would mix commit authority and independent verification in one phase.
- Rejected alternative: using free-text execution notes as audit evidence. P19D explicitly added machine-readable replay metadata for idempotency and conflict detection.
- Known tradeoff: P19E adds another phase before P20, but it creates a stable runtime verification fact that later lifecycle work can consume.
- Open question resolved for this design: P19E should not fail fast after the first bad row. It should emit per-row audit states and an aggregate `partial_audit_failure` status when only some rows fail verification, preserving P19D non-atomic truth.

## Acceptance
- Before implementation starts:
  - this design document exists under `docs/plans/`
  - an implementation plan exists under `docs/plans/`
  - P19D repository-wide green verification is recorded in `docs/handoff/CURRENT_STATUS.md`
  - the approved public-boundary sync list names every file that must move with P19E
  - the independent risk subprocess findings are incorporated
- Before completion can be claimed:
  - P19E tests are written before production code and observed red for missing tool/module behavior
  - tests prove wrong P19D identity and contract version are rejected
  - tests prove P19D missing `non_atomicity_notice` is rejected
  - tests prove `committed` rows with matching runtime replay metadata become `verified`
  - tests prove `already_committed` rows with matching runtime replay metadata become `already_committed_verified`
  - tests prove runtime-missing committed rows become `missing_runtime_record`
  - tests prove metadata drift becomes `metadata_mismatch`
  - tests prove P19D conflict rows become `idempotency_conflict_confirmed` without runtime writes
  - tests prove P19D failed rows are preserved as `commit_failed_preserved`
  - tests prove `runtime_write_count == 0`
  - source guard proves P19E does not call `security_execution_record`, direct SQLite writes, `open_session`, or runtime mutation APIs
  - boundary, catalog, dispatcher, frozen manifest, contract registry, decision log, handoff, and task journal are synchronized
  - focused P19E tests, adjacent replay writer tests, boundary guards, `cargo check`, and repository-wide regression pass

## Cross-Artifact Contract

| Boundary Item | Source Of Truth | Runtime Entrypoints | Frozen/Derived Artifacts | Guard Tests | Required Sync |
|---|---|---|---|---|---|
| `security_portfolio_execution_replay_commit_audit` | this design, implementation module, P19E CLI tests | `src/ops/stock.rs`, `src/ops/stock_execution_and_position_management.rs`, `src/tools/catalog.rs`, `src/tools/dispatcher.rs`, `src/tools/dispatcher/stock_ops.rs` | `tests/stock_formal_boundary_manifest_source_guard.rs`, contract registry, decision log, current status, handoff issues, task journal | P19E CLI tests, stock formal boundary guard, catalog grouping guard, dispatcher grouping guard, cargo check, full regression | add module, grouped export, catalog entry, dispatcher route, frozen manifest entry, governance rows, handoff notes |
| P19E read-only runtime audit boundary | P19E design and P19D replay metadata contract | runtime store read API only | P19E source guard and audit document rows | P19E source guard, P19E metadata tests, existing `security_portfolio_execution_replay_commit_writer_cli` | allow only readback of runtime records; forbid `security_execution_record`, direct writes, `open_session`, and store mutation APIs |
| replay metadata verification | `SecurityExecutionRecordDocument` replay metadata fields from P19D | runtime record readback | P19E audit row evidence | `verified`, `missing_runtime_record`, `metadata_mismatch`, and conflict-preservation tests | compare idempotency key, payload hash, source P19C ref, and target execution ref using machine-readable fields |

## Independent Risk Pass
- Mode: `user-approved-subagent`.
- Trigger: P19E adds a new public tool after P19D and can drift across public boundary, catalog, dispatcher, frozen manifest, governance docs, and runtime-read-only source guards.
- Fresh-context question: After P19D, should the next phase be P19E commit audit or P20 lifecycle closeout, and what artifact will drift if the boundary is added?
- Findings:
  - No frozen P19E or P20 contract exists after P19D.
  - P19E is lower risk than P20 because it verifies P19D runtime writes without claiming lifecycle closure.
  - P19E must synchronize `stock.rs`, grouped stock execution module, catalog, dispatcher, dispatcher stock ops, frozen manifest, grouping guards, contract registry, decision log, handoff, and task journal.
  - P19E must use machine-readable replay metadata and must not parse notes as truth.
  - P19E must preserve P19D non-atomic partial commit truth.
- Blocking gaps:
  - P20 remains blocked until lifecycle state, closeout criteria, partial-commit semantics, and closed-position archive boundaries are explicitly designed.
  - P19E has no current design blocker after this contract, but implementation must not start until the implementation plan is saved and approved.

## Next Skill
- Use `writing-plans` to create the P19E implementation plan.
- Do not use `test-driven-development` or edit production code until the design and plan are approved.
