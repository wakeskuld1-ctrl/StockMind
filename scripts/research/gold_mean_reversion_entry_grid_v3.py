#!/usr/bin/env python
# 2026-04-28 CST: Added because the current gold mean-reversion prototype is
# too sparse to serve as a main trading module.
# Purpose: scan wider entry thresholds and identify whether frequency can rise
# without destroying the edge under the already-approved fixed exits.

from __future__ import annotations

import argparse
import importlib.util
import itertools
import json
from pathlib import Path
import sys

import pandas as pd


DEFAULT_OUTPUT_ROOT = Path(r"E:\SM\docs\research\gold_mean_reversion_entry_grid_v3_10y_20260428")
DEFAULT_START_DATE = "2016-01-01"
DEFAULT_END_DATE = "2026-04-28"
RET_THRESHOLDS = [-0.015, -0.02, -0.025, -0.03]
MA20_THRESHOLDS = [-0.01, -0.015, -0.02, -0.025]
RISK_FILTER_OPTIONS = [True, False]


def load_v2_module():
    module_path = Path(r"E:\SM\scripts\research\gold_mean_reversion_exit_v2.py")
    spec = importlib.util.spec_from_file_location("gold_mean_reversion_exit_v2", module_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


V2_MODULE = load_v2_module()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--start-date", default=DEFAULT_START_DATE)
    parser.add_argument("--end-date", default=DEFAULT_END_DATE)
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    return parser.parse_args()


def build_config_key(ret_threshold: float, ma20_threshold: float, use_risk_filter: bool) -> str:
    return f"ret{ret_threshold:.3f}_ma{ma20_threshold:.3f}_risk{int(use_risk_filter)}"


def flag_entry_config(frame: pd.DataFrame, ret_threshold: float, ma20_threshold: float, use_risk_filter: bool) -> pd.DataFrame:
    out = frame.copy()
    condition = (out["ret_5d"] <= ret_threshold) & (out["close_vs_ma20"] <= ma20_threshold)
    if use_risk_filter:
        condition = condition & (out["failure_risk_flag"] == 0)
    out["entry_flag"] = condition.astype(int)
    return out


def build_grid_trades(frame: pd.DataFrame) -> pd.DataFrame:
    sample = frame[frame["entry_flag"] == 1].copy()
    rows = []
    for _, row in sample.iterrows():
        trade_date = str(pd.to_datetime(row["trade_date"]).date())
        for holding_days in [10, 15]:
            result = V2_MODULE.simulate_fixed_holding_exit(row, holding_days)
            result["trade_date"] = trade_date
            rows.append(result)
    return pd.DataFrame(rows)


def summarize_grid_results(trades: pd.DataFrame) -> pd.DataFrame:
    rows = []
    grouped = trades.groupby(["config_key", "exit_rule"], dropna=False)
    for (config_key, exit_rule), subset in grouped:
        trade_dates = pd.to_datetime(subset["trade_date"])
        years_covered = int(trade_dates.dt.year.nunique())
        sample_count = int(len(subset))
        avg_return = float(subset["realized_return"].mean())
        avg_holding_days = float(subset["holding_days"].mean())
        efficiency = V2_MODULE.BASE_MODULE.compute_time_efficiency(avg_return, max(int(round(avg_holding_days)), 1))
        positive_year_ratio = float(
            subset.assign(year=trade_dates.dt.year).groupby("year")["realized_return"].mean().gt(0).mean()
        )
        rows.append(
            {
                "config_key": config_key,
                "exit_rule": exit_rule,
                "sample_count": sample_count,
                "years_covered": years_covered,
                "events_per_year": sample_count / years_covered if years_covered > 0 else 0.0,
                "win_rate": float((subset["realized_return"] > 0).mean()),
                "avg_return": avg_return,
                "median_return": float(subset["realized_return"].median()),
                "avg_max_drawdown": float(subset["realized_drawdown"].mean()),
                "avg_max_runup": float(subset["realized_runup"].mean()),
                "return_per_day": efficiency["return_per_day"],
                "annualized_equivalent": efficiency["annualized_equivalent"],
                "positive_year_ratio": positive_year_ratio,
            }
        )
    return pd.DataFrame(rows)


def extract_frontier_candidates(summary: pd.DataFrame) -> pd.DataFrame:
    eligible = summary[
        (summary["events_per_year"] >= 4.0)
        & (summary["win_rate"] >= 0.60)
        & (summary["avg_return"] > 0.01)
        & (summary["median_return"] > 0)
        & (summary["positive_year_ratio"] >= 0.70)
    ].copy()
    if eligible.empty:
        return eligible
    return eligible.sort_values(
        ["events_per_year", "avg_return", "win_rate", "return_per_day"],
        ascending=[False, False, False, False],
    ).reset_index(drop=True)


def parse_config_key(config_key: str) -> dict[str, float | bool]:
    parts = config_key.split("_")
    return {
        "ret_threshold": float(parts[0].replace("ret", "")),
        "ma20_threshold": float(parts[1].replace("ma", "")),
        "use_risk_filter": bool(int(parts[2].replace("risk", ""))),
    }


def run_grid(start_date: str, end_date: str) -> tuple[pd.DataFrame, pd.DataFrame, dict[str, int]]:
    base_frame, counts = V2_MODULE.BROAD_MODULE.RULE_MODULE.BASE_MODULE.prepare_analysis_frame(start_date, end_date)
    base_frame = V2_MODULE.BROAD_MODULE.RULE_MODULE.build_failure_flags(
        V2_MODULE.BROAD_MODULE.RULE_MODULE.assign_bucket_labels(base_frame)
    )
    all_trades = []
    config_rows = []
    for ret_threshold, ma20_threshold, use_risk_filter in itertools.product(
        RET_THRESHOLDS, MA20_THRESHOLDS, RISK_FILTER_OPTIONS
    ):
        config_key = build_config_key(ret_threshold, ma20_threshold, use_risk_filter)
        flagged = flag_entry_config(base_frame, ret_threshold, ma20_threshold, use_risk_filter)
        flagged["config_key"] = config_key
        trades = build_grid_trades(flagged)
        if trades.empty:
            continue
        trades["config_key"] = config_key
        all_trades.append(trades)
        config_rows.append(
            {
                "config_key": config_key,
                "ret_threshold": ret_threshold,
                "ma20_threshold": ma20_threshold,
                "use_risk_filter": use_risk_filter,
                "event_count": int(flagged["entry_flag"].sum()),
            }
        )
    trade_frame = pd.concat(all_trades, ignore_index=True) if all_trades else pd.DataFrame()
    config_frame = pd.DataFrame(config_rows)
    counts["grid_config_count"] = int(len(config_frame))
    return trade_frame, config_frame, counts


def main() -> int:
    args = parse_args()
    trades, configs, counts = run_grid(args.start_date, args.end_date)
    summary = summarize_grid_results(trades)
    frontier = extract_frontier_candidates(summary)
    config_details = configs.merge(summary, on="config_key", how="left")

    output_root = Path(args.output_root)
    output_root.mkdir(parents=True, exist_ok=True)
    trades.to_csv(output_root / "gold_mean_reversion_entry_grid_v3_trades.csv", index=False, encoding="utf-8-sig")
    configs.to_csv(output_root / "gold_mean_reversion_entry_grid_v3_configs.csv", index=False, encoding="utf-8-sig")
    config_details.to_csv(output_root / "gold_mean_reversion_entry_grid_v3_summary.csv", index=False, encoding="utf-8-sig")
    frontier.to_csv(output_root / "gold_mean_reversion_entry_grid_v3_frontier.csv", index=False, encoding="utf-8-sig")

    payload = {
        "data_counts": counts,
        "frontier_top": frontier.head(10).to_dict(orient="records"),
    }
    (output_root / "summary.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(payload, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
