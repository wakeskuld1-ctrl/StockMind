# Nikkei Prototype Add Failure Mining Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a narrow train-only augmentation lane that mines prototype-driven Nikkei `add` failures aligned to untouched-validation `premature_add` negatives and compares baseline vs augmented continuation training on the same real validation slice.

**Architecture:** Reuse the existing real-failure experiment runner, but tighten the builder to an `add`-only prototype context that matches the dominant untouched-validation negative shape. Keep official replay exports unchanged. Measure only real-validation performance and prioritize `balanced_accuracy` plus negative recall.

**Tech Stack:** Python, pandas, scikit-learn, existing Nikkei replay/continuation scripts, markdown summaries

---

### Task 1: Freeze the prototype-add failure contract in tests

**Files:**
- Modify: `D:\SM\scripts\test_nikkei_real_failure_event_balance.py`
- Reference: `D:\SM\docs\plans\2026-04-29-nikkei-prototype-add-failure-mining-design.md`

**Step 1: Write the failing prototype-add schema test**

Require the builder to emit rows with:

- `sample_source=real_failure_event_mining`
- `source_sample_id`
- `is_real_failure_event`
- `failure_label_horizon`
- `mined_action_direction=add`
- `mined_failure_reason=prototype_add_failure`

**Step 2: Write the failing add-only filter test**

Require:

- `daily_position + add + blank event labels + high position + near resistance` rows can be mined
- `reduce` rows cannot be mined in this round
- rows far from the prototype shape cannot be mined

**Step 3: Run test to verify it fails**

Run:

```powershell
python D:\SM\scripts\test_nikkei_real_failure_event_balance.py
```

Expected: FAIL because the builder still mines mixed event-style rows.

### Task 2: Implement the prototype-add builder

**Files:**
- Modify: `D:\SM\scripts\build_nikkei_replay_samples.py`
- Test: `D:\SM\scripts\test_nikkei_real_failure_event_balance.py`

**Step 1: Add prototype-add context helper**

Implement a helper that uses:

- `signal_direction`
- `signal_family`
- `base_position_v3`
- `dist_res20`
- `dist_sup20`
- `avg_component_vr`
- optional `candidate_action_label`
- optional `candidate_event_type`

**Step 2: Remove reduce mining from this round**

Build separately tagged mined rows only for prototype `add` failures.

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

**Step 1: Refresh prototype-add failure pools**

Generate new mined pools for:

- `1d`
- `3d`
- `5d`

**Step 2: Refresh baseline vs augmented comparisons**

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

**Step 1: Record the prototype-add rules**

Document:

- why this round is `add` only
- how untouched-validation negatives drove the rule rewrite
- what prototype thresholds were used

**Step 2: Record the refreshed metrics**

Document:

- mined sample counts
- baseline vs augmented balance-aware metrics
- whether prototype alignment improved results

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

**Step 2: Spot-check refreshed experiment outputs**

Run:

```powershell
Get-Content D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\07_real_failure_event_experiment\1d\comparison_summary.csv -TotalCount 40
Get-Content D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\07_real_failure_event_experiment\5d\distribution_summary.csv -TotalCount 40
```

Expected: files exist, are non-empty, and show prototype-add-only mined statistics.
