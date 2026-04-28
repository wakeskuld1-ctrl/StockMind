#!/usr/bin/env python
"""
Daily Nikkei V3 model scoring research utility.

2026-04-27 CST: Added because the HGB/RF enhanced V3 daily process needs a
repeatable no-future-label scorer before it is promoted into the formal Tool.
Purpose: rebuild live features through an as-of date, train leak-safe HGB and RF
models from known historical adjustment labels, emit daily recommendations and
Tool-consumable adjustment artifacts for review.
"""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

import numpy as np
import pandas as pd
from sklearn.ensemble import HistGradientBoostingClassifier, RandomForestClassifier
from sklearn.inspection import permutation_importance
from sklearn.metrics import accuracy_score, balanced_accuracy_score


DEFAULT_ANALYSIS_ROOT = Path(
    r"D:\.stockmind_runtime\nikkei_current_rerun_20260426_direction_head_yfinance_10y_long_volume_behavior\analysis_exports"
)
DEFAULT_ADJUSTMENT_ROOT = DEFAULT_ANALYSIS_ROOT / "adjustment_point_analysis"
DEFAULT_OUTPUT_ROOT = Path(r"D:\.stockmind_runtime\nikkei_etf_daily_model_scoring_20260427")
CONTRACT_VERSION = "nikkei_v3_hgb_adjustment.v1"
MODEL_SET_VERSION = "research_daily_hgb_rf_v3_20260427"
LABEL_HORIZON_TRADING_DAYS = 20


