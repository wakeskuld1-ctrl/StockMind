#!/usr/bin/env python
# 2026-04-28 CST: Added because the previous ML comparison showed HGB under
# rolling_5y is the most promising signal purifier.
# Purpose: tune HGB selection intensity versus the rule baseline and determine
# whether a softer filter can preserve more total return while still improving
# quality and drawdown.

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import sys

import pandas as pd


DEFAULT_OUTPUT_ROOT = Path(r"E:\SM\docs\research\gold_hgb_hold15d_tuning_20260428")
DEFAULT_INPUT_ROOT = Path(r"E:\SM\docs\research\gold_ml_hold15d_experiment_20260428")
TOP_FRACTIONS = [0.7, 0.5, 0.3, 0.2]


def load_ml_module():
    module_path = Path(r"E:\SM\scripts\research\gold_ml_hold15d_experiment.py")
    spec = importlib.util.spec_from_file_location("gold_ml_hold15d_experiment", module_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


ML_MODULE = load_ml_module()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input-root", default=str(DEFAULT_INPUT_ROOT))
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    return parser.parse_args()


def apply_selection_threshold(frame: pd.DataFrame, top_fraction: float) -> pd.DataFrame:
    out = frame.copy()
    keep_count = max(int(round(len(out) * top_fraction)), 1)
    selected_idx = out.sort_values("score", ascending=False).head(keep_count).index
    out["selected_flag"] = 0
    out.loc[selected_idx, "selected_flag"] = 1
    return out


def build_policy_predictions(predictions: pd.DataFrame) -> pd.DataFrame:
    rows = []
    baseline = predictions[
        (predictions["split_mode"] == "rolling_5y")
        & (predictions["model_name"] == "rule_baseline")
        & (predictions["selection_policy"] == "all")
    ].copy()
    baseline["policy_name"] = "rule_baseline"
    rows.append(baseline)

    hgb = predictions[
        (predictions["split_mode"] == "rolling_5y")
        & (predictions["model_name"] == "hgb")
        & (predictions["selection_policy"] == "top_50pct")
    ].copy()
    base_cols = ["trade_date", "valid_year", "future_ret_15d", "hold_max_drawdown_15d", "hold_max_runup_15d", "score"]
    hgb_base = hgb[base_cols].drop_duplicates(subset=["trade_date"]).copy()
    for top_fraction in TOP_FRACTIONS:
        policy = apply_selection_threshold(hgb_base, top_fraction)
        policy["policy_name"] = f"top_{int(top_fraction * 100)}pct"
        rows.append(policy)
    return pd.concat(rows, ignore_index=True)


def summarize_tuning_results(trades: pd.DataFrame) -> pd.DataFrame:
    rows = []
    selected = trades.copy()
    if "selected_flag" not in selected.columns:
        selected["selected_flag"] = 1
    grouped = selected[selected["selected_flag"] == 1].groupby("policy_name", dropna=False)
    for policy_name, subset in grouped:
        years = int(pd.to_datetime(subset["trade_date"]).dt.year.nunique())
        rows.append(
            {
                "policy_name": policy_name,
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


def run_sequential_compound_backtest(trades: pd.DataFrame, initial_capital: float = 1_000_000.0) -> dict[str, float]:
    subset = trades.copy()
    subset["trade_date"] = pd.to_datetime(subset["trade_date"])
    subset = subset.sort_values("trade_date")
    capital = initial_capital
    peak = capital
    max_drawdown = 0.0
    for _, row in subset.iterrows():
        capital *= (1.0 + row["future_ret_15d"])
        peak = max(peak, capital)
        max_drawdown = min(max_drawdown, capital / peak - 1.0)
    years = (subset["trade_date"].max() - subset["trade_date"].min()).days / 365.25 if len(subset) > 1 else 0
    cagr = (capital / initial_capital) ** (1 / years) - 1 if years > 0 else None
    return {
        "terminal_capital": capital,
        "total_return": capital / initial_capital - 1.0,
        "max_drawdown": max_drawdown,
        "cagr": cagr,
    }


def build_backtest_table(trades: pd.DataFrame) -> pd.DataFrame:
    rows = []
    for policy_name, subset in trades[trades["selected_flag"] == 1].groupby("policy_name", dropna=False):
        metrics = run_sequential_compound_backtest(subset)
        rows.append({"policy_name": policy_name, **metrics})
    return pd.DataFrame(rows)


def extract_best_policy(summary: pd.DataFrame, backtests: pd.DataFrame) -> dict[str, object]:
    merged = summary.merge(backtests, on="policy_name", how="left")
    candidates = merged[
        (merged["positive_year_ratio"] >= 0.80)
        & (merged["median_return"] > 0)
        & (merged["events_per_year"] >= 5)
    ].copy()
    if candidates.empty:
        return {}

    baseline = candidates[candidates["policy_name"] == "rule_baseline"].iloc[0] if not candidates[candidates["policy_name"] == "rule_baseline"].empty else None
    if baseline is not None:
        filtered = candidates[
            (candidates["policy_name"] != "rule_baseline")
            & (candidates["terminal_capital"] >= baseline["terminal_capital"] * 0.90)
            & (candidates["max_drawdown"] >= baseline["max_drawdown"])
        ].copy()
        if not filtered.empty:
            candidates = filtered
    ranked = candidates.sort_values(
        ["terminal_capital", "max_drawdown", "win_rate", "avg_return"],
        ascending=[False, False, False, False],
    )
    return ranked.iloc[0].to_dict()


def main() -> int:
    args = parse_args()
    input_root = Path(args.input_root)
    predictions = pd.read_csv(input_root / "gold_ml_hold15d_predictions.csv")
    policy_predictions = build_policy_predictions(predictions)
    summary = summarize_tuning_results(policy_predictions)
    backtests = build_backtest_table(policy_predictions)
    best = extract_best_policy(summary, backtests)

    output_root = Path(args.output_root)
    output_root.mkdir(parents=True, exist_ok=True)
    policy_predictions.to_csv(output_root / "gold_hgb_hold15d_tuning_predictions.csv", index=False, encoding="utf-8-sig")
    summary.to_csv(output_root / "gold_hgb_hold15d_tuning_summary.csv", index=False, encoding="utf-8-sig")
    backtests.to_csv(output_root / "gold_hgb_hold15d_tuning_backtests.csv", index=False, encoding="utf-8-sig")

    payload = {
        "best_policy": best,
        "summary": summary.merge(backtests, on="policy_name", how="left").sort_values("terminal_capital", ascending=False).to_dict(orient="records"),
    }
    (output_root / "summary.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(payload, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
