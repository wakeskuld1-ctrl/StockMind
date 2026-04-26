# P20C Lifecycle Closeout Archive Writer Design

## Intent
- Goal: add P20C as the first controlled lifecycle closeout/archive writer after P20B evidence readiness.
- Scope: consume one formal P20B `SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageDocument`, write durable closeout archive records only for rows with `evidence_status == "evidence_ready_for_closeout_archive_preflight"`, preserve blocked P20B rows without writes, and return row-level archive results with idempotency evidence.
- Non-goals: do not replay broker fills, do not create broker orders, do not materialize positions, do not rewrite existing `security_execution_records`, do not infer closed truth from P20A/P20B free text, do not call a nonexistent `security_closed_position_archive` route, and do not promise all-row atomic rollback.
- Success definition: callers can distinguish archived, already-archived, skipped, and blocked rows; repeated P20C calls with the same P20B evidence are idempotent; archive facts are durable and traceable to P20B, P20A, P19E, P19D, and P19C.
- Delivery form: this design doc first; after approval, an implementation plan, TDD tests, runtime archive repository/schema changes, stock public boundary wiring, governance docs, handoff notes, task journal, focused verification, guards, `cargo check`, and repository-wide regression.

## Contract
- Tool name: `security_portfolio_execution_lifecycle_closeout_archive_writer`.
- Request contract: `SecurityPortfolioExecutionLifecycleCloseoutArchiveWriterRequest`.
- Primary output contract: `SecurityPortfolioExecutionLifecycleCloseoutArchiveWriterDocument` wrapped by `SecurityPortfolioExecutionLifecycleCloseoutArchiveWriterResult`.
- Runtime archive record contract: `SecurityPortfolioExecutionLifecycleCloseoutArchiveRecord`.
- Required input:
  - one P20B `SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageDocument`
  - optional `created_at`
  - optional `archive_mode`, default `commit`
- Required P20B identity:
  - `document_type == "security_portfolio_execution_lifecycle_closeout_evidence_package"`
  - `contract_version == "security_portfolio_execution_lifecycle_closeout_evidence_package.v1"`
  - `runtime_write_count == 0`
  - source P20A, P19E, P19D, P19C refs are present
  - source non-atomicity notice is present
- Rows eligible for archive writes:
  - `evidence_status == "evidence_ready_for_closeout_archive_preflight"`
  - `closeout_evidence_ready == true`
  - `runtime_record_present == true`
  - non-empty `target_execution_record_ref`
  - runtime `position_state == "closed"`
  - runtime exit date is present
  - runtime exit price is positive
  - runtime exit reason is present and not `position_still_open`
  - runtime replay idempotency key, payload hash, and source P19C ref match P20B row evidence
- Rows not eligible for archive writes:
  - any P20B blocked status
  - unknown P20B evidence status
  - missing machine-readable runtime closeout evidence
  - account, symbol, or replay lineage drift
- Archive record required fields:
  - `closeout_archive_record_id`
  - `contract_version`
  - `document_type`
  - `created_at`
  - `account_id`
  - `symbol`
  - `analysis_date`
  - `source_p20c_ref`
  - `source_p20b_ref`
  - `source_p20a_ref`
  - `source_p19e_ref`
  - `source_p19d_ref`
  - `source_p19c_ref`
  - `target_execution_record_ref`
  - `archive_idempotency_key`
  - `archive_payload_hash`
  - `commit_idempotency_key`
  - `canonical_commit_payload_hash`
  - `runtime_position_state`
  - `runtime_actual_exit_date`
  - `runtime_actual_exit_price`
  - `runtime_exit_reason`
  - `archive_status`
  - `payload_json`
- Archive row statuses:
  - `archived`
  - `already_archived_verified`
  - `skipped_p20b_not_ready`
  - `blocked_missing_archive_evidence`
  - `blocked_archive_identity_conflict`
  - `blocked_archive_payload_conflict`
  - `blocked_unknown_p20b_evidence_status`
  - `write_failed_preserved`
- Document statuses:
  - `no_archive_candidates`
  - `archive_complete`
  - `partial_archive_complete`
  - `blocked`
  - `rejected`
