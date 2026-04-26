# P20B Lifecycle Closeout Evidence Package Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add P20B as a read-only lifecycle closeout evidence package that consumes P20A readiness and verifies closed execution-record evidence without writing runtime, post-trade, archive, position, or lifecycle facts.

**Architecture:** P20B validates a P20A `SecurityPortfolioExecutionLifecycleCloseoutReadinessDocument`, preserves P20A blocked rows as evidence blockers, and point-reads runtime execution records only for P20A eligible rows. A row becomes `evidence_ready_for_closeout_archive_preflight` only when runtime closed-position fields and replay metadata match; P20B remains a pre-archive evidence package, not a lifecycle writer.

**Tech Stack:** Rust, serde, thiserror, existing StockMind CLI dispatcher/catalog, P20A readiness contracts, execution-store point-read facade, Cargo integration tests and source guards.

---

### Risk Synchronization Gate
**Risk subprocess mode:** attempted `user-approved-subagent`; blocked by external quota error; fallback `inline-fresh-pass`.

**Question asked:** What artifact will drift if P20B exposes closeout evidence after P20A, and what semantic boundary will be crossed if evidence readiness is mistaken for lifecycle closure or archive production?

**Boundary items:**
- `security_portfolio_execution_lifecycle_closeout_evidence_package`
- `SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageRequest`
- `SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageRow`
- `SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageDocument`
- `SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageResult`
- public stock module export
- execution-and-position-management grouped export
- tool catalog entry
- dispatcher route
- frozen stock-boundary manifest entry
- contract registry and decision log rows
- P20B source guard proving read-only evidence packaging and no lifecycle/archive claim

**Must-sync files:**
- `D:\SM\docs\plans\2026-04-26-p20b-lifecycle-closeout-evidence-package-design.md`
- `D:\SM\docs\plans\2026-04-26-p20b-lifecycle-closeout-evidence-package-plan.md`
- `D:\SM\src\ops\security_portfolio_execution_lifecycle_closeout_evidence_package.rs`
- `D:\SM\src\ops\stock.rs`
- `D:\SM\src\ops\stock_execution_and_position_management.rs`
- `D:\SM\src\tools\catalog.rs`
- `D:\SM\src\tools\dispatcher.rs`
- `D:\SM\src\tools\dispatcher\stock_ops.rs`
- `D:\SM\tests\security_portfolio_execution_lifecycle_closeout_evidence_package_cli.rs`
- `D:\SM\tests\stock_formal_boundary_manifest_source_guard.rs`
- `D:\SM\docs\governance\contract_registry.md`
- `D:\SM\docs\governance\decision_log.md`
- `D:\SM\docs\handoff\CURRENT_STATUS.md`
- `D:\SM\docs\handoff\HANDOFF_ISSUES.md`
- `D:\SM\.trae\CHANGELOG_TASK.md`

**Must-run checks:**
- P20B RED focused test
- P20B GREEN focused test
- adjacent P20A focused test
- P20B source guard proving no write/archive/post-trade/lifecycle paths
- `stock_formal_boundary_manifest_source_guard`
- `stock_catalog_grouping_source_guard`
- `stock_dispatcher_grouping_source_guard`
- `cargo check`
- repository-wide `cargo test -- --nocapture`
- `git diff --check -- <touched tracked files>` plus trailing-whitespace check for new untracked files

**Blockers resolved into hard constraints:**
- P20B must not write runtime facts.
- P20B must not call `security_execution_record`.
- P20B must not call `security_post_trade_review`.
- P20B must not call or depend on `security_closed_position_archive`.
- P20B must not call SQLite `execute`, SQLite `execute_batch`, `open_session`, `upsert_execution_record`, repository upsert functions, or runtime mutation APIs.
- P20B may point-read runtime execution records only by the P20A row `target_execution_record_ref`.
- P20B must not treat P20A `eligible_for_closeout_preflight` as closed-position evidence.
- P20B must not treat P19E `verified` or P19D replay metadata as broker-fill evidence.
- P20B must preserve P19D/P19E/P20A non-atomic partial truth and row-level blockers.
- Current local evidence does not show an available `security_closed_position_archive` implementation or route; P20B may only name archive production as a future boundary context.

---

### Task 1: P20B CLI RED Tests

**Files:**
- Create: `D:\SM\tests\security_portfolio_execution_lifecycle_closeout_evidence_package_cli.rs`

**Step 1: Write failing tests**

