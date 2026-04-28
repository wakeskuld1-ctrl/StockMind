#!/usr/bin/env python
"""
Governed operator entrypoint for the Nikkei HGB/RF daily workflow.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import sys
from pathlib import Path
from typing import Any

import pandas as pd


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SCORER_PATH = REPO_ROOT / (
    r"docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts"
    r"\03_daily_hgb_rf_scoring_full_snapshot\daily_hgb_rf_v3_scoring.py"
)
DEFAULT_ANALYSIS_ROOT = Path(
    r"D:\.stockmind_runtime\nikkei_current_rerun_20260426_direction_head_yfinance_10y_long_volume_behavior\analysis_exports"
)
DEFAULT_OUTPUT_ROOT = Path(r"D:\.stockmind_runtime\nikkei_etf_daily_model_scoring_20260427")
DEFAULT_JOURNAL_UPSERT_PATH = Path(
    r"C:\Users\wakes\.codex\skills\nikkei-live-journal\scripts\upsert_journal.py"
)
LIVE_POLICY = "live_pre_year"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run the Nikkei daily HGB/RF live-policy workflow.")
    parser.add_argument("--as-of-date", required=True)
    parser.add_argument("--score-start-date", required=True)
    parser.add_argument("--analysis-root", default=str(DEFAULT_ANALYSIS_ROOT))
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    parser.add_argument("--journal-dir", default=None)
    parser.add_argument("--journal-etf-symbol", default="159866")
    return parser.parse_args()


# 2026-04-28 CST: Added explicit module loading so this operator entrypoint can
# call the governed scorer without mutating PYTHONPATH. Purpose: keep the daily
# workflow callable from scripts while preserving the research file location.
def load_scorer_module(module_path: Path = DEFAULT_SCORER_PATH):
    spec = importlib.util.spec_from_file_location("nikkei_daily_scorer", module_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load scorer module from {module_path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


# 2026-04-28 CST: Added explicit journal module loading because the governed
# daily workflow now owns signal-fact persistence into the live journal. Purpose:
# reuse the journal schema implementation instead of duplicating CSV/snapshot code.
def load_journal_upsert_module(module_path: Path = DEFAULT_JOURNAL_UPSERT_PATH):
    spec = importlib.util.spec_from_file_location("nikkei_live_journal_upsert", module_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load journal upsert module from {module_path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


# 2026-04-28 CST: Added because live guidance must only consume policy-qualified
# artifact tables. Purpose: reject legacy ambiguous file names before execution.
def load_live_artifact_table(
    *,
    output_root: Path,
    as_of_date: str,
    artifact_csv_name: str | None = None,
    train_policy: str = LIVE_POLICY,
) -> pd.DataFrame:
    csv_name = artifact_csv_name or f"05_latest_adjustment_artifacts_{train_policy}.csv"
    if train_policy not in csv_name:
        raise FileNotFoundError("policy-qualified live artifact csv is required")
    artifact_path = output_root / csv_name
    if not artifact_path.exists():
        raise FileNotFoundError(f"policy-qualified live artifact csv not found: {artifact_path}")
    frame = pd.read_csv(artifact_path)
    policy_frame = frame[frame["train_policy"] == train_policy].copy()
    if policy_frame.empty:
        raise ValueError(f"no {train_policy} artifact rows found in {artifact_path}")
    policy_frame["artifact_as_of_date"] = pd.to_datetime(policy_frame["as_of_date"])
    requested_as_of_date = pd.Timestamp(as_of_date)
    eligible = policy_frame[policy_frame["artifact_as_of_date"] <= requested_as_of_date].copy()
    if eligible.empty:
        raise ValueError(
            f"no {train_policy} artifact rows found on or before as_of_date={as_of_date}"
        )
    effective_as_of_date = eligible["artifact_as_of_date"].max()
    filtered = eligible[eligible["artifact_as_of_date"] == effective_as_of_date].copy()
    filtered["requested_as_of_date"] = requested_as_of_date.date().isoformat()
    filtered["effective_as_of_date"] = effective_as_of_date.date().isoformat()
    return filtered.drop(columns=["artifact_as_of_date"]).sort_values("model_id").reset_index(drop=True)


# 2026-04-28 CST: Added so workflow consumers can verify the scoring batch
# boundary in one file. Purpose: pair the printed summary with machine-readable
# metadata that names the same live-policy run.
def load_manifest(*, output_root: Path, train_policy: str = LIVE_POLICY) -> dict[str, object]:
    manifest_path = output_root / f"06_daily_workflow_manifest_{train_policy}.json"
    if not manifest_path.exists():
        raise FileNotFoundError(f"workflow manifest not found: {manifest_path}")
    return json.loads(manifest_path.read_text(encoding="utf-8"))


def _model_display_name(model_id: str) -> str:
    if model_id.startswith("hgb"):
        return "HGB"
    if model_id.startswith("rf"):
        return "RF"
    return model_id


# 2026-04-28 CST: Added because signal-fact journaling needs one stable mapping
# from HGB/RF adjustments into an operator-facing state label and action. Purpose:
# keep workflow output and journal rows aligned on the same position-language contract.
def classify_rating_state(primary_adjustment: int, secondary_adjustment: int) -> tuple[str, str]:
    if primary_adjustment <= -1 and secondary_adjustment <= -1:
        return "risk_reduce", "sell_to_target"
    if primary_adjustment <= -1 or secondary_adjustment <= -1:
        return "mixed_hold_watch", "hold_watch"
    if primary_adjustment >= 1 and secondary_adjustment >= 1:
        return "risk_add", "buy_to_target"
    if primary_adjustment >= 1 or secondary_adjustment >= 1:
        return "mixed_hold_watch", "probe_or_hold"
    return "neutral_hold", "hold"


# 2026-04-28 CST: Added because downstream journal replay needs explicit HGB/RF
# role separation, not just two arbitrary artifact rows. Purpose: pick the primary
# and secondary model rows deterministically from the governed artifact table.
def select_primary_secondary_rows(artifact_table: pd.DataFrame) -> tuple[pd.Series, pd.Series]:
    if artifact_table.empty:
        raise ValueError("artifact table is empty")
    primary_candidates = artifact_table[artifact_table["model_id"].astype(str).str.startswith("hgb")]
    secondary_candidates = artifact_table[artifact_table["model_id"].astype(str).str.startswith("rf")]
    if primary_candidates.empty or secondary_candidates.empty:
        raise ValueError("artifact table must contain both HGB and RF rows")
    return primary_candidates.iloc[0], secondary_candidates.iloc[0]


# 2026-04-28 CST: Added because signal-fact snapshots must preserve exact source
# references for later replay. Purpose: convert manifest file names into absolute
# paths and include the manifest itself as a first-class provenance artifact.
def build_source_artifacts(*, output_root: Path, manifest: dict[str, Any]) -> list[str]:
    source_artifacts = [
        str(output_root / file_name)
        for file_name in manifest.get("generated_files", [])
        if isinstance(file_name, str) and file_name.strip()
    ]
    source_artifacts.append(str(output_root / f"06_daily_workflow_manifest_{manifest['train_policy']}.json"))
    deduped: list[str] = []
    seen = set()
    for artifact_path in source_artifacts:
        if artifact_path not in seen:
            deduped.append(artifact_path)
            seen.add(artifact_path)
    return deduped


# 2026-04-28 CST: Added because Task 5 requires the governed workflow to persist
# signal facts into the Nikkei live journal without mixing execution/review data.
# Purpose: derive one stable journal row from the effective HGB/RF live artifacts.
def build_signal_journal_record(
    *,
    artifact_table: pd.DataFrame,
    manifest: dict[str, Any],
    output_root: Path,
    journal_etf_symbol: str,
) -> dict[str, Any]:
    primary_row, secondary_row = select_primary_secondary_rows(artifact_table)
    primary_adjustment = int(primary_row["adjustment"])
    secondary_adjustment = int(secondary_row["adjustment"])
    rating_state, planned_action = classify_rating_state(primary_adjustment, secondary_adjustment)
    signal_date = str(primary_row.get("effective_as_of_date") or primary_row["as_of_date"])
    signal_rationale = (
        f"HGB adjustment={primary_adjustment} and RF adjustment={secondary_adjustment}. "
        f"Primary target_position_proxy={float(primary_row['target_position_proxy']):.2f}."
    )
    return {
        "signal_date": signal_date,
        "market_symbol": "NK225",
        "etf_symbol": journal_etf_symbol,
        "train_policy": str(primary_row["train_policy"]),
        "primary_model_id": str(primary_row["model_id"]),
        "secondary_model_id": str(secondary_row["model_id"]),
        "primary_adjustment": primary_adjustment,
        "secondary_adjustment": secondary_adjustment,
        "primary_prob_down": float(primary_row.get("prob_down", 0.0)),
        "secondary_prob_down": float(secondary_row.get("prob_down", 0.0)),
        "base_position_v3": float(primary_row["base_position_v3"]),
        "target_position_proxy": float(primary_row["target_position_proxy"]),
        "rating_state": rating_state,
        "planned_action": planned_action,
        "signal_rationale": signal_rationale,
        "source_artifacts": build_source_artifacts(output_root=output_root, manifest=manifest),
    }


# 2026-04-28 CST: Added because the journal write must occur inside the governed
# workflow run, not as a manual follow-up step. Purpose: persist only signal-fact
# fields for the selected ETF while leaving execution/review columns untouched.
def maybe_persist_signal_journal_record(
    *,
    artifact_table: pd.DataFrame,
    manifest: dict[str, Any],
    output_root: Path,
    journal_dir: str | Path | None,
    journal_etf_symbol: str,
) -> dict[str, Any] | None:
    if journal_dir is None:
        return None
    upsert_module = load_journal_upsert_module()
    record = build_signal_journal_record(
        artifact_table=artifact_table,
        manifest=manifest,
        output_root=output_root,
        journal_etf_symbol=journal_etf_symbol,
    )
    return upsert_module.persist_record(Path(journal_dir), record, render_markdown=True)


# 2026-04-28 CST: Added stable stdout formatting because operators need the same
# fields every day for comparison and later journal replay. Purpose: print one
# deterministic summary block per HGB/RF artifact row.
def print_latest_summary(*, artifact_table: pd.DataFrame, manifest: dict[str, object]) -> None:
    effective_as_of_date = (
        artifact_table["effective_as_of_date"].iloc[0]
        if "effective_as_of_date" in artifact_table.columns and not artifact_table.empty
        else manifest.get("latest_artifact_as_of_date", manifest["as_of_date"])
    )
    print(
        "Nikkei daily workflow summary "
        f"train_policy={manifest['train_policy']} "
        f"as_of_date={manifest['as_of_date']} "
        f"effective_signal_date={effective_as_of_date} "
        f"score_start_date={manifest['score_start_date']}"
    )
    for _, row in artifact_table.iterrows():
        print(
            f"{_model_display_name(str(row['model_id']))}: "
            f"adjustment={int(row['adjustment'])} "
            f"target_position_proxy={float(row['target_position_proxy']):.2f} "
            f"prob_down={float(row.get('prob_down', 0.0)):.2f} "
            f"prob_neutral={float(row.get('prob_neutral', 0.0)):.2f} "
            f"prob_up={float(row.get('prob_up', 0.0)):.2f}"
        )


# 2026-04-28 CST: Added as the shared workflow body for tests and CLI. Purpose:
# run live_pre_year scoring once, then read back the governed artifact table and
# manifest from disk instead of trusting in-memory objects.
def run_daily_workflow(
    *,
    as_of_date: str,
    score_start_date: str,
    analysis_root: str | Path = DEFAULT_ANALYSIS_ROOT,
    output_root: str | Path = DEFAULT_OUTPUT_ROOT,
    journal_dir: str | Path | None = None,
    journal_etf_symbol: str = "159866",
):
    scorer = load_scorer_module()
    output_root_path = Path(output_root)
    scorer.run_daily_scoring(
        as_of_date=as_of_date,
        score_start_date=score_start_date,
        analysis_root=str(analysis_root),
        output_root=str(output_root_path),
        train_policy=LIVE_POLICY,
    )
    artifact_table = load_live_artifact_table(output_root=output_root_path, as_of_date=as_of_date)
    manifest = load_manifest(output_root=output_root_path, train_policy=LIVE_POLICY)
    journal_result = maybe_persist_signal_journal_record(
        artifact_table=artifact_table,
        manifest=manifest,
        output_root=output_root_path,
        journal_dir=journal_dir,
        journal_etf_symbol=journal_etf_symbol,
    )
    print_latest_summary(artifact_table=artifact_table, manifest=manifest)
    return {
        "artifact_table": artifact_table,
        "manifest": manifest,
        "output_root": output_root_path,
        "journal_result": journal_result,
    }


def main() -> None:
    args = parse_args()
    run_daily_workflow(
        as_of_date=args.as_of_date,
        score_start_date=args.score_start_date,
        analysis_root=args.analysis_root,
        output_root=args.output_root,
        journal_dir=args.journal_dir,
        journal_etf_symbol=args.journal_etf_symbol,
    )


if __name__ == "__main__":
    main()
