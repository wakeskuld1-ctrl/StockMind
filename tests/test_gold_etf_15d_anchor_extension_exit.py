import importlib.util
from pathlib import Path
import sys

import pandas as pd


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_etf_15d_anchor_extension_exit.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_etf_15d_anchor_extension_exit", SCRIPT_PATH)
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


def test_resolve_anchor_extension_exits_next_open_after_return_falls_below_d15_anchor():
    module = load_module()
    history = build_history(
        [
            10.0,
            10.1,
            10.2,
            10.3,
            10.4,
            10.5,
            10.6,
            10.7,
            10.8,
            10.9,
            11.0,
            11.1,
            11.2,
            11.3,
            11.4,
            11.35,
            11.2,
            11.2,
            11.0,
        ]
    )

    exit_idx, reason, anchor_return, extension_days = module.resolve_anchor_extension_exit(
        history=history,
        first_entry_idx=0,
        weighted_entry_price=10.0,
        rebound_check_day=5,
        anchor_day=15,
        max_hold_days=30,
    )

    assert exit_idx == 16
    assert reason == "anchor_return_break_d16"
    assert round(anchor_return, 6) == 0.14
    assert extension_days == 1


def test_resolve_anchor_extension_keeps_original_5d_failure_guard():
    module = load_module()
    history = build_history([10.0, 9.9, 9.8, 9.7, 9.6, 9.5, 9.4, 9.3, 9.2, 9.1, 9.0, 8.9, 8.8, 8.7, 8.6, 8.5])

    exit_idx, reason, anchor_return, extension_days = module.resolve_anchor_extension_exit(
        history=history,
        first_entry_idx=0,
        weighted_entry_price=10.0,
        rebound_check_day=5,
        anchor_day=15,
        max_hold_days=30,
    )

    assert exit_idx == 5
    assert reason == "fail_to_rebound_d5"
    assert anchor_return is None
    assert extension_days == 0


def test_build_max_hold_days_extends_to_200d_after_user_requested_wider_scan():
    module = load_module()

    assert module.build_max_hold_days() == [20, 30, 40, 60, 90, 120, 160, 200]


def test_summarize_anchor_extension_reports_post_15d_contribution_and_long_hold_counts():
    module = load_module()
    trade_log = pd.DataFrame(
        {
            "trade_return": [0.10, 0.05, -0.02],
            "return_at_anchor_day": [0.04, 0.03, None],
            "exit_capital": [1_100_000.0, 1_155_000.0, 1_131_900.0],
            "hold_trading_days": [22, 45, 6],
            "exit_reason": ["anchor_return_break_d21", "max_hold_40d", "fail_to_rebound_d5"],
        }
    )
    equity_curve = pd.DataFrame(
        {
            "date": pd.to_datetime(["2025-01-01", "2025-02-01"]),
            "equity": [1_000_000.0, 1_131_900.0],
            "drawdown": [0.0, -0.08],
        }
    )

    summary = module.summarize_anchor_backtest(
        trade_log=trade_log,
        equity_curve=equity_curve,
        config_name="anchor15_max40",
        initial_capital=1_000_000.0,
        max_hold_days=40,
    )

    assert summary["sample_count"] == 3
    assert summary["hold_over_20d_count"] == 2
    assert summary["hold_over_40d_count"] == 1
    assert round(summary["avg_post_anchor_return_contribution"], 6) == round(((0.10 - 0.04) + (0.05 - 0.03)) / 2, 6)
