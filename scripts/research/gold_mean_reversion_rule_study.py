#!/usr/bin/env python
# 2026-04-28 CST: Added because the approved next step is to turn the broad
# gold mean-reversion finding into a stable rule candidate instead of stopping
# at structure-level commentary.
# Purpose: layer oversold depth, MA deviation, macro resonance, and failure-risk
# filters so the next decision can be based on a concrete rule draft.

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import sys

import numpy as np
import pandas as pd


DEFAULT_OUTPUT_ROOT = Path(r"E:\SM\docs\research\gold_mean_reversion_rule_10y_20260428")
DEFAULT_START_DATE = "2016-01-01"
DEFAULT_END_DATE = "2026-04-28"
HOLDING_WINDOWS = [3, 5, 10, 15, 20, 30]


def load_base_module():
    base_path = Path(r"E:\SM\scripts\research\gold_structure_proxy_analysis.py")
    spec = importlib.util.spec_from_file_location("gold_structure_proxy_analysis", base_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


BASE_MODULE = load_base_module()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--start-date", default=DEFAULT_START_DATE)
    parser.add_argument("--end-date", default=DEFAULT_END_DATE)
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    return parser.parse_args()


def assign_bucket_labels(frame: pd.DataFrame) -> pd.DataFrame:
    out = frame.copy()
    if "volume_ratio_1d_vs_20d" not in out.columns:
        out["volume_ratio_1d_vs_20d"] = 1.0
    out["ret_5d_bucket"] = np.select(
        [out["ret_5d"] <= -0.05, out["ret_5d"] <= -0.025],
        ["ret5_extreme", "ret5_moderate"],
        default="ret5_shallow",
    )
    out["ma20_gap_bucket"] = np.select(
        [out["close_vs_ma20"] <= -0.04, out["close_vs_ma20"] <= -0.02],
        ["ma20_gap_deep", "ma20_gap_moderate"],
        default="ma20_gap_shallow",
    )
    out["volume_bucket"] = np.select(
        [out["volume_ratio_1d_vs_20d"] > 1.4, out["volume_ratio_1d_vs_20d"] >= 0.8],
        ["vol_hot", "vol_mild"],
        default="vol_cold",
    )
    return out


def build_failure_flags(frame: pd.DataFrame) -> pd.DataFrame:
    out = frame.copy()
    # 2026-04-28 CST: Added because a tradable mean-reversion rule needs an
    # explicit way to reject “oversold” samples that are actually unstable.
    # Purpose: separate controlled pullbacks from high-shock continuation drops.
    day_range_pct = (out["gold_high"] / out["gold_low"]) - 1.0
    out["high_volatility_shock_flag"] = (
        (day_range_pct >= 0.06) | (out["volume_ratio_1d_vs_20d"] >= 1.6)
    ).astype(int)
    out["downtrend_break_flag"] = (
        ((out["gold_close"] < out["ma20"]) | (out["gold_close"] < out["ma30"])) & (out["ma20"] < out["ma30"])
    ).astype(int)
    out["failure_risk_flag"] = (
        (out["high_volatility_shock_flag"] == 1) | (out["downtrend_break_flag"] == 1)
    ).astype(int)
    return out


def isolate_mean_reversion_samples(frame: pd.DataFrame) -> pd.DataFrame:
    return frame[frame["structure_name"] == "mean_reversion"].copy()


def summarize_rule_layers(frame: pd.DataFrame, holding_windows: list[int] | None = None) -> pd.DataFrame:
    if holding_windows is None:
        holding_windows = HOLDING_WINDOWS
    rows: list[dict[str, float | int | str]] = []
    grouped = frame.groupby(
        ["ret_5d_bucket", "ma20_gap_bucket", "resonance_regime"],
        dropna=False,
    )
    for (ret_bucket, gap_bucket, resonance_regime), subset in grouped:
        failure_risk_share = float(subset["failure_risk_flag"].mean()) if not subset.empty else np.nan
        for holding_days in holding_windows:
            summary = BASE_MODULE.summarize_holding_window(subset, holding_days)
            summary["ret_5d_bucket"] = ret_bucket
            summary["ma20_gap_bucket"] = gap_bucket
            summary["resonance_regime"] = resonance_regime
            summary["failure_risk_share"] = failure_risk_share
            rows.append(summary)
    return pd.DataFrame(rows)


def extract_candidate_rule(summary: pd.DataFrame) -> dict[str, float | int | str]:
    eligible = summary[
        (summary["sample_count"] >= 8)
        & (summary["avg_return"] > 0)
        & (summary["win_rate"] >= 0.55)
        & (summary["failure_risk_share"] <= 0.60)
    ].copy()
    if eligible.empty:
        return {}
    ranked = eligible.sort_values(
        ["failure_risk_share", "return_per_day", "win_rate", "avg_max_drawdown"],
        ascending=[True, False, False, False],
    )
    return ranked.iloc[0].to_dict()


def build_rule_diagnostics(frame: pd.DataFrame) -> pd.DataFrame:
    rows = []
    for bucket_name, subset in frame.groupby("ret_5d_bucket", dropna=False):
        rows.append(
            {
                "layer_type": "ret_5d_bucket",
                "layer_name": bucket_name,
                "sample_count": int(len(subset)),
                "avg_ret_5d": float(subset["future_ret_5d"].mean()),
                "avg_ret_10d": float(subset["future_ret_10d"].mean()),
                "avg_ret_15d": float(subset["future_ret_15d"].mean()),
                "win_rate_5d": float((subset["future_ret_5d"] > 0).mean()),
                "failure_risk_share": float(subset["failure_risk_flag"].mean()),
            }
        )
    for bucket_name, subset in frame.groupby("ma20_gap_bucket", dropna=False):
        rows.append(
            {
                "layer_type": "ma20_gap_bucket",
                "layer_name": bucket_name,
                "sample_count": int(len(subset)),
                "avg_ret_5d": float(subset["future_ret_5d"].mean()),
                "avg_ret_10d": float(subset["future_ret_10d"].mean()),
                "avg_ret_15d": float(subset["future_ret_15d"].mean()),
                "win_rate_5d": float((subset["future_ret_5d"] > 0).mean()),
                "failure_risk_share": float(subset["failure_risk_flag"].mean()),
            }
        )
    for bucket_name, subset in frame.groupby("resonance_regime", dropna=False):
        rows.append(
            {
                "layer_type": "resonance_regime",
                "layer_name": bucket_name,
                "sample_count": int(len(subset)),
                "avg_ret_5d": float(subset["future_ret_5d"].mean()),
                "avg_ret_10d": float(subset["future_ret_10d"].mean()),
                "avg_ret_15d": float(subset["future_ret_15d"].mean()),
                "win_rate_5d": float((subset["future_ret_5d"] > 0).mean()),
                "failure_risk_share": float(subset["failure_risk_flag"].mean()),
            }
        )
    return pd.DataFrame(rows)


def prepare_mean_reversion_frame(start_date: str, end_date: str) -> tuple[pd.DataFrame, dict[str, int]]:
    frame, counts = BASE_MODULE.prepare_analysis_frame(start_date, end_date)
    mean_reversion = isolate_mean_reversion_samples(frame)
    mean_reversion = assign_bucket_labels(mean_reversion)
    mean_reversion = build_failure_flags(mean_reversion)
    counts["mean_reversion_rows"] = int(len(mean_reversion))
    return mean_reversion, counts


def main() -> int:
    args = parse_args()
    frame, counts = prepare_mean_reversion_frame(args.start_date, args.end_date)
    summary = summarize_rule_layers(frame)
    diagnostics = build_rule_diagnostics(frame)
    candidate_rule = extract_candidate_rule(summary)

    output_root = Path(args.output_root)
    output_root.mkdir(parents=True, exist_ok=True)
    frame.to_csv(output_root / "gold_mean_reversion_samples.csv", index=False, encoding="utf-8-sig")
    summary.to_csv(output_root / "gold_mean_reversion_rule_layers.csv", index=False, encoding="utf-8-sig")
    diagnostics.to_csv(output_root / "gold_mean_reversion_layer_diagnostics.csv", index=False, encoding="utf-8-sig")

    payload = {
        "data_counts": counts,
        "candidate_rule": candidate_rule,
        "top_rule_layers": summary.sort_values(
            ["failure_risk_share", "return_per_day", "win_rate"],
            ascending=[True, False, False],
        ).head(10).to_dict(orient="records"),
    }
    (output_root / "summary.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(payload, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
