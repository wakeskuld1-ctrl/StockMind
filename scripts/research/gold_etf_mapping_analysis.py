#!/usr/bin/env python
# 2026-04-28 CST: Added because the gold-proxy rule research now needs to
# evaluate whether A-share gold ETFs can actually carry the signal.
# Purpose: map the approved gold mean-reversion rule from gold proxy dates onto
# all listed A-share gold ETFs, then rank ETFs by coverage, liquidity, and
# signal-carry performance.

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import sys

import numpy as np
import pandas as pd

try:
    import yfinance as yf
except ImportError as exc:  # pragma: no cover
    raise SystemExit("yfinance is required for ETF mapping research.") from exc


DEFAULT_OUTPUT_ROOT = Path(r"E:\SM\docs\research\gold_etf_mapping_20260428")
DEFAULT_START_DATE = "2016-01-01"
DEFAULT_END_DATE = "2026-04-28"
HOLDING_WINDOWS = [10, 15]


def load_grid_module():
    module_path = Path(r"E:\SM\scripts\research\gold_mean_reversion_entry_grid_v3.py")
    spec = importlib.util.spec_from_file_location("gold_mean_reversion_entry_grid_v3", module_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


GRID_MODULE = load_grid_module()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--start-date", default=DEFAULT_START_DATE)
    parser.add_argument("--end-date", default=DEFAULT_END_DATE)
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    return parser.parse_args()


def build_etf_universe() -> pd.DataFrame:
    rows = [
        {"symbol": "518880.SH", "yf_ticker": "518880.SS", "fund_name": "华安黄金ETF", "group_name": "au9999"},
        {"symbol": "518800.SH", "yf_ticker": "518800.SS", "fund_name": "国泰黄金ETF", "group_name": "au9999"},
        {"symbol": "159934.SZ", "yf_ticker": "159934.SZ", "fund_name": "易方达黄金ETF", "group_name": "au9999"},
        {"symbol": "159937.SZ", "yf_ticker": "159937.SZ", "fund_name": "博时黄金ETF", "group_name": "au9999"},
        {"symbol": "518850.SH", "yf_ticker": "518850.SS", "fund_name": "华安黄金ETF9999", "group_name": "au9999"},
        {"symbol": "518660.SH", "yf_ticker": "518660.SS", "fund_name": "工银黄金ETF", "group_name": "au9999"},
        {"symbol": "159812.SZ", "yf_ticker": "159812.SZ", "fund_name": "前海开源黄金ETF", "group_name": "au9999"},
        {"symbol": "518680.SH", "yf_ticker": "518680.SS", "fund_name": "富国上海金ETF", "group_name": "shanghai_gold"},
        {"symbol": "518600.SH", "yf_ticker": "518600.SS", "fund_name": "广发上海金ETF", "group_name": "shanghai_gold"},
        {"symbol": "518860.SH", "yf_ticker": "518860.SS", "fund_name": "建信上海金AUETF", "group_name": "shanghai_gold"},
        {"symbol": "518890.SH", "yf_ticker": "518890.SS", "fund_name": "中银上海金ETF", "group_name": "shanghai_gold"},
    ]
    return pd.DataFrame(rows)


def normalize_yfinance_history(frame: pd.DataFrame, symbol: str) -> pd.DataFrame:
    out = frame.copy()
    if isinstance(out.columns, pd.MultiIndex):
        out.columns = [col[0] for col in out.columns]
    renamed = out.rename(
        columns={
            "Open": "open",
            "High": "high",
            "Low": "low",
            "Close": "close",
            "Adj Close": "adj_close",
            "Volume": "volume",
        }
    ).reset_index()
    date_col = "Date" if "Date" in renamed.columns else "index"
    renamed["trade_date"] = pd.to_datetime(renamed[date_col])
    renamed["symbol"] = symbol
    keep_cols = ["trade_date", "symbol", "open", "high", "low", "close", "adj_close", "volume"]
    for column in keep_cols:
        if column not in renamed.columns:
            renamed[column] = np.nan
    return renamed[keep_cols].sort_values("trade_date").reset_index(drop=True)


def fetch_etf_history(yf_ticker: str, symbol: str, start_date: str, end_date: str) -> pd.DataFrame:
    frame = yf.download(yf_ticker, start=start_date, end=end_date, progress=False, threads=False, auto_adjust=False)
    if frame.empty:
        return pd.DataFrame(columns=["trade_date", "symbol", "open", "high", "low", "close", "adj_close", "volume"])
    return normalize_yfinance_history(frame, symbol=symbol)


def build_forward_metrics(frame: pd.DataFrame) -> pd.DataFrame:
    out = frame.copy()
    closes = out["close"].to_numpy()
    highs = out["high"].to_numpy()
    lows = out["low"].to_numpy()
    for holding_days in HOLDING_WINDOWS:
        future_ret = np.full(len(out), np.nan)
        hold_dd = np.full(len(out), np.nan)
        hold_up = np.full(len(out), np.nan)
        for idx in range(len(out)):
            if idx + holding_days < len(out):
                entry = closes[idx]
                future_ret[idx] = closes[idx + holding_days] / entry - 1.0
                hold_dd[idx] = lows[idx + 1 : idx + holding_days + 1].min() / entry - 1.0
                hold_up[idx] = highs[idx + 1 : idx + holding_days + 1].max() / entry - 1.0
        out[f"future_ret_{holding_days}d"] = future_ret
        out[f"hold_max_drawdown_{holding_days}d"] = hold_dd
        out[f"hold_max_runup_{holding_days}d"] = hold_up
    out["proxy_turnover"] = out["close"] * out["volume"]
    return out


def build_gold_rule_signals(start_date: str, end_date: str) -> pd.DataFrame:
    base_frame, _ = GRID_MODULE.V2_MODULE.BROAD_MODULE.RULE_MODULE.BASE_MODULE.prepare_analysis_frame(start_date, end_date)
    flagged = GRID_MODULE.flag_entry_config(base_frame, ret_threshold=-0.02, ma20_threshold=-0.015, use_risk_filter=False)
    return flagged[flagged["entry_flag"] == 1][["trade_date"]].drop_duplicates().reset_index(drop=True)


def map_rule_signals_to_etf_returns(gold_signals: pd.DataFrame, etf: pd.DataFrame) -> dict[str, float | int]:
    merged = gold_signals.merge(etf, on="trade_date", how="inner")
    usable = merged.dropna(subset=["future_ret_15d"]).copy()
    if usable.empty:
        return {
            "signal_count": 0,
            "avg_return_15d": np.nan,
            "win_rate_15d": np.nan,
            "avg_drawdown_15d": np.nan,
            "avg_runup_15d": np.nan,
            "avg_proxy_turnover": np.nan,
        }
    return {
        "signal_count": int(len(usable)),
        "avg_return_15d": float(usable["future_ret_15d"].mean()),
        "win_rate_15d": float((usable["future_ret_15d"] > 0).mean()),
        "avg_drawdown_15d": float(usable["hold_max_drawdown_15d"].mean()),
        "avg_runup_15d": float(usable["hold_max_runup_15d"].mean()),
        "avg_proxy_turnover": float(usable["proxy_turnover"].mean()),
    }


def rank_etfs(summary: pd.DataFrame) -> pd.DataFrame:
    ranked = summary.copy()
    ranked["score_signal"] = ranked["avg_return_15d"].rank(pct=True, ascending=True)
    ranked["score_win"] = ranked["win_rate_15d"].rank(pct=True, ascending=True)
    ranked["score_liquidity"] = ranked["avg_proxy_turnover"].rank(pct=True, ascending=True)
    ranked["score_history"] = ranked["history_days"].rank(pct=True, ascending=True)
    ranked["composite_score"] = (
        ranked["score_signal"] * 0.35
        + ranked["score_win"] * 0.25
        + ranked["score_liquidity"] * 0.25
        + ranked["score_history"] * 0.15
    )
    return ranked.sort_values(["composite_score", "avg_return_15d", "avg_proxy_turnover"], ascending=[False, False, False]).reset_index(drop=True)


def main() -> int:
    args = parse_args()
    universe = build_etf_universe()
    gold_signals = build_gold_rule_signals(args.start_date, args.end_date)

    etf_frames = []
    rows = []
    for _, row in universe.iterrows():
        history = fetch_etf_history(row["yf_ticker"], row["symbol"], args.start_date, args.end_date)
        if history.empty:
            summary = {
                "symbol": row["symbol"],
                "fund_name": row["fund_name"],
                "group_name": row["group_name"],
                "history_days": 0,
                "history_start": None,
                "history_end": None,
                "signal_count": 0,
                "avg_return_15d": np.nan,
                "win_rate_15d": np.nan,
                "avg_drawdown_15d": np.nan,
                "avg_runup_15d": np.nan,
                "avg_proxy_turnover": np.nan,
            }
        else:
            history = build_forward_metrics(history)
            etf_frames.append(history.assign(fund_name=row["fund_name"], group_name=row["group_name"]))
            mapped = map_rule_signals_to_etf_returns(gold_signals, history)
            summary = {
                "symbol": row["symbol"],
                "fund_name": row["fund_name"],
                "group_name": row["group_name"],
                "history_days": int(len(history)),
                "history_start": history["trade_date"].min().date().isoformat(),
                "history_end": history["trade_date"].max().date().isoformat(),
                **mapped,
            }
        rows.append(summary)

    summary = pd.DataFrame(rows)
    ranked = rank_etfs(summary)
    ranked["pool_tag"] = "extended_pool"
    ranked.loc[ranked.index[:4], "pool_tag"] = "main_pool"

    output_root = Path(args.output_root)
    output_root.mkdir(parents=True, exist_ok=True)
    summary.to_csv(output_root / "gold_etf_mapping_summary.csv", index=False, encoding="utf-8-sig")
    ranked.to_csv(output_root / "gold_etf_mapping_ranked.csv", index=False, encoding="utf-8-sig")
    if etf_frames:
        pd.concat(etf_frames, ignore_index=True).to_csv(output_root / "gold_etf_mapping_histories.csv", index=False, encoding="utf-8-sig")

    payload = {
        "signal_count": int(len(gold_signals)),
        "main_pool": ranked[ranked["pool_tag"] == "main_pool"][["symbol", "fund_name", "group_name", "composite_score"]].to_dict(orient="records"),
        "top_ranked": ranked.head(11).to_dict(orient="records"),
    }
    (output_root / "summary.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(payload, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
