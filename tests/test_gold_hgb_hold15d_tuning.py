import importlib.util
from pathlib import Path
import sys

import pandas as pd


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_hgb_hold15d_tuning.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_hgb_hold15d_tuning", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_apply_selection_threshold_marks_top_fraction_rows():
    module = load_module()
    frame = pd.DataFrame(
        {
            "score": [0.9, 0.7, 0.4, 0.2],
        }
    )

    selected = module.apply_selection_threshold(frame, top_fraction=0.5)

    assert list(selected["selected_flag"]) == [1, 1, 0, 0]


def test_summarize_tuning_results_reports_frequency_and_quality():
    module = load_module()
    trades = pd.DataFrame(
        [
            {"policy_name": "top_50pct", "trade_date": "2025-01-03", "future_ret_15d": 0.03, "hold_max_drawdown_15d": -0.01, "hold_max_runup_15d": 0.05},
            {"policy_name": "top_50pct", "trade_date": "2026-01-03", "future_ret_15d": 0.01, "hold_max_drawdown_15d": -0.02, "hold_max_runup_15d": 0.03},
        ]
    )

    summary = module.summarize_tuning_results(trades)

    row = summary.iloc[0]
    assert row["sample_count"] == 2
    assert row["events_per_year"] == 1
    assert round(row["win_rate"], 6) == 1.0


def test_run_sequential_compound_backtest_respects_trade_order():
    module = load_module()
    trades = pd.DataFrame(
        [
            {"trade_date": "2025-01-03", "future_ret_15d": 0.10},
            {"trade_date": "2025-02-03", "future_ret_15d": -0.05},
        ]
    )

    result = module.run_sequential_compound_backtest(trades, initial_capital=1_000_000)

    assert round(result["terminal_capital"], 2) == 1_045_000.00
    assert round(result["total_return"], 6) == 0.045


def test_extract_best_policy_balances_return_and_drawdown_against_baseline():
    module = load_module()
    summary = pd.DataFrame(
        [
            {"policy_name": "rule_baseline", "sample_count": 100, "events_per_year": 18, "win_rate": 0.74, "avg_return": 0.023, "median_return": 0.020, "positive_year_ratio": 1.0},
            {"policy_name": "top_70pct", "sample_count": 70, "events_per_year": 12, "win_rate": 0.79, "avg_return": 0.026, "median_return": 0.023, "positive_year_ratio": 1.0},
            {"policy_name": "top_20pct", "sample_count": 20, "events_per_year": 3, "win_rate": 0.93, "avg_return": 0.038, "median_return": 0.035, "positive_year_ratio": 1.0},
        ]
    )
    backtests = pd.DataFrame(
        [
            {"policy_name": "rule_baseline", "terminal_capital": 12_000_000, "max_drawdown": -0.16, "cagr": 0.60},
            {"policy_name": "top_70pct", "terminal_capital": 11_800_000, "max_drawdown": -0.10, "cagr": 0.57},
            {"policy_name": "top_20pct", "terminal_capital": 3_500_000, "max_drawdown": -0.05, "cagr": 0.28},
        ]
    )

    best = module.extract_best_policy(summary, backtests)

    assert best["policy_name"] == "top_70pct"
