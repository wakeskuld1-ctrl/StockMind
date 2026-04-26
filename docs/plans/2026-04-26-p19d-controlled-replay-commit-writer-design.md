# P19D Controlled Replay Commit Writer Design

## Intent
- Goal: add P19D as a controlled per-row replay commit writer after P19C commit preflight.
- Scope: consume a formal P19C `SecurityPortfolioExecutionReplayCommitPreflightDocument`, validate that every row still matches its frozen commit payload hash and idempotency candidate, detect already-committed runtime records, call the existing `security_execution_record` only for rows that are still safe to write, and emit one explicit P19D commit document.
- Non-goals: do not modify P19B to accept `execution_mode = "commit"`, do not let P19C write runtime facts, do not create broker orders, do not replay broker fills, do not materialize positions outside the existing execution-record runtime path, do not close lifecycle, and do not claim bundle atomicity.
- Success definition: callers can rerun the same P19D request without creating duplicate replay commits, can distinguish `committed`, `already_committed`, `commit_failed`, and rejected states, and can see a truthful non-atomicity notice when a subset of rows commits.
- Delivery form: design doc, implementation plan, later Rust module, CLI tests, source guards, stock boundary wiring, governance docs, handoff notes, and append-only task journal entry.

## Contract
- Tool name: `security_portfolio_execution_replay_commit_writer`.
- Request contract: `SecurityPortfolioExecutionReplayCommitWriterRequest`.
- Primary output contract: `SecurityPortfolioExecutionReplayCommitWriterDocument` wrapped by `SecurityPortfolioExecutionReplayCommitWriterResult`.
- Required inputs:
  - one P19C `SecurityPortfolioExecutionReplayCommitPreflightDocument`
  - `commit_mode = "controlled_per_row"`
  - optional `created_at`
- Required P19C identity:
  - `document_type == "security_portfolio_execution_replay_commit_preflight"`
  - `contract_version == "security_portfolio_execution_replay_commit_preflight.v1"`
  - `preflight_mode == "commit_preflight_only"`
  - `runtime_write_count == 0`
  - `preflight_status == "commit_preflight_ready"` or `preflight_status == "no_commit_work"`
- Required row identity:
  - rows eligible for runtime writes must have `preflight_status == "preflight_ready"`
  - `runtime_execution_record_ref` must be empty on the P19C input
  - `commit_idempotency_key` must be non-empty and unique
  - `canonical_commit_payload_hash` must match a fresh hash of the P19C `commit_payload_preview`
  - `planned_execution_record_ref` must start with `preflight:`
- Runtime record mapping:
  - P19D must derive one deterministic `target_execution_record_ref` from the P19C `commit_idempotency_key`.
  - Recommended format: `execution-record-replay:{sha256(commit_idempotency_key)}`.
  - P19D must not use a random timestamp as the runtime execution record id.
  - P19D must put the target ref into a `SecurityExecutionRecordRequest` path that forces `security_execution_record` to emit that deterministic id, or it must refuse implementation until such a path is explicitly added and tested.
  - The deterministic id path must be implemented inside `security_execution_record`, not by direct P19D store writes.
  - Recommended contract addition: `SecurityExecutionReplayCommitControl { target_execution_record_ref, commit_idempotency_key, canonical_commit_payload_hash, source_p19c_ref }` as an optional request field.
- Replay metadata:
  - `SecurityExecutionRecordDocument` must expose machine-readable optional replay metadata fields when replay commit control is present.
  - Required fields: `replay_commit_idempotency_key`, `replay_commit_payload_hash`, `replay_commit_source_p19c_ref`.
  - P19D may also add human-readable notes, but notes alone are not sufficient for `already_committed` or conflict detection.
- Already-committed detection:
  - Before calling `security_execution_record`, P19D must query `SecurityExecutionStore::load_execution_record(target_execution_record_ref)`.
  - If no record exists, the row may proceed to commit.
  - If a record exists and its machine-readable replay fields prove the same `commit_idempotency_key` and `canonical_commit_payload_hash`, emit `already_committed` and do not call `security_execution_record`.
  - If a record exists at the same target ref with different idempotency or payload evidence, hard-fail the row as `idempotency_conflict` and do not overwrite it.
  - The same conflict-safe check must also exist inside the `security_execution_record` replay-control write path, inside its runtime session, before upsert.
  - If the internal session check sees a concurrent existing target ref, it must return a typed already-exists or conflict signal rather than overwrite.
