#!/usr/bin/env python
"""
Contract tests for the Nikkei simulated-action balance experiment.
"""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

import pandas as pd


REPO_ROOT = Path(r"D:\SM")
BUILD_SAMPLES_PATH = REPO_ROOT / r"scripts\build_nikkei_replay_samples.py"
RUN_EXPERIMENT_PATH = REPO_ROOT / r"scripts\run_nikkei_simulated_action_balance.py"

EXPECTED_SIMULATED_SOURCE = "simulated_action_replay"


# 2026-04-29 CST: Added explicit file-path loading because this experiment is
# intentionally isolated inside the Python research lane. Purpose: fail on
# missing governed entrypoints instead of depending on package-install state.
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


class SimulatedActionBuilderContractTests(unittest.TestCase):
    # 2026-04-29 CST: Added because simulated rows must remain traceable and
    # separately tagged instead of polluting the official replay source of truth.
    # Purpose: lock source tagging, trace-back, and label derivation on the new
    # augmentation lane before any experiment uses it.
    def test_builder_emits_separately_tagged_simulated_action_rows(self) -> None:
        builder = load_module("nikkei_simulated_builder_under_test", BUILD_SAMPLES_PATH)
        self.assertTrue(
            hasattr(builder, "build_simulated_action_samples"),
            "builder must expose build_simulated_action_samples",
        )
        simulated = builder.build_simulated_action_samples(
            pd.DataFrame(
                [
                    {
                        "sample_id": "hist-1",
                        "sample_source": "historical_research",
                        "signal_date": "2026-03-01",
                        "signal_family": "breakout_followthrough",
                        "candidate_event_type": "breakout_20d",
                        "candidate_action_label": "confirmed_breakout_add_or_hold",
                        "downside_suggested_action": None,
                        "base_position_v3": 0.35,
                        "dist_res20": -0.01,
                        "dist_sup20": 0.04,
                        "dist_sup60": 0.09,
                        "weighted_vol_down": 0.02,
                        "component_above200_breadth": 0.88,
                        "avg_component_vr": 1.05,
                        "horizon_1d_close_return": 0.02,
                        "horizon_3d_close_return": 0.03,
                        "horizon_5d_close_return": 0.04,
                        "horizon_1d_max_drawdown": -0.01,
                        "horizon_3d_max_drawdown": -0.02,
                        "horizon_5d_max_drawdown": -0.02,
                        "next_signal_primary_adjustment": 1,
                    },
                    {
                        "sample_id": "hist-2",
                        "sample_source": "historical_research",
                        "signal_date": "2026-03-02",
                        "signal_family": "support_test",
                        "candidate_event_type": "near_support_20d",
                        "candidate_action_label": "support_fail_watch",
                        "downside_suggested_action": "reduce_or_avoid",
                        "base_position_v3": 0.35,
                        "dist_res20": 0.03,
                        "dist_sup20": 0.01,
                        "dist_sup60": 0.02,
                        "weighted_vol_down": 0.09,
                        "component_above200_breadth": 0.72,
                        "avg_component_vr": 1.18,
                        "horizon_1d_close_return": 0.03,
                        "horizon_3d_close_return": 0.04,
                        "horizon_5d_close_return": 0.05,
                        "horizon_1d_max_drawdown": -0.005,
                        "horizon_3d_max_drawdown": -0.008,
                        "horizon_5d_max_drawdown": -0.01,
                        "next_signal_primary_adjustment": 1,
                    },
                ]
            )
        )
        self.assertIsInstance(simulated, pd.DataFrame)
        self.assertGreaterEqual(len(simulated), 2)
        self.assertTrue((simulated["sample_source"] == EXPECTED_SIMULATED_SOURCE).all())
        self.assertTrue(simulated["is_simulated_action"].all())
        self.assertTrue(simulated["source_sample_id"].isin(["hist-1", "hist-2"]).all())
        self.assertIn("simulated_action_direction", simulated.columns)
        self.assertIn("simulated_action_reason", simulated.columns)
        self.assertIn("replay_label_1d", simulated.columns)
        self.assertIn("continuation_label_5d", simulated.columns)


