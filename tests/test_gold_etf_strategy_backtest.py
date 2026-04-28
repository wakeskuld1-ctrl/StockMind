import importlib.util
from pathlib import Path
import sys

import pandas as pd


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_etf_strategy_backtest.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_etf_strategy_backtest", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_build_policy_signal_table_marks_all_exit_policies():
    module = load_module()
    signals = pd.DataFrame(
        {
            "trade_date": pd.to_datetime(["2025-01-01", "2025-01-02", "2025-01-03"]),
            "score": [0.9, 0.5, 0.1],
        }
    )

    policy_table = module.build_policy_signal_table(signals)

    assert set(policy_table["policy_name"]) == {
        "hold_15d_baseline",
        "time_or_break_ma10",
        "time_or_fail_to_rebound",
    }
    baseline = policy_table[policy_table["policy_name"] == "hold_15d_baseline"].sort_values("trade_date")
    assert list(baseline["selected_flag"]) == [1, 1, 1]


def test_map_policy_to_etf_trades_keeps_selected_signals_only():
    module = load_module()
    policy = pd.DataFrame(
        {
            "trade_date": pd.to_datetime(["2025-01-01", "2025-01-02"]),
            "policy_name": ["hold_15d_baseline", "hold_15d_baseline"],
            "selected_flag": [1, 0],
        }
    )
    etf = pd.DataFrame(
        {
            "trade_date": pd.to_datetime(["2025-01-01", "2025-01-02"]),
            "future_ret_15d": [0.03, 0.01],
            "hold_max_drawdown_15d": [-0.01, -0.02],
            "hold_max_runup_15d": [0.04, 0.03],
        }
    )

    trades = module.map_policy_to_etf_trades(policy, etf, symbol="518880.SH")

    assert len(trades) == 1
    assert trades["symbol"].iloc[0] == "518880.SH"
    assert trades["future_ret_15d"].iloc[0] == 0.03


def test_summarize_etf_backtest_reports_terminal_capital_and_drawdown():
    module = load_module()
    trades = pd.DataFrame(
        {
            "symbol": ["518880.SH", "518880.SH"],
            "policy_name": ["hold_15d_baseline", "hold_15d_baseline"],
            "trade_date": pd.to_datetime(["2025-01-01", "2025-02-01"]),
            "future_ret_15d": [0.10, -0.05],
            "hold_max_drawdown_15d": [-0.01, -0.03],
            "hold_max_runup_15d": [0.12, 0.02],
        }
    )

    summary = module.summarize_etf_backtest(trades, initial_capital=1_000_000)

    row = summary.iloc[0]
    assert round(row["terminal_capital"], 2) == 1_045_000.00
    assert round(row["total_return"], 6) == 0.045
    assert row["sample_count"] == 2


def test_rank_final_etf_choices_prefers_higher_terminal_with_reasonable_drawdown():
    module = load_module()
    summary = pd.DataFrame(
        [
            {"symbol": "518880.SH", "policy_name": "hold_15d_baseline", "terminal_capital": 1_500_000, "max_drawdown": -0.14, "cagr": 0.08},
            {"symbol": "518800.SH", "policy_name": "hold_15d_baseline", "terminal_capital": 1_450_000, "max_drawdown": -0.12, "cagr": 0.075},
        ]
    )

    ranked = module.rank_final_etf_choices(summary)

    assert ranked.iloc[0]["symbol"] == "518880.SH"


def test_build_policy_signal_table_uses_true_rule_baseline_dates():
    module = load_module()
    baseline = pd.DataFrame(
        {
            "trade_date": pd.to_datetime(["2025-01-01", "2025-01-02", "2025-01-03", "2025-01-06"]),
            "selected_flag": [1, 1, 1, 1],
        }
    )
    scored = pd.DataFrame(
        {
            "trade_date": pd.to_datetime(["2025-01-01", "2025-01-03", "2025-01-06"]),
            "score": [0.9, 0.5, 0.1],
        }
    )

    policy_table = module.build_policy_signal_table(baseline, scored)
    baseline_dates = policy_table[policy_table["policy_name"] == "hold_15d_baseline"]["trade_date"].sort_values().tolist()

    assert baseline_dates == baseline["trade_date"].tolist()


def test_run_single_position_backtest_uses_t1_open_and_skips_overlap():
    module = load_module()
    etf = pd.DataFrame(
        {
            "trade_date": pd.to_datetime(
                ["2025-01-01", "2025-01-02", "2025-01-03", "2025-01-06", "2025-01-07", "2025-01-08", "2025-01-09"]
            ),
            "open": [10.0, 10.5, 11.0, 12.0, 11.8, 12.5, 12.8],
            "close": [10.2, 10.8, 11.5, 11.9, 12.2, 12.7, 12.9],
        }
    )
    policy = pd.DataFrame(
        {
            "trade_date": pd.to_datetime(["2025-01-01", "2025-01-02", "2025-01-06"]),
            "policy_name": ["hold_15d_baseline", "hold_15d_baseline", "hold_15d_baseline"],
            "selected_flag": [1, 1, 1],
        }
    )

    trade_log, equity_curve = module.run_single_position_backtest(
        policy_signals=policy,
        etf_history=etf,
        symbol="518880.SH",
        hold_days=2,
        initial_capital=100_000.0,
    )

    assert len(trade_log) == 2
    first_trade = trade_log.iloc[0]
    second_trade = trade_log.iloc[1]
    assert str(first_trade["signal_date"].date()) == "2025-01-01"
    assert str(first_trade["entry_date"].date()) == "2025-01-02"
    assert first_trade["entry_price"] == 10.5
    assert str(first_trade["exit_date"].date()) == "2025-01-06"
    assert first_trade["exit_price"] == 12.0
    assert round(first_trade["trade_return"], 6) == round(12.0 / 10.5 - 1.0, 6)
    assert str(second_trade["signal_date"].date()) == "2025-01-06"
    assert str(second_trade["entry_date"].date()) == "2025-01-07"
    assert str(equity_curve[equity_curve["position_flag"] == 1]["date"].min().date()) == "2025-01-02"
    assert "cash" in equity_curve.columns


