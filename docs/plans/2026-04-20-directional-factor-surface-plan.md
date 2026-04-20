# Directional Factor Surface Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Freeze a governed up/down/neutral factor surface for Nikkei index modeling before any next retraining pass.

**Architecture:** Keep the current training contract unchanged for now, and add a new directional feature layer on top of the existing evidence-bundle raw feature seed. This lets us separate field semantics from trainer migration, so we can review factor direction, keep backward compatibility, and only switch training after the new directional targets are approved.

**Tech Stack:** Rust, serde_json raw feature seed, grouped feature snapshot, cargo test

---

### Task 1: Freeze directional helper rules

**Files:**
- Modify: `E:\SM\src\ops\security_decision_evidence_bundle.rs`
- Test: `E:\SM\src\ops\security_decision_evidence_bundle.rs`

**Step 1: Write the failing test**

- Add unit tests for:
  - breakout direction vs stage
  - trend direction-strength binding
  - directional volume confirmation
  - alignment direction vs consistency
  - market direction regime vs volatility regime
  - RSI/MACD fixed directional semantics

**Step 2: Run test to verify it fails**

Run:

```powershell
$env:CARGO_TARGET_DIR='E:\SM\target_directional_factors'; cargo test --lib derive_breakout_direction_and_stage_split_up_down_and_structure -- --nocapture
```

Expected: fail because the new directional helper functions do not exist yet.

**Step 3: Write minimal implementation**

- Add helper functions that freeze:
  - `breakout_direction`
  - `breakout_stage`
  - `trend_direction_strength`
  - `volume_direction_state`
  - `alignment_direction`
  - `alignment_consistency`
  - `market_direction_regime`
  - `market_volatility_regime`
  - `rsi_direction_state`
  - `rsi_extreme_state`
  - `macd_histogram_direction`

**Step 4: Run test to verify it passes**

Run:

```powershell
$env:CARGO_TARGET_DIR='E:\SM\target_directional_factors'; cargo test --lib security_decision_evidence_bundle -- --nocapture
```

Expected: all evidence-bundle unit tests pass.

### Task 2: Project directional fields into the canonical feature seed

**Files:**
- Modify: `E:\SM\src\ops\security_decision_evidence_bundle.rs`
- Modify: `E:\SM\src\ops\security_feature_snapshot.rs`

**Step 1: Add raw directional fields**

- Project the new directional fields into `build_evidence_bundle_feature_seed(...)`.
- Also backfill already-computed but previously unprojected directional source signals:
  - `divergence_signal`
  - `timing_signal`
  - `bollinger_midline_signal`

**Step 2: Add grouped directional view**

- Add a grouped snapshot section for directional fields so later training selection can audit one surface without reparsing raw fields.

**Step 3: Verify compile and regression safety**

Run:

```powershell
$env:CARGO_TARGET_DIR='E:\SM\target_directional_factors'; cargo test --lib -- --nocapture
```

Expected: all library tests pass, confirming the new feature surface is additive and non-breaking.

### Task 3: Freeze old-to-new mapping for the next training cutover

**Files:**
- Create: `E:\SM\docs\plans\2026-04-20-directional-factor-surface-plan.md`
- Modify: `E:\SM\CHANGELOG_TASK.MD`

**Step 1: Record the field mapping**

- Document which fields are:
  - direction factors
  - condition factors
  - already available upstream but not yet in trainer selection

**Step 2: Record non-goals**

- Do not retrain yet
- Do not switch `training_feature_configs()` yet
- Do not replace `positive_return_10d` with dual-head targets yet

**Step 3: Define acceptance for the next session**

- The next training session may start only after:
  - directional factor inventory is frozen
  - trainer feature selection is updated
  - up/down targets are approved

### Current frozen mapping

**Direction factors already projected now**

- `trend_direction_state`
- `trend_direction_strength`
- `volume_direction_state`
- `breakout_direction`
- `breakout_stage`
- `alignment_direction`
- `alignment_consistency`
- `market_direction_regime`
- `market_volatility_regime`
- `flow_direction_state`
- `mean_reversion_direction_state`
- `range_position_direction_state`
- `bollinger_position_direction_state`
- `bollinger_midline_direction_state`
- `rsrs_direction_state`
- `divergence_direction_state`
- `timing_direction_state`
- `rsi_direction_state`
- `rsi_extreme_state`
- `macd_histogram_direction`

**Condition factors intentionally left non-directional**

- `atr_ratio_14`
- `bollinger_bandwidth_signal`
- `volatility_state`
- `risk_note_count`
- `data_gap_count`
- `announcement_count`
- `hard_risk_score`
- `negative_attention_score`
- `positive_support_score`
- `event_net_impact_score`

**Next cutover contract**

- Migrate trainer feature selection to the directional layer
- Add dual targets for up/down training
- Re-run Nikkei decade training only after both of the above are frozen