class SimulatedActionExperimentContractTests(unittest.TestCase):
    # 2026-04-29 CST: Added because the whole point of Scheme A is safe
    # augmentation. Purpose: prove simulated rows can augment training without
    # leaking into the baseline validation truth slice.
    def test_experiment_keeps_validation_real_only_and_emits_comparison_summary(self) -> None:
        experiment = load_module("nikkei_balance_experiment_under_test", RUN_EXPERIMENT_PATH)
        self.assertTrue(
            hasattr(experiment, "run_simulated_action_balance_experiment"),
            "experiment runner must expose run_simulated_action_balance_experiment",
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
                        "candidate_action_label": "confirmed_breakout_add_or_hold",
                        "downside_suggested_action": None,
                        "signal_direction": "add",
                        "action_type": "add_risk",
                        "rating_state": "risk_add",
                        "base_position_v3": 0.35,
                        "dist_res20": -0.02,
                        "dist_sup20": 0.04,
                        "dist_sup60": 0.08,
                        "weighted_vol_down": 0.01,
                        "component_above200_breadth": 0.90,
                        "avg_component_vr": 1.05,
                        "horizon_1d_close_return": 0.01,
                        "horizon_3d_close_return": 0.02,
                        "horizon_5d_close_return": 0.03,
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
                        "sample_id": "r2",
                        "sample_source": "historical_research",
                        "signal_date": "2026-01-02",
                        "signal_family": "breakout_followthrough",
                        "candidate_event_type": "breakout_20d",
                        "candidate_action_label": "false_breakout_avoid_chase",
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
                        "sample_id": "r3",
                        "sample_source": "historical_research",
                        "signal_date": "2026-01-03",
                        "signal_family": "support_test",
                        "candidate_event_type": "near_support_20d",
                        "candidate_action_label": "support_fail_watch",
                        "downside_suggested_action": "tighten_risk",
                        "signal_direction": "reduce",
                        "action_type": "reduce_risk",
                        "rating_state": "risk_reduce",
                        "base_position_v3": 0.35,
                        "dist_res20": 0.02,
                        "dist_sup20": 0.01,
                        "dist_sup60": 0.02,
                        "weighted_vol_down": 0.08,
                        "component_above200_breadth": 0.74,
                        "avg_component_vr": 1.14,
                        "horizon_1d_close_return": -0.01,
                        "horizon_3d_close_return": -0.02,
                        "horizon_5d_close_return": -0.03,
                        "horizon_1d_max_drawdown": -0.02,
                        "horizon_3d_max_drawdown": -0.03,
                        "horizon_5d_max_drawdown": -0.04,
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
                        "signal_family": "support_test",
                        "candidate_event_type": "near_support_20d",
                        "candidate_action_label": "support_hold_watch",
                        "downside_suggested_action": "reduce_or_avoid",
                        "signal_direction": "reduce",
                        "action_type": "reduce_risk",
                        "rating_state": "risk_reduce",
                        "base_position_v3": 0.35,
                        "dist_res20": 0.01,
                        "dist_sup20": 0.01,
                        "dist_sup60": 0.02,
                        "weighted_vol_down": 0.07,
                        "component_above200_breadth": 0.76,
                        "avg_component_vr": 1.10,
                        "horizon_1d_close_return": 0.03,
                        "horizon_3d_close_return": 0.04,
                        "horizon_5d_close_return": 0.05,
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
                        "sample_id": "r5",
                        "sample_source": "historical_research",
                        "signal_date": "2026-01-05",
                        "signal_family": "breakdown_followthrough",
                        "candidate_event_type": "breakdown_20d",
                        "candidate_action_label": "confirmed_breakdown_reduce",
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
                        "sample_id": "r6",
                        "sample_source": "historical_research",
                        "signal_date": "2026-01-06",
                        "signal_family": "breakout_followthrough",
                        "candidate_event_type": "breakout_60d",
                        "candidate_action_label": "confirmed_breakout_add_or_hold",
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
                        "sample_id": "r7",
                        "sample_source": "historical_research",
                        "signal_date": "2026-01-07",
                        "signal_family": "breakout_followthrough",
                        "candidate_event_type": "breakout_20d",
                        "candidate_action_label": "false_breakout_avoid_chase",
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
                        "sample_id": "r8",
                        "sample_source": "historical_research",
                        "signal_date": "2026-01-08",
                        "signal_family": "support_test",
                        "candidate_event_type": "near_support_20d",
                        "candidate_action_label": "support_hold_watch",
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

            result = experiment.run_simulated_action_balance_experiment(
                sample_path=sample_path,
                output_root=temp_root,
                label_horizon="5d",
            )
            comparison = pd.read_csv(result["comparison_summary_path"], encoding="utf-8-sig")
            augmented_predictions = pd.read_csv(result["augmented_predictions_path"], encoding="utf-8-sig")

        self.assertIsInstance(result, dict)
        self.assertIn("comparison_summary_path", result)
        self.assertIn("distribution_summary_path", result)
        self.assertIn("augmented_predictions_path", result)
        self.assertGreater(len(comparison), 0)
        self.assertIn("baseline_balanced_accuracy", comparison.columns)
        self.assertIn("augmented_balanced_accuracy", comparison.columns)
        validation_rows = augmented_predictions[augmented_predictions["data_split"] == "validation"].copy()
        self.assertGreater(len(validation_rows), 0)
        self.assertNotIn(EXPECTED_SIMULATED_SOURCE, set(validation_rows["sample_source"].astype(str)))


if __name__ == "__main__":
    unittest.main()
