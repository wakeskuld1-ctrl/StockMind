#!/usr/bin/env python
# 2026-04-29 CST: Added because the approved next step is exit-only data
# analysis after entry timing was declared uncontrollable for live execution.
# Purpose: compare interpretable post-15D exit styles while keeping the frozen
# gold ETF two-layer entry contract and 5D failure guard unchanged.

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import sys
from typing import Any

import pandas as pd


DEFAULT_OUTPUT_ROOT = Path(r"E:\SM\docs\research\gold_etf_exit_style_analysis_20260429")
DEFAULT_SYMBOL = "518800.SH"
INITIAL_CAPITAL = 1_000_000.0
ANCHOR_DAY = 15
REBOUND_CHECK_DAY = 5
MAX_HOLD_DAYS = 60


def load_position_module():
    # 2026-04-29 CST: Added because this analysis must reuse the validated data
    # loaders, layered-entry helpers, and equity curve math from prior research.
    module_path = Path(r"E:\SM\scripts\research\gold_etf_position_param_optimization.py")
    spec = importlib.util.spec_from_file_location("gold_etf_position_param_optimization", module_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def build_formal_two_layer_config() -> dict[str, Any]:
    # 2026-04-29 CST: Added because exit analysis must not silently mutate the
    # currently approved two-layer entry and position sizing route.
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


def build_exit_rule_space() -> list[dict[str, Any]]:
    # 2026-04-29 CST: Added because方案A requires three interpretable families
    # to be compared before any strategy refinement is promoted.
    rules: list[dict[str, Any]] = []
    for drawdown in [0.01, 0.015, 0.02, 0.03]:
        rules.append({"family": "high_watermark_drawdown", "drawdown": drawdown})
    for ma_window in [5, 10, 20]:
        rules.append({"family": "ma_trend", "ma_window": ma_window})
    for profit_trigger in [0.03, 0.05, 0.08]:
        for giveback_ratio in [0.30, 0.50]:
            rules.append(
                {
                    "family": "staged_profit_giveback",
                    "profit_trigger": profit_trigger,
                    "giveback_ratio": giveback_ratio,
                }
            )
    return rules


def rule_to_name(rule: dict[str, Any]) -> str:
    family = str(rule["family"])
    if family == "high_watermark_drawdown":
        return f"hwm_dd_{float(rule['drawdown']):.3f}"
    if family == "ma_trend":
        return f"ma{int(rule['ma_window'])}_trend"
    if family == "staged_profit_giveback":
        return f"profit_{float(rule['profit_trigger']):.3f}_giveback_{float(rule['giveback_ratio']):.2f}"
    raise ValueError(f"unknown rule family: {family}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    parser.add_argument("--symbol", default=DEFAULT_SYMBOL)
    parser.add_argument("--initial-capital", type=float, default=INITIAL_CAPITAL)
    return parser.parse_args()


def resolve_post15_exit(
    history: pd.DataFrame,
    first_entry_idx: int,
    weighted_entry_price: float,
    rule: dict[str, Any],
    rebound_check_day: int,
    anchor_day: int,
    max_hold_days: int,
) -> tuple[int, str]:
    # 2026-04-29 CST: Added because every exit family must share the same T
    # close decision and T+1 open execution boundary.
    rebound_check_idx = first_entry_idx + rebound_check_day - 1
    if rebound_check_idx + 1 < len(history):
        rebound_row = history.iloc[rebound_check_idx]
        if float(rebound_row["close"]) <= weighted_entry_price:
            return rebound_check_idx + 1, f"fail_to_rebound_d{rebound_check_day}"

    anchor_idx = first_entry_idx + anchor_day - 1
    if anchor_idx >= len(history):
        return len(history), "insufficient_anchor_history"

    final_signal_idx = min(first_entry_idx + max_hold_days - 1, len(history) - 2)
    family = str(rule["family"])
    if family == "high_watermark_drawdown":
        return resolve_high_watermark_drawdown(history, first_entry_idx, anchor_idx, final_signal_idx, weighted_entry_price, rule, max_hold_days)
    if family == "ma_trend":
        return resolve_ma_trend(history, first_entry_idx, anchor_idx, final_signal_idx, rule, max_hold_days)
    if family == "staged_profit_giveback":
        return resolve_staged_profit_giveback(history, first_entry_idx, anchor_idx, final_signal_idx, weighted_entry_price, rule, max_hold_days)
    raise ValueError(f"unknown rule family: {family}")


def resolve_high_watermark_drawdown(
    history: pd.DataFrame,
    first_entry_idx: int,
    anchor_idx: int,
    final_signal_idx: int,
    weighted_entry_price: float,
    rule: dict[str, Any],
    max_hold_days: int,
) -> tuple[int, str]:
    # 2026-04-29 CST: Added because this family tests whether post-15D profit
    # can be protected by trailing from the best close instead of fixed time.
    best_return = float(history.iloc[anchor_idx]["close"]) / weighted_entry_price - 1.0
    drawdown = float(rule["drawdown"])
    for close_idx in range(anchor_idx + 1, final_signal_idx + 1):
        close_return = float(history.iloc[close_idx]["close"]) / weighted_entry_price - 1.0
        best_return = max(best_return, close_return)
        if best_return - close_return >= drawdown:
            signal_day = close_idx - first_entry_idx + 1
            return close_idx + 1, f"hwm_dd_{drawdown:.3f}_d{signal_day}"
    planned_exit_idx = first_entry_idx + max_hold_days
    return min(planned_exit_idx, len(history)), f"max_hold_{max_hold_days}d"


def resolve_ma_trend(
    history: pd.DataFrame,
    first_entry_idx: int,
    anchor_idx: int,
    final_signal_idx: int,
    rule: dict[str, Any],
    max_hold_days: int,
) -> tuple[int, str]:
    # 2026-04-29 CST: Added because this family tests whether a simple trend
    # line can decide the post-15D exit more cleanly than a fixed calendar stop.
    ma_window = int(rule["ma_window"])
    closes = pd.to_numeric(history["close"], errors="coerce")
    ma = closes.rolling(ma_window, min_periods=ma_window).mean()
    for close_idx in range(anchor_idx + 1, final_signal_idx + 1):
        if pd.isna(ma.iloc[close_idx]):
            continue
        if float(closes.iloc[close_idx]) < float(ma.iloc[close_idx]):
            signal_day = close_idx - first_entry_idx + 1
            return close_idx + 1, f"ma{ma_window}_break_d{signal_day}"
    planned_exit_idx = first_entry_idx + max_hold_days
    return min(planned_exit_idx, len(history)), f"max_hold_{max_hold_days}d"


def resolve_staged_profit_giveback(
    history: pd.DataFrame,
    first_entry_idx: int,
    anchor_idx: int,
    final_signal_idx: int,
    weighted_entry_price: float,
    rule: dict[str, Any],
    max_hold_days: int,
) -> tuple[int, str]:
    # 2026-04-29 CST: Added because this family tests whether confirmed profit
    # zones deserve a looser giveback rule than raw fixed-day liquidation.
    profit_trigger = float(rule["profit_trigger"])
    giveback_ratio = float(rule["giveback_ratio"])
    armed = False
    best_return = float(history.iloc[anchor_idx]["close"]) / weighted_entry_price - 1.0
    if best_return >= profit_trigger:
        armed = True
    for close_idx in range(anchor_idx + 1, final_signal_idx + 1):
        close_return = float(history.iloc[close_idx]["close"]) / weighted_entry_price - 1.0
        best_return = max(best_return, close_return)
        if best_return >= profit_trigger:
            armed = True
        if armed:
            stop_return = best_return * (1.0 - giveback_ratio)
            if close_return <= stop_return:
                signal_day = close_idx - first_entry_idx + 1
                return close_idx + 1, f"profit_{profit_trigger:.3f}_giveback_{giveback_ratio:.2f}_d{signal_day}"
    planned_exit_idx = first_entry_idx + max_hold_days
    return min(planned_exit_idx, len(history)), f"max_hold_{max_hold_days}d"


def run_exit_rule_backtest(
    etf_history: pd.DataFrame,
    gold_signals: pd.DataFrame,
    config: dict[str, Any],
    rule: dict[str, Any],
    initial_capital: float,
    symbol: str,
    anchor_day: int = ANCHOR_DAY,
    max_hold_days: int = MAX_HOLD_DAYS,
) -> tuple[pd.DataFrame, pd.DataFrame]:
    # 2026-04-29 CST: Added because each candidate exit rule needs a full
    # portfolio replay, not only isolated event-label statistics.
    base = load_position_module()
    history = etf_history.copy().sort_values("trade_date").reset_index(drop=True)
    history["trade_date"] = pd.to_datetime(history["trade_date"])
    signal_lookup = gold_signals.copy().sort_values("trade_date").reset_index(drop=True)
    signal_dates = set(signal_lookup.loc[signal_lookup["parent_signal"] == 1, "trade_date"])
    signal_meta = signal_lookup.set_index("trade_date")[["ret_5d", "close_vs_ma20", "parent_signal"]]

    trades: list[dict[str, Any]] = []
    next_free_idx = 0
    capital = initial_capital
    config = base.validate_config(config)

    for signal_date in sorted(signal_dates):
        signal_rows = history.index[history["trade_date"] == signal_date].tolist()
        if not signal_rows:
            continue
        signal_idx = signal_rows[0]
        entry_idx = signal_idx + 1
        if entry_idx >= len(history) or entry_idx < next_free_idx:
            continue

        first_row = history.iloc[entry_idx]
        layer_entries: list[dict[str, Any]] = [
            {
                "entry_idx": entry_idx,
                "entry_date": first_row["trade_date"],
                "entry_price": float(first_row["open"]),
                "weight": float(config["first_entry_weight"]),
            }
        ]
        if bool(config["allow_second_entry"]):
            second_layer = base.maybe_add_layer(
                history=history,
                signal_dates=signal_dates,
                start_idx=entry_idx + 1,
                reference_price=float(first_row["open"]),
                trigger_drawdown=float(config["second_entry_trigger_drawdown"]),
                requires_parent_signal=bool(config["second_entry_requires_parent_signal"]),
                weight=float(config["second_entry_weight"]),
            )
            if second_layer is not None:
                layer_entries.append(second_layer)

        total_weight = sum(float(layer["weight"]) for layer in layer_entries)
        if total_weight <= 0.0:
            continue
        weighted_entry_price = sum(float(layer["entry_price"]) * float(layer["weight"]) for layer in layer_entries) / total_weight
        exit_idx, exit_reason = resolve_post15_exit(
            history=history,
            first_entry_idx=int(layer_entries[0]["entry_idx"]),
            weighted_entry_price=weighted_entry_price,
            rule=rule,
            rebound_check_day=int(config["rebound_check_day"]),
            anchor_day=anchor_day,
            max_hold_days=max_hold_days,
        )
        if exit_idx >= len(history):
            continue

        exit_row = history.iloc[exit_idx]
        exit_price = float(exit_row["open"])
        trade_return = (exit_price / weighted_entry_price - 1.0) * total_weight
        entry_capital = capital
        exit_capital = capital * (1.0 + trade_return)
        anchor_idx = int(layer_entries[0]["entry_idx"]) + anchor_day - 1
        anchor_return = None
        if anchor_idx < len(history):
            anchor_return = float(history.iloc[anchor_idx]["close"]) / weighted_entry_price - 1.0
        repair_days, repair_date = base.compute_repair_metrics(
            history,
            first_entry_idx=int(layer_entries[0]["entry_idx"]),
            exit_idx=exit_idx,
            weighted_entry_price=weighted_entry_price,
        )

        trades.append(
            {
                "symbol": symbol,
                "signal_date": signal_date,
                "entry_date": layer_entries[0]["entry_date"],
                "exit_date": exit_row["trade_date"],
                "exit_family": rule["family"],
                "exit_config": rule_to_name(rule),
                "entry_layers": len(layer_entries),
                "layer_entry_dates": "|".join(layer["entry_date"].date().isoformat() for layer in layer_entries),
                "layer_entry_prices": "|".join(f"{float(layer['entry_price']):.4f}" for layer in layer_entries),
                "layer_weights": "|".join(f"{float(layer['weight']):.2f}" for layer in layer_entries),
                "total_weight": total_weight,
                "weighted_entry_price": weighted_entry_price,
                "exit_price": exit_price,
                "exit_reason": exit_reason,
                "trade_return": trade_return,
                "return_at_anchor_day": anchor_return,
                "post_anchor_return_contribution": trade_return - anchor_return * total_weight if anchor_return is not None else None,
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
    equity_curve = base.build_equity_curve(history, trade_log, initial_capital)
    return trade_log, equity_curve


def summarize_exit_backtest(
    trade_log: pd.DataFrame,
    equity_curve: pd.DataFrame,
    rule: dict[str, Any],
    initial_capital: float,
) -> dict[str, Any]:
    # 2026-04-29 CST: Added because the first pass is data analysis, so every
    # rule must expose comparable risk, return, duration, and tail contribution.
    config_name = rule_to_name(rule)
    if trade_log.empty:
        return {
            "config_name": config_name,
            "family": rule["family"],
            "sample_count": 0,
            "terminal_capital": initial_capital,
            "total_return": 0.0,
            "max_drawdown": 0.0,
            "return_drawdown_ratio": None,
        }
    terminal_capital = float(trade_log["exit_capital"].iloc[-1])
    total_return = terminal_capital / initial_capital - 1.0
    max_drawdown = float(equity_curve["drawdown"].min()) if not equity_curve.empty else 0.0
    post_anchor = pd.to_numeric(trade_log.get("post_anchor_return_contribution"), errors="coerce")
    return {
        "config_name": config_name,
        "family": rule["family"],
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
        "avg_post_anchor_return_contribution": float(post_anchor.dropna().mean()) if post_anchor.notna().any() else None,
        "median_post_anchor_return_contribution": float(post_anchor.dropna().median()) if post_anchor.notna().any() else None,
        "fail_rebound_exit_count": int(trade_log["exit_reason"].astype(str).str.startswith("fail_to_rebound").sum()),
        "max_hold_exit_count": int(trade_log["exit_reason"].astype(str).str.startswith("max_hold").sum()),
    }


def rank_exit_results(summary: pd.DataFrame) -> pd.DataFrame:
    # 2026-04-29 CST: Added because策略筛选 must prioritize efficiency and
    # drawdown control before raw cumulative return.
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
    return ranked.sort_values(["return_drawdown_ratio", "total_return"], ascending=[False, False]).reset_index(drop=True)


def build_family_summary(ranked: pd.DataFrame) -> pd.DataFrame:
    # 2026-04-29 CST: Added because the user needs to know which exit family is
    # worth refining, not just which single parameter won.
    rows: list[dict[str, Any]] = []
    for family, group in ranked.groupby("family", sort=False):
        best = group.sort_values(["return_drawdown_ratio", "total_return"], ascending=[False, False]).iloc[0]
        rows.append(
            {
                "family": family,
                "best_config_name": best["config_name"],
                "best_total_return": best["total_return"],
                "best_max_drawdown": best["max_drawdown"],
                "best_return_drawdown_ratio": best["return_drawdown_ratio"],
                "family_config_count": int(len(group)),
            }
        )
    return pd.DataFrame(rows).sort_values(["best_return_drawdown_ratio", "best_total_return"], ascending=[False, False])


def run_analysis(initial_capital: float, symbol: str) -> tuple[pd.DataFrame, pd.DataFrame, pd.DataFrame, pd.DataFrame, pd.DataFrame]:
    # 2026-04-29 CST: Added because all rule families must run on the same
    # frozen history and signal set for a fair first-pass comparison.
    base = load_position_module()
    etf_history = base.load_etf_history(base.DEFAULT_MAPPING_ROOT, symbol)
    gold_signals = base.load_gold_signals(base.DEFAULT_GOLD_ROOT)
    config = build_formal_two_layer_config()

    rows: list[dict[str, Any]] = []
    logs: dict[str, pd.DataFrame] = {}
    curves: dict[str, pd.DataFrame] = {}
    for rule in build_exit_rule_space():
        trade_log, equity_curve = run_exit_rule_backtest(
            etf_history=etf_history,
            gold_signals=gold_signals,
            config=config,
            rule=rule,
            initial_capital=initial_capital,
            symbol=symbol,
        )
        summary = summarize_exit_backtest(trade_log, equity_curve, rule, initial_capital)
        rows.append({**summary, **{key: value for key, value in rule.items() if key != "family"}})
        logs[rule_to_name(rule)] = trade_log
        curves[rule_to_name(rule)] = equity_curve

    summary_df = pd.DataFrame(rows)
    ranked = rank_exit_results(summary_df)
    family_summary = build_family_summary(ranked)
    best_name = ranked.iloc[0]["config_name"] if not ranked.empty else ""
    return summary_df, ranked, family_summary, logs.get(best_name, pd.DataFrame()), curves.get(best_name, pd.DataFrame())


def write_outputs(
    output_root: Path,
    summary_df: pd.DataFrame,
    ranked: pd.DataFrame,
    family_summary: pd.DataFrame,
    best_trade_log: pd.DataFrame,
    best_equity_curve: pd.DataFrame,
) -> None:
    # 2026-04-29 CST: Added because this is a data-analysis pass and must leave
    # stable artifacts for later strategy refinement.
    output_root.mkdir(parents=True, exist_ok=True)
    summary_df.to_csv(output_root / "exit_style_summary.csv", index=False)
    ranked.to_csv(output_root / "exit_style_ranked.csv", index=False)
    family_summary.to_csv(output_root / "exit_family_summary.csv", index=False)
    best_trade_log.to_csv(output_root / "best_exit_trade_log.csv", index=False)
    best_equity_curve.to_csv(output_root / "best_exit_equity_curve.csv", index=False)
    payload = {
        "study": "gold_etf_exit_style_analysis",
        "anchor_day": ANCHOR_DAY,
        "rebound_check_day": REBOUND_CHECK_DAY,
        "max_hold_days": MAX_HOLD_DAYS,
        "rule_count": int(len(summary_df)),
        "best_config_name": ranked.iloc[0]["config_name"] if not ranked.empty else None,
        "best_family": ranked.iloc[0]["family"] if not ranked.empty else None,
        "best_total_return": float(ranked.iloc[0]["total_return"]) if not ranked.empty else None,
        "best_max_drawdown": float(ranked.iloc[0]["max_drawdown"]) if not ranked.empty else None,
        "best_return_drawdown_ratio": float(ranked.iloc[0]["return_drawdown_ratio"]) if not ranked.empty else None,
    }
    (output_root / "summary.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")


def main() -> int:
    args = parse_args()
    summary_df, ranked, family_summary, best_trade_log, best_equity_curve = run_analysis(
        initial_capital=float(args.initial_capital),
        symbol=args.symbol,
    )
    write_outputs(Path(args.output_root), summary_df, ranked, family_summary, best_trade_log, best_equity_curve)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
