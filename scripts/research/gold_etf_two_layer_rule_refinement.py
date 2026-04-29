#!/usr/bin/env python
# 2026-04-29 CST: Added because the approved next step is to freeze the
# validated two-layer rule and run a narrowly bounded refinement study on it.
# Purpose: expose the formal small-range search space and run a constrained
# optimization around the validated two-layer 518800.SH position rule.

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import sys


DEFAULT_OUTPUT_ROOT = Path(r"E:\SM\docs\research\gold_etf_two_layer_refinement_20260429")
FORMAL_TWO_LAYER_BASELINE = "f0.50_s1_-0.04_1_0.35_t0_0.00_0_0.00_h20_r5"


def load_position_module():
    module_path = Path(r"E:\SM\scripts\research\gold_etf_position_param_optimization.py")
    spec = importlib.util.spec_from_file_location("gold_etf_position_param_optimization", module_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def build_small_range_space() -> dict[str, list[object]]:
    # 2026-04-29 CST: Added because the user approved a bounded B-stage search
    # only around the validated two-layer rule instead of reopening the whole surface.
    return {
        "first_entry_weight": [0.50],
        "allow_second_entry": [True],
        "second_entry_trigger_drawdown": [-0.03, -0.04, -0.05],
        "second_entry_requires_parent_signal": [False, True],
        "second_entry_weight": [0.25, 0.30, 0.35, 0.40],
        "allow_third_entry": [False],
        "third_entry_trigger_drawdown": [-0.04],
        "third_entry_requires_parent_signal": [False],
        "third_entry_weight": [0.0],
        "max_total_weight": [1.0],
        "max_hold_days": [15, 20],
        "rebound_check_day": [5, 7],
    }


def build_refinement_comparison(ranked):
    if ranked.empty:
        return ranked
    baseline = ranked[ranked["config_name"] == FORMAL_TWO_LAYER_BASELINE].head(1).copy()
    best = ranked.head(1).copy()
    if baseline.empty:
        baseline = best.copy()
        baseline["comparison_role"] = "formal_two_layer_baseline_fallback"
    else:
        baseline["comparison_role"] = "formal_two_layer_baseline"
    best["comparison_role"] = "optimized_best"
    return __import__("pandas").concat([baseline, best], ignore_index=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    base = load_position_module()
    parameter_space = build_small_range_space()
    etf_history = base.load_etf_history(base.DEFAULT_MAPPING_ROOT, base.DEFAULT_SYMBOL)
    gold_signals = base.load_gold_signals(base.DEFAULT_GOLD_ROOT)
    summary_df, ranked, best_trade_log, best_equity_curve = base.run_parameter_search(
        etf_history=etf_history,
        gold_signals=gold_signals,
        parameter_space=parameter_space,
        initial_capital=base.INITIAL_CAPITAL,
        symbol=base.DEFAULT_SYMBOL,
        max_configs=0,
    )
    comparison = build_refinement_comparison(ranked)
    output_root = Path(args.output_root)
    output_root.mkdir(parents=True, exist_ok=True)
    summary_df.to_csv(output_root / "parameter_grid_results.csv", index=False)
    ranked.to_csv(output_root / "parameter_grid_ranked.csv", index=False)
    comparison.to_csv(output_root / "baseline_vs_optimized_summary.csv", index=False)
    best_trade_log.to_csv(output_root / "optimized_trade_log.csv", index=False)
    best_equity_curve.to_csv(output_root / "optimized_equity_curve.csv", index=False)
    (output_root / "summary.json").write_text(
        json.dumps(
            {
                "search_mode": "small_range_bounded_refinement",
                "config_count": int(len(summary_df)),
                "best_config_name": ranked.iloc[0]["config_name"] if not ranked.empty else None,
                "best_total_return": float(ranked.iloc[0]["total_return"]) if not ranked.empty else None,
                "best_max_drawdown": float(ranked.iloc[0]["max_drawdown"]) if not ranked.empty else None,
                "best_median_repair_days": float(ranked.iloc[0]["median_repair_days"]) if not ranked.empty else None,
                "best_repair_within_5d": float(ranked.iloc[0]["repair_within_5d"]) if not ranked.empty else None,
            },
            ensure_ascii=False,
            indent=2,
        ),
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
