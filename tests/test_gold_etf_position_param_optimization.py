import importlib.util
from pathlib import Path
import sys

import pandas as pd
import pytest


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_etf_position_param_optimization.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_etf_position_param_optimization", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def build_sample_etf_history():
    return pd.DataFrame(
        {
            "trade_date": pd.to_datetime(
                [
                    "2025-01-01",
                    "2025-01-02",
                    "2025-01-03",
                    "2025-01-06",
                    "2025-01-07",
                    "2025-01-08",
                    "2025-01-09",
                    "2025-01-10",
                ]
            ),
            "symbol": ["518800.SH"] * 8,
            "open": [10.0, 10.0, 9.8, 9.7, 10.0, 10.4, 10.5, 10.6],
            "high": [10.1, 10.0, 9.9, 10.0, 10.3, 10.5, 10.6, 10.7],
            "low": [9.9, 9.7, 9.5, 9.6, 9.9, 10.3, 10.4, 10.5],
            "close": [10.0, 9.8, 9.6, 9.95, 10.2, 10.45, 10.55, 10.65],
        }
    )


def build_sample_gold_signals():
    return pd.DataFrame(
        {
            "trade_date": pd.to_datetime(["2025-01-01", "2025-01-03"]),
            "ret_5d": [-0.03, -0.025],
            "close_vs_ma20": [-0.02, -0.018],
            "parent_signal": [1, 1],
        }
    )


def test_validate_config_rejects_overweight_staged_entries():
    module = load_module()
    config = {
        "first_entry_weight": 0.5,
        "allow_second_entry": True,
        "second_entry_trigger_drawdown": -0.03,
        "second_entry_requires_parent_signal": True,
        "second_entry_weight": 0.3,
        "allow_third_entry": True,
        "third_entry_trigger_drawdown": -0.04,
        "third_entry_requires_parent_signal": True,
        "third_entry_weight": 0.3,
        "max_total_weight": 1.0,
        "max_hold_days": 20,
        "rebound_check_day": 5,
    }

    with pytest.raises(ValueError):
        module.validate_config(config)


def test_build_parameter_grid_emits_only_valid_configs():
    module = load_module()
    grid = module.build_parameter_grid(
        {
            "first_entry_weight": [0.3],
            "allow_second_entry": [False, True],
            "second_entry_trigger_drawdown": [-0.03],
            "second_entry_requires_parent_signal": [True],
            "second_entry_weight": [0.3],
            "allow_third_entry": [False],
            "third_entry_trigger_drawdown": [-0.04],
            "third_entry_requires_parent_signal": [True],
            "third_entry_weight": [0.3],
            "max_total_weight": [1.0],
            "max_hold_days": [20],
            "rebound_check_day": [5],
        }
    )

    assert len(grid) == 2
    assert all(config["first_entry_weight"] == 0.3 for config in grid)


def test_build_parameter_grid_deduplicates_disabled_layer_variants():
    module = load_module()
    grid = module.build_parameter_grid(
        {
            "first_entry_weight": [0.5],
            "allow_second_entry": [False],
            "second_entry_trigger_drawdown": [-0.03],
            "second_entry_requires_parent_signal": [True],
            "second_entry_weight": [0.0, 0.25],
            "allow_third_entry": [False, True],
            "third_entry_trigger_drawdown": [-0.04],
            "third_entry_requires_parent_signal": [True],
            "third_entry_weight": [0.0, 0.25],
            "max_total_weight": [1.0],
            "max_hold_days": [20],
            "rebound_check_day": [5],
        }
    )

    assert len(grid) == 1


