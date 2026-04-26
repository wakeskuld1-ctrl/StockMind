# P19E Replay Commit Audit Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add P19E as a read-only runtime replay verification layer that audits P19D replay commit results without writing runtime facts or claiming lifecycle closeout.

**Architecture:** P19E consumes a P19D commit-writer document, validates P19D identity and non-atomicity metadata, reads target runtime execution records, compares machine-readable replay metadata, and emits row-level audit states plus aggregate counts. It is deliberately read-only and must preserve P19D `committed`, `already_committed`, `commit_failed`, and `idempotency_conflict` truth without turning replay commits into lifecycle closure.

**Tech Stack:** Rust, serde, thiserror, existing StockMind CLI dispatcher/catalog, `SecurityExecutionStore` read APIs, P19D replay metadata fields, Cargo integration tests.

---

### Risk Synchronization Gate
**Risk subprocess mode:** user-approved-subagent.

**Question asked:** What artifact will drift if P19E exposes a new public read-only replay audit boundary after P19D?

**Boundary items:**
- `security_portfolio_execution_replay_commit_audit`
- `SecurityPortfolioExecutionReplayCommitAuditRequest`
- `SecurityPortfolioExecutionReplayCommitAuditDocument`
- `SecurityPortfolioExecutionReplayCommitAuditResult`
- public stock module export
- execution-and-position-management grouped export
- tool catalog entry
- dispatcher route
- frozen stock-boundary manifest entry
- contract registry and decision log rows
- P19E source guard for read-only runtime behavior and no lifecycle closeout claim

**Must-sync files:**
- `D:\SM\docs\plans\2026-04-26-p19e-replay-commit-audit-design.md`
- `D:\SM\docs\plans\2026-04-26-p19e-replay-commit-audit-plan.md`
- `D:\SM\src\ops\security_portfolio_execution_replay_commit_audit.rs`
- `D:\SM\src\ops\stock.rs`
- `D:\SM\src\ops\stock_execution_and_position_management.rs`
- `D:\SM\src\tools\catalog.rs`
- `D:\SM\src\tools\dispatcher.rs`
- `D:\SM\src\tools\dispatcher\stock_ops.rs`
- `D:\SM\tests\security_portfolio_execution_replay_commit_audit_cli.rs`
- `D:\SM\tests\stock_formal_boundary_manifest_source_guard.rs`
- `D:\SM\docs\governance\contract_registry.md`
- `D:\SM\docs\governance\decision_log.md`
- `D:\SM\docs\handoff\CURRENT_STATUS.md`
- `D:\SM\docs\handoff\HANDOFF_ISSUES.md`
- `D:\SM\.trae\CHANGELOG_TASK.md`

**Must-run checks:**
- P19E RED focused test
- P19E GREEN focused test
- P19D replay commit writer adjacent tests
- P19E source guard proving read-only behavior
- `stock_formal_boundary_manifest_source_guard`
- `stock_catalog_grouping_source_guard`
- `stock_dispatcher_grouping_source_guard`
- `cargo check`
- repository-wide `cargo test -- --nocapture`

**Blockers resolved into hard constraints:**
- P19E must read runtime records only; it must not write runtime facts.
- P19E must not call `security_execution_record`.
- P19E must not call SQLite `execute`, `open_session`, store mutation APIs, or repository write functions.
- P19E must use machine-readable replay metadata fields, not notes parsing.
- P19E must preserve P19D non-atomicity and row status truth.
- P19E must not claim broker fill, position materialization, or lifecycle closeout.

---

### Task 1: P19E CLI RED Tests

**Files:**
- Create: `D:\SM\tests\security_portfolio_execution_replay_commit_audit_cli.rs`

**Step 1: Write failing tests**

Add tests for:
- catalog includes `security_portfolio_execution_replay_commit_audit`
- wrong P19D document type hard-fails
- wrong P19D contract version hard-fails
- missing P19D `non_atomicity_notice` hard-fails
- P19D no-work document returns `no_commit_work` with `runtime_write_count == 0`
- P19D `committed` row with matching runtime replay metadata returns `verified`
- P19D `already_committed` row with matching runtime replay metadata returns `already_committed_verified`
- P19D `committed` row with missing runtime record returns `missing_runtime_record`
- P19D `committed` row with mismatched runtime idempotency key returns `metadata_mismatch`
- P19D `committed` row with mismatched runtime payload hash returns `metadata_mismatch`
- P19D `committed` row with mismatched runtime source P19C ref returns `metadata_mismatch`
- P19D `commit_failed` row returns `commit_failed_preserved`
- P19D `idempotency_conflict` row returns `idempotency_conflict_confirmed`
- mixed verified and failed audit rows return `partial_audit_failure`
- source guard confirms P19E does not call `security_execution_record`
- source guard confirms P19E does not call direct runtime write APIs, including repository write functions, SQLite `execute`, `open_session`, or store mutation APIs
- source guard confirms P19E preserves non-atomicity wording and does not claim lifecycle closeout

