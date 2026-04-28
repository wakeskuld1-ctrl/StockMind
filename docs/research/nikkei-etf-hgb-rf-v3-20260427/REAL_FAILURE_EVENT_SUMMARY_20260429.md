# Nikkei Real Failure Event Summary (2026-04-29)

## 1. Scope

This summary covers the refreshed real-failure-event continuation-balance experiment for Nikkei.

The goal was narrower than the prior simulated-action pass, narrower than the first broad real-failure pass, and later specialized again for `5D`:

- mine only real historical failure contexts,
- then tighten that mining to prototype-driven `add` failures aligned to untouched-validation negatives,
- then split `5D` into a dedicated slow-fail rule while freezing `1D/3D`,
- keep the mined pool separately tagged,
- and test whether a horizon-specific negative-only pool improves continuation performance on the same untouched real validation slice.

This summary reflects the latest approved Scheme A refinement:

- the builder is horizon-specific;
- each exported mined pool keeps only `continuation_label_<horizon> = 0`;
- this round mines `add` only;
- this round allows `daily_position` prototype rows even when explicit event fields are blank;
- `5D` now uses a dedicated slow-fail rule instead of the shared `1D/3D` add prototype.

## 2. Artifact Outputs

The experiment snapshot was written to:

- `artifacts/07_real_failure_event_experiment/1d/`
- `artifacts/07_real_failure_event_experiment/3d/`
- `artifacts/07_real_failure_event_experiment/5d/`

Each horizon package includes:

- `01_real_failure_event_samples.csv`
- `comparison_summary.csv`
- `distribution_summary.csv`
- `baseline_predictions_<horizon>.csv`
- `augmented_predictions_<horizon>.csv`
- `experiment_summary.json`

## 3. Prototype-Add Mining Rules

The builder mines only governed `historical_research` rows.

This round mines only prototype-driven `add` failures.

Candidate rows must satisfy the dominant untouched-validation negative shape:

- `signal_direction = add`
- `signal_family in {daily_position, breakout_followthrough}`
- `base_position_v3 >= 0.18`
- `dist_res20 <= 0.02`
- `dist_sup20 >= 0.02`
- `avg_component_vr >= 0.74`

Explicit event fields are optional in this round:

- blank `candidate_action_label` and blank `candidate_event_type` are allowed
- if explicit labels exist, only mild add-failure labels may pass:
  - `candidate_action_label in {resistance_reject_watch, false_breakout_avoid_chase, uncertain_breakout_wait, resistance_break_watch}`
  - `candidate_event_type in {near_resistance_20d, breakout_20d, breakout_60d}`

Rows dominated by downside-defense or `reduce` semantics are rejected in this round.

For `1D/3D`, the shared prototype-add rule remains unchanged.

For `5D`, the builder now uses two dedicated slow-fail subcontexts:

- `prototype_add_failure_5d_resistance_exhaustion`
  - higher position
  - close to resistance
  - broad market breadth already elevated
- `prototype_add_failure_5d_extended_drift`
  - higher position
  - already extended beyond resistance
  - then drifts into a slower multi-day failure

All mined rows are tagged as:

- `sample_source=real_failure_event_mining`
- `is_real_failure_event=True`
- `source_sample_id=<real sample id>`
- `failure_label_horizon=<requested horizon>`
- `mined_action_direction=add`
- `mined_failure_reason=prototype_add_failure`

## 4. Horizon-Specific Mined Sample Size

The builder now emits pure negative-only prototype-add pools for each requested horizon.

### `1D`

- full mined pool count: `38`
- mined train count: `22`
- label mix inside mined train: `0 = 22`

### `3D`

- full mined pool count: `39`
- mined train count: `25`
- label mix inside mined train: `0 = 25`

### `5D`

- full mined pool count: `9`
- mined train count: `1`
- label mix inside mined train: `0 = 1`

## 5. Comparison Result

The comparison kept the validation slice real-only.

Mined rows were added to training only, never to validation.