@dataclass(frozen=True)
class ModelSpec:
    name: str
    estimator: object


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Score daily Nikkei V3 HGB/RF model recommendations.")
    parser.add_argument("--as-of-date", default="2026-04-24")
    parser.add_argument("--score-start-date", default="2026-03-28")
    parser.add_argument("--analysis-root", default=str(DEFAULT_ANALYSIS_ROOT))
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    parser.add_argument(
        "--train-policy",
        choices=["live_pre_year", "known_labels_asof"],
        default="live_pre_year",
        help=(
            "live_pre_year follows the prior candidate split: train before Q4 of the "
            "previous year and validate on that Q4; known_labels_asof is diagnostic "
            "and includes labels whose 20D horizon has completed by as_of_date."
        ),
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    run_daily_scoring(
        as_of_date=args.as_of_date,
        score_start_date=args.score_start_date,
        analysis_root=args.analysis_root,
        output_root=args.output_root,
        train_policy=args.train_policy,
    )


# 2026-04-28 CST: Added to let the governed daily workflow call the scorer
# directly without rebuilding CLI argv strings. Purpose: keep one execution path
# for CLI runs and scripted live_pre_year daily workflow runs.
def run_daily_scoring(
    *,
    as_of_date: str,
    score_start_date: str,
    analysis_root: str | Path,
    output_root: str | Path,
    train_policy: str,
) -> dict[str, object]:
    analysis_root = Path(analysis_root)
    adjustment_root = analysis_root / "adjustment_point_analysis"
    output_root = Path(output_root)
    output_root.mkdir(parents=True, exist_ok=True)

    as_of_date = pd.Timestamp(as_of_date)
    score_start_date = pd.Timestamp(score_start_date)
    training_frame = load_training_frame(adjustment_root)
    features = feature_columns(training_frame)
    train_frame = select_training_rows(training_frame, as_of_date, train_policy)
    score_frame = build_live_feature_frame(
        analysis_root=analysis_root,
        adjustment_root=adjustment_root,
        as_of_date=as_of_date,
        score_start_date=score_start_date,
        expected_features=features,
    )

    model_specs = [
        ModelSpec(
            name="hgb_l2_leaf20_live",
            estimator=HistGradientBoostingClassifier(
                max_leaf_nodes=20,
                l2_regularization=1.0,
                learning_rate=0.1,
                random_state=42,
            ),
        ),
        ModelSpec(
            name="rf_depth4_leaf20_live",
            estimator=RandomForestClassifier(
                n_estimators=500,
                max_depth=4,
                min_samples_leaf=20,
                class_weight="balanced_subsample",
                random_state=42,
                n_jobs=1,
            ),
        ),
    ]

    scored_frames = []
    metrics_rows = []
    importance_rows = []
    driver_rows = []
    artifact_rows = []
    for spec in model_specs:
        model = spec.estimator
        model.fit(train_frame[features], train_frame["adjustment_label"])
        scored = score_model(spec.name, model, score_frame, features)
        scored_frames.append(scored)
        metrics_rows.append(
            evaluate_model(
                spec.name,
                model,
                training_frame,
                train_frame,
                features,
                as_of_date,
                train_policy,
            )
        )
        importance_rows.extend(
            compute_importance_rows(
                spec.name,
                model,
                training_frame,
                train_frame,
                features,
                as_of_date,
                train_policy,
            )
        )
        driver_rows.extend(compute_driver_rows(spec.name, scored, train_frame, importance_rows, features))
        artifact_rows.append(build_latest_artifact_row(spec.name, scored, as_of_date, train_policy))

    all_scores = pd.concat(scored_frames, ignore_index=True)
    metrics = pd.DataFrame(metrics_rows)
    importances = pd.DataFrame(importance_rows).sort_values(["model", "importance"], ascending=[True, False])
    drivers = pd.DataFrame(driver_rows)
    artifacts = pd.DataFrame(artifact_rows)

    manifest = write_outputs(
        output_root,
        all_scores,
        metrics,
        importances,
        drivers,
        artifacts,
        as_of_date,
        train_policy,
        score_start_date=score_start_date,
        analysis_root=analysis_root,
    )
    print_summary(all_scores, metrics, artifacts, output_root)
    return manifest


def load_training_frame(adjustment_root: Path) -> pd.DataFrame:
    path = adjustment_root / "55_v3_adjustment_model_dataset.csv"
    frame = pd.read_csv(path, parse_dates=["date"])
    frame = frame.sort_values("date").reset_index(drop=True)
    return frame


def feature_columns(frame: pd.DataFrame) -> list[str]:
    excluded = {
        "date",
        "close",
        "market_regime",
        "future20_return",
        "future20_drawdown",
        "adjustment_label",
    }
    return [column for column in frame.columns if column not in excluded]


def select_training_rows(frame: pd.DataFrame, as_of_date: pd.Timestamp, policy: str) -> pd.DataFrame:
    if policy == "live_pre_year":
        # 2026-04-27 CST: Corrected after the first research run exposed in-sample
        # validation leakage. Reason: the prior candidate process used previous Q4
        # as validation, so live scoring must not train on that quarter.
        # Purpose: keep daily research scoring aligned with the approved audit split.
        validation_start = pd.Timestamp(year=as_of_date.year - 1, month=10, day=1)
        train = frame[frame["date"] < validation_start].copy()
    else:
        cutoff_index = frame.index[frame["date"] <= as_of_date]
        if cutoff_index.empty:
            raise ValueError(f"as_of_date {as_of_date.date()} is before the training frame")
        asof_pos = int(cutoff_index.max())
        max_label_pos = max(-1, asof_pos - LABEL_HORIZON_TRADING_DAYS)
        train = frame.iloc[: max_label_pos + 1].copy()
    if train.empty:
        raise ValueError(f"no training rows selected for policy={policy}")
    return train.dropna(subset=feature_columns(frame) + ["adjustment_label"]).copy()


def build_live_feature_frame(
    analysis_root: Path,
    adjustment_root: Path,
    as_of_date: pd.Timestamp,
    score_start_date: pd.Timestamp,
    expected_features: list[str],
) -> pd.DataFrame:
    n225 = load_nikkei_daily(analysis_root)
    n225 = add_nikkei_features(n225)
    v3_base = load_v3_base_position(adjustment_root)
    component_breadth = build_component_breadth(adjustment_root)
    frame = n225.merge(v3_base, on="date", how="left").merge(component_breadth, on="date", how="left")
    fill_component_gaps(frame)
    frame = frame[frame["date"].between(score_start_date, as_of_date)].copy()
    missing = [column for column in expected_features if column not in frame.columns]
    if missing:
        raise ValueError(f"live feature frame missing columns: {missing}")
    frame = frame.dropna(subset=expected_features).copy()
    return frame


def load_nikkei_daily(analysis_root: Path) -> pd.DataFrame:
    path = analysis_root / "11_stock_history_NK225_VOL_YFINANCE.csv"
    frame = pd.read_csv(path, parse_dates=["trade_date"])
    frame = frame.rename(columns={"trade_date": "date"})
    return frame[["date", "open", "high", "low", "close", "volume"]].sort_values("date").reset_index(drop=True)


def add_nikkei_features(frame: pd.DataFrame) -> pd.DataFrame:
    result = frame.copy()
    for window in [1, 3, 5, 10, 20]:
        result[f"ret{window}"] = result["close"].pct_change(window)
    for window in [20, 50, 200]:
        result[f"ma{window}"] = result["close"].rolling(window, min_periods=window).mean()
    result["dist_ma20"] = result["close"] / result["ma20"] - 1
    result["dist_ma50"] = result["close"] / result["ma50"] - 1
    result["dist_ma200"] = result["close"] / result["ma200"] - 1
    result["ma50_over_ma200"] = result["ma50"] / result["ma200"] - 1
    result["ma200_slope20"] = result["ma200"] / result["ma200"].shift(20) - 1
    result["volume_ratio60"] = result["volume"] / result["volume"].shift(1).rolling(60, min_periods=20).mean()
    for window in [20, 60]:
        result[f"prior_high{window}"] = result["close"].shift(1).rolling(window, min_periods=window).max()
        result[f"prior_low{window}"] = result["close"].shift(1).rolling(window, min_periods=window).min()
        result[f"breakout{window}"] = (result["close"] > result[f"prior_high{window}"]).astype(int)
        result[f"breakdown{window}"] = (result["close"] < result[f"prior_low{window}"]).astype(int)
    result["below200"] = (result["close"] < result["ma200"]).astype(int)
    result["dist_res20"] = result["close"] / result["prior_high20"] - 1
    result["dist_sup20"] = result["close"] / result["prior_low20"] - 1
    result["dist_sup60"] = result["close"] / result["prior_low60"] - 1
    result["regime_bull"] = (
        (result["close"] > result["ma200"])
        & (result["ma50"] > result["ma200"])
        & (result["ma200_slope20"] > 0)
    ).astype(int)
    result["regime_bear"] = (
        (result["close"] < result["ma200"])
        & (result["ma50"] < result["ma200"])
        & (result["ma200_slope20"] < 0)
    ).astype(int)
    result["regime_range"] = ((result["regime_bull"] == 0) & (result["regime_bear"] == 0)).astype(int)
    return result


def load_v3_base_position(adjustment_root: Path) -> pd.DataFrame:
    path = adjustment_root / "61_V3_base_2022_2026_curve.csv"
    frame = pd.read_csv(path, parse_dates=["date"])
    return frame[["date", "position"]].rename(columns={"position": "base_position_v3"})


def build_component_breadth(adjustment_root: Path) -> pd.DataFrame:
    weights = pd.read_csv(adjustment_root / "15_nikkei_top30_official_weights.csv")
    weights["normalized_top30_weight"] = weights["official_weight"] / weights["official_weight"].sum()
    raw_dir = adjustment_root / "component_yfinance_raw_top30_retry"
    component_frames = []
    for _, row in weights.iterrows():
        ticker = str(row["ticker_yahoo"])
        path = raw_dir / f"{ticker.replace('.', '_')}.csv"
        if not path.exists():
            continue
        component = pd.read_csv(path, parse_dates=["Date"])
        if component.empty:
            continue
        component = normalize_component_history(component, float(row["normalized_top30_weight"]))
        component_frames.append(
            component[
                [
                    "date",
                    "weight",
                    "component_b20_vol",
                    "component_b60_vol",
                    "component_volume_down",
                    "component_bd20_vol",
                    "component_above200",
                    "volume_ratio_60d",
                ]
            ]
        )
    if not component_frames:
        raise ValueError("no component history files were available")
    combined = pd.concat(component_frames, ignore_index=True)
    aggregated = combined.groupby("date").apply(component_aggregate_row).reset_index()
    return aggregated


def normalize_component_history(component: pd.DataFrame, weight: float) -> pd.DataFrame:
    result = component.rename(columns={"Date": "date"}).copy()
    result["date"] = result["date"].dt.tz_localize(None).dt.normalize()
    result = result.sort_values("date").reset_index(drop=True)
    result["weight"] = weight
    result["volume_ratio_60d"] = result["Volume"] / result["Volume"].shift(1).rolling(60, min_periods=20).mean()
    for window in [20, 60, 200]:
        result[f"prior_high{window}"] = result["Close"].shift(1).rolling(window, min_periods=window).max()
        result[f"prior_low{window}"] = result["Close"].shift(1).rolling(window, min_periods=window).min()
        result[f"ma{window}"] = result["Close"].rolling(window, min_periods=window).mean()
    result["component_breakout20"] = (result["Close"] > result["prior_high20"]).astype(int)
    result["component_breakout60"] = (result["Close"] > result["prior_high60"]).astype(int)
    result["component_breakdown20"] = (result["Close"] < result["prior_low20"]).astype(int)
    result["component_volume_confirmed"] = (result["volume_ratio_60d"] >= 1.2).astype(int)
    result["component_1d_return"] = result["Close"].pct_change(1)
    result["component_volume_down"] = (
        (result["component_1d_return"] < 0) & (result["volume_ratio_60d"] >= 1.2)
    ).astype(int)
    result["component_above200"] = (result["Close"] > result["ma200"]).astype(int)
    result["component_b20_vol"] = (
        (result["component_breakout20"] == 1) & (result["component_volume_confirmed"] == 1)
    ).astype(int)
    result["component_b60_vol"] = (
        (result["component_breakout60"] == 1) & (result["component_volume_confirmed"] == 1)
    ).astype(int)
    result["component_bd20_vol"] = (
        (result["component_breakdown20"] == 1) & (result["component_volume_confirmed"] == 1)
    ).astype(int)
    return result


def component_aggregate_row(group: pd.DataFrame) -> pd.Series:
    return pd.Series(
        {
            "component_b20_vol_count": int(group["component_b20_vol"].sum()),
            "component_b60_vol_count": int(group["component_b60_vol"].sum()),
            "component_vol_down_count": int(group["component_volume_down"].sum()),
            "component_bd20_vol_count": int(group["component_bd20_vol"].sum()),
            "weighted_b20_vol": float((group["component_b20_vol"] * group["weight"]).sum()),
            "weighted_b60_vol": float((group["component_b60_vol"] * group["weight"]).sum()),
            "weighted_vol_down": float((group["component_volume_down"] * group["weight"]).sum()),
            "weighted_bd20_vol": float((group["component_bd20_vol"] * group["weight"]).sum()),
            "component_above200_breadth": float((group["component_above200"] * group["weight"]).sum()),
            "avg_component_vr": float(group["volume_ratio_60d"].mean()),
        }
    )


def fill_component_gaps(frame: pd.DataFrame) -> None:
    zero_columns = [
        "component_b20_vol_count",
        "component_b60_vol_count",
        "component_vol_down_count",
        "component_bd20_vol_count",
        "weighted_b20_vol",
        "weighted_b60_vol",
        "weighted_vol_down",
        "weighted_bd20_vol",
    ]
    for column in zero_columns:
        frame[column] = frame[column].fillna(0)
    for column in ["component_above200_breadth", "avg_component_vr"]:
        frame[column] = frame[column].fillna(frame[column].median())


def score_model(model_name: str, model: object, frame: pd.DataFrame, features: list[str]) -> pd.DataFrame:
    result = frame.copy()
    probabilities = model.predict_proba(result[features])
    classes = [int(label) for label in model.classes_]
    result["model"] = model_name
    result["pred_adjustment"] = model.predict(result[features]).astype(int)
    for index, label in enumerate(classes):
        result[f"prob_{label}"] = probabilities[:, index]
    for label in [-1, 0, 1]:
        column = f"prob_{label}"
        if column not in result.columns:
            result[column] = 0.0
    result["target_position_proxy"] = (result["base_position_v3"] + 0.25 * result["pred_adjustment"]).clip(0, 1)
    result["action_vs_35pct"] = np.select(
        [
            result["target_position_proxy"] >= 0.50,
            result["target_position_proxy"] <= 0.25,
        ],
        ["buy_or_add", "reduce_or_hold_low"],
        default="hold_around_35pct",
    )
    return result


def evaluate_model(
    model_name: str,
    model: object,
    full_frame: pd.DataFrame,
    train_frame: pd.DataFrame,
    features: list[str],
    as_of_date: pd.Timestamp,
    train_policy: str,
) -> dict[str, object]:
    validation, validation_basis = select_validation_rows(
        full_frame, train_frame, as_of_date, train_policy
    )
    if validation.empty:
        validation = train_frame.tail(min(len(train_frame), 252)).copy()
        validation_basis = "fallback_in_sample_diagnostic_only"
    predictions = model.predict(validation[features])
    return {
        "model": model_name,
        "validation_basis": validation_basis,
        "train_start": train_frame["date"].min().date().isoformat(),
        "train_end": train_frame["date"].max().date().isoformat(),
        "train_rows": int(len(train_frame)),
        "validation_start": validation["date"].min().date().isoformat(),
        "validation_end": validation["date"].max().date().isoformat(),
        "validation_rows": int(len(validation)),
        "validation_accuracy": float(accuracy_score(validation["adjustment_label"], predictions)),
        "validation_balanced_accuracy": float(
            balanced_accuracy_score(validation["adjustment_label"], predictions)
        ),
        "validation_pred_counts": json.dumps(
            {str(int(k)): int(v) for k, v in pd.Series(predictions).value_counts().to_dict().items()},
            ensure_ascii=False,
        ),
    }


def compute_importance_rows(
    model_name: str,
    model: object,
    full_frame: pd.DataFrame,
    train_frame: pd.DataFrame,
    features: list[str],
    as_of_date: pd.Timestamp,
    train_policy: str,
) -> list[dict[str, object]]:
    validation, _ = select_validation_rows(full_frame, train_frame, as_of_date, train_policy)
    if validation.empty:
        validation = train_frame.tail(min(len(train_frame), 252)).copy()
    importance = permutation_importance(
        model,
        validation[features],
        validation["adjustment_label"],
        n_repeats=5,
        random_state=42,
        n_jobs=1,
    )
    return [
        {
            "model": model_name,
            "feature": feature,
            "importance": float(score),
            "importance_std": float(std),
        }
        for feature, score, std in zip(features, importance.importances_mean, importance.importances_std)
    ]


def select_validation_rows(
    full_frame: pd.DataFrame,
    train_frame: pd.DataFrame,
    as_of_date: pd.Timestamp,
    train_policy: str,
) -> tuple[pd.DataFrame, str]:
    if train_policy == "live_pre_year":
        validation_start = pd.Timestamp(year=as_of_date.year - 1, month=10, day=1)
        validation_end = pd.Timestamp(year=as_of_date.year, month=1, day=1)
        validation = full_frame[
            (full_frame["date"] >= validation_start) & (full_frame["date"] < validation_end)
        ].copy()
        return validation, "out_of_sample_previous_q4"
    return train_frame.tail(min(len(train_frame), 252)).copy(), "in_sample_diagnostic_only"


def compute_driver_rows(
    model_name: str,
    scored: pd.DataFrame,
    train_frame: pd.DataFrame,
    importance_rows: Iterable[dict[str, object]],
    features: list[str],
) -> list[dict[str, object]]:
    importance_map = {
        row["feature"]: max(0.0, float(row["importance"]))
        for row in importance_rows
        if row["model"] == model_name
    }
    train_mean = train_frame[features].mean(numeric_only=True)
    train_std = train_frame[features].std(numeric_only=True).replace(0, np.nan)
    rows = []
    for _, row in scored.iterrows():
        candidates = []
        for feature in features:
            z_score = 0.0
            if feature in train_std and pd.notna(train_std[feature]):
                z_score = float((row[feature] - train_mean[feature]) / train_std[feature])
            driver_score = abs(z_score) * importance_map.get(feature, 0.0)
            candidates.append((driver_score, feature, z_score, row[feature]))
        for rank, (score, feature, z_score, value) in enumerate(sorted(candidates, reverse=True)[:6], start=1):
            rows.append(
                {
                    "model": model_name,
                    "date": row["date"].date().isoformat(),
                    "rank": rank,
                    "feature": feature,
                    "value": float(value),
                    "z_score_vs_train": float(z_score),
                    "driver_score_proxy": float(score),
                    "pred_adjustment": int(row["pred_adjustment"]),
                    "target_position_proxy": float(row["target_position_proxy"]),
                }
            )
    return rows


def build_latest_artifact_row(
    model_name: str,
    scored: pd.DataFrame,
    as_of_date: pd.Timestamp,
    train_policy: str,
) -> dict[str, object]:
    eligible = scored[scored["date"] <= as_of_date].copy()
    if eligible.empty:
        raise ValueError(f"no score rows at or before {as_of_date.date()} for {model_name}")
    latest = eligible.sort_values("date").iloc[-1]
    return {
        "contract_version": CONTRACT_VERSION,
        "model_set_version": MODEL_SET_VERSION,
        "model_id": model_name,
        "train_policy": train_policy,
        "as_of_date": latest["date"].date().isoformat(),
        "adjustment": int(latest["pred_adjustment"]),
        "base_position_v3": float(latest["base_position_v3"]),
        "target_position_proxy": float(latest["target_position_proxy"]),
        "prob_down": float(latest["prob_-1"]),
        "prob_neutral": float(latest["prob_0"]),
        "prob_up": float(latest["prob_1"]),
    }


def write_outputs(
    output_root: Path,
    scores: pd.DataFrame,
    metrics: pd.DataFrame,
    importances: pd.DataFrame,
    drivers: pd.DataFrame,
    artifacts: pd.DataFrame,
    as_of_date: pd.Timestamp,
    train_policy: str,
    score_start_date: pd.Timestamp | None = None,
    analysis_root: Path | None = None,
) -> dict[str, object]:
    generated_files: list[str] = []
    latest_artifact_as_of_date = (
        pd.to_datetime(artifacts["as_of_date"]).max().date().isoformat()
        if not artifacts.empty
        else as_of_date.date().isoformat()
    )

    score_path = output_root / f"01_daily_model_scores_{train_policy}.csv"
    metrics_path = output_root / f"02_model_validation_metrics_{train_policy}.csv"
    importance_path = output_root / f"03_global_feature_importance_{train_policy}.csv"
    drivers_path = output_root / f"04_local_driver_explanations_{train_policy}.csv"
    artifacts_path = output_root / f"05_latest_adjustment_artifacts_{train_policy}.csv"

    scores.to_csv(score_path, index=False, encoding="utf-8-sig")
    metrics.to_csv(metrics_path, index=False, encoding="utf-8-sig")
    importances.to_csv(importance_path, index=False, encoding="utf-8-sig")
    drivers.to_csv(drivers_path, index=False, encoding="utf-8-sig")
    artifacts.to_csv(artifacts_path, index=False, encoding="utf-8-sig")
    generated_files.extend(
        [
            score_path.name,
            metrics_path.name,
            importance_path.name,
            drivers_path.name,
            artifacts_path.name,
        ]
    )
    for _, row in artifacts.iterrows():
        # 2026-04-27 CST: Include train_policy to prevent live and diagnostic
        # artifacts from overwriting each other during paired research runs.
        artifact_path = output_root / f"{row['model_id']}_{train_policy}_{as_of_date.date().isoformat()}_adjustment.json"
        artifact_path.write_text(json.dumps(row.to_dict(), ensure_ascii=False, indent=2), encoding="utf-8")
        generated_files.append(artifact_path.name)

    # 2026-04-28 CST: Added a machine-readable manifest because the governed
    # workflow must prove which policy/date produced each live artifact batch.
    # Purpose: let downstream daily operators and tests validate live-only files.
    manifest = {
        "contract_version": CONTRACT_VERSION,
        "model_set_version": MODEL_SET_VERSION,
        "train_policy": train_policy,
        "as_of_date": as_of_date.date().isoformat(),
        "latest_artifact_as_of_date": latest_artifact_as_of_date,
        "score_start_date": (
            score_start_date.date().isoformat()
            if score_start_date is not None
            else pd.to_datetime(scores["date"]).min().date().isoformat()
        ),
        "analysis_root": str(analysis_root) if analysis_root is not None else None,
        "generated_files": generated_files,
    }
    manifest_path = output_root / f"06_daily_workflow_manifest_{train_policy}.json"
    manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
    generated_files.append(manifest_path.name)
    return manifest


def print_summary(scores: pd.DataFrame, metrics: pd.DataFrame, artifacts: pd.DataFrame, output_root: Path) -> None:
    print("\n=== Model validation ===")
    print(metrics.to_string(index=False))
    print("\n=== Latest artifacts ===")
    print(artifacts.to_string(index=False))
    print("\n=== Recent model scores ===")
    columns = [
        "model",
        "date",
        "close",
        "base_position_v3",
        "pred_adjustment",
        "target_position_proxy",
        "prob_-1",
        "prob_0",
        "prob_1",
        "breakout20",
        "breakout60",
        "weighted_b20_vol",
        "weighted_vol_down",
        "action_vs_35pct",
    ]
    print(scores[columns].tail(20).to_string(index=False))
    print(f"\nOutputs written to: {output_root}")


if __name__ == "__main__":
    main()