- Runtime counts:
  - `runtime_read_count` may increase when checking existing archive records for idempotency/conflicts.
  - `runtime_write_count` increments only for newly inserted archive records.
  - Existing archive records verified as matching must count as reads, not writes.
- Idempotency contract:
  - deterministic `archive_idempotency_key` derives from account id, symbol, target execution record ref, P20B ref, commit idempotency key, and canonical commit payload hash.
  - deterministic `archive_payload_hash` derives from the canonical archive payload, not free text.
  - repeated calls with the same canonical payload return `already_archived_verified`.
  - same archive key with different payload hash returns `blocked_archive_payload_conflict` and must not overwrite.
  - archive writes must use insert-only or conflict-check-before-write semantics; `ON CONFLICT DO UPDATE` is forbidden for archive records.
- Atomicity contract:
  - P20C is controlled per-row and non-atomic across rows.
  - A write failure for one row must not erase prior successful row results.
  - The document must include a `non_atomicity_notice`.
  - If an implementation later chooses a batch transaction, that change must be explicitly redesigned; it cannot be smuggled into this contract.
- Rejection conditions:
  - unsupported P20B document type or contract version
  - P20B `runtime_write_count != 0`
  - missing P20B/P20A/P19E/P19D/P19C refs
  - missing source non-atomicity notice
  - request attempts broker execution, broker-fill replay, position materialization, execution-record rewrite, post-trade review generation, or free-text-only archive evidence
  - archive mode other than `commit` unless a future design adds dry-run or repair mode
- Traceability requirements:
  - every archive row must preserve P20B row status, target execution ref, runtime exit evidence, replay metadata, source refs, archive idempotency key, and archive payload hash.
  - skipped and blocked rows must be emitted with explicit blockers.
  - archived rows must be auditable without rereading P20B free text.

## Hard Rejection Red Lines
- Do not call or depend on `security_closed_position_archive` unless a later local implementation and route exist and the P20C design is updated.
- Do not call `security_execution_record` from P20C.
- Do not update existing `security_execution_records`.
- Do not call `security_post_trade_review`.
- Do not use free-text notes as archive evidence.
- Do not convert P20B `evidence_ready_for_closeout_archive_preflight` into lifecycle closure without a durable archive write.
- Do not overwrite an existing archive record with a different payload hash.
- Do not treat a partially archived document as fully closed.

## Decision
- Chosen approach: P20C creates a new controlled archive writer with its own runtime archive record contract and insert-only persistence boundary.
- Why: local evidence shows no callable `security_closed_position_archive` implementation or route in current `src`/`tests`; P20B already supplies the required pre-archive evidence, so P20C should define the first durable closeout archive writer instead of adding another read-only preflight layer.
- Rejected alternative: direct call to `security_closed_position_archive`. It is not locally present and would hide an unavailable dependency behind stale handoff language.
- Rejected alternative: P20C as another resolver/preflight only. It would defer the first durable closeout write to P20D and repeat the pattern already covered by P20A/P20B.
- Rejected alternative: update `security_execution_records` to mark archive status. That would mix execution truth and archive lifecycle truth in one table and make replay metadata conflicts harder to isolate.
- Known tradeoff: adding a dedicated archive record table/repository increases runtime schema surface, but it gives clear idempotency, conflict, and audit semantics.
- Open question: whether archive records should later feed a separate post-trade review package. This is out of P20C scope and must not be inferred.

## Acceptance
- Before implementation starts:
  - this design document is approved by the user
  - a separate P20C implementation plan exists under `docs/plans/`
  - the plan names every runtime schema, repository, public boundary, manifest, catalog, dispatcher, governance, handoff, and test artifact touched by P20C
  - the plan defines RED tests before production code
