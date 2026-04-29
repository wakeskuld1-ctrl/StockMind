#!/usr/bin/env python
# 2026-04-29 CST: Added because the approved research route is to test whether
# the 15D repair profit can be trailed instead of forcing a fixed 15D/20D exit.
# Purpose: keep the validated two-layer 518800.SH entry rule unchanged while
# scanning 15D-anchor extension exits through a 200D maximum holding window.

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import sys
from typing import Any

import pandas as pd


DEFAULT_OUTPUT_ROOT = Path(r"E:\SM\docs\research\gold_etf_15d_anchor_extension_20260429")
DEFAULT_SYMBOL = "518800.SH"
INITIAL_CAPITAL = 1_000_000.0
ANCHOR_DAY = 15
REBOUND_CHECK_DAY = 5


def load_position_module():
    # 2026-04-29 CST: Added because this study must reuse the frozen parent
    # signal, ETF history, layered-entry, and equity-curve semantics from the
    # validated position-management research instead of redefining them.
    module_path = Path(r"E:\SM\scripts\research\gold_etf_position_param_optimization.py")
    spec = importlib.util.spec_from_file_location("gold_etf_position_param_optimization", module_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def build_formal_two_layer_config() -> dict[str, Any]:
    # 2026-04-29 CST: Added because the user approved freezing the optimized
    # two-layer entry contract before researching only the exit extension.
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


def build_max_hold_days() -> list[int]:
    # 2026-04-29 CST: Added because the user explicitly expanded the long-tail
    # scan from 40D to 200D to see whether a strong gold regime keeps running.
    return [20, 30, 40, 60, 90, 120, 160, 200]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    parser.add_argument("--symbol", default=DEFAULT_SYMBOL)
    parser.add_argument("--initial-capital", type=float, default=INITIAL_CAPITAL)
    return parser.parse_args()


def resolve_anchor_extension_exit(
    history: pd.DataFrame,
    first_entry_idx: int,
    weighted_entry_price: float,
    rebound_check_day: int,
    anchor_day: int,
    max_hold_days: int,
) -> tuple[int, str, float | None, int]:
    # 2026-04-29 CST: Added because exit timing must remain T-close signal /
    # T+1-open execution while replacing fixed time exit with a 15D return floor.
    if anchor_day <= rebound_check_day:
        raise ValueError("anchor_day must be greater than rebound_check_day")
    if max_hold_days < anchor_day:
        raise ValueError("max_hold_days must be at least anchor_day")

    rebound_check_idx = first_entry_idx + rebound_check_day - 1
    if rebound_check_idx + 1 < len(history):
        rebound_row = history.iloc[rebound_check_idx]
        if float(rebound_row["close"]) <= weighted_entry_price:
            return rebound_check_idx + 1, f"fail_to_rebound_d{rebound_check_day}", None, 0

    anchor_idx = first_entry_idx + anchor_day - 1
    if anchor_idx >= len(history):
        return len(history), "insufficient_anchor_history", None, 0

    anchor_return = float(history.iloc[anchor_idx]["close"]) / weighted_entry_price - 1.0
    max_observation_idx = min(first_entry_idx + max_hold_days - 1, len(history) - 2)
    for close_idx in range(anchor_idx + 1, max_observation_idx + 1):
        close_return = float(history.iloc[close_idx]["close"]) / weighted_entry_price - 1.0
        if close_return < anchor_return:
            signal_day = close_idx - first_entry_idx + 1
            extension_days = signal_day - anchor_day
            return close_idx + 1, f"anchor_return_break_d{signal_day}", anchor_return, extension_days

    planned_exit_idx = first_entry_idx + max_hold_days
    if planned_exit_idx < len(history):
        return planned_exit_idx, f"max_hold_{max_hold_days}d", anchor_return, max_hold_days - anchor_day
    return len(history), "insufficient_history", anchor_return, max(0, len(history) - 1 - anchor_idx)


def run_anchor_extension_backtest(
    etf_history: pd.DataFrame,
    gold_signals: pd.DataFrame,
    config: dict[str, Any],
    initial_capital: float,
    symbol: str,
    max_hold_days: int,
    anchor_day: int = ANCHOR_DAY,
) -> tuple[pd.DataFrame, pd.DataFrame]:
    # 2026-04-29 CST: Added because this path must isolate the exit-rule study
    # while preserving the already validated layered-entry construction.
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
        exit_idx, exit_reason, anchor_return, extension_days = resolve_anchor_extension_exit(
            history=history,
            first_entry_idx=int(layer_entries[0]["entry_idx"]),
            weighted_entry_price=weighted_entry_price,
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
                "extension_days_after_anchor": extension_days,
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


def summarize_anchor_backtest(
    trade_log: pd.DataFrame,
    equity_curve: pd.DataFrame,
    config_name: str,
    initial_capital: float,
    max_hold_days: int,
) -> dict[str, Any]:
    # 2026-04-29 CST: Added because the conclusion must show not only total
    # return but whether the post-15D tail actually contributes enough value.
    if trade_log.empty:
        return {
            "config_name": config_name,
            "max_hold_days": max_hold_days,
            "sample_count": 0,
            "terminal_capital": initial_capital,
            "total_return": 0.0,
            "max_drawdown": 0.0,
            "return_drawdown_ratio": None,
        }

    terminal_capital = float(trade_log["exit_capital"].iloc[-1])
    total_return = terminal_capital / initial_capital - 1.0
    max_drawdown = float(equity_curve["drawdown"].min()) if not equity_curve.empty else 0.0
    if "post_anchor_return_contribution" in trade_log.columns:
        post_anchor = pd.to_numeric(trade_log["post_anchor_return_contribution"], errors="coerce")
    else:
        # 2026-04-29 CST: Added because summary tests and downstream readers may
        # provide only raw trade return plus the 15D anchor return.
        post_anchor = pd.to_numeric(trade_log["trade_return"], errors="coerce") - pd.to_numeric(
            trade_log.get("return_at_anchor_day"), errors="coerce"
        )
    return {
        "config_name": config_name,
        "max_hold_days": max_hold_days,
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
        "hold_over_20d_count": int((trade_log["hold_trading_days"] > 20).sum()),
        "hold_over_40d_count": int((trade_log["hold_trading_days"] > 40).sum()),
        "hold_over_60d_count": int((trade_log["hold_trading_days"] > 60).sum()),
        "hold_over_120d_count": int((trade_log["hold_trading_days"] > 120).sum()),
        "avg_post_anchor_return_contribution": float(post_anchor.dropna().mean()) if post_anchor.notna().any() else None,
        "median_post_anchor_return_contribution": float(post_anchor.dropna().median()) if post_anchor.notna().any() else None,
        "anchor_break_exit_count": int(trade_log["exit_reason"].astype(str).str.startswith("anchor_return_break").sum()),
        "max_hold_exit_count": int(trade_log["exit_reason"].astype(str).str.startswith("max_hold").sum()),
        "fail_rebound_exit_count": int(trade_log["exit_reason"].astype(str).str.startswith("fail_to_rebound").sum()),
    }


def run_scan(initial_capital: float, symbol: str) -> tuple[pd.DataFrame, pd.DataFrame, pd.DataFrame]:
    # 2026-04-29 CST: Added because max-hold sensitivity must be evaluated on
    # the same data and capital base for direct comparison.
    base = load_position_module()
    etf_history = base.load_etf_history(base.DEFAULT_MAPPING_ROOT, symbol)
    gold_signals = base.load_gold_signals(base.DEFAULT_GOLD_ROOT)
    config = build_formal_two_layer_config()

    summaries: list[dict[str, Any]] = []
    best_trade_log = pd.DataFrame()
    best_equity_curve = pd.DataFrame()
    for max_hold_days in build_max_hold_days():
        trade_log, equity_curve = run_anchor_extension_backtest(
            etf_history=etf_history,
            gold_signals=gold_signals,
            config=config,
            initial_capital=initial_capital,
            symbol=symbol,
            max_hold_days=max_hold_days,
        )
        config_name = f"anchor{ANCHOR_DAY}_max{max_hold_days}_r{REBOUND_CHECK_DAY}"
        summaries.append(
            summarize_anchor_backtest(
                trade_log=trade_log,
                equity_curve=equity_curve,
                config_name=config_name,
                initial_capital=initial_capital,
                max_hold_days=max_hold_days,
            )
        )
        if best_trade_log.empty or summaries[-1]["return_drawdown_ratio"] > max(
            summary.get("return_drawdown_ratio") or -9999 for summary in summaries[:-1]
        ):
            best_trade_log = trade_log
            best_equity_curve = equity_curve

    summary_df = pd.DataFrame(summaries)
    ranked = summary_df.sort_values(["return_drawdown_ratio", "total_return"], ascending=[False, False]).reset_index(drop=True)
    return summary_df, ranked, best_trade_log, best_equity_curve


def write_outputs(output_root: Path, summary_df: pd.DataFrame, ranked: pd.DataFrame, best_trade_log: pd.DataFrame, best_equity_curve: pd.DataFrame) -> None:
    # 2026-04-29 CST: Added because future daily execution needs stable CSV and
    # JSON artifacts instead of relying on chat-only conclusions.
    output_root.mkdir(parents=True, exist_ok=True)
    summary_df.to_csv(output_root / "anchor_extension_summary.csv", index=False)
    ranked.to_csv(output_root / "anchor_extension_ranked.csv", index=False)
    best_trade_log.to_csv(output_root / "anchor_extension_trade_log.csv", index=False)
    best_equity_curve.to_csv(output_root / "anchor_extension_equity_curve.csv", index=False)
    payload = {
        "study": "gold_etf_15d_anchor_extension_exit",
        "anchor_day": ANCHOR_DAY,
        "rebound_check_day": REBOUND_CHECK_DAY,
        "max_hold_days": build_max_hold_days(),
        "best_config_name": ranked.iloc[0]["config_name"] if not ranked.empty else None,
        "best_total_return": float(ranked.iloc[0]["total_return"]) if not ranked.empty else None,
        "best_max_drawdown": float(ranked.iloc[0]["max_drawdown"]) if not ranked.empty else None,
        "best_return_drawdown_ratio": float(ranked.iloc[0]["return_drawdown_ratio"]) if not ranked.empty else None,
    }
    (output_root / "summary.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")


def main() -> int:
    args = parse_args()
    summary_df, ranked, best_trade_log, best_equity_curve = run_scan(initial_capital=float(args.initial_capital), symbol=args.symbol)
    write_outputs(Path(args.output_root), summary_df, ranked, best_trade_log, best_equity_curve)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
