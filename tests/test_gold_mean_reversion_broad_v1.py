import importlib.util
from pathlib import Path
import sys

import pandas as pd


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_mean_reversion_broad_v1.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_mean_reversion_broad_v1", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_flag_broad_mean_reversion_events_accepts_wider_oversold_band():
    module = load_module()
    frame = pd.DataFrame(
        {
            "ret_5d": [-0.035, -0.01, -0.045],
            "close_vs_ma20": [-0.03, -0.01, -0.05],
            "failure_risk_flag": [0, 0, 1],
        }
    )

    flagged = module.flag_broad_mean_reversion_events(frame)

    assert list(flagged["broad_event_flag"]) == [1, 0, 0]


def test_build_trade_table_generates_forward_metrics_for_broad_events():
    module = load_module()
    frame = pd.DataFrame(
        [
            {
                "trade_date": "2026-01-01",
                "broad_event_flag": 1,
                "future_ret_5d": 0.02,
                "hold_max_drawdown_5d": -0.01,
                "hold_max_runup_5d": 0.03,
                "future_ret_10d": 0.03,
                "hold_max_drawdown_10d": -0.02,
                "hold_max_runup_10d": 0.05,
            },
            {
                "trade_date": "2026-01-02",
                "broad_event_flag": 0,
                "future_ret_5d": 0.01,
                "hold_max_drawdown_5d": -0.01,
                "hold_max_runup_5d": 0.02,
                "future_ret_10d": 0.02,
                "hold_max_drawdown_10d": -0.01,
                "hold_max_runup_10d": 0.03,
            },
        ]
    )

    trades = module.build_trade_table(frame, holding_windows=[5, 10])

    assert len(trades) == 2
    assert set(trades["holding_days"]) == {5, 10}
    assert set(trades["trade_date"]) == {"2026-01-01"}


def test_summarize_yearly_stability_reports_per_year_counts_and_returns():
    module = load_module()
    trades = pd.DataFrame(
        [
            {"trade_date": "2025-01-03", "holding_days": 5, "future_return": 0.02, "max_drawdown": -0.01},
            {"trade_date": "2025-06-03", "holding_days": 5, "future_return": -0.01, "max_drawdown": -0.02},
            {"trade_date": "2026-01-03", "holding_days": 5, "future_return": 0.03, "max_drawdown": -0.01},
        ]
    )

    summary = module.summarize_yearly_stability(trades)

    assert set(summary["year"]) == {2025, 2026}
    assert summary[summary["year"] == 2025]["sample_count"].iloc[0] == 2
    assert round(summary[summary["year"] == 2026]["avg_return"].iloc[0], 6) == 0.03


def test_extract_broad_v1_verdict_requires_minimum_sample_and_positive_edge():
    module = load_module()
    overall = pd.DataFrame(
        [
            {"holding_days": 5, "sample_count": 40, "win_rate": 0.62, "avg_return": 0.012, "avg_max_drawdown": -0.01, "return_per_day": 0.0024},
            {"holding_days": 10, "sample_count": 40, "win_rate": 0.70, "avg_return": 0.018, "avg_max_drawdown": -0.015, "return_per_day": 0.0018},
        ]
    )
    yearly = pd.DataFrame(
        [
            {"holding_days": 5, "year": 2021, "sample_count": 8, "avg_return": 0.01},
            {"holding_days": 5, "year": 2022, "sample_count": 7, "avg_return": 0.02},
            {"holding_days": 5, "year": 2023, "sample_count": 9, "avg_return": -0.01},
            {"holding_days": 5, "year": 2024, "sample_count": 8, "avg_return": 0.015},
            {"holding_days": 10, "year": 2021, "sample_count": 8, "avg_return": 0.005},
            {"holding_days": 10, "year": 2022, "sample_count": 7, "avg_return": 0.01},
            {"holding_days": 10, "year": 2023, "sample_count": 9, "avg_return": 0.02},
            {"holding_days": 10, "year": 2024, "sample_count": 8, "avg_return": 0.018},
        ]
    )

    verdict = module.extract_broad_v1_verdict(overall, yearly)

    assert verdict["holding_days"] == 5
    assert verdict["is_repeatable_pattern"] is True
