#!/usr/bin/env python
# 2026-04-29 CST: Added because the approved route is to test whether
# parameterized position management can improve the validated gold ETF baseline.
# Purpose: run a constrained parameter search on layered entries for 518800.SH
# while preserving the existing T-close / T+1-open execution contract.

from __future__ import annotations

import argparse
import itertools
import json
from pathlib import Path
from typing import Any

import pandas as pd


DEFAULT_MAPPING_ROOT = Path(r"E:\SM\docs\research\gold_etf_mapping_20260428")
DEFAULT_GOLD_ROOT = Path(r"E:\SM\docs\research\gold_structure_proxy_10y_20260428")
DEFAULT_OUTPUT_ROOT = Path(r"E:\SM\docs\research\gold_etf_position_param_opt_20260429")
DEFAULT_SYMBOL = "518800.SH"
INITIAL_CAPITAL = 1_000_000.0

DEFAULT_PARAMETER_SPACE = {
    "first_entry_weight": [0.20, 0.25, 0.30, 0.35, 0.40, 0.50],
    "allow_second_entry": [False, True],
    "second_entry_trigger_drawdown": [-0.02, -0.03, -0.04, -0.05],
    "second_entry_requires_parent_signal": [False, True],
    "second_entry_weight": [0.20, 0.25, 0.30, 0.35],
    "allow_third_entry": [False, True],
    "third_entry_trigger_drawdown": [-0.02, -0.03, -0.04, -0.05],
    "third_entry_requires_parent_signal": [False, True],
    "third_entry_weight": [0.20, 0.25, 0.30, 0.35],
    "max_total_weight": [1.00],
    "max_hold_days": [10, 15, 20, 25],
    "rebound_check_day": [3, 5, 7],
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mapping-root", default=str(DEFAULT_MAPPING_ROOT))
    parser.add_argument("--gold-root", default=str(DEFAULT_GOLD_ROOT))
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    parser.add_argument("--symbol", default=DEFAULT_SYMBOL)
    parser.add_argument("--initial-capital", type=float, default=INITIAL_CAPITAL)
    parser.add_argument("--max-configs", type=int, default=0)
    return parser.parse_args()


def validate_config(config: dict[str, Any]) -> dict[str, Any]:
    normalized = dict(config)
    if float(normalized["first_entry_weight"]) <= 0.0:
        raise ValueError("first_entry_weight must be positive")
    if int(normalized["rebound_check_day"]) >= int(normalized["max_hold_days"]):
        raise ValueError("rebound_check_day must be below max_hold_days")

    total_weight = float(normalized["first_entry_weight"])
    if bool(normalized["allow_second_entry"]):
        total_weight += float(normalized["second_entry_weight"])
    if bool(normalized["allow_third_entry"]):
        total_weight += float(normalized["third_entry_weight"])
    if total_weight > float(normalized["max_total_weight"]) + 1e-12:
        raise ValueError("entry weights exceed max_total_weight")

    if bool(normalized["allow_third_entry"]) and not bool(normalized["allow_second_entry"]):
        raise ValueError("third entry requires second entry to be enabled")
    if bool(normalized["allow_third_entry"]) and float(normalized["third_entry_trigger_drawdown"]) > float(
        normalized["second_entry_trigger_drawdown"]
    ):
        raise ValueError("third entry trigger cannot be shallower than second entry trigger")

    if not bool(normalized["allow_second_entry"]):
        normalized["allow_third_entry"] = False
        normalized["second_entry_weight"] = 0.0
        normalized["second_entry_requires_parent_signal"] = False
        normalized["second_entry_trigger_drawdown"] = 0.0
    if not bool(normalized["allow_third_entry"]):
        normalized["third_entry_weight"] = 0.0
        normalized["third_entry_requires_parent_signal"] = False
        normalized["third_entry_trigger_drawdown"] = 0.0
    return normalized


def build_parameter_grid(parameter_space: dict[str, list[Any]]) -> list[dict[str, Any]]:
    keys = list(parameter_space.keys())
    configs: list[dict[str, Any]] = []
    seen: set[tuple[Any, ...]] = set()
    for values in itertools.product(*(parameter_space[key] for key in keys)):
        candidate = {key: value for key, value in zip(keys, values)}
        try:
            normalized = validate_config(candidate)
        except ValueError:
            continue
        signature = tuple(normalized[key] for key in keys)
        if signature in seen:
            continue
        seen.add(signature)
        configs.append(normalized)
    return configs


def load_etf_history(mapping_root: Path, symbol: str) -> pd.DataFrame:
    # 2026-04-29 CST: Added because optimization must reuse the frozen ETF
    # history source instead of downloading a mutable external copy.
    frame = pd.read_csv(mapping_root / "gold_etf_mapping_histories.csv")
    frame["trade_date"] = pd.to_datetime(frame["trade_date"])
    frame = frame[frame["symbol"] == symbol].copy()
    return frame.sort_values("trade_date").reset_index(drop=True)


def load_gold_signals(gold_root: Path) -> pd.DataFrame:
    # 2026-04-29 CST: Added because optimization must use the frozen gold parent
    # signal source as the only truth for mean-reversion triggers.
    frame = pd.read_csv(gold_root / "gold_proxy_flagged_events.csv")
    frame["trade_date"] = pd.to_datetime(frame["trade_date"])
    frame["parent_signal"] = ((frame["ret_5d"] <= -0.02) & (frame["close_vs_ma20"] <= -0.015)).astype(int)
    keep_cols = ["trade_date", "ret_5d", "close_vs_ma20", "parent_signal"]
    return frame[keep_cols].drop_duplicates(subset=["trade_date"]).sort_values("trade_date").reset_index(drop=True)


def run_position_management_backtest(
    etf_history: pd.DataFrame,
    gold_signals: pd.DataFrame,
    config: dict[str, Any],
    initial_capital: float,
    symbol: str,
) -> tuple[pd.DataFrame, pd.DataFrame]:
    history = etf_history.copy().sort_values("trade_date").reset_index(drop=True)
    history["trade_date"] = pd.to_datetime(history["trade_date"])
    signal_lookup = gold_signals.copy().sort_values("trade_date").reset_index(drop=True)
    signal_dates = set(signal_lookup.loc[signal_lookup["parent_signal"] == 1, "trade_date"])
    signal_meta = signal_lookup.set_index("trade_date")[["ret_5d", "close_vs_ma20", "parent_signal"]]

    trades: list[dict[str, Any]] = []
    next_free_idx = 0
    capital = initial_capital
    config = validate_config(config)

    for signal_date in sorted(signal_dates):
        signal_rows = history.index[history["trade_date"] == signal_date].tolist()
        if not signal_rows:
            continue
        signal_idx = signal_rows[0]
        entry_idx = signal_idx + 1
        if entry_idx >= len(history) or entry_idx < next_free_idx:
            continue

        layer_entries: list[dict[str, Any]] = []
        first_row = history.iloc[entry_idx]
        layer_entries.append(
            {
                "entry_idx": entry_idx,
                "entry_date": first_row["trade_date"],
                "entry_price": float(first_row["open"]),
                "weight": float(config["first_entry_weight"]),
            }
        )

        last_reference_price = float(first_row["open"])
        next_candidate_idx = entry_idx + 1
        if bool(config["allow_second_entry"]):
            second_layer = maybe_add_layer(
                history=history,
                signal_dates=signal_dates,
                start_idx=next_candidate_idx,
                reference_price=last_reference_price,
                trigger_drawdown=float(config["second_entry_trigger_drawdown"]),
                requires_parent_signal=bool(config["second_entry_requires_parent_signal"]),
                weight=float(config["second_entry_weight"]),
            )
            if second_layer is not None:
                layer_entries.append(second_layer)
                last_reference_price = float(second_layer["entry_price"])
                next_candidate_idx = int(second_layer["entry_idx"]) + 1

        if bool(config["allow_third_entry"]):
            third_layer = maybe_add_layer(
                history=history,
                signal_dates=signal_dates,
                start_idx=next_candidate_idx,
                reference_price=last_reference_price,
                trigger_drawdown=float(config["third_entry_trigger_drawdown"]),
                requires_parent_signal=bool(config["third_entry_requires_parent_signal"]),
                weight=float(config["third_entry_weight"]),
            )
            if third_layer is not None:
                layer_entries.append(third_layer)

        exit_idx, exit_reason = resolve_position_exit(history, layer_entries, config)
        if exit_idx >= len(history):
            continue
        exit_row = history.iloc[exit_idx]
        total_weight = sum(float(layer["weight"]) for layer in layer_entries)
        if total_weight <= 0.0:
            continue

        entry_capital = capital
        weighted_entry_price = sum(float(layer["entry_price"]) * float(layer["weight"]) for layer in layer_entries) / total_weight
        exit_price = float(exit_row["open"])
        trade_return = (exit_price / weighted_entry_price - 1.0) * total_weight
        exit_capital = capital * (1.0 + trade_return)
        repair_days, repair_date = compute_repair_metrics(history, first_entry_idx=int(layer_entries[0]["entry_idx"]), exit_idx=exit_idx, weighted_entry_price=weighted_entry_price)

        trades.append(
            {
                "symbol": symbol,
                "signal_date": signal_date,
                "entry_date": layer_entries[0]["entry_date"],
                "exit_date": exit_row["trade_date"],
                "entry_layers": len(layer_entries),
                "layer_entry_dates": "|".join(layer["entry_date"].date().isoformat() for layer in layer_entries),
                "layer_entry_prices": "|".join(f"{float(layer['entry_price']):.4f}" for layer in layer_entries),
                "layer_weights": "|".join(f"{float(layer['weight']):.2f}" for layer in layer_entries),
                "total_weight": total_weight,
                "weighted_entry_price": weighted_entry_price,
                "exit_price": exit_price,
                "exit_reason": exit_reason,
                "trade_return": trade_return,
                "entry_capital": entry_capital,
                "exit_capital": exit_capital,
                "hold_calendar_days": int((exit_row["trade_date"] - layer_entries[0]["entry_date"]).days),
                "hold_trading_days": int(exit_idx - int(layer_entries[0]["entry_idx"]) + 1),
                "repair_days": repair_days,
                "repair_date": repair_date,
                "ret_5d": float(signal_meta.loc[signal_date, "ret_5d"]) if signal_date in signal_meta.index else None,
                "close_vs_ma20": float(signal_meta.loc[signal_date, "close_vs_ma20"]) if signal_date in signal_meta.index else None,
            }
        )
        capital = exit_capital
        next_free_idx = exit_idx + 1

    trade_log = pd.DataFrame(trades)
    equity_curve = build_equity_curve(history, trade_log, initial_capital)
    return trade_log, equity_curve


def maybe_add_layer(
    history: pd.DataFrame,
    signal_dates: set[pd.Timestamp],
    start_idx: int,
    reference_price: float,
    trigger_drawdown: float,
    requires_parent_signal: bool,
    weight: float,
) -> dict[str, Any] | None:
    if weight <= 0.0:
        return None
    threshold_price = reference_price * (1.0 + trigger_drawdown)
    for signal_idx in range(start_idx, len(history) - 1):
        row = history.iloc[signal_idx]
        if float(row["close"]) > threshold_price:
            continue
        if requires_parent_signal and row["trade_date"] not in signal_dates:
            continue
        entry_idx = signal_idx + 1
        entry_row = history.iloc[entry_idx]
        return {
            "entry_idx": entry_idx,
            "entry_date": entry_row["trade_date"],
            "entry_price": float(entry_row["open"]),
            "weight": float(weight),
        }
    return None


def resolve_position_exit(history: pd.DataFrame, layer_entries: list[dict[str, Any]], config: dict[str, Any]) -> tuple[int, str]:
    first_entry_idx = int(layer_entries[0]["entry_idx"])
    weighted_cost = sum(float(layer["entry_price"]) * float(layer["weight"]) for layer in layer_entries) / sum(
        float(layer["weight"]) for layer in layer_entries
    )
    rebound_check_idx = first_entry_idx + int(config["rebound_check_day"]) - 1
    if rebound_check_idx + 1 < len(history):
        rebound_row = history.iloc[rebound_check_idx]
        if float(rebound_row["close"]) <= weighted_cost:
            return rebound_check_idx + 1, f"fail_to_rebound_d{int(config['rebound_check_day'])}"

    planned_exit_idx = first_entry_idx + int(config["max_hold_days"])
    if planned_exit_idx < len(history):
        return planned_exit_idx, "time_exit"
    return len(history), "insufficient_history"


def compute_repair_metrics(history: pd.DataFrame, first_entry_idx: int, exit_idx: int, weighted_entry_price: float) -> tuple[float | None, str | None]:
    for idx in range(first_entry_idx, min(exit_idx + 1, len(history))):
        row = history.iloc[idx]
        if float(row["close"]) >= weighted_entry_price:
            return float(idx - first_entry_idx + 1), row["trade_date"].date().isoformat()
    return None, None


def build_equity_curve(history: pd.DataFrame, trade_log: pd.DataFrame, initial_capital: float) -> pd.DataFrame:
    rows: list[dict[str, Any]] = []
    peak_equity = initial_capital
    cash = initial_capital

    for row in history.itertuples(index=False):
        active = pd.DataFrame()
        if not trade_log.empty:
            active = trade_log[(trade_log["entry_date"] <= row.trade_date) & (trade_log["exit_date"] >= row.trade_date)]

        if active.empty:
            equity = cash
            position_flag = 0
        else:
            trade = active.iloc[0]
            total_weight = float(trade["total_weight"])
            weighted_cost = float(trade["weighted_entry_price"])
            invested_capital = float(trade["entry_capital"]) * total_weight
            units = invested_capital / weighted_cost if weighted_cost > 0 else 0.0
            residual_cash = float(trade["entry_capital"]) * (1.0 - total_weight)
            equity = residual_cash + units * float(row.close)
            position_flag = 1
            if row.trade_date == trade["exit_date"]:
                equity = float(trade["exit_capital"])
                cash = equity
                position_flag = 0
        peak_equity = max(peak_equity, equity)
        rows.append(
            {
                "date": row.trade_date,
                "equity": equity,
                "drawdown": equity / peak_equity - 1.0 if peak_equity > 0 else 0.0,
                "position_flag": position_flag,
            }
        )
    return pd.DataFrame(rows)


def summarize_backtest(trade_log: pd.DataFrame, equity_curve: pd.DataFrame, config_name: str, initial_capital: float) -> dict[str, Any]:
    if trade_log.empty:
        return {
            "config_name": config_name,
            "sample_count": 0,
            "terminal_capital": initial_capital,
            "total_return": 0.0,
            "max_drawdown": 0.0,
            "win_rate": None,
            "avg_trade_return": None,
            "median_trade_return": None,
            "avg_layers": None,
            "avg_hold_trading_days": None,
            "median_repair_days": None,
            "repair_within_5d": None,
            "repair_within_10d": None,
            "cagr": None,
        }

    terminal_capital = float(trade_log["exit_capital"].iloc[-1])
    total_return = terminal_capital / initial_capital - 1.0
    max_drawdown = float(equity_curve["drawdown"].min()) if not equity_curve.empty else 0.0
    years = max((equity_curve["date"].max() - equity_curve["date"].min()).days / 365.25, 1e-9)
    cagr = terminal_capital / initial_capital
    cagr = cagr ** (1.0 / years) - 1.0 if years > 0 else None
    repair_series = pd.to_numeric(trade_log.get("repair_days"), errors="coerce")
    return {
        "config_name": config_name,
        "sample_count": int(len(trade_log)),
        "terminal_capital": terminal_capital,
        "total_return": total_return,
        "max_drawdown": max_drawdown,
        "win_rate": float((trade_log["trade_return"] > 0).mean()),
        "avg_trade_return": float(trade_log["trade_return"].mean()),
        "median_trade_return": float(trade_log["trade_return"].median()),
        "avg_layers": float(trade_log["entry_layers"].mean()),
        "avg_hold_trading_days": float(trade_log["hold_trading_days"].mean()),
        "median_repair_days": float(repair_series.dropna().median()) if repair_series.notna().any() else None,
        "repair_within_5d": float((repair_series.fillna(9999) <= 5).mean()),
        "repair_within_10d": float((repair_series.fillna(9999) <= 10).mean()),
        "cagr": cagr,
    }


def config_to_name(config: dict[str, Any]) -> str:
    return (
        f"f{config['first_entry_weight']:.2f}_"
        f"s{int(config['allow_second_entry'])}_{config['second_entry_trigger_drawdown']:.2f}_{int(config['second_entry_requires_parent_signal'])}_{config['second_entry_weight']:.2f}_"
        f"t{int(config['allow_third_entry'])}_{config['third_entry_trigger_drawdown']:.2f}_{int(config['third_entry_requires_parent_signal'])}_{config['third_entry_weight']:.2f}_"
        f"h{int(config['max_hold_days'])}_r{int(config['rebound_check_day'])}"
    )


def rank_results(summary: pd.DataFrame) -> pd.DataFrame:
    ranked = summary.copy()
    ranked["return_drawdown_ratio"] = ranked.apply(
        lambda row: row["total_return"] / abs(row["max_drawdown"]) if row["max_drawdown"] not in [0, None] else None,
        axis=1,
    )
    return ranked.sort_values(["return_drawdown_ratio", "total_return"], ascending=[False, False]).reset_index(drop=True)


def run_parameter_search(
    etf_history: pd.DataFrame,
    gold_signals: pd.DataFrame,
    parameter_space: dict[str, list[Any]],
    initial_capital: float,
    symbol: str,
    max_configs: int = 0,
) -> tuple[pd.DataFrame, pd.DataFrame, pd.DataFrame, pd.DataFrame]:
    configs = build_parameter_grid(parameter_space)
    if max_configs > 0:
        configs = configs[:max_configs]

    rows: list[dict[str, Any]] = []
    best_trade_log = pd.DataFrame()
    best_equity_curve = pd.DataFrame()
    for config in configs:
        config_name = config_to_name(config)
        trade_log, equity_curve = run_position_management_backtest(
            etf_history=etf_history,
            gold_signals=gold_signals,
            config=config,
            initial_capital=initial_capital,
            symbol=symbol,
        )
        summary = summarize_backtest(trade_log, equity_curve, config_name, initial_capital)
        summary.update(config)
        rows.append(summary)
        if best_trade_log.empty or summary["total_return"] > float(rows[0]["total_return"] if rows else -1.0):
            pass

    summary_df = pd.DataFrame(rows)
    if summary_df.empty:
        return summary_df, summary_df, best_trade_log, best_equity_curve
    ranked = rank_results(summary_df)
    best_config_name = ranked.iloc[0]["config_name"]
    best_config = next(config for config in configs if config_to_name(config) == best_config_name)
    best_trade_log, best_equity_curve = run_position_management_backtest(
        etf_history=etf_history,
        gold_signals=gold_signals,
        config=best_config,
        initial_capital=initial_capital,
        symbol=symbol,
    )
    return summary_df, ranked, best_trade_log, best_equity_curve


def build_baseline_comparison(ranked: pd.DataFrame) -> pd.DataFrame:
    if ranked.empty:
        return pd.DataFrame()
    baseline_name = config_to_name(
        validate_config(
            {
                "first_entry_weight": 1.0,
                "allow_second_entry": False,
                "second_entry_trigger_drawdown": -0.03,
                "second_entry_requires_parent_signal": True,
                "second_entry_weight": 0.0,
                "allow_third_entry": False,
                "third_entry_trigger_drawdown": -0.04,
                "third_entry_requires_parent_signal": True,
                "third_entry_weight": 0.0,
                "max_total_weight": 1.0,
                "max_hold_days": 20,
                "rebound_check_day": 5,
            }
        )
    )
    baseline = ranked[ranked["config_name"] == baseline_name].head(1).copy()
    best = ranked.head(1).copy()
    if baseline.empty:
        baseline = best.copy()
        baseline["comparison_role"] = "baseline_fallback"
    else:
        baseline["comparison_role"] = "baseline"
    best["comparison_role"] = "optimized_best"
    return pd.concat([baseline, best], ignore_index=True)


def write_outputs(
    output_root: Path,
    summary_df: pd.DataFrame,
    ranked: pd.DataFrame,
    comparison: pd.DataFrame,
    best_trade_log: pd.DataFrame,
    best_equity_curve: pd.DataFrame,
) -> None:
    output_root.mkdir(parents=True, exist_ok=True)
    summary_df.to_csv(output_root / "parameter_grid_results.csv", index=False)
    ranked.to_csv(output_root / "parameter_grid_ranked.csv", index=False)
    comparison.to_csv(output_root / "baseline_vs_optimized_summary.csv", index=False)
    best_trade_log.to_csv(output_root / "optimized_trade_log.csv", index=False)
    best_equity_curve.to_csv(output_root / "optimized_equity_curve.csv", index=False)
    payload = {
        "config_count": int(len(summary_df)),
        "best_config_name": ranked.iloc[0]["config_name"] if not ranked.empty else None,
        "best_total_return": float(ranked.iloc[0]["total_return"]) if not ranked.empty else None,
        "best_max_drawdown": float(ranked.iloc[0]["max_drawdown"]) if not ranked.empty else None,
    }
    (output_root / "summary.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")


def main() -> int:
    args = parse_args()
    mapping_root = Path(args.mapping_root)
    gold_root = Path(args.gold_root)
    output_root = Path(args.output_root)

    etf_history = load_etf_history(mapping_root, args.symbol)
    gold_signals = load_gold_signals(gold_root)

    parameter_space = dict(DEFAULT_PARAMETER_SPACE)
    # 2026-04-29 CST: Added because the baseline must always exist inside the
    # same search surface for direct like-for-like comparison.
    parameter_space["first_entry_weight"] = [0.20, 0.25, 0.30, 0.35, 0.40, 0.50, 1.00]
    parameter_space["second_entry_weight"] = [0.0, 0.20, 0.25, 0.30, 0.35]
    parameter_space["third_entry_weight"] = [0.0, 0.20, 0.25, 0.30, 0.35]

    summary_df, ranked, best_trade_log, best_equity_curve = run_parameter_search(
        etf_history=etf_history,
        gold_signals=gold_signals,
        parameter_space=parameter_space,
        initial_capital=float(args.initial_capital),
        symbol=args.symbol,
        max_configs=int(args.max_configs),
    )
    comparison = build_baseline_comparison(ranked)
    write_outputs(output_root, summary_df, ranked, comparison, best_trade_log, best_equity_curve)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