- Write path:
  - A1 intentionally reuses the existing `security_execution_record` orchestration.
  - A1 is per-row controlled commit, not a bundle transaction.
  - Each successful `security_execution_record` call may commit its own runtime transaction.
  - P19D output must include `non_atomicity_notice` stating that rows before a later failure may already be committed.
- Output row statuses:
  - `committed`
  - `already_committed`
  - `commit_failed`
  - `idempotency_conflict`
  - `skipped_no_commit_work`
- Output document statuses:
  - `no_commit_work`
  - `committed`
  - `committed_with_already_committed`
  - `partial_commit_failure`
  - `rejected`
- Output counts:
  - `commit_row_count`
  - `committed_count`
  - `already_committed_count`
  - `failed_commit_count`
  - `idempotency_conflict_count`
  - `runtime_write_count`
- Traceability requirements:
  - Preserve P19C, P19B, P19A, P18, P17, P16, P15, P14, P13, preview, and P12 refs where P19C exposes them.
  - Each committed or already committed row must include `commit_idempotency_key`, `canonical_commit_payload_hash`, `target_execution_record_ref`, and source `planned_execution_record_ref`.
  - Each runtime write must add a high-signal note to the `SecurityExecutionRecordRequest` carrying P19D tool name, P19C ref, commit idempotency key, and canonical payload hash.
- Rejection conditions:
  - unsupported `commit_mode`
  - wrong P19C document type or contract version
  - P19C input contains non-zero `runtime_write_count`
  - P19C input contains runtime refs
  - P19C input status is not `commit_preflight_ready` or `no_commit_work`
  - payload hash drift from P19C preview
  - duplicate `commit_idempotency_key`
  - duplicate target runtime ref
  - existing runtime record at target ref with conflicting payload/idempotency evidence
  - missing fields needed to build `SecurityExecutionRecordRequest`
  - any attempt to claim bundle rollback or broker execution

## Hard Rejection Red Lines
- Do not modify P19B so `execution_mode = "commit"` stops hard-failing.
- Do not modify P19C so it writes runtime facts.
- Do not write directly to SQLite from P19D unless a new approved runtime contract replaces A1.
- Do not bypass `security_execution_record` in A1.
- Do not call `SecurityExecutionStore::upsert_execution_record` directly from P19D under A1.
- Do not call any runtime write API directly from P19D under A1, including repository write functions, SQLite `execute`, `open_session`, or store mutation APIs.
- Do not treat `ON CONFLICT(execution_record_id) DO UPDATE` as safe idempotency; it can overwrite unless prechecked.
- Do not use timestamps or random values in `target_execution_record_ref`.
- Do not claim bundle atomicity or rollback.
- Do not hide partial commits behind a generic error.

## Decision
- Chosen approach: Scheme A1, controlled per-row replay commit writer using `security_execution_record`.
- Why: the current code already has a governed runtime write mainline through `security_execution_record`, and that function owns its own session/commit. A1 preserves that boundary and adds missing idempotency and replay status controls around it.
- Rejected alternative: enabling commit mode in P19B. This would mix dry-run validation and runtime writes in an already-approved dry-run phase.
- Rejected alternative: making P19C call `security_execution_record`. P19C is frozen as preflight-only and has a source guard against runtime writes.
- Rejected alternative: true bundle-atomic P19D using the existing `security_execution_record` function as-is. It is false because `security_execution_record` opens and commits its own runtime session.
- Rejected alternative: direct `SecurityExecutionStore::upsert_execution_record` from P19D. It would bypass the existing execution-record assembly, journal, plan, outcome, and runtime semantics.
- Known tradeoff: A1 can produce partial commits. The output contract must make this explicit and recoverable.
- Open question resolved for A1 design: implementation must add a narrow replay commit control path to `SecurityExecutionRecordRequest`; lifecycle overlay is not the right idempotency mechanism for P19D replay writes.

## Acceptance
- Before implementation starts:
  - this design document exists under `docs/plans/`
  - an implementation plan exists under `docs/plans/`
  - P19C post-implementation full regression is recorded green in `docs/handoff/CURRENT_STATUS.md`
  - the deterministic runtime ref strategy is decided as `SecurityExecutionReplayCommitControl` inside `security_execution_record`
  - the independent risk subprocess findings have been incorporated
