#!/usr/bin/env python
# 2026-04-28 CST: Added because the approved next step is to test whether
# machine learning can improve the current gold mean-reversion hold_15d rule.
# Purpose: compare HGB, RF, and XGBoost against the fixed rule baseline under
# expanding-window main validation and 5-year rolling-window robustness checks.

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import sys

import numpy as np
import pandas as pd
from sklearn.ensemble import HistGradientBoostingClassifier, RandomForestClassifier
from sklearn.impute import SimpleImputer
from sklearn.metrics import accuracy_score, average_precision_score, brier_score_loss, precision_score, recall_score, roc_auc_score
from xgboost import XGBClassifier


DEFAULT_OUTPUT_ROOT = Path(r"E:\SM\docs\research\gold_ml_hold15d_experiment_20260428")
DEFAULT_START_DATE = "2016-01-01"
DEFAULT_END_DATE = "2026-04-28"
PARENT_RET_THRESHOLD = -0.02
PARENT_MA20_THRESHOLD = -0.015


def load_grid_module():
    module_path = Path(r"E:\SM\scripts\research\gold_mean_reversion_entry_grid_v3.py")
    spec = importlib.util.spec_from_file_location("gold_mean_reversion_entry_grid_v3", module_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


GRID_MODULE = load_grid_module()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--start-date", default=DEFAULT_START_DATE)
    parser.add_argument("--end-date", default=DEFAULT_END_DATE)
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    return parser.parse_args()


def get_feature_columns() -> list[str]:
    return [
        "ret_3d",
        "ret_5d",
        "ret_10d",
        "close_vs_ma10",
        "close_vs_ma20",
        "pullback_depth_from_high",
        "volume_ratio_1d_vs_20d",
        "avg_volume_5d",
        "avg_volume_20d",
        "usd_ret_lb",
        "oil_ret_lb",
        "high_volatility_shock_flag",
        "downtrend_break_flag",
    ]


def build_rule_event_dataset(frame: pd.DataFrame, feature_columns: list[str]) -> pd.DataFrame:
    sample = frame[
        (frame["ret_5d"] <= PARENT_RET_THRESHOLD)
        & (frame["close_vs_ma20"] <= PARENT_MA20_THRESHOLD)
        & frame["future_ret_15d"].notna()
    ].copy()
    sample["label"] = (sample["future_ret_15d"] > 0).astype(int)
    sample["year"] = pd.to_datetime(sample["trade_date"]).dt.year
    keep_cols = ["trade_date", "year", "future_ret_15d", "hold_max_drawdown_15d", "hold_max_runup_15d", "label"] + feature_columns
    for column in keep_cols:
        if column not in sample.columns:
            sample[column] = np.nan
    return sample[keep_cols].reset_index(drop=True)


def build_walk_forward_splits(
    dataset: pd.DataFrame,
    mode: str,
    min_train_year: int = 2016,
    first_valid_year: int = 2021,
) -> list[dict[str, object]]:
    max_year = int(dataset["year"].max())
    splits = []
    for valid_year in range(first_valid_year, max_year + 1):
        if mode == "expanding":
            train_years = list(range(min_train_year, valid_year))
        elif mode == "rolling_5y":
            train_years = list(range(valid_year - 5, valid_year))
        else:
            raise ValueError(f"Unsupported mode: {mode}")
        splits.append({"train_years": train_years, "valid_year": valid_year})
    return splits


def build_model_registry() -> dict[str, object]:
    return {
        "hgb": HistGradientBoostingClassifier(
            learning_rate=0.05,
            max_depth=3,
            max_iter=200,
            min_samples_leaf=5,
            random_state=42,
        ),
        "rf": RandomForestClassifier(
            n_estimators=300,
            max_depth=4,
            min_samples_leaf=5,
            random_state=42,
        ),
        "xgb": XGBClassifier(
            n_estimators=300,
            max_depth=3,
            learning_rate=0.05,
            min_child_weight=5,
            subsample=1.0,
            colsample_bytree=1.0,
            reg_lambda=1.0,
            objective="binary:logistic",
            eval_metric="logloss",
            random_state=42,
        ),
    }


def train_and_score_split(
    dataset: pd.DataFrame,
    feature_columns: list[str],
    split: dict[str, object],
    model_registry: dict[str, object],
    split_mode: str,
) -> tuple[pd.DataFrame, pd.DataFrame]:
    train = dataset[dataset["year"].isin(split["train_years"])].copy()
    valid = dataset[dataset["year"] == split["valid_year"]].copy()
    if train.empty or valid.empty or train["label"].nunique() < 2:
        return pd.DataFrame(), pd.DataFrame()

    imputer = SimpleImputer(strategy="median")
    x_train = imputer.fit_transform(train[feature_columns])
    x_valid = imputer.transform(valid[feature_columns])
    y_train = train["label"].to_numpy()
    y_valid = valid["label"].to_numpy()

    prediction_rows = []
    metric_rows = []

    baseline = valid.copy()
    baseline["model_name"] = "rule_baseline"
    baseline["score"] = 1.0
    baseline["selection_policy"] = "all"
    baseline["selected_flag"] = 1
    baseline["split_mode"] = split_mode
    baseline["valid_year"] = split["valid_year"]
    prediction_rows.append(baseline)

    for model_name, model in model_registry.items():
        fitted = model.fit(x_train, y_train)
        scores = fitted.predict_proba(x_valid)[:, 1]
        preds = (scores >= 0.5).astype(int)
        metric_rows.append(
            {
                "split_mode": split_mode,
                "valid_year": split["valid_year"],
                "model_name": model_name,
                "auc": roc_auc_score(y_valid, scores) if len(np.unique(y_valid)) > 1 else np.nan,
                "pr_auc": average_precision_score(y_valid, scores),
                "accuracy": accuracy_score(y_valid, preds),
                "precision": precision_score(y_valid, preds, zero_division=0),
                "recall": recall_score(y_valid, preds, zero_division=0),
                "brier": brier_score_loss(y_valid, scores),
                "train_count": int(len(train)),
                "valid_count": int(len(valid)),
            }
        )
        for selection_policy, top_pct in [("top_50pct", 0.5), ("top_30pct", 0.3)]:
            scored = valid.copy()
            scored["model_name"] = model_name
            scored["score"] = scores
            scored["selection_policy"] = selection_policy
            scored["split_mode"] = split_mode
            scored["valid_year"] = split["valid_year"]
            threshold_rank = max(int(np.ceil(len(scored) * top_pct)), 1)
            top_index = scored.sort_values("score", ascending=False).head(threshold_rank).index
            scored["selected_flag"] = 0
            scored.loc[top_index, "selected_flag"] = 1
            prediction_rows.append(scored)

    return pd.concat(prediction_rows, ignore_index=True), pd.DataFrame(metric_rows)


def summarize_model_strategy(predictions: pd.DataFrame) -> pd.DataFrame:
    rows = []
    selected = predictions[predictions["selected_flag"] == 1].copy()
    if "split_mode" not in selected.columns:
        selected["split_mode"] = "unknown"
    if "trade_date" not in selected.columns:
        selected["trade_date"] = pd.Timestamp("2025-01-01")
    if "hold_max_drawdown_15d" not in selected.columns:
        selected["hold_max_drawdown_15d"] = np.nan
    if "hold_max_runup_15d" not in selected.columns:
        selected["hold_max_runup_15d"] = np.nan
    grouped = selected.groupby(["split_mode", "model_name", "selection_policy"], dropna=False)
    for (split_mode, model_name, selection_policy), subset in grouped:
        years = int(subset["valid_year"].nunique()) if "valid_year" in subset.columns else int(pd.to_datetime(subset["trade_date"]).dt.year.nunique())
        rows.append(
            {
                "split_mode": split_mode,
                "model_name": model_name,
                "selection_policy": selection_policy,
                "sample_count": int(len(subset)),
                "years_covered": years,
                "events_per_year": len(subset) / years if years > 0 else 0.0,
                "win_rate": float((subset["future_ret_15d"] > 0).mean()),
                "avg_return": float(subset["future_ret_15d"].mean()),
                "median_return": float(subset["future_ret_15d"].median()),
                "avg_max_drawdown": float(subset["hold_max_drawdown_15d"].mean()),
                "avg_max_runup": float(subset["hold_max_runup_15d"].mean()),
                "positive_year_ratio": float(
                    subset.assign(year=pd.to_datetime(subset["trade_date"]).dt.year)
                    .groupby("year")["future_ret_15d"]
                    .mean()
                    .gt(0)
                    .mean()
                ),
            }
        )
    return pd.DataFrame(rows)


def extract_model_winner(summary: pd.DataFrame) -> dict[str, object]:
    eligible = summary[
        (summary["sample_count"] >= 20)
        & (summary["positive_year_ratio"] >= 0.70)
        & (summary["median_return"] > 0)
    ].copy()
    if eligible.empty:
        return {}
    ranked = eligible.sort_values(
        ["avg_return", "win_rate", "median_return", "positive_year_ratio"],
        ascending=[False, False, False, False],
    )
    return ranked.iloc[0].to_dict()


def prepare_base_frame(start_date: str, end_date: str) -> pd.DataFrame:
    base_frame, _ = GRID_MODULE.V2_MODULE.BROAD_MODULE.RULE_MODULE.BASE_MODULE.prepare_analysis_frame(start_date, end_date)
    base_frame = GRID_MODULE.V2_MODULE.BROAD_MODULE.RULE_MODULE.build_failure_flags(
        GRID_MODULE.V2_MODULE.BROAD_MODULE.RULE_MODULE.assign_bucket_labels(base_frame)
    )
    return base_frame


def run_experiment(start_date: str, end_date: str) -> tuple[pd.DataFrame, pd.DataFrame, pd.DataFrame]:
    base_frame = prepare_base_frame(start_date, end_date)
    feature_columns = get_feature_columns()
    dataset = build_rule_event_dataset(base_frame, feature_columns)
    model_registry = build_model_registry()

    prediction_frames = []
    metric_frames = []
    for split_mode in ["expanding", "rolling_5y"]:
        splits = build_walk_forward_splits(dataset, mode=split_mode, min_train_year=2016, first_valid_year=2021)
        for split in splits:
            preds, metrics = train_and_score_split(dataset, feature_columns, split, model_registry, split_mode)
            if not preds.empty:
                prediction_frames.append(preds)
            if not metrics.empty:
                metric_frames.append(metrics)

    predictions = pd.concat(prediction_frames, ignore_index=True) if prediction_frames else pd.DataFrame()
    metrics = pd.concat(metric_frames, ignore_index=True) if metric_frames else pd.DataFrame()
    strategy_summary = summarize_model_strategy(predictions)
    return dataset, predictions, metrics.merge(strategy_summary, on=["split_mode", "model_name"], how="left") if not metrics.empty else metrics


def main() -> int:
    args = parse_args()
    base_frame = prepare_base_frame(args.start_date, args.end_date)
    feature_columns = get_feature_columns()
    dataset = build_rule_event_dataset(base_frame, feature_columns)
    model_registry = build_model_registry()

    prediction_frames = []
    metric_frames = []
    for split_mode in ["expanding", "rolling_5y"]:
        splits = build_walk_forward_splits(dataset, mode=split_mode, min_train_year=2016, first_valid_year=2021)
        for split in splits:
            preds, metrics = train_and_score_split(dataset, feature_columns, split, model_registry, split_mode)
            if not preds.empty:
                prediction_frames.append(preds)
            if not metrics.empty:
                metric_frames.append(metrics)

    predictions = pd.concat(prediction_frames, ignore_index=True) if prediction_frames else pd.DataFrame()
    metrics = pd.concat(metric_frames, ignore_index=True) if metric_frames else pd.DataFrame()
    strategy_summary = summarize_model_strategy(predictions)
    winner = extract_model_winner(strategy_summary)

    output_root = Path(args.output_root)
    output_root.mkdir(parents=True, exist_ok=True)
    dataset.to_csv(output_root / "gold_ml_hold15d_dataset.csv", index=False, encoding="utf-8-sig")
    predictions.to_csv(output_root / "gold_ml_hold15d_predictions.csv", index=False, encoding="utf-8-sig")
    metrics.to_csv(output_root / "gold_ml_hold15d_model_metrics.csv", index=False, encoding="utf-8-sig")
    strategy_summary.to_csv(output_root / "gold_ml_hold15d_strategy_summary.csv", index=False, encoding="utf-8-sig")

    payload = {
        "dataset_rows": int(len(dataset)),
        "class_balance": float(dataset["label"].mean()) if not dataset.empty else None,
        "winner": winner,
        "top_strategies": strategy_summary.sort_values(
            ["avg_return", "win_rate", "median_return", "positive_year_ratio"],
            ascending=[False, False, False, False],
        ).head(12).to_dict(orient="records"),
    }
    (output_root / "summary.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(payload, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
