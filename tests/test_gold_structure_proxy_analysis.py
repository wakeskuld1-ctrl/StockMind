import importlib.util
from pathlib import Path
import sys

import pandas as pd


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_structure_proxy_analysis.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_structure_proxy_analysis", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_compute_time_efficiency_keeps_short_holding_more_capital_efficient():
    module = load_module()

    short_metrics = module.compute_time_efficiency(avg_return=0.01, holding_days=3)
    long_metrics = module.compute_time_efficiency(avg_return=0.03, holding_days=30)

    assert round(short_metrics["return_per_day"], 6) == round(0.01 / 3, 6)
    assert round(long_metrics["return_per_day"], 6) == round(0.03 / 30, 6)
    assert short_metrics["annualized_equivalent"] > long_metrics["annualized_equivalent"]


def test_build_environment_flags_creates_usd_and_oil_regime_columns():
    module = load_module()
    frame = pd.DataFrame(
        {
            "trade_date": pd.to_datetime(["2026-01-01", "2026-01-02", "2026-01-03", "2026-01-04"]),
            "usd_close": [100.0, 101.0, 102.0, 103.0],
            "oil_close": [70.0, 69.0, 68.0, 67.0],
        }
    )

    flagged = module.build_environment_flags(frame, lookback_days=2)

    assert "usd_regime" in flagged.columns
    assert "oil_regime" in flagged.columns
    assert "resonance_regime" in flagged.columns
    assert flagged.loc[3, "usd_regime"] == "usd_strong"
    assert flagged.loc[3, "oil_regime"] == "oil_weak"
    assert flagged.loc[3, "resonance_regime"] == "usd_up_oil_down"


def test_summarize_structure_by_environment_reports_all_holding_windows():
    module = load_module()
    frame = pd.DataFrame(
        [
            {
                "structure_name": "trend_continuation",
                "resonance_regime": "usd_down_oil_up",
                "future_ret_3d": 0.02,
                "hold_max_drawdown_3d": -0.01,
                "hold_max_runup_3d": 0.04,
                "future_ret_5d": 0.03,
                "hold_max_drawdown_5d": -0.02,
                "hold_max_runup_5d": 0.05,
                "future_ret_10d": 0.04,
                "hold_max_drawdown_10d": -0.03,
                "hold_max_runup_10d": 0.06,
                "future_ret_15d": 0.05,
                "hold_max_drawdown_15d": -0.04,
                "hold_max_runup_15d": 0.07,
                "future_ret_20d": 0.06,
                "hold_max_drawdown_20d": -0.05,
                "hold_max_runup_20d": 0.08,
                "future_ret_30d": 0.07,
                "hold_max_drawdown_30d": -0.06,
                "hold_max_runup_30d": 0.09,
            },
            {
                "structure_name": "trend_continuation",
                "resonance_regime": "usd_down_oil_up",
                "future_ret_3d": -0.01,
                "hold_max_drawdown_3d": -0.02,
                "hold_max_runup_3d": 0.01,
                "future_ret_5d": 0.01,
                "hold_max_drawdown_5d": -0.03,
                "hold_max_runup_5d": 0.02,
                "future_ret_10d": 0.02,
                "hold_max_drawdown_10d": -0.04,
                "hold_max_runup_10d": 0.03,
                "future_ret_15d": 0.01,
                "hold_max_drawdown_15d": -0.05,
                "hold_max_runup_15d": 0.04,
                "future_ret_20d": 0.00,
                "hold_max_drawdown_20d": -0.06,
                "hold_max_runup_20d": 0.05,
                "future_ret_30d": -0.02,
                "hold_max_drawdown_30d": -0.08,
                "hold_max_runup_30d": 0.06,
            },
        ]
    )

    summary = module.summarize_structure_by_environment(frame)

    assert set(summary["holding_days"]) == {3, 5, 10, 15, 20, 30}
    assert set(summary["structure_name"]) == {"trend_continuation"}
    assert set(summary["resonance_regime"]) == {"usd_down_oil_up"}
    row_3d = summary[summary["holding_days"] == 3].iloc[0]
    assert row_3d["sample_count"] == 2
    assert round(row_3d["win_rate"], 6) == 0.5
