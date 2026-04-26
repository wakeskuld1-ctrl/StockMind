# P19B Execution Replay Dry-Run Executor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add P19B as a controlled replay executor boundary that initially validates replay work in `dry_run` mode only.

**Architecture:** P19B consumes only `SecurityPortfolioExecutionReplayRequestPackageDocument`, validates lineage, replay-request status, row evidence, and deterministic idempotency keys, then emits a dry-run executor document. It rejects `commit` mode and performs no runtime writes, broker-fill replay, position materialization, or lifecycle closeout.

**Tech Stack:** Rust, serde, thiserror, existing StockMind CLI dispatcher/catalog, Cargo integration tests.

---

### Task 1: P19B CLI Red Tests

**Files:**
- Create: `tests/security_portfolio_execution_replay_executor_cli.rs`

**Step 1: Write the failing tests**

Add tests for:
- catalog includes `security_portfolio_execution_replay_executor`
- no-replay P19A package produces a dry-run document with no rows
- one ready replay request row produces one `validated_for_dry_run` executor row
- `commit` mode is rejected in this phase
- duplicate deterministic idempotency keys are rejected
- replay request rows without evidence are rejected
- non-ready replay request rows are rejected

**Step 2: Run test to verify it fails**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_red'; cargo test --test security_portfolio_execution_replay_executor_cli -- --nocapture
```

Expected: fail because the P19B route does not exist yet.

### Task 2: P19B Dry-Run Implementation

**Files:**
- Create: `src/ops/security_portfolio_execution_replay_executor.rs`
- Modify: `src/ops/stock.rs`
- Modify: `src/ops/stock_execution_and_position_management.rs`
- Modify: `src/tools/catalog.rs`
- Modify: `src/tools/dispatcher.rs`
- Modify: `src/tools/dispatcher/stock_ops.rs`

**Step 1: Write minimal implementation**

Define request/result/document/row/error types and `security_portfolio_execution_replay_executor`.

Core behavior:
- accept `execution_mode = "dry_run"` only
- reject `execution_mode = "commit"` with a clear unsupported-mode error
- validate required P19A lineage refs
- validate P19A summary counts
- validate P19A `replay_request_status` values: `no_replay_requested` and `replay_requested`
- validate each replay row has `repair_class == "governed_retry_candidate"`
- validate each replay row has `replay_request_status == "ready_for_replay_request"`
- validate each replay row has at least one `replay_evidence_refs` entry
- derive a deterministic idempotency key from account, analysis date, symbol, action, requested gross pct, P19A package ref, and evidence refs
- reject duplicate idempotency keys in one document
- emit dry-run rows with `dry_run_status = "validated_for_dry_run"`
- never create runtime execution refs or call `security_execution_record`

**Step 2: Run test to verify it passes**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_green'; cargo test --test security_portfolio_execution_replay_executor_cli -- --nocapture
```

Expected: all P19B focused tests pass.

### Task 3: Guard And Governance Updates

**Files:**
- Modify: `tests/stock_formal_boundary_manifest_source_guard.rs`
- Modify: `docs/governance/contract_registry.md`
- Modify: `docs/governance/decision_log.md`
- Modify: `docs/handoff/CURRENT_STATUS.md`
- Modify: `docs/handoff/HANDOFF_ISSUES.md`

**Step 1: Update source guards and docs**

Add P19B to the frozen public manifest expectation and governance docs. Keep wording explicit that P19B is dry-run-only and does not write runtime facts.

**Step 2: Run guard tests**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_guard'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_guard'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_guard'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
```

Expected: all guard tests pass.

### Task 4: Final Focused Verification And Journal

**Files:**
- Modify: `.trae/CHANGELOG_TASK.md`

**Step 1: Run focused verification**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_final'; cargo test --test security_portfolio_execution_replay_executor_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_final'; cargo check
```

Expected: focused P19B tests and `cargo check` pass.

**Step 2: Append task journal**

Append one entry to `.trae/CHANGELOG_TASK.md` with changed files, reason, remaining gaps, risks, and verification commands.
