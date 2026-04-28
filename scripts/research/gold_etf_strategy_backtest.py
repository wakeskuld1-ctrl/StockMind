#!/usr/bin/env python
# 2026-04-28 CST: Added because ETF mapping identified the main pool and the
# next step is to compare baseline versus light-filtered execution on actual ETF
# candidates.
# 2026-04-28 CST: Revised to remove overlap distortion and enforce a real
# single-position backtest using T+1 open execution after close-based signals.
# Purpose: run a single-position T+1-open backtest on the four main-pool gold
# ETFs using one shared entry rule and multiple exit policies.

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import sys

import pandas as pd


DEFAULT_OUTPUT_ROOT = Path(r"E:\SM\docs\research\gold_etf_strategy_backtest_20260428")
DEFAULT_MAPPING_ROOT = Path(r"E:\SM\docs\research\gold_etf_mapping_20260428")
DEFAULT_ML_ROOT = Path(r"E:\SM\docs\research\gold_ml_hold15d_experiment_20260428")
MAIN_POOL = ["518880.SH", "518800.SH", "159937.SZ", "159934.SZ"]
HOLD_DAYS = 15
INITIAL_CAPITAL = 1_000_000.0
EXIT_POLICIES = [
    "hold_15d_baseline",
    "time_or_break_ma10",
    "time_or_fail_to_rebound",
]


