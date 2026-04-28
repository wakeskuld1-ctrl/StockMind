#!/usr/bin/env python
# 2026-04-28 CST: Added because pure top-N filtering improved quality but cut
# frequency too aggressively.
# Purpose: test whether removing only the weakest HGB-ranked events improves
# drawdown and stability without sacrificing too much total return.

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import sys

import pandas as pd


DEFAULT_OUTPUT_ROOT = Path(r"E:\SM\docs\research\gold_hgb_hold15d_light_filter_20260428")
DEFAULT_INPUT_ROOT = Path(r"E:\SM\docs\research\gold_ml_hold15d_experiment_20260428")
DROP_FRACTIONS = [0.10, 0.20, 0.30]


def load_tuning_module():
    module_path = Path(r"E:\SM\scripts\research\gold_hgb_hold15d_tuning.py")
    spec = importlib.util.spec_from_file_location("gold_hgb_hold15d_tuning", module_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


TUNING_MODULE = load_tuning_module()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input-root", default=str(DEFAULT_INPUT_ROOT))
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    return parser.parse_args()


def apply_bottom_filter(frame: pd.DataFrame, drop_fraction: float) -> pd.DataFrame:
    out = frame.copy()
    drop_count = max(int(round(len(out) * drop_fraction)), 1)
    drop_idx = out.sort_values("score", ascending=True).head(drop_count).index
    out["selected_flag"] = 1
    out.loc[drop_idx, "selected_flag"] = 0
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
    hgb_base = hgb[["trade_date", "valid_year", "future_ret_15d", "hold_max_drawdown_15d", "hold_max_runup_15d", "score"]].drop_duplicates(subset=["trade_date"]).copy()
    for drop_fraction in DROP_FRACTIONS:
        policy = apply_bottom_filter(hgb_base, drop_fraction)
        policy["policy_name"] = f"drop_bottom_{int(drop_fraction * 100)}pct"
        rows.append(policy)
    return pd.concat(rows, ignore_index=True)


def summarize_filter_results(trades: pd.DataFrame) -> pd.DataFrame:
    selected = trades.copy()
    if "selected_flag" not in selected.columns:
        selected["selected_flag"] = 1
    rows = []
    for policy_name, subset in selected[selected["selected_flag"] == 1].groupby("policy_name", dropna=False):
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


def build_backtest_table(trades: pd.DataFrame) -> pd.DataFrame:
    rows = []
    selected = trades[trades["selected_flag"] == 1].copy()
    for policy_name, subset in selected.groupby("policy_name", dropna=False):
        metrics = TUNING_MODULE.run_sequential_compound_backtest(subset)
        rows.append({"policy_name": policy_name, **metrics})
    return pd.DataFrame(rows)


def extract_best_light_filter(summary: pd.DataFrame, backtests: pd.DataFrame) -> dict[str, object]:
    merged = summary.merge(backtests, on="policy_name", how="left")
    baseline = merged[merged["policy_name"] == "rule_baseline"].iloc[0] if not merged[merged["policy_name"] == "rule_baseline"].empty else None
    candidates = merged[
        (merged["policy_name"] != "rule_baseline")
        & (merged["positive_year_ratio"] >= 0.80)
        & (merged["events_per_year"] >= 10)
        & (merged["terminal_capital"] >= baseline["terminal_capital"] * 0.90 if baseline is not None else True)
        & (merged["max_drawdown"] >= baseline["max_drawdown"] if baseline is not None else True)
    ].copy()
    if candidates.empty:
        return baseline.to_dict() if baseline is not None else {}
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
    summary = summarize_filter_results(policy_predictions)
    backtests = build_backtest_table(policy_predictions)
    best = extract_best_light_filter(summary, backtests)

    output_root = Path(args.output_root)
    output_root.mkdir(parents=True, exist_ok=True)
    policy_predictions.to_csv(output_root / "gold_hgb_hold15d_light_filter_predictions.csv", index=False, encoding="utf-8-sig")
    summary.to_csv(output_root / "gold_hgb_hold15d_light_filter_summary.csv", index=False, encoding="utf-8-sig")
    backtests.to_csv(output_root / "gold_hgb_hold15d_light_filter_backtests.csv", index=False, encoding="utf-8-sig")

    payload = {
        "best_policy": best,
        "summary": summary.merge(backtests, on="policy_name", how="left").sort_values("terminal_capital", ascending=False).to_dict(orient="records"),
    }
    (output_root / "summary.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(payload, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
