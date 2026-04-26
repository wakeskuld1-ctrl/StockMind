# P19D Controlled Replay Commit Writer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add P19D as a controlled per-row runtime replay commit writer that consumes P19C preflight evidence and writes through `security_execution_record` with explicit idempotency and partial-commit semantics.

**Architecture:** P19D validates a P19C preflight document, re-hashes each payload preview, derives a deterministic target runtime execution-record ref, checks the runtime store for already-committed rows, then calls the existing `security_execution_record` only for rows that are safe to commit. A1 is deliberately non-atomic across rows and must report partial commits truthfully.

**Tech Stack:** Rust, serde, thiserror, sha2, existing StockMind CLI dispatcher/catalog, `security_execution_record`, `SecurityExecutionStore`, Cargo integration tests.

---

### Risk Synchronization Gate
**Risk subprocess mode:** user-approved-subagent completed after one failed 429 attempt.

**Question asked:** What artifact will drift if P19D exposes the first controlled runtime replay writer without synchronized public boundary, idempotency, source guards, and non-atomicity docs?

**Boundary items:**
- `security_portfolio_execution_replay_commit_writer`
- `SecurityPortfolioExecutionReplayCommitWriterRequest`
- `SecurityPortfolioExecutionReplayCommitWriterDocument`
- `SecurityPortfolioExecutionReplayCommitWriterResult`
- public stock module export
- execution-and-position-management grouped export
- tool catalog entry
- dispatcher route
- frozen stock-boundary manifest entry
- contract registry and decision log rows
- `SecurityExecutionReplayCommitControl`
- machine-readable replay metadata fields on `SecurityExecutionRecordDocument`
- P19D source guard for no direct runtime write APIs and no bundle-atomic claim

**Must-sync files:**
- `D:\SM\docs\plans\2026-04-26-p19d-controlled-replay-commit-writer-design.md`
- `D:\SM\docs\plans\2026-04-26-p19d-controlled-replay-commit-writer-plan.md`
- `D:\SM\src\ops\security_portfolio_execution_replay_commit_writer.rs`
- `D:\SM\src\ops\security_execution_record.rs`
- `D:\SM\src\ops\stock.rs`
- `D:\SM\src\ops\stock_execution_and_position_management.rs`
- `D:\SM\src\runtime\security_execution_store_execution_record_repository.rs` if replay-control conflict checks require repository support
- `D:\SM\src\tools\catalog.rs`
- `D:\SM\src\tools\dispatcher.rs`
- `D:\SM\src\tools\dispatcher\stock_ops.rs`
- `D:\SM\tests\security_portfolio_execution_replay_commit_writer_cli.rs`
- `D:\SM\tests\stock_formal_boundary_manifest_source_guard.rs`
- `D:\SM\docs\governance\contract_registry.md`
- `D:\SM\docs\governance\decision_log.md`
- `D:\SM\docs\handoff\CURRENT_STATUS.md`
- `D:\SM\docs\handoff\HANDOFF_ISSUES.md`
- `D:\SM\.trae\CHANGELOG_TASK.md`

**Must-run checks:**
- P19D RED focused test
- P19D GREEN focused test
- `security_execution_record_cli` if `SecurityExecutionRecordRequest` changes
- runtime execution store tests if store/session/repository changes
- source guard proving P19D does not call direct runtime write APIs
- `stock_formal_boundary_manifest_source_guard`
- `stock_catalog_grouping_source_guard`
- `stock_dispatcher_grouping_source_guard`
- `cargo check`
- repository-wide `cargo test -- --nocapture`

**Blockers resolved into hard constraints:**
- Deterministic target execution-record id injection is `SecurityExecutionReplayCommitControl` inside `security_execution_record`.
- Already-committed evidence must use machine-readable replay metadata fields, not notes-only parsing.
- P19D must not call direct runtime write APIs: repository write functions, SQLite `execute`, `open_session`, or store mutation APIs.
- P19D may call `SecurityExecutionStore::load_execution_record` only for precheck/read evidence.
- Do not implement true bundle atomicity in A1.
- Do not let P19B accept commit mode or P19C write runtime facts.

---

### Task 1: Add Execution Record Replay-Control Contract

