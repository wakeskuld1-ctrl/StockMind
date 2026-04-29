#!/usr/bin/env python
# 2026-04-29 CST: Added because the previous candidate rule had an execution
# gap when D15 missed the threshold but D16-D20 later repaired above it.
# Purpose: optimize the D15-unmet branch with rolling threshold checks and
# capital-efficiency metrics.

from __future__ import annotations

import argparse
import importlib.util
import itertools
import json
from pathlib import Path
import sys
from typing import Any

import pandas as pd


DEFAULT_OUTPUT_ROOT = Path(r"E:\SM\docs\research\gold_etf_partial_exit_unmet_branch_20260429")
DEFAULT_SYMBOL = "518800.SH"
INITIAL_CAPITAL = 1_000_000.0
ANCHOR_DAY = 15
REBOUND_CHECK_DAY = 5
LIVE_RULE_RETURN_DRAWDOWN_RATIO = 7.90


def load_grid_module():
    # 2026-04-29 CST: Added because this branch study must reuse the same
    # partial-exit accounting and equity-curve mechanics as the prior optimizer.
    module_path = Path(r"E:\SM\scripts\research\gold_etf_partial_exit_global_grid.py")
    spec = importlib.util.spec_from_file_location("gold_etf_partial_exit_global_grid", module_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def build_branch_grid() -> list[dict[str, Any]]:
    # 2026-04-29 CST: Added because the user challenged the D15-unmet branch,
    # so this grid explicitly varies rolling hit windows and unmet exit days.
    space = {
        "partial_exit_ratio": [0.5, 0.6, 0.7],
        "anchor_return_threshold": [0.005, 0.01, 0.015],
        "anchor_window_end_day": [15, 18, 20],
        "trailing_drawdown": [0.008, 0.01, 0.012, 0.015],
        "trailing_start_offset": [0, 2],
        "unmet_exit_day": [20, 30, 45, 60],
    }
    keys = list(space.keys())
    return [{key: value for key, value in zip(keys, values)} for values in itertools.product(*(space[key] for key in keys))]


def config_to_name(config: dict[str, Any]) -> str:
    return (
        f"p{float(config['partial_exit_ratio']):.2f}_"
        f"thr{float(config['anchor_return_threshold']):.3f}_"
        f"win{int(config['anchor_window_end_day'])}_"
        f"dd{float(config['trailing_drawdown']):.3f}_"
        f"off{int(config['trailing_start_offset'])}_"
        f"unmet{int(config['unmet_exit_day'])}"
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    parser.add_argument("--symbol", default=DEFAULT_SYMBOL)
    parser.add_argument("--initial-capital", type=float, default=INITIAL_CAPITAL)
    return parser.parse_args()


def resolve_rolling_anchor_events(
    history: pd.DataFrame,
    first_entry_idx: int,
    weighted_entry_price: float,
    total_weight: float,
    config: dict[str, Any],
    rebound_check_day: int,
    anchor_day: int,
) -> list[dict[str, Any]]:
    # 2026-04-29 CST: Added because D15-unmet does not imply no later repair;
    # D15-D20 rolling hits must be executable and measured.
    grid = load_grid_module()
    rebound_check_idx = first_entry_idx + rebound_check_day - 1
    if rebound_check_idx + 1 < len(history):
        rebound_row = history.iloc[rebound_check_idx]
        if float(rebound_row["close"]) <= weighted_entry_price:
            return [
                grid.build_exit_event(
                    history=history,
                    exit_idx=rebound_check_idx + 1,
                    exit_weight=total_weight,
                    event_type="final_exit",
                    reason=f"fail_to_rebound_d{rebound_check_day}",
                )
            ]

    hit_day = None
    threshold = float(config["anchor_return_threshold"])
    end_day = int(config["anchor_window_end_day"])
    for day in range(anchor_day, end_day + 1):
        close_idx = first_entry_idx + day - 1
        if close_idx >= len(history) - 1:
            break
        close_return = float(history.iloc[close_idx]["close"]) / weighted_entry_price - 1.0
        if close_return > threshold:
            hit_day = day
            break

    if hit_day is None:
        return [
            grid.build_exit_event(
                history=history,
                exit_idx=first_entry_idx + int(config["unmet_exit_day"]),
                exit_weight=total_weight,
                event_type="final_exit",
                reason=f"anchor_unmet_exit_d{int(config['unmet_exit_day'])}",
            )
        ]

    partial_exit_idx = first_entry_idx + hit_day
    partial_weight = total_weight * float(config["partial_exit_ratio"])
    remaining_weight = total_weight - partial_weight
    events = [
        grid.build_exit_event(
            history=history,
            exit_idx=partial_exit_idx,
            exit_weight=partial_weight,
            event_type="partial_exit",
            reason=f"partial_anchor_hit_d{hit_day}",
        )
    ]
    if remaining_weight <= 1e-12:
        return events

    trailing_start_day = hit_day + int(config["trailing_start_offset"])
    final_event = grid.resolve_trailing_final_exit(
        history=history,
        first_entry_idx=first_entry_idx,
        weighted_entry_price=weighted_entry_price,
        remaining_weight=remaining_weight,
        trailing_drawdown=float(config["trailing_drawdown"]),
        trailing_start_day=trailing_start_day,
        max_hold_days=int(config["unmet_exit_day"]),
    )
    return events + ([final_event] if final_event else [])


def run_branch_backtest(
    etf_history: pd.DataFrame,
    gold_signals: pd.DataFrame,
    entry_config: dict[str, Any],
    branch_config: dict[str, Any],
    initial_capital: float,
    symbol: str,
    candidates: list[dict[str, Any]] | None = None,
) -> tuple[pd.DataFrame, pd.DataFrame]:
    # 2026-04-29 CST: Added because branch variants must be replayed at
    # account level with the same no-overlap semantics as earlier studies.
    grid = load_grid_module()
    base = grid.load_position_module()
    history = etf_history.copy().sort_values("trade_date").reset_index(drop=True)
    history["trade_date"] = pd.to_datetime(history["trade_date"])
    signal_lookup = gold_signals.copy().sort_values("trade_date").reset_index(drop=True)
    signal_dates = set(signal_lookup.loc[signal_lookup["parent_signal"] == 1, "trade_date"])
    signal_meta = signal_lookup.set_index("trade_date")[["ret_5d", "close_vs_ma20", "parent_signal"]]
    entry_config = base.validate_config(entry_config)
    if candidates is None:
        candidates = grid.prepare_entry_candidates(history, signal_dates, signal_meta, entry_config, symbol)

    trades: list[dict[str, Any]] = []
    next_free_idx = 0
    capital = initial_capital
    for candidate in candidates:
        if int(candidate["entry_idx"]) < next_free_idx:
            continue
        layer_entries = candidate["layer_entries"]
        total_weight = float(candidate["total_weight"])
        weighted_entry_price = float(candidate["weighted_entry_price"])
        events = resolve_rolling_anchor_events(
            history=history,
            first_entry_idx=int(candidate["entry_idx"]),
            weighted_entry_price=weighted_entry_price,
            total_weight=total_weight,
            config=branch_config,
            rebound_check_day=int(entry_config["rebound_check_day"]),
            anchor_day=ANCHOR_DAY,
        )
        if not events:
            continue
        final_exit_idx = max(int(event["exit_idx"]) for event in events)
        if final_exit_idx >= len(history):
            continue
        trade_return = grid.compute_weighted_trade_return(events, weighted_entry_price)
        entry_capital = capital
        exit_capital = capital * (1.0 + trade_return)
        trades.append(
            {
                "symbol": symbol,
                "config_name": config_to_name(branch_config),
                "signal_date": candidate["signal_date"],
                "entry_date": layer_entries[0]["entry_date"],
                "final_exit_date": events[-1]["exit_date"],
                "entry_layers": len(layer_entries),
                "layer_entry_dates": "|".join(layer["entry_date"].date().isoformat() for layer in layer_entries),
                "layer_entry_prices": "|".join(f"{float(layer['entry_price']):.4f}" for layer in layer_entries),
                "layer_weights": "|".join(f"{float(layer['weight']):.2f}" for layer in layer_entries),
                "total_weight": total_weight,
                "weighted_entry_price": weighted_entry_price,
                "event_dates": "|".join(pd.to_datetime(event["exit_date"]).date().isoformat() for event in events),
                "event_prices": "|".join(f"{float(event['exit_price']):.4f}" for event in events),
                "event_weights": "|".join(f"{float(event['exit_weight']):.4f}" for event in events),
                "event_reasons": "|".join(str(event["reason"]) for event in events),
                "trade_return": trade_return,
                "entry_capital": entry_capital,
                "exit_capital": exit_capital,
                "hold_calendar_days": int((events[-1]["exit_date"] - layer_entries[0]["entry_date"]).days),
                "hold_trading_days": int(final_exit_idx - int(candidate["entry_idx"]) + 1),
                "ret_5d": candidate["ret_5d"],
                "close_vs_ma20": candidate["close_vs_ma20"],
            }
        )
        capital = exit_capital
        next_free_idx = final_exit_idx + 1
    trade_log = pd.DataFrame(trades)
    equity_curve = grid.build_partial_equity_curve(history, trade_log, initial_capital)
    return trade_log, equity_curve


def summarize_efficiency(trade_log: pd.DataFrame, equity_curve: pd.DataFrame, config_name: str, initial_capital: float) -> dict[str, Any]:
    # 2026-04-29 CST: Added because the user explicitly challenged D60 on IRR
    # and capital efficiency, not just cumulative return.
    if trade_log.empty:
        return {
            "config_name": config_name,
            "sample_count": 0,
            "terminal_capital": initial_capital,
            "total_return": 0.0,
            "max_drawdown": 0.0,
            "return_drawdown_ratio": None,
            "cagr": None,
            "return_per_hold_day": None,
        }
    terminal_capital = float(trade_log["exit_capital"].iloc[-1])
    total_return = terminal_capital / initial_capital - 1.0
    max_drawdown = float(equity_curve["drawdown"].min()) if not equity_curve.empty else 0.0
    years = max((pd.to_datetime(equity_curve["date"]).max() - pd.to_datetime(equity_curve["date"]).min()).days / 365.25, 1e-9)
    cagr = (terminal_capital / initial_capital) ** (1.0 / years) - 1.0
    total_hold_days = float(pd.to_numeric(trade_log["hold_trading_days"], errors="coerce").sum())
    return {
        "config_name": config_name,
        "sample_count": int(len(trade_log)),
        "terminal_capital": terminal_capital,
        "total_return": total_return,
        "max_drawdown": max_drawdown,
        "return_drawdown_ratio": total_return / abs(max_drawdown) if max_drawdown != 0 else None,
        "cagr": cagr,
        "win_rate": float((trade_log["trade_return"] > 0).mean()),
        "avg_trade_return": float(trade_log["trade_return"].mean()),
        "median_trade_return": float(trade_log["trade_return"].median()),
        "avg_hold_trading_days": float(trade_log["hold_trading_days"].mean()),
        "median_hold_trading_days": float(trade_log["hold_trading_days"].median()),
        "total_hold_trading_days": total_hold_days,
        "return_per_hold_day": float(pd.to_numeric(trade_log["trade_return"], errors="coerce").sum() / total_hold_days) if total_hold_days > 0 else None,
    }


def rank_branch_results(summary: pd.DataFrame) -> pd.DataFrame:
    # 2026-04-29 CST: Added because branch selection should rank by risk-adjusted
    # and time-efficiency metrics together.
    ranked = summary.copy()
    ranked["beats_live_return_drawdown_ratio"] = ranked["return_drawdown_ratio"] > LIVE_RULE_RETURN_DRAWDOWN_RATIO
    return ranked.sort_values(["return_drawdown_ratio", "cagr", "return_per_hold_day"], ascending=[False, False, False]).reset_index(drop=True)


def run_branch_grid(initial_capital: float, symbol: str) -> tuple[pd.DataFrame, pd.DataFrame, pd.DataFrame, pd.DataFrame]:
    # 2026-04-29 CST: Added because all branch variants must share the same
    # prepared entry candidates for fair and efficient comparison.
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
    rows: list[dict[str, Any]] = []
    logs: dict[str, pd.DataFrame] = {}
    curves: dict[str, pd.DataFrame] = {}
    for config in build_branch_grid():
        trade_log, equity_curve = run_branch_backtest(
            etf_history=etf_history,
            gold_signals=gold_signals,
            entry_config=entry_config,
            branch_config=config,
            initial_capital=initial_capital,
            symbol=symbol,
            candidates=candidates,
        )
        summary = summarize_efficiency(trade_log, equity_curve, config_to_name(config), initial_capital)
        summary.update(config)
        rows.append(summary)
        logs[config_to_name(config)] = trade_log
        curves[config_to_name(config)] = equity_curve
    summary_df = pd.DataFrame(rows)
    ranked = rank_branch_results(summary_df)
    best_name = ranked.iloc[0]["config_name"] if not ranked.empty else ""
    return summary_df, ranked, logs.get(best_name, pd.DataFrame()), curves.get(best_name, pd.DataFrame())


def write_outputs(output_root: Path, summary_df: pd.DataFrame, ranked: pd.DataFrame, best_trade_log: pd.DataFrame, best_equity_curve: pd.DataFrame) -> None:
    # 2026-04-29 CST: Added because the corrected branch logic must leave
    # durable evidence before any live-rule update.
    output_root.mkdir(parents=True, exist_ok=True)
    summary_df.to_csv(output_root / "unmet_branch_summary.csv", index=False)
    ranked.to_csv(output_root / "unmet_branch_ranked.csv", index=False)
    ranked.head(25).to_csv(output_root / "unmet_branch_top25.csv", index=False)
    best_trade_log.to_csv(output_root / "best_unmet_branch_trade_log.csv", index=False)
    best_equity_curve.to_csv(output_root / "best_unmet_branch_equity_curve.csv", index=False)
    payload = {
        "study": "gold_etf_partial_exit_unmet_branch",
        "config_count": int(len(summary_df)),
        "best_config_name": ranked.iloc[0]["config_name"] if not ranked.empty else None,
        "best_total_return": float(ranked.iloc[0]["total_return"]) if not ranked.empty else None,
        "best_max_drawdown": float(ranked.iloc[0]["max_drawdown"]) if not ranked.empty else None,
        "best_return_drawdown_ratio": float(ranked.iloc[0]["return_drawdown_ratio"]) if not ranked.empty else None,
        "best_cagr": float(ranked.iloc[0]["cagr"]) if not ranked.empty else None,
        "best_return_per_hold_day": float(ranked.iloc[0]["return_per_hold_day"]) if not ranked.empty else None,
    }
    (output_root / "summary.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")


def main() -> int:
    args = parse_args()
    summary_df, ranked, best_trade_log, best_equity_curve = run_branch_grid(
        initial_capital=float(args.initial_capital),
        symbol=args.symbol,
    )
    write_outputs(Path(args.output_root), summary_df, ranked, best_trade_log, best_equity_curve)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
