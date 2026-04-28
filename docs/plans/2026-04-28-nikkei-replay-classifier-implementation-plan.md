# Nikkei Replay Classifier Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the first Nikkei `post-signal replay classifier` pipeline so the project can classify governed add/reduce signal quality over `1D / 3D / 5D` event horizons instead of relying on generic `1w` future direction.

**Architecture:** Keep the first implementation offline and Python-based. Reuse the current research package and live journal as data sources, build one event-anchored replay sample dataset, derive replay labels, train an initial classifier, and export auditable summaries. Do not change Rust Tool boundaries or journal CSV schema in v1.

**Tech Stack:** Python, pandas, existing Nikkei research CSV artifacts, existing live journal artifacts, markdown summaries

---

### Task 1: Freeze the replay sample contract in tests

**Files:**
- Create: `D:\SM\scripts\test_nikkei_replay_classifier.py`
- Reference: `D:\SM\docs\plans\2026-04-28-prediction-enhancement-replay-design.md`

**Step 1: Write the failing schema test**

Add tests that require one builder function to emit rows with at least these fields:

- `signal_date`
- `sample_source`
- `signal_family`
- `signal_direction`
- `action_type`
- `base_position_v3`
- `rating_state`
- `dist_res20`
- `dist_sup20`
- `dist_sup60`
- `weighted_vol_down`
- `component_above200_breadth`
- `avg_component_vr`
- `horizon_1d_close_return`
- `horizon_3d_close_return`
- `horizon_5d_close_return`

Test skeleton:

```python
def test_build_replay_samples_emits_required_columns():
    samples = module.build_replay_samples(...)
    required = {...}
    assert required.issubset(set(samples.columns))
```

**Step 2: Write the failing source-separation test**

Require every row to contain a `sample_source` field with values such as:

- `historical_research`
- `live_journal`

and reject silent mixing.

```python
def test_build_replay_samples_marks_source_layer():
    assert set(samples["sample_source"]).issubset({"historical_research", "live_journal"})
```

**Step 3: Run the tests to verify failure**

Run:

```powershell
python D:\SM\scripts\test_nikkei_replay_classifier.py
```

Expected: FAIL because the replay sample builder does not exist yet.

### Task 2: Implement replay sample building

**Files:**
- Create: `D:\SM\scripts\build_nikkei_replay_samples.py`
- Read: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\01_training_and_intermediate_full_snapshot\analysis_exports\adjustment_point_analysis\04_adjustment_event_candidates.csv`
- Read: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\01_training_and_intermediate_full_snapshot\analysis_exports\adjustment_point_analysis\24_downside_reduction_logic_samples.csv`
- Read: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\01_training_and_intermediate_full_snapshot\analysis_exports\adjustment_point_analysis\55_v3_adjustment_model_dataset.csv`
- Read: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\02_live_like_backtest_full_snapshot\05_live_like_rule_audit.csv`
- Read: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\02_live_like_backtest_full_snapshot\09_no_deadband_decision_audit.csv`
- Read: `D:\SM\docs\trading-journal\nikkei\journal.csv`

**Step 1: Implement minimum sample builder**

Create a builder that:

- loads historical research event rows;
- normalizes them into one common event schema;
- appends live journal rows only with `sample_source="live_journal"`;
- preserves a source-specific event id;
- does not use post-`T0` values as input features.

**Step 2: Implement source-specific normalizers**

Add separate functions for:

- historical adjustment-point rows
- live journal rows

so the merge happens only after each source is normalized.

**Step 3: Save one auditable sample export**

Write the built sample table to:

- `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\04_replay_classifier_full_snapshot\01_replay_event_samples.csv`

**Step 4: Run the schema tests**

Run:

```powershell
python D:\SM\scripts\test_nikkei_replay_classifier.py
```

Expected: schema tests now pass, later label tests still fail.

### Task 3: Freeze the replay-label contract in tests

**Files:**
- Modify: `D:\SM\scripts\test_nikkei_replay_classifier.py`

**Step 1: Write the failing label tests**

Add tests that require the label derivation logic to emit:

- `replay_label_1d`
- `replay_label_3d`
- `replay_label_5d`

with allowed values:

- `correct_reduce`
- `acceptable_reduce`
- `premature_reduce`
- `late_reduce`
- `correct_add`
- `acceptable_add`
- `premature_add`
- `late_add`
- `inconclusive`

```python
def test_derive_replay_labels_emits_governed_label_vocabulary():
    labels = module.derive_replay_labels(samples)
    allowed = {...}
    assert set(labels["replay_label_1d"]).issubset(allowed)
