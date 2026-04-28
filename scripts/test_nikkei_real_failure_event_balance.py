#!/usr/bin/env python
"""
Contract tests for the Nikkei real-failure-event balance experiment.
"""

from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

import pandas as pd


REPO_ROOT = Path(r"D:\SM")
BUILD_SAMPLES_PATH = REPO_ROOT / r"scripts\build_nikkei_replay_samples.py"
RUN_EXPERIMENT_PATH = REPO_ROOT / r"scripts\run_nikkei_real_failure_event_balance.py"

EXPECTED_FAILURE_SOURCE = "real_failure_event_mining"


# 2026-04-29 CST: Added explicit file-path loading because the failure-event
# experiment stays inside the Python research lane. Purpose: fail on missing
# governed entrypoints instead of package/import state.
def load_module(module_name: str, module_path: Path):
    if not module_path.exists():
        raise FileNotFoundError(f"missing future module: {module_path}")
    spec = importlib.util.spec_from_file_location(module_name, module_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module {module_name} from {module_path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


class RealFailureEventBuilderContractTests(unittest.TestCase):
    # 2026-04-29 CST: Added because this lane must mine only real failure
    # semantics, not reintroduce broad positive augmentation. Purpose: freeze a
    # separately tagged, traceable, negative-only failure pool contract.
    def test_builder_emits_negative_only_real_failure_rows(self) -> None:
        builder = load_module("nikkei_real_failure_builder_under_test", BUILD_SAMPLES_PATH)
        self.assertTrue(
            hasattr(builder, "build_real_failure_event_samples"),
            "builder must expose build_real_failure_event_samples",
        )
        mined = builder.build_real_failure_event_samples(
            pd.DataFrame(
                [
                    {
                        "sample_id": "hist-a",
                        "sample_source": "historical_research",
                        "signal_date": "2026-03-01",
                        "signal_family": "daily_position",
                        "signal_direction": "add",
                        "candidate_event_type": None,
                        "candidate_action_label": None,
                        "candidate_stood_1d": False,
                        "candidate_stood_3d": False,
                        "candidate_stood_5d": False,
                        "downside_suggested_action": None,
                        "base_position_v3": 0.35,
                        "dist_res20": -0.01,
                        "dist_sup20": 0.03,
                        "dist_sup60": 0.08,
                        "weighted_vol_down": 0.04,
                        "component_above200_breadth": 0.80,
                        "avg_component_vr": 0.98,
                        "horizon_1d_close_return": -0.01,
                        "horizon_3d_close_return": -0.02,
                        "horizon_5d_close_return": -0.03,
                        "horizon_1d_max_drawdown": -0.02,
                        "horizon_3d_max_drawdown": -0.04,
                        "horizon_5d_max_drawdown": -0.05,
                        "next_signal_primary_adjustment": -1,
                    },
                    {
                        "sample_id": "hist-b",
                        "sample_source": "historical_research",
                        "signal_date": "2026-03-02",
                        "signal_family": "daily_position",
                        "signal_direction": "reduce",
                        "candidate_event_type": "near_support_20d",
                        "candidate_action_label": "support_hold_watch",
                        "candidate_stood_1d": True,
                        "candidate_stood_3d": True,
                        "candidate_stood_5d": True,
                        "downside_suggested_action": "tighten_risk",
                        "base_position_v3": 0.35,
                        "dist_res20": 0.01,
                        "dist_sup20": 0.01,
                        "dist_sup60": 0.02,
                        "weighted_vol_down": 0.06,
                        "component_above200_breadth": 0.78,
                        "avg_component_vr": 1.06,
                        "horizon_1d_close_return": 0.02,
                        "horizon_3d_close_return": 0.03,
                        "horizon_5d_close_return": 0.03,
                        "horizon_1d_max_drawdown": -0.01,
                        "horizon_3d_max_drawdown": -0.01,
                        "horizon_5d_max_drawdown": -0.01,
                        "next_signal_primary_adjustment": 1,
                    },
                    {
                        "sample_id": "hist-c",
                        "sample_source": "historical_research",
                        "signal_date": "2026-03-03",
                        "signal_family": "breakout_followthrough",
                        "signal_direction": "add",
                        "candidate_event_type": "breakout_60d",
                        "candidate_action_label": "confirmed_breakout_add_or_hold",
                        "candidate_stood_1d": True,
                        "candidate_stood_3d": True,
                        "candidate_stood_5d": True,
                        "downside_suggested_action": None,
                        "base_position_v3": 0.35,
                        "dist_res20": -0.03,
                        "dist_sup20": 0.05,
                        "dist_sup60": 0.10,
                        "weighted_vol_down": 0.01,
                        "component_above200_breadth": 0.92,
                        "avg_component_vr": 1.08,
                        "horizon_1d_close_return": 0.02,
                        "horizon_3d_close_return": 0.03,
                        "horizon_5d_close_return": 0.04,
                        "horizon_1d_max_drawdown": -0.01,
                        "horizon_3d_max_drawdown": -0.02,
                        "horizon_5d_max_drawdown": -0.02,
                        "next_signal_primary_adjustment": 1,
                    },
                    {
                        "sample_id": "hist-d",
                        "sample_source": "historical_research",
                        "signal_date": "2026-03-04",
                        "signal_family": "breakout_followthrough",
                        "signal_direction": "add",
                        "candidate_event_type": "near_resistance_20d",
                        "candidate_action_label": "resistance_reject_watch",
                        "candidate_stood_1d": False,
                        "candidate_stood_3d": False,
                        "candidate_stood_5d": False,
                        "downside_suggested_action": None,
                        "base_position_v3": 0.62,
                        "dist_res20": 0.01,
                        "dist_sup20": 0.08,
                        "dist_sup60": 0.12,
                        "weighted_vol_down": 0.02,
                        "component_above200_breadth": 0.91,
                        "avg_component_vr": 1.16,
                        "horizon_1d_close_return": -0.01,
                        "horizon_3d_close_return": -0.02,
                        "horizon_5d_close_return": -0.03,
                        "horizon_1d_max_drawdown": -0.02,
                        "horizon_3d_max_drawdown": -0.03,
                        "horizon_5d_max_drawdown": -0.04,
                        "next_signal_primary_adjustment": -1,
                    },
                ]
            ),
            label_horizon="1d",
        )
        self.assertIsInstance(mined, pd.DataFrame)
        self.assertGreaterEqual(len(mined), 2)
        self.assertTrue((mined["sample_source"] == EXPECTED_FAILURE_SOURCE).all())
        self.assertTrue(mined["is_real_failure_event"].all())
        self.assertTrue((mined["failure_label_horizon"] == "1d").all())
        self.assertTrue((mined["mined_action_direction"] == "add").all())
        self.assertTrue((mined["mined_failure_reason"] == "prototype_add_failure").all())
        self.assertTrue(mined["source_sample_id"].isin(["hist-a", "hist-d"]).all())
        self.assertIn("mined_action_direction", mined.columns)
        self.assertIn("mined_failure_reason", mined.columns)
        self.assertIn("replay_label_1d", mined.columns)
        self.assertIn("continuation_label_1d", mined.columns)
        labels = set(mined["continuation_label_1d"].dropna().astype(int).tolist())
        self.assertEqual(labels, {0}, "mined pool must retain negative continuation rows only")

    # 2026-04-29 CST: Added because scheme A changed the builder contract from
    # generic pooled mining to horizon-specific pool emission. Purpose: ensure
    # a row that is negative on another horizon but positive on the requested
    # horizon cannot leak into the exported mined pool.
    def test_builder_filters_by_requested_horizon_only(self) -> None:
        builder = load_module("nikkei_real_failure_builder_horizon_under_test", BUILD_SAMPLES_PATH)
        mined = builder.build_real_failure_event_samples(
            pd.DataFrame(
                [
                    {
                        "sample_id": "hist-only-1d-negative",
                        "sample_source": "historical_research",
                        "signal_date": "2026-03-04",
                        "signal_family": "daily_position",
                        "signal_direction": "add",
                        "candidate_event_type": None,
                        "candidate_action_label": None,
                        "candidate_stood_1d": False,
                        "candidate_stood_3d": True,
                        "candidate_stood_5d": True,
                        "downside_suggested_action": None,
                        "base_position_v3": 0.35,
                        "dist_res20": -0.02,
                        "dist_sup20": 0.02,
                        "dist_sup60": 0.05,
                        "weighted_vol_down": 0.03,
                        "component_above200_breadth": 0.82,
                        "avg_component_vr": 1.01,
                        "horizon_1d_close_return": -0.02,
                        "horizon_3d_close_return": 0.01,
                        "horizon_5d_close_return": 0.03,
                        "horizon_1d_max_drawdown": -0.03,
                        "horizon_3d_max_drawdown": -0.01,
                        "horizon_5d_max_drawdown": -0.01,
                        "next_signal_primary_adjustment": 1,
                    },
                    {
                        "sample_id": "hist-5d-negative",
                        "sample_source": "historical_research",
                        "signal_date": "2026-03-05",
                        "signal_family": "daily_position",
                        "signal_direction": "add",
                        "candidate_event_type": None,
                        "candidate_action_label": None,
                        "candidate_stood_1d": False,
                        "candidate_stood_3d": False,
                        "candidate_stood_5d": False,
                        "downside_suggested_action": None,
                        "base_position_v3": 0.35,
                        "dist_res20": -0.01,
                        "dist_sup20": 0.03,
                        "dist_sup60": 0.08,
                        "weighted_vol_down": 0.04,
                        "component_above200_breadth": 0.80,
                        "avg_component_vr": 0.98,
                        "horizon_1d_close_return": -0.01,
                        "horizon_3d_close_return": -0.02,
                        "horizon_5d_close_return": -0.03,
                        "horizon_1d_max_drawdown": -0.02,
                        "horizon_3d_max_drawdown": -0.04,
                        "horizon_5d_max_drawdown": -0.05,
                        "next_signal_primary_adjustment": -1,
                    },
                ]
            ),
            label_horizon="1d",
        )
        source_ids = set(mined["source_sample_id"].astype(str))
        self.assertEqual(source_ids, {"hist-only-1d-negative", "hist-5d-negative"})
        self.assertTrue((mined["continuation_label_1d"].astype(int) == 0).all())

    # 2026-04-29 CST: Added because the third mining round must align with the
    # dominant untouched-validation shape rather than broad mixed-direction
    # event semantics. Purpose: freeze add-only prototype selection and reject
    # reduce-style or low-quality non-prototype rows.
    def test_builder_keeps_only_add_prototype_rows_in_round_three(self) -> None:
        builder = load_module("nikkei_real_failure_builder_prototype_under_test", BUILD_SAMPLES_PATH)
        mined = builder.build_real_failure_event_samples(
            pd.DataFrame(
                [
                    {
                        "sample_id": "keep-prototype-add",
                        "sample_source": "historical_research",
                        "signal_date": "2026-03-06",
                        "signal_family": "daily_position",
                        "signal_direction": "add",
                        "candidate_event_type": None,
                        "candidate_action_label": None,
                        "candidate_stood_1d": False,
                        "candidate_stood_3d": False,
                        "candidate_stood_5d": False,
                        "downside_suggested_action": None,
                        "base_position_v3": 0.42,
                        "dist_res20": 0.00,
                        "dist_sup20": 0.05,
                        "dist_sup60": 0.11,
                        "weighted_vol_down": 0.05,
                        "component_above200_breadth": 0.78,
                        "avg_component_vr": 0.95,
                        "horizon_1d_close_return": -0.01,
                        "horizon_3d_close_return": -0.02,
                        "horizon_5d_close_return": -0.03,
                        "horizon_1d_max_drawdown": -0.02,
                        "horizon_3d_max_drawdown": -0.03,
                        "horizon_5d_max_drawdown": -0.04,
                        "next_signal_primary_adjustment": -1,
                    },
                    {
                        "sample_id": "reject-reduce",
                        "sample_source": "historical_research",
                        "signal_date": "2026-03-07",
                        "signal_family": "daily_position",
                        "signal_direction": "reduce",
                        "candidate_event_type": "near_support_20d",
                        "candidate_action_label": "support_hold_watch",
                        "candidate_stood_1d": True,
                        "candidate_stood_3d": True,
                        "candidate_stood_5d": True,
                        "downside_suggested_action": "tighten_risk",
                        "base_position_v3": 0.40,
                        "dist_res20": -0.01,
                        "dist_sup20": 0.04,
                        "dist_sup60": 0.10,
                        "weighted_vol_down": 0.03,
                        "component_above200_breadth": 0.79,
                        "avg_component_vr": 1.02,
                        "horizon_1d_close_return": 0.01,
                        "horizon_3d_close_return": 0.03,
                        "horizon_5d_close_return": 0.04,
                        "horizon_1d_max_drawdown": -0.01,
                        "horizon_3d_max_drawdown": -0.01,
                        "horizon_5d_max_drawdown": -0.01,
                        "next_signal_primary_adjustment": 1,
                    },
                    {
                        "sample_id": "reject-low-position",
                        "sample_source": "historical_research",
                        "signal_date": "2026-03-08",
                        "signal_family": "daily_position",
                        "signal_direction": "add",
                        "candidate_event_type": None,
                        "candidate_action_label": None,
                        "candidate_stood_1d": False,
                        "candidate_stood_3d": False,
                        "candidate_stood_5d": False,
                        "downside_suggested_action": None,
                        "base_position_v3": 0.05,
                        "dist_res20": -0.01,
                        "dist_sup20": 0.05,
                        "dist_sup60": 0.10,
                        "weighted_vol_down": 0.03,
                        "component_above200_breadth": 0.80,
                        "avg_component_vr": 0.96,
                        "horizon_1d_close_return": -0.01,
                        "horizon_3d_close_return": -0.02,
                        "horizon_5d_close_return": -0.03,
                        "horizon_1d_max_drawdown": -0.02,
                        "horizon_3d_max_drawdown": -0.03,
                        "horizon_5d_max_drawdown": -0.04,
                        "next_signal_primary_adjustment": -1,
                    },
                ]
            ),
            label_horizon="1d",
        )
        source_ids = set(mined["source_sample_id"].astype(str))
        self.assertEqual(source_ids, {"keep-prototype-add"})
        self.assertTrue((mined["mined_action_direction"] == "add").all())
        self.assertTrue((mined["mined_failure_reason"] == "prototype_add_failure").all())

    # 2026-04-29 CST: Added because the fourth round must stop using one shared
    # add prototype for all horizons. Purpose: freeze a dedicated 5D slow-fail
    # rule with traceable sub-reasons while keeping 1D/3D behavior unchanged.
    def test_builder_uses_dedicated_5d_slow_fail_reasons(self) -> None:
        builder = load_module("nikkei_real_failure_builder_5d_under_test", BUILD_SAMPLES_PATH)
        frame = pd.DataFrame(
            [
                {
                    "sample_id": "keep-1d-only-prototype",
                    "sample_source": "historical_research",
                    "signal_date": "2026-03-09",
                    "signal_family": "daily_position",
                    "signal_direction": "add",
                    "candidate_event_type": None,
                    "candidate_action_label": None,
                    "candidate_stood_1d": False,
                    "candidate_stood_3d": False,
                    "candidate_stood_5d": False,
                    "downside_suggested_action": None,
                    "base_position_v3": 0.22,
                    "dist_res20": -0.01,
                    "dist_sup20": 0.04,
                    "dist_sup60": 0.10,
                    "weighted_vol_down": 0.04,
                    "component_above200_breadth": 0.70,
                    "avg_component_vr": 0.96,
                    "horizon_1d_close_return": -0.01,
                    "horizon_3d_close_return": -0.02,
                    "horizon_5d_close_return": -0.03,
                    "horizon_1d_max_drawdown": -0.02,
                    "horizon_3d_max_drawdown": -0.03,
                    "horizon_5d_max_drawdown": -0.04,
                    "next_signal_primary_adjustment": -1,
                },
                {
                    "sample_id": "keep-5d-resistance",
                    "sample_source": "historical_research",
                    "signal_date": "2026-03-10",
                    "signal_family": "breakout_followthrough",
                    "signal_direction": "add",
                    "candidate_event_type": "breakout_20d",
                    "candidate_action_label": "false_breakout_avoid_chase",
                    "candidate_stood_1d": False,
                    "candidate_stood_3d": False,
                    "candidate_stood_5d": False,
                    "downside_suggested_action": None,
                    "base_position_v3": 0.68,
                    "dist_res20": 0.01,
                    "dist_sup20": 0.09,
                    "dist_sup60": 0.15,
                    "weighted_vol_down": 0.05,
                    "component_above200_breadth": 0.94,
                    "avg_component_vr": 1.18,
                    "horizon_1d_close_return": 0.01,
                    "horizon_3d_close_return": -0.01,
                    "horizon_5d_close_return": -0.03,
                    "horizon_1d_max_drawdown": -0.01,
                    "horizon_3d_max_drawdown": -0.02,
                    "horizon_5d_max_drawdown": -0.03,
                    "next_signal_primary_adjustment": -1,
                },
                {
                    "sample_id": "keep-5d-drift",
                    "sample_source": "historical_research",
                    "signal_date": "2026-03-11",
                    "signal_family": "daily_position",
                    "signal_direction": "add",
                    "candidate_event_type": None,
                    "candidate_action_label": None,
                    "candidate_stood_1d": False,
                    "candidate_stood_3d": False,
                    "candidate_stood_5d": False,
                    "downside_suggested_action": None,
                    "base_position_v3": 0.36,
                    "dist_res20": -0.07,
                    "dist_sup20": 0.05,
                    "dist_sup60": 0.11,
                    "weighted_vol_down": 0.03,
                    "component_above200_breadth": 0.80,
                    "avg_component_vr": 0.86,
                    "horizon_1d_close_return": 0.00,
                    "horizon_3d_close_return": -0.01,
                    "horizon_5d_close_return": -0.03,
                    "horizon_1d_max_drawdown": -0.01,
                    "horizon_3d_max_drawdown": -0.02,
                    "horizon_5d_max_drawdown": -0.04,
                    "next_signal_primary_adjustment": -1,
                },
            ]
        )

        mined_1d = builder.build_real_failure_event_samples(frame, label_horizon="1d")
        mined_5d = builder.build_real_failure_event_samples(frame, label_horizon="5d")

        self.assertIn("keep-1d-only-prototype", set(mined_1d["source_sample_id"].astype(str)))
        source_ids_5d = set(mined_5d["source_sample_id"].astype(str))
        self.assertEqual(source_ids_5d, {"keep-5d-resistance", "keep-5d-drift"})
        reasons_5d = set(mined_5d["mined_failure_reason"].astype(str))
        self.assertEqual(
            reasons_5d,
            {
                "prototype_add_failure_5d_resistance_exhaustion",
                "prototype_add_failure_5d_extended_drift",
            },
        )


class RealFailureEventExperimentContractTests(unittest.TestCase):
    # 2026-04-29 CST: Added because the value of this slice comes from using
    # real failure rows without corrupting real validation truth. Purpose: lock
    # train-only augmentation and machine-readable comparison outputs.
    def test_experiment_keeps_validation_real_only_and_emits_failure_comparison(self) -> None:
        experiment = load_module("nikkei_real_failure_experiment_under_test", RUN_EXPERIMENT_PATH)
        self.assertTrue(
            hasattr(experiment, "run_nikkei_real_failure_event_balance"),
            "experiment runner must expose run_nikkei_real_failure_event_balance",
        )
        with tempfile.TemporaryDirectory() as tmp_dir:
            temp_root = Path(tmp_dir)
            sample_path = temp_root / "replay_samples.csv"
            pd.DataFrame(
                [
                    {
                        "sample_id": "r1",
                        "sample_source": "historical_research",
                        "signal_date": "2026-01-01",
                        "signal_family": "breakout_followthrough",
                        "candidate_event_type": "breakout_20d",
                        "candidate_action_label": "false_breakout_avoid_chase",
                        "candidate_stood_1d": False,
                        "candidate_stood_3d": False,
                        "candidate_stood_5d": False,
                        "downside_suggested_action": None,
                        "signal_direction": "add",
                        "action_type": "add_risk",
                        "rating_state": "risk_add",
                        "base_position_v3": 0.35,
                        "dist_res20": -0.01,
                        "dist_sup20": 0.03,
                        "dist_sup60": 0.08,
                        "weighted_vol_down": 0.03,
                        "component_above200_breadth": 0.84,
                        "avg_component_vr": 1.02,
                        "horizon_1d_close_return": -0.02,
                        "horizon_3d_close_return": -0.03,
                        "horizon_5d_close_return": -0.04,
                        "horizon_1d_max_drawdown": -0.03,
                        "horizon_3d_max_drawdown": -0.05,
                        "horizon_5d_max_drawdown": -0.06,
                        "next_signal_primary_adjustment": -1,
                        "replay_label_5d": "premature_add",
                        "continuation_label_5d": 0,
                        "is_continuation_eligible": True,
                        "is_outcome_observed_5d": True,
                    },
                    {
                        "sample_id": "r2",
                        "sample_source": "historical_research",
                        "signal_date": "2026-01-02",
                        "signal_family": "support_test",
                        "candidate_event_type": "near_support_20d",
                        "candidate_action_label": "support_hold_watch",
                        "candidate_stood_1d": True,
                        "candidate_stood_3d": True,
                        "candidate_stood_5d": True,
                        "downside_suggested_action": "tighten_risk",
                        "signal_direction": "reduce",
                        "action_type": "reduce_risk",
                        "rating_state": "risk_reduce",
                        "base_position_v3": 0.35,
                        "dist_res20": 0.01,
                        "dist_sup20": 0.01,
                        "dist_sup60": 0.02,
                        "weighted_vol_down": 0.06,
                        "component_above200_breadth": 0.78,
                        "avg_component_vr": 1.06,
                        "horizon_1d_close_return": 0.02,
                        "horizon_3d_close_return": 0.03,
                        "horizon_5d_close_return": 0.03,
                        "horizon_1d_max_drawdown": -0.01,
                        "horizon_3d_max_drawdown": -0.01,
                        "horizon_5d_max_drawdown": -0.01,
                        "next_signal_primary_adjustment": 1,
                        "replay_label_5d": "premature_reduce",
                        "continuation_label_5d": 0,
                        "is_continuation_eligible": True,
                        "is_outcome_observed_5d": True,
                    },
                    {
                        "sample_id": "r3",
                        "sample_source": "historical_research",
                        "signal_date": "2026-01-03",
                        "signal_family": "breakdown_followthrough",
                        "candidate_event_type": "breakdown_20d",
                        "candidate_action_label": "confirmed_breakdown_reduce",
                        "candidate_stood_1d": None,
                        "candidate_stood_3d": None,
                        "candidate_stood_5d": None,
                        "downside_suggested_action": "reduce_or_avoid",
                        "signal_direction": "reduce",
                        "action_type": "reduce_risk",
                        "rating_state": "risk_reduce",
                        "base_position_v3": 0.35,
                        "dist_res20": 0.04,
                        "dist_sup20": 0.10,
                        "dist_sup60": 0.12,
                        "weighted_vol_down": 0.10,
                        "component_above200_breadth": 0.68,
                        "avg_component_vr": 1.20,
                        "horizon_1d_close_return": -0.02,
                        "horizon_3d_close_return": -0.03,
                        "horizon_5d_close_return": -0.04,
                        "horizon_1d_max_drawdown": -0.03,
                        "horizon_3d_max_drawdown": -0.05,
                        "horizon_5d_max_drawdown": -0.06,
                        "next_signal_primary_adjustment": -1,
                        "replay_label_5d": "correct_reduce",
                        "continuation_label_5d": 1,
                        "is_continuation_eligible": True,
                        "is_outcome_observed_5d": True,
                    },
                    {
                        "sample_id": "r4",
                        "sample_source": "historical_research",
                        "signal_date": "2026-01-04",
                        "signal_family": "breakout_followthrough",
                        "candidate_event_type": "breakout_60d",
                        "candidate_action_label": "confirmed_breakout_add_or_hold",
                        "candidate_stood_1d": True,
                        "candidate_stood_3d": True,
                        "candidate_stood_5d": True,
                        "downside_suggested_action": None,
                        "signal_direction": "add",
                        "action_type": "add_risk",
                        "rating_state": "risk_add",
                        "base_position_v3": 0.35,
                        "dist_res20": -0.03,
                        "dist_sup20": 0.05,
                        "dist_sup60": 0.10,
                        "weighted_vol_down": 0.01,
                        "component_above200_breadth": 0.92,
                        "avg_component_vr": 1.08,
                        "horizon_1d_close_return": 0.02,
                        "horizon_3d_close_return": 0.03,
                        "horizon_5d_close_return": 0.04,
                        "horizon_1d_max_drawdown": -0.01,
                        "horizon_3d_max_drawdown": -0.02,
                        "horizon_5d_max_drawdown": -0.02,
                        "next_signal_primary_adjustment": 1,
                        "replay_label_5d": "correct_add",
                        "continuation_label_5d": 1,
                        "is_continuation_eligible": True,
                        "is_outcome_observed_5d": True,
                    },
                    {
                        "sample_id": "r5",
                        "sample_source": "historical_research",
                        "signal_date": "2026-01-05",
                        "signal_family": "breakout_followthrough",
                        "candidate_event_type": "breakout_20d",
                        "candidate_action_label": "false_breakout_avoid_chase",
                        "candidate_stood_1d": False,
                        "candidate_stood_3d": False,
                        "candidate_stood_5d": False,
                        "downside_suggested_action": None,
                        "signal_direction": "add",
                        "action_type": "add_risk",
                        "rating_state": "risk_add",
                        "base_position_v3": 0.35,
                        "dist_res20": -0.01,
                        "dist_sup20": 0.03,
                        "dist_sup60": 0.07,
                        "weighted_vol_down": 0.04,
                        "component_above200_breadth": 0.80,
                        "avg_component_vr": 0.98,
                        "horizon_1d_close_return": -0.01,
                        "horizon_3d_close_return": -0.02,
                        "horizon_5d_close_return": -0.03,
                        "horizon_1d_max_drawdown": -0.02,
                        "horizon_3d_max_drawdown": -0.04,
                        "horizon_5d_max_drawdown": -0.05,
                        "next_signal_primary_adjustment": -1,
                        "replay_label_5d": "premature_add",
                        "continuation_label_5d": 0,
                        "is_continuation_eligible": True,
                        "is_outcome_observed_5d": True,
                    },
                    {
                        "sample_id": "r6",
                        "sample_source": "historical_research",
                        "signal_date": "2026-01-06",
                        "signal_family": "support_test",
                        "candidate_event_type": "near_support_20d",
                        "candidate_action_label": "support_hold_watch",
                        "candidate_stood_1d": True,
                        "candidate_stood_3d": True,
                        "candidate_stood_5d": True,
                        "downside_suggested_action": "watch",
                        "signal_direction": "reduce",
                        "action_type": "reduce_risk",
                        "rating_state": "risk_reduce",
                        "base_position_v3": 0.35,
                        "dist_res20": 0.01,
                        "dist_sup20": 0.01,
                        "dist_sup60": 0.02,
                        "weighted_vol_down": 0.06,
                        "component_above200_breadth": 0.78,
                        "avg_component_vr": 1.06,
                        "horizon_1d_close_return": 0.02,
                        "horizon_3d_close_return": 0.03,
                        "horizon_5d_close_return": 0.03,
                        "horizon_1d_max_drawdown": -0.01,
                        "horizon_3d_max_drawdown": -0.01,
                        "horizon_5d_max_drawdown": -0.01,
                        "next_signal_primary_adjustment": 1,
                        "replay_label_5d": "premature_reduce",
                        "continuation_label_5d": 0,
                        "is_continuation_eligible": True,
                        "is_outcome_observed_5d": True,
                    },
                ]
            ).to_csv(sample_path, index=False, encoding="utf-8-sig")

            result = experiment.run_nikkei_real_failure_event_balance(
                sample_path=sample_path,
                output_root=temp_root,
                label_horizon="5d",
            )
            comparison = pd.read_csv(result["comparison_summary_path"], encoding="utf-8-sig")
            augmented_predictions = pd.read_csv(result["augmented_predictions_path"], encoding="utf-8-sig")
            mined_pool = pd.read_csv(result["real_failure_sample_path"], encoding="utf-8-sig")

        self.assertIsInstance(result, dict)
        self.assertIn("comparison_summary_path", result)
        self.assertIn("distribution_summary_path", result)
        self.assertIn("augmented_predictions_path", result)
        self.assertGreater(len(comparison), 0)
        self.assertIn("baseline_balanced_accuracy", comparison.columns)
        self.assertIn("augmented_balanced_accuracy", comparison.columns)
        self.assertTrue((mined_pool["failure_label_horizon"] == "5d").all())
        self.assertTrue((mined_pool["continuation_label_5d"].astype(int) == 0).all())
        validation_rows = augmented_predictions[augmented_predictions["data_split"] == "validation"].copy()
        self.assertGreater(len(validation_rows), 0)
        self.assertNotIn(EXPECTED_FAILURE_SOURCE, set(validation_rows["sample_source"].astype(str)))


if __name__ == "__main__":
    unittest.main()
