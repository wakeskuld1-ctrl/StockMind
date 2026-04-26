# P19A Execution Replay Request Package Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add P19A as a side-effect-free replay request package downstream of the P18 repair-intent package.

**Architecture:** P19A consumes only `SecurityPortfolioExecutionRepairPackageDocument`, validates P18 lineage and summary consistency, selects only `governed_retry_candidate` rows, and emits a replay request document for a future executor. It does not write runtime state, call execution-record persistence, replay broker fills, materialize positions, or close lifecycle.

**Tech Stack:** Rust, serde, thiserror, existing StockMind CLI dispatcher/catalog, Cargo integration tests.

---

### Task 1: P19A CLI Red Tests

**Files:**
- Create: `tests/security_portfolio_execution_replay_request_package_cli.rs`

**Step 1: Write the failing tests**

Add tests for:
- catalog includes `security_portfolio_execution_replay_request_package`
- a no-repair P18 document emits an empty replay request package
- only `governed_retry_candidate` rows become replay request rows
- `manual_follow_up` and `blocked_pending_decision` rows are excluded and counted
- unknown `repair_class` is rejected
- retry candidate without replay evidence is rejected

**Step 2: Run test to verify it fails**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_red'; cargo test --test security_portfolio_execution_replay_request_package_cli -- --nocapture
```

Expected: fail because the P19A route does not exist yet.

### Task 2: P19A Implementation

**Files:**
- Create: `src/ops/security_portfolio_execution_replay_request_package.rs`
- Modify: `src/ops/stock.rs`
- Modify: `src/ops/stock_execution_and_position_management.rs`
- Modify: `src/tools/catalog.rs`
- Modify: `src/tools/dispatcher.rs`
- Modify: `src/tools/dispatcher/stock_ops.rs`

**Step 1: Write minimal implementation**

Define request/result/document/row/error types and `security_portfolio_execution_replay_request_package`.

Core behavior:
- validate required P18 lineage refs
- validate P18 summary counts
- validate P18 `repair_status` values: `no_repair_required` and `repair_required`
- include only `governed_retry_candidate`
- exclude `manual_follow_up` and `blocked_pending_decision`
- reject unknown repair classes
- reject retry candidates without `execution_record_ref`, `execution_journal_ref`, or retry/replay blocker text

**Step 2: Run test to verify it passes**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_green'; cargo test --test security_portfolio_execution_replay_request_package_cli -- --nocapture
```

Expected: all P19A focused tests pass.

### Task 3: Guard And Governance Updates

**Files:**
- Modify: `tests/stock_formal_boundary_manifest_source_guard.rs`
- Modify: `docs/governance/contract_registry.md`
- Modify: `docs/governance/decision_log.md`
- Modify: `docs/handoff/CURRENT_STATUS.md`
- Modify: `docs/handoff/HANDOFF_ISSUES.md`

**Step 1: Update source guards and docs**

Add P19A to the frozen public manifest expectation and governance docs. Keep wording explicit that P19A is a replay-request package, not an executor.

**Step 2: Run guard tests**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_guard'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_guard'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_guard'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
```

Expected: all guard tests pass.

### Task 4: Final Focused Verification And Journal

**Files:**
- Modify: `.trae/CHANGELOG_TASK.md`

**Step 1: Run focused verification**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_final'; cargo test --test security_portfolio_execution_replay_request_package_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_final'; cargo check
```

Expected: focused P19A tests and `cargo check` pass.

**Step 2: Append task journal**

Append one entry to `.trae/CHANGELOG_TASK.md` with changed files, reason, remaining gaps, risks, and verification commands.
