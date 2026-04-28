"""
Run the Nikkei real-failure-event balance experiment.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

import pandas as pd

SCRIPT_ROOT = Path(__file__).resolve().parent
if str(SCRIPT_ROOT) not in sys.path:
    sys.path.insert(0, str(SCRIPT_ROOT))

from build_nikkei_replay_samples import (
    REAL_FAILURE_EVENT_SAMPLE_SOURCE,
    build_real_failure_event_samples,
)
from run_nikkei_simulated_action_balance import (
    _binary_balanced_accuracy,
    _filter_observed_rows,
)
from train_nikkei_continuation_head import (
    CONTINUATION_TARGET_DEFINITION_VERSION,
    LABEL_COLUMN_TEMPLATE,
    OBSERVED_COLUMN_TEMPLATE,
    _build_feature_frame,
    _build_time_aware_splits,
    _fit_classifier,
    _predict_probabilities,
)
from sklearn.metrics import accuracy_score, recall_score


# 2026-04-29 CST: Added because real failure-event mining is the follow-up to
# the failed broad simulated augmentation pass. Purpose: compare baseline
# continuation training against a narrow negative-only failure pool while
# keeping validation frozen to real rows only.
def run_nikkei_real_failure_event_balance(
    sample_path: str | Path,
    output_root: str | Path,
    label_horizon: str = "5d",
) -> dict[str, Any]:
    sample_path = Path(sample_path)
    output_root = Path(output_root)
    output_root.mkdir(parents=True, exist_ok=True)

    samples = pd.read_csv(sample_path, encoding="utf-8-sig")
    mined = build_real_failure_event_samples(samples, label_horizon=label_horizon)
    mined_path = output_root / "01_real_failure_event_samples.csv"
    mined.to_csv(mined_path, index=False, encoding="utf-8-sig")

    comparison, distribution, baseline_predictions, augmented_predictions = _run_single_horizon_experiment(
        real_samples=samples,
        mined_failure_samples=mined,
        label_horizon=label_horizon,
    )

    comparison_summary_path = output_root / "comparison_summary.csv"
    distribution_summary_path = output_root / "distribution_summary.csv"
    baseline_predictions_path = output_root / f"baseline_predictions_{label_horizon}.csv"
    augmented_predictions_path = output_root / f"augmented_predictions_{label_horizon}.csv"
    comparison.to_csv(comparison_summary_path, index=False, encoding="utf-8-sig")
    distribution.to_csv(distribution_summary_path, index=False, encoding="utf-8-sig")
    baseline_predictions.to_csv(baseline_predictions_path, index=False, encoding="utf-8-sig")
    augmented_predictions.to_csv(augmented_predictions_path, index=False, encoding="utf-8-sig")

    summary = {
        "target_definition_version": CONTINUATION_TARGET_DEFINITION_VERSION,
        "label_horizon": label_horizon,
        "comparison_summary_path": str(comparison_summary_path),
        "distribution_summary_path": str(distribution_summary_path),
        "baseline_predictions_path": str(baseline_predictions_path),
        "augmented_predictions_path": str(augmented_predictions_path),
        "real_failure_sample_path": str(mined_path),
    }
    (output_root / "experiment_summary.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    return summary


def _run_single_horizon_experiment(
    *,
    real_samples: pd.DataFrame,
    mined_failure_samples: pd.DataFrame,
    label_horizon: str,
) -> tuple[pd.DataFrame, pd.DataFrame, pd.DataFrame, pd.DataFrame]:
    label_column = LABEL_COLUMN_TEMPLATE.format(label_horizon=label_horizon)
    observed_column = OBSERVED_COLUMN_TEMPLATE.format(label_horizon=label_horizon)

    real_observed = _filter_observed_rows(real_samples, label_column=label_column, observed_column=observed_column)
    if real_observed.empty:
        raise ValueError("no real observed rows available for baseline experiment")
    split_names = _build_time_aware_splits(real_observed)
    train_mask = split_names == "train"
    validation_mask = split_names == "validation"
    baseline_train = real_observed.loc[train_mask].copy()
    validation = real_observed.loc[validation_mask].copy()

    mined_train = pd.DataFrame(columns=real_observed.columns)
    if not mined_failure_samples.empty and label_column in mined_failure_samples.columns and observed_column in mined_failure_samples.columns:
        mined_train = _filter_observed_rows(
            mined_failure_samples,
            label_column=label_column,
            observed_column=observed_column,
        )
    if not mined_train.empty:
        cutoff_date = pd.to_datetime(validation["signal_date"], errors="coerce").min()
        if pd.notna(cutoff_date):
            mined_dates = pd.to_datetime(mined_train["signal_date"], errors="coerce")
            mined_train = mined_train.loc[mined_dates < cutoff_date].copy()

    baseline_eval, baseline_predictions = _fit_and_evaluate(
        train_frame=baseline_train,
        validation_frame=validation,
        label_column=label_column,
        split_name="baseline",
    )
    augmented_train = (
        baseline_train.copy()
        if mined_train.empty
        else pd.concat([baseline_train, mined_train], ignore_index=True, sort=False)
    )
    augmented_eval, augmented_predictions = _fit_and_evaluate(
        train_frame=augmented_train,
        validation_frame=validation,
        label_column=label_column,
        split_name="augmented",
    )

    comparison = pd.DataFrame(
        [
            {
                "label_horizon": label_horizon,
                "baseline_train_count": len(baseline_train),
                "augmented_train_count": len(augmented_train),
                "validation_count": len(validation),
                "real_failure_train_count": len(mined_train),
                "baseline_accuracy": baseline_eval["accuracy"],
                "augmented_accuracy": augmented_eval["accuracy"],
                "baseline_balanced_accuracy": baseline_eval["balanced_accuracy"],
                "augmented_balanced_accuracy": augmented_eval["balanced_accuracy"],
                "baseline_negative_recall": baseline_eval["negative_recall"],
                "augmented_negative_recall": augmented_eval["negative_recall"],
            }
        ]
    )
    distribution = pd.DataFrame(
        [
            {"dataset": "baseline_train", "label_name": key, "sample_count": value}
            for key, value in baseline_train[label_column].astype(int).astype(str).value_counts().to_dict().items()
        ]
        + (
            [
                {"dataset": "real_failure_train", "label_name": key, "sample_count": value}
                for key, value in mined_train[label_column].astype(int).astype(str).value_counts().to_dict().items()
            ]
            if not mined_train.empty and label_column in mined_train.columns
            else []
        )
        + [
            {"dataset": "validation_real", "label_name": key, "sample_count": value}
            for key, value in validation[label_column].astype(int).astype(str).value_counts().to_dict().items()
        ]
    )
    return comparison, distribution, baseline_predictions, augmented_predictions


def _fit_and_evaluate(
    *,
    train_frame: pd.DataFrame,
    validation_frame: pd.DataFrame,
    label_column: str,
    split_name: str,
) -> tuple[dict[str, float], pd.DataFrame]:
    train_target = train_frame[label_column].astype(int).astype(str)
    validation_target = validation_frame[label_column].astype(int).astype(str)
    train_features = _build_feature_frame(train_frame, label_column=label_column)
    validation_features = _build_feature_frame(validation_frame, label_column=label_column)
    model, classifier_name, fallback_reason = _fit_classifier(train_features, train_target)
    predictions = model.predict(validation_features)
    probabilities = _predict_probabilities(model, validation_features)

    negative_recall = float("nan")
    if "0" in set(validation_target):
        negative_recall = float(recall_score(validation_target, predictions, pos_label="0", average="binary"))

    prediction_frame = validation_frame[["sample_id", "sample_source", "signal_date"]].copy()
    prediction_frame["data_split"] = "validation"
    prediction_frame["experiment_split"] = split_name
    prediction_frame["actual_label"] = validation_target.values
    prediction_frame["predicted_label"] = predictions
    prediction_frame["classifier_name"] = classifier_name
    prediction_frame["fallback_reason"] = fallback_reason
    for class_name, values in probabilities.items():
        prediction_frame[f"prob_{class_name}"] = values

    metrics = {
        "accuracy": float(accuracy_score(validation_target, predictions)),
        "balanced_accuracy": _binary_balanced_accuracy(validation_target, predictions),
        "negative_recall": negative_recall,
    }
    return metrics, prediction_frame
