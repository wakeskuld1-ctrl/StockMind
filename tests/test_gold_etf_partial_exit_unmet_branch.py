import importlib.util
from pathlib import Path
import sys

import pandas as pd


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_etf_partial_exit_unmet_branch.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_etf_partial_exit_unmet_branch", SCRIPT_PATH)
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


def test_rolling_anchor_window_triggers_partial_exit_after_d15_when_late_repair_occurs():
    module = load_module()
    history = build_history([10.1] * 14 + [10.05, 10.2, 10.25, 10.1, 10.0, 9.9, 9.8])
    config = {
        "partial_exit_ratio": 0.7,
        "anchor_return_threshold": 0.01,
        "anchor_window_end_day": 20,
        "trailing_drawdown": 0.01,
        "trailing_start_offset": 2,
        "unmet_exit_day": 30,
    }

    events = module.resolve_rolling_anchor_events(
        history=history,
        first_entry_idx=0,
        weighted_entry_price=10.0,
        total_weight=0.9,
        config=config,
        rebound_check_day=5,
        anchor_day=15,
    )

    assert events[0]["event_type"] == "partial_exit"
    assert events[0]["exit_idx"] == 16
    assert events[0]["reason"] == "partial_anchor_hit_d16"
    assert events[1]["event_type"] == "final_exit"


def test_unmet_branch_exits_at_configured_day_when_repair_never_reaches_threshold():
    module = load_module()
    history = build_history([10.1] * 14 + [10.05, 10.04, 10.03, 10.02, 10.01, 10.0, 9.99, 9.98])
    config = {
        "partial_exit_ratio": 0.7,
        "anchor_return_threshold": 0.01,
        "anchor_window_end_day": 20,
        "trailing_drawdown": 0.01,
        "trailing_start_offset": 2,
        "unmet_exit_day": 20,
    }

    events = module.resolve_rolling_anchor_events(
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
    assert events[0]["reason"] == "anchor_unmet_exit_d20"


def test_summarize_efficiency_reports_annualized_and_return_per_hold_day():
    module = load_module()
    trade_log = pd.DataFrame(
        {
            "trade_return": [0.05, 0.02],
            "hold_trading_days": [10, 30],
            "exit_capital": [1_050_000.0, 1_071_000.0],
        }
    )
    equity_curve = pd.DataFrame(
        {
            "date": pd.to_datetime(["2025-01-01", "2026-01-01"]),
            "equity": [1_000_000.0, 1_071_000.0],
            "drawdown": [0.0, -0.05],
        }
    )

    summary = module.summarize_efficiency(
        trade_log=trade_log,
        equity_curve=equity_curve,
        config_name="demo",
        initial_capital=1_000_000.0,
    )

    assert summary["sample_count"] == 2
    assert summary["avg_hold_trading_days"] == 20.0
    assert round(summary["return_per_hold_day"], 6) == round((0.05 + 0.02) / 40, 6)
    assert summary["cagr"] > 0


def test_build_branch_grid_includes_rolling_windows_and_unmet_exit_days():
    module = load_module()

    grid = module.build_branch_grid()

    assert any(config["anchor_window_end_day"] == 20 for config in grid)
    assert any(config["unmet_exit_day"] == 20 for config in grid)
    assert any(config["unmet_exit_day"] == 30 for config in grid)
    assert any(config["unmet_exit_day"] == 45 for config in grid)
