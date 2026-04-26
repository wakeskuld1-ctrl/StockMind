# P19C Execution Replay Commit Preflight Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add P19C as a side-effect-free replay commit preflight boundary that freezes future commit inputs without writing runtime facts.

**Architecture:** P19C consumes a P19B dry-run executor document and the matching P14 request-enrichment bundle. It validates lineage, row matching, structured apply context, preflight-only mode, canonical commit payload hashes, and durable idempotency candidates. It emits a preflight document for a later P19D runtime writer and must not call `security_execution_record`.

**Tech Stack:** Rust, serde, thiserror, existing StockMind CLI dispatcher/catalog, Cargo integration tests.

---

### Risk Synchronization Gate
**Risk subprocess mode:** user-approved-subagent
**Question asked:** What artifact will drift if P19C adds a new public replay commit-preflight boundary without synchronized stock bus, frozen manifest, governance, and runtime-write non-goal evidence?

**Boundary items:**
- `security_portfolio_execution_replay_commit_preflight`
- `SecurityPortfolioExecutionReplayCommitPreflightRequest`
- `SecurityPortfolioExecutionReplayCommitPreflightDocument`
- `SecurityPortfolioExecutionReplayCommitPreflightResult`
- public stock module export
- execution-and-position-management grouped export
- tool catalog entry
- dispatcher route
- frozen public stock-boundary manifest entry
- contract registry and decision log rows

**Must-sync files:**
- `D:\SM\docs\plans\2026-04-26-p19c-execution-replay-commit-preflight-design.md`
- `D:\SM\docs\plans\2026-04-26-p19c-execution-replay-commit-preflight-plan.md`
- `D:\SM\src\ops\security_portfolio_execution_replay_commit_preflight.rs`
- `D:\SM\src\ops\stock.rs`
- `D:\SM\src\ops\stock_execution_and_position_management.rs`
- `D:\SM\src\tools\catalog.rs`
- `D:\SM\src\tools\dispatcher.rs`
- `D:\SM\src\tools\dispatcher\stock_ops.rs`
- `D:\SM\tests\security_portfolio_execution_replay_commit_preflight_cli.rs`
- `D:\SM\tests\stock_formal_boundary_manifest_source_guard.rs`
- `D:\SM\tests\stock_catalog_grouping_source_guard.rs` if the catalog grouping expectation changes
- `D:\SM\tests\stock_dispatcher_grouping_source_guard.rs` if the dispatcher grouping expectation changes
- `D:\SM\docs\governance\contract_registry.md`
- `D:\SM\docs\governance\decision_log.md`
- `D:\SM\docs\handoff\CURRENT_STATUS.md`
- `D:\SM\docs\handoff\HANDOFF_ISSUES.md`
- `D:\SM\.trae\CHANGELOG_TASK.md`

**Must-run checks:**
- P19C RED focused test
- P19C GREEN focused test
- `stock_formal_boundary_manifest_source_guard`
- `stock_catalog_grouping_source_guard`
- `stock_dispatcher_grouping_source_guard`
- `cargo check`
- repository-wide regression after P19C if implementation proceeds and focused verification is green

**Blockers:**
- Do not implement runtime writes in P19C.
- Do not extend P19B to accept `commit`.
- Do not call `security_execution_record`.
- Do not output non-empty `runtime_execution_record_ref`.
- Do not name any P19C planned/preflight ref as a runtime ref.
- Do not claim durable runtime idempotency without a P19D ledger or equivalent persistent contract.
- Do not claim bundle rollback semantics while `security_execution_record` owns its own commit.

---

### Task 1: P19C CLI Red Tests

**Files:**
- Create: `D:\SM\tests\security_portfolio_execution_replay_commit_preflight_cli.rs`

**Step 1: Write the failing tests**

Add tests for:
- catalog includes `security_portfolio_execution_replay_commit_preflight`
- empty P19B no-work document produces `no_commit_work`
- one validated P19B row plus matching P14 ready row produces one `preflight_ready` row
- P19C rejects unsupported `preflight_mode`
- P19C rejects P19B documents whose `execution_mode` is not `dry_run`
- P19C rejects any request or fixture that tries to authorize `execution_mode = "commit"`
- P19C rejects any P19B row with `runtime_execution_record_ref`
- P19C rejects P19B `no_replay_work` status when executor rows are present
- P19C rejects missing P14 row matches
- P19C rejects ambiguous P14 row matches
- P19C rejects matched P14 rows not marked `ready_for_apply`
- P19C rejects duplicate/conflicting commit idempotency keys
- P19C output proves `runtime_write_count == 0` and contains no runtime execution record refs
- source inspection proves P19C does not contain `security_execution_record(`