Add tests for:
- catalog includes `security_portfolio_execution_lifecycle_closeout_evidence_package`
- wrong P20A document type hard-fails
- wrong P20A contract version hard-fails
- P20A `runtime_write_count != 0` hard-fails
- missing P20A source P19E ref hard-fails
- missing P20A source P19D ref hard-fails
- missing P20A source P19C ref hard-fails
- missing P20A source non-atomicity notice hard-fails
- P20A no-row document returns `no_closeout_evidence_candidates` with `runtime_read_count == 0` and `runtime_write_count == 0`
- P20A blocked row maps to `blocked_p20a_not_eligible` without runtime read
- P20A eligible row with missing runtime record maps to `blocked_missing_runtime_record`
- P20A eligible row with open runtime record maps to `blocked_runtime_record_not_closed`
- P20A eligible row with closed runtime record but empty exit date maps to `blocked_missing_exit_evidence`
- P20A eligible row with closed runtime record but zero exit price maps to `blocked_missing_exit_evidence`
- P20A eligible row with closed runtime record but `exit_reason == "position_still_open"` maps to `blocked_missing_exit_evidence`
- P20A eligible row with replay metadata mismatch maps to `blocked_replay_metadata_mismatch`
- P20A eligible row with account or symbol mismatch maps to `blocked_account_or_symbol_mismatch`
- P20A eligible row with closed runtime record and matching replay metadata maps to `evidence_ready_for_closeout_archive_preflight`
- mixed evidence-ready and blocked rows return `partial_closeout_evidence_ready`
- source guard confirms P20B does not call `security_execution_record`
- source guard confirms P20B does not call `security_post_trade_review`
- source guard confirms P20B does not call or depend on `security_closed_position_archive`
- source guard confirms P20B does not call direct runtime write APIs, including SQLite `.execute`, SQLite `.execute_batch`, `open_session`, `upsert_execution_record`, repository upsert functions, or store mutation APIs
- source guard confirms P20B emits `runtime_write_count = 0` and does not claim lifecycle closure

**Step 2: Build runtime fixtures in the test**

Use existing test helper patterns from adjacent runtime CLI tests to create an isolated execution store fixture. Insert runtime execution records through existing fixture setup helpers or an approved writer setup path inside the test fixture only; production P20B source must remain read-only.

The fixture should create at least:
- one closed matching runtime record
- one open runtime record
- one closed record with missing exit date
- one closed record with zero exit price
- one closed record with `position_still_open` exit reason
- one closed record with replay metadata mismatch
- one closed record with account mismatch
- one closed record with symbol mismatch

**Step 3: Run test to verify RED**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_red'; cargo test --test security_portfolio_execution_lifecycle_closeout_evidence_package_cli -- --nocapture
```

Expected: fail because the P20B route and module do not exist.

### Task 2: P20B Implementation

**Files:**
- Create: `D:\SM\src\ops\security_portfolio_execution_lifecycle_closeout_evidence_package.rs`

**Step 1: Define contracts**

Required structs:
- `SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageRequest`
- `SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageRow`
- `SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageDocument`
- `SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageResult`
- `SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageError`

Required request fields:
- `portfolio_execution_lifecycle_closeout_readiness: SecurityPortfolioExecutionLifecycleCloseoutReadinessDocument`
- existing runtime store configuration fields following the P19E read-only request pattern
- optional `created_at`

**Step 2: Implement P20A identity validation**

Validate:
- `document_type == "security_portfolio_execution_lifecycle_closeout_readiness"`
- `contract_version == "security_portfolio_execution_lifecycle_closeout_readiness.v1"`
- `runtime_write_count == 0`
- source P19E ref is present
- source P19D ref is present
- source P19C ref is present
- source non-atomicity notice is present

**Step 3: Implement eligible-row evidence validation**

For rows with P20A status `eligible_for_closeout_preflight`, validate:
- `closeout_preflight_eligible == true`
- target execution record ref is present
- commit idempotency key is present
- canonical commit payload hash is present
- source P19C ref is present
- runtime replay idempotency key is present
- runtime replay payload hash is present
- runtime replay source P19C ref is present

Hard-fail the request if an eligible row lacks required machine-readable evidence.

**Step 4: Implement read-only runtime evidence mapping**

For each P20A row:
- if readiness status is not eligible, emit `blocked_p20a_not_eligible` and do not read runtime
- if readiness status is unknown but marked ineligible, emit `blocked_unknown_p20a_readiness_status` and do not read runtime
- if eligible, point-read the runtime execution record by `target_execution_record_ref`
- if no record exists, emit `blocked_missing_runtime_record`
- if `runtime.execution_record_id != target_execution_record_ref`, emit `blocked_runtime_record_identity_mismatch`
- if account or symbol mismatches, emit `blocked_account_or_symbol_mismatch`
- if replay metadata mismatches, emit `blocked_replay_metadata_mismatch`
- if `position_state != "closed"`, emit `blocked_runtime_record_not_closed`
- if exit date is empty, exit price is not positive, or exit reason is empty/`position_still_open`, emit `blocked_missing_exit_evidence`
- otherwise emit `evidence_ready_for_closeout_archive_preflight`

**Step 5: Emit evidence package document**

Include:
- document identity and contract version
- source P20A ref
- source P19E ref
- source P19D ref
- source P19C ref
- preserved source non-atomicity notice
- `runtime_read_count`
- `runtime_write_count = 0`
- row evidence statuses
- blockers
- counts
- aggregate evidence status
- summary text that says evidence readiness is not lifecycle closure and not archive production

Aggregate status rules:
- zero rows -> `no_closeout_evidence_candidates`
- evidence-ready rows only -> `closeout_evidence_ready`
- evidence-ready and blocked rows -> `partial_closeout_evidence_ready`
- blocked rows only -> `blocked`

### Task 3: Public Boundary Wiring

**Files:**
- Modify: `D:\SM\src\ops\stock.rs`
- Modify: `D:\SM\src\ops\stock_execution_and_position_management.rs`
- Modify: `D:\SM\src\tools\catalog.rs`
- Modify: `D:\SM\src\tools\dispatcher.rs`
- Modify: `D:\SM\src\tools\dispatcher\stock_ops.rs`

**Step 1: Wire P20B after P20A**

Add module/export/catalog/dispatcher route immediately after P20A.

**Step 2: Run focused GREEN**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_green'; cargo test --test security_portfolio_execution_lifecycle_closeout_evidence_package_cli -- --nocapture
```

