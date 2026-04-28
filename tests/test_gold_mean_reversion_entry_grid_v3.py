import importlib.util
from pathlib import Path
import sys

import pandas as pd


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_mean_reversion_entry_grid_v3.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_mean_reversion_entry_grid_v3", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_flag_entry_config_respects_ret_ma_and_risk_filter_switch():
    module = load_module()
    frame = pd.DataFrame(
        {
            "ret_5d": [-0.03, -0.018, -0.028],
            "close_vs_ma20": [-0.025, -0.02, -0.03],
            "failure_risk_flag": [0, 0, 1],
        }
    )

    strict = module.flag_entry_config(frame, ret_threshold=-0.025, ma20_threshold=-0.02, use_risk_filter=True)
    loose = module.flag_entry_config(frame, ret_threshold=-0.015, ma20_threshold=-0.02, use_risk_filter=False)

    assert list(strict["entry_flag"]) == [1, 0, 0]
    assert list(loose["entry_flag"]) == [1, 1, 1]


def test_build_grid_trades_creates_two_exit_versions_for_each_event():
    module = load_module()
    frame = pd.DataFrame(
        [
            {
                "trade_date": "2026-01-01",
                "entry_flag": 1,
                "future_ret_10d": 0.03,
                "hold_max_drawdown_10d": -0.02,
                "hold_max_runup_10d": 0.05,
                "future_ret_15d": 0.04,
                "hold_max_drawdown_15d": -0.03,
                "hold_max_runup_15d": 0.06,
            }
        ]
    )

    trades = module.build_grid_trades(frame)

    assert len(trades) == 2
    assert set(trades["exit_rule"]) == {"hold_10d", "hold_15d"}


def test_summarize_grid_results_reports_event_frequency_and_quality():
    module = load_module()
    trades = pd.DataFrame(
        [
            {"config_key": "A", "trade_date": "2025-01-03", "exit_rule": "hold_10d", "holding_days": 10, "realized_return": 0.02, "realized_drawdown": -0.01, "realized_runup": 0.03},
            {"config_key": "A", "trade_date": "2026-01-03", "exit_rule": "hold_10d", "holding_days": 10, "realized_return": 0.01, "realized_drawdown": -0.02, "realized_runup": 0.02},
        ]
    )

    summary = module.summarize_grid_results(trades)

    row = summary.iloc[0]
    assert row["sample_count"] == 2
    assert row["years_covered"] == 2
    assert row["events_per_year"] == 1
    assert round(row["win_rate"], 6) == 1.0


def test_extract_frontier_candidates_prefers_more_frequent_configs_when_edge_survives():
    module = load_module()
    summary = pd.DataFrame(
        [
            {"config_key": "narrow", "exit_rule": "hold_10d", "sample_count": 32, "events_per_year": 3.2, "win_rate": 0.90, "avg_return": 0.029, "median_return": 0.024, "avg_max_drawdown": -0.017, "return_per_day": 0.0029, "positive_year_ratio": 0.875},
            {"config_key": "balanced", "exit_rule": "hold_10d", "sample_count": 60, "events_per_year": 6.0, "win_rate": 0.72, "avg_return": 0.018, "median_return": 0.015, "avg_max_drawdown": -0.018, "return_per_day": 0.0018, "positive_year_ratio": 0.80},
            {"config_key": "too_loose", "exit_rule": "hold_10d", "sample_count": 120, "events_per_year": 12.0, "win_rate": 0.52, "avg_return": 0.002, "median_return": -0.001, "avg_max_drawdown": -0.03, "return_per_day": 0.0002, "positive_year_ratio": 0.50},
        ]
    )

    frontier = module.extract_frontier_candidates(summary)

    assert frontier.iloc[0]["config_key"] == "balanced"
