# Nikkei Volatility-Normalized Mean Reversion Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the current raw `close_vs_sma20` fixed-band Nikkei mean-reversion bucket with a volatility-normalized bucket based on ATR14, so the middle bucket stays small and the weak-direction buckets carry the main information density.

**Architecture:** Keep the user-facing training feature name `mean_reversion_deviation_20d`, but change its semantics from "raw MA20 percentage deviation" to "MA20 deviation measured in ATR14 units." Add one raw numeric helper field for replay/explanation, then derive the categorical bucket from that normalized value in evidence, snapshot, and runtime scorecard. Use TDD to lock the new bucket math before touching production logic.

**Tech Stack:** Rust, Cargo tests, governed snapshot/scorecard/training contracts, local SQLite-backed Nikkei history fixtures.

---

### Task 1: Freeze the New Bucket Contract

**Files:**
- Modify: `E:\SM\src\ops\security_decision_evidence_bundle.rs`
- Modify: `E:\SM\tests\security_feature_snapshot_cli.rs`

**Step 1: Write the failing test**

Add a new helper contract test in `security_decision_evidence_bundle.rs` that proves:

```rust
assert_eq!(
    derive_mean_reversion_normalized_distance_20d(0.026, 0.01),
    2.6
);
assert_eq!(
    derive_mean_reversion_deviation_bucket_20d(-2.61),
    "strong_down"
);
assert_eq!(
    derive_mean_reversion_deviation_bucket_20d(-0.16),
    "weak_down"
);
assert_eq!(
    derive_mean_reversion_deviation_bucket_20d(0.0),
    "neutral"
);
assert_eq!(
    derive_mean_reversion_deviation_bucket_20d(0.16),
    "weak_up"
);
assert_eq!(
    derive_mean_reversion_deviation_bucket_20d(2.61),
    "strong_up"
);
```

Update the snapshot CLI test so the expected bucket is computed from the normalized value instead of raw `close_vs_sma20`.

**Step 2: Run test to verify it fails**

Run:

```powershell
$env:CARGO_TARGET_DIR='E:\SM\target_task2'; cargo test --lib derive_mean_reversion_deviation_bucket_20d_uses_atr_normalized_bands -- --nocapture
```

Expected: FAIL because the production code still uses raw percentage bands and the new helper does not exist yet.

### Task 2: Implement the Normalized Metric

**Files:**
- Modify: `E:\SM\src\ops\security_decision_evidence_bundle.rs`
- Modify: `E:\SM\src\ops\security_feature_snapshot.rs`
- Modify: `E:\SM\src\ops\security_scorecard.rs`

**Step 1: Write minimal implementation**

Add one numeric helper:

```rust
pub fn derive_mean_reversion_normalized_distance_20d(
    close_vs_sma20: f64,
    atr_ratio_14: f64,
) -> f64 {
    if atr_ratio_14.abs() <= f64::EPSILON {
        0.0
    } else {
        close_vs_sma20 / atr_ratio_14
    }
}
```

Change the bucket contract to:

```rust
if normalized_value < -2.6 {
    "strong_down"
} else if normalized_value < -0.15 {
    "weak_down"
} else if normalized_value <= 0.15 {
    "neutral"
} else if normalized_value <= 2.6 {
    "weak_up"
} else {
    "strong_up"
}
```

Expose the raw normalized field, then derive `mean_reversion_deviation_20d` from that normalized field in both snapshot and scorecard.

**Step 2: Run focused tests**

Run:

```powershell
$env:CARGO_TARGET_DIR='E:\SM\target_task2'; cargo test --lib derive_mean_reversion_deviation_bucket_20d_uses_atr_normalized_bands -- --nocapture
$env:CARGO_TARGET_DIR='E:\SM\target_task2'; cargo test --test security_feature_snapshot_cli security_feature_snapshot_freezes_raw_and_group_features_with_hash -- --nocapture
```

Expected: PASS.

### Task 3: Lock Training Contract Compatibility

**Files:**
- Modify: `E:\SM\tests\security_scorecard_cli.rs`
- Modify: `E:\SM\tests\security_scorecard_training_cli.rs`

**Step 1: Write/adjust tests**

Assert the runtime scorecard keeps exposing:

```rust
raw_snapshot["mean_reversion_deviation_20d"]
```

and the training feature list still contains:

```rust
"mean_reversion_deviation_20d"
```

without reintroducing `mean_reversion_state_20d` as the primary training field.

**Step 2: Run tests**

Run:

```powershell
$env:CARGO_TARGET_DIR='E:\SM\target_task2'; cargo test --test security_scorecard_cli -- --nocapture
$env:CARGO_TARGET_DIR='E:\SM\target_task2'; cargo test --test security_scorecard_training_cli -- --nocapture
```

Expected: PASS.

### Task 4: Verification and Handoff

**Files:**
- Modify: `E:\SM\CHANGELOG_TASK.MD`

**Step 1: Record verification**

Append a task-journal entry with:
- reason for switching from raw percentage to ATR-normalized deviation
- selected thresholds `0.15 / 2.6`
- tests executed
- remaining risk that score quality still needs a real retrain

**Step 2: Final validation**

Run the minimal full regression set:

```powershell
$env:CARGO_TARGET_DIR='E:\SM\target_task2'; cargo test --lib derive_mean_reversion_deviation_bucket_20d_uses_atr_normalized_bands -- --nocapture
$env:CARGO_TARGET_DIR='E:\SM\target_task2'; cargo test --test security_feature_snapshot_cli -- --nocapture
$env:CARGO_TARGET_DIR='E:\SM\target_task2'; cargo test --test security_scorecard_cli -- --nocapture
$env:CARGO_TARGET_DIR='E:\SM\target_task2'; cargo test --test security_scorecard_training_cli -- --nocapture
```

Expected: PASS.
