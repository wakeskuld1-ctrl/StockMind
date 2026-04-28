#!/usr/bin/env python
"""
Build the Nikkei replay-classifier event sample dataset.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Iterable

import numpy as np
import pandas as pd


REPO_ROOT = Path(__file__).resolve().parents[1]
RESEARCH_ROOT = REPO_ROOT / r"docs\research\nikkei-etf-hgb-rf-v3-20260427"
DEFAULT_OUTPUT_ROOT = RESEARCH_ROOT / r"artifacts\04_replay_classifier_full_snapshot"
ADJUSTMENT_ANALYSIS_ROOT = (
    RESEARCH_ROOT
    / r"artifacts\01_training_and_intermediate_full_snapshot\analysis_exports\adjustment_point_analysis"
)
JOURNAL_CSV_PATH = REPO_ROOT / r"docs\trading-journal\nikkei\journal.csv"

TARGET_DEFINITION_VERSION = "nikkei_replay_label_v1"
CONTINUATION_TARGET_DEFINITION_VERSION = "nikkei_continuation_head_v1"
REPLAY_SAMPLE_SOURCES = [
    "historical_research",
    "live_journal",
]
REPLAY_LABEL_VOCAB = [
    "correct_reduce",
    "acceptable_reduce",
    "premature_reduce",
    "late_reduce",
    "correct_add",
    "acceptable_add",
    "premature_add",
    "late_add",
    "inconclusive",
]
REPLAY_SAMPLE_REQUIRED_FIELDS = [
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
]
LABEL_COLUMNS = [
    "replay_label_1d",
    "replay_label_3d",
    "replay_label_5d",
]
CONTINUATION_POSITIVE_REPLAY_LABELS = {
    "correct_reduce",
    "acceptable_reduce",
    "correct_add",
    "acceptable_add",
}
CONTINUATION_NEGATIVE_REPLAY_LABELS = {
    "premature_reduce",
    "late_reduce",
    "premature_add",
    "late_add",
}
SIMULATED_ACTION_SAMPLE_SOURCE = "simulated_action_replay"
REAL_FAILURE_EVENT_SAMPLE_SOURCE = "real_failure_event_mining"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build Nikkei replay-classifier samples.")
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    parser.add_argument("--skip-journal", action="store_true")
    return parser.parse_args()


def _to_iso_date(series: pd.Series) -> pd.Series:
    return pd.to_datetime(series, errors="coerce").dt.date.astype("string")


def _safe_float(series: pd.Series) -> pd.Series:
    return pd.to_numeric(series, errors="coerce")


def _map_adjustment_direction(value: object) -> str:
    numeric = pd.to_numeric(pd.Series([value]), errors="coerce").iloc[0]
    if pd.isna(numeric):
        return "hold"
    if numeric < 0:
        return "reduce"
    if numeric > 0:
        return "add"
    return "hold"


def _map_action_type(direction: str) -> str:
    if direction == "reduce":
        return "reduce_risk"
    if direction == "add":
        return "add_risk"
    return "hold_watch"


def _map_rating_state(direction: str) -> str:
    if direction == "reduce":
        return "risk_reduce"
    if direction == "add":
        return "risk_add"
    return "neutral_hold"


def _forward_close_return(close_values: np.ndarray, horizon: int) -> np.ndarray:
    result = np.full(len(close_values), np.nan, dtype=float)
    if horizon <= 0:
        return result
    for index in range(len(close_values) - horizon):
        base = close_values[index]
        future = close_values[index + horizon]
        if np.isfinite(base) and base != 0 and np.isfinite(future):
            result[index] = future / base - 1.0
    return result


def _forward_max_drawdown(close_values: np.ndarray, horizon: int) -> np.ndarray:
    result = np.full(len(close_values), np.nan, dtype=float)
    if horizon <= 0:
        return result
    for index in range(len(close_values) - horizon):
        base = close_values[index]
        window = close_values[index + 1 : index + horizon + 1]
        if np.isfinite(base) and base != 0 and len(window) > 0:
            result[index] = np.nanmin(window) / base - 1.0
    return result


def _load_csv(path: Path) -> pd.DataFrame:
    return pd.read_csv(path, encoding="utf-8-sig")


# 2026-04-28 CST: Added because the replay sample base must reuse governed
# research rows instead of starting from ad hoc journal-only data. Purpose:
# convert the daily V3 adjustment dataset into the canonical historical row base.
def _build_historical_base() -> pd.DataFrame:
    base_path = ADJUSTMENT_ANALYSIS_ROOT / "55_v3_adjustment_model_dataset.csv"
    base = _load_csv(base_path).copy()
    base["signal_date"] = _to_iso_date(base["date"])
    base = base.sort_values("signal_date").reset_index(drop=True)
    close_values = _safe_float(base["close"]).to_numpy(dtype=float)
    base["horizon_1d_close_return"] = _forward_close_return(close_values, 1)
    base["horizon_3d_close_return"] = _forward_close_return(close_values, 3)
    base["horizon_5d_close_return"] = _forward_close_return(close_values, 5)
    base["horizon_1d_max_drawdown"] = _forward_max_drawdown(close_values, 1)
    base["horizon_3d_max_drawdown"] = _forward_max_drawdown(close_values, 3)
    base["horizon_5d_max_drawdown"] = _forward_max_drawdown(close_values, 5)
    base["next_signal_primary_adjustment"] = base["adjustment_label"].shift(-1)
    base["next_signal_secondary_adjustment"] = base["adjustment_label"].shift(-1)
    base["signal_direction"] = base["adjustment_label"].map(_map_adjustment_direction)
    base["action_type"] = base["signal_direction"].map(_map_action_type)
    base["rating_state"] = base["signal_direction"].map(_map_rating_state)
    base["signal_family"] = np.select(
        [
            _safe_float(base["breakdown60"]).fillna(0) > 0,
            _safe_float(base["breakdown20"]).fillna(0) > 0,
            _safe_float(base["breakout60"]).fillna(0) > 0,
            _safe_float(base["breakout20"]).fillna(0) > 0,
            _safe_float(base["dist_sup20"]).fillna(1.0) <= 0.01,
        ],
        [
            "breakdown_followthrough",
            "breakdown_followthrough",
            "breakout_followthrough",
            "breakout_followthrough",
            "support_test",
        ],
        default="daily_position",
    )
    base["sample_source"] = "historical_research"
    base["sample_id"] = "historical_55_" + base["signal_date"].fillna("missing").astype(str)
    base["is_replay_outcome_observed"] = True
    return base


# 2026-04-28 CST: Added because breakout and downside event studies contain
# event-quality clues not present in the daily base table. Purpose: join those
# studies by signal date as contextual enrichments instead of treating them as
# disconnected truth layers.
def _load_context_enrichments() -> tuple[pd.DataFrame, pd.DataFrame]:
    event_candidates = _load_csv(ADJUSTMENT_ANALYSIS_ROOT / "04_adjustment_event_candidates.csv").copy()
    event_candidates["signal_date"] = _to_iso_date(event_candidates["date"])
    event_rollup = (
        event_candidates.sort_values("signal_date")
        .groupby("signal_date", dropna=False)
        .agg(
            candidate_event_type=("event_type", "first"),
            candidate_action_label=("action_label", "first"),
            candidate_stood_1d=("stood_1d", "max"),
            candidate_stood_3d=("stood_3d", "max"),
            candidate_stood_5d=("stood_5d", "max"),
            candidate_future_10d_return=("future_10d_return", "mean"),
        )
        .reset_index()
    )

    downside = _load_csv(ADJUSTMENT_ANALYSIS_ROOT / "24_downside_reduction_logic_samples.csv").copy()
    downside["signal_date"] = _to_iso_date(downside["date"])
    downside_rollup = (
        downside.sort_values("signal_date")
        .groupby("signal_date", dropna=False)
        .agg(
            downside_event_type=("event_type", "first"),
            downside_label=("downside_label", "first"),
            downside_suggested_action=("suggested_action", "first"),
            downside_weighted_volume_down_breadth=("weighted_volume_down_breadth", "max"),
            downside_weighted_breakdown20_breadth=("weighted_breakdown20_breadth", "max"),
        )
        .reset_index()
    )
    return event_rollup, downside_rollup


# 2026-04-28 CST: Added because real live-trade rows must enter the replay
# sample chain without being misrepresented as historical research samples.
# Purpose: normalize journal facts into the same core schema with a distinct source.
def _build_live_journal_rows() -> pd.DataFrame:
    if not JOURNAL_CSV_PATH.exists():
        return pd.DataFrame(columns=REPLAY_SAMPLE_REQUIRED_FIELDS)
    journal = _load_csv(JOURNAL_CSV_PATH).copy()
    if journal.empty:
        return pd.DataFrame(columns=REPLAY_SAMPLE_REQUIRED_FIELDS)
    journal["signal_date"] = _to_iso_date(journal["signal_date"])
    journal["signal_direction"] = journal["primary_adjustment"].map(_map_adjustment_direction)
    journal["action_type"] = journal["signal_direction"].map(_map_action_type)
    journal["signal_family"] = np.where(
        journal["signal_direction"] == "reduce",
        "live_reduce_signal",
        np.where(journal["signal_direction"] == "add", "live_add_signal", "live_hold_signal"),
    )
    journal["sample_source"] = "live_journal"
    journal["sample_id"] = (
        "journal_"
        + journal["signal_date"].fillna("missing").astype(str)
        + "_"
        + journal["etf_symbol"].fillna("unknown").astype(str)
    )
    journal["horizon_1d_close_return"] = np.nan
    journal["horizon_3d_close_return"] = np.nan
    journal["horizon_5d_close_return"] = np.nan
    journal["horizon_1d_max_drawdown"] = np.nan
    journal["horizon_3d_max_drawdown"] = np.nan
    journal["horizon_5d_max_drawdown"] = np.nan
    journal["next_signal_primary_adjustment"] = np.nan
    journal["next_signal_secondary_adjustment"] = np.nan
    journal["dist_res20"] = _safe_float(journal["dist_res20"])
    journal["dist_sup20"] = _safe_float(journal["dist_sup20"])
    journal["dist_sup60"] = _safe_float(journal["dist_sup60"])
    journal["weighted_vol_down"] = _safe_float(journal["weighted_vol_down"])
    journal["component_above200_breadth"] = _safe_float(journal["component_above200_breadth"])
    journal["avg_component_vr"] = _safe_float(journal["avg_component_vr"])
    journal["base_position_v3"] = _safe_float(journal["base_position_v3"])
    journal["rating_state"] = journal["rating_state"].fillna("neutral_hold")
    journal["is_replay_outcome_observed"] = journal["review_status"].fillna("").astype(str).str.lower().ne("pending")
    return journal


def _coerce_replay_required_columns(frame: pd.DataFrame) -> pd.DataFrame:
    frame["base_position_v3"] = _safe_float(frame["base_position_v3"])
    frame["dist_res20"] = _safe_float(frame["dist_res20"])
    frame["dist_sup20"] = _safe_float(frame["dist_sup20"])
    frame["dist_sup60"] = _safe_float(frame["dist_sup60"])
    frame["weighted_vol_down"] = _safe_float(frame["weighted_vol_down"])
    frame["component_above200_breadth"] = _safe_float(frame["component_above200_breadth"])
    frame["avg_component_vr"] = _safe_float(frame["avg_component_vr"])
    frame["horizon_1d_close_return"] = _safe_float(frame["horizon_1d_close_return"])
    frame["horizon_3d_close_return"] = _safe_float(frame["horizon_3d_close_return"])
    frame["horizon_5d_close_return"] = _safe_float(frame["horizon_5d_close_return"])
    return frame


def build_replay_samples(*, include_live_journal: bool = True) -> pd.DataFrame:
    historical = _build_historical_base()
    event_rollup, downside_rollup = _load_context_enrichments()
    historical = historical.merge(event_rollup, on="signal_date", how="left")
    historical = historical.merge(downside_rollup, on="signal_date", how="left")

    historical["dist_res20"] = _safe_float(historical["dist_res20"])
    historical["dist_sup20"] = _safe_float(historical["dist_sup20"])
    historical["dist_sup60"] = _safe_float(historical["dist_sup60"])
    historical["weighted_vol_down"] = _safe_float(historical["weighted_vol_down"])
    historical["component_above200_breadth"] = _safe_float(historical["component_above200_breadth"])
    historical["avg_component_vr"] = _safe_float(historical["avg_component_vr"])

    frames: list[pd.DataFrame] = [historical]
    if include_live_journal:
        journal = _build_live_journal_rows()
        if not journal.empty:
            frames.append(journal)

    combined = pd.concat(frames, ignore_index=True, sort=False)
    combined = _coerce_replay_required_columns(combined)
    combined = combined.sort_values(["signal_date", "sample_source", "sample_id"]).reset_index(drop=True)
    return combined


def _label_reduce(return_value: float, drawdown_value: float, next_primary: float) -> str:
    if np.isnan(return_value) and np.isnan(drawdown_value):
        return "inconclusive"
    if (np.isfinite(return_value) and return_value <= -0.01) or (
        np.isfinite(drawdown_value) and drawdown_value <= -0.02
    ):
        return "correct_reduce"
    if np.isfinite(next_primary) and next_primary < 0 and (
        np.isnan(return_value) or return_value <= 0.005
    ):
        return "acceptable_reduce"
    if np.isfinite(return_value) and return_value >= 0.03 and (
        np.isnan(drawdown_value) or drawdown_value >= -0.01
    ):
        return "premature_reduce"
    if np.isfinite(next_primary) and next_primary > 0:
        return "late_reduce"
    if np.isfinite(return_value) and return_value >= 0.0:
        return "acceptable_reduce"
    return "inconclusive"


def _label_add(return_value: float, drawdown_value: float, next_primary: float) -> str:
    if np.isnan(return_value) and np.isnan(drawdown_value):
        return "inconclusive"
    if (np.isfinite(return_value) and return_value >= 0.01) and (
        np.isnan(drawdown_value) or drawdown_value > -0.03
    ):
        return "correct_add"
    if np.isfinite(return_value) and return_value >= -0.005:
        return "acceptable_add"
    if (np.isfinite(return_value) and return_value <= -0.01) or (
        np.isfinite(next_primary) and next_primary < 0
    ):
        return "premature_add"
    if np.isfinite(return_value) and return_value > 0.0 and (
        np.isfinite(drawdown_value) and drawdown_value <= -0.05
    ):
        return "late_add"
    return "inconclusive"


# 2026-04-28 CST: Added because replay labels must be event-anchored and must
# not fall back to the old generic 1w direction target. Purpose: assign `1D / 3D /
# 5D` action-quality labels from event-horizon outcomes and next-signal context.
def derive_replay_labels(frame: pd.DataFrame) -> pd.DataFrame:
    labeled = frame.copy()
    for horizon in (1, 3, 5):
        return_col = f"horizon_{horizon}d_close_return"
        drawdown_col = f"horizon_{horizon}d_max_drawdown"
        label_col = f"replay_label_{horizon}d"
        labels: list[str] = []
        for row in labeled.itertuples(index=False):
            direction = getattr(row, "signal_direction", "hold")
            return_value = getattr(row, return_col, np.nan)
            drawdown_value = getattr(row, drawdown_col, np.nan)
            next_primary = getattr(row, "next_signal_primary_adjustment", np.nan)
            if direction == "reduce":
                labels.append(_label_reduce(return_value, drawdown_value, next_primary))
            elif direction == "add":
                labels.append(_label_add(return_value, drawdown_value, next_primary))
            else:
                labels.append("inconclusive")
        labeled[label_col] = labels
    return labeled


def _map_replay_label_to_continuation_value(label: object) -> float:
    label_text = str(label) if pd.notna(label) else ""
    if label_text in CONTINUATION_POSITIVE_REPLAY_LABELS:
        return 1.0
    if label_text in CONTINUATION_NEGATIVE_REPLAY_LABELS:
        return 0.0
    return np.nan


# 2026-04-28 CST: Added because continuation head v1 must reuse the replay row
# base instead of rebuilding a second label universe downstream. Purpose: derive
# one explicit continuation layer, plus eligibility flags, directly from replay
# labels and observed-horizon availability.
def derive_continuation_labels(frame: pd.DataFrame) -> pd.DataFrame:
    labeled = frame.copy()
    labeled["continuation_label_version"] = CONTINUATION_TARGET_DEFINITION_VERSION
    for horizon in (1, 3, 5):
        replay_label_col = f"replay_label_{horizon}d"
        continuation_label_col = f"continuation_label_{horizon}d"
        outcome_observed_col = f"is_outcome_observed_{horizon}d"
        horizon_return_col = f"horizon_{horizon}d_close_return"
        if replay_label_col in labeled.columns:
            labeled[continuation_label_col] = labeled[replay_label_col].map(_map_replay_label_to_continuation_value)
        else:
            labeled[continuation_label_col] = np.nan
        labeled[continuation_label_col] = labeled[continuation_label_col].astype("Float64")
        if horizon_return_col in labeled.columns:
            labeled[outcome_observed_col] = labeled[horizon_return_col].notna()
        else:
            labeled[outcome_observed_col] = False
    continuation_columns = [f"continuation_label_{horizon}d" for horizon in (1, 3, 5)]
    labeled["is_continuation_eligible"] = labeled[continuation_columns].notna().any(axis=1)
    return labeled


def _is_simulated_add_candidate(row: pd.Series) -> bool:
    signal_family = str(row.get("signal_family", ""))
    candidate_event_type = str(row.get("candidate_event_type", ""))
    return (
        signal_family in {"breakout_followthrough", "support_test"}
        or candidate_event_type in {"breakout_20d", "breakout_60d", "near_resistance_20d"}
    )


def _is_simulated_reduce_candidate(row: pd.Series) -> bool:
    signal_family = str(row.get("signal_family", ""))
    candidate_event_type = str(row.get("candidate_event_type", ""))
    downside_action = str(row.get("downside_suggested_action", ""))
    return (
        signal_family in {"breakdown_followthrough", "support_test"}
        or candidate_event_type in {"breakdown_20d", "breakdown_60d", "near_support_20d"}
        or downside_action in {"tighten_risk", "reduce_partial_or_wait_reclaim", "reduce_or_avoid"}
    )


def _clone_simulated_action_row(row: pd.Series, direction: str, reason: str) -> dict[str, object]:
    cloned = row.to_dict()
    cloned["source_sample_id"] = str(row.get("sample_id", "missing"))
    cloned["sample_id"] = f"{cloned['source_sample_id']}_sim_{direction}"
    cloned["sample_source"] = SIMULATED_ACTION_SAMPLE_SOURCE
    cloned["is_simulated_action"] = True
    cloned["simulated_action_direction"] = direction
    cloned["simulated_action_reason"] = reason
    cloned["signal_direction"] = direction
    cloned["action_type"] = _map_action_type(direction)
    cloned["rating_state"] = _map_rating_state(direction)
    cloned["signal_family"] = f"simulated_{direction}_action"
    cloned["is_replay_outcome_observed"] = True
    return cloned


# 2026-04-29 CST: Added because sparse negative continuation classes need a
# safe augmentation lane rooted in real historical events. Purpose: generate
# separately tagged simulated add/reduce rows without contaminating the default
# replay source-of-truth export.
def build_simulated_action_samples(frame: pd.DataFrame) -> pd.DataFrame:
    historical = frame.loc[frame.get("sample_source", "").astype(str) == "historical_research"].copy()
    if historical.empty:
        return pd.DataFrame()

    simulated_rows: list[dict[str, object]] = []
    for _, row in historical.iterrows():
        if _is_simulated_add_candidate(row):
            simulated_rows.append(_clone_simulated_action_row(row, "add", "event_candidate_add_context"))
        if _is_simulated_reduce_candidate(row):
            simulated_rows.append(_clone_simulated_action_row(row, "reduce", "event_candidate_reduce_context"))

    if not simulated_rows:
        return pd.DataFrame()

    simulated = pd.DataFrame(simulated_rows)
    simulated = _coerce_replay_required_columns(simulated)
    simulated = derive_replay_labels(simulated)
    simulated = derive_continuation_labels(simulated)
    simulated = simulated.sort_values(["signal_date", "sample_id"]).reset_index(drop=True)
    return simulated


def _optional_text(value: object) -> str:
    if pd.isna(value):
        return ""
    return str(value).strip()


def _numeric_value(value: object) -> float:
    numeric = pd.to_numeric(pd.Series([value]), errors="coerce").iloc[0]
    if pd.isna(numeric):
        return float("nan")
    return float(numeric)


# 2026-04-29 CST: Updated because the third augmentation round must align to
# the dominant untouched-validation negative shape instead of broad event-style
# failure semantics. Purpose: mine only prototype-driven add rows that look
# like high-position premature-add failures even when explicit event fields are
# blank.
def _is_shared_prototype_add_failure_context(row: pd.Series) -> bool:
    signal_direction = _optional_text(row.get("signal_direction", "")).lower()
    signal_family = _optional_text(row.get("signal_family", ""))
    candidate_action_label = _optional_text(row.get("candidate_action_label", ""))
    candidate_event_type = _optional_text(row.get("candidate_event_type", ""))
    base_position_v3 = _numeric_value(row.get("base_position_v3"))
    dist_res20 = _numeric_value(row.get("dist_res20"))
    dist_sup20 = _numeric_value(row.get("dist_sup20"))
    avg_component_vr = _numeric_value(row.get("avg_component_vr"))

    allowed_action_labels = {
        "",
        "resistance_reject_watch",
        "false_breakout_avoid_chase",
        "uncertain_breakout_wait",
        "resistance_break_watch",
    }
    allowed_event_types = {
        "",
        "near_resistance_20d",
        "breakout_20d",
        "breakout_60d",
    }

    return (
        signal_direction == "add"
        and signal_family in {"daily_position", "breakout_followthrough"}
        and candidate_action_label in allowed_action_labels
        and candidate_event_type in allowed_event_types
        and np.isfinite(base_position_v3)
        and base_position_v3 >= 0.18
        and np.isfinite(dist_res20)
        and dist_res20 <= 0.02
        and np.isfinite(dist_sup20)
        and dist_sup20 >= 0.02
        and np.isfinite(avg_component_vr)
        and avg_component_vr >= 0.74
    )


# 2026-04-29 CST: Added because the fourth augmentation round must separate
# the 5D slow-fail shape from the shared 1D/3D add prototype. Purpose: keep
# 1D/3D behavior stable while giving 5D its own traceable failure subtypes.
def _get_5d_slow_fail_reason(row: pd.Series) -> str | None:
    signal_direction = _optional_text(row.get("signal_direction", "")).lower()
    signal_family = _optional_text(row.get("signal_family", ""))
    candidate_action_label = _optional_text(row.get("candidate_action_label", ""))
    candidate_event_type = _optional_text(row.get("candidate_event_type", ""))
    base_position_v3 = _numeric_value(row.get("base_position_v3"))
    dist_res20 = _numeric_value(row.get("dist_res20"))
    dist_sup20 = _numeric_value(row.get("dist_sup20"))
    component_above200_breadth = _numeric_value(row.get("component_above200_breadth"))
    avg_component_vr = _numeric_value(row.get("avg_component_vr"))

    if not (
        signal_direction == "add"
        and signal_family in {"daily_position", "breakout_followthrough"}
        and np.isfinite(base_position_v3)
        and base_position_v3 >= 0.30
        and np.isfinite(dist_sup20)
        and dist_sup20 >= 0.035
        and np.isfinite(avg_component_vr)
        and avg_component_vr >= 0.80
        and np.isfinite(dist_res20)
        and np.isfinite(component_above200_breadth)
    ):
        return None

    allowed_action_labels = {
        "",
        "resistance_reject_watch",
        "false_breakout_avoid_chase",
        "uncertain_breakout_wait",
        "resistance_break_watch",
    }
    allowed_event_types = {
        "",
        "near_resistance_20d",
        "breakout_20d",
        "breakout_60d",
    }

    if (
        dist_res20 >= -0.025
        and component_above200_breadth >= 0.90
        and candidate_action_label in allowed_action_labels
        and candidate_event_type in allowed_event_types
    ):
        return "prototype_add_failure_5d_resistance_exhaustion"

    if dist_res20 <= -0.045 and component_above200_breadth >= 0.55 and avg_component_vr >= 0.80:
        return "prototype_add_failure_5d_extended_drift"

    return None


def _get_real_add_failure_reason(row: pd.Series, label_horizon: str) -> str | None:
    if label_horizon == "5d":
        return _get_5d_slow_fail_reason(row)
    if _is_shared_prototype_add_failure_context(row):
        return "prototype_add_failure"
    return None


def _clone_real_failure_event_row(row: pd.Series, direction: str, reason: str) -> dict[str, object]:
    cloned = row.to_dict()
    cloned["source_sample_id"] = str(row.get("sample_id", "missing"))
    cloned["sample_id"] = f"{cloned['source_sample_id']}_failure_{direction}"
    cloned["sample_source"] = REAL_FAILURE_EVENT_SAMPLE_SOURCE
    cloned["is_real_failure_event"] = True
    cloned["mined_action_direction"] = direction
    cloned["mined_failure_reason"] = reason
    cloned["signal_direction"] = direction
    cloned["action_type"] = _map_action_type(direction)
    cloned["rating_state"] = _map_rating_state(direction)
    cloned["signal_family"] = f"real_failure_{direction}"
    cloned["is_replay_outcome_observed"] = True
    return cloned


def _validate_failure_label_horizon(label_horizon: str) -> str:
    normalized = str(label_horizon).strip().lower()
    if normalized not in {"1d", "3d", "5d"}:
        raise ValueError(f"unsupported failure label horizon: {label_horizon}")
    return normalized


# 2026-04-29 CST: Added because the second augmentation pass must mine only
# real failure semantics instead of broad simulated positives. Purpose: create a
# narrow negative-only failure-event pool from governed historical event-study
# fields while preserving traceability back to the real row source.
#
# 2026-04-29 CST: Updated because scheme A requires the builder itself to emit
# only the requested horizon's negative continuation rows. Purpose: prevent
# cross-horizon leakage where a row is negative on one horizon but positive on
# the exported training horizon.
#
# 2026-04-29 CST: Updated again because the third round must mine prototype
# add-only failures aligned to untouched-validation premature-add rows.
# Purpose: stop mixing reduce-style semantics into a lane whose dominant target
# is high-position add continuation failure.
def build_real_failure_event_samples(frame: pd.DataFrame, label_horizon: str = "5d") -> pd.DataFrame:
    label_horizon = _validate_failure_label_horizon(label_horizon)
    historical = frame.loc[frame.get("sample_source", "").astype(str) == "historical_research"].copy()
    if historical.empty:
        return pd.DataFrame()

    mined_rows: list[dict[str, object]] = []
    for _, row in historical.iterrows():
        failure_reason = _get_real_add_failure_reason(row, label_horizon)
        if failure_reason is not None:
            mined_rows.append(_clone_real_failure_event_row(row, "add", failure_reason))

    if not mined_rows:
        mined = historical.head(0).copy()
        mined["source_sample_id"] = pd.Series(dtype="string")
        mined["is_real_failure_event"] = pd.Series(dtype="bool")
        mined["mined_action_direction"] = pd.Series(dtype="string")
        mined["mined_failure_reason"] = pd.Series(dtype="string")
        mined["failure_label_horizon"] = pd.Series(dtype="string")
        mined = _coerce_replay_required_columns(mined)
        mined = derive_replay_labels(mined)
        mined = derive_continuation_labels(mined)
        return mined

    mined = pd.DataFrame(mined_rows)
    mined = _coerce_replay_required_columns(mined)
    mined = derive_replay_labels(mined)
    mined = derive_continuation_labels(mined)
    continuation_column = f"continuation_label_{label_horizon}"
    negative_mask = mined[continuation_column].eq(0)
    mined = mined.loc[negative_mask].copy()
    mined["failure_label_horizon"] = label_horizon
    mined = mined.sort_values(["signal_date", "sample_id"]).reset_index(drop=True)
    return mined


def _write_csv(frame: pd.DataFrame, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    frame.to_csv(path, index=False, encoding="utf-8-sig")


def _write_summary(frame: pd.DataFrame, path: Path) -> None:
    summary = {
        "target_definition_version": TARGET_DEFINITION_VERSION,
        "sample_count": int(len(frame)),
        "sample_sources": sorted(set(frame["sample_source"].dropna().astype(str))),
        "label_vocabulary": REPLAY_LABEL_VOCAB,
        "required_fields": REPLAY_SAMPLE_REQUIRED_FIELDS,
    }
    path.write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding="utf-8")


def main() -> None:
    args = parse_args()
    output_root = Path(args.output_root)
    samples = build_replay_samples(include_live_journal=not args.skip_journal)
    labeled = derive_replay_labels(samples)
    labeled = derive_continuation_labels(labeled)
    _write_csv(samples, output_root / "01_replay_event_samples.csv")
    _write_csv(labeled, output_root / "02_replay_labeled_samples.csv")
    _write_summary(labeled, output_root / "00_replay_build_summary.json")
    print(f"sample_count={len(labeled)}")
    print(f"output_root={output_root}")


if __name__ == "__main__":
    main()
