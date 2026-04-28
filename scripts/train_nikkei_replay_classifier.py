"""
Minimal replay-classifier trainer for Nikkei replay samples.
"""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import pandas as pd
from sklearn.compose import ColumnTransformer
from sklearn.dummy import DummyClassifier
from sklearn.impute import SimpleImputer
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import accuracy_score
from sklearn.pipeline import Pipeline
from sklearn.preprocessing import OneHotEncoder


# 2026-04-28 CST: Added because the replay trainer must publish one stable
# target contract independent of future model upgrades. Purpose: keep the
# smoke-level trainer aligned with the governed replay label taxonomy.
TARGET_DEFINITION_VERSION = "nikkei_replay_label_v1"
REPLAY_LABEL_VOCABULARY = [
    "correct_reduce",
    "acceptable_reduce",
    "premature_reduce",
    "late_reduce",
    "correct_add",
    "acceptable_add",
    "premature_add",
    "late_add",
    "inconclusive",
]
LABEL_COLUMN_TEMPLATE = "replay_label_{label_horizon}"
LABEL_COLUMNS = {"replay_label_1d", "replay_label_3d", "replay_label_5d"}
PREFERRED_FEATURE_COLUMNS = [
    "sample_source",
    "signal_family",
    "signal_direction",
    "action_type",
    "base_position_v3",
    "rating_state",
    "dist_res20",
    "dist_sup20",
    "dist_sup60",
    "weighted_vol_down",
    "component_above200_breadth",
    "avg_component_vr",
    "horizon_1d_close_return",
    "horizon_3d_close_return",
    "horizon_5d_close_return",
]


