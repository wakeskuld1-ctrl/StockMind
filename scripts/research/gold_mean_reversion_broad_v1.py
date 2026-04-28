#!/usr/bin/env python
# 2026-04-28 CST: Added because the narrow gold mean-reversion layers proved
# informative but too sparse for a repeatability claim.
# Purpose: widen the event band, increase sample count, and test whether the
# positive edge survives at a level closer to a practical repeatable pattern.

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import sys

import numpy as np
import pandas as pd


DEFAULT_OUTPUT_ROOT = Path(r"E:\SM\docs\research\gold_mean_reversion_broad_v1_10y_20260428")
DEFAULT_START_DATE = "2016-01-01"
DEFAULT_END_DATE = "2026-04-28"
HOLDING_WINDOWS = [3, 5, 10, 15, 20, 30]


def load_rule_module():
    module_path = Path(r"E:\SM\scripts\research\gold_mean_reversion_rule_study.py")
    spec = importlib.util.spec_from_file_location("gold_mean_reversion_rule_study", module_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


RULE_MODULE = load_rule_module()
BASE_MODULE = RULE_MODULE.BASE_MODULE


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--start-date", default=DEFAULT_START_DATE)
    parser.add_argument("--end-date", default=DEFAULT_END_DATE)
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    return parser.parse_args()


def flag_broad_mean_reversion_events(frame: pd.DataFrame) -> pd.DataFrame:
    out = frame.copy()
    out["broad_event_flag"] = (
        (out["ret_5d"] <= -0.025)
        & (out["close_vs_ma20"] <= -0.02)
        & (out["failure_risk_flag"] == 0)
    ).astype(int)
    return out


def build_trade_table(frame: pd.DataFrame, holding_windows: list[int] | None = None) -> pd.DataFrame:
    if holding_windows is None:
        holding_windows = HOLDING_WINDOWS
    sample = frame[frame["broad_event_flag"] == 1].copy()
    rows = []
    for _, row in sample.iterrows():
        for holding_days in holding_windows:
            rows.append(
                {
                    "trade_date": str(pd.to_datetime(row["trade_date"]).date()),
                    "holding_days": int(holding_days),
                    "future_return": row[f"future_ret_{holding_days}d"],
                    "max_drawdown": row[f"hold_max_drawdown_{holding_days}d"],
                    "max_runup": row[f"hold_max_runup_{holding_days}d"],
                    "ret_5d": row.get("ret_5d", np.nan),
                    "close_vs_ma20": row.get("close_vs_ma20", np.nan),
                    "resonance_regime": row.get("resonance_regime", "unknown"),
                }
            )
    return pd.DataFrame(rows)


def summarize_overall(trades: pd.DataFrame) -> pd.DataFrame:
    rows = []
    for holding_days, subset in trades.groupby("holding_days", dropna=False):
        avg_return = float(subset["future_return"].mean()) if not subset.empty else np.nan
        efficiency = BASE_MODULE.compute_time_efficiency(avg_return, int(holding_days))
        rows.append(
            {
                "holding_days": int(holding_days),
                "sample_count": int(len(subset)),
                "win_rate": float((subset["future_return"] > 0).mean()) if not subset.empty else np.nan,
                "avg_return": avg_return,
                "median_return": float(subset["future_return"].median()) if not subset.empty else np.nan,
                "avg_max_drawdown": float(subset["max_drawdown"].mean()) if not subset.empty else np.nan,
                "avg_max_runup": float(subset["max_runup"].mean()) if not subset.empty else np.nan,
                "return_per_day": efficiency["return_per_day"],
                "annualized_equivalent": efficiency["annualized_equivalent"],
            }
        )
    return pd.DataFrame(rows)


def summarize_yearly_stability(trades: pd.DataFrame) -> pd.DataFrame:
    out = trades.copy()
    out["trade_date"] = pd.to_datetime(out["trade_date"])
    out["year"] = out["trade_date"].dt.year
    rows = []
    grouped = out.groupby(["holding_days", "year"], dropna=False)
    for (holding_days, year), subset in grouped:
        rows.append(
            {
                "holding_days": int(holding_days),
                "year": int(year),
                "sample_count": int(len(subset)),
                "win_rate": float((subset["future_return"] > 0).mean()),
                "avg_return": float(subset["future_return"].mean()),
                "avg_max_drawdown": float(subset["max_drawdown"].mean()),
            }
        )
    return pd.DataFrame(rows)


def extract_broad_v1_verdict(overall: pd.DataFrame, yearly: pd.DataFrame) -> dict[str, float | int | bool]:
    rows = []
    for _, row in overall.iterrows():
        holding_days = row["holding_days"]
        yearly_subset = yearly[yearly["holding_days"] == holding_days].copy()
        positive_year_ratio = float((yearly_subset["avg_return"] > 0).mean()) if not yearly_subset.empty else 0.0
        enough_years = int((yearly_subset["sample_count"] >= 3).sum())
        rows.append(
            {
                **row.to_dict(),
                "positive_year_ratio": positive_year_ratio,
                "years_with_enough_samples": enough_years,
                "is_repeatable_pattern": bool(
                    row["sample_count"] >= 30
                    and row["avg_return"] > 0
                    and row["win_rate"] >= 0.58
                    and positive_year_ratio >= 0.70
                ),
            }
        )
    verdict_frame = pd.DataFrame(rows).sort_values(
        ["is_repeatable_pattern", "return_per_day", "win_rate", "avg_return"],
        ascending=[False, False, False, False],
    )
    return verdict_frame.iloc[0].to_dict() if not verdict_frame.empty else {}


def prepare_broad_event_frame(start_date: str, end_date: str) -> tuple[pd.DataFrame, dict[str, int]]:
    frame, counts = RULE_MODULE.prepare_mean_reversion_frame(start_date, end_date)
    flagged = flag_broad_mean_reversion_events(frame)
    counts["broad_event_rows"] = int(flagged["broad_event_flag"].sum())
    return flagged, counts


def main() -> int:
    args = parse_args()
    frame, counts = prepare_broad_event_frame(args.start_date, args.end_date)
    trades = build_trade_table(frame)
    overall = summarize_overall(trades)
    yearly = summarize_yearly_stability(trades)
    verdict = extract_broad_v1_verdict(overall, yearly)

    output_root = Path(args.output_root)
    output_root.mkdir(parents=True, exist_ok=True)
    frame.to_csv(output_root / "gold_mean_reversion_broad_v1_samples.csv", index=False, encoding="utf-8-sig")
    trades.to_csv(output_root / "gold_mean_reversion_broad_v1_trades.csv", index=False, encoding="utf-8-sig")
    overall.to_csv(output_root / "gold_mean_reversion_broad_v1_overall.csv", index=False, encoding="utf-8-sig")
    yearly.to_csv(output_root / "gold_mean_reversion_broad_v1_yearly.csv", index=False, encoding="utf-8-sig")

    payload = {
        "data_counts": counts,
        "verdict": verdict,
        "overall": overall.to_dict(orient="records"),
    }
    (output_root / "summary.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(payload, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
