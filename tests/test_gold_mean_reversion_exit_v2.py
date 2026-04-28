import importlib.util
from pathlib import Path
import sys

import pandas as pd


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_mean_reversion_exit_v2.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_mean_reversion_exit_v2", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_simulate_fixed_holding_exit_returns_target_window_metrics():
    module = load_module()
    row = pd.Series(
        {
            "future_ret_10d": 0.03,
            "hold_max_drawdown_10d": -0.02,
            "hold_max_runup_10d": 0.05,
        }
    )

    result = module.simulate_fixed_holding_exit(row, holding_days=10)

    assert result["realized_return"] == 0.03
    assert result["realized_drawdown"] == -0.02
    assert result["holding_days"] == 10


def test_simulate_break_rule_exit_stops_on_first_trigger_day():
    module = load_module()
    row = pd.Series(
        {
            "gold_close": 100.0,
            "gold_low_path_15d": "99|97|96|95",
            "gold_high_path_15d": "101|103|104|106",
            "gold_close_path_15d": "100|98|97|105",
            "ma5_path_15d": "99|99|99|99",
            "ma10_path_15d": "98|98|98|98",
        }
    )

    result = module.simulate_break_rule_exit(row, break_rule="break_event_low", max_holding_days=15)

    assert result["exit_day"] == 2
    assert round(result["realized_return"], 6) == -0.02


def test_summarize_exit_results_compares_multiple_rules():
    module = load_module()
    trades = pd.DataFrame(
        [
            {"exit_rule": "hold_10d", "holding_days": 10, "realized_return": 0.03, "realized_drawdown": -0.02, "realized_runup": 0.05},
            {"exit_rule": "hold_10d", "holding_days": 10, "realized_return": -0.01, "realized_drawdown": -0.03, "realized_runup": 0.01},
            {"exit_rule": "break_ma5", "holding_days": 4, "realized_return": 0.01, "realized_drawdown": -0.01, "realized_runup": 0.02},
        ]
    )

    summary = module.summarize_exit_results(trades)

    assert set(summary["exit_rule"]) == {"hold_10d", "break_ma5"}
    hold_row = summary[summary["exit_rule"] == "hold_10d"].iloc[0]
    assert hold_row["sample_count"] == 2
    assert round(hold_row["win_rate"], 6) == 0.5


def test_extract_v2_verdict_prefers_better_risk_adjusted_exit():
    module = load_module()
    summary = pd.DataFrame(
        [
            {"exit_rule": "hold_10d", "sample_count": 32, "win_rate": 0.90, "avg_return": 0.029, "avg_max_drawdown": -0.017, "return_per_day": 0.0029},
            {"exit_rule": "break_ma5", "sample_count": 32, "win_rate": 0.78, "avg_return": 0.018, "avg_max_drawdown": -0.008, "return_per_day": 0.0032},
        ]
    )
    yearly = pd.DataFrame(
        [
            {"exit_rule": "hold_10d", "year": 2021, "sample_count": 6, "avg_return": 0.01},
            {"exit_rule": "hold_10d", "year": 2022, "sample_count": 4, "avg_return": 0.02},
            {"exit_rule": "hold_10d", "year": 2023, "sample_count": 5, "avg_return": 0.01},
            {"exit_rule": "hold_10d", "year": 2024, "sample_count": 6, "avg_return": 0.03},
            {"exit_rule": "break_ma5", "year": 2021, "sample_count": 6, "avg_return": 0.005},
            {"exit_rule": "break_ma5", "year": 2022, "sample_count": 4, "avg_return": 0.006},
            {"exit_rule": "break_ma5", "year": 2023, "sample_count": 5, "avg_return": -0.002},
            {"exit_rule": "break_ma5", "year": 2024, "sample_count": 6, "avg_return": 0.007},
        ]
    )

    verdict = module.extract_v2_verdict(summary, yearly)

    assert verdict["exit_rule"] == "hold_10d"
