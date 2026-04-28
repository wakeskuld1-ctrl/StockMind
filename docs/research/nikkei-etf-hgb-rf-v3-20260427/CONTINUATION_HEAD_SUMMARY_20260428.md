# Nikkei Continuation Head Summary (2026-04-28)

## 1. Scope

This summary covers the first offline Nikkei `continuation head` built on top of the replay-classifier sample base.

The purpose is narrower than generic direction prediction:

- after a governed signal appears,
- and after replay labels define whether that action looked correct, acceptable, premature, or late,
- estimate whether the continuation quality looks usable or not over `1D / 3D / 5D`.

This is a second-stage research layer, not a live execution authority.

## 2. Artifact Outputs

The first continuation-head snapshot was written to:

- `artifacts/05_continuation_head_full_snapshot/01_continuation_labeled_samples.csv`
- `artifacts/05_continuation_head_full_snapshot/02_continuation_metrics_1d.csv`
- `artifacts/05_continuation_head_full_snapshot/02_continuation_metrics_3d.csv`
- `artifacts/05_continuation_head_full_snapshot/02_continuation_metrics_5d.csv`
- `artifacts/05_continuation_head_full_snapshot/03_continuation_predictions_1d.csv`
- `artifacts/05_continuation_head_full_snapshot/03_continuation_predictions_3d.csv`
- `artifacts/05_continuation_head_full_snapshot/03_continuation_predictions_5d.csv`
- `artifacts/05_continuation_head_full_snapshot/training_summary_1d.json`
- `artifacts/05_continuation_head_full_snapshot/training_summary_3d.json`
- `artifacts/05_continuation_head_full_snapshot/training_summary_5d.json`

## 3. Label Contract

The first continuation mapping is binary and replay-derived:

- positive continuation (`1`)
  - `correct_add`
  - `acceptable_add`
  - `correct_reduce`
  - `acceptable_reduce`
- negative continuation (`0`)
  - `premature_add`
  - `late_add`
  - `premature_reduce`
  - `late_reduce`
- excluded from supervision
  - `inconclusive`

This means continuation head does not replace replay labels. It compresses replay outcomes into a second-stage quality target.

## 4. Sample Size And Class Balance

Observed continuation samples by horizon:

| Horizon | Eligible Samples | Positive | Negative |
|---|---:|---:|---:|
| `1D` | `1034` | `980` | `54` |
| `3D` | `1045` | `975` | `70` |
| `5D` | `1058` | `990` | `68` |

This is the key structural limit in the first pass:

- the target is highly imbalanced toward `continuation = 1`;
- a naive majority-class predictor would already look strong on plain accuracy;
- continuation quality must therefore be interpreted with class-balance-aware diagnostics, not accuracy alone.

## 5. First Model Result

Model:

- `LogisticRegression`
- no Dummy fallback was needed on any horizon

Validation accuracy:

| Horizon | Train Accuracy | Validation Accuracy |
|---|---:|---:|
| `1D` | `0.6312` | `0.6135` |
| `3D` | `0.6160` | `0.5789` |
| `5D` | `0.6277` | `0.5708` |

Majority-class validation baseline:

| Horizon | Majority Baseline Accuracy |
|---|---:|
| `1D` | `0.9227` |
| `3D` | `0.9234` |
| `5D` | `0.9481` |

Balanced accuracy computed from the exported validation predictions:

| Horizon | Model Balanced Accuracy | Majority Baseline Balanced Accuracy |
|---|---:|---:|
| `1D` | `0.7906` | `0.5000` |
| `3D` | `0.7147` | `0.5000` |
| `5D` | `0.6018` | `0.5000` |

## 6. Interpretation

The first continuation head is usable as a research scaffold, but not yet as a production decision layer.

What is true:

- the continuation pipeline is now real, testable, and reproducible;
- the model is learning something beyond the trivial majority baseline when judged by balanced accuracy;
- `1D` looks materially more learnable than `5D`.

What is also true:

- raw validation accuracy is worse than the majority baseline because the model is trying to recover minority negative outcomes;
- the label space is too skewed for accuracy to be a trustworthy headline metric;
- `5D` continuation is only weakly separated in this first pass.

## 7. What Continuation Adds Beyond Replay

Replay classifier asks:

- was the action quality correct, acceptable, premature, or late?

Continuation head asks:

- after compressing that replay truth, does this event look like a usable continuation or a stop-quality continuation?

So the intended stack is now:

1. base position framework
2. HGB/RF adjustment model
3. replay classifier
4. continuation head

The fourth layer is only meaningful if the second and third layers remain the governing entry points.

## 8. Limits

- The first trainer is still a simple offline sklearn pipeline.
- The target is highly imbalanced.
- `late_add` and `late_reduce` remain sparse or absent upstream, so the negative class is partly synthetic-compressed.
- Continuation output is not wired into the daily operator.
- This summary does not prove live trading alpha.

## 9. Next Step

Recommended next step:

1. keep replay classifier as the first signal-quality gate;
2. treat continuation head as a research-only refinement layer for now;
3. optimize continuation on balance-aware metrics, especially `balanced_accuracy`, not raw accuracy;
4. consider a later split between `add` and `reduce` continuation only after sample density improves.

## 10. Follow-Up Augmentation Result

The first simulated-action balance experiment was completed on `2026-04-29`.

Use:

- `SIMULATED_ACTION_BALANCE_SUMMARY_20260429.md`
- `artifacts/06_simulated_action_balance_experiment/`

Current takeaway:

- the first augmentation increased raw validation accuracy,
- but reduced `balanced_accuracy` and minority negative recall,
- so the current bottleneck is negative-sample quality, not only negative-sample count.

## 11. Follow-Up Real Failure Event Result

The second balance experiment was completed on `2026-04-29`.

Use:

- `REAL_FAILURE_EVENT_SUMMARY_20260429.md`
- `artifacts/07_real_failure_event_experiment/`

Current takeaway:

- the approved Scheme A builder now emits horizon-specific pure negative pools only;
- the later prototype-add refinement improved the broad real-failure pass materially on `1D / 3D`, and partially on `5D`;
- the later dedicated `5D` slow-fail rule pushed `5D balanced_accuracy` slightly above baseline, but only with one mined training row after the time split;
- so the active gap is now effective `5D` sample density inside the specialized slow-fail family, plus the remaining representativeness split inside `premature_add`.
