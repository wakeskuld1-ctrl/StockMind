import importlib.util
from pathlib import Path
import sys

import pandas as pd


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_hgb_hold15d_light_filter.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_hgb_hold15d_light_filter", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_apply_bottom_filter_removes_lowest_scores_only():
    module = load_module()
    frame = pd.DataFrame({"score": [0.9, 0.7, 0.4, 0.2]})

    filtered = module.apply_bottom_filter(frame, drop_fraction=0.25)

    assert list(filtered["selected_flag"]) == [1, 1, 1, 0]


def test_summarize_filter_results_tracks_frequency_and_quality():
    module = load_module()
    trades = pd.DataFrame(
        [
            {"policy_name": "drop_bottom_10pct", "trade_date": "2025-01-03", "future_ret_15d": 0.03, "hold_max_drawdown_15d": -0.01, "hold_max_runup_15d": 0.05},
            {"policy_name": "drop_bottom_10pct", "trade_date": "2026-01-03", "future_ret_15d": 0.01, "hold_max_drawdown_15d": -0.02, "hold_max_runup_15d": 0.03},
        ]
    )

    summary = module.summarize_filter_results(trades)

    row = summary.iloc[0]
    assert row["sample_count"] == 2
    assert row["events_per_year"] == 1
    assert round(row["win_rate"], 6) == 1.0


def test_extract_best_light_filter_prefers_kept_frequency_with_drawdown_improvement():
    module = load_module()
    summary = pd.DataFrame(
        [
            {"policy_name": "rule_baseline", "sample_count": 100, "events_per_year": 18, "win_rate": 0.74, "avg_return": 0.023, "median_return": 0.020, "positive_year_ratio": 1.0},
            {"policy_name": "drop_bottom_10pct", "sample_count": 90, "events_per_year": 16, "win_rate": 0.76, "avg_return": 0.024, "median_return": 0.022, "positive_year_ratio": 1.0},
            {"policy_name": "drop_bottom_30pct", "sample_count": 70, "events_per_year": 12, "win_rate": 0.80, "avg_return": 0.026, "median_return": 0.023, "positive_year_ratio": 1.0},
        ]
    )
    backtests = pd.DataFrame(
        [
            {"policy_name": "rule_baseline", "terminal_capital": 12_000_000, "max_drawdown": -0.16, "cagr": 0.60},
            {"policy_name": "drop_bottom_10pct", "terminal_capital": 11_700_000, "max_drawdown": -0.12, "cagr": 0.58},
            {"policy_name": "drop_bottom_30pct", "terminal_capital": 9_000_000, "max_drawdown": -0.08, "cagr": 0.48},
        ]
    )

    best = module.extract_best_light_filter(summary, backtests)

    assert best["policy_name"] == "drop_bottom_10pct"