# 2026-04-28 CST: Added because the smoke contract requires one callable
# training entrypoint with deterministic artifact outputs. Purpose: train a
# minimal multi-class replay model and emit machine-readable artifacts.
def train_replay_classifier(
    sample_path: str | Path,
    output_root: str | Path,
    label_horizon: str = "5d",
) -> dict[str, Any]:
    sample_path = Path(sample_path)
    output_root = Path(output_root)
    output_root.mkdir(parents=True, exist_ok=True)

    label_column = LABEL_COLUMN_TEMPLATE.format(label_horizon=label_horizon)
    samples = pd.read_csv(sample_path, encoding="utf-8-sig")
    if label_column not in samples.columns:
        raise ValueError(f"missing required label column: {label_column}")
    if samples.empty:
        raise ValueError("replay sample CSV cannot be empty")
    if "is_replay_outcome_observed" in samples.columns:
        observed_mask = samples["is_replay_outcome_observed"].fillna(False).astype(bool)
        observed_samples = samples.loc[observed_mask].copy()
        if observed_samples.empty:
            raise ValueError("no observed replay-outcome rows available for training")
    else:
        observed_mask = pd.Series([True] * len(samples), index=samples.index)
        observed_samples = samples.copy()

    feature_frame = _build_feature_frame(observed_samples, label_column)
    target = observed_samples[label_column].fillna("inconclusive").astype(str)
    split_names = _build_time_aware_splits(observed_samples)
    train_mask = split_names == "train"
    model, classifier_name, fallback_reason = _fit_classifier(
        feature_frame.loc[train_mask],
        target.loc[train_mask],
    )
    predictions = model.predict(feature_frame)
    probabilities = _predict_probabilities(model, feature_frame)
    train_accuracy = float(accuracy_score(target.loc[train_mask], predictions[train_mask]))
    validation_mask = split_names == "validation"
    validation_accuracy = (
        float(accuracy_score(target.loc[validation_mask], predictions[validation_mask]))
        if validation_mask.any()
        else float("nan")
    )

    predictions_frame = pd.DataFrame(
        {
            "sample_id": observed_samples.get(
                "sample_id",
                pd.Series(range(len(observed_samples)), index=observed_samples.index),
            ).astype(str),
            "signal_date": observed_samples.get("signal_date", pd.Series([""] * len(observed_samples))).astype(str),
            "data_split": split_names,
            "actual_label": target,
            "predicted_label": predictions,
        }
    )
    for class_name, column_values in probabilities.items():
        predictions_frame[f"prob_{class_name}"] = column_values

    metrics_frame = pd.DataFrame(
        [
            {"metric_name": "sample_count", "metric_value": float(len(observed_samples))},
            {"metric_name": "train_sample_count", "metric_value": float(train_mask.sum())},
            {"metric_name": "validation_sample_count", "metric_value": float(validation_mask.sum())},
            {"metric_name": "observed_label_count", "metric_value": float(target.nunique())},
            {"metric_name": "train_accuracy", "metric_value": train_accuracy},
            {"metric_name": "validation_accuracy", "metric_value": validation_accuracy},
            {"metric_name": "used_dummy_fallback", "metric_value": float(classifier_name == "DummyClassifier")},
        ]
    )
    label_counts_frame = (
        target.value_counts(dropna=False)
        .rename_axis("label_name")
        .reset_index(name="sample_count")
    )

    summary = {
        "target_definition_version": TARGET_DEFINITION_VERSION,
        "label_horizon": label_horizon,
        "label_column": label_column,
        "label_vocabulary": REPLAY_LABEL_VOCABULARY,
        "sample_count": int(len(observed_samples)),
        "observed_outcome_sample_count": int(len(observed_samples)),
        "train_sample_count": int(train_mask.sum()),
        "validation_sample_count": int(validation_mask.sum()),
        "observed_label_count": int(target.nunique()),
        "observed_labels": sorted(target.unique().tolist()),
        "classifier_name": classifier_name,
        "fallback_reason": fallback_reason,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "train_accuracy": train_accuracy,
        "validation_accuracy": validation_accuracy,
    }

    metrics_path = output_root / "03_replay_classifier_metrics.csv"
    predictions_path = output_root / "04_replay_classifier_predictions.csv"
    label_counts_path = output_root / "05_replay_label_counts.csv"
    confusion_path = output_root / "06_replay_confusion_matrix.csv"
    training_summary_path = output_root / "training_summary.json"
    confusion_frame = pd.crosstab(
        predictions_frame["actual_label"],
        predictions_frame["predicted_label"],
    ).reset_index()
    metrics_frame.to_csv(metrics_path, index=False, encoding="utf-8-sig")
    predictions_frame.to_csv(predictions_path, index=False, encoding="utf-8-sig")
    label_counts_frame.to_csv(label_counts_path, index=False, encoding="utf-8-sig")
    confusion_frame.to_csv(confusion_path, index=False, encoding="utf-8-sig")
    training_summary_path.write_text(
        json.dumps(summary, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )

    return {
        "target_definition_version": TARGET_DEFINITION_VERSION,
        "label_vocabulary": REPLAY_LABEL_VOCABULARY,
        "metrics_path": str(metrics_path),
        "predictions_path": str(predictions_path),
        "label_counts_path": str(label_counts_path),
        "confusion_path": str(confusion_path),
        "training_summary_path": str(training_summary_path),
        "classifier_name": classifier_name,
        "fallback_reason": fallback_reason,
    }


# 2026-04-28 CST: Added because replay samples mix categorical descriptors and
# numeric market features, while the smoke contract only guarantees a CSV row
# set. Purpose: derive one robust feature matrix without requiring extra schema files.
def _build_feature_frame(samples: pd.DataFrame, label_column: str) -> pd.DataFrame:
    feature_columns = [
        column for column in PREFERRED_FEATURE_COLUMNS if column in samples.columns and column != label_column
    ]
    feature_frame = samples[feature_columns].copy()
    feature_frame = feature_frame.dropna(axis=1, how="all")
    if feature_frame.empty:
        feature_frame = pd.DataFrame({"constant_feature": [1.0] * len(samples)})
    return feature_frame


# 2026-04-28 CST: Added because replay metrics should not be reported from a
# single in-sample fit only. Purpose: assign a simple time-aware train/validation
# split using signal_date order before the first classifier summary is exported.
def _build_time_aware_splits(samples: pd.DataFrame) -> pd.Series:
    signal_dates = pd.to_datetime(samples.get("signal_date"), errors="coerce")
    if signal_dates.notna().sum() < 5:
        return pd.Series(["train"] * len(samples), index=samples.index, dtype="string")
    ordered = signal_dates.sort_values(kind="mergesort")
    cutoff_position = max(int(len(ordered) * 0.8), 1)
    cutoff_date = ordered.iloc[cutoff_position - 1]
    split_names = pd.Series("validation", index=samples.index, dtype="string")
    split_names.loc[signal_dates <= cutoff_date] = "train"
    if (split_names == "validation").sum() == 0:
        split_names.iloc[-1] = "validation"
    return split_names


# 2026-04-28 CST: Added because LogisticRegression is the preferred minimal
# multi-class learner, but the smoke contract uses a two-row dataset that can be
# too small or degenerate. Purpose: try LogisticRegression first and fall back to
# DummyClassifier when class count or solver constraints block training.
def _fit_classifier(
    features: pd.DataFrame,
    target: pd.Series,
) -> tuple[Pipeline, str, str | None]:
    numeric_columns = features.select_dtypes(include=["number", "bool"]).columns.tolist()
    categorical_columns = [column for column in features.columns if column not in numeric_columns]

    preprocess = ColumnTransformer(
        transformers=[
            (
                "numeric",
                Pipeline(
                    steps=[
                        ("imputer", SimpleImputer(strategy="constant", fill_value=0.0)),
                    ]
                ),
                numeric_columns,
            ),
            (
                "categorical",
                Pipeline(
                    steps=[
                        ("imputer", SimpleImputer(strategy="most_frequent")),
                        ("encoder", OneHotEncoder(handle_unknown="ignore")),
                    ]
                ),
                categorical_columns,
            ),
        ],
        remainder="drop",
    )

    if target.nunique() < 2:
        dummy_model = Pipeline(
            steps=[
                ("preprocess", preprocess),
                ("classifier", DummyClassifier(strategy="most_frequent")),
            ]
        )
        dummy_model.fit(features, target)
        return dummy_model, "DummyClassifier", "insufficient_distinct_labels"

    logistic_model = Pipeline(
        steps=[
            ("preprocess", preprocess),
            (
                "classifier",
                LogisticRegression(
                    max_iter=5000,
                    solver="lbfgs",
                    class_weight="balanced",
                ),
            ),
        ]
    )
    try:
        logistic_model.fit(features, target)
        return logistic_model, "LogisticRegression", None
    except Exception as error:
        dummy_model = Pipeline(
            steps=[
                ("preprocess", preprocess),
                ("classifier", DummyClassifier(strategy="most_frequent")),
            ]
        )
        dummy_model.fit(features, target)
        return dummy_model, "DummyClassifier", f"logistic_regression_failed:{type(error).__name__}"


# 2026-04-28 CST: Added because the smoke trainer should publish per-class
# probabilities into predictions.csv when the classifier supports them.
# Purpose: keep the first artifact set useful for later replay inspection.
def _predict_probabilities(model: Pipeline, features: pd.DataFrame) -> dict[str, list[float]]:
    classifier = model.named_steps["classifier"]
    if not hasattr(classifier, "predict_proba"):
        return {}

    probabilities = model.predict_proba(features)
    class_names = [str(label) for label in classifier.classes_]
    probability_map: dict[str, list[float]] = {}
    for class_name in REPLAY_LABEL_VOCABULARY:
        if class_name in class_names:
            class_index = class_names.index(class_name)
            probability_map[class_name] = probabilities[:, class_index].tolist()
        else:
            probability_map[class_name] = [0.0] * len(features)
    return probability_map
