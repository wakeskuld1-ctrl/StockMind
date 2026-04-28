# Nikkei Replay Classifier Summary (2026-04-28)

## 1. Scope

This summary covers the first offline replay-classifier pipeline for Nikkei signal-quality evaluation.

The goal is not generic future weekly direction. The goal is to classify whether a governed signal was:

- `correct`
- `acceptable`
- `premature`
- `late`
- or `inconclusive`

across `1D / 3D / 5D` event horizons.

## 2. Artifact Outputs

The first replay-classifier snapshot was written to:

- `artifacts/04_replay_classifier_full_snapshot/01_replay_event_samples.csv`
- `artifacts/04_replay_classifier_full_snapshot/02_replay_labeled_samples.csv`
- `artifacts/04_replay_classifier_full_snapshot/03_replay_classifier_metrics.csv`
- `artifacts/04_replay_classifier_full_snapshot/04_replay_classifier_predictions.csv`
- `artifacts/04_replay_classifier_full_snapshot/05_replay_label_counts.csv`
- `artifacts/04_replay_classifier_full_snapshot/training_summary.json`

## 3. Sample Base

- Total replay samples: `2205`
- Source layers:
  - `historical_research`
  - `live_journal`
- Historical row base: `55_v3_adjustment_model_dataset.csv`
- Context enrichments:
  - `04_adjustment_event_candidates.csv`
  - `24_downside_reduction_logic_samples.csv`
- Real execution rows:
  - `docs/trading-journal/nikkei/journal.csv`

## 4. Label Distribution (`5D`)

Observed labels in the first `5D` run:

- `inconclusive`: `1147`
- `correct_reduce`: `361`
- `correct_add`: `314`
- `acceptable_reduce`: `200`
- `acceptable_add`: `115`
- `premature_add`: `48`
- `premature_reduce`: `20`

`late_add` and `late_reduce` were not observed in this first pass.

## 5. Optimized First Model Result

Model:

- `LogisticRegression`
- no Dummy fallback was needed

Metrics:

- sample count: `2203`
- observed outcome sample count: `2203`
- train sample count: `1762`
- validation sample count: `441`
- observed label count: `7`
- train accuracy: `0.8048`
- validation accuracy: `0.7211`

Compared with the earlier unbalanced first pass, the optimized version improved validation accuracy from about `0.6780` to `0.7211`.

## 6. What Was Cleaned

Two concrete cleanups were applied before the optimized rerun:

1. live journal rows with no completed replay outcome were marked as not-yet-observed and removed from supervised training;
2. the trainer stopped feeding sparse metadata fields and now uses the compact replay feature set plus `class_weight=balanced`.

This means the current result is less distorted by pending live rows and better aligned with minority replay classes.

## 7. Initial Interpretation

This first result is meaningfully better than the prior near-random generic `1w` direction framing, but it must not yet be treated as a production-ready live signal.

What looks promising:

- the event-anchored label design is learnable;
- the replay task produces a coherent validation score instead of collapsing near random;
- `correct_reduce` and `correct_add` already form visible clusters.

What still looks weak:

- `inconclusive` remains the dominant class;
- `premature_*` classes are sparse;
- the trainer is still a first-pass offline classifier, not a governed live operator model.

## 8. Error Pattern Snapshot

The dominant remaining validation confusions are:

- `acceptable_add -> premature_add`
- `correct_add -> acceptable_add`
- `correct_add -> premature_add`
- `acceptable_reduce -> correct_reduce`
- `acceptable_reduce -> premature_reduce`
- `premature_reduce -> acceptable_reduce`
- `correct_reduce -> acceptable_reduce`
- `correct_reduce -> premature_reduce`

This means the current feature set can identify direction and broad action family, but still under-separates signal quality within the same family:

- clean continuation
- noisy continuation
- fake breakout / premature add
- timely reduce
- slightly early reduce
- over-defensive reduce

That was consistent with the original design hypothesis that a later `continuation head` might still be needed after replay classification is stabilized.

## 9. Limits

- The first trainer uses a simple offline sklearn pipeline.
- Label rules are heuristic and event-anchored, but not yet committee-governed.
- `late_add` and `late_reduce` are currently absent from the observed label set.
- Live journal rows are included structurally, but real reviewed outcomes are still sparse.
- This summary does not yet prove live trading alpha; it only proves the replay task is more structurally coherent than the generic `1w` direction task.

## 10. Continuation Follow-Up

That follow-up has now been implemented as:

- `CONTINUATION_HEAD_SUMMARY_20260428.md`
- `artifacts/05_continuation_head_full_snapshot/`

Replay classifier remains the first gate. Continuation head is the second-stage refinement layer for within-family continuation quality, not a replacement for replay labels.

## 11. Next Step

Recommended next step:

1. keep replay classifier as the governing event-quality layer;
2. improve sparse replay negative classes, especially `premature_*` and missing `late_*`;
3. optimize continuation head on balance-aware metrics before considering any live-operator integration.
