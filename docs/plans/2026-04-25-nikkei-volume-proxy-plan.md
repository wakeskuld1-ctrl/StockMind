# Nikkei Volume Proxy Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a separate Nikkei volume proxy path so weekly training keeps FRED prices while using non-zero proxy volume for volume features.

**Architecture:** Extend `SecurityScorecardTrainingRequest` with `volume_proxy_symbol`, load proxy rows beside spot/futures rows in weekly collection, and pass them to the weekly feature builder. The feature builder chooses proxy volume first, futures volume second, and spot volume last.

**Tech Stack:** Rust, existing `stock_history.db`, existing `security_scorecard_training` CLI, existing `stock_price_history` import tool.

---

### Task 1: Request Contract And Red Test

**Files:**
- Modify: `D:\SM\src\ops\security_scorecard_training.rs`
- Modify: `D:\SM\tests\security_scorecard_training_cli.rs`

**Step 1: Write failing tests**

Add a test that:
- imports FRED-like spot rows with `volume=0` as `NK225.IDX`
- imports proxy rows with varying `volume` as `NK225_VOL.PROXY`
- runs `security_scorecard_training` with `volume_proxy_symbol="NK225_VOL.PROXY"` and no `futures_symbol`
- asserts `weekly_volume_ratio_4w` has more than one distinct observed value
- asserts futures feature names are absent

**Step 2: Run red test**

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_volume_proxy_red'
cargo test security_scorecard_training_nikkei_weekly_uses_volume_proxy_without_futures_features --test security_scorecard_training_cli -- --nocapture
```

Expected: compile or assertion failure because `volume_proxy_symbol` is not implemented.

### Task 2: Minimal Implementation

**Files:**
- Modify: `D:\SM\src\ops\security_scorecard_training.rs`

**Step 1: Add request field**

Add:
```rust
#[serde(default)]
pub volume_proxy_symbol: Option<String>,
```

**Step 2: Load proxy rows in weekly collectors**

In both weekly collection paths, load `volume_proxy_symbol` rows when provided.

**Step 3: Extend weekly feature builder**

Change the internal builder to accept:
```rust
spot_rows: &[StockHistoryRow],
futures_rows: Option<&[StockHistoryRow]>,
volume_proxy_rows: Option<&[StockHistoryRow]>,
```

Use proxy volume before futures volume before spot volume.

### Task 3: Verification

**Files:**
- Test: `D:\SM\tests\security_scorecard_training_cli.rs`

Run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_volume_proxy_green'
cargo test security_scorecard_training_nikkei_weekly_uses_volume_proxy_without_futures_features --test security_scorecard_training_cli -- --nocapture
```

Then run:
```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_volume_proxy_green_full'
cargo test weekly_ --test security_scorecard_training_cli -- --nocapture
```

### Task 4: Data Import And Real Rerun

**Files/Data:**
- Import proxy CSV into `D:\.stockmind_runtime\nikkei_10y_market_20260425\stock_history.db`
- Symbol: `NK225_VOL.PROXY`

**Step 1: Prefer local/provided OHLCV source**

Use an existing local `^N225` OHLCV CSV if full history cannot be downloaded.

**Step 2: Import through governed tool**

Run `import_stock_price_history` with `symbol=NK225_VOL.PROXY`.

**Step 3: Rerun weekly training**

Run current weekly `direction_head` with `volume_proxy_symbol="NK225_VOL.PROXY"`.

**Step 4: Compare**

Confirm `weekly_volume_*` no longer appear under zero-variance features when coverage is sufficient.
