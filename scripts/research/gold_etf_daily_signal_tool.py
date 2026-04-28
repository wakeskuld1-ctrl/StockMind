#!/usr/bin/env python
# 2026-04-28 CST: Added because the approved next step is to package the
# validated 518800.SH rule into a daily decision tool and future skill entry.
# Purpose: emit a next-trading-day buy/hold/sell recommendation using the
# current gold ETF rulebook and local research artifacts.

from __future__ import annotations

import argparse
import json
from pathlib import Path

import pandas as pd


DEFAULT_MAPPING_ROOT = Path(r"E:\SM\docs\research\gold_etf_mapping_20260428")
DEFAULT_SIGNAL_ROOT = Path(r"E:\SM\docs\research\gold_ml_hold15d_experiment_20260428")
DEFAULT_GOLD_PROXY_ROOT = Path(r"E:\SM\docs\research\gold_structure_proxy_10y_20260428")
DEFAULT_SYMBOL = "518800.SH"
SUPPORTED_STRATEGIES = {
    "fail_to_rebound_d5_hold_20d": {"hold_days": 20, "rebound_check_day": 5},
    "fail_to_rebound_d3_hold_20d": {"hold_days": 20, "rebound_check_day": 3},
    "hold_15d_baseline": {"hold_days": 15, "rebound_check_day": None},
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--as-of-date", required=True)
    parser.add_argument("--symbol", default=DEFAULT_SYMBOL)
    parser.add_argument("--strategy-name", default="fail_to_rebound_d5_hold_20d")
    parser.add_argument("--mapping-root", default=str(DEFAULT_MAPPING_ROOT))
    parser.add_argument("--signal-root", default=str(DEFAULT_SIGNAL_ROOT))
    parser.add_argument("--gold-proxy-root", default=str(DEFAULT_GOLD_PROXY_ROOT))
    parser.add_argument("--position-entry-date")
    parser.add_argument("--position-entry-price", type=float)
    return parser.parse_args()


def load_histories(mapping_root: Path, symbol: str) -> pd.DataFrame:
    # 2026-04-28 CST: Added because the daily tool must reuse the verified ETF
    # history source instead of recomputing from external downloads.
    frame = pd.read_csv(mapping_root / "gold_etf_mapping_histories.csv")
    frame["trade_date"] = pd.to_datetime(frame["trade_date"])
    return frame[frame["symbol"] == symbol].copy().sort_values("trade_date").reset_index(drop=True)


def load_gold_frame(gold_proxy_root: Path) -> pd.DataFrame:
    frame = pd.read_csv(gold_proxy_root / "gold_proxy_flagged_events.csv", usecols=["trade_date", "gold_close", "ret_5d", "close_vs_ma20"])
    frame["trade_date"] = pd.to_datetime(frame["trade_date"])
    return frame.drop_duplicates(subset=["trade_date"]).sort_values("trade_date").reset_index(drop=True)


def load_signal_dates(signal_root: Path) -> pd.DataFrame:
    frame = pd.read_csv(signal_root / "gold_ml_hold15d_predictions.csv")
    signal_dates = frame[
        (frame["split_mode"] == "rolling_5y")
        & (frame["model_name"] == "rule_baseline")
        & (frame["selection_policy"] == "all")
        & (frame["selected_flag"] == 1)
    ][["trade_date"]].drop_duplicates().copy()
    signal_dates["trade_date"] = pd.to_datetime(signal_dates["trade_date"])
    return signal_dates.sort_values("trade_date").reset_index(drop=True)


def evaluate_flat_signal(as_of_date: pd.Timestamp, signal_dates: pd.DataFrame, symbol: str, strategy_name: str) -> dict[str, object]:
    signal_hit = bool((signal_dates["trade_date"] == as_of_date).any())
    return {
        "state": "flat",
        "action": "buy_next_open" if signal_hit else "observe",
        "symbol": symbol,
        "strategy_name": strategy_name,
        "signal_date": as_of_date.date().isoformat() if signal_hit else None,
        "reason": "gold_parent_rule_triggered" if signal_hit else "no_parent_signal",
    }


def evaluate_position_signal(
    as_of_date: pd.Timestamp,
    history: pd.DataFrame,
    strategy_name: str,
    entry_date: pd.Timestamp,
    entry_price: float,
    symbol: str,
) -> dict[str, object]:
    config = SUPPORTED_STRATEGIES[strategy_name]
    history = history.copy()
    history = history[history["trade_date"] >= entry_date].reset_index(drop=True)
    as_of_rows = history[history["trade_date"] <= as_of_date].copy()
    if as_of_rows.empty:
        return {
            "state": "position",
            "action": "unknown",
            "symbol": symbol,
            "strategy_name": strategy_name,
            "reason": "entry_date_outside_history",
        }
    holding_days = len(as_of_rows)
    latest_row = as_of_rows.iloc[-1]
    if config["rebound_check_day"] is not None and holding_days == int(config["rebound_check_day"]):
        if float(latest_row["close"]) <= float(entry_price):
            return {
                "state": "position",
                "action": "sell_next_open",
                "symbol": symbol,
                "strategy_name": strategy_name,
                "reason": f"fail_to_rebound_d{int(config['rebound_check_day'])}",
                "holding_days": holding_days,
                "as_of_close": float(latest_row["close"]),
            }
    if holding_days >= int(config["hold_days"]):
        return {
            "state": "position",
            "action": "sell_next_open",
            "symbol": symbol,
            "strategy_name": strategy_name,
            "reason": "max_hold_reached",
            "holding_days": holding_days,
            "as_of_close": float(latest_row["close"]),
        }
    return {
        "state": "position",
        "action": "hold",
        "symbol": symbol,
        "strategy_name": strategy_name,
        "reason": "rule_not_triggered",
        "holding_days": holding_days,
        "as_of_close": float(latest_row["close"]),
    }


def build_daily_report(args: argparse.Namespace) -> dict[str, object]:
    as_of_date = pd.Timestamp(args.as_of_date)
    mapping_root = Path(args.mapping_root)
    signal_root = Path(args.signal_root)
    gold_proxy_root = Path(args.gold_proxy_root)

    histories = load_histories(mapping_root, args.symbol)
    gold_frame = load_gold_frame(gold_proxy_root)
    signal_dates = load_signal_dates(signal_root)
    latest_gold = gold_frame[gold_frame["trade_date"] <= as_of_date].sort_values("trade_date").tail(1)
    flat_signal = evaluate_flat_signal(as_of_date, signal_dates, args.symbol, args.strategy_name)

    gold_snapshot = latest_gold.to_dict(orient="records")[0] if not latest_gold.empty else None
    if gold_snapshot and isinstance(gold_snapshot.get("trade_date"), pd.Timestamp):
        gold_snapshot["trade_date"] = gold_snapshot["trade_date"].date().isoformat()

    report = {
        "as_of_date": as_of_date.date().isoformat(),
        "symbol": args.symbol,
        "strategy_name": args.strategy_name,
        "rule_assumption": "t_close_signal_then_t1_open_execution",
        "proxy_premium_note": "not_using_proxy_premium_filter_in_formal_rule",
        "gold_snapshot": gold_snapshot,
        "flat_signal": flat_signal,
        "position_signal": None,
    }
    if args.position_entry_date and args.position_entry_price is not None:
        report["position_signal"] = evaluate_position_signal(
            as_of_date=as_of_date,
            history=histories,
            strategy_name=args.strategy_name,
            entry_date=pd.Timestamp(args.position_entry_date),
            entry_price=float(args.position_entry_price),
            symbol=args.symbol,
        )
    return report


def main() -> int:
    args = parse_args()
    report = build_daily_report(args)
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
