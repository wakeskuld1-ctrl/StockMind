# Nikkei XGBoost Model Compare Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a strict like-for-like comparison experiment for `HGB / RF / XGBoost` on the Nikkei V3 adjustment dataset without changing production research logic.

**Architecture:** Reuse the approved dataset, feature selection boundary, split boundary, and position-mapping rule from the packaged HGB/RF research artifacts. Implement the comparison as a standalone verification script under `.verification`, then emit isolated experiment outputs for metrics, backtests, and latest comparable signals.

**Tech Stack:** Python, pandas, numpy, scikit-learn, xgboost

---

## Boundary Contract

| Boundary | Role | Path / Field | Single Source Of Truth | Forbidden Reuse | Fallback Policy |
|---|---|---|---|---|---|
| Artifact root | This run's temporary outputs only | `.verification/nikkei_xgb_model_compare_20260427/` | New experiment directory | Must not write into packaged `docs/research/.../artifacts` | Fail fast if output path resolves outside `.verification` |
| History/data root | Training and backtest input data | `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts/01_training_and_intermediate_full_snapshot/analysis_exports/adjustment_point_analysis/55_v3_adjustment_model_dataset.csv` | Packaged dataset snapshot | Must not swap to refreshed daily scorer inputs | Reject run if dataset columns differ from expected schema |
| Live feature root | Strict test backtest and latest signal inputs | `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts/01_training_and_intermediate_full_snapshot/analysis_exports/*` | Packaged scorer input family | Must not mix with refreshed staged data | Reject run if the scorer feature builder cannot reproduce expected columns |
| Feature boundary | Model input columns | `feature_columns()` logic from packaged scorer | Exclude only approved non-feature fields | Must not add or remove features for XGBoost only | Fail fast on missing feature columns |
| Split boundary | Train/valid/test comparison windows | hard-coded date masks | Approved audit split | Must not shift dates to help one model | Reject run if split sample count is zero |
| Position boundary | Backtest position mapping | `base_position_v3 + 0.25 * pred_adjustment`, clipped to `[0, 1]` | Approved HGB/RF mapping | Must not tune mapping for XGBoost | Reject run if prediction labels fall outside `{-1,0,1}` |

### Field Mapping
- `dataset_path` -> history/data root only
- `analysis_root` -> live feature root only
- `output_root` -> artifact root only
- `feature_columns` -> feature boundary only
- `split masks` -> split boundary only
- `target_position` -> position boundary only

## Task 1: Create the standalone compare script

**Files:**
- Create: `.verification/nikkei_xgb_model_compare_20260427/run_model_compare.py`

**Step 1:** Load the packaged dataset and verify required columns exist.

**Step 2:** Recreate the approved feature exclusion rule and split masks.

**Step 3:** Define three model specs:
- `hgb_l2_leaf20`
- `rf_depth4_leaf20`
- `xgb_depth4_lr005`

**Step 4:** Add strict guards for:
- output root staying under `.verification`
- feature completeness
- split non-emptiness
- prediction labels limited to `-1/0/1`

## Task 2: Emit strict classification outputs

**Files:**
- Modify: `.verification/nikkei_xgb_model_compare_20260427/run_model_compare.py`

**Step 1:** Train each model on the approved train split only.

**Step 2:** Score `train / valid / test / all`.

**Step 3:** Write `model_compare_classification_metrics.csv` with:
- `model`
- `split`
- `sample_count`
- `accuracy`
- `balanced_accuracy`
- `label_counts`
- `pred_counts`

## Task 3: Emit backtest outputs

**Files:**
- Modify: `.verification/nikkei_xgb_model_compare_20260427/run_model_compare.py`

**Step 1:** Build strict-test backtests using the train-only model on the approved test split.

**Step 2:** Build full-sample diagnostic backtests by training on all labeled rows and scoring all labeled rows.

**Step 3:** Write:
- `model_compare_strict_test_backtest.csv`
- `model_compare_full_backtest_summary.csv`
- per-model equity curves for traceability

## Task 4: Emit latest comparable signal and importance outputs

**Files:**
- Modify: `.verification/nikkei_xgb_model_compare_20260427/run_model_compare.py`

**Step 1:** Use the latest labeled row in the packaged dataset as the latest comparable snapshot.

**Step 2:** Write `model_compare_latest_signal_snapshot.csv`.

**Step 3:** Write `xgb_feature_importance.csv` from native XGBoost importances.

## Task 5: Verification

**Files:**
- Output only: `.verification/nikkei_xgb_model_compare_20260427/*`

**Step 1:** Confirm HGB and RF reproduced metrics are close to packaged `56/59/60` outputs.

**Step 2:** Confirm all tables include sample counts and dates.

**Step 3:** Confirm XGBoost results are evaluated under the exact same split and position rule.

**Step 4:** Record any mismatch between reproduced HGB/RF numbers and packaged numbers as a verification limit instead of hiding it.
