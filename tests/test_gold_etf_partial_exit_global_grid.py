import importlib.util
from pathlib import Path
import sys

import pandas as pd


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_etf_partial_exit_global_grid.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_etf_partial_exit_global_grid", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def build_history(closes: list[float]) -> pd.DataFrame:
    dates = pd.bdate_range("2025-01-01", periods=len(closes))
    return pd.DataFrame(
        {
            "trade_date": dates,
            "symbol": ["518800.SH"] * len(closes),
            "open": closes,
            "high": [price + 0.1 for price in closes],
            "low": [price - 0.1 for price in closes],
            "close": closes,
        }
    )


def test_build_parameter_grid_includes_interpretable_global_space():
    module = load_module()

    grid = module.build_parameter_grid()

    assert any(config["partial_exit_ratio"] == 0.5 for config in grid)
    assert any(config["partial_exit_condition"] == "anchor_return_gt_0" for config in grid)
    assert any(config["trailing_drawdown"] == 0.015 for config in grid)
    assert any(config["trailing_start_day"] == 15 for config in grid)
    assert any(config["max_hold_days"] == 45 for config in grid)
    assert any(config["loss_anchor_action"] == "time_exit_20d" for config in grid)


def test_resolve_partial_exit_sells_at_d16_open_when_anchor_condition_is_met():
    module = load_module()
    history = build_history([10.1] * 14 + [10.6, 10.7, 10.8, 10.65, 10.45, 10.3])
    config = {
        "partial_exit_ratio": 0.5,
        "partial_exit_condition": "anchor_return_gt_0",
        "trailing_drawdown": 0.015,
        "trailing_start_day": 15,
        "max_hold_days": 30,
        "loss_anchor_action": "time_exit_20d",
    }

    events = module.resolve_partial_exit_events(
        history=history,
        first_entry_idx=0,
        weighted_entry_price=10.0,
        total_weight=0.9,
        config=config,
        rebound_check_day=5,
        anchor_day=15,
    )

    assert events[0]["event_type"] == "partial_exit"
    assert events[0]["exit_idx"] == 15
    assert events[0]["exit_weight"] == 0.45
    assert events[0]["reason"] == "partial_anchor_return_gt_0_d15"
    assert events[1]["event_type"] == "final_exit"
    assert events[1]["exit_idx"] == 18
    assert events[1]["reason"] == "trail_dd_0.015_d18"


def test_resolve_partial_exit_skips_partial_when_anchor_condition_fails_and_uses_loss_action():
    module = load_module()
    history = build_history([10.1] * 14 + [9.9, 9.95, 10.0, 10.05, 10.1, 10.2, 10.3])
    config = {
        "partial_exit_ratio": 0.5,
        "partial_exit_condition": "anchor_return_gt_0",
        "trailing_drawdown": 0.015,
        "trailing_start_day": 15,
        "max_hold_days": 30,
        "loss_anchor_action": "time_exit_20d",
    }

    events = module.resolve_partial_exit_events(
        history=history,
        first_entry_idx=0,
        weighted_entry_price=10.0,
        total_weight=0.9,
        config=config,
        rebound_check_day=5,
        anchor_day=15,
    )

    assert len(events) == 1
    assert events[0]["event_type"] == "final_exit"
    assert events[0]["exit_idx"] == 20
    assert events[0]["exit_weight"] == 0.9
    assert events[0]["reason"] == "anchor_loss_time_exit_20d"


def test_compute_trade_return_handles_partial_and_final_exit_weights():
    module = load_module()
    events = [
        {"exit_price": 10.6, "exit_weight": 0.45},
        {"exit_price": 10.45, "exit_weight": 0.45},
    ]

    trade_return = module.compute_weighted_trade_return(events, weighted_entry_price=10.0)

    assert round(trade_return, 6) == round(0.45 * 0.06 + 0.45 * 0.045, 6)


def test_rank_parameter_results_keeps_acceptance_columns():
    module = load_module()
    summary = pd.DataFrame(
        [
            {"config_name": "weak", "total_return": 0.8, "max_drawdown": -0.2},
            {"config_name": "strong", "total_return": 1.0, "max_drawdown": -0.12},
        ]
    )

    ranked = module.rank_parameter_results(summary)

    assert ranked.iloc[0]["config_name"] == "strong"
    assert "beats_live_return_drawdown_ratio" in ranked.columns
    assert "defensive_candidate" in ranked.columns