- Before completion can be claimed:
  - P19D tests are written before production code and observed red for missing tool/module behavior
  - tests prove wrong P19C identity is rejected
  - tests prove P19C runtime refs and non-zero runtime write count are rejected
  - tests prove payload hash drift is rejected
  - tests prove duplicate commit keys and duplicate target refs are rejected
  - tests prove already-committed same-payload rows do not call `security_execution_record` again
  - tests prove conflicting already-committed rows hard-fail without overwrite
  - tests prove `security_execution_record` replay-control writes do not overwrite a conflicting existing runtime record
  - tests prove `SecurityExecutionRecordDocument` carries machine-readable replay metadata fields
  - tests prove partial failure reports `partial_commit_failure` and accurate counts
  - source guard proves P19D does not claim bundle atomicity and does not directly call any runtime write API except `security_execution_record`
  - boundary, catalog, dispatcher, frozen manifest, contract registry, decision log, handoff, and task journal are synchronized
  - focused P19D tests, boundary guards, `cargo check`, and repository-wide regression pass

## Cross-Artifact Contract

| Boundary Item | Source Of Truth | Runtime Entrypoints | Frozen/Derived Artifacts | Guard Tests | Required Sync |
|---|---|---|---|---|---|
| `security_portfolio_execution_replay_commit_writer` | this design, implementation module, P19D CLI tests | `src/ops/stock.rs`, `src/ops/stock_execution_and_position_management.rs`, `src/tools/catalog.rs`, `src/tools/dispatcher.rs`, `src/tools/dispatcher/stock_ops.rs` | `tests/stock_formal_boundary_manifest_source_guard.rs`, contract registry, decision log, current status, handoff issues, task journal | P19D CLI tests, stock formal boundary guard, catalog grouping guard, dispatcher grouping guard, cargo check, full regression | add module, grouped export, catalog entry, dispatcher route, frozen manifest entry, governance rows, handoff notes |
| P19D runtime write boundary | `security_execution_record` plus P19D wrapper contract | `security_execution_record` only; `SecurityExecutionStore::load_execution_record` allowed for precheck | P19D source guard and runtime idempotency tests | P19D CLI source/runtime tests, existing `security_execution_record_cli`, runtime store tests if touched | deterministic target ref, already-committed precheck, no direct SQLite/store/repository writes or `open_session` from P19D |
| deterministic replay idempotency | P19D design, `SecurityExecutionReplayCommitControl`, and execution-record replay-control path | `SecurityExecutionStore::load_execution_record`, `security_execution_record` replay-control write path | P19D output doc rows and machine-readable `SecurityExecutionRecordDocument` replay metadata fields | already-committed/conflict tests, replay-control no-overwrite test | target ref derivation, payload hash field, idempotency field, source P19C ref field |

## Independent Risk Pass
- Mode: `user-approved-subagent` completed after one failed 429 attempt.
- Trigger: P19D is the first post-P19C phase allowed to write runtime facts, so cross-artifact and runtime-idempotency drift are high risk.
- Fresh-context question: What will break or be misrepresented if A1 uses existing per-row `security_execution_record` calls to turn P19C preflight evidence into runtime records?
- Findings:
  - A1 must not claim bundle atomicity because `security_execution_record` opens and commits its own runtime session.
  - The runtime store upserts by `execution_record_id`; without a deterministic target ref plus session-local conflict-safe replay-control check, retry can overwrite rather than safely no-op.
  - P19C payload preview does not by itself persist to runtime; P19D must carry commit key/hash/source evidence into machine-readable execution-record fields.
  - Direct runtime writes would bypass existing execution assembly and are outside A1.
  - Existing P15 behavior is a useful precedent for per-row writes but not sufficient for replay idempotency.
- Blocking gaps:
  - Resolved in this design: deterministic id injection is through `SecurityExecutionReplayCommitControl` inside `security_execution_record`.
  - Resolved in this design: already-committed evidence must be machine-readable replay metadata, not notes-only parsing.
  - Resolved in this design: P19D source guards must forbid all direct runtime write paths except the `security_execution_record` call path, while allowing `load_execution_record` prechecks.

## Next Skill
- Use `writing-plans` to create the P19D implementation plan.
- Do not use `test-driven-development` or edit production code until the design and plan are approved.
