#!/usr/bin/env python
"""
Contract tests for the Nikkei replay-classifier pipeline.
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
TRAIN_CLASSIFIER_PATH = REPO_ROOT / r"scripts\train_nikkei_replay_classifier.py"

EXPECTED_REPLAY_SAMPLE_FIELDS = {
    "sample_id",
    "signal_date",
    "sample_source",
    "signal_family",
    "signal_direction",
    "action_type",
    "base_position_v3",
    "rating_state",
    "dist_res20",
    "dist_sup20",
    "dist_sup60",
    "weighted_vol_down",
    "component_above200_breadth",
    "avg_component_vr",
    "horizon_1d_close_return",
    "horizon_3d_close_return",
    "horizon_5d_close_return",
}
EXPECTED_SAMPLE_SOURCES = {
    "historical_research",
    "live_journal",
}
EXPECTED_REPLAY_LABEL_VOCAB = {
    "correct_reduce",
    "acceptable_reduce",
    "premature_reduce",
    "late_reduce",
    "correct_add",
    "acceptable_add",
    "premature_add",
    "late_add",
    "inconclusive",
}
FORBIDDEN_GENERIC_TARGETS = {
    "positive_return_1w",
    "negative_return_1w",
    "generic_1w_target",
}


# 2026-04-28 CST: Added explicit file-based module loading because this replay
# contract test must keep working before the future modules exist. Purpose:
# fail with a targeted contract error instead of breaking the test harness.
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


class ReplayBuilderContractTests(unittest.TestCase):
    # 2026-04-28 CST: Added because the replay builder must expose one stable
    # sample schema before implementation expands. Purpose: prevent ad hoc field
    # drift between research rows and live-journal rows.
    def test_builder_publishes_governed_replay_sample_schema(self) -> None:
        builder = load_module("nikkei_replay_builder_under_test", BUILD_SAMPLES_PATH)
        self.assertTrue(
            hasattr(builder, "REPLAY_SAMPLE_REQUIRED_FIELDS"),
            "builder must publish REPLAY_SAMPLE_REQUIRED_FIELDS",
        )
        self.assertEqual(
            set(builder.REPLAY_SAMPLE_REQUIRED_FIELDS),
            EXPECTED_REPLAY_SAMPLE_FIELDS,
        )
        self.assertTrue(
            hasattr(builder, "build_replay_samples"),
            "builder must expose build_replay_samples",
        )

    # 2026-04-28 CST: Added because the replay dataset merges synthetic research
    # events and live journal evidence. Purpose: force the builder to tag source
    # provenance instead of silently flattening both truths together.
    def test_builder_marks_sample_source_layer(self) -> None:
        builder = load_module("nikkei_replay_builder_sources_under_test", BUILD_SAMPLES_PATH)
        self.assertTrue(
            hasattr(builder, "REPLAY_SAMPLE_SOURCES"),
            "builder must publish REPLAY_SAMPLE_SOURCES",
        )
        self.assertEqual(set(builder.REPLAY_SAMPLE_SOURCES), EXPECTED_SAMPLE_SOURCES)

    # 2026-04-28 CST: Added because the future replay model needs an auditable
    # event dataset, not just constants. Purpose: prove the builder can emit a
    # non-empty DataFrame with the approved core fields.
    def test_builder_emits_non_empty_sample_frame_with_required_fields(self) -> None:
        builder = load_module("nikkei_replay_builder_frame_under_test", BUILD_SAMPLES_PATH)
        frame = builder.build_replay_samples()
        self.assertIsInstance(frame, pd.DataFrame)
        self.assertGreater(len(frame), 0, "builder must emit at least one replay sample")
        self.assertTrue(EXPECTED_REPLAY_SAMPLE_FIELDS.issubset(set(frame.columns)))
        self.assertTrue(set(frame["sample_source"].dropna().unique()).issubset(EXPECTED_SAMPLE_SOURCES))
        self.assertIn(
            "is_replay_outcome_observed",
            frame.columns,
            "builder must mark whether each row has an observed replay outcome",
        )

    # 2026-04-28 CST: Added because the user explicitly rejected fallback to the
    # generic weekly-direction target. Purpose: ensure replay labels are derived
    # from event-horizon fields without requiring any generic 1w target column.
    def test_builder_derives_replay_labels_without_generic_1w_target(self) -> None:
        builder = load_module("nikkei_replay_builder_target_under_test", BUILD_SAMPLES_PATH)
        self.assertEqual(builder.TARGET_DEFINITION_VERSION, "nikkei_replay_label_v1")
        test_frame = pd.DataFrame(
            [
                {
                    "signal_direction": "reduce",
                    "action_type": "reduce_risk",
                    "horizon_1d_close_return": -0.01,
                    "horizon_3d_close_return": -0.02,
                    "horizon_5d_close_return": -0.03,
                    "horizon_1d_max_drawdown": -0.02,
                    "horizon_3d_max_drawdown": -0.04,
                    "horizon_5d_max_drawdown": -0.05,
                    "next_signal_primary_adjustment": -1,
                    "next_signal_secondary_adjustment": -1,
                }
            ]
        )
        labeled = builder.derive_replay_labels(test_frame)
        self.assertIn("replay_label_1d", labeled.columns)
        self.assertIn("replay_label_3d", labeled.columns)
        self.assertIn("replay_label_5d", labeled.columns)
        self.assertTrue(
            set(labeled[["replay_label_1d", "replay_label_3d", "replay_label_5d"]].stack().unique()).issubset(
                EXPECTED_REPLAY_LABEL_VOCAB
            )
        )
        self.assertTrue(set(labeled.columns).isdisjoint(FORBIDDEN_GENERIC_TARGETS))

    # 2026-04-28 CST: Added because the replay classifier must publish its label
    # vocabulary separately from raw sample generation. Purpose: lock the approved
    # outcome taxonomy before trainer implementation starts.
    def test_builder_publishes_replay_label_vocabulary(self) -> None:
        builder = load_module("nikkei_replay_builder_labels_under_test", BUILD_SAMPLES_PATH)
        self.assertTrue(
            hasattr(builder, "REPLAY_LABEL_VOCAB"),
            "builder must publish REPLAY_LABEL_VOCAB",
        )
        self.assertEqual(set(builder.REPLAY_LABEL_VOCAB), EXPECTED_REPLAY_LABEL_VOCAB)


class ReplayTrainerContractTests(unittest.TestCase):
    # 2026-04-28 CST: Added because the first replay trainer needs a smoke-level
    # machine-readable contract before more advanced evaluation work begins.
    # Purpose: guarantee one callable training entrypoint plus artifact outputs.
    def test_training_script_smoke_contract(self) -> None:
        trainer = load_module("nikkei_replay_trainer_under_test", TRAIN_CLASSIFIER_PATH)
        self.assertTrue(
            hasattr(trainer, "train_replay_classifier"),
            "trainer must expose train_replay_classifier",
        )
        with tempfile.TemporaryDirectory() as tmp_dir:
            temp_root = Path(tmp_dir)
            sample_path = temp_root / "replay_samples.csv"
            pd.DataFrame(
                [
                    {
                        "sample_id": "sample-1",
                        "signal_date": "2026-04-24",
                        "sample_source": "historical_research",
                        "signal_family": "breakdown_followthrough",
                        "signal_direction": "reduce",
                        "action_type": "reduce_risk",
                        "base_position_v3": 0.35,
                        "rating_state": "risk_reduce",
                        "dist_res20": 0.01,
                        "dist_sup20": 0.18,
                        "dist_sup60": 0.18,
                        "weighted_vol_down": 0.11,
                        "component_above200_breadth": 0.82,
                        "avg_component_vr": 1.13,
                        "horizon_1d_close_return": -0.01,
                        "horizon_3d_close_return": -0.02,
                        "horizon_5d_close_return": -0.03,
                        "is_replay_outcome_observed": True,
                        "replay_label_1d": "correct_reduce",
                        "replay_label_3d": "correct_reduce",
                        "replay_label_5d": "correct_reduce",
                    },
                    {
                        "sample_id": "sample-2",
                        "signal_date": "2026-04-25",
                        "sample_source": "historical_research",
                        "signal_family": "breakout_followthrough",
                        "signal_direction": "add",
                        "action_type": "add_risk",
                        "base_position_v3": 0.25,
                        "rating_state": "risk_add",
                        "dist_res20": -0.02,
                        "dist_sup20": 0.05,
                        "dist_sup60": 0.09,
                        "weighted_vol_down": 0.02,
                        "component_above200_breadth": 0.91,
                        "avg_component_vr": 0.98,
                        "horizon_1d_close_return": 0.01,
                        "horizon_3d_close_return": 0.03,
                        "horizon_5d_close_return": 0.04,
                        "is_replay_outcome_observed": False,
                        "replay_label_1d": "correct_add",
                        "replay_label_3d": "correct_add",
                        "replay_label_5d": "correct_add",
                    },
                ]
            ).to_csv(sample_path, index=False, encoding="utf-8-sig")

            result = trainer.train_replay_classifier(
                sample_path=sample_path,
                output_root=temp_root,
                label_horizon="5d",
            )
            summary = json.loads(Path(result["training_summary_path"]).read_text(encoding="utf-8"))

        self.assertIsInstance(result, dict, "trainer must return a machine-readable summary dict")
        self.assertEqual(result.get("target_definition_version"), "nikkei_replay_label_v1")
        self.assertEqual(set(result.get("label_vocabulary", [])), EXPECTED_REPLAY_LABEL_VOCAB)
        self.assertIn("metrics_path", result)
        self.assertIn("predictions_path", result)
        self.assertIn("confusion_path", result)
        self.assertIn("training_summary_path", result)
        self.assertEqual(summary.get("observed_outcome_sample_count"), 1)


if __name__ == "__main__":
    unittest.main()