def load_light_filter_module():
    module_path = Path(r"E:\SM\scripts\research\gold_hgb_hold15d_light_filter.py")
    spec = importlib.util.spec_from_file_location("gold_hgb_hold15d_light_filter", module_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mapping-root", default=str(DEFAULT_MAPPING_ROOT))
    parser.add_argument("--ml-root", default=str(DEFAULT_ML_ROOT))
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    parser.add_argument("--hold-days", type=int, default=HOLD_DAYS)
    parser.add_argument("--initial-capital", type=float, default=INITIAL_CAPITAL)
    return parser.parse_args()


def build_policy_signal_table(baseline_signals: pd.DataFrame, scored_signals: pd.DataFrame | None = None) -> pd.DataFrame:
    if scored_signals is None:
        scored_signals = baseline_signals.copy()
        baseline_signals = baseline_signals[["trade_date"]].copy()
        baseline_signals["selected_flag"] = 1
    base = baseline_signals[["trade_date"]].drop_duplicates().copy()
    base["policy_name"] = "hold_15d_baseline"
    base["selected_flag"] = 1
    frames = [base[["trade_date", "policy_name", "selected_flag"]]]
    for policy_name in EXIT_POLICIES[1:]:
        policy = base.copy()
        policy["policy_name"] = policy_name
        frames.append(policy[["trade_date", "policy_name", "selected_flag"]])
    return pd.concat(frames, ignore_index=True).sort_values(["policy_name", "trade_date"]).reset_index(drop=True)


def map_policy_to_etf_trades(policy: pd.DataFrame, etf: pd.DataFrame, symbol: str) -> pd.DataFrame:
    merged = policy.merge(etf, on="trade_date", how="inner")
    selected = merged[merged["selected_flag"] == 1].copy()
    selected["symbol"] = symbol
    return selected


def run_single_position_backtest(
    policy_signals: pd.DataFrame,
    etf_history: pd.DataFrame,
    symbol: str,
    hold_days: int,
    initial_capital: float,
    exit_policy_name: str | None = None,
) -> tuple[pd.DataFrame, pd.DataFrame]:
    history = etf_history.copy().sort_values("trade_date").reset_index(drop=True)
    history["trade_date"] = pd.to_datetime(history["trade_date"])
    if "ma10" not in history.columns:
        history["ma10"] = history["close"].rolling(10, min_periods=1).mean()
    signal_table = policy_signals[policy_signals["selected_flag"] == 1].copy()
    signal_table["trade_date"] = pd.to_datetime(signal_table["trade_date"])
    signal_table = signal_table.sort_values("trade_date").reset_index(drop=True)

    date_to_idx = {trade_date: idx for idx, trade_date in enumerate(history["trade_date"])}
    trades: list[dict[str, object]] = []
    next_available_idx = 0
    capital = initial_capital

    for signal in signal_table.itertuples(index=False):
        signal_idx = date_to_idx.get(signal.trade_date)
        if signal_idx is None:
            continue
        entry_idx = signal_idx + 1
        if entry_idx >= len(history):
            continue
        if entry_idx < next_available_idx:
            continue

        entry_row = history.iloc[entry_idx]
        resolved_policy_name = exit_policy_name or str(signal.policy_name)
        exit_idx, trigger_idx, exit_reason = resolve_exit(
            history=history,
            entry_idx=entry_idx,
            hold_days=hold_days,
            exit_policy_name=resolved_policy_name,
            entry_price=float(entry_row["open"]),
        )
        if exit_idx >= len(history):
            continue
        exit_row = history.iloc[exit_idx]
        entry_price = float(entry_row["open"])
        exit_price = float(exit_row["open"])
        trade_return = exit_price / entry_price - 1.0
        entry_capital = capital
        exit_capital = capital * (1.0 + trade_return)
        trigger_date = history.iloc[trigger_idx]["trade_date"] if trigger_idx is not None else pd.NaT

        trades.append(
            {
                "symbol": symbol,
                "policy_name": resolved_policy_name,
                "signal_date": signal.trade_date,
                "entry_date": entry_row["trade_date"],
                "trigger_date": trigger_date,
                "exit_date": exit_row["trade_date"],
                "signal_index": signal_idx,
                "entry_index": entry_idx,
                "trigger_index": trigger_idx,
                "exit_index": exit_idx,
                "entry_price": entry_price,
                "exit_price": exit_price,
                "exit_reason": exit_reason,
                "trade_return": trade_return,
                "entry_capital": entry_capital,
                "exit_capital": exit_capital,
                "hold_trading_days": hold_days,
                "hold_calendar_days": int((exit_row["trade_date"] - entry_row["trade_date"]).days),
            }
        )
        capital = exit_capital
        next_available_idx = exit_idx + 1

    trade_log = pd.DataFrame(trades)
    equity_curve = build_daily_equity_curve(history, trade_log, initial_capital)
    return trade_log, equity_curve


def resolve_exit(
    history: pd.DataFrame,
    entry_idx: int,
    hold_days: int,
    exit_policy_name: str,
    entry_price: float,
) -> tuple[int, int | None, str]:
    planned_exit_idx = entry_idx + hold_days
    if planned_exit_idx >= len(history):
        return len(history), None, "insufficient_history"

    if exit_policy_name == "time_or_break_ma10":
        for check_idx in range(entry_idx, planned_exit_idx):
            row = history.iloc[check_idx]
            if float(row["close"]) < float(row["ma10"]):
                return check_idx + 1, check_idx, "break_ma10"
    elif exit_policy_name == "time_or_fail_to_rebound":
        check_idx = entry_idx + 4
        if check_idx < planned_exit_idx and check_idx + 1 < len(history):
            row = history.iloc[check_idx]
            if float(row["close"]) <= entry_price:
                return check_idx + 1, check_idx, "fail_to_rebound"

    return planned_exit_idx, None, "time_exit"


def build_daily_equity_curve(history: pd.DataFrame, trade_log: pd.DataFrame, initial_capital: float) -> pd.DataFrame:
    history = history.copy().sort_values("trade_date").reset_index(drop=True)
    history["trade_date"] = pd.to_datetime(history["trade_date"])
    rows: list[dict[str, object]] = []
    cash = initial_capital
    equity = initial_capital
    peak_equity = initial_capital

    for row in history.itertuples(index=False):
        active = pd.DataFrame()
        if not trade_log.empty:
            active = trade_log[
                (trade_log["entry_date"] <= row.trade_date)
                & (trade_log["exit_date"] >= row.trade_date)
            ]

        if active.empty:
            position_flag = 0
            position_symbol = None
            position_size = 0.0
            equity = cash
        else:
            active_trade = active.iloc[0]
            position_flag = 1
            position_symbol = active_trade["symbol"]
            invested_capital = float(active_trade["entry_capital"])
            entry_price = float(active_trade["entry_price"])
            position_size = invested_capital / entry_price
            cash = 0.0
            equity = position_size * float(row.close)
            if row.trade_date == active_trade["exit_date"]:
                cash = float(active_trade["exit_capital"])
                equity = cash
                position_flag = 0
                position_symbol = None
                position_size = 0.0

        peak_equity = max(peak_equity, equity)
        rows.append(
            {
                "date": row.trade_date,
                "cash": cash,
                "equity": equity,
                "position_flag": position_flag,
                "position_symbol": position_symbol,
                "position_size": position_size,
                "drawdown": equity / peak_equity - 1.0 if peak_equity > 0 else 0.0,
            }
        )
        if position_flag == 0 and not active.empty:
            cash = float(active.iloc[0]["exit_capital"])

    return pd.DataFrame(rows)


def summarize_single_position_backtest(
    trade_log: pd.DataFrame,
    equity_curve: pd.DataFrame,
    symbol: str,
    policy_name: str,
    initial_capital: float,
) -> dict[str, object]:
    if trade_log.empty:
        return {
            "symbol": symbol,
            "policy_name": policy_name,
            "sample_count": 0,
            "years_covered": 0,
            "events_per_year": 0.0,
            "win_rate": None,
            "avg_return": None,
            "median_return": None,
            "terminal_capital": initial_capital,
            "total_return": 0.0,
            "max_drawdown": 0.0,
            "cagr": None,
        }

    terminal_capital = (
        float(trade_log["exit_capital"].iloc[-1])
        if "exit_capital" in trade_log.columns
        else float(equity_curve["equity"].iloc[-1])
    )
    years_covered = int(pd.to_datetime(trade_log["entry_date"]).dt.year.nunique())
    avg_return = float(trade_log["trade_return"].mean())
    median_return = float(trade_log["trade_return"].median())
    win_rate = float((trade_log["trade_return"] > 0).mean())
    if equity_curve.empty:
        max_drawdown = 0.0
    elif "drawdown" in equity_curve.columns:
        max_drawdown = float(equity_curve["drawdown"].min())
    else:
        running_peak = equity_curve["equity"].cummax()
        max_drawdown = float((equity_curve["equity"] / running_peak - 1.0).min())
    start_date = pd.to_datetime(trade_log["entry_date"].min())
    end_date = pd.to_datetime(trade_log["exit_date"].max())
    years = (end_date - start_date).days / 365.25 if end_date > start_date else 0.0
    cagr = (terminal_capital / initial_capital) ** (1 / years) - 1 if years > 0 else None
    return {
        "symbol": symbol,
        "policy_name": policy_name,
        "sample_count": int(len(trade_log)),
        "years_covered": years_covered,
        "events_per_year": len(trade_log) / years_covered if years_covered > 0 else 0.0,
        "win_rate": win_rate,
        "avg_return": avg_return,
        "median_return": median_return,
        "avg_hold_calendar_days": float(trade_log["hold_calendar_days"].mean()),
        "terminal_capital": terminal_capital,
        "total_return": terminal_capital / initial_capital - 1.0,
        "max_drawdown": max_drawdown,
        "cagr": cagr,
    }


def summarize_etf_backtest(backtests: list[dict[str, object]] | pd.DataFrame, initial_capital: float | None = None) -> pd.DataFrame:
    if isinstance(backtests, pd.DataFrame):
        base_capital = initial_capital if initial_capital is not None else INITIAL_CAPITAL
        rows = []
        for (symbol, policy_name), subset in backtests.groupby(["symbol", "policy_name"], dropna=False):
            capital = base_capital
            for trade_return in subset["future_ret_15d"]:
                capital *= (1.0 + trade_return)
            rows.append(
                {
                    "symbol": symbol,
                    "policy_name": policy_name,
                    "sample_count": int(len(subset)),
                    "terminal_capital": capital,
                    "total_return": capital / base_capital - 1.0,
                }
            )
        return pd.DataFrame(rows)
    rows = [item["summary"] for item in backtests]
    return pd.DataFrame(rows)


def rank_final_etf_choices(summary: pd.DataFrame) -> pd.DataFrame:
    ranked = summary.copy()
    return ranked.sort_values(
        ["terminal_capital", "cagr", "max_drawdown"],
        ascending=[False, False, False],
    ).reset_index(drop=True)


def load_policy_signals(predictions: pd.DataFrame) -> pd.DataFrame:
    baseline = predictions[
        (predictions["split_mode"] == "rolling_5y")
        & (predictions["model_name"] == "rule_baseline")
        & (predictions["selection_policy"] == "all")
        & (predictions["selected_flag"] == 1)
    ][["trade_date", "selected_flag"]].drop_duplicates(subset=["trade_date"]).copy()
    baseline["trade_date"] = pd.to_datetime(baseline["trade_date"])

    return build_policy_signal_table(baseline)


def main() -> int:
    args = parse_args()
    mapping_root = Path(args.mapping_root)
    ml_root = Path(args.ml_root)

    etf_histories = pd.read_csv(mapping_root / "gold_etf_mapping_histories.csv")
    etf_histories["trade_date"] = pd.to_datetime(etf_histories["trade_date"])

    predictions = pd.read_csv(ml_root / "gold_ml_hold15d_predictions.csv")
    policy_signals = load_policy_signals(predictions)

    backtests: list[dict[str, object]] = []
    all_trade_logs = []
    all_equity_curves = []

    for symbol in MAIN_POOL:
        etf = etf_histories[etf_histories["symbol"] == symbol][["trade_date", "open", "close"]].copy()
        for policy_name in EXIT_POLICIES:
            policy = policy_signals[policy_signals["policy_name"] == policy_name].copy()
            trade_log, equity_curve = run_single_position_backtest(
                policy_signals=policy,
                etf_history=etf,
                symbol=symbol,
                hold_days=args.hold_days,
                initial_capital=args.initial_capital,
                exit_policy_name=policy_name,
            )
            if not trade_log.empty:
                trade_log["policy_name"] = policy_name
            if not equity_curve.empty:
                equity_curve["symbol"] = symbol
                equity_curve["policy_name"] = policy_name
            summary = summarize_single_position_backtest(
                trade_log=trade_log,
                equity_curve=equity_curve,
                symbol=symbol,
                policy_name=policy_name,
                initial_capital=args.initial_capital,
            )
            backtests.append({"summary": summary, "trade_log": trade_log, "equity_curve": equity_curve})
            all_trade_logs.append(trade_log)
            all_equity_curves.append(equity_curve)

    summary = summarize_etf_backtest(backtests)
    ranked = rank_final_etf_choices(summary)

    output_root = Path(args.output_root)
    output_root.mkdir(parents=True, exist_ok=True)
    policy_signals.to_csv(output_root / "gold_etf_strategy_policy_signals.csv", index=False, encoding="utf-8-sig")
    pd.concat(all_trade_logs, ignore_index=True).to_csv(
        output_root / "gold_etf_strategy_trade_log.csv", index=False, encoding="utf-8-sig"
    )
    pd.concat(all_equity_curves, ignore_index=True).to_csv(
        output_root / "gold_etf_strategy_equity_curve.csv", index=False, encoding="utf-8-sig"
    )
    summary.to_csv(output_root / "gold_etf_strategy_summary.csv", index=False, encoding="utf-8-sig")
    ranked.to_csv(output_root / "gold_etf_strategy_ranked.csv", index=False, encoding="utf-8-sig")

    payload = {
        "top_choices": ranked.head(8).to_dict(orient="records"),
    }
    (output_root / "summary.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(payload, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
