#!/usr/bin/env python
# 2026-04-28 CST: Added because the next approved step is to add a daily
# premium proxy and determine the best ETF plus rule combination across the
# full A-share gold ETF universe.
# Purpose: run a full-universe gold ETF comparison using proxy premium filters
# instead of verified NAV premium, and rank ETF-rule combinations accordingly.

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import sys

import pandas as pd
import math


DEFAULT_OUTPUT_ROOT = Path(r"E:\SM\docs\research\gold_etf_all_proxy_premium_20260428")
DEFAULT_MAPPING_ROOT = Path(r"E:\SM\docs\research\gold_etf_mapping_20260428")
DEFAULT_ML_ROOT = Path(r"E:\SM\docs\research\gold_ml_hold15d_experiment_20260428")
DEFAULT_GOLD_PROXY_ROOT = Path(r"E:\SM\docs\research\gold_structure_proxy_10y_20260428")
DEFAULT_START_DATE = "2021-01-01"
DEFAULT_END_DATE = "2026-04-28"
INITIAL_CAPITAL = 1_000_000.0
PREMIUM_FILTERS = {
    "no_filter": None,
    "light_filter": 0.01,
    "strict_filter": 0.0,
}


def load_module(module_path: Path, module_name: str):
    spec = importlib.util.spec_from_file_location(module_name, module_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


BACKTEST_MODULE = load_module(Path(r"E:\SM\scripts\research\gold_etf_strategy_backtest.py"), "gold_etf_strategy_backtest")
MAPPING_MODULE = load_module(Path(r"E:\SM\scripts\research\gold_etf_mapping_analysis.py"), "gold_etf_mapping_analysis")
GRID_MODULE = load_module(Path(r"E:\SM\scripts\research\gold_mean_reversion_entry_grid_v3.py"), "gold_mean_reversion_entry_grid_v3")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mapping-root", default=str(DEFAULT_MAPPING_ROOT))
    parser.add_argument("--ml-root", default=str(DEFAULT_ML_ROOT))
    parser.add_argument("--gold-proxy-root", default=str(DEFAULT_GOLD_PROXY_ROOT))
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    parser.add_argument("--start-date", default=DEFAULT_START_DATE)
    parser.add_argument("--end-date", default=DEFAULT_END_DATE)
    parser.add_argument("--initial-capital", type=float, default=INITIAL_CAPITAL)
    return parser.parse_args()


def build_strategy_grid() -> pd.DataFrame:
    return pd.DataFrame(
        [
            {"strategy_name": "hold_15d_baseline", "hold_days": 15, "rebound_check_day": None},
            {"strategy_name": "fail_to_rebound_d3_hold_20d", "hold_days": 20, "rebound_check_day": 3},
            {"strategy_name": "fail_to_rebound_d5_hold_20d", "hold_days": 20, "rebound_check_day": 5},
        ]
    )


def load_gold_proxy_frame(start_date: str, end_date: str, gold_proxy_root: Path) -> pd.DataFrame:
    local_file = gold_proxy_root / "gold_proxy_flagged_events.csv"
    if local_file.exists():
        frame = pd.read_csv(local_file, usecols=["trade_date", "gold_close"])
        frame["trade_date"] = pd.to_datetime(frame["trade_date"])
        frame = frame.drop_duplicates(subset=["trade_date"]).sort_values("trade_date")
        mask = (frame["trade_date"] >= pd.Timestamp(start_date)) & (frame["trade_date"] <= pd.Timestamp(end_date))
        return frame.loc[mask].reset_index(drop=True)
    base_frame, _ = GRID_MODULE.V2_MODULE.BROAD_MODULE.RULE_MODULE.BASE_MODULE.prepare_analysis_frame(start_date, end_date)
    return base_frame[["trade_date", "gold_close"]].drop_duplicates().copy()


def load_baseline_signals(predictions: pd.DataFrame) -> pd.DataFrame:
    baseline = predictions[
        (predictions["split_mode"] == "rolling_5y")
        & (predictions["model_name"] == "rule_baseline")
        & (predictions["selection_policy"] == "all")
        & (predictions["selected_flag"] == 1)
    ][["trade_date", "selected_flag"]].drop_duplicates(subset=["trade_date"]).copy()
    baseline["trade_date"] = pd.to_datetime(baseline["trade_date"])
    return baseline


def compute_proxy_premium(frame: pd.DataFrame, window: int = 60) -> pd.DataFrame:
    out = frame.copy().sort_values(["symbol", "trade_date"]).reset_index(drop=True)
    out["trade_date"] = pd.to_datetime(out["trade_date"])
    out["price_gold_ratio"] = out["close"] / out["gold_close"]
    min_periods = min(window, 20)
    out["ratio_anchor"] = (
        out.groupby("symbol")["price_gold_ratio"]
        .transform(lambda s: s.rolling(window=window, min_periods=min_periods).median().shift(1))
    )
    out["proxy_nav"] = out["gold_close"] * out["ratio_anchor"]
    out["premium_proxy"] = out["close"] / out["proxy_nav"] - 1.0
    return out


def apply_premium_filter(signals: pd.DataFrame, ceiling: float | None, filter_name: str) -> pd.DataFrame:
    out = signals.copy()
    out["premium_filter"] = filter_name
    if ceiling is None:
        return out
    out.loc[out["premium_proxy"] > ceiling, "selected_flag"] = 0
    return out


def run_strategy_backtest(
    signals: pd.DataFrame,
    etf_history: pd.DataFrame,
    symbol: str,
    strategy_name: str,
    hold_days: int,
    rebound_check_day: int | None,
    initial_capital: float,
) -> tuple[pd.DataFrame, pd.DataFrame]:
    history = etf_history[["trade_date", "open", "close"]].copy().sort_values("trade_date").reset_index(drop=True)
    history["trade_date"] = pd.to_datetime(history["trade_date"])
    signal_table = signals[signals["selected_flag"] == 1].copy()
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
        if entry_idx >= len(history) or entry_idx < next_available_idx:
            continue
        entry_row = history.iloc[entry_idx]
        exit_idx, trigger_idx, exit_reason = resolve_strategy_exit(
            history=history,
            entry_idx=entry_idx,
            hold_days=hold_days,
            rebound_check_day=rebound_check_day,
            entry_price=float(entry_row["open"]),
        )
        if exit_idx >= len(history):
            continue
        exit_row = history.iloc[exit_idx]
        trade_return = float(exit_row["open"]) / float(entry_row["open"]) - 1.0
        entry_capital = capital
        exit_capital = capital * (1.0 + trade_return)
        trigger_date = history.iloc[trigger_idx]["trade_date"] if trigger_idx is not None else pd.NaT
        trades.append(
            {
                "symbol": symbol,
                "policy_name": strategy_name,
                "strategy_name": strategy_name,
                "signal_date": signal.trade_date,
                "entry_date": entry_row["trade_date"],
                "trigger_date": trigger_date,
                "exit_date": exit_row["trade_date"],
                "entry_price": float(entry_row["open"]),
                "exit_price": float(exit_row["open"]),
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
    if not equity_curve.empty:
        equity_curve["strategy_name"] = strategy_name
    return trade_log, equity_curve


def resolve_strategy_exit(
    history: pd.DataFrame,
    entry_idx: int,
    hold_days: int,
    rebound_check_day: int | None,
    entry_price: float,
) -> tuple[int, int | None, str]:
    planned_exit_idx = entry_idx + hold_days
    if planned_exit_idx >= len(history):
        return len(history), None, "insufficient_history"
    if rebound_check_day is None or (isinstance(rebound_check_day, float) and math.isnan(rebound_check_day)):
        return planned_exit_idx, None, "time_exit"
    check_idx = entry_idx + int(rebound_check_day) - 1
    if check_idx < planned_exit_idx and check_idx + 1 < len(history):
        row = history.iloc[check_idx]
        if float(row["close"]) <= entry_price:
            return check_idx + 1, check_idx, f"fail_to_rebound_d{int(rebound_check_day)}"
    return planned_exit_idx, None, "time_exit"


def summarize_run(
    trade_log: pd.DataFrame,
    equity_curve: pd.DataFrame,
    symbol: str,
    strategy_name: str,
    premium_filter: str,
    initial_capital: float,
) -> dict[str, object]:
    summary = BACKTEST_MODULE.summarize_single_position_backtest(
        trade_log=trade_log,
        equity_curve=equity_curve,
        symbol=symbol,
        policy_name=strategy_name,
        initial_capital=initial_capital,
    )
    summary["strategy_name"] = strategy_name
    summary["premium_filter"] = premium_filter
    return summary


def rank_proxy_premium_results(summary: pd.DataFrame) -> pd.DataFrame:
    return summary.sort_values(
        ["terminal_capital", "cagr", "max_drawdown"],
        ascending=[False, False, False],
    ).reset_index(drop=True)


def main() -> int:
    args = parse_args()
    mapping_root = Path(args.mapping_root)
    ml_root = Path(args.ml_root)
    gold_proxy_root = Path(args.gold_proxy_root)
    output_root = Path(args.output_root)

    histories = pd.read_csv(mapping_root / "gold_etf_mapping_histories.csv")
    histories["trade_date"] = pd.to_datetime(histories["trade_date"])
    gold_proxy = load_gold_proxy_frame(args.start_date, args.end_date, gold_proxy_root)
    histories = histories.merge(gold_proxy, on="trade_date", how="left")
    histories = compute_proxy_premium(histories)

    predictions = pd.read_csv(ml_root / "gold_ml_hold15d_predictions.csv")
    signals = load_baseline_signals(predictions)
    strategy_grid = build_strategy_grid()
    universe = histories["symbol"].dropna().drop_duplicates().tolist()

    summaries = []
    trade_logs = []
    equity_curves = []
    filtered_signal_rows = []

    for symbol in universe:
        etf = histories[histories["symbol"] == symbol].copy().sort_values("trade_date")
        signal_frame = signals.merge(
            etf[["trade_date", "premium_proxy"]].drop_duplicates(subset=["trade_date"]),
            on="trade_date",
            how="inner",
        )
        for filter_name, ceiling in PREMIUM_FILTERS.items():
            filtered_signals = apply_premium_filter(signal_frame, ceiling=ceiling, filter_name=filter_name)
            filtered_signals["symbol"] = symbol
            filtered_signal_rows.append(filtered_signals)
            for strategy in strategy_grid.itertuples(index=False):
                trade_log, equity_curve = run_strategy_backtest(
                    signals=filtered_signals[["trade_date", "selected_flag"]].copy(),
                    etf_history=etf,
                    symbol=symbol,
                    strategy_name=strategy.strategy_name,
                    hold_days=int(strategy.hold_days),
                    rebound_check_day=strategy.rebound_check_day,
                    initial_capital=args.initial_capital,
                )
                if not trade_log.empty:
                    trade_log["premium_filter"] = filter_name
                    trade_log["strategy_name"] = strategy.strategy_name
                if not equity_curve.empty:
                    equity_curve["symbol"] = symbol
                    equity_curve["premium_filter"] = filter_name
                    equity_curve["strategy_name"] = strategy.strategy_name
                summaries.append(
                    summarize_run(
                        trade_log=trade_log,
                        equity_curve=equity_curve,
                        symbol=symbol,
                        strategy_name=strategy.strategy_name,
                        premium_filter=filter_name,
                        initial_capital=args.initial_capital,
                    )
                )
                trade_logs.append(trade_log)
                equity_curves.append(equity_curve)

    summary = pd.DataFrame(summaries)
    ranked = rank_proxy_premium_results(summary)
    filtered_signals = pd.concat(filtered_signal_rows, ignore_index=True) if filtered_signal_rows else pd.DataFrame()
    trade_log = pd.concat(trade_logs, ignore_index=True) if trade_logs else pd.DataFrame()
    equity_curve = pd.concat(equity_curves, ignore_index=True) if equity_curves else pd.DataFrame()

    output_root.mkdir(parents=True, exist_ok=True)
    histories.to_csv(output_root / "gold_etf_proxy_premium_histories.csv", index=False, encoding="utf-8-sig")
    filtered_signals.to_csv(output_root / "gold_etf_proxy_premium_signals.csv", index=False, encoding="utf-8-sig")
    trade_log.to_csv(output_root / "gold_etf_proxy_premium_trade_log.csv", index=False, encoding="utf-8-sig")
    equity_curve.to_csv(output_root / "gold_etf_proxy_premium_equity_curve.csv", index=False, encoding="utf-8-sig")
    summary.to_csv(output_root / "gold_etf_proxy_premium_summary.csv", index=False, encoding="utf-8-sig")
    ranked.to_csv(output_root / "gold_etf_proxy_premium_ranked.csv", index=False, encoding="utf-8-sig")

    payload = {
        "assumption": "daily_proxy_premium_not_verified_nav",
        "top_ranked": ranked.head(15).to_dict(orient="records"),
    }
    (output_root / "summary.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(payload, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