**Step 2: Run test to verify it fails**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_red'; cargo test --test security_portfolio_execution_replay_commit_preflight_cli -- --nocapture
```

Expected: fail because the P19C route does not exist yet.

### Task 2: P19C Preflight Implementation

**Files:**
- Create: `D:\SM\src\ops\security_portfolio_execution_replay_commit_preflight.rs`

**Step 1: Define request/result/document/row/error structs**

Required structs:
- `SecurityPortfolioExecutionReplayCommitPreflightRequest`
- `SecurityPortfolioExecutionReplayCommitPreflightRow`
- `SecurityPortfolioExecutionReplayCommitPreflightDocument`
- `SecurityPortfolioExecutionReplayCommitPreflightResult`
- `SecurityPortfolioExecutionReplayCommitPreflightError`

Required request fields:
- `portfolio_execution_replay_executor: SecurityPortfolioExecutionReplayExecutorDocument`
- `portfolio_execution_request_enrichment: SecurityPortfolioExecutionRequestEnrichmentDocument`
- `preflight_mode: String`
- `created_at: String`

**Step 2: Implement preflight validation only**

Core behavior:
- accept only `preflight_mode = "commit_preflight_only"`
- validate P19B document type, contract version, `execution_mode = "dry_run"`, and `runtime_write_count = 0`
- reject any P19B row with `runtime_execution_record_ref`
- reject any commit-mode authorization field or request shape
- reject `dry_run_status = "no_replay_work"` if P19B contains executor rows
- validate P19B/P14 lineage refs match
- validate P14 bundle is not blocked and summary counts are coherent
- for each P19B validated row, find exactly one matching P14 enriched row
- require matched P14 row `enrichment_status = "ready_for_apply"`
- require structured apply context fields needed for a future `SecurityExecutionRecordRequest`
- build a canonical commit payload preview without calling `security_execution_record`
- derive `commit_idempotency_key` and `canonical_commit_payload_hash`
- inherit and verify the P19B row idempotency key as source evidence without treating it as durable runtime idempotency
- reject duplicate keys and same-key/different-hash conflicts
- emit `runtime_write_count = 0`

**Step 3: Keep runtime-write red lines explicit in code comments**

Near the main function, add a concise English comment explaining:
- reason: P19C freezes commit inputs before runtime writes
- purpose: prevent commit-mode semantics from leaking into dry-run executor
- boundary: no `security_execution_record`, no runtime ledger, no broker replay

### Task 3: Public Tool Wiring

**Files:**
- Modify: `D:\SM\src\ops\stock.rs`
- Modify: `D:\SM\src\ops\stock_execution_and_position_management.rs`
- Modify: `D:\SM\src\tools\catalog.rs`
- Modify: `D:\SM\src\tools\dispatcher.rs`
- Modify: `D:\SM\src\tools\dispatcher\stock_ops.rs`

**Step 1: Wire P19C after P19B**

Add:
- stock public module entry immediately after P19B
- grouped execution-and-position-management export immediately after P19B
- tool catalog entry immediately after P19B
- dispatcher branch
- stock_ops import and dispatch function

**Step 2: Run focused P19C test**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_green'; cargo test --test security_portfolio_execution_replay_commit_preflight_cli -- --nocapture
```

Expected: all P19C focused tests pass.

### Task 4: Boundary And Governance Sync

**Files:**
- Modify: `D:\SM\tests\stock_formal_boundary_manifest_source_guard.rs`
- Modify: `D:\SM\docs\governance\contract_registry.md`
- Modify: `D:\SM\docs\governance\decision_log.md`
- Modify: `D:\SM\docs\handoff\CURRENT_STATUS.md`
- Modify: `D:\SM\docs\handoff\HANDOFF_ISSUES.md`

**Step 1: Sync frozen manifest and governance docs**

Add P19C to:
- frozen public stock-boundary manifest
- contract registry
- decision log
- current status and handoff issues

Wording must state:
- P19C is commit-preflight-only
- P19C does not write runtime facts
- P19C does not call `security_execution_record`
- P19D remains the earliest possible runtime-write phase

**Step 2: Run guard tests**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_guard'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_guard'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_guard'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
```

Expected: all guard tests pass.

### Task 5: Final Focused Verification And Journal

**Files:**
- Modify: `D:\SM\.trae\CHANGELOG_TASK.md`

**Step 1: Run final focused verification**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_final'; cargo test --test security_portfolio_execution_replay_commit_preflight_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_final'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_final'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_final'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_final'; cargo check
```

Expected: focused P19C tests, guards, and `cargo check` pass.

**Step 2: Append task journal**

Append one entry to `D:\SM\.trae\CHANGELOG_TASK.md` with changed files, reason, remaining gaps, risks, and verification commands.

**Step 3: Decide on repository-wide regression**

If focused verification passes, ask for or run the approved repository-wide regression before moving to P19D:

```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p19c'; cargo test -- --nocapture
```

Expected: repository-wide regression completes with exit code 0 before any P19D design starts.
