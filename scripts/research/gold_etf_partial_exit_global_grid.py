#!/usr/bin/env python
# 2026-04-29 CST: Added because the approved route is an interpretable
# exhaustive grid search for partial D15 exits plus trailing liquidation.
# Purpose: find the best rule inside a fixed parameter space without claiming
# a market-wide true global optimum.

from __future__ import annotations

import argparse
import importlib.util
import itertools
import json
from pathlib import Path
import sys
from typing import Any

import pandas as pd


DEFAULT_OUTPUT_ROOT = Path(r"E:\SM\docs\research\gold_etf_partial_exit_global_grid_20260429")
DEFAULT_SYMBOL = "518800.SH"
INITIAL_CAPITAL = 1_000_000.0
ANCHOR_DAY = 15
REBOUND_CHECK_DAY = 5
LIVE_RULE_RETURN_DRAWDOWN_RATIO = 7.90
DEFENSIVE_DRAWDOWN_TARGET = -0.13
DEFENSIVE_RETURN_FLOOR = 1.05
_POSITION_MODULE = None


def load_position_module():
    # 2026-04-29 CST: Added because the partial-exit optimizer must reuse the
    # frozen gold signal and two-layer entry contract from prior validated work.
    global _POSITION_MODULE
    if _POSITION_MODULE is not None:
        return _POSITION_MODULE
    module_path = Path(r"E:\SM\scripts\research\gold_etf_position_param_optimization.py")
    spec = importlib.util.spec_from_file_location("gold_etf_position_param_optimization", module_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    _POSITION_MODULE = module
    return module


def build_formal_two_layer_config() -> dict[str, Any]:
    # 2026-04-29 CST: Added because this optimization is exit-only and must not
    # change the approved two-layer entry and add-position parameters.
    return {
        "first_entry_weight": 0.50,
        "allow_second_entry": True,
        "second_entry_trigger_drawdown": -0.05,
        "second_entry_requires_parent_signal": True,
        "second_entry_weight": 0.40,
        "allow_third_entry": False,
        "third_entry_trigger_drawdown": -0.04,
        "third_entry_requires_parent_signal": False,
        "third_entry_weight": 0.0,
        "max_total_weight": 1.0,
        "max_hold_days": 20,
        "rebound_check_day": REBOUND_CHECK_DAY,
    }


def build_parameter_grid() -> list[dict[str, Any]]:
    # 2026-04-29 CST: Added because方案A requires exhaustive search across a
    # human-executable parameter space rather than a black-box optimizer.
    space = {
        "partial_exit_ratio": [0.0, 0.3, 0.5, 0.7],
        "partial_exit_condition": ["always", "anchor_return_gt_0", "anchor_return_gt_0.01", "anchor_return_gt_0.02"],
        "trailing_drawdown": [0.01, 0.012, 0.015, 0.018, 0.02, 0.025, 0.03],
        "trailing_start_day": [12, 15, 18],
        "max_hold_days": [20, 30, 45, 60],
        "loss_anchor_action": ["time_exit_20d", "no_partial_trailing", "hold_to_max"],
    }
    keys = list(space.keys())
    configs: list[dict[str, Any]] = []
    seen: set[tuple[Any, ...]] = set()
    for values in itertools.product(*(space[key] for key in keys)):
        config = {key: value for key, value in zip(keys, values)}
        normalized = normalize_config(config)
        signature = tuple(normalized[key] for key in keys)
        if signature in seen:
            continue
        seen.add(signature)
        configs.append(normalized)
    return configs


def normalize_config(config: dict[str, Any]) -> dict[str, Any]:
    normalized = dict(config)
    if float(normalized["partial_exit_ratio"]) <= 0:
        normalized["partial_exit_ratio"] = 0.0
        normalized["partial_exit_condition"] = "always"
    return normalized


def config_to_name(config: dict[str, Any]) -> str:
    return (
        f"p{float(config['partial_exit_ratio']):.2f}_"
        f"c{config['partial_exit_condition']}_"
        f"dd{float(config['trailing_drawdown']):.3f}_"
        f"s{int(config['trailing_start_day'])}_"
        f"h{int(config['max_hold_days'])}_"
        f"loss_{config['loss_anchor_action']}"
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    parser.add_argument("--symbol", default=DEFAULT_SYMBOL)
    parser.add_argument("--initial-capital", type=float, default=INITIAL_CAPITAL)
    parser.add_argument("--max-configs", type=int, default=0)
    return parser.parse_args()


def anchor_condition_met(anchor_return: float, condition: str) -> bool:
    # 2026-04-29 CST: Added because D15 partial liquidation must be conditional
    # and auditable instead of an unconditional discretionary sell.
    if condition == "always":
        return True
    if condition == "anchor_return_gt_0":
        return anchor_return > 0.0
    if condition == "anchor_return_gt_0.005":
        return anchor_return > 0.005
    if condition == "anchor_return_gt_0.01":
        return anchor_return > 0.01
    if condition == "anchor_return_gt_0.015":
        return anchor_return > 0.015
    if condition == "anchor_return_gt_0.02":
        return anchor_return > 0.02
    raise ValueError(f"unknown partial exit condition: {condition}")


def resolve_partial_exit_events(
    history: pd.DataFrame,
    first_entry_idx: int,
    weighted_entry_price: float,
    total_weight: float,
    config: dict[str, Any],
    rebound_check_day: int,
    anchor_day: int,
) -> list[dict[str, Any]]:
    # 2026-04-29 CST: Added because partial exits need event-level accounting:
    # D15 partial sale and later final sale are different cash-flow events.
    config = normalize_config(config)
    rebound_check_idx = first_entry_idx + rebound_check_day - 1
    if rebound_check_idx + 1 < len(history):
        rebound_row = history.iloc[rebound_check_idx]
        if float(rebound_row["close"]) <= weighted_entry_price:
            return [
                build_exit_event(
                    history=history,
                    exit_idx=rebound_check_idx + 1,
                    exit_weight=total_weight,
                    event_type="final_exit",
                    reason=f"fail_to_rebound_d{rebound_check_day}",
                )
            ]

    anchor_idx = first_entry_idx + anchor_day - 1
    if anchor_idx + 1 >= len(history):
        return []
    anchor_return = float(history.iloc[anchor_idx]["close"]) / weighted_entry_price - 1.0

    remaining_weight = total_weight
    events: list[dict[str, Any]] = []
    if anchor_return >= 0 or config["loss_anchor_action"] == "no_partial_trailing":
        if float(config["partial_exit_ratio"]) > 0 and anchor_condition_met(anchor_return, str(config["partial_exit_condition"])):
            partial_weight = total_weight * float(config["partial_exit_ratio"])
            events.append(
                build_exit_event(
                    history=history,
                    exit_idx=anchor_idx + 1,
                    exit_weight=partial_weight,
                    event_type="partial_exit",
                    reason=f"partial_{config['partial_exit_condition']}_d{anchor_day}",
                )
            )
            remaining_weight = total_weight - partial_weight
        if remaining_weight <= 1e-12:
            return events
        final_event = resolve_trailing_final_exit(
            history=history,
            first_entry_idx=first_entry_idx,
            weighted_entry_price=weighted_entry_price,
            remaining_weight=remaining_weight,
            trailing_drawdown=float(config["trailing_drawdown"]),
            trailing_start_day=int(config["trailing_start_day"]),
            max_hold_days=int(config["max_hold_days"]),
        )
        return events + [final_event] if final_event else events

    if config["loss_anchor_action"] == "time_exit_20d":
        return [
            build_exit_event(
                history=history,
                exit_idx=first_entry_idx + 20,
                exit_weight=total_weight,
                event_type="final_exit",
                reason="anchor_loss_time_exit_20d",
            )
        ]
    if config["loss_anchor_action"] == "hold_to_max":
        return [
            build_exit_event(
                history=history,
                exit_idx=first_entry_idx + int(config["max_hold_days"]),
                exit_weight=total_weight,
                event_type="final_exit",
                reason=f"anchor_loss_max_hold_{int(config['max_hold_days'])}d",
            )
        ]
    raise ValueError(f"unknown loss_anchor_action: {config['loss_anchor_action']}")


def resolve_trailing_final_exit(
    history: pd.DataFrame,
    first_entry_idx: int,
    weighted_entry_price: float,
    remaining_weight: float,
    trailing_drawdown: float,
    trailing_start_day: int,
    max_hold_days: int,
) -> dict[str, Any] | None:
    # 2026-04-29 CST: Added because the remaining position needs a mechanical
    # high-watermark stop after the optional D15 partial sale.
    start_idx = first_entry_idx + trailing_start_day - 1
    if start_idx >= len(history) - 1:
        return None
    final_signal_idx = min(first_entry_idx + max_hold_days - 1, len(history) - 2)
    best_return = float(history.iloc[start_idx]["close"]) / weighted_entry_price - 1.0
    for close_idx in range(start_idx + 1, final_signal_idx + 1):
        close_return = float(history.iloc[close_idx]["close"]) / weighted_entry_price - 1.0
        best_return = max(best_return, close_return)
        if best_return - close_return >= trailing_drawdown:
            signal_day = close_idx - first_entry_idx + 1
            return build_exit_event(
                history=history,
                exit_idx=close_idx + 1,
                exit_weight=remaining_weight,
                event_type="final_exit",
                reason=f"trail_dd_{trailing_drawdown:.3f}_d{signal_day}",
            )
    return build_exit_event(
        history=history,
        exit_idx=first_entry_idx + max_hold_days,
        exit_weight=remaining_weight,
        event_type="final_exit",
        reason=f"max_hold_{max_hold_days}d",
    )


def build_exit_event(history: pd.DataFrame, exit_idx: int, exit_weight: float, event_type: str, reason: str) -> dict[str, Any]:
    idx = min(exit_idx, len(history) - 1)
    row = history.iloc[idx]
    return {
        "event_type": event_type,
        "exit_idx": idx,
        "exit_date": row["trade_date"],
        "exit_price": float(row["open"]),
        "exit_weight": float(exit_weight),
        "reason": reason,
    }


def compute_weighted_trade_return(events: list[dict[str, Any]], weighted_entry_price: float) -> float:
    # 2026-04-29 CST: Added because partial exits realize different weights at
    # different prices, so a single final exit price would distort the result.
    return sum(float(event["exit_weight"]) * (float(event["exit_price"]) / weighted_entry_price - 1.0) for event in events)


def run_partial_exit_backtest(
    etf_history: pd.DataFrame,
    gold_signals: pd.DataFrame,
    entry_config: dict[str, Any],
    exit_config: dict[str, Any],
    initial_capital: float,
    symbol: str,
    candidates: list[dict[str, Any]] | None = None,
) -> tuple[pd.DataFrame, pd.DataFrame]:
    # 2026-04-29 CST: Added because each parameter candidate must be replayed
    # as an account-level sequence with no overlapping positions.
    base = load_position_module()
    history = etf_history.copy().sort_values("trade_date").reset_index(drop=True)
    history["trade_date"] = pd.to_datetime(history["trade_date"])
    signal_lookup = gold_signals.copy().sort_values("trade_date").reset_index(drop=True)
    signal_dates = set(signal_lookup.loc[signal_lookup["parent_signal"] == 1, "trade_date"])
    signal_meta = signal_lookup.set_index("trade_date")[["ret_5d", "close_vs_ma20", "parent_signal"]]
    entry_config = base.validate_config(entry_config)
    exit_config = normalize_config(exit_config)
    if candidates is None:
        candidates = prepare_entry_candidates(history, signal_dates, signal_meta, entry_config, symbol)

    trades: list[dict[str, Any]] = []
    next_free_idx = 0
    capital = initial_capital
    for candidate in candidates:
        if int(candidate["entry_idx"]) < next_free_idx:
            continue
        layer_entries = candidate["layer_entries"]
        total_weight = float(candidate["total_weight"])
        weighted_entry_price = float(candidate["weighted_entry_price"])
        events = resolve_partial_exit_events(
            history=history,
            first_entry_idx=int(candidate["entry_idx"]),
            weighted_entry_price=weighted_entry_price,
            total_weight=total_weight,
            config=exit_config,
            rebound_check_day=int(entry_config["rebound_check_day"]),
            anchor_day=ANCHOR_DAY,
        )
        if not events:
            continue
        final_exit_idx = max(int(event["exit_idx"]) for event in events)
        if final_exit_idx >= len(history):
            continue

        trade_return = compute_weighted_trade_return(events, weighted_entry_price)
        entry_capital = capital
        exit_capital = capital * (1.0 + trade_return)
        repair_days, repair_date = base.compute_repair_metrics(
            history,
            first_entry_idx=int(layer_entries[0]["entry_idx"]),
            exit_idx=final_exit_idx,
            weighted_entry_price=weighted_entry_price,
        )
        anchor_idx = int(candidate["entry_idx"]) + ANCHOR_DAY - 1
        anchor_return = float(history.iloc[anchor_idx]["close"]) / weighted_entry_price - 1.0 if anchor_idx < len(history) else None

        trades.append(
            {
                "symbol": symbol,
                "config_name": config_to_name(exit_config),
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
                "partial_exit_ratio": float(exit_config["partial_exit_ratio"]),
                "partial_exit_condition": exit_config["partial_exit_condition"],
                "trailing_drawdown": float(exit_config["trailing_drawdown"]),
                "trailing_start_day": int(exit_config["trailing_start_day"]),
                "max_hold_days": int(exit_config["max_hold_days"]),
                "loss_anchor_action": exit_config["loss_anchor_action"],
                "trade_return": trade_return,
                "return_at_anchor_day": anchor_return,
                "entry_capital": entry_capital,
                "exit_capital": exit_capital,
                "hold_calendar_days": int((events[-1]["exit_date"] - layer_entries[0]["entry_date"]).days),
                "hold_trading_days": int(final_exit_idx - int(layer_entries[0]["entry_idx"]) + 1),
                "repair_days": repair_days,
                "repair_date": repair_date,
                "ret_5d": candidate["ret_5d"],
                "close_vs_ma20": candidate["close_vs_ma20"],
            }
        )
        capital = exit_capital
        next_free_idx = final_exit_idx + 1

    trade_log = pd.DataFrame(trades)
    equity_curve = build_partial_equity_curve(history, trade_log, initial_capital)
    return trade_log, equity_curve


def prepare_entry_candidates(
    history: pd.DataFrame,
    signal_dates: set[pd.Timestamp],
    signal_meta: pd.DataFrame,
    entry_config: dict[str, Any],
    symbol: str,
) -> list[dict[str, Any]]:
    # 2026-04-29 CST: Added because entry layers are independent of the exit
    # grid and should be computed once per signal date, not once per parameter.
    base = load_position_module()
    candidates: list[dict[str, Any]] = []
    for signal_date in sorted(signal_dates):
        signal_rows = history.index[history["trade_date"] == signal_date].tolist()
        if not signal_rows:
            continue
        signal_idx = signal_rows[0]
        entry_idx = signal_idx + 1
        if entry_idx >= len(history):
            continue
        first_row = history.iloc[entry_idx]
        layer_entries: list[dict[str, Any]] = [
            {
                "entry_idx": entry_idx,
                "entry_date": first_row["trade_date"],
                "entry_price": float(first_row["open"]),
                "weight": float(entry_config["first_entry_weight"]),
            }
        ]
        if bool(entry_config["allow_second_entry"]):
            second_layer = base.maybe_add_layer(
                history=history,
                signal_dates=signal_dates,
                start_idx=entry_idx + 1,
                reference_price=float(first_row["open"]),
                trigger_drawdown=float(entry_config["second_entry_trigger_drawdown"]),
                requires_parent_signal=bool(entry_config["second_entry_requires_parent_signal"]),
                weight=float(entry_config["second_entry_weight"]),
            )
            if second_layer is not None:
                layer_entries.append(second_layer)
        total_weight = sum(float(layer["weight"]) for layer in layer_entries)
        weighted_entry_price = sum(float(layer["entry_price"]) * float(layer["weight"]) for layer in layer_entries) / total_weight
        candidates.append(
            {
                "symbol": symbol,
                "signal_date": signal_date,
                "entry_idx": entry_idx,
                "layer_entries": layer_entries,
                "total_weight": total_weight,
                "weighted_entry_price": weighted_entry_price,
                "ret_5d": float(signal_meta.loc[signal_date, "ret_5d"]) if signal_date in signal_meta.index else None,
                "close_vs_ma20": float(signal_meta.loc[signal_date, "close_vs_ma20"]) if signal_date in signal_meta.index else None,
            }
        )
    return candidates


def build_partial_equity_curve(history: pd.DataFrame, trade_log: pd.DataFrame, initial_capital: float) -> pd.DataFrame:
    # 2026-04-29 CST: Added because partial exits change remaining invested
    # weight before the final exit, which affects drawdown and must be modeled.
    rows: list[dict[str, Any]] = []
    cash = initial_capital
    peak = initial_capital
    trades = []
    if not trade_log.empty:
        ordered = trade_log.sort_values("entry_date").reset_index(drop=True)
        for trade in ordered.to_dict("records"):
            trades.append(
                {
                    **trade,
                    "entry_date": pd.Timestamp(trade["entry_date"]),
                    "final_exit_date": pd.Timestamp(trade["final_exit_date"]),
                    "event_dates": [pd.Timestamp(value) for value in str(trade["event_dates"]).split("|")],
                    "event_weights": [float(value) for value in str(trade["event_weights"]).split("|")],
                    "event_prices": [float(value) for value in str(trade["event_prices"]).split("|")],
                }
            )
    trade_ptr = 0
    active_trade = None
    for row in history.itertuples(index=False):
        row_date = pd.Timestamp(row.trade_date)
        if active_trade is None and trade_ptr < len(trades) and trades[trade_ptr]["entry_date"] <= row_date:
            active_trade = trades[trade_ptr]
        equity = cash
        position_flag = 0
        if active_trade is not None and active_trade["entry_date"] <= row_date <= active_trade["final_exit_date"]:
            entry_capital = float(active_trade["entry_capital"])
            total_weight = float(active_trade["total_weight"])
            weighted_entry_price = float(active_trade["weighted_entry_price"])
            realized_cash = entry_capital * (1.0 - total_weight)
            remaining_weight = total_weight
            for event_date, event_weight, event_price in zip(
                active_trade["event_dates"],
                active_trade["event_weights"],
                active_trade["event_prices"],
            ):
                if event_date < row_date:
                    realized_cash += entry_capital * event_weight * (event_price / weighted_entry_price)
                    remaining_weight -= event_weight
            units = entry_capital * remaining_weight / weighted_entry_price if weighted_entry_price > 0 else 0.0
            equity = realized_cash + units * float(row.close)
            position_flag = 1 if remaining_weight > 1e-12 else 0
            if row_date == active_trade["final_exit_date"]:
                equity = float(active_trade["exit_capital"])
                cash = equity
                position_flag = 0
                trade_ptr += 1
                active_trade = None
        peak = max(peak, equity)
        rows.append({"date": row.trade_date, "equity": equity, "drawdown": equity / peak - 1.0 if peak > 0 else 0.0, "position_flag": position_flag})
    return pd.DataFrame(rows)


def summarize_backtest(trade_log: pd.DataFrame, equity_curve: pd.DataFrame, config: dict[str, Any], initial_capital: float) -> dict[str, Any]:
    # 2026-04-29 CST: Added because every candidate needs the same acceptance
    # metrics before we can decide whether it is a live-rule replacement.
    config_name = config_to_name(config)
    if trade_log.empty:
        return {
            "config_name": config_name,
            "sample_count": 0,
            "terminal_capital": initial_capital,
            "total_return": 0.0,
            "max_drawdown": 0.0,
            "return_drawdown_ratio": None,
        }
    terminal_capital = float(trade_log["exit_capital"].iloc[-1])
    total_return = terminal_capital / initial_capital - 1.0
    max_drawdown = float(equity_curve["drawdown"].min()) if not equity_curve.empty else 0.0
    return {
        "config_name": config_name,
        "sample_count": int(len(trade_log)),
        "terminal_capital": terminal_capital,
        "total_return": total_return,
        "max_drawdown": max_drawdown,
        "return_drawdown_ratio": total_return / abs(max_drawdown) if max_drawdown != 0 else None,
        "win_rate": float((trade_log["trade_return"] > 0).mean()),
        "avg_trade_return": float(trade_log["trade_return"].mean()),
        "median_trade_return": float(trade_log["trade_return"].median()),
        "avg_hold_trading_days": float(trade_log["hold_trading_days"].mean()),
        "median_hold_trading_days": float(trade_log["hold_trading_days"].median()),
        "partial_exit_ratio": float(config["partial_exit_ratio"]),
        "partial_exit_condition": config["partial_exit_condition"],
        "trailing_drawdown": float(config["trailing_drawdown"]),
        "trailing_start_day": int(config["trailing_start_day"]),
        "max_hold_days": int(config["max_hold_days"]),
        "loss_anchor_action": config["loss_anchor_action"],
    }


def rank_parameter_results(summary: pd.DataFrame) -> pd.DataFrame:
    # 2026-04-29 CST: Added because promotion requires explicit acceptance
    # columns, not only a sorted total-return list.
    ranked = summary.copy()
    if "return_drawdown_ratio" not in ranked.columns:
        ranked["return_drawdown_ratio"] = ranked.apply(
            lambda row: row["total_return"] / abs(row["max_drawdown"]) if row["max_drawdown"] not in [0, None] else None,
            axis=1,
        )
    else:
        ranked["return_drawdown_ratio"] = ranked["return_drawdown_ratio"].fillna(
            ranked.apply(
                lambda row: row["total_return"] / abs(row["max_drawdown"]) if row["max_drawdown"] not in [0, None] else None,
                axis=1,
            )
        )
    ranked["beats_live_return_drawdown_ratio"] = ranked["return_drawdown_ratio"] > LIVE_RULE_RETURN_DRAWDOWN_RATIO
    ranked["defensive_candidate"] = (ranked["total_return"] >= DEFENSIVE_RETURN_FLOOR) & (ranked["max_drawdown"] >= DEFENSIVE_DRAWDOWN_TARGET)
    return ranked.sort_values(["return_drawdown_ratio", "total_return"], ascending=[False, False]).reset_index(drop=True)


def run_grid(initial_capital: float, symbol: str, max_configs: int = 0) -> tuple[pd.DataFrame, pd.DataFrame, pd.DataFrame, pd.DataFrame]:
    # 2026-04-29 CST: Added because this is the approved exhaustive search
    # runner for the bounded parameter grid.
    base = load_position_module()
    etf_history = base.load_etf_history(base.DEFAULT_MAPPING_ROOT, symbol)
    gold_signals = base.load_gold_signals(base.DEFAULT_GOLD_ROOT)
    entry_config = build_formal_two_layer_config()
    history = etf_history.copy().sort_values("trade_date").reset_index(drop=True)
    history["trade_date"] = pd.to_datetime(history["trade_date"])
    signal_lookup = gold_signals.copy().sort_values("trade_date").reset_index(drop=True)
    signal_dates = set(signal_lookup.loc[signal_lookup["parent_signal"] == 1, "trade_date"])
    signal_meta = signal_lookup.set_index("trade_date")[["ret_5d", "close_vs_ma20", "parent_signal"]]
    candidates = prepare_entry_candidates(history, signal_dates, signal_meta, base.validate_config(entry_config), symbol)
    configs = build_parameter_grid()
    if max_configs > 0:
        configs = configs[:max_configs]

    rows: list[dict[str, Any]] = []
    best_trade_log = pd.DataFrame()
    best_equity_curve = pd.DataFrame()
    for idx, config in enumerate(configs, start=1):
        trade_log, equity_curve = run_partial_exit_backtest(
            etf_history=etf_history,
            gold_signals=gold_signals,
            entry_config=entry_config,
            exit_config=config,
            initial_capital=initial_capital,
            symbol=symbol,
            candidates=candidates,
        )
        rows.append(summarize_backtest(trade_log, equity_curve, config, initial_capital))
    summary_df = pd.DataFrame(rows)
    ranked = rank_parameter_results(summary_df)
    if not ranked.empty:
        best_config_name = ranked.iloc[0]["config_name"]
        best_config = next(config for config in configs if config_to_name(config) == best_config_name)
        best_trade_log, best_equity_curve = run_partial_exit_backtest(
            etf_history=etf_history,
            gold_signals=gold_signals,
            entry_config=entry_config,
            exit_config=best_config,
            initial_capital=initial_capital,
            symbol=symbol,
            candidates=candidates,
        )
    return summary_df, ranked, best_trade_log, best_equity_curve


def write_outputs(output_root: Path, summary_df: pd.DataFrame, ranked: pd.DataFrame, best_trade_log: pd.DataFrame, best_equity_curve: pd.DataFrame) -> None:
    # 2026-04-29 CST: Added because the optimizer output must be traceable and
    # reusable for later rulebook decisions.
    output_root.mkdir(parents=True, exist_ok=True)
    summary_df.to_csv(output_root / "partial_exit_grid_summary.csv", index=False)
    ranked.to_csv(output_root / "partial_exit_grid_ranked.csv", index=False)
    ranked.head(25).to_csv(output_root / "partial_exit_grid_top25.csv", index=False)
    best_trade_log.to_csv(output_root / "best_partial_exit_trade_log.csv", index=False)
    best_equity_curve.to_csv(output_root / "best_partial_exit_equity_curve.csv", index=False)
    payload = {
        "study": "gold_etf_partial_exit_global_grid",
        "rule_count": int(len(summary_df)),
        "best_config_name": ranked.iloc[0]["config_name"] if not ranked.empty else None,
        "best_total_return": float(ranked.iloc[0]["total_return"]) if not ranked.empty else None,
        "best_max_drawdown": float(ranked.iloc[0]["max_drawdown"]) if not ranked.empty else None,
        "best_return_drawdown_ratio": float(ranked.iloc[0]["return_drawdown_ratio"]) if not ranked.empty else None,
        "beats_live_return_drawdown_ratio": bool(ranked.iloc[0]["beats_live_return_drawdown_ratio"]) if not ranked.empty else False,
        "defensive_candidate": bool(ranked.iloc[0]["defensive_candidate"]) if not ranked.empty else False,
        "acceptance_live_rule_ratio": LIVE_RULE_RETURN_DRAWDOWN_RATIO,
    }
    (output_root / "summary.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")


def main() -> int:
    args = parse_args()
    summary_df, ranked, best_trade_log, best_equity_curve = run_grid(
        initial_capital=float(args.initial_capital),
        symbol=args.symbol,
        max_configs=int(args.max_configs),
    )
    write_outputs(Path(args.output_root), summary_df, ranked, best_trade_log, best_equity_curve)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
