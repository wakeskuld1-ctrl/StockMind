# Nikkei Official Turnover Import Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a receiver/importer for Nikkei official Total Trading Value files so the system can build `NK225_TURNOVER.NIKKEI` after manual official export.

**Architecture:** Add one stock data-pipeline operation that parses an official turnover file, aligns dates to existing `NK225.IDX` price rows, and imports rows through `StockHistoryStore`. Wire it through the stock catalog/dispatcher and verify with CLI tests plus manifest checks.

**Tech Stack:** Rust, existing CLI dispatcher, existing `StockHistoryStore`, existing `security_volume_source_manifest`.

---

### Task 1: Red CLI Test

**Files:**
- Create: `D:\SM\tests\security_nikkei_turnover_import_cli.rs`

**Steps:**
- Write a fixture file with columns `Date,Total Trading Value(Tril.Yen)`.
- Seed `NK225.IDX` price rows for the same dates.
- Call `security_nikkei_turnover_import`.
- Assert import result and then call `security_volume_source_manifest`.
- Run:
  ```powershell
  $env:CARGO_TARGET_DIR='D:\SM\target_nikkei_turnover_import_red'
  cargo test --test security_nikkei_turnover_import_cli -- --nocapture
  ```
- Expected: fail because the tool is not registered.

### Task 2: Import Operation

**Files:**
- Create: `D:\SM\src\ops\security_nikkei_turnover_import.rs`

**Steps:**
- Define request/result structs.
- Parse CSV/TSV/copy text.
- Find date and total-trading-value columns by normalized header.
- Parse dates in `YYYY-MM-DD`, `Apr/01/2026`, and `Apr 01 2026`.
- Load matching price rows from `StockHistoryStore`.
- Build proxy rows with price OHLC copied from the price source and `volume` set to scaled turnover.
- Import via `StockHistoryStore::import_rows`.

### Task 3: Public Tool Wiring

**Files:**
- Modify: `D:\SM\src\ops\stock.rs`
- Modify: `D:\SM\src\ops\stock_data_pipeline.rs`
- Modify: `D:\SM\src\tools\catalog.rs`
- Modify: `D:\SM\src\tools\dispatcher.rs`
- Modify: `D:\SM\src\tools\dispatcher\stock_ops.rs`

**Steps:**
- Expose the module under the formal stock boundary.
- Re-export under stock data pipeline.
- Add catalog and dispatcher entries.

### Task 4: Verification

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_turnover_import_green'
cargo test --test security_nikkei_turnover_import_cli -- --nocapture
```

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_turnover_import_check'
cargo check
```

Run relevant guards:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_turnover_import_guards'
cargo test --test stock_catalog_grouping_source_guard --test stock_dispatcher_grouping_source_guard --test stock_formal_boundary_manifest_source_guard -- --nocapture
```
