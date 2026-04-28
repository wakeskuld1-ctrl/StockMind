import importlib.util
from pathlib import Path
import sys

import pandas as pd


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_etf_daily_signal_tool.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_etf_daily_signal_tool", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_evaluate_flat_signal_returns_buy_when_signal_hits():
    module = load_module()
    signal_dates = pd.DataFrame({"trade_date": pd.to_datetime(["2026-04-28"])})

    report = module.evaluate_flat_signal(pd.Timestamp("2026-04-28"), signal_dates, "518800.SH", "fail_to_rebound_d5_hold_20d")

    assert report["action"] == "buy_next_open"
    assert report["reason"] == "gold_parent_rule_triggered"


def test_evaluate_position_signal_returns_sell_on_failed_rebound_day():
    module = load_module()
    history = pd.DataFrame(
        {
            "trade_date": pd.to_datetime(["2026-04-21", "2026-04-22", "2026-04-23", "2026-04-24", "2026-04-27"]),
            "close": [10.1, 10.0, 9.9, 9.8, 9.7],
        }
    )

    report = module.evaluate_position_signal(
        as_of_date=pd.Timestamp("2026-04-27"),
        history=history,
        strategy_name="fail_to_rebound_d5_hold_20d",
        entry_date=pd.Timestamp("2026-04-21"),
        entry_price=10.0,
        symbol="518800.SH",
    )

    assert report["action"] == "sell_next_open"
    assert report["reason"] == "fail_to_rebound_d5"


def test_evaluate_position_signal_returns_hold_when_rule_not_triggered():
    module = load_module()
    history = pd.DataFrame(
        {
            "trade_date": pd.to_datetime(["2026-04-21", "2026-04-22", "2026-04-23"]),
            "close": [10.2, 10.3, 10.4],
        }
    )

    report = module.evaluate_position_signal(
        as_of_date=pd.Timestamp("2026-04-23"),
        history=history,
        strategy_name="fail_to_rebound_d3_hold_20d",
        entry_date=pd.Timestamp("2026-04-21"),
        entry_price=10.0,
        symbol="518800.SH",
    )

    assert report["action"] == "hold"
    assert report["reason"] == "rule_not_triggered"
