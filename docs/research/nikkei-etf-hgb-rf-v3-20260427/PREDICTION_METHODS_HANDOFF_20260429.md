# Nikkei Prediction Methods Handoff (2026-04-29)

## Scope

This handoff is only for the two prediction-enhancement methods added on top of the Nikkei HGB/RF V3 position framework:

1. `Replay Classifier`
2. `Continuation Head`

This document is not the handoff for the base HGB/RF risk-position model itself.

## Why These Two Methods Exist

The original direction-prediction framing was too broad for the real trading question.

The later research reframed the problem into:

- after a governed adjustment point appears,
- judge whether the action itself looked correct / acceptable / premature / late,
- then judge whether that action has usable continuation quality.

So the two methods answer different questions:

### Method 1: Replay Classifier

Question:

- "Was this action good, acceptable, too early, or too late?"

Output family:

- `correct_add`
- `acceptable_add`
- `premature_add`
- `late_add`
- `correct_reduce`
- `acceptable_reduce`
- `premature_reduce`
- `late_reduce`
- `inconclusive`

### Method 2: Continuation Head

Question:

- "After compressing replay truth, is this a usable continuation or a stop-quality continuation?"

Output family:

- `1 = continuation usable`
- `0 = continuation not usable`

This is a second-stage research layer, not a standalone execution model.

## Code Entry Points

### Sample Builder

- `D:\SM\scripts\build_nikkei_replay_samples.py`

What it does:

- builds replay-event samples
- derives replay labels
- derives continuation labels
- builds experimental negative-augmentation pools

### Replay Trainer

- `D:\SM\scripts\train_nikkei_replay_classifier.py`

### Continuation Trainer

- `D:\SM\scripts\train_nikkei_continuation_head.py`

### Experimental Augmentation Runners

- `D:\SM\scripts\run_nikkei_simulated_action_balance.py`
- `D:\SM\scripts\run_nikkei_real_failure_event_balance.py`

### Contract Tests

- `D:\SM\scripts\test_nikkei_replay_classifier.py`
- `D:\SM\scripts\test_nikkei_continuation_head.py`
- `D:\SM\scripts\test_nikkei_simulated_action_balance.py`
- `D:\SM\scripts\test_nikkei_real_failure_event_balance.py`

## Artifact Map

### Replay Classifier

- Summary:
  - `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\REPLAY_CLASSIFIER_SUMMARY_20260428.md`
- Snapshot:
  - `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\04_replay_classifier_full_snapshot\`

### Continuation Head

- Summary:
  - `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\CONTINUATION_HEAD_SUMMARY_20260428.md`
- Snapshot:
  - `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\05_continuation_head_full_snapshot\`

### Augmentation Experiment 1: Simulated Action Balance

- Summary:
  - `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\SIMULATED_ACTION_BALANCE_SUMMARY_20260429.md`
- Snapshot:
  - `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\06_simulated_action_balance_experiment\`

### Augmentation Experiment 2-4: Real Failure Mining

- Summary:
  - `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\REAL_FAILURE_EVENT_SUMMARY_20260429.md`
- Snapshot:
  - `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\07_real_failure_event_experiment\`

## What Was Done Round By Round

### Round 0: Build Replay Classifier

What changed:

- built event-anchored replay samples
- trained the first replay classifier
- froze the replay label vocabulary and sample schema

What it proved:

- generic future up/down labels were too broad
- event-anchored replay truth is learnable enough to be useful as a first gate

### Round 1: Build Continuation Head

What changed:

- compressed replay labels into binary continuation truth
- trained `1D / 3D / 5D` continuation heads

What it proved:

- continuation is a real second-stage problem
- but class imbalance is severe

### Round 2: Simulated Action Augmentation

What changed:

- mined real historical rows
- generated simulated add/reduce action samples
- added them to train only

What it proved:

- raw `accuracy` went up
- `balanced_accuracy` and negative recall got worse
- so the bottleneck was not just sample count

### Round 3: Broad Real Failure Mining

What changed:

- stopped simulating actions
- mined only real failure-style rows
- fixed the builder to be horizon-specific and negative-only

What it proved:

- the failure was not just a builder-filter bug
- even real failure rows were still not shaped like untouched validation negatives

### Round 4: Prototype-Driven Add Failure Mining

What changed:

- inspected untouched validation negatives directly
- found that the dominant failure shape was `premature_add`
- narrowed mining to `add` only
- allowed `daily_position` rows with blank event fields

What it proved:

- this was the correct direction
- `1D / 3D` improved materially versus the broad real-failure pass
- but still did not beat baseline

### Round 5: 5D Slow-Fail Specialization

What changed:

- separated `5D` from the shared `1D/3D` add prototype
- added two `5D` slow-fail reasons:
  - `prototype_add_failure_5d_resistance_exhaustion`
  - `prototype_add_failure_5d_extended_drift`

What it proved:

- `5D` is not the same failure shape as `1D/3D`
- the dedicated `5D` rule slightly lifted `5D balanced_accuracy` above baseline
- but effective mined train count under the time-aware split is only `1`

## Current Best Understanding

### Replay Classifier

- The replay layer is the first useful prediction-enhancement method.
- It is event-anchored and much closer to the real trading decision than generic future direction labels.
- It should remain the first signal-quality gate.

### Continuation Head

- The continuation layer is real and useful as research.
- But it is extremely sensitive to class imbalance and negative-sample shape.
- The active bottleneck is no longer "do we have a continuation head."
- The active bottleneck is:
  - whether the negative samples actually match real failure shapes
  - and whether there are enough pre-validation examples for the specialized `5D` lane

## Current Bottlenecks

### Bottleneck 1: 1D/3D still below baseline

- The prototype-add direction improved the broad real-failure pass.
- But `1D/3D` still have not exceeded baseline `balanced_accuracy`.

### Bottleneck 2: 5D sample density

- The dedicated `5D` rule is better than the shared prototype for `5D`.
- But after the time-aware split, only `1` mined `5D` training row remains.
- So the problem has shifted from "rule shape" to "usable pre-validation sample density."

## What To Read First Next Time

1. `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\PREDICTION_METHODS_HANDOFF_20260429.md`
2. `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\REPLAY_CLASSIFIER_SUMMARY_20260428.md`
3. `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\CONTINUATION_HEAD_SUMMARY_20260428.md`
4. `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\REAL_FAILURE_EVENT_SUMMARY_20260429.md`
5. `D:\SM\scripts\build_nikkei_replay_samples.py`
6. `D:\SM\scripts\run_nikkei_real_failure_event_balance.py`

## Next Recommended Research Step

Do not broaden the mining rules again.

The next correct step is:

1. keep `Replay Classifier` and `Continuation Head` as the two prediction methods;
2. keep `5D` specialized;
3. mine more pre-validation historical rows that match the two `5D` slow-fail subtypes;
4. only then rerun the continuation experiment.

## Verification Commands

Fresh commands already used on this line:

```powershell
python D:\SM\scripts\test_nikkei_real_failure_event_balance.py
python D:\SM\scripts\test_nikkei_continuation_head.py
python D:\SM\scripts\test_nikkei_replay_classifier.py
python D:\SM\scripts\test_run_nikkei_hgb_rf_daily_workflow.py
python C:\Users\wakes\.codex\skills\nikkei-live-journal\scripts\test_upsert_journal.py
```

## Boundary Warning

Do not confuse these two prediction methods with the base HGB/RF position-sizing model.

The intended stack is:

1. HGB/RF V3 adjustment model
2. Replay Classifier
3. Continuation Head

The last two are enhancement layers, not replacements for the first one.
