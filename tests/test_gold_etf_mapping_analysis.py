import importlib.util
from pathlib import Path
import sys

import pandas as pd


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_etf_mapping_analysis.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_etf_mapping_analysis", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_build_etf_universe_contains_11_gold_etfs_and_groups():
    module = load_module()

    universe = module.build_etf_universe()

    assert len(universe) == 11
    assert set(universe["group_name"]) == {"au9999", "shanghai_gold"}


def test_normalize_yfinance_history_flattens_columns_and_adds_symbol():
    module = load_module()
    frame = pd.DataFrame(
        {
            ("Open", "518880.SS"): [1.0],
            ("High", "518880.SS"): [1.1],
            ("Low", "518880.SS"): [0.9],
            ("Close", "518880.SS"): [1.05],
            ("Adj Close", "518880.SS"): [1.04],
            ("Volume", "518880.SS"): [1000],
        },
        index=pd.to_datetime(["2024-01-02"]),
    )

    normalized = module.normalize_yfinance_history(frame, symbol="518880.SH")

    assert normalized["symbol"].iloc[0] == "518880.SH"
    assert "close" in normalized.columns
    assert normalized["close"].iloc[0] == 1.05


def test_map_rule_signals_to_etf_returns_reports_forward_metrics():
    module = load_module()
    gold_signals = pd.DataFrame({"trade_date": pd.to_datetime(["2024-01-02", "2024-01-03"])})
    etf = pd.DataFrame(
        {
            "trade_date": pd.to_datetime(["2024-01-02", "2024-01-03"]),
            "future_ret_15d": [0.03, 0.01],
            "hold_max_drawdown_15d": [-0.01, -0.02],
            "hold_max_runup_15d": [0.04, 0.03],
            "proxy_turnover": [1000000, 1200000],
        }
    )

    summary = module.map_rule_signals_to_etf_returns(gold_signals, etf)

    assert summary["signal_count"] == 2
    assert round(summary["avg_return_15d"], 6) == 0.02
    assert round(summary["win_rate_15d"], 6) == 1.0


def test_rank_etfs_prefers_better_signal_carry_and_liquidity():
    module = load_module()
    summary = pd.DataFrame(
        [
            {"symbol": "A", "signal_count": 50, "win_rate_15d": 0.75, "avg_return_15d": 0.025, "avg_proxy_turnover": 10000000, "history_days": 1000},
            {"symbol": "B", "signal_count": 50, "win_rate_15d": 0.70, "avg_return_15d": 0.020, "avg_proxy_turnover": 5000000, "history_days": 1000},
        ]
    )

    ranked = module.rank_etfs(summary)

    assert ranked.iloc[0]["symbol"] == "A"
