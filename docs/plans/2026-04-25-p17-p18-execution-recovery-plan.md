# P17/P18 Execution Recovery Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rebuild the missing P17 reconciliation bridge and P18 repair-intent package in `D:\SM` after P16.

**Architecture:** P17 is a side-effect-free consumer of `SecurityPortfolioExecutionStatusBridgeDocument` and emits frozen reconciliation truth. P18 is a side-effect-free consumer of P17 and emits repair intent only, keeping replay, execution, position materialization, and lifecycle closure out of this recovery slice.

**Tech Stack:** Rust, serde, thiserror, existing StockMind CLI dispatcher/catalog, Cargo integration tests.

---

### Task 1: P17 CLI Red Tests

**Files:**
- Create: `tests/security_portfolio_execution_reconciliation_bridge_cli.rs`

**Step 1: Write the failing test**

Add tests for:
- catalog includes `security_portfolio_execution_reconciliation_bridge`
- a fully applied P16 document produces a fully settled reconciliation artifact
- an apply-failed P16 row becomes `reconciliation_required`
- unsupported P16 execution status is rejected

**Step 2: Run test to verify it fails**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p17_recovery_red'; cargo test --test security_portfolio_execution_reconciliation_bridge_cli -- --nocapture
```

Expected: fail because the test target or tool route does not exist yet.

### Task 2: P17 Implementation

**Files:**
- Create: `src/ops/security_portfolio_execution_reconciliation_bridge.rs`
- Modify: `src/ops/stock.rs`
- Modify: `src/ops/stock_execution_and_position_management.rs`
- Modify: `src/tools/catalog.rs`
- Modify: `src/tools/dispatcher.rs`
- Modify: `src/tools/dispatcher/stock_ops.rs`

**Step 1: Write minimal implementation**

Define request/result/document/row/error types and `security_portfolio_execution_reconciliation_bridge`.

Core row mapping:
- `applied` -> `settled`
- `skipped_non_executable_hold` -> `skipped_hold`
- `apply_failed` -> `reconciliation_required`

Batch mapping:
- `fully_applied` -> `fully_settled`
- `applied_with_open_items`, `applied_with_skipped_holds`, `partial_failure` -> `reconciliation_required`
- `rejected` -> `blocked`

**Step 2: Run test to verify it passes**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p17_recovery_green'; cargo test --test security_portfolio_execution_reconciliation_bridge_cli -- --nocapture
```

Expected: all P17 focused tests pass.

### Task 3: P18 CLI Red Tests

**Files:**
- Create: `tests/security_portfolio_execution_repair_package_cli.rs`

**Step 1: Write the failing test**

Add tests for:
- catalog includes `security_portfolio_execution_repair_package`
- fully settled P17 document produces no repair rows
- manual follow-up row is emitted only when P17 row explicitly requires manual follow-up
- blocked/pending governance text produces `blocked_pending_decision`
- retryable failed row produces `governed_retry_candidate`
- ambiguous unresolved row hard-fails

**Step 2: Run test to verify it fails**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p18_recovery_red'; cargo test --test security_portfolio_execution_repair_package_cli -- --nocapture
```

Expected: fail because the P18 module and route do not exist yet.

### Task 4: P18 Implementation

**Files:**
- Create: `src/ops/security_portfolio_execution_repair_package.rs`
- Modify: `src/ops/stock.rs`
- Modify: `src/ops/stock_execution_and_position_management.rs`
- Modify: `src/tools/catalog.rs`
- Modify: `src/tools/dispatcher.rs`
- Modify: `src/tools/dispatcher/stock_ops.rs`

**Step 1: Write minimal implementation**

Define request/result/document/row/error types and `security_portfolio_execution_repair_package`.

Repair classification:
- `requires_manual_follow_up=true` -> `manual_follow_up`
- blocker/pending text contains blocked/pending governance signals -> `blocked_pending_decision`
- failed row with execution refs or retryable text -> `governed_retry_candidate`
- otherwise unresolved row -> hard fail `AmbiguousRepairClassification`

**Step 2: Run test to verify it passes**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p18_recovery_green'; cargo test --test security_portfolio_execution_repair_package_cli -- --nocapture
```

Expected: all P18 focused tests pass.

### Task 5: Guard And Governance Updates

**Files:**
- Modify: `tests/stock_formal_boundary_manifest_source_guard.rs`
- Modify: `docs/governance/contract_registry.md`
- Modify: `docs/governance/decision_log.md`
- Modify: `docs/handoff/CURRENT_STATUS.md`
- Modify: `docs/handoff/HANDOFF_ISSUES.md`

**Step 1: Update source guards and docs**

Add P17/P18 to the frozen public manifest expectation and governance docs. Keep wording explicit that P18 is not P19 replay/execution/lifecycle closeout.

**Step 2: Run guard tests**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p17_p18_recovery_guard'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p17_p18_recovery_guard'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p17_p18_recovery_guard'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
```

Expected: all guard tests pass.

### Task 6: Final Focused Verification And Journal

**Files:**
- Modify: `.trae/CHANGELOG_TASK.md`

**Step 1: Run focused verification**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p17_p18_recovery_final'; cargo test --test security_portfolio_execution_reconciliation_bridge_cli --test security_portfolio_execution_repair_package_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p17_p18_recovery_final'; cargo check
```

Expected: focused P17/P18 tests and cargo check pass. If cargo check is blocked by unrelated dirty-worktree code, record the exact failure and do not claim repository health.

**Step 2: Append task journal**

Append one entry to `.trae/CHANGELOG_TASK.md` with changed files, reason, remaining gaps, risks, and verification commands.
