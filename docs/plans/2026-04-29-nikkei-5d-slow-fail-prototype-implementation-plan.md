# Nikkei 5D Slow-Fail Prototype Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a dedicated `5D` slow-fail prototype-add mining rule while freezing the current `1D/3D` prototype-add behavior, then compare baseline vs augmented continuation training on the same real validation slice.

**Architecture:** Reuse the current real-failure experiment runner and the current `1D/3D` prototype-add rule. Specialize only the builder branch used when `label_horizon = 5d`, and keep the official replay exports unchanged. Measure only real-validation performance and prioritize `5D balanced_accuracy` plus `5D` negative recall.

**Tech Stack:** Python, pandas, scikit-learn, existing Nikkei replay/continuation scripts, markdown summaries

---

### Task 1: Freeze the 5D-specific builder contract in tests

**Files:**
- Modify: `D:\SM\scripts\test_nikkei_real_failure_event_balance.py`
- Reference: `D:\SM\docs\plans\2026-04-29-nikkei-5d-slow-fail-prototype-design.md`

**Step 1: Write the failing 5D specialization test**

Require:

- `1D/3D` prototype rows still pass as before
- `5D` can mine `resistance_exhaustion_5d`
- `5D` can mine `extended_add_drift_5d`
- `5D` rows expose a dedicated `mined_failure_reason`

**Step 2: Write the failing 5D freeze test**

Require that a row acceptable to the generic `1D/3D` prototype but not to the new `5D` rule is rejected when `label_horizon = 5d`.

**Step 3: Run test to verify it fails**

Run:

```powershell
python D:\SM\scripts\test_nikkei_real_failure_event_balance.py
```

Expected: FAIL because the builder still shares one add-prototype rule across all horizons.

### Task 2: Implement the 5D dedicated builder rule

**Files:**
- Modify: `D:\SM\scripts\build_nikkei_replay_samples.py`
- Test: `D:\SM\scripts\test_nikkei_real_failure_event_balance.py`

**Step 1: Add a horizon-aware add-failure helper**

Implement:

- current prototype-add branch for `1D/3D`
- dedicated slow-fail branch for `5D`

**Step 2: Add dedicated 5D reason tags**

Emit:

- `prototype_add_failure_5d_resistance_exhaustion`
- `prototype_add_failure_5d_extended_drift`

**Step 3: Keep horizon-specific negative retention**

Retain only rows with `continuation_label_<label_horizon> = 0`.

**Step 4: Run tests**

Run:

```powershell
python D:\SM\scripts\test_nikkei_real_failure_event_balance.py
```

Expected: PASS.

### Task 3: Re-run the experiment

**Files:**
- Read: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\04_replay_classifier_full_snapshot\02_replay_labeled_samples.csv`
- Write: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\07_real_failure_event_experiment\*`

**Step 1: Refresh failure pools**

Generate new mined pools for:

- `1d`
- `3d`
- `5d`

**Step 2: Refresh comparisons**

Overwrite:

- `comparison_summary.csv`
- `distribution_summary.csv`
- `baseline_predictions_<horizon>.csv`
- `augmented_predictions_<horizon>.csv`
- `experiment_summary.json`

**Step 3: Spot-check outputs**

Read refreshed comparison and distribution files.

### Task 4: Update summaries and handoff

**Files:**
- Modify: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\REAL_FAILURE_EVENT_SUMMARY_20260429.md`
- Modify: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\CONTINUATION_HEAD_SUMMARY_20260428.md`
- Modify: `D:\SM\docs\handoff\CURRENT_STATUS.md`

**Step 1: Record the 5D-specific rule**

Document:

- why `5D` was separated from `1D/3D`
- what the two `5D` subcontexts are
- what thresholds were used

**Step 2: Record refreshed metrics**

Document:

- mined sample counts
- baseline vs augmented balance-aware metrics
- whether the `5D` specialization improved over the shared prototype-add version

### Task 5: Fresh verification before claiming completion

**Files:**
- Verify all modified files from Tasks 1-4

**Step 1: Run required tests**

Run:

```powershell
python D:\SM\scripts\test_nikkei_real_failure_event_balance.py
python D:\SM\scripts\test_nikkei_continuation_head.py
python D:\SM\scripts\test_nikkei_replay_classifier.py
python D:\SM\scripts\test_run_nikkei_hgb_rf_daily_workflow.py
python C:\Users\wakes\.codex\skills\nikkei-live-journal\scripts\test_upsert_journal.py
```

Expected: PASS.

**Step 2: Spot-check refreshed outputs**

Run:

```powershell
Get-Content D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\07_real_failure_event_experiment\5d\comparison_summary.csv -TotalCount 20
Get-Content D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\07_real_failure_event_experiment\5d\distribution_summary.csv -TotalCount 20
```

Expected: files exist, are non-empty, and show `5D`-specific mined statistics.
