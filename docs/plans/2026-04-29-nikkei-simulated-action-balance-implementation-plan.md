# Nikkei Simulated Action Balance Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a safe augmentation lane that mines real Nikkei event rows, generates separately tagged simulated add/reduce action samples, and compares baseline vs augmented continuation training on the same real validation slice.

**Architecture:** Keep the official replay and continuation source-of-truth exports unchanged. Add a simulated-sample builder plus an isolated comparison runner. Use TDD to freeze train-only augmentation and real-only validation. Export one experiment package under a new research artifact root.

**Tech Stack:** Python, pandas, scikit-learn, existing replay/continuation scripts, markdown research docs

---

### Task 1: Freeze the simulated-sample contract in tests

**Files:**
- Create: `D:\SM\scripts\test_nikkei_simulated_action_balance.py`
- Reference: `D:\SM\docs\plans\2026-04-29-nikkei-simulated-action-balance-design.md`

**Step 1: Write the failing simulated-row schema test**

Require one builder entrypoint that emits rows with:

- `sample_source=simulated_action_replay`
- `source_sample_id`
- `is_simulated_action`
- `simulated_action_direction`
- replay labels
- continuation labels

**Step 2: Write the failing validation-isolation test**

Require one comparison runner that keeps simulated rows out of validation.

**Step 3: Run test to verify it fails**

Run:

```powershell
python D:\SM\scripts\test_nikkei_simulated_action_balance.py
```

Expected: FAIL because the simulated builder and comparison runner do not exist yet.

### Task 2: Implement simulated action sample generation

**Files:**
- Modify: `D:\SM\scripts\build_nikkei_replay_samples.py`
- Test: `D:\SM\scripts\test_nikkei_simulated_action_balance.py`

**Step 1: Add candidate filters**

Add governed filters for:

- simulated add candidates
- simulated reduce candidates

using the approved event fields.

**Step 2: Add simulated-row builder**

Create rows that:

- preserve real market context and forward outcomes
- overwrite action semantics to simulated `add` or `reduce`
- derive replay and continuation labels from the simulated direction

**Step 3: Run tests**

Run:

```powershell
python D:\SM\scripts\test_nikkei_simulated_action_balance.py
```

Expected: builder tests pass; comparison tests still fail.

### Task 3: Freeze the augmentation comparison contract in tests

**Files:**
- Modify: `D:\SM\scripts\test_nikkei_simulated_action_balance.py`

**Step 1: Write the failing comparison summary test**

Require one comparison runner that emits:

- baseline counts
- augmented counts
- per-horizon validation accuracy
- per-horizon balanced accuracy

**Step 2: Write the failing real-only validation test**

Require the exported validation rows to exclude `sample_source=simulated_action_replay`.

**Step 3: Run tests to verify failure**

Run:

```powershell
python D:\SM\scripts\test_nikkei_simulated_action_balance.py
```

Expected: FAIL because the runner does not exist yet.

### Task 4: Implement the augmentation comparison runner

**Files:**
- Create: `D:\SM\scripts\run_nikkei_simulated_action_balance.py`
- Reuse or Read: `D:\SM\scripts\train_nikkei_continuation_head.py`
- Test: `D:\SM\scripts\test_nikkei_simulated_action_balance.py`

**Step 1: Implement baseline vs augmented split logic**

Use:

- real observed rows for split definition
- real train rows for baseline training
- real train rows plus simulated rows for augmented training
- the same real validation rows for both runs

**Step 2: Emit experiment artifacts**

Write outputs under:

- `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\06_simulated_action_balance_experiment\`

including:

- simulated sample CSV
- distribution tables
- per-horizon comparison summary CSV / JSON

**Step 3: Run tests**

Run:

```powershell
python D:\SM\scripts\test_nikkei_simulated_action_balance.py
```

Expected: PASS.

### Task 5: Run the real experiment

**Files:**
- Read: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\04_replay_classifier_full_snapshot\02_replay_labeled_samples.csv`
- Write: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\06_simulated_action_balance_experiment\*`

**Step 1: Generate simulated samples**

Run the simulated builder on the real historical replay rows.

**Step 2: Run baseline vs augmented comparison**

Compute the per-horizon before/after metrics.

**Step 3: Spot-check exports**

Read the distribution and summary files to confirm sample growth and validation isolation.

### Task 6: Document results

**Files:**
- Modify: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\CONTINUATION_HEAD_SUMMARY_20260428.md`
- Create: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\SIMULATED_ACTION_BALANCE_SUMMARY_20260429.md`
- Modify: `D:\SM\docs\handoff\CURRENT_STATUS.md`

**Step 1: Write the balance summary**

Document:

- simulated sample rules
- baseline vs augmented sample counts
- per-horizon before/after metrics
- whether augmentation helped
- whether simulated rows are still isolated from source-of-truth training

**Step 2: Update handoff**

Record whether the augmentation is only experimental or ready for the next governance step.

### Task 7: Fresh verification before claiming completion

**Files:**
- Verify all created and modified files from Tasks 1-6

**Step 1: Run new and existing tests**

Run:

```powershell
python D:\SM\scripts\test_nikkei_simulated_action_balance.py
python D:\SM\scripts\test_nikkei_continuation_head.py
python D:\SM\scripts\test_nikkei_replay_classifier.py
python D:\SM\scripts\test_run_nikkei_hgb_rf_daily_workflow.py
python C:\Users\wakes\.codex\skills\nikkei-live-journal\scripts\test_upsert_journal.py
```

Expected: PASS.

**Step 2: Spot-check experiment outputs**

Run:

```powershell
Get-Content D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\06_simulated_action_balance_experiment\comparison_summary.csv -TotalCount 40
Get-Content D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\06_simulated_action_balance_experiment\distribution_summary.csv -TotalCount 40
```

Expected: files exist, are non-empty, and show separate baseline / augmented statistics.
