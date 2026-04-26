# P20A Lifecycle Closeout Readiness Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add P20A as a side-effect-free lifecycle closeout readiness layer that consumes P19E audit truth and emits row-level closeout-preflight eligibility without writing runtime, post-trade, archive, position, or lifecycle facts.

**Architecture:** P20A validates a P19E `SecurityPortfolioExecutionReplayCommitAuditDocument`, maps only `verified` and `already_committed_verified` rows to `eligible_for_closeout_preflight`, preserves every other P19E row state as an explicit blocker, and emits aggregate readiness counts. P20A deliberately does not call runtime writers, `security_post_trade_review`, or any closed-position archive path; it prepares a governed fact for a future P20B writer.

**Tech Stack:** Rust, serde, thiserror, existing StockMind CLI dispatcher/catalog, P19E audit contracts, Cargo integration tests and source guards.

---

### Risk Synchronization Gate
**Risk subprocess mode:** user-approved-subagent.

**Question asked:** What artifact will drift if P20A exposes lifecycle closeout readiness after P19E, and what semantic boundary will be crossed if readiness is mistaken for lifecycle closure?

**Boundary items:**
- `security_portfolio_execution_lifecycle_closeout_readiness`
- `SecurityPortfolioExecutionLifecycleCloseoutReadinessRequest`
- `SecurityPortfolioExecutionLifecycleCloseoutReadinessRow`
- `SecurityPortfolioExecutionLifecycleCloseoutReadinessDocument`
- `SecurityPortfolioExecutionLifecycleCloseoutReadinessResult`
- public stock module export
- execution-and-position-management grouped export
- tool catalog entry
- dispatcher route
- frozen stock-boundary manifest entry
- contract registry and decision log rows
- P20A source guard proving side-effect-free readiness and no lifecycle closeout claim

**Must-sync files:**
- `D:\SM\docs\plans\2026-04-26-p20a-lifecycle-closeout-readiness-design.md`
- `D:\SM\docs\plans\2026-04-26-p20a-lifecycle-closeout-readiness-plan.md`
- `D:\SM\src\ops\security_portfolio_execution_lifecycle_closeout_readiness.rs`
- `D:\SM\src\ops\stock.rs`
- `D:\SM\src\ops\stock_execution_and_position_management.rs`
- `D:\SM\src\tools\catalog.rs`
- `D:\SM\src\tools\dispatcher.rs`
- `D:\SM\src\tools\dispatcher\stock_ops.rs`
- `D:\SM\tests\security_portfolio_execution_lifecycle_closeout_readiness_cli.rs`
- `D:\SM\tests\stock_formal_boundary_manifest_source_guard.rs`
- `D:\SM\docs\governance\contract_registry.md`
- `D:\SM\docs\governance\decision_log.md`
- `D:\SM\docs\handoff\CURRENT_STATUS.md`
- `D:\SM\docs\handoff\HANDOFF_ISSUES.md`
- `D:\SM\.trae\CHANGELOG_TASK.md`

**Must-run checks:**
- P20A RED focused test
- P20A GREEN focused test
- adjacent P19E focused test
- P20A source guard proving no write/archive/post-trade paths
- `stock_formal_boundary_manifest_source_guard`
- `stock_catalog_grouping_source_guard`
- `stock_dispatcher_grouping_source_guard`
- `cargo check`
- repository-wide `cargo test -- --nocapture`

**Blockers resolved into hard constraints:**
- P20A must not write runtime facts.
- P20A must not call `security_execution_record`.
- P20A must not call `security_post_trade_review`.
- P20A must not call or depend on `security_closed_position_archive`.
- P20A must not call SQLite `execute`, `open_session`, store mutation APIs, or repository write functions.
- P20A must not treat P19E `verified` as lifecycle closed, broker fill, closed execution record, or closed-position archive readiness.
- P20A must preserve P19D/P19E non-atomic partial truth and row-level blockers.
- Current local evidence does not show an available `security_closed_position_archive` implementation or route; P20A may only name it as future boundary context.

---

### Task 1: P20A CLI RED Tests

**Files:**
- Create: `D:\SM\tests\security_portfolio_execution_lifecycle_closeout_readiness_cli.rs`

**Step 1: Write failing tests**

Add tests for:
- catalog includes `security_portfolio_execution_lifecycle_closeout_readiness`
- wrong P19E document type hard-fails
- wrong P19E contract version hard-fails
- P19E `runtime_write_count != 0` hard-fails
- missing P19D source ref hard-fails
- missing P19C source ref hard-fails
- missing source non-atomicity notice hard-fails
- P19E no-row document returns `no_closeout_candidates` with `runtime_write_count == 0`
- P19E `verified` row returns `eligible_for_closeout_preflight`
- P19E `already_committed_verified` row returns `eligible_for_closeout_preflight`
- P19E `missing_runtime_record` row returns `blocked_missing_runtime_record`
- P19E `metadata_mismatch` row returns `blocked_metadata_mismatch`
- P19E `commit_failed_preserved` row returns `blocked_commit_failed`
- P19E `idempotency_conflict_confirmed` row returns `blocked_idempotency_conflict`
- P19E `skipped_no_commit_work_preserved` row returns `blocked_no_commit_work`
- P19E `not_auditable` row returns `blocked_not_auditable`
- unknown P19E row status returns `blocked_unknown_audit_status`
- mixed eligible and blocked rows return `partial_closeout_preflight_ready`
- source guard confirms P20A does not call `security_execution_record`
- source guard confirms P20A does not call `security_post_trade_review`
- source guard confirms P20A does not call or reference a closed-position archive writer path except in explicit negative guard text
- source guard confirms P20A does not call direct runtime write APIs, including repository write functions, SQLite `execute`, `open_session`, or store mutation APIs
- source guard confirms P20A emits `runtime_write_count = 0` and does not claim lifecycle closure

