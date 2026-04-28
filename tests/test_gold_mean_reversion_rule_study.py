import importlib.util
from pathlib import Path
import sys

import pandas as pd


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_mean_reversion_rule_study.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_mean_reversion_rule_study", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_assign_bucket_labels_separates_moderate_and_extreme_oversold_depth():
    module = load_module()
    frame = pd.DataFrame(
        {
            "ret_5d": [-0.06, -0.03, -0.015],
            "close_vs_ma20": [-0.05, -0.025, -0.01],
        }
    )

    bucketed = module.assign_bucket_labels(frame)

    assert list(bucketed["ret_5d_bucket"]) == ["ret5_extreme", "ret5_moderate", "ret5_shallow"]
    assert list(bucketed["ma20_gap_bucket"]) == ["ma20_gap_deep", "ma20_gap_moderate", "ma20_gap_shallow"]


def test_build_failure_flags_marks_high_volatility_shock_and_downtrend_break():
    module = load_module()
    frame = pd.DataFrame(
        {
            "gold_close": [100.0, 98.0],
            "gold_high": [101.0, 100.0],
            "gold_low": [99.0, 93.0],
            "ma20": [99.0, 97.0],
            "ma30": [98.0, 98.5],
            "volume_ratio_1d_vs_20d": [1.0, 1.8],
        }
    )

    flagged = module.build_failure_flags(frame)

    assert flagged.loc[1, "high_volatility_shock_flag"] == 1
    assert flagged.loc[1, "downtrend_break_flag"] == 1
    assert flagged.loc[1, "failure_risk_flag"] == 1


def test_summarize_rule_layers_reports_bucket_and_environment_metrics():
    module = load_module()
    frame = pd.DataFrame(
        [
            {
                "ret_5d_bucket": "ret5_moderate",
                "ma20_gap_bucket": "ma20_gap_moderate",
                "resonance_regime": "usd_down_oil_up",
                "failure_risk_flag": 0,
                "future_ret_5d": 0.02,
                "hold_max_drawdown_5d": -0.01,
                "hold_max_runup_5d": 0.03,
                "future_ret_10d": 0.03,
                "hold_max_drawdown_10d": -0.02,
                "hold_max_runup_10d": 0.05,
                "future_ret_15d": 0.04,
                "hold_max_drawdown_15d": -0.02,
                "hold_max_runup_15d": 0.06,
            },
            {
                "ret_5d_bucket": "ret5_moderate",
                "ma20_gap_bucket": "ma20_gap_moderate",
                "resonance_regime": "usd_down_oil_up",
                "failure_risk_flag": 0,
                "future_ret_5d": -0.01,
                "hold_max_drawdown_5d": -0.03,
                "hold_max_runup_5d": 0.02,
                "future_ret_10d": 0.01,
                "hold_max_drawdown_10d": -0.03,
                "hold_max_runup_10d": 0.03,
                "future_ret_15d": 0.02,
                "hold_max_drawdown_15d": -0.04,
                "hold_max_runup_15d": 0.04,
            },
        ]
    )

    summary = module.summarize_rule_layers(frame, holding_windows=[5, 10, 15])

    assert set(summary["holding_days"]) == {5, 10, 15}
    assert set(summary["ret_5d_bucket"]) == {"ret5_moderate"}
    assert set(summary["ma20_gap_bucket"]) == {"ma20_gap_moderate"}
    assert set(summary["resonance_regime"]) == {"usd_down_oil_up"}
    row_5d = summary[summary["holding_days"] == 5].iloc[0]
    assert row_5d["sample_count"] == 2
    assert round(row_5d["win_rate"], 6) == 0.5


def test_extract_candidate_rule_prefers_positive_edge_and_low_failure_risk():
    module = load_module()
    summary = pd.DataFrame(
        [
            {
                "ret_5d_bucket": "ret5_moderate",
                "ma20_gap_bucket": "ma20_gap_moderate",
                "resonance_regime": "usd_down_oil_up",
                "holding_days": 10,
                "sample_count": 20,
                "win_rate": 0.75,
                "avg_return": 0.03,
                "avg_max_drawdown": -0.015,
                "return_per_day": 0.003,
                "failure_risk_share": 0.10,
            },
            {
                "ret_5d_bucket": "ret5_extreme",
                "ma20_gap_bucket": "ma20_gap_deep",
                "resonance_regime": "usd_down_oil_up",
                "holding_days": 10,
                "sample_count": 20,
                "win_rate": 0.70,
                "avg_return": 0.025,
                "avg_max_drawdown": -0.03,
                "return_per_day": 0.0025,
                "failure_risk_share": 0.35,
            },
        ]
    )

    candidate = module.extract_candidate_rule(summary)

    assert candidate["ret_5d_bucket"] == "ret5_moderate"
    assert candidate["ma20_gap_bucket"] == "ma20_gap_moderate"
    assert candidate["holding_days"] == 10
