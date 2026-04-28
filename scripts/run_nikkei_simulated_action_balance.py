"""
Run the Nikkei simulated-action balance experiment.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

import pandas as pd
from sklearn.metrics import accuracy_score, recall_score

SCRIPT_ROOT = Path(__file__).resolve().parent
if str(SCRIPT_ROOT) not in sys.path:
    sys.path.insert(0, str(SCRIPT_ROOT))

from build_nikkei_replay_samples import (
    SIMULATED_ACTION_SAMPLE_SOURCE,
    build_simulated_action_samples,
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


# 2026-04-29 CST: Added because Scheme A requires a safe way to compare real
# baseline training against train-only simulated augmentation. Purpose: keep the
# validation slice frozen to real rows while exporting before/after metrics.
def run_simulated_action_balance_experiment(
    sample_path: str | Path,
    output_root: str | Path,
    label_horizon: str = "5d",
) -> dict[str, Any]:
    sample_path = Path(sample_path)
    output_root = Path(output_root)
    output_root.mkdir(parents=True, exist_ok=True)

    samples = pd.read_csv(sample_path, encoding="utf-8-sig")
    simulated = build_simulated_action_samples(samples)
    simulated_path = output_root / "01_simulated_action_samples.csv"
    simulated.to_csv(simulated_path, index=False, encoding="utf-8-sig")

    comparison, distribution, baseline_predictions, augmented_predictions = _run_single_horizon_experiment(
        real_samples=samples,
        simulated_samples=simulated,
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
        "simulated_sample_path": str(simulated_path),
    }
    (output_root / "experiment_summary.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    return summary


def _run_single_horizon_experiment(
    *,
    real_samples: pd.DataFrame,
    simulated_samples: pd.DataFrame,
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
    simulated_train = _filter_observed_rows(simulated_samples, label_column=label_column, observed_column=observed_column)
    if not simulated_train.empty:
        cutoff_date = pd.to_datetime(validation["signal_date"], errors="coerce").min()
        if pd.notna(cutoff_date):
            simulated_dates = pd.to_datetime(simulated_train["signal_date"], errors="coerce")
            simulated_train = simulated_train.loc[simulated_dates < cutoff_date].copy()

    baseline_eval, baseline_predictions = _fit_and_evaluate(
        train_frame=baseline_train,
        validation_frame=validation,
        label_column=label_column,
        split_name="baseline",
    )
    augmented_train = pd.concat([baseline_train, simulated_train], ignore_index=True, sort=False)
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
                "simulated_train_count": len(simulated_train),
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
        + [
            {"dataset": "simulated_train", "label_name": key, "sample_count": value}
            for key, value in simulated_train[label_column].astype(int).astype(str).value_counts().to_dict().items()
        ]
        + [
            {"dataset": "validation_real", "label_name": key, "sample_count": value}
            for key, value in validation[label_column].astype(int).astype(str).value_counts().to_dict().items()
        ]
    )
    return comparison, distribution, baseline_predictions, augmented_predictions


def _filter_observed_rows(samples: pd.DataFrame, *, label_column: str, observed_column: str) -> pd.DataFrame:
    if samples.empty or label_column not in samples.columns:
        return pd.DataFrame(columns=samples.columns)
    eligible_mask = samples.get("is_continuation_eligible", pd.Series([True] * len(samples), index=samples.index))
    eligible_mask = eligible_mask.fillna(False).astype(bool)
    if observed_column in samples.columns:
        observed_mask = samples[observed_column].fillna(False).astype(bool)
    else:
        observed_mask = samples[label_column].notna()
    mask = eligible_mask & observed_mask & samples[label_column].notna()
    return samples.loc[mask].copy()


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


def _binary_balanced_accuracy(actual: pd.Series, predicted: pd.Series) -> float:
    actual_text = actual.astype(str)
    predicted_text = pd.Series(predicted, index=actual.index).astype(str)
    recalls: list[float] = []
    for label in ("0", "1"):
        label_mask = actual_text == label
        if not label_mask.any():
            recalls.append(0.5)
            continue
        recalls.append(float((predicted_text.loc[label_mask] == label).mean()))
    return float(sum(recalls) / len(recalls))