**Step 2: Run test to verify RED**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_red'; cargo test --test security_portfolio_execution_replay_commit_audit_cli -- --nocapture
```

Expected: fail because the P19E route and module do not exist.

### Task 2: P19E Implementation

**Files:**
- Create: `D:\SM\src\ops\security_portfolio_execution_replay_commit_audit.rs`

**Step 1: Define contracts**

Required structs:
- `SecurityPortfolioExecutionReplayCommitAuditRequest`
- `SecurityPortfolioExecutionReplayCommitAuditRow`
- `SecurityPortfolioExecutionReplayCommitAuditDocument`
- `SecurityPortfolioExecutionReplayCommitAuditResult`
- `SecurityPortfolioExecutionReplayCommitAuditError`

**Step 2: Implement P19D identity validation**

Validate:
- `document_type == "security_portfolio_execution_replay_commit_writer"`
- `contract_version == "security_portfolio_execution_replay_commit_writer.v1"`
- P19D `non_atomicity_notice` is present
- committed/already-committed rows have target runtime refs, commit idempotency keys, and canonical payload hashes

**Step 3: Implement read-only runtime lookup**

Use only the existing runtime store read path to load target execution records.

Rules:
- missing runtime record for a P19D committed/already-committed row becomes `missing_runtime_record`
- matching runtime replay metadata becomes `verified` or `already_committed_verified`
- idempotency key, payload hash, or source P19C ref mismatch becomes `metadata_mismatch`
- P19D `commit_failed` rows become `commit_failed_preserved`
- P19D `idempotency_conflict` rows become `idempotency_conflict_confirmed`
- P19D no-work rows become `skipped_no_commit_work_preserved`

**Step 4: Emit audit document**

Include:
- document identity and contract version
- source P19D ref
- preserved P19D non-atomicity notice
- `runtime_write_count = 0`
- row audit statuses
- counts
- blockers
- aggregate document status

### Task 3: Public Boundary Wiring

**Files:**
- Modify: `D:\SM\src\ops\stock.rs`
- Modify: `D:\SM\src\ops\stock_execution_and_position_management.rs`
- Modify: `D:\SM\src\tools\catalog.rs`
- Modify: `D:\SM\src\tools\dispatcher.rs`
- Modify: `D:\SM\src\tools\dispatcher\stock_ops.rs`

**Step 1: Wire P19E after P19D**

Add module/export/catalog/dispatcher route immediately after P19D.

**Step 2: Run focused GREEN**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_green'; cargo test --test security_portfolio_execution_replay_commit_audit_cli -- --nocapture
```

Expected: focused P19E tests pass.

### Task 4: Boundary And Governance Sync

**Files:**
- Modify: `D:\SM\tests\stock_formal_boundary_manifest_source_guard.rs`
- Modify: `D:\SM\docs\governance\contract_registry.md`
- Modify: `D:\SM\docs\governance\decision_log.md`
- Modify: `D:\SM\docs\handoff\CURRENT_STATUS.md`
- Modify: `D:\SM\docs\handoff\HANDOFF_ISSUES.md`

**Step 1: Run guard before sync**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_guard'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
```

Expected: fail if P19E is exposed in `stock.rs` but absent from the frozen public manifest.

**Step 2: Sync frozen manifest and governance docs**

Wording must say:
- P19E is a read-only replay commit audit layer
- P19E consumes P19D and reads runtime execution records
- P19E verifies machine-readable replay metadata
- P19E writes no runtime facts
- P19E is not broker execution, broker-fill replay, position materialization, or lifecycle closeout

**Step 3: Run guard tests**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_guard'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_guard'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_guard'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
```

Expected: all guard tests pass.

### Task 5: Adjacent Regression And Source Guard Closeout

**Files:**
- Modify if needed: `D:\SM\tests\security_portfolio_execution_replay_commit_audit_cli.rs`

**Step 1: Run adjacent P19D tests**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_adjacent'; cargo test --test security_portfolio_execution_replay_commit_writer_cli -- --nocapture
```

Expected: P19D remains green and its runtime writer semantics are unchanged.

**Step 2: Re-run focused P19E source guards**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_adjacent'; cargo test --test security_portfolio_execution_replay_commit_audit_cli -- --nocapture
```

Expected: P19E focused tests and source guards pass.

### Task 6: Final Verification And Journal

**Files:**
- Modify: `D:\SM\.trae\CHANGELOG_TASK.md`

**Step 1: Run final focused verification**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_final'; cargo test --test security_portfolio_execution_replay_commit_audit_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_final'; cargo test --test security_portfolio_execution_replay_commit_writer_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_final'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_final'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_final'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_final'; cargo check
```

Expected: focused tests, adjacent P19D tests, guards, and `cargo check` pass.

**Step 2: Append task journal**

Append one entry to `D:\SM\.trae\CHANGELOG_TASK.md` with changed files, reason, remaining gaps, risks, and verification commands.

**Step 3: Run repository-wide regression**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p19e'; cargo test -- --nocapture
```

Expected: repository-wide regression completes with exit code 0 before claiming P19E complete.
