#!/usr/bin/env python
"""
Regression tests for the governed Nikkei daily workflow.
"""

from __future__ import annotations

import importlib.util
import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

import pandas as pd


REPO_ROOT = Path(r"D:\SM")
SCORER_PATH = REPO_ROOT / (
    r"docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts"
    r"\03_daily_hgb_rf_scoring_full_snapshot\daily_hgb_rf_v3_scoring.py"
)
WORKFLOW_PATH = REPO_ROOT / r"scripts\run_nikkei_hgb_rf_daily_workflow.py"


def load_module(module_name: str, path: Path):
    spec = importlib.util.spec_from_file_location(module_name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module {module_name} from {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


class DailyScorerContractTests(unittest.TestCase):
    def test_write_outputs_emits_policy_qualified_names_and_manifest(self) -> None:
        scorer = load_module("daily_scorer_under_test", SCORER_PATH)
        with tempfile.TemporaryDirectory() as tmp_dir:
            output_root = Path(tmp_dir)
            as_of_date = pd.Timestamp("2026-04-27")
            scores = pd.DataFrame(
                [
                    {
                        "model": "hgb_l2_leaf20_live",
                        "date": "2026-04-27",
                        "close": 100.0,
                        "base_position_v3": 0.35,
                        "pred_adjustment": -1,
                        "target_position_proxy": 0.10,
                        "prob_-1": 0.6,
                        "prob_0": 0.3,
                        "prob_1": 0.1,
                        "breakout20": 0,
                        "breakout60": 0,
                        "weighted_b20_vol": 0.2,
                        "weighted_vol_down": 0.1,
                        "action_vs_35pct": "reduce_or_hold_low",
                    }
                ]
            )
            metrics = pd.DataFrame([{"model": "hgb_l2_leaf20_live", "validation_accuracy": 0.5}])
            importances = pd.DataFrame([{"model": "hgb_l2_leaf20_live", "feature": "breakout20", "importance": 0.1}])
            drivers = pd.DataFrame([{"model": "hgb_l2_leaf20_live", "date": "2026-04-27", "feature": "breakout20"}])
            artifacts = pd.DataFrame(
                [
                    {
                        "contract_version": "nikkei_v3_hgb_adjustment.v1",
                        "model_set_version": "research_daily_hgb_rf_v3_20260427",
                        "model_id": "hgb_l2_leaf20_live",
                        "train_policy": "live_pre_year",
                        "as_of_date": "2026-04-27",
                        "adjustment": -1,
                        "base_position_v3": 0.35,
                        "target_position_proxy": 0.10,
                        "prob_down": 0.6,
                        "prob_neutral": 0.3,
                        "prob_up": 0.1,
                    }
                ]
            )

            scorer.write_outputs(
                output_root=output_root,
                scores=scores,
                metrics=metrics,
                importances=importances,
                drivers=drivers,
                artifacts=artifacts,
                as_of_date=as_of_date,
                train_policy="live_pre_year",
            )

            generated_names = sorted(path.name for path in output_root.iterdir())
            self.assertIn("05_latest_adjustment_artifacts_live_pre_year.csv", generated_names)
            self.assertIn(
                "hgb_l2_leaf20_live_live_pre_year_2026-04-27_adjustment.json",
                generated_names,
            )
            manifest_path = output_root / "06_daily_workflow_manifest_live_pre_year.json"
            self.assertTrue(manifest_path.exists(), "daily scorer must emit a machine-readable manifest")
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            self.assertEqual(manifest["train_policy"], "live_pre_year")
            self.assertEqual(manifest["as_of_date"], "2026-04-27")
            self.assertEqual(manifest["score_start_date"], "2026-04-27")
            self.assertEqual(manifest["latest_artifact_as_of_date"], "2026-04-27")
            self.assertIn("05_latest_adjustment_artifacts_live_pre_year.csv", manifest["generated_files"])


class DailyWorkflowContractTests(unittest.TestCase):
    def test_workflow_prefers_policy_qualified_live_artifacts(self) -> None:
        workflow = load_module("nikkei_daily_workflow_under_test", WORKFLOW_PATH)
        with tempfile.TemporaryDirectory() as tmp_dir:
            output_root = Path(tmp_dir)
            live_csv = output_root / "05_latest_adjustment_artifacts_live_pre_year.csv"
            pd.DataFrame(
                [
                    {
                        "model_id": "hgb_l2_leaf20_live",
                        "train_policy": "live_pre_year",
                        "as_of_date": "2026-04-27",
                        "adjustment": -1,
                        "target_position_proxy": 0.10,
                    }
                ]
            ).to_csv(live_csv, index=False, encoding="utf-8-sig")
            (output_root / "hgb_l2_leaf20_live_2026-04-27_adjustment.json").write_text("{}", encoding="utf-8")

            loaded = workflow.load_live_artifact_table(output_root=output_root, as_of_date="2026-04-27")
            self.assertEqual(list(loaded["train_policy"]), ["live_pre_year"])

            with self.assertRaisesRegex(FileNotFoundError, "policy-qualified"):
                workflow.load_live_artifact_table(
                    output_root=output_root,
                    as_of_date="2026-04-27",
                    artifact_csv_name="05_latest_adjustment_artifacts.csv",
                )

    def test_workflow_prints_stable_hgb_rf_summary(self) -> None:
        workflow = load_module("nikkei_daily_workflow_under_test", WORKFLOW_PATH)
        with tempfile.TemporaryDirectory() as tmp_dir:
            output_root = Path(tmp_dir)
            summary_csv = output_root / "05_latest_adjustment_artifacts_live_pre_year.csv"
            pd.DataFrame(
                [
                    {
                        "model_id": "hgb_l2_leaf20_live",
                        "train_policy": "live_pre_year",
                        "as_of_date": "2026-04-27",
                        "adjustment": -1,
                        "target_position_proxy": 0.10,
                        "prob_down": 0.60,
                        "prob_neutral": 0.30,
                        "prob_up": 0.10,
                    },
                    {
                        "model_id": "rf_depth4_leaf20_live",
                        "train_policy": "live_pre_year",
                        "as_of_date": "2026-04-27",
                        "adjustment": 0,
                        "target_position_proxy": 0.35,
                        "prob_down": 0.25,
                        "prob_neutral": 0.55,
                        "prob_up": 0.20,
                    },
                ]
            ).to_csv(summary_csv, index=False, encoding="utf-8-sig")
            manifest_path = output_root / "06_daily_workflow_manifest_live_pre_year.json"
            manifest_path.write_text(
                json.dumps(
                    {
                        "train_policy": "live_pre_year",
                        "as_of_date": "2026-04-27",
                        "score_start_date": "2026-04-01",
                        "generated_files": [summary_csv.name],
                    },
                    ensure_ascii=False,
                    indent=2,
                ),
                encoding="utf-8",
            )

            buffer = io.StringIO()
            with redirect_stdout(buffer):
                workflow.print_latest_summary(
                    artifact_table=workflow.load_live_artifact_table(output_root=output_root, as_of_date="2026-04-27"),
                    manifest=workflow.load_manifest(output_root=output_root, train_policy="live_pre_year"),
                )
            text = buffer.getvalue()
            self.assertIn("train_policy=live_pre_year", text)
            self.assertIn("as_of_date=2026-04-27", text)
            self.assertIn("HGB", text)
            self.assertIn("RF", text)

    def test_workflow_falls_back_to_latest_available_artifact_on_or_before_requested_date(self) -> None:
        workflow = load_module("nikkei_daily_workflow_under_test", WORKFLOW_PATH)
        with tempfile.TemporaryDirectory() as tmp_dir:
            output_root = Path(tmp_dir)
            summary_csv = output_root / "05_latest_adjustment_artifacts_live_pre_year.csv"
            pd.DataFrame(
                [
                    {
                        "model_id": "hgb_l2_leaf20_live",
                        "train_policy": "live_pre_year",
                        "as_of_date": "2026-04-24",
                        "adjustment": -1,
                        "target_position_proxy": 0.10,
                        "prob_down": 0.60,
                        "prob_neutral": 0.30,
                        "prob_up": 0.10,
                    },
                    {
                        "model_id": "rf_depth4_leaf20_live",
                        "train_policy": "live_pre_year",
                        "as_of_date": "2026-04-24",
                        "adjustment": 0,
                        "target_position_proxy": 0.35,
                        "prob_down": 0.25,
                        "prob_neutral": 0.55,
                        "prob_up": 0.20,
                    },
                ]
            ).to_csv(summary_csv, index=False, encoding="utf-8-sig")
            manifest_path = output_root / "06_daily_workflow_manifest_live_pre_year.json"
            manifest_path.write_text(
                json.dumps(
                    {
                        "train_policy": "live_pre_year",
                        "as_of_date": "2026-04-27",
                        "score_start_date": "2026-04-01",
                        "latest_artifact_as_of_date": "2026-04-24",
                        "generated_files": [summary_csv.name],
                    },
                    ensure_ascii=False,
                    indent=2,
                ),
                encoding="utf-8",
            )

            loaded = workflow.load_live_artifact_table(output_root=output_root, as_of_date="2026-04-27")
            self.assertEqual(list(loaded["as_of_date"].astype(str).unique()), ["2026-04-24"])


if __name__ == "__main__":
    unittest.main()
