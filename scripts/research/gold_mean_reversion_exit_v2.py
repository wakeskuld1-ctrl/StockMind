#!/usr/bin/env python
# 2026-04-28 CST: Added because the broad V1 entry now needs exit testing
# before we can judge whether the pattern is close to tradable.
# Purpose: compare fixed-holding exits against simple break-based exits on the
# same broad mean-reversion entry set.

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import sys

import numpy as np
import pandas as pd


DEFAULT_OUTPUT_ROOT = Path(r"E:\SM\docs\research\gold_mean_reversion_exit_v2_10y_20260428")
DEFAULT_START_DATE = "2016-01-01"
DEFAULT_END_DATE = "2026-04-28"


def load_broad_module():
    module_path = Path(r"E:\SM\scripts\research\gold_mean_reversion_broad_v1.py")
    spec = importlib.util.spec_from_file_location("gold_mean_reversion_broad_v1", module_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


BROAD_MODULE = load_broad_module()
BASE_MODULE = BROAD_MODULE.BASE_MODULE


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--start-date", default=DEFAULT_START_DATE)
    parser.add_argument("--end-date", default=DEFAULT_END_DATE)
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    return parser.parse_args()


def _parse_path(raw: str) -> list[float]:
    return [float(x) for x in str(raw).split("|") if str(x).strip()]


def simulate_fixed_holding_exit(row: pd.Series, holding_days: int) -> dict[str, float | int | str]:
    return {
        "exit_rule": f"hold_{holding_days}d",
        "holding_days": int(holding_days),
        "exit_day": int(holding_days),
        "realized_return": float(row[f"future_ret_{holding_days}d"]),
        "realized_drawdown": float(row[f"hold_max_drawdown_{holding_days}d"]),
        "realized_runup": float(row[f"hold_max_runup_{holding_days}d"]),
    }


def simulate_break_rule_exit(row: pd.Series, break_rule: str, max_holding_days: int = 15) -> dict[str, float | int | str]:
    lows = _parse_path(row[f"gold_low_path_{max_holding_days}d"])
    highs = _parse_path(row[f"gold_high_path_{max_holding_days}d"])
    closes = _parse_path(row[f"gold_close_path_{max_holding_days}d"])
    ma5s = _parse_path(row[f"ma5_path_{max_holding_days}d"])
    ma10s = _parse_path(row[f"ma10_path_{max_holding_days}d"])
    entry = float(row["gold_close"])
    event_low = min(lows[0], entry)

    exit_day = len(closes)
    exit_close = closes[-1]
    for idx, close in enumerate(closes, start=1):
        trigger = False
        if break_rule == "break_event_low":
            trigger = close < event_low
        elif break_rule == "break_ma5":
            trigger = close < ma5s[idx - 1]
        elif break_rule == "break_ma10":
            trigger = close < ma10s[idx - 1]
        if trigger:
            exit_day = idx
            exit_close = close
            break

    realized_drawdown = min(lows[:exit_day]) / entry - 1.0
    realized_runup = max(highs[:exit_day]) / entry - 1.0
    return {
        "exit_rule": break_rule,
        "holding_days": int(exit_day),
        "exit_day": int(exit_day),
        "realized_return": float(exit_close / entry - 1.0),
        "realized_drawdown": float(realized_drawdown),
        "realized_runup": float(realized_runup),
    }


def summarize_exit_results(trades: pd.DataFrame) -> pd.DataFrame:
    rows = []
    for exit_rule, subset in trades.groupby("exit_rule", dropna=False):
        avg_return = float(subset["realized_return"].mean())
        avg_holding_days = float(subset["holding_days"].mean())
        efficiency = BASE_MODULE.compute_time_efficiency(avg_return, max(int(round(avg_holding_days)), 1))
        rows.append(
            {
                "exit_rule": exit_rule,
                "sample_count": int(len(subset)),
                "avg_holding_days": avg_holding_days,
                "win_rate": float((subset["realized_return"] > 0).mean()),
                "avg_return": avg_return,
                "median_return": float(subset["realized_return"].median()),
                "avg_max_drawdown": float(subset["realized_drawdown"].mean()),
                "avg_max_runup": float(subset["realized_runup"].mean()),
                "return_per_day": efficiency["return_per_day"],
                "annualized_equivalent": efficiency["annualized_equivalent"],
            }
        )
    return pd.DataFrame(rows)


def summarize_yearly_results(trades: pd.DataFrame) -> pd.DataFrame:
    out = trades.copy()
    out["trade_date"] = pd.to_datetime(out["trade_date"])
    out["year"] = out["trade_date"].dt.year
    rows = []
    for (exit_rule, year), subset in out.groupby(["exit_rule", "year"], dropna=False):
        rows.append(
            {
                "exit_rule": exit_rule,
                "year": int(year),
                "sample_count": int(len(subset)),
                "win_rate": float((subset["realized_return"] > 0).mean()),
                "avg_return": float(subset["realized_return"].mean()),
                "avg_max_drawdown": float(subset["realized_drawdown"].mean()),
            }
        )
    return pd.DataFrame(rows)


def extract_v2_verdict(summary: pd.DataFrame, yearly: pd.DataFrame) -> dict[str, float | int | str | bool]:
    rows = []
    for _, row in summary.iterrows():
        exit_rule = row["exit_rule"]
        yearly_subset = yearly[yearly["exit_rule"] == exit_rule]
        positive_year_ratio = float((yearly_subset["avg_return"] > 0).mean()) if not yearly_subset.empty else 0.0
        stable_year_count = int((yearly_subset["sample_count"] >= 3).sum()) if not yearly_subset.empty else 0
        median_return = row["median_return"] if "median_return" in row.index else row["avg_return"]
        rows.append(
            {
                **row.to_dict(),
                "median_return": median_return,
                "positive_year_ratio": positive_year_ratio,
                "stable_year_count": stable_year_count,
            }
        )
    ranked = pd.DataFrame(rows).sort_values(
        ["positive_year_ratio", "win_rate", "median_return", "avg_return", "avg_max_drawdown"],
        ascending=[False, False, False, False, False],
    )
    return ranked.iloc[0].to_dict() if not ranked.empty else {}


def build_exit_path_columns(frame: pd.DataFrame, max_holding_days: int = 15) -> pd.DataFrame:
    out = frame.copy()
    closes = out["gold_close"].to_numpy()
    highs = out["gold_high"].to_numpy()
    lows = out["gold_low"].to_numpy()
    ma5 = out["ma5"].to_numpy()
    ma10 = out["ma10"].to_numpy()
    low_paths, high_paths, close_paths, ma5_paths, ma10_paths = [], [], [], [], []
    for idx in range(len(out)):
        end = min(idx + max_holding_days, len(out) - 1)
        low_paths.append("|".join(f"{x:.6f}" for x in lows[idx + 1 : end + 1]))
        high_paths.append("|".join(f"{x:.6f}" for x in highs[idx + 1 : end + 1]))
        close_paths.append("|".join(f"{x:.6f}" for x in closes[idx + 1 : end + 1]))
        ma5_paths.append("|".join(f"{x:.6f}" for x in ma5[idx + 1 : end + 1]))
        ma10_paths.append("|".join(f"{x:.6f}" for x in ma10[idx + 1 : end + 1]))
    out[f"gold_low_path_{max_holding_days}d"] = low_paths
    out[f"gold_high_path_{max_holding_days}d"] = high_paths
    out[f"gold_close_path_{max_holding_days}d"] = close_paths
    out[f"ma5_path_{max_holding_days}d"] = ma5_paths
    out[f"ma10_path_{max_holding_days}d"] = ma10_paths
    return out


def prepare_broad_event_sample(start_date: str, end_date: str) -> tuple[pd.DataFrame, dict[str, int]]:
    frame, counts = BROAD_MODULE.prepare_broad_event_frame(start_date, end_date)
    frame = build_exit_path_columns(frame, max_holding_days=15)
    sample = frame[frame["broad_event_flag"] == 1].copy()
    counts["v2_event_rows"] = int(len(sample))
    return sample, counts


def run_all_exit_rules(sample: pd.DataFrame) -> pd.DataFrame:
    rows = []
    for _, row in sample.iterrows():
        trade_date = str(pd.to_datetime(row["trade_date"]).date())
        for holding_days in [10, 15]:
            result = simulate_fixed_holding_exit(row, holding_days)
            result["trade_date"] = trade_date
            rows.append(result)
        for break_rule in ["break_event_low", "break_ma5", "break_ma10"]:
            result = simulate_break_rule_exit(row, break_rule, max_holding_days=15)
            result["trade_date"] = trade_date
            rows.append(result)
    return pd.DataFrame(rows)


def main() -> int:
    args = parse_args()
    sample, counts = prepare_broad_event_sample(args.start_date, args.end_date)
    trades = run_all_exit_rules(sample)
    summary = summarize_exit_results(trades)
    yearly = summarize_yearly_results(trades)
    verdict = extract_v2_verdict(summary, yearly)

    output_root = Path(args.output_root)
    output_root.mkdir(parents=True, exist_ok=True)
    sample.to_csv(output_root / "gold_mean_reversion_exit_v2_samples.csv", index=False, encoding="utf-8-sig")
    trades.to_csv(output_root / "gold_mean_reversion_exit_v2_trades.csv", index=False, encoding="utf-8-sig")
    summary.to_csv(output_root / "gold_mean_reversion_exit_v2_summary.csv", index=False, encoding="utf-8-sig")
    yearly.to_csv(output_root / "gold_mean_reversion_exit_v2_yearly.csv", index=False, encoding="utf-8-sig")

    payload = {
        "data_counts": counts,
        "verdict": verdict,
        "summary": summary.to_dict(orient="records"),
    }
    (output_root / "summary.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(payload, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