def test_summarize_single_position_backtest_reports_trade_metrics():
    module = load_module()
    trade_log = pd.DataFrame(
        {
            "symbol": ["518880.SH", "518880.SH"],
            "policy_name": ["hold_15d_baseline", "hold_15d_baseline"],
            "signal_date": pd.to_datetime(["2025-01-01", "2025-02-01"]),
            "entry_date": pd.to_datetime(["2025-01-02", "2025-02-03"]),
            "exit_date": pd.to_datetime(["2025-01-06", "2025-02-05"]),
            "trade_return": [0.10, -0.05],
            "hold_calendar_days": [4, 2],
        }
    )
    equity_curve = pd.DataFrame(
        {
            "date": pd.to_datetime(["2025-01-02", "2025-01-06", "2025-02-03", "2025-02-05"]),
            "equity": [1_000_000.0, 1_100_000.0, 1_100_000.0, 1_045_000.0],
        }
    )

    summary = module.summarize_single_position_backtest(
        trade_log=trade_log,
        equity_curve=equity_curve,
        symbol="518880.SH",
        policy_name="hold_15d_baseline",
        initial_capital=1_000_000.0,
    )

    assert round(summary["terminal_capital"], 2) == 1_045_000.00
    assert round(summary["total_return"], 6) == 0.045
    assert summary["sample_count"] == 2


def test_break_ma10_exits_next_open_before_hold_limit():
    module = load_module()
    etf = pd.DataFrame(
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
                    "2025-01-13",
                ]
            ),
            "open": [10.0, 10.1, 10.2, 10.3, 10.2, 9.9, 9.7, 9.6, 9.5],
            "close": [10.0, 10.1, 10.2, 10.3, 10.1, 9.8, 9.6, 9.5, 9.4],
            "ma10": [9.9, 10.0, 10.05, 10.1, 10.05, 10.0, 9.95, 9.9, 9.85],
        }
    )
    policy = pd.DataFrame(
        {
            "trade_date": pd.to_datetime(["2025-01-01"]),
            "policy_name": ["time_or_break_ma10"],
            "selected_flag": [1],
        }
    )

    trade_log, _ = module.run_single_position_backtest(
        policy_signals=policy,
        etf_history=etf,
        symbol="518880.SH",
        hold_days=5,
        initial_capital=100_000.0,
        exit_policy_name="time_or_break_ma10",
    )

    assert len(trade_log) == 1
    trade = trade_log.iloc[0]
    assert str(trade["trigger_date"].date()) == "2025-01-08"
    assert str(trade["exit_date"].date()) == "2025-01-09"
    assert trade["exit_reason"] == "break_ma10"


def test_fail_to_rebound_exits_after_fifth_holding_day_check():
    module = load_module()
    etf = pd.DataFrame(
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
                    "2025-01-13",
                ]
            ),
            "open": [10.0, 10.0, 9.9, 9.8, 9.85, 9.9, 9.95, 9.7, 9.6],
            "close": [10.0, 9.95, 9.85, 9.8, 9.82, 9.9, 9.92, 9.75, 9.65],
            "ma10": [9.8] * 9,
        }
    )
    policy = pd.DataFrame(
        {
            "trade_date": pd.to_datetime(["2025-01-01"]),
            "policy_name": ["time_or_fail_to_rebound"],
            "selected_flag": [1],
        }
    )

    trade_log, _ = module.run_single_position_backtest(
        policy_signals=policy,
        etf_history=etf,
        symbol="518880.SH",
        hold_days=5,
        initial_capital=100_000.0,
        exit_policy_name="time_or_fail_to_rebound",
    )

    assert len(trade_log) == 1
    trade = trade_log.iloc[0]
    assert str(trade["trigger_date"].date()) == "2025-01-08"
    assert str(trade["exit_date"].date()) == "2025-01-09"
    assert trade["exit_reason"] == "fail_to_rebound"


def test_hold_15d_baseline_uses_time_exit_when_no_rule_trigger():
    module = load_module()
    etf = pd.DataFrame(
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
                ]
            ),
            "open": [10.0, 10.1, 10.2, 10.3, 10.4, 10.5, 10.6],
            "close": [10.0, 10.1, 10.2, 10.3, 10.4, 10.5, 10.6],
            "ma10": [9.9] * 7,
        }
    )
    policy = pd.DataFrame(
        {
            "trade_date": pd.to_datetime(["2025-01-01"]),
            "policy_name": ["hold_15d_baseline"],
            "selected_flag": [1],
        }
    )

    trade_log, _ = module.run_single_position_backtest(
        policy_signals=policy,
        etf_history=etf,
        symbol="518880.SH",
        hold_days=2,
        initial_capital=100_000.0,
        exit_policy_name="hold_15d_baseline",
    )

    trade = trade_log.iloc[0]
    assert trade["exit_reason"] == "time_exit"
    assert str(trade["exit_date"].date()) == "2025-01-06"