```

**Step 2: Write the failing horizon-consistency test**

Require `1D / 3D / 5D` labels to be derived from event-horizon fields, not generic weekly-return columns.

```python
def test_labels_use_event_horizons_not_generic_1w_target():
    assert "positive_return_1w" not in derivation_inputs
```

**Step 3: Run the tests to verify failure**

Run:

```powershell
python D:\SM\scripts\test_nikkei_replay_classifier.py
```

Expected: FAIL because replay-label derivation is not implemented yet.

### Task 4: Implement replay-label derivation

**Files:**
- Modify: `D:\SM\scripts\build_nikkei_replay_samples.py`

**Step 1: Add horizon field derivation**

Derive and store:

- `horizon_1d_close_return`
- `horizon_3d_close_return`
- `horizon_5d_close_return`
- `horizon_1d_max_drawdown`
- `horizon_3d_max_drawdown`
- `horizon_5d_max_drawdown`
- `next_signal_primary_adjustment`
- `next_signal_secondary_adjustment`

**Step 2: Add replay-label derivation**

Implement rule-based labels by action polarity:

- reduce-family labels
- add-family labels
- fallback `inconclusive`

using the approved design semantics rather than a generic return threshold only.

**Step 3: Save one labeled sample export**

Write:

- `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\04_replay_classifier_full_snapshot\02_replay_labeled_samples.csv`

**Step 4: Run tests**

Run:

```powershell
python D:\SM\scripts\test_nikkei_replay_classifier.py
```

Expected: sample and label tests pass.

### Task 5: Train the first replay classifier

**Files:**
- Create: `D:\SM\scripts\train_nikkei_replay_classifier.py`
- Input: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\04_replay_classifier_full_snapshot\02_replay_labeled_samples.csv`

**Step 1: Write the failing training smoke test**

Extend `D:\SM\scripts\test_nikkei_replay_classifier.py` with a smoke test that requires:

- a non-empty metrics summary
- sample count by label
- a saved predictions table

```python
def test_train_replay_classifier_emits_metrics_and_predictions():
    outputs = module.train_replay_classifier(...)
    assert outputs["metrics"]["sample_count"] > 0
```

**Step 2: Implement the minimum trainer**

Train the first classifier on replay labels with:

- one frozen feature list from the sample builder;
- one time-aware split;
- one simple baseline model before any ensemble expansion.

The trainer must emit:

- metrics summary
- confusion summary
- per-row predictions

**Step 3: Save outputs**

Write:

- `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\04_replay_classifier_full_snapshot\03_replay_classifier_metrics.csv`
- `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\04_replay_classifier_full_snapshot\04_replay_classifier_predictions.csv`
- `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\04_replay_classifier_full_snapshot\05_replay_label_counts.csv`

**Step 4: Run tests**

Run:

```powershell
python D:\SM\scripts\test_nikkei_replay_classifier.py
```

Expected: all replay-classifier tests pass.

### Task 6: Export one auditable markdown summary

**Files:**
- Create: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\REPLAY_CLASSIFIER_SUMMARY_20260428.md`

**Step 1: Summarize results**

Document:

- sample source mix
- label balance
- key metrics
- common `premature_reduce` and `correct_reduce` patterns
- limits and next steps

**Step 2: Link the summary from handoff docs if results are usable**

If the outputs are coherent, update:

- `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\ALGORITHM_HANDOFF_MANUAL.md`
- `D:\SM\docs\handoff\CURRENT_STATUS.md`

with one short section pointing to the replay-classifier summary.

**Step 3: Run a readback check**

Run:

```powershell
Get-Content D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\REPLAY_CLASSIFIER_SUMMARY_20260428.md -TotalCount 120
```

Expected: summary includes metrics, limits, and next-step recommendations.

### Task 7: Fresh verification before claiming completion

**Files:**
- Verify created and modified files from Tasks 1-6

**Step 1: Run the full Python test file**

Run:

```powershell
python D:\SM\scripts\test_nikkei_replay_classifier.py
```

Expected: all tests pass.

**Step 2: Run the existing workflow regression to prove no accidental breakage**

Run:

```powershell
python D:\SM\scripts\test_run_nikkei_hgb_rf_daily_workflow.py
python C:\Users\wakes\.codex\skills\nikkei-live-journal\scripts\test_upsert_journal.py
```

Expected:

- daily workflow tests pass
- live journal tests pass

**Step 3: Spot-check the main exports**

Run:

```powershell
Get-Content D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\04_replay_classifier_full_snapshot\03_replay_classifier_metrics.csv -TotalCount 20
Get-Content D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\04_replay_classifier_full_snapshot\05_replay_label_counts.csv -TotalCount 20
```

Expected: files are present, readable, and non-empty.
