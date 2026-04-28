# Nikkei Simulated Action Balance Summary (2026-04-29)

## 1. Scope

This summary covers the first safe augmentation experiment for Nikkei continuation research.

The purpose was not to replace the governed replay or continuation source of truth.

The purpose was:

- mine real historical event rows,
- generate separately tagged simulated add/reduce action samples,
- and test whether train-only augmentation improves continuation performance on the same untouched real validation slice.

## 2. Artifact Outputs

The experiment snapshot was written to:

- `artifacts/06_simulated_action_balance_experiment/1d/`
- `artifacts/06_simulated_action_balance_experiment/3d/`
- `artifacts/06_simulated_action_balance_experiment/5d/`

Each horizon package includes:

- `01_simulated_action_samples.csv`
- `comparison_summary.csv`
- `distribution_summary.csv`
- `baseline_predictions_<horizon>.csv`
- `augmented_predictions_<horizon>.csv`

## 3. Simulation Rules

The experiment used real historical rows only.

Simulated `add` rows were generated from breakout / resistance / support-style contexts such as:

- `signal_family in {breakout_followthrough, support_test}`
- `candidate_event_type in {breakout_20d, breakout_60d, near_resistance_20d}`

Simulated `reduce` rows were generated from breakdown / downside / support-failure contexts such as:

- `signal_family in {breakdown_followthrough, support_test}`
- `candidate_event_type in {breakdown_20d, breakdown_60d, near_support_20d}`
- `downside_suggested_action in {tighten_risk, reduce_partial_or_wait_reclaim, reduce_or_avoid}`

All simulated rows were tagged as:

- `sample_source=simulated_action_replay`
- `is_simulated_action=True`
- `source_sample_id=<real sample id>`

## 4. Simulated Sample Size

Total simulated rows generated from the historical replay base:

- total rows: `817`
- simulated `add`: `527`
- simulated `reduce`: `290`

Continuation label mix inside the simulated pool:

| Horizon | Positive | Negative | Missing |
|---|---:|---:|---:|
| `1D` | `568` | `171` | `78` |
| `3D` | `534` | `222` | `61` |
| `5D` | `536` | `230` | `51` |

## 5. Comparison Result

The comparison was done correctly on the same real validation slice.

Simulated rows were added to training only, never to validation.

### `1D`

| Metric | Baseline | Augmented |
|---|---:|---:|
| train count | `827` | `1451` |
| simulated train count | `0` | `624` |
| validation accuracy | `0.6135` | `0.8551` |
| validation balanced accuracy | `0.7906` | `0.5779` |
| negative recall | `1.0000` | `0.2500` |

### `3D`

| Metric | Baseline | Augmented |
|---|---:|---:|
| train count | `836` | `1474` |
| simulated train count | `0` | `638` |
| validation accuracy | `0.5789` | `0.9187` |
| validation balanced accuracy | `0.7147` | `0.4974` |
| negative recall | `0.8750` | `0.0000` |

### `5D`

| Metric | Baseline | Augmented |
|---|---:|---:|
| train count | `846` | `1494` |
| simulated train count | `0` | `648` |
| validation accuracy | `0.5708` | `0.9481` |
| validation balanced accuracy | `0.6018` | `0.5000` |
| negative recall | `0.6364` | `0.0000` |

## 6. Interpretation

The experiment did increase raw validation accuracy.

But that was the wrong improvement.

What actually happened:

- the augmentation pushed the model much harder toward the majority positive class;
- `balanced_accuracy` fell on every horizon;
- minority negative recall collapsed, especially on `3D` and `5D`.

This means the first simulated-action pool improved the model's willingness to predict `continuation = 1`, but made it worse at detecting the minority situations we actually needed help with.

## 7. What This Proves

This experiment was still useful because it answered the right question.

It proved:

- the current problem is not only sparse quantity;
- the bigger problem is simulated negative-shape quality;
- naive event-derived augmentation is not ready to join the governed training base.

So the current bottleneck is:

- not “we need more rows at any cost,”
- but “we need negative rows that look more like real failure cases seen in untouched validation.”

## 8. Decision

Current decision:

- keep the simulated-action pool experimental only;
- do not merge it into the official replay or continuation source-of-truth datasets;
- do not treat the first augmentation as a production improvement.

## 9. Next Step

Recommended next step:

1. move from broad event augmentation to targeted failure-event mining;
2. focus on real negative cases:
   - false breakout
   - support fail after add
   - premature reduce before continued rally
   - late reduce after real downside expansion
3. compare future experiments primarily on:
   - `balanced_accuracy`
   - negative recall
   - real validation only