def test_run_backtest_adds_second_layer_after_drawdown_trigger():
    module = load_module()
    history = build_sample_etf_history()
    gold = build_sample_gold_signals()
    config = module.validate_config(
        {
            "first_entry_weight": 0.5,
            "allow_second_entry": True,
            "second_entry_trigger_drawdown": -0.03,
            "second_entry_requires_parent_signal": True,
            "second_entry_weight": 0.5,
            "allow_third_entry": False,
            "third_entry_trigger_drawdown": -0.04,
            "third_entry_requires_parent_signal": True,
            "third_entry_weight": 0.0,
            "max_total_weight": 1.0,
            "max_hold_days": 5,
            "rebound_check_day": 3,
        }
    )

    trade_log, equity_curve = module.run_position_management_backtest(
        etf_history=history,
        gold_signals=gold,
        config=config,
        initial_capital=100_000.0,
        symbol="518800.SH",
    )

    assert len(trade_log) == 1
    trade = trade_log.iloc[0]
    assert trade["entry_layers"] == 2
    assert trade["layer_entry_dates"] == "2025-01-02|2025-01-06"
    assert trade["layer_entry_prices"] == "10.0000|9.7000"
    assert trade["exit_reason"] == "time_exit"
    assert len(equity_curve) == len(history)


def test_run_backtest_blocks_second_layer_without_parent_signal_confirmation():
    module = load_module()
    history = build_sample_etf_history()
    gold = build_sample_gold_signals()
    gold.loc[gold["trade_date"] == pd.Timestamp("2025-01-03"), "parent_signal"] = 0
    config = module.validate_config(
        {
            "first_entry_weight": 0.5,
            "allow_second_entry": True,
            "second_entry_trigger_drawdown": -0.03,
            "second_entry_requires_parent_signal": True,
            "second_entry_weight": 0.5,
            "allow_third_entry": False,
            "third_entry_trigger_drawdown": -0.04,
            "third_entry_requires_parent_signal": True,
            "third_entry_weight": 0.0,
            "max_total_weight": 1.0,
            "max_hold_days": 5,
            "rebound_check_day": 3,
        }
    )

    trade_log, _ = module.run_position_management_backtest(
        etf_history=history,
        gold_signals=gold,
        config=config,
        initial_capital=100_000.0,
        symbol="518800.SH",
    )

    trade = trade_log.iloc[0]
    assert trade["entry_layers"] == 1
    assert trade["layer_entry_dates"] == "2025-01-02"


def test_run_backtest_records_repair_day_when_close_recovers_weighted_cost():
    module = load_module()
    history = build_sample_etf_history()
    gold = build_sample_gold_signals()
    config = module.validate_config(
        {
            "first_entry_weight": 0.5,
            "allow_second_entry": True,
            "second_entry_trigger_drawdown": -0.03,
            "second_entry_requires_parent_signal": True,
            "second_entry_weight": 0.5,
            "allow_third_entry": False,
            "third_entry_trigger_drawdown": -0.04,
            "third_entry_requires_parent_signal": True,
            "third_entry_weight": 0.0,
            "max_total_weight": 1.0,
            "max_hold_days": 5,
            "rebound_check_day": 3,
        }
    )

    trade_log, _ = module.run_position_management_backtest(
        etf_history=history,
        gold_signals=gold,
        config=config,
        initial_capital=100_000.0,
        symbol="518800.SH",
    )

    trade = trade_log.iloc[0]
    assert trade["repair_days"] == 3
    assert str(pd.to_datetime(trade["repair_date"]).date()) == "2025-01-06"


def test_summarize_backtest_reports_repair_metrics():
    module = load_module()
    trade_log = pd.DataFrame(
        {
            "trade_return": [0.05, -0.01, 0.03],
            "entry_layers": [1, 2, 1],
            "hold_trading_days": [10, 8, 12],
            "exit_capital": [1_050_000.0, 1_039_500.0, 1_070_685.0],
            "repair_days": [2.0, None, 6.0],
        }
    )
    equity_curve = pd.DataFrame(
        {
            "date": pd.to_datetime(["2025-01-01", "2025-01-10"]),
            "equity": [1_000_000.0, 1_070_685.0],
            "drawdown": [0.0, -0.05],
        }
    )

    summary = module.summarize_backtest(
        trade_log=trade_log,
        equity_curve=equity_curve,
        config_name="demo",
        initial_capital=1_000_000.0,
    )

    assert summary["median_repair_days"] == 4.0
    assert round(summary["repair_within_5d"], 6) == round(1 / 3, 6)
