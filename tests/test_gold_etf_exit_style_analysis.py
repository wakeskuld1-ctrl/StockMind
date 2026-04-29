import importlib.util
from pathlib import Path
import sys

import pandas as pd


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_etf_exit_style_analysis.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_etf_exit_style_analysis", SCRIPT_PATH)
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


def test_high_watermark_drawdown_exit_starts_after_d15_and_exits_next_open():
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
            11.7,
            11.6,
            11.45,
            11.3,
            11.2,
        ]
    )

    exit_idx, reason = module.resolve_post15_exit(
        history=history,
        first_entry_idx=0,
        weighted_entry_price=10.0,
        rule={"family": "high_watermark_drawdown", "drawdown": 0.02},
        rebound_check_day=5,
        anchor_day=15,
        max_hold_days=30,
    )

    assert exit_idx == 18
    assert reason == "hwm_dd_0.020_d18"


def test_ma_trend_exit_uses_close_below_selected_ma_after_d15():
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
            11.5,
            11.6,
            11.7,
            11.1,
            11.0,
            10.9,
        ]
    )

    exit_idx, reason = module.resolve_post15_exit(
        history=history,
        first_entry_idx=0,
        weighted_entry_price=10.0,
        rule={"family": "ma_trend", "ma_window": 5},
        rebound_check_day=5,
        anchor_day=15,
        max_hold_days=30,
    )

    assert exit_idx == 19
    assert reason == "ma5_break_d19"


def test_staged_profit_giveback_exits_after_profit_zone_is_reached():
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
            10.3,
            10.35,
            10.4,
            10.42,
            10.4,
            10.4,
            10.55,
            10.7,
            10.62,
            10.34,
            10.3,
        ]
    )

    exit_idx, reason = module.resolve_post15_exit(
        history=history,
        first_entry_idx=0,
        weighted_entry_price=10.0,
        rule={"family": "staged_profit_giveback", "profit_trigger": 0.05, "giveback_ratio": 0.5},
        rebound_check_day=5,
        anchor_day=15,
        max_hold_days=30,
    )

    assert exit_idx == 20
    assert reason == "profit_0.050_giveback_0.50_d20"


def test_build_exit_rule_space_contains_three_interpretable_families():
    module = load_module()

    rules = module.build_exit_rule_space()
    families = {rule["family"] for rule in rules}

    assert families == {"high_watermark_drawdown", "ma_trend", "staged_profit_giveback"}
    assert any(rule["family"] == "high_watermark_drawdown" and rule["drawdown"] == 0.02 for rule in rules)
    assert any(rule["family"] == "ma_trend" and rule["ma_window"] == 10 for rule in rules)
    assert any(
        rule["family"] == "staged_profit_giveback" and rule["profit_trigger"] == 0.05 and rule["giveback_ratio"] == 0.5
        for rule in rules
    )


def test_rank_exit_results_prioritizes_return_drawdown_ratio_then_total_return():
    module = load_module()
    summary = pd.DataFrame(
        [
            {"config_name": "a", "total_return": 0.9, "max_drawdown": -0.2},
            {"config_name": "b", "total_return": 0.8, "max_drawdown": -0.1},
            {"config_name": "c", "total_return": 1.0, "max_drawdown": -0.2},
        ]
    )

    ranked = module.rank_exit_results(summary)

    assert list(ranked["config_name"]) == ["b", "c", "a"]
