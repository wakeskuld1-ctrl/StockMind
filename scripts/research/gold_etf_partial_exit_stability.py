#!/usr/bin/env python
# 2026-04-29 CST: Added because the best partial-exit grid result must be
# decomposed before any promotion to a live rule.
# Purpose: run a narrow stability check around the top parameter cluster and
# split the best candidate's trades by year.

from __future__ import annotations

import argparse
import importlib.util
import itertools
import json
from pathlib import Path
import sys
from typing import Any

import pandas as pd


DEFAULT_OUTPUT_ROOT = Path(r"E:\SM\docs\research\gold_etf_partial_exit_stability_20260429")
DEFAULT_SYMBOL = "518800.SH"
INITIAL_CAPITAL = 1_000_000.0
LIVE_RULE_RETURN_DRAWDOWN_RATIO = 7.90


def load_grid_module():
    # 2026-04-29 CST: Added because the stability pass must reuse the exact
    # partial-exit event accounting from the approved grid optimizer.
    module_path = Path(r"E:\SM\scripts\research\gold_etf_partial_exit_global_grid.py")
    spec = importlib.util.spec_from_file_location("gold_etf_partial_exit_global_grid", module_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def build_narrow_grid() -> list[dict[str, Any]]:
    # 2026-04-29 CST: Added because the current task is to test whether the top
    # line is a robust neighborhood instead of one isolated best-fit point.
    space = {
        "partial_exit_ratio": [0.5, 0.6, 0.7],
        "partial_exit_condition": ["anchor_return_gt_0.005", "anchor_return_gt_0.01", "anchor_return_gt_0.015"],
        "trailing_drawdown": [0.008, 0.01, 0.012, 0.015],
        "trailing_start_day": [15, 18],
        "max_hold_days": [45, 60],
        "loss_anchor_action": ["hold_to_max"],
    }
    keys = list(space.keys())
    return [{key: value for key, value in zip(keys, values)} for values in itertools.product(*(space[key] for key in keys))]


def config_to_name(config: dict[str, Any]) -> str:
    grid = load_grid_module()
    return grid.config_to_name(config)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    parser.add_argument("--symbol", default=DEFAULT_SYMBOL)
    parser.add_argument("--initial-capital", type=float, default=INITIAL_CAPITAL)
    return parser.parse_args()


def build_yearly_trade_summary(trade_log: pd.DataFrame) -> pd.DataFrame:
    # 2026-04-29 CST: Added because yearly contribution concentration is the
    # main overfit risk for this candidate line.
    if trade_log.empty:
        return pd.DataFrame()
    frame = trade_log.copy()
    frame["entry_date"] = pd.to_datetime(frame["entry_date"])
    frame["year"] = frame["entry_date"].dt.year
    rows: list[dict[str, Any]] = []
    for year, group in frame.groupby("year", sort=True):
        compounded = float((1.0 + pd.to_numeric(group["trade_return"], errors="coerce")).prod() - 1.0)
        rows.append(
            {
                "year": int(year),
                "sample_count": int(len(group)),
                "compounded_trade_return": compounded,
                "win_rate": float((group["trade_return"] > 0).mean()),
                "avg_trade_return": float(group["trade_return"].mean()),
                "median_trade_return": float(group["trade_return"].median()),
                "avg_hold_trading_days": float(group["hold_trading_days"].mean()),
                "max_single_trade_return": float(group["trade_return"].max()),
                "min_single_trade_return": float(group["trade_return"].min()),
            }
        )
    return pd.DataFrame(rows)


def build_narrow_grid_diagnostics(ranked: pd.DataFrame) -> dict[str, Any]:
    # 2026-04-29 CST: Added because promotion should depend on cluster strength,
    # not just the single best row.
    top = ranked.head(10)
    best = ranked.iloc[0] if not ranked.empty else None
    return {
        "config_count": int(len(ranked)),
        "beats_live_count": int(ranked["beats_live_return_drawdown_ratio"].sum()) if "beats_live_return_drawdown_ratio" in ranked else 0,
        "beats_live_share": float(ranked["beats_live_return_drawdown_ratio"].mean()) if "beats_live_return_drawdown_ratio" in ranked and len(ranked) else 0.0,
        "top10_median_return_drawdown_ratio": float(top["return_drawdown_ratio"].median()) if not top.empty else None,
        "top10_min_return_drawdown_ratio": float(top["return_drawdown_ratio"].min()) if not top.empty else None,
        "top10_max_return_drawdown_ratio": float(top["return_drawdown_ratio"].max()) if not top.empty else None,
        "best_config_name": best["config_name"] if best is not None else None,
        "best_total_return": float(best["total_return"]) if best is not None and "total_return" in ranked.columns else None,
        "best_max_drawdown": float(best["max_drawdown"]) if best is not None and "max_drawdown" in ranked.columns else None,
        "best_return_drawdown_ratio": float(best["return_drawdown_ratio"]) if best is not None and "return_drawdown_ratio" in ranked.columns else None,
    }


def run_narrow_grid(initial_capital: float, symbol: str) -> tuple[pd.DataFrame, pd.DataFrame, pd.DataFrame, pd.DataFrame]:
    # 2026-04-29 CST: Added because the narrow grid should be fast enough to
    # inspect the top region with the same mechanics as the broader optimizer.
    grid = load_grid_module()
    base = grid.load_position_module()
    etf_history = base.load_etf_history(base.DEFAULT_MAPPING_ROOT, symbol)
    gold_signals = base.load_gold_signals(base.DEFAULT_GOLD_ROOT)
    entry_config = grid.build_formal_two_layer_config()
    history = etf_history.copy().sort_values("trade_date").reset_index(drop=True)
    history["trade_date"] = pd.to_datetime(history["trade_date"])
    signal_lookup = gold_signals.copy().sort_values("trade_date").reset_index(drop=True)
    signal_dates = set(signal_lookup.loc[signal_lookup["parent_signal"] == 1, "trade_date"])
    signal_meta = signal_lookup.set_index("trade_date")[["ret_5d", "close_vs_ma20", "parent_signal"]]
    candidates = grid.prepare_entry_candidates(history, signal_dates, signal_meta, base.validate_config(entry_config), symbol)
    configs = build_narrow_grid()
    rows: list[dict[str, Any]] = []
    logs: dict[str, pd.DataFrame] = {}
    curves: dict[str, pd.DataFrame] = {}
    for config in configs:
        trade_log, equity_curve = grid.run_partial_exit_backtest(
            etf_history=etf_history,
            gold_signals=gold_signals,
            entry_config=entry_config,
            exit_config=config,
            initial_capital=initial_capital,
            symbol=symbol,
            candidates=candidates,
        )
        rows.append(grid.summarize_backtest(trade_log, equity_curve, config, initial_capital))
        logs[grid.config_to_name(config)] = trade_log
        curves[grid.config_to_name(config)] = equity_curve
    summary_df = pd.DataFrame(rows)
    ranked = grid.rank_parameter_results(summary_df)
    best_name = ranked.iloc[0]["config_name"] if not ranked.empty else ""
    return summary_df, ranked, logs.get(best_name, pd.DataFrame()), curves.get(best_name, pd.DataFrame())


def write_outputs(
    output_root: Path,
    summary_df: pd.DataFrame,
    ranked: pd.DataFrame,
    yearly: pd.DataFrame,
    best_trade_log: pd.DataFrame,
    best_equity_curve: pd.DataFrame,
) -> None:
    # 2026-04-29 CST: Added because the line decomposition needs stable
    # artifacts for rulebook review and future handoff.
    output_root.mkdir(parents=True, exist_ok=True)
    summary_df.to_csv(output_root / "narrow_grid_summary.csv", index=False)
    ranked.to_csv(output_root / "narrow_grid_ranked.csv", index=False)
    ranked.head(25).to_csv(output_root / "narrow_grid_top25.csv", index=False)
    yearly.to_csv(output_root / "best_yearly_trade_summary.csv", index=False)
    best_trade_log.to_csv(output_root / "best_stability_trade_log.csv", index=False)
    best_equity_curve.to_csv(output_root / "best_stability_equity_curve.csv", index=False)
    payload = {
        "study": "gold_etf_partial_exit_stability",
        "live_rule_return_drawdown_ratio": LIVE_RULE_RETURN_DRAWDOWN_RATIO,
        **build_narrow_grid_diagnostics(ranked),
    }
    (output_root / "summary.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")


def main() -> int:
    args = parse_args()
    summary_df, ranked, best_trade_log, best_equity_curve = run_narrow_grid(
        initial_capital=float(args.initial_capital),
        symbol=args.symbol,
    )
    yearly = build_yearly_trade_summary(best_trade_log)
    write_outputs(Path(args.output_root), summary_df, ranked, yearly, best_trade_log, best_equity_curve)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