**Step 2: Run test to verify RED**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_red'; cargo test --test security_portfolio_execution_lifecycle_closeout_readiness_cli -- --nocapture
```

Expected: fail because the P20A route and module do not exist.

### Task 2: P20A Implementation

**Files:**
- Create: `D:\SM\src\ops\security_portfolio_execution_lifecycle_closeout_readiness.rs`

**Step 1: Define contracts**

Required structs:
- `SecurityPortfolioExecutionLifecycleCloseoutReadinessRequest`
- `SecurityPortfolioExecutionLifecycleCloseoutReadinessRow`
- `SecurityPortfolioExecutionLifecycleCloseoutReadinessDocument`
- `SecurityPortfolioExecutionLifecycleCloseoutReadinessResult`
- `SecurityPortfolioExecutionLifecycleCloseoutReadinessError`

**Step 2: Implement P19E identity validation**

Validate:
- `document_type == "security_portfolio_execution_replay_commit_audit"`
- `contract_version == "security_portfolio_execution_replay_commit_audit.v1"`
- `runtime_write_count == 0`
- source P19D ref is present
- source P19C ref is present
- source non-atomicity notice is present

**Step 3: Implement eligible-row evidence validation**

For rows with P19E status `verified` or `already_committed_verified`, validate:
- target execution record ref is present
- commit idempotency key is present
- canonical commit payload hash is present
- source P19C ref is present
- runtime replay idempotency key is present
- runtime replay payload hash is present
- runtime replay source P19C ref is present

Hard-fail the request if an eligible row lacks required machine-readable evidence.

**Step 4: Implement row-status mapping**

Map:
- `verified` -> `eligible_for_closeout_preflight`
- `already_committed_verified` -> `eligible_for_closeout_preflight`
- `missing_runtime_record` -> `blocked_missing_runtime_record`
- `metadata_mismatch` -> `blocked_metadata_mismatch`
- `commit_failed_preserved` -> `blocked_commit_failed`
- `idempotency_conflict_confirmed` -> `blocked_idempotency_conflict`
- `skipped_no_commit_work_preserved` -> `blocked_no_commit_work`
- `not_auditable` -> `blocked_not_auditable`
- unknown status -> `blocked_unknown_audit_status`

**Step 5: Emit readiness document**

Include:
- document identity and contract version
- source P19E ref
- source P19D ref
- source P19C ref
- preserved source non-atomicity notice
- `runtime_write_count = 0`
- row readiness statuses
- blockers
- counts
- aggregate readiness status
- summary text that says readiness is not lifecycle closure

Aggregate status rules:
- zero rows -> `no_closeout_candidates`
- eligible rows only -> `closeout_preflight_ready`
- eligible and blocked rows -> `partial_closeout_preflight_ready`
- blocked rows only -> `blocked`

### Task 3: Public Boundary Wiring

**Files:**
- Modify: `D:\SM\src\ops\stock.rs`
- Modify: `D:\SM\src\ops\stock_execution_and_position_management.rs`
- Modify: `D:\SM\src\tools\catalog.rs`
- Modify: `D:\SM\src\tools\dispatcher.rs`
- Modify: `D:\SM\src\tools\dispatcher\stock_ops.rs`

**Step 1: Wire P20A after P19E**

Add module/export/catalog/dispatcher route immediately after P19E.

**Step 2: Run focused GREEN**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_green'; cargo test --test security_portfolio_execution_lifecycle_closeout_readiness_cli -- --nocapture
```

Expected: focused P20A tests pass.

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
$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_guard'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
```

Expected: fail if P20A is exposed in `stock.rs` but absent from the frozen public manifest.

**Step 2: Sync frozen manifest and governance docs**

Wording must say:
- P20A is side-effect-free lifecycle closeout readiness
- P20A consumes P19E audit truth
- P20A maps only `verified` / `already_committed_verified` rows to readiness eligibility
- P20A writes no runtime facts
- P20A does not call `security_execution_record`
- P20A does not call `security_post_trade_review`
- P20A does not call or depend on `security_closed_position_archive`
- P20A is not broker execution, broker-fill replay, position materialization, or lifecycle closure

**Step 3: Run guard tests**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_guard'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_guard'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_guard'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
```

Expected: all guard tests pass.

### Task 5: Adjacent Regression And Source Guard Closeout

**Files:**
- Modify if needed: `D:\SM\tests\security_portfolio_execution_lifecycle_closeout_readiness_cli.rs`

**Step 1: Run adjacent P19E tests**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_adjacent'; cargo test --test security_portfolio_execution_replay_commit_audit_cli -- --nocapture
```

Expected: P19E remains green and its read-only audit semantics are unchanged.

**Step 2: Re-run focused P20A source guards**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_adjacent'; cargo test --test security_portfolio_execution_lifecycle_closeout_readiness_cli -- --nocapture
```

Expected: P20A focused tests and source guards pass.

### Task 6: Final Verification And Journal

**Files:**
- Modify: `D:\SM\.trae\CHANGELOG_TASK.md`

**Step 1: Run final focused verification**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_final'; cargo test --test security_portfolio_execution_lifecycle_closeout_readiness_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_final'; cargo test --test security_portfolio_execution_replay_commit_audit_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_final'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_final'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_final'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_final'; cargo check
```

Expected: focused tests, adjacent P19E tests, guards, and `cargo check` pass.

**Step 2: Append task journal**

Append one entry to `D:\SM\.trae\CHANGELOG_TASK.md` with changed files, reason, remaining gaps, risks, and verification commands.

**Step 3: Run repository-wide regression**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p20a'; cargo test -- --nocapture
```

Expected: repository-wide regression completes with exit code 0 before claiming P20A complete.