**Files:**
- Modify: `D:\SM\src\ops\security_execution_record.rs`
- Modify if required: `D:\SM\src\runtime\security_execution_store_execution_record_repository.rs`
- Test: `D:\SM\tests\security_execution_record_cli.rs` or existing unit tests in `security_execution_record.rs`

**Step 1: Write failing replay-control tests**

Add tests proving:
- a replay-control request forces the resulting `execution_record_id` to `target_execution_record_ref`
- the output `SecurityExecutionRecordDocument` exposes `replay_commit_idempotency_key`
- the output `SecurityExecutionRecordDocument` exposes `replay_commit_payload_hash`
- the output `SecurityExecutionRecordDocument` exposes `replay_commit_source_p19c_ref`
- an existing conflicting runtime record at the target ref is not overwritten

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_execution_record_red'; cargo test --test security_execution_record_cli -- --nocapture
```

Expected: fail because replay-control request fields and output metadata do not exist yet.

**Step 2: Implement minimal replay-control support**

Add:
- `SecurityExecutionReplayCommitControl { target_execution_record_ref, commit_idempotency_key, canonical_commit_payload_hash, source_p19c_ref }`
- optional `replay_commit_control` on `SecurityExecutionRecordRequest`
- optional machine-readable replay metadata fields on `SecurityExecutionRecordDocument`
- a session-local check before upsert that returns same-payload already-exists or conflicting-idempotency without overwriting

**Step 3: Verify replay-control GREEN**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_execution_record_green'; cargo test --test security_execution_record_cli -- --nocapture
```

Expected: replay-control tests pass and existing execution-record behavior remains unchanged when `replay_commit_control` is absent.

### Task 2: P19D CLI RED Tests

**Files:**
- Create: `D:\SM\tests\security_portfolio_execution_replay_commit_writer_cli.rs`

**Step 1: Write failing tests**

Add tests for:
- catalog includes `security_portfolio_execution_replay_commit_writer`
- no-work P19C document returns `no_commit_work` with zero runtime writes
- wrong P19C document type hard-fails
- wrong P19C contract version hard-fails
- P19C input with non-zero `runtime_write_count` hard-fails
- P19C input with row `runtime_execution_record_ref` hard-fails
- unsupported `commit_mode` hard-fails
- payload hash drift hard-fails
- duplicate commit idempotency keys hard-fail
- duplicate deterministic target refs hard-fail
- already-committed matching runtime record returns `already_committed` without another write
- existing target runtime record with conflicting idempotency/hash evidence returns `idempotency_conflict`
- one happy-path row commits through `security_execution_record` and returns a runtime ref
- partial runtime failure reports `partial_commit_failure`, not all-or-nothing success
- source guard confirms P19D calls `security_execution_record`
- source guard confirms P19D does not call direct runtime write APIs, including repository write functions, SQLite `execute`, `open_session`, or store mutation APIs
- source guard allows `SecurityExecutionStore::load_execution_record` only as a read precheck
- source guard confirms P19D output includes a non-atomicity notice and does not claim bundle atomicity

**Step 2: Run test to verify RED**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_red'; cargo test --test security_portfolio_execution_replay_commit_writer_cli -- --nocapture
```

Expected: fail because the P19D route does not exist.

### Task 3: P19D Replay Metadata And Conflict RED Tests

**Files:**
- Modify: `D:\SM\tests\security_portfolio_execution_replay_commit_writer_cli.rs`

**Step 1: Add P19D-specific RED tests**

Add tests proving:
- P19D builds replay-control requests with deterministic target refs derived from commit idempotency keys
- committed rows expose machine-readable replay metadata in runtime records
- already-committed matching runtime rows are detected from machine-readable fields
- conflicting existing target refs produce `idempotency_conflict`
- conflicting existing target refs are not overwritten by P19D or by the inner `security_execution_record` replay-control path

**Step 2: Run test to verify RED**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_metadata_red'; cargo test --test security_portfolio_execution_replay_commit_writer_cli -- --nocapture
```

Expected: fail because P19D does not exist yet or does not emit replay-control requests.

**Step 3: Keep conflict tests separate from happy path**

Expected: a focused happy-path implementation cannot pass the conflict tests unless idempotency and no-overwrite behavior are implemented.

### Task 4: P19D Implementation