### `1D`

| Metric | Baseline | Augmented |
|---|---:|---:|
| train count | `827` | `908` |
| real failure train count | `0` | `22` |
| validation accuracy | `0.6135` | `0.6473` |
| validation balanced accuracy | `0.7906` | `0.7230` |
| negative recall | `1.0000` | `0.8125` |

### `3D`

| Metric | Baseline | Augmented |
|---|---:|---:|
| train count | `836` | `861` |
| real failure train count | `0` | `25` |
| validation accuracy | `0.5789` | `0.6459` |
| validation balanced accuracy | `0.7147` | `0.6363` |
| negative recall | `0.8750` | `0.6250` |

### `5D`

| Metric | Baseline | Augmented |
|---|---:|---:|
| train count | `846` | `847` |
| real failure train count | `0` | `1` |
| validation accuracy | `0.5708` | `0.5802` |
| validation balanced accuracy | `0.6018` | `0.6067` |
| negative recall | `0.6364` | `0.6364` |

## 6. Interpretation

The prototype-add tightening did move the experiment in the right direction.

What is now true:

- the mined pool is no longer mixed with requested-horizon positives;
- the pool is now aligned much more closely to the dominant untouched-validation negative shape;
- `1D` and `3D` balance-aware metrics improved materially versus the previous broad real-failure pass.

What also remains true:

- none of the three horizons are ready to join governed training;
- `1D/3D` still remain below baseline despite improved shape alignment;
- `5D` now slightly beats its own baseline, but with a very small train-time mined sample count.

Compared with the previous broad real-failure pass:

- `1D balanced_accuracy` improved from `0.5805 -> 0.7230`
- `1D negative recall` improved from `0.2500 -> 0.8125`
- `3D balanced_accuracy` improved from `0.5000 -> 0.6363`
- `3D negative recall` improved from `0.0000 -> 0.6250`
- `5D balanced_accuracy` improved from `0.5000 -> 0.5841`
- `5D negative recall` improved from `0.0000 -> 0.2727`

Compared with the previous shared prototype-add pass:

- `5D balanced_accuracy` improved again from `0.5841 -> 0.6067`
- `5D negative recall` improved from `0.2727 -> 0.6364`
- `5D` augmented train-time mined negatives shrank from `28 -> 1`

So the earlier failure was not only caused by mixed-horizon leakage, and the later `5D` specialization does help, but it is now constrained by sample density.

## 7. What This Proves

This refreshed pass answers the current quality-control question.

It proves:

- the problem was not just a builder-filter bug;
- the dominant untouched-validation negatives are indeed prototype-driven `premature_add` rows;
- mining closer to that prototype shape does improve `1D` and `3D` behavior;
- a dedicated `5D` slow-fail rule can outperform the prior shared `5D` prototype and slightly beat the `5D` baseline;
- but the current `5D` result is still fragile because the time-aware train split keeps only one mined training row.

The active bottleneck is now:

- remaining representativeness gaps inside the `premature_add` family,
- especially multi-day continuation weakness on `5D`,
- plus low effective sample density after the time split,
- not broad quantity.

## 8. Decision

Current decision:

- keep the real-failure pool experimental only;
- do not merge it into the governed continuation training base;
- do not headline raw accuracy as evidence of improvement;
- treat this round as evidence that prototype alignment and `5D` specialization are the correct direction, but not yet a production-ready solution.

## 9. Recommended Next Step

Recommended next step:

1. keep the focus on prototype-driven `add` failures, not broad mixed-direction mining;
2. treat the `5D` rule as separately specialized from now on;
3. increase effective pre-validation `5D` sample density by mining earlier historical rows that match the two `5D` slow-fail subcontexts;
4. split the current prototype-add family into tighter subtypes, for example:
   - blank-event `daily_position` premature add
   - near-resistance rejection after add
   - breakout-followthrough that fails by `3D/5D`
5. compare only on:
   - `balanced_accuracy`
   - negative recall
   - real validation only
