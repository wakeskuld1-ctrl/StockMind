# Nikkei Volume Source Manifest Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a governed JSON manifest that lists Nikkei volume sources and their training readiness.

**Architecture:** Reuse `StockHistoryStore` as the single data source. Add one store summary query for volume/source statistics, one stock data-pipeline operation, and normal catalog/dispatcher wiring. Keep the manifest read-only and do not alter scorecard training decisions.

**Tech Stack:** Rust, `rusqlite`, existing CLI dispatcher, existing `stock_history.db` test fixtures.

---

### Task 1: Red CLI Test

**Files:**
- Create: `D:\SM\tests\security_volume_source_manifest_cli.rs`

**Step 1: Write the failing test**
- Import zero-volume `NK225.IDX` rows.
- Import shorter non-zero `NK225_VOL.PROXY` rows.
- Call `security_volume_source_manifest`.
- Assert the tool exists in catalog.
- Assert `NK225.IDX` is `no_volume`.
- Assert `NK225_VOL.PROXY` is `usable_short_proxy`.

**Step 2: Run red test**

```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_volume_manifest_red'
cargo test --test security_volume_source_manifest_cli -- --nocapture
```

Expected: fail because the tool is not registered yet.

### Task 2: Store Summary Query

**Files:**
- Modify: `D:\SM\src\runtime\stock_history_store.rs`

**Step 1: Add `StockHistoryVolumeSourceSummary`**
- Include source names, row count, non-zero/zero volume counts, volume min/max, and date range.

**Step 2: Add `load_volume_source_summary`**
- Query `stock_price_history` by symbol and optional `as_of_date`.
- Return `None` when no rows exist.

### Task 3: Manifest Operation

**Files:**
- Create: `D:\SM\src\ops\security_volume_source_manifest.rs`
- Modify: `D:\SM\src\ops\stock.rs`
- Modify: `D:\SM\src\ops\stock_data_pipeline.rs`

**Step 1: Add request/result structs**
- `SecurityVolumeSourceManifestRequest`
- `SecurityVolumeSourceManifestResult`
- `SecurityVolumeSourceStatus`
- `SecurityVolumeSourceManifestSummary`

**Step 2: Implement status rules**
- `missing_history`
- `no_volume`
- `usable_short_proxy`
- `train_ready_volume_proxy`

### Task 4: Public Tool Wiring

**Files:**
- Modify: `D:\SM\src\tools\catalog.rs`
- Modify: `D:\SM\src\tools\dispatcher.rs`
- Modify: `D:\SM\src\tools\dispatcher\stock_ops.rs`

**Step 1: Add catalog entry**
- Add `security_volume_source_manifest` to the data-pipeline group.

**Step 2: Add dispatcher entry**
- Parse request and return `ToolResponse::ok(json!(result))`.

### Task 5: Green Verification

Run:

```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_volume_manifest_green'
cargo test --test security_volume_source_manifest_cli -- --nocapture
```

Expected: `2 passed; 0 failed`.

Run:

```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_volume_manifest_check'
cargo check
```

Expected: success.
