import importlib.util
from pathlib import Path
import sys

import pandas as pd


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_etf_structure_analysis.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_etf_structure_analysis", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_compute_time_efficiency_converts_return_to_daily_and_annualized_equivalent():
    module = load_module()

    metrics = module.compute_time_efficiency(avg_return=0.03, holding_days=30)

    assert round(metrics["return_per_day"], 6) == 0.001
    assert metrics["annualized_equivalent"] > 0


def test_summarize_holding_window_reports_win_rate_and_time_efficiency():
    module = load_module()
    frame = pd.DataFrame(
        [
            {"future_ret_10d": 0.02, "hold_max_drawdown_10d": -0.01, "hold_max_runup_10d": 0.03},
            {"future_ret_10d": -0.01, "hold_max_drawdown_10d": -0.03, "hold_max_runup_10d": 0.01},
            {"future_ret_10d": 0.03, "hold_max_drawdown_10d": -0.02, "hold_max_runup_10d": 0.04},
        ]
    )

    summary = module.summarize_holding_window(frame, holding_days=10)

    assert summary["sample_count"] == 3
    assert round(summary["win_rate"], 6) == round(2 / 3, 6)
    assert round(summary["avg_return"], 6) == 0.013333
    assert "annualized_equivalent" in summary
