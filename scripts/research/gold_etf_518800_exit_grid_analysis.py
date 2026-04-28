#!/usr/bin/env python
# 2026-04-28 CST: Added because the next approved step is to optimize the
# 518800.SH exit rule with a small, fully explainable parameter grid.
# Purpose: sweep rebound-check day and max-hold day on 518800.SH, then test
# simple round-trip cost sensitivity on the top grid candidates.

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import sys

import pandas as pd


DEFAULT_OUTPUT_ROOT = Path(r"E:\SM\docs\research\gold_etf_518800_exit_grid_20260428")
DEFAULT_MAPPING_ROOT = Path(r"E:\SM\docs\research\gold_etf_mapping_20260428")
DEFAULT_ML_ROOT = Path(r"E:\SM\docs\research\gold_ml_hold15d_experiment_20260428")
TARGET_SYMBOL = "518800.SH"
INITIAL_CAPITAL = 1_000_000.0
REBOUND_CHECK_DAYS = [3, 5, 7]
HOLD_DAYS_GRID = [10, 15, 20]
COST_SCENARIOS = {
    "low_cost": 0.0005,
    "mid_cost": 0.0015,
    "high_cost": 0.0030,
}


def load_backtest_module():
    module_path = Path(r"E:\SM\scripts\research\gold_etf_strategy_backtest.py")
    spec = importlib.util.spec_from_file_location("gold_etf_strategy_backtest", module_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


BACKTEST_MODULE = load_backtest_module()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mapping-root", default=str(DEFAULT_MAPPING_ROOT))
    parser.add_argument("--ml-root", default=str(DEFAULT_ML_ROOT))
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    parser.add_argument("--initial-capital", type=float, default=INITIAL_CAPITAL)
    return parser.parse_args()


def build_parameter_grid(rebound_check_days: list[int], hold_days_list: list[int]) -> pd.DataFrame:
    rows = []
    for rebound_check_day in rebound_check_days:
        for hold_days in hold_days_list:
            rows.append(
                {
                    "rebound_check_day": rebound_check_day,
                    "hold_days": hold_days,
                    "policy_name": f"fail_to_rebound_d{rebound_check_day}_hold_{hold_days}d",
                }
            )
    return pd.DataFrame(rows)


def load_baseline_signals(predictions: pd.DataFrame) -> pd.DataFrame:
    baseline = predictions[
        (predictions["split_mode"] == "rolling_5y")
        & (predictions["model_name"] == "rule_baseline")
        & (predictions["selection_policy"] == "all")
        & (predictions["selected_flag"] == 1)
    ][["trade_date", "selected_flag"]].drop_duplicates(subset=["trade_date"]).copy()
    baseline["trade_date"] = pd.to_datetime(baseline["trade_date"])
    return baseline


def resolve_rebound_exit(
    history: pd.DataFrame,
    entry_idx: int,
    hold_days: int,
    rebound_check_day: int,
    entry_price: float,
) -> tuple[int, int | None, str]:
    planned_exit_idx = entry_idx + hold_days
    if planned_exit_idx >= len(history):
        return len(history), None, "insufficient_history"
    check_idx = entry_idx + rebound_check_day - 1
    if check_idx < planned_exit_idx and check_idx + 1 < len(history):
        check_row = history.iloc[check_idx]
        if float(check_row["close"]) <= entry_price:
            return check_idx + 1, check_idx, f"fail_to_rebound_d{rebound_check_day}"
    return planned_exit_idx, None, "time_exit"


def run_grid_backtest(
    policy_signals: pd.DataFrame,
    etf_history: pd.DataFrame,
    symbol: str,
    hold_days: int,
    rebound_check_day: int,
    initial_capital: float,
) -> tuple[pd.DataFrame, pd.DataFrame]:
    history = etf_history.copy().sort_values("trade_date").reset_index(drop=True)
    history["trade_date"] = pd.to_datetime(history["trade_date"])
    signal_table = policy_signals[policy_signals["selected_flag"] == 1].copy()
    signal_table["trade_date"] = pd.to_datetime(signal_table["trade_date"])
    signal_table = signal_table.sort_values("trade_date").reset_index(drop=True)
    date_to_idx = {trade_date: idx for idx, trade_date in enumerate(history["trade_date"])}

    trades: list[dict[str, object]] = []
    next_available_idx = 0
    capital = initial_capital
    policy_name = f"fail_to_rebound_d{rebound_check_day}_hold_{hold_days}d"

    for signal in signal_table.itertuples(index=False):
        signal_idx = date_to_idx.get(signal.trade_date)
        if signal_idx is None:
            continue
        entry_idx = signal_idx + 1
        if entry_idx >= len(history) or entry_idx < next_available_idx:
            continue
        entry_row = history.iloc[entry_idx]
        exit_idx, trigger_idx, exit_reason = resolve_rebound_exit(
            history=history,
            entry_idx=entry_idx,
            hold_days=hold_days,
            rebound_check_day=rebound_check_day,
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
                "policy_name": policy_name,
                "rebound_check_day": rebound_check_day,
                "hold_days": hold_days,
                "signal_date": signal.trade_date,
                "entry_date": entry_row["trade_date"],
                "trigger_date": trigger_date,
                "exit_date": exit_row["trade_date"],
                "entry_price": entry_price,
                "exit_price": exit_price,
                "exit_reason": exit_reason,
                "trade_return": trade_return,
                "entry_capital": entry_capital,
                "exit_capital": exit_capital,
                "hold_calendar_days": int((exit_row["trade_date"] - entry_row["trade_date"]).days),
            }
        )
        capital = exit_capital
        next_available_idx = exit_idx + 1

    trade_log = pd.DataFrame(trades)
    equity_curve = BACKTEST_MODULE.build_daily_equity_curve(history, trade_log, initial_capital)
    return trade_log, equity_curve


