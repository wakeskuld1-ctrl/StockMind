# Nikkei Real Failure Event Mining Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a narrow train-only augmentation lane that mines real Nikkei failure events from governed historical event-study contexts, emits horizon-specific negative-only pools, and compares baseline vs failure-mined continuation training on the same real validation slice.

**Architecture:** Reuse the existing replay and continuation scripts, but make the dedicated real-failure event builder horizon-specific and keep the experiment runner aligned to the same requested horizon. Keep official replay exports unchanged. Measure only real-validation performance and prioritize balance-aware metrics.

**Tech Stack:** Python, pandas, scikit-learn, existing Nikkei replay/continuation scripts, markdown summaries

---

### Task 1: Freeze the horizon-specific real-failure event contract in tests

**Files:**
- Create: `D:\SM\scripts\test_nikkei_real_failure_event_balance.py`
- Reference: `D:\SM\docs\plans\2026-04-29-nikkei-real-failure-event-mining-design.md`

**Step 1: Write the failing failure-pool schema test**

Require one builder entrypoint that accepts `label_horizon` and emits rows with:

- `sample_source=real_failure_event_mining`
- `source_sample_id`
- `is_real_failure_event`
- `failure_label_horizon`
- `mined_action_direction`
- `mined_failure_reason`
- replay labels
- continuation labels

**Step 2: Write the failing negative-only retention test**

Require the mined pool to retain only rows whose derived continuation label is negative for the requested horizon.

**Step 3: Run test to verify it fails**

Run:

```powershell
python D:\SM\scripts\test_nikkei_real_failure_event_balance.py
```

Expected: FAIL because the builder and runner do not exist yet.

### Task 2: Implement the horizon-specific real-failure event builder

**Files:**
- Modify: `D:\SM\scripts\build_nikkei_replay_samples.py`
- Test: `D:\SM\scripts\test_nikkei_real_failure_event_balance.py`

**Step 1: Add narrow failure-event filters**

Implement explicit add-failure and reduce-failure rule filters using:

- `candidate_action_label`
- `candidate_event_type`
- `candidate_stood_*`
- `downside_suggested_action`

**Step 2: Add the builder**

Build separately tagged mined rows, derive replay labels, derive continuation labels, and keep only rows with `continuation_label_<label_horizon> = 0`.

**Step 3: Run tests**

Run:

```powershell
python D:\SM\scripts\test_nikkei_real_failure_event_balance.py
```

Expected: builder tests pass; runner tests still fail.

### Task 3: Freeze the experiment-runner contract in tests

**Files:**
- Modify: `D:\SM\scripts\test_nikkei_real_failure_event_balance.py`

**Step 1: Write the failing comparison-summary test**

Require one runner that emits:

- baseline and augmented counts
- baseline and augmented validation accuracy
- baseline and augmented balanced accuracy
- baseline and augmented negative recall

**Step 2: Write the failing validation-isolation test**

Require validation rows to remain real-only.

**Step 3: Run test to verify it fails**

Run:

```powershell
python D:\SM\scripts\test_nikkei_real_failure_event_balance.py
```

Expected: FAIL because the runner does not exist yet.

### Task 4: Implement the real-failure experiment runner

**Files:**
- Create: `D:\SM\scripts\run_nikkei_real_failure_event_balance.py`
- Reuse or Read: `D:\SM\scripts\train_nikkei_continuation_head.py`
- Test: `D:\SM\scripts\test_nikkei_real_failure_event_balance.py`

**Step 1: Implement baseline vs mined-failure split logic**

Use:

- real observed rows for split definition
- real train rows for baseline
- real train rows plus horizon-matched mined-failure rows for augmented training
- the same real validation rows for both runs

**Step 2: Emit experiment artifacts**

Write outputs under:

- `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\07_real_failure_event_experiment\`

including:

- mined failure sample CSV
- distribution summary
- comparison summary
- baseline and augmented prediction exports

**Step 3: Run tests**

Run:

```powershell
python D:\SM\scripts\test_nikkei_real_failure_event_balance.py
```

Expected: PASS.

### Task 5: Run the real experiment

**Files:**
- Read: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\04_replay_classifier_full_snapshot\02_replay_labeled_samples.csv`
- Write: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\07_real_failure_event_experiment\*`

**Step 1: Generate the mined failure pool**

Run the builder on the governed historical replay rows.

**Step 2: Run baseline vs failure-mined comparison**

Compute per-horizon metrics for:

- `1d`
- `3d`
- `5d`

**Step 3: Spot-check outputs**

Read the exported distribution and comparison files.

### Task 6: Document results

**Files:**
- Create: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\REAL_FAILURE_EVENT_SUMMARY_20260429.md`
- Modify: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\CONTINUATION_HEAD_SUMMARY_20260428.md`
- Modify: `D:\SM\docs\handoff\CURRENT_STATUS.md`

**Step 1: Write the summary**

Document:

- mining rules
- mined failure sample counts
- baseline vs augmented metrics
- whether the narrow failure pool helped
- whether the pool stays experimental

**Step 2: Update handoff**

Record whether the broad simulated-action lane should remain frozen and whether real failure mining became the preferred next path.

### Task 7: Fresh verification before claiming completion

**Files:**
- Verify all created and modified files from Tasks 1-6

**Step 1: Run new and existing tests**

Run:

```powershell
python D:\SM\scripts\test_nikkei_real_failure_event_balance.py
python D:\SM\scripts\test_nikkei_continuation_head.py
python D:\SM\scripts\test_nikkei_replay_classifier.py
python D:\SM\scripts\test_run_nikkei_hgb_rf_daily_workflow.py
python C:\Users\wakes\.codex\skills\nikkei-live-journal\scripts\test_upsert_journal.py
```

Expected: PASS.

**Step 2: Spot-check experiment outputs**

Run:

```powershell
Get-Content D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\07_real_failure_event_experiment\5d\comparison_summary.csv -TotalCount 40
Get-Content D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\07_real_failure_event_experiment\5d\distribution_summary.csv -TotalCount 40
```

Expected: files exist, are non-empty, and show separate baseline / augmented statistics.