Expected: focused P20B tests pass.

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
$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_guard'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
```

Expected: fail if P20B is exposed in `stock.rs` but absent from the frozen public manifest.

**Step 2: Sync frozen manifest and governance docs**

Wording must say:
- P20B is a read-only lifecycle closeout evidence package
- P20B consumes P20A readiness truth
- P20B point-reads target runtime execution records only for P20A eligible rows
- P20B verifies `position_state == "closed"`, exit evidence, replay metadata, account, and symbol
- P20B writes no runtime facts
- P20B does not call `security_execution_record`
- P20B does not call `security_post_trade_review`
- P20B does not call or depend on `security_closed_position_archive`
- P20B is not broker execution, broker-fill replay, position materialization, archive production, or lifecycle closure

**Step 3: Run guard tests**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_guard'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_guard'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_guard'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
```

Expected: all guard tests pass.

### Task 5: Adjacent Regression And Source Guard Closeout

**Files:**
- Modify if needed: `D:\SM\tests\security_portfolio_execution_lifecycle_closeout_evidence_package_cli.rs`

**Step 1: Run adjacent P20A tests**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_adjacent'; cargo test --test security_portfolio_execution_lifecycle_closeout_readiness_cli -- --nocapture
```

Expected: P20A remains green and its side-effect-free readiness semantics are unchanged.

**Step 2: Run P20B source guard**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_adjacent'; cargo test --test security_portfolio_execution_lifecycle_closeout_evidence_package_cli security_portfolio_execution_lifecycle_closeout_evidence_package_source_guard_is_read_only -- --nocapture
```

Expected: source guard passes and proves P20B has no forbidden write/archive/post-trade/lifecycle paths.

### Task 6: Final Verification

**Files:**
- All P20B touched files

**Step 1: Run final focused and adjacent tests**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_final'; cargo test --test security_portfolio_execution_lifecycle_closeout_evidence_package_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_final'; cargo test --test security_portfolio_execution_lifecycle_closeout_readiness_cli -- --nocapture
```

Expected:
- P20B focused tests pass
- P20A adjacent tests pass

**Step 2: Run final boundary guards**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_final'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_final'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_final'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
```

Expected:
- formal boundary guard passes
- catalog grouping guard passes
- dispatcher grouping guard passes

**Step 3: Run cargo check**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_final'; cargo check
```

Expected: `cargo check` exits 0.

### Task 7: Task Journal And Repository Regression

**Files:**
- Modify: `D:\SM\.trae\CHANGELOG_TASK.md`

**Step 1: Append task journal**

Append a new 2026-04-26 entry summarizing:
- P20B new module and tests
- public boundary/catalog/dispatcher/governance sync
- P20B read-only evidence package semantics
- verification commands and outcomes
- remaining risk that future archive/lifecycle writer is still undesigned

Do not rewrite historical entries.

**Step 2: Run repository-wide regression**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p20b'; cargo test -- --nocapture
```

Expected: repository-wide regression exits 0.

**Step 3: Run diff health**

Run:
```powershell
git diff --check -- .trae\CHANGELOG_TASK.md docs\governance\contract_registry.md docs\governance\decision_log.md docs\handoff\CURRENT_STATUS.md docs\handoff\HANDOFF_ISSUES.md src\ops\stock.rs src\ops\stock_execution_and_position_management.rs src\tools\catalog.rs src\tools\dispatcher.rs src\tools\dispatcher\stock_ops.rs tests\stock_formal_boundary_manifest_source_guard.rs
```

For new untracked files, also check trailing whitespace with:
```powershell
$files = @('src\ops\security_portfolio_execution_lifecycle_closeout_evidence_package.rs','tests\security_portfolio_execution_lifecycle_closeout_evidence_package_cli.rs','docs\plans\2026-04-26-p20b-lifecycle-closeout-evidence-package-plan.md'); $matches = Select-String -Path $files -Pattern '[ \t]$'; if ($matches) { $matches | ForEach-Object { "$($_.Path):$($_.LineNumber): trailing whitespace" }; exit 1 }
```

Expected: no whitespace errors.

## Execution Note
- Do not commit, stage, reset, or clean generated/runtime fixture files unless the user explicitly asks.
- Do not run `rustfmt` on `src\ops\stock.rs`; it can recursively format frozen out-of-line modules. Format only leaf files or use a method that avoids touching frozen legacy modules.