def summarize_grid_backtest(
    trade_log: pd.DataFrame,
    equity_curve: pd.DataFrame,
    symbol: str,
    policy_name: str,
    rebound_check_day: int,
    hold_days: int,
    initial_capital: float,
) -> dict[str, object]:
    summary = BACKTEST_MODULE.summarize_single_position_backtest(
        trade_log=trade_log,
        equity_curve=equity_curve,
        symbol=symbol,
        policy_name=policy_name,
        initial_capital=initial_capital,
    )
    summary["rebound_check_day"] = rebound_check_day
    summary["hold_days"] = hold_days
    return summary


def apply_roundtrip_costs(trade_log: pd.DataFrame, initial_capital: float, cost_rate: float) -> dict[str, object]:
    if trade_log.empty:
        return {
            "sample_count": 0,
            "terminal_capital": initial_capital,
            "total_return": 0.0,
            "cagr": None,
        }
    capital = initial_capital
    for trade_return in trade_log["trade_return"]:
        net_trade_return = (1.0 + float(trade_return)) * (1.0 - cost_rate) - 1.0
        capital *= (1.0 + net_trade_return)
    start_date = pd.to_datetime(trade_log["entry_date"].min())
    end_date = pd.to_datetime(trade_log["exit_date"].max())
    years = (end_date - start_date).days / 365.25 if end_date > start_date else 0.0
    cagr = (capital / initial_capital) ** (1 / years) - 1 if years > 0 else None
    return {
        "sample_count": int(len(trade_log)),
        "terminal_capital": capital,
        "total_return": capital / initial_capital - 1.0,
        "cagr": cagr,
    }


def rank_grid_results(summary: pd.DataFrame) -> pd.DataFrame:
    return summary.sort_values(
        ["terminal_capital", "cagr", "max_drawdown"],
        ascending=[False, False, False],
    ).reset_index(drop=True)


def main() -> int:
    args = parse_args()
    mapping_root = Path(args.mapping_root)
    ml_root = Path(args.ml_root)
    output_root = Path(args.output_root)

    etf_histories = pd.read_csv(mapping_root / "gold_etf_mapping_histories.csv")
    etf_histories["trade_date"] = pd.to_datetime(etf_histories["trade_date"])
    etf = etf_histories[etf_histories["symbol"] == TARGET_SYMBOL][["trade_date", "open", "close"]].copy()

    predictions = pd.read_csv(ml_root / "gold_ml_hold15d_predictions.csv")
    signals = load_baseline_signals(predictions)
    grid = build_parameter_grid(REBOUND_CHECK_DAYS, HOLD_DAYS_GRID)

    summaries = []
    trade_logs = []
    equity_curves = []
    for row in grid.itertuples(index=False):
        trade_log, equity_curve = run_grid_backtest(
            policy_signals=signals,
            etf_history=etf,
            symbol=TARGET_SYMBOL,
            hold_days=int(row.hold_days),
            rebound_check_day=int(row.rebound_check_day),
            initial_capital=args.initial_capital,
        )
        trade_logs.append(trade_log)
        equity_curve["rebound_check_day"] = int(row.rebound_check_day)
        equity_curve["hold_days"] = int(row.hold_days)
        equity_curves.append(equity_curve)
        summaries.append(
            summarize_grid_backtest(
                trade_log=trade_log,
                equity_curve=equity_curve,
                symbol=TARGET_SYMBOL,
                policy_name=str(row.policy_name),
                rebound_check_day=int(row.rebound_check_day),
                hold_days=int(row.hold_days),
                initial_capital=args.initial_capital,
            )
        )

    summary = pd.DataFrame(summaries)
    ranked = rank_grid_results(summary)
    top_candidates = ranked.head(3).copy()

    cost_rows = []
    for candidate in top_candidates.itertuples(index=False):
        trade_log = next(
            frame
            for frame in trade_logs
            if not frame.empty
            and int(frame["rebound_check_day"].iloc[0]) == int(candidate.rebound_check_day)
            and int(frame["hold_days"].iloc[0]) == int(candidate.hold_days)
        )
        for scenario_name, cost_rate in COST_SCENARIOS.items():
            cost_metrics = apply_roundtrip_costs(trade_log, initial_capital=args.initial_capital, cost_rate=cost_rate)
            cost_rows.append(
                {
                    "rebound_check_day": int(candidate.rebound_check_day),
                    "hold_days": int(candidate.hold_days),
                    "policy_name": str(candidate.policy_name),
                    "cost_scenario": scenario_name,
                    "cost_rate": cost_rate,
                    **cost_metrics,
                }
            )

    cost_summary = pd.DataFrame(cost_rows)
    output_root.mkdir(parents=True, exist_ok=True)
    summary.to_csv(output_root / "gold_etf_518800_exit_grid_summary.csv", index=False, encoding="utf-8-sig")
    ranked.to_csv(output_root / "gold_etf_518800_exit_grid_ranked.csv", index=False, encoding="utf-8-sig")
    pd.concat(trade_logs, ignore_index=True).to_csv(
        output_root / "gold_etf_518800_exit_grid_trade_log.csv", index=False, encoding="utf-8-sig"
    )
    pd.concat(equity_curves, ignore_index=True).to_csv(
        output_root / "gold_etf_518800_exit_grid_equity_curve.csv", index=False, encoding="utf-8-sig"
    )
    cost_summary.to_csv(output_root / "gold_etf_518800_exit_grid_cost_sensitivity.csv", index=False, encoding="utf-8-sig")

    payload = {
        "top_grid": ranked.head(5).to_dict(orient="records"),
        "top_cost_sensitivity": cost_summary.to_dict(orient="records"),
    }
    (output_root / "summary.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(payload, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