**Files:**
- Create: `D:\SM\src\ops\security_portfolio_execution_replay_commit_writer.rs`

**Step 1: Define contracts**

Required structs:
- `SecurityPortfolioExecutionReplayCommitWriterRequest`
- `SecurityPortfolioExecutionReplayCommitWriterRow`
- `SecurityPortfolioExecutionReplayCommitWriterDocument`
- `SecurityPortfolioExecutionReplayCommitWriterResult`
- `SecurityPortfolioExecutionReplayCommitWriterError`

**Step 2: Implement validation**

Validate:
- `commit_mode == "controlled_per_row"`
- P19C document type/version/mode/status
- P19C `runtime_write_count == 0`
- no P19C row runtime refs
- unique commit keys
- unique target refs
- fresh hash of `commit_payload_preview` equals P19C `canonical_commit_payload_hash`

**Step 3: Implement already-committed precheck**

Use:
- `SecurityExecutionStore::workspace_default()`
- `load_execution_record(target_execution_record_ref)`

Rules:
- missing record: safe to call `security_execution_record`
- matching machine-readable replay metadata fields: return `already_committed`
- conflicting machine-readable replay metadata fields: return `idempotency_conflict`

**Step 4: Implement per-row commit**

Build `SecurityExecutionRecordRequest` from P19C payload preview:
- symbol
- analysis date
- decision ref
- execution action
- execution status
- executed gross pct
- account id
- market/sector context
- as-of date
- actual entry date
- actual position pct
- default review/lookback/risk fields matching P15 unless a better local constant exists
- `replay_commit_control` containing deterministic target ref, P19C ref, commit key, and payload hash
- supplemental human-readable notes with P19D, P19C ref, commit key, and payload hash

Call `security_execution_record`.

**Step 5: Emit document**

Include row statuses, counts, refs, `runtime_write_count`, blockers, rationale, and `non_atomicity_notice`.

### Task 5: Public Boundary Wiring

**Files:**
- Modify: `D:\SM\src\ops\stock.rs`
- Modify: `D:\SM\src\ops\stock_execution_and_position_management.rs`
- Modify: `D:\SM\src\tools\catalog.rs`
- Modify: `D:\SM\src\tools\dispatcher.rs`
- Modify: `D:\SM\src\tools\dispatcher\stock_ops.rs`

**Step 1: Wire P19D after P19C**

Add module/export/catalog/dispatcher route immediately after P19C.

**Step 2: Run focused GREEN**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_green'; cargo test --test security_portfolio_execution_replay_commit_writer_cli -- --nocapture
```

Expected: focused P19D tests pass.

### Task 6: Boundary And Governance Sync

**Files:**
- Modify: `D:\SM\tests\stock_formal_boundary_manifest_source_guard.rs`
- Modify: `D:\SM\docs\governance\contract_registry.md`
- Modify: `D:\SM\docs\governance\decision_log.md`
- Modify: `D:\SM\docs\handoff\CURRENT_STATUS.md`
- Modify: `D:\SM\docs\handoff\HANDOFF_ISSUES.md`

**Step 1: Sync frozen manifest and governance docs**

Wording must say:
- P19D is controlled per-row runtime replay commit
- P19D writes only through `security_execution_record`
- P19D is not broker execution
- P19D is not bundle atomic
- P19D preserves P19C hash/idempotency evidence

**Step 2: Run guard tests**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_guard'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_guard'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_guard'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
```

Expected: all guard tests pass.

### Task 7: Final Verification And Journal

**Files:**
- Modify: `D:\SM\.trae\CHANGELOG_TASK.md`

**Step 1: Run final focused verification**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_final'; cargo test --test security_portfolio_execution_replay_commit_writer_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_final'; cargo test --test security_execution_record_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_final'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_final'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_final'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_final'; cargo check
```

Expected: focused tests, adjacent execution-record tests, guards, and `cargo check` pass.

**Step 2: Append task journal**

Append one entry to `D:\SM\.trae\CHANGELOG_TASK.md` with changed files, reason, remaining gaps, risks, and verification commands.

**Step 3: Run repository-wide regression**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p19d'; cargo test -- --nocapture
```

Expected: repository-wide regression completes with exit code 0 before claiming P19D complete.
