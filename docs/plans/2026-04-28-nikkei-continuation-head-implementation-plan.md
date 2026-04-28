# Nikkei Continuation Head Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the first offline Nikkei `continuation head` that reuses replay-classifier samples and predicts whether a governed signal continues cleanly over `1D / 3D / 5D`.

**Architecture:** Keep the work fully inside the Python research lane. Extend the replay sample builder with continuation-label derivation, add a continuation trainer, export per-horizon artifacts, and update handoff docs after fresh verification. Do not modify Rust boundaries or the daily operator.

**Tech Stack:** Python, pandas, scikit-learn, existing replay sample artifacts, markdown handoff docs

---

### Task 1: Freeze the continuation-label contract in tests

**Files:**
- Create: `D:\SM\scripts\test_nikkei_continuation_head.py`
- Reference: `D:\SM\docs\plans\2026-04-28-nikkei-continuation-head-design.md`

**Step 1: Write the failing label-mapping test**

Require one callable derivation entrypoint that maps replay labels into:

- `continuation_label_1d`
- `continuation_label_3d`
- `continuation_label_5d`

with this frozen mapping:

- positive: `correct_add`, `acceptable_add`, `correct_reduce`, `acceptable_reduce`
- negative: `premature_add`, `late_add`, `premature_reduce`, `late_reduce`
- excluded: `inconclusive`

**Step 2: Write the failing exclusion test**

Require the supervised training frame to exclude `inconclusive` continuation rows.

**Step 3: Run test to verify it fails**

Run:

```powershell
python D:\SM\scripts\test_nikkei_continuation_head.py
```

Expected: FAIL because continuation derivation and trainer do not exist yet.

### Task 2: Implement continuation-label derivation on top of replay samples

**Files:**
- Modify: `D:\SM\scripts\build_nikkei_replay_samples.py`
- Test: `D:\SM\scripts\test_nikkei_continuation_head.py`

**Step 1: Add the minimum derivation entrypoint**

Implement a function that reads replay labels and publishes:

- `continuation_label_1d`
- `continuation_label_3d`
- `continuation_label_5d`

**Step 2: Keep the mapping isolated**

Store the replay-to-continuation mapping as named constants so the label contract is not buried inside trainer logic.

**Step 3: Run test to verify it passes**

Run:

```powershell
python D:\SM\scripts\test_nikkei_continuation_head.py
```

Expected: mapping tests pass; trainer tests still fail.

### Task 3: Freeze the continuation trainer contract in tests

**Files:**
- Modify: `D:\SM\scripts\test_nikkei_continuation_head.py`

**Step 1: Write the failing trainer smoke test**

Require one callable training entrypoint that:

- accepts a labeled sample CSV
- accepts `label_horizon`
- emits metrics, predictions, confusion matrix, label counts, and summary JSON

**Step 2: Write the failing target-version test**

Require a continuation target definition version and explicit label horizon in the summary output.

**Step 3: Run test to verify it fails**

Run:

```powershell
python D:\SM\scripts\test_nikkei_continuation_head.py
```

Expected: FAIL because the trainer does not exist yet.

### Task 4: Implement the continuation trainer

**Files:**
- Create: `D:\SM\scripts\train_nikkei_continuation_head.py`
- Test: `D:\SM\scripts\test_nikkei_continuation_head.py`

**Step 1: Implement minimum trainer**

Train one binary classifier from the replay sample base using:

- the existing governed feature subset
- one time-aware split
- exclusion of rows with missing or `inconclusive` continuation labels

**Step 2: Emit governed artifacts**

Write outputs under:

- `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\05_continuation_head_full_snapshot\`

Per horizon, export:

- `01_continuation_labeled_samples.csv`
- `02_continuation_metrics_<horizon>.csv`
- `03_continuation_predictions_<horizon>.csv`
- `04_continuation_label_counts_<horizon>.csv`
- `05_continuation_confusion_<horizon>.csv`
- `training_summary_<horizon>.json`

**Step 3: Run test to verify it passes**

Run:

```powershell
python D:\SM\scripts\test_nikkei_continuation_head.py
```

Expected: PASS.

### Task 5: Run the real continuation build and training flow

**Files:**
- Read: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\04_replay_classifier_full_snapshot\02_replay_labeled_samples.csv`
- Write: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\05_continuation_head_full_snapshot\*`

**Step 1: Export labeled continuation samples**

Run the builder path so the continuation-labeled sample export is refreshed from the current replay dataset.

**Step 2: Train for all three horizons**

Run the continuation trainer for:

- `1d`
- `3d`
- `5d`

**Step 3: Spot-check outputs**

Read the metrics, counts, and summaries to confirm files are non-empty and consistent with the contract.

### Task 6: Update research and handoff docs

**Files:**
- Modify: `D:\SM\docs\handoff\CURRENT_STATUS.md`
- Modify: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\ALGORITHM_HANDOFF_MANUAL.md`
- Modify: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\MODEL_SUMMARY_20260428.md`
- Modify: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\REPLAY_CLASSIFIER_SUMMARY_20260428.md`
- Create or Modify: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\CONTINUATION_HEAD_SUMMARY_20260428.md`

**Step 1: Write the continuation summary**

Document:

- label mapping
- sample counts per horizon
- core metrics
- main error modes
- what continuation adds beyond replay classification

**Step 2: Sync handoff docs**

Update handoff docs so continuation head is described as current implemented state instead of future work.

**Step 3: Read back the summary**

Run:

```powershell
Get-Content D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\CONTINUATION_HEAD_SUMMARY_20260428.md -TotalCount 120
```

Expected: summary includes metrics, limits, and next-step guidance.

### Task 7: Fresh verification before claiming completion

**Files:**
- Verify all created and modified files from Tasks 1-6

**Step 1: Run continuation tests**

Run:

```powershell
python D:\SM\scripts\test_nikkei_continuation_head.py
```

Expected: PASS.

**Step 2: Run replay and workflow regressions**

Run:

```powershell
python D:\SM\scripts\test_nikkei_replay_classifier.py
python D:\SM\scripts\test_run_nikkei_hgb_rf_daily_workflow.py
python C:\Users\wakes\.codex\skills\nikkei-live-journal\scripts\test_upsert_journal.py
```

Expected: PASS.

**Step 3: Spot-check artifact outputs**

Run:

```powershell
Get-Content D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\05_continuation_head_full_snapshot\training_summary_1d.json -TotalCount 80
Get-Content D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\05_continuation_head_full_snapshot\training_summary_3d.json -TotalCount 80
Get-Content D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\05_continuation_head_full_snapshot\training_summary_5d.json -TotalCount 80
```

Expected: files exist, declare the target version, and include non-empty sample counts.
