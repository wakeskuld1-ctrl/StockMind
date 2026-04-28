import importlib.util
from pathlib import Path
import sys

import pandas as pd


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_etf_518800_exit_grid_analysis.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_etf_518800_exit_grid_analysis", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_build_parameter_grid_returns_nine_combinations():
    module = load_module()
    grid = module.build_parameter_grid([3, 5, 7], [10, 15, 20])

    assert len(grid) == 9
    assert set(grid["rebound_check_day"]) == {3, 5, 7}
    assert set(grid["hold_days"]) == {10, 15, 20}


def test_run_grid_backtest_exits_on_failed_rebound_day_plus_one():
    module = load_module()
    etf = pd.DataFrame(
        {
            "trade_date": pd.to_datetime(
                ["2025-01-01", "2025-01-02", "2025-01-03", "2025-01-06", "2025-01-07", "2025-01-08", "2025-01-09"]
            ),
            "open": [10.0, 10.1, 9.9, 9.8, 9.85, 9.7, 9.6],
            "close": [10.0, 9.95, 9.85, 9.8, 9.82, 9.75, 9.65],
        }
    )
    signals = pd.DataFrame(
        {
            "trade_date": pd.to_datetime(["2025-01-01"]),
            "selected_flag": [1],
        }
    )

    trade_log, _ = module.run_grid_backtest(
        policy_signals=signals,
        etf_history=etf,
        symbol="518800.SH",
        hold_days=5,
        rebound_check_day=3,
        initial_capital=100_000.0,
    )

    trade = trade_log.iloc[0]
    assert str(trade["trigger_date"].date()) == "2025-01-06"
    assert str(trade["exit_date"].date()) == "2025-01-07"
    assert trade["exit_reason"] == "fail_to_rebound_d3"


def test_apply_roundtrip_costs_reduces_terminal_capital():
    module = load_module()
    trade_log = pd.DataFrame(
        {
            "trade_return": [0.10, -0.05],
            "entry_date": pd.to_datetime(["2025-01-02", "2025-02-03"]),
            "exit_date": pd.to_datetime(["2025-01-06", "2025-02-05"]),
            "hold_calendar_days": [4, 2],
        }
    )

    costed = module.apply_roundtrip_costs(trade_log, initial_capital=1_000_000.0, cost_rate=0.001)

    assert round(costed["terminal_capital"], 2) < 1_045_000.00
    assert costed["sample_count"] == 2


def test_rank_grid_results_prefers_higher_terminal_then_lower_drawdown():
    module = load_module()
    summary = pd.DataFrame(
        [
            {"rebound_check_day": 5, "hold_days": 15, "terminal_capital": 1_600_000.0, "max_drawdown": -0.18, "cagr": 0.10},
            {"rebound_check_day": 3, "hold_days": 20, "terminal_capital": 1_550_000.0, "max_drawdown": -0.12, "cagr": 0.095},
        ]
    )

    ranked = module.rank_grid_results(summary)

    assert int(ranked.iloc[0]["rebound_check_day"]) == 5
