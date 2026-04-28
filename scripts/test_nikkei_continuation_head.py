#!/usr/bin/env python
"""
Contract tests for the Nikkei continuation-head pipeline.
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
TRAIN_CONTINUATION_PATH = REPO_ROOT / r"scripts\train_nikkei_continuation_head.py"

EXPECTED_CONTINUATION_LABEL_COLUMNS = {
    "continuation_label_1d",
    "continuation_label_3d",
    "continuation_label_5d",
}
EXPECTED_CONTINUATION_VERSION = "nikkei_continuation_head_v1"


# 2026-04-28 CST: Added explicit file-based loading because the continuation
# head is introduced as an offline research slice first. Purpose: keep the
# contract test independent from package-install state and fail on missing
# governed entrypoints rather than import-path accidents.
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


class ContinuationBuilderContractTests(unittest.TestCase):
    # 2026-04-28 CST: Added because continuation head v1 must not invent a
    # second sample universe. Purpose: force the replay sample builder to expose
    # explicit continuation labels on top of the existing replay-labeled rows.
    def test_builder_derives_continuation_labels_from_replay_labels(self) -> None:
        builder = load_module("nikkei_continuation_builder_under_test", BUILD_SAMPLES_PATH)
        self.assertTrue(
            hasattr(builder, "derive_continuation_labels"),
            "builder must expose derive_continuation_labels",
        )
        self.assertEqual(
            getattr(builder, "CONTINUATION_TARGET_DEFINITION_VERSION", None),
            EXPECTED_CONTINUATION_VERSION,
        )
        labeled = builder.derive_continuation_labels(
            pd.DataFrame(
                [
                    {
                        "sample_id": "row-1",
                        "replay_label_1d": "correct_add",
                        "replay_label_3d": "acceptable_reduce",
                        "replay_label_5d": "premature_add",
                        "horizon_1d_close_return": 0.01,
                        "horizon_3d_close_return": -0.01,
                        "horizon_5d_close_return": -0.02,
                    },
                    {
                        "sample_id": "row-2",
                        "replay_label_1d": "inconclusive",
                        "replay_label_3d": "late_reduce",
                        "replay_label_5d": "correct_reduce",
                        "horizon_1d_close_return": None,
                        "horizon_3d_close_return": -0.03,
                        "horizon_5d_close_return": -0.01,
                    },
                ]
            )
        )
        self.assertTrue(EXPECTED_CONTINUATION_LABEL_COLUMNS.issubset(set(labeled.columns)))
        self.assertEqual(int(labeled.loc[0, "continuation_label_1d"]), 1)
        self.assertEqual(int(labeled.loc[0, "continuation_label_3d"]), 1)
        self.assertEqual(int(labeled.loc[0, "continuation_label_5d"]), 0)
        self.assertTrue(pd.isna(labeled.loc[1, "continuation_label_1d"]))
        self.assertEqual(int(labeled.loc[1, "continuation_label_3d"]), 0)
        self.assertEqual(int(labeled.loc[1, "continuation_label_5d"]), 1)

    # 2026-04-28 CST: Added because continuation training must exclude rows
    # whose replay outcomes are unavailable or intentionally non-supervised.
    # Purpose: prevent the trainer from silently fitting on inconclusive rows.
    def test_builder_marks_continuation_eligibility_and_horizon_observation_flags(self) -> None:
        builder = load_module("nikkei_continuation_eligibility_under_test", BUILD_SAMPLES_PATH)
        labeled = builder.derive_continuation_labels(
            pd.DataFrame(
                [
                    {
                        "sample_id": "row-1",
                        "replay_label_1d": "correct_add",
                        "replay_label_3d": "acceptable_add",
                        "replay_label_5d": "acceptable_reduce",
                        "horizon_1d_close_return": 0.01,
                        "horizon_3d_close_return": 0.02,
                        "horizon_5d_close_return": -0.01,
                    },
                    {
                        "sample_id": "row-2",
                        "replay_label_1d": "inconclusive",
                        "replay_label_3d": "inconclusive",
                        "replay_label_5d": "inconclusive",
                        "horizon_1d_close_return": None,
                        "horizon_3d_close_return": None,
                        "horizon_5d_close_return": None,
                    },
                ]
            )
        )
        self.assertIn("continuation_label_version", labeled.columns)
        self.assertIn("is_continuation_eligible", labeled.columns)
        self.assertIn("is_outcome_observed_1d", labeled.columns)
        self.assertIn("is_outcome_observed_3d", labeled.columns)
        self.assertIn("is_outcome_observed_5d", labeled.columns)
        self.assertTrue(bool(labeled.loc[0, "is_continuation_eligible"]))
        self.assertFalse(bool(labeled.loc[1, "is_continuation_eligible"]))
        self.assertTrue(bool(labeled.loc[0, "is_outcome_observed_1d"]))
        self.assertFalse(bool(labeled.loc[1, "is_outcome_observed_1d"]))


class ContinuationTrainerContractTests(unittest.TestCase):
    # 2026-04-28 CST: Added because the first continuation head needs one
    # stable callable trainer with machine-readable outputs. Purpose: lock the
    # artifact contract before the offline research lane expands.
    def test_training_script_smoke_contract(self) -> None:
        trainer = load_module("nikkei_continuation_trainer_under_test", TRAIN_CONTINUATION_PATH)
        self.assertTrue(
            hasattr(trainer, "train_continuation_head"),
            "trainer must expose train_continuation_head",
        )
        with tempfile.TemporaryDirectory() as tmp_dir:
            temp_root = Path(tmp_dir)
            sample_path = temp_root / "continuation_samples.csv"
            pd.DataFrame(
                [
                    {
                        "sample_id": "sample-1",
                        "signal_date": "2026-04-21",
                        "sample_source": "historical_research",
                        "signal_family": "breakout_followthrough",
                        "signal_direction": "add",
                        "action_type": "add_risk",
                        "base_position_v3": 0.35,
                        "rating_state": "risk_add",
                        "dist_res20": -0.02,
                        "dist_sup20": 0.03,
                        "dist_sup60": 0.08,
                        "weighted_vol_down": 0.01,
                        "component_above200_breadth": 0.90,
                        "avg_component_vr": 1.05,
                        "continuation_label_5d": 1,
                        "is_continuation_eligible": True,
                        "is_outcome_observed_5d": True,
                    },
                    {
                        "sample_id": "sample-2",
                        "signal_date": "2026-04-22",
                        "sample_source": "historical_research",
                        "signal_family": "breakdown_followthrough",
                        "signal_direction": "reduce",
                        "action_type": "reduce_risk",
                        "base_position_v3": 0.20,
                        "rating_state": "risk_reduce",
                        "dist_res20": 0.03,
                        "dist_sup20": 0.12,
                        "dist_sup60": 0.14,
                        "weighted_vol_down": 0.08,
                        "component_above200_breadth": 0.78,
                        "avg_component_vr": 1.20,
                        "continuation_label_5d": 0,
                        "is_continuation_eligible": True,
                        "is_outcome_observed_5d": True,
                    },
                    {
                        "sample_id": "sample-3",
                        "signal_date": "2026-04-23",
                        "sample_source": "live_journal",
                        "signal_family": "live_add_signal",
                        "signal_direction": "add",
                        "action_type": "add_risk",
                        "base_position_v3": 0.35,
                        "rating_state": "risk_add",
                        "dist_res20": -0.01,
                        "dist_sup20": 0.04,
                        "dist_sup60": 0.09,
                        "weighted_vol_down": 0.02,
                        "component_above200_breadth": 0.88,
                        "avg_component_vr": 0.97,
                        "continuation_label_5d": None,
                        "is_continuation_eligible": False,
                        "is_outcome_observed_5d": False,
                    },
                ]
            ).to_csv(sample_path, index=False, encoding="utf-8-sig")

            result = trainer.train_continuation_head(
                sample_path=sample_path,
                output_root=temp_root,
                label_horizon="5d",
            )
            summary = json.loads(Path(result["training_summary_path"]).read_text(encoding="utf-8"))

        self.assertIsInstance(result, dict, "trainer must return a machine-readable summary dict")
        self.assertEqual(result.get("target_definition_version"), EXPECTED_CONTINUATION_VERSION)
        self.assertIn("metrics_path", result)
        self.assertIn("predictions_path", result)
        self.assertIn("label_counts_path", result)
        self.assertIn("confusion_path", result)
        self.assertIn("training_summary_path", result)
        self.assertEqual(summary.get("label_horizon"), "5d")
        self.assertEqual(summary.get("observed_outcome_sample_count"), 2)
        self.assertEqual(summary.get("eligible_sample_count"), 2)


if __name__ == "__main__":
    unittest.main()