- Before completion can be claimed:
  - RED tests fail first for missing P20C tool/module/schema behavior
  - tests reject wrong P20B document identity and contract version
  - tests reject P20B `runtime_write_count != 0`
  - tests reject missing source refs and missing non-atomicity notice
  - tests prove no P20B rows returns `no_archive_candidates` with zero writes
  - tests prove P20B blocked rows become `skipped_p20b_not_ready` and do not write
  - tests prove one evidence-ready row writes one archive record with deterministic archive idempotency key and payload hash
  - tests prove repeated same input returns `already_archived_verified` without a second write
  - tests prove same archive key with different payload hash returns `blocked_archive_payload_conflict` without overwrite
  - tests prove mixed ready and blocked rows return `partial_archive_complete`
  - tests prove write failures are preserved row-level as `write_failed_preserved`
  - source guard proves no `security_closed_position_archive(`, no `security_execution_record(`, no `security_post_trade_review(`, no execution-record update, and no archive `ON CONFLICT DO UPDATE`
  - runtime schema/repository/session tests pass for insert, load, idempotent verify, and conflict reject
  - P20C focused CLI tests, adjacent P20B tests, formal boundary guard, catalog guard, dispatcher guard, `cargo check`, and repository-wide regression pass

## Cross-Artifact Contract

| Boundary Item | Source Of Truth | Runtime Entrypoints | Frozen/Derived Artifacts | Guard Tests | Required Sync |
|---|---|---|---|---|---|
| `security_portfolio_execution_lifecycle_closeout_archive_writer` | this design, P20C implementation module, P20C CLI tests | `src/ops/stock.rs`, `src/ops/stock_execution_and_position_management.rs`, `src/tools/catalog.rs`, `src/tools/dispatcher.rs`, `src/tools/dispatcher/stock_ops.rs` | `tests/stock_formal_boundary_manifest_source_guard.rs`, contract registry, decision log, current status, handoff issues, task journal | P20C CLI tests, stock formal boundary guard, catalog grouping guard, dispatcher grouping guard, cargo check, full regression | add module, grouped export, catalog entry, dispatcher route, frozen manifest entry, governance rows, handoff notes |
| P20C archive runtime persistence | P20C archive record contract and runtime repository tests | `SecurityExecutionStore` facade/session plus new archive repository functions | `src/runtime/security_execution_store_schema.rs`, repository context/session/facade, runtime module exports | runtime archive repository tests, P20C idempotency tests, source guard | add archive table, insert-only repository, load-by-id/key, conflict check, session/facade method names |
| archive idempotency and conflict guard | canonical archive payload contract | P20C writer and archive repository | P20C CLI tests, source guard | idempotent rerun test, payload-conflict test | deterministic archive key/hash, no overwrite, conflict blockers |
| P20B to P20C evidence boundary | P20B document contract and P20C request validation | P20C writer only consumes P20B document | P20C request validation tests and source guard | adjacent P20B tests, P20C validation tests | preserve P20B/P20A/P19E/P19D/P19C refs and runtime closeout evidence |

## Independent Risk Pass
- Mode: inline-fresh-pass.
- Trigger: P20C adds the first write path after two read-only closeout gates, touches public tool boundaries and runtime persistence, and can easily blur archive production with lifecycle closure.
- Fresh-context question: Can P20C safely create durable archive facts from P20B evidence without relying on nonexistent archive routes, rewriting execution records, or overclaiming lifecycle closure?
- Findings:
  - Local file search found P20A/P20B closeout files but no `security_closed_position_archive` source or test file.
  - Public boundary search found P20A/P20B routes but no `security_closed_position_archive` route in current stock dispatcher/catalog.
  - Current runtime schema has execution, position-plan, and adjustment-event tables only; no archive table exists.
  - Existing `SecurityExecutionStoreSession` supports transactional writes and conflict-check reads for execution records; P20C should extend the same runtime boundary pattern for archive records rather than writing SQLite ad hoc in the ops module.
  - P20B already preserves closed runtime evidence and replay metadata, so P20C does not need broker replay or execution-record mutation.
- Blocking gaps:
  - A separate implementation plan is required before code.
  - The archive schema/table name and repository method names must be frozen in the implementation plan before TDD.
  - If the user wants all-row atomic archive closure instead of per-row non-atomic writes, this design must be revised before implementation.

## Next Skill
- Use `writing-plans` after the user approves this P20C design.
- Do not use `test-driven-development` or edit production code until the P20C implementation plan is written and approved.
