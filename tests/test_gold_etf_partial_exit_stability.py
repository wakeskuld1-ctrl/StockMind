import importlib.util
from pathlib import Path
import sys

import pandas as pd


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_etf_partial_exit_stability.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_etf_partial_exit_stability", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_build_narrow_grid_focuses_on_top_parameter_neighborhood():
    module = load_module()

    grid = module.build_narrow_grid()

    assert len(grid) == 144
    assert any(config["partial_exit_ratio"] == 0.6 for config in grid)
    assert any(config["partial_exit_condition"] == "anchor_return_gt_0.005" for config in grid)
    assert any(config["trailing_drawdown"] == 0.008 for config in grid)
    assert all(config["loss_anchor_action"] == "hold_to_max" for config in grid)


def test_build_yearly_trade_summary_compounds_returns_by_entry_year():
    module = load_module()
    trade_log = pd.DataFrame(
        {
            "entry_date": pd.to_datetime(["2024-01-02", "2024-02-01", "2025-01-03"]),
            "trade_return": [0.10, -0.05, 0.20],
            "hold_trading_days": [20, 30, 25],
            "event_reasons": ["a", "b", "c"],
        }
    )

    summary = module.build_yearly_trade_summary(trade_log)

    row_2024 = summary[summary["year"] == 2024].iloc[0]
    assert row_2024["sample_count"] == 2
    assert round(row_2024["compounded_trade_return"], 6) == round((1.10 * 0.95) - 1.0, 6)
    assert row_2024["win_rate"] == 0.5


def test_build_narrow_grid_diagnostics_reports_cluster_strength():
    module = load_module()
    ranked = pd.DataFrame(
        {
            "config_name": ["a", "b", "c", "d"],
            "return_drawdown_ratio": [9.0, 8.5, 7.0, 6.0],
            "total_return": [1.2, 1.1, 0.8, 0.7],
            "beats_live_return_drawdown_ratio": [True, True, False, False],
            "partial_exit_ratio": [0.7, 0.6, 0.5, 0.7],
            "partial_exit_condition": ["anchor_return_gt_0.01"] * 4,
            "trailing_drawdown": [0.01, 0.012, 0.015, 0.008],
            "trailing_start_day": [18, 18, 15, 15],
            "max_hold_days": [60, 60, 45, 45],
        }
    )

    diagnostics = module.build_narrow_grid_diagnostics(ranked)

    assert diagnostics["config_count"] == 4
    assert diagnostics["beats_live_count"] == 2
    assert diagnostics["top10_median_return_drawdown_ratio"] == 7.75
    assert diagnostics["best_config_name"] == "a"
