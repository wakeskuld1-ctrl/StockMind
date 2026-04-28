import importlib.util
from pathlib import Path
import sys

import pandas as pd


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_etf_all_proxy_premium_analysis.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_etf_all_proxy_premium_analysis", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_compute_proxy_premium_adds_anchor_and_proxy_columns():
    module = load_module()
    frame = pd.DataFrame(
        {
            "trade_date": pd.date_range("2025-01-01", periods=6, freq="B"),
            "symbol": ["518800.SH"] * 6,
            "close": [2.00, 2.02, 2.04, 2.03, 2.06, 2.10],
            "gold_close": [100, 101, 102, 103, 104, 105],
        }
    )

    out = module.compute_proxy_premium(frame, window=3)

    assert "ratio_anchor" in out.columns
    assert "premium_proxy" in out.columns
    assert out["premium_proxy"].notna().sum() >= 2


def test_apply_premium_filter_respects_threshold():
    module = load_module()
    signals = pd.DataFrame(
        {
            "trade_date": pd.to_datetime(["2025-01-01", "2025-01-02", "2025-01-03"]),
            "selected_flag": [1, 1, 1],
            "premium_proxy": [0.002, 0.012, -0.003],
        }
    )

    filtered = module.apply_premium_filter(signals, ceiling=0.01, filter_name="light")

    assert list(filtered["selected_flag"]) == [1, 0, 1]
    assert set(filtered["premium_filter"]) == {"light"}


def test_rank_proxy_premium_results_prefers_higher_terminal_and_lower_drawdown():
    module = load_module()
    summary = pd.DataFrame(
        [
            {"symbol": "518800.SH", "strategy_name": "d5_hold20", "premium_filter": "light", "terminal_capital": 1_800_000.0, "max_drawdown": -0.18, "cagr": 0.12},
            {"symbol": "518880.SH", "strategy_name": "d3_hold20", "premium_filter": "light", "terminal_capital": 1_700_000.0, "max_drawdown": -0.12, "cagr": 0.11},
        ]
    )

    ranked = module.rank_proxy_premium_results(summary)

    assert ranked.iloc[0]["symbol"] == "518800.SH"


def test_build_strategy_grid_contains_three_strategies():
    module = load_module()

    grid = module.build_strategy_grid()

    assert set(grid["strategy_name"]) == {
        "hold_15d_baseline",
        "fail_to_rebound_d3_hold_20d",
        "fail_to_rebound_d5_hold_20d",
    }
