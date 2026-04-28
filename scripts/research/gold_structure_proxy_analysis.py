#!/usr/bin/env python
# 2026-04-28 CST: Added because the approved research route now pivots from
# missing A-share gold ETF data to free-market proxy data for gold itself.
# Purpose: test whether gold has stable 3D~30D medium-short structures and
# whether USD and oil resonance changes the edge before expanding to ETF trades.

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
import pandas as pd

try:
    import yfinance as yf
except ImportError as exc:  # pragma: no cover
    raise SystemExit("yfinance is required for gold proxy research.") from exc


DEFAULT_OUTPUT_ROOT = Path(r"E:\SM\docs\research\gold_structure_proxy_10y_20260428")
DEFAULT_START_DATE = "2016-01-01"
DEFAULT_END_DATE = "2026-04-28"
HOLDING_WINDOWS = [3, 5, 10, 15, 20, 30]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--start-date", default=DEFAULT_START_DATE)
    parser.add_argument("--end-date", default=DEFAULT_END_DATE)
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    return parser.parse_args()


def compute_time_efficiency(avg_return: float, holding_days: int) -> dict[str, float]:
    if holding_days <= 0 or pd.isna(avg_return):
        return {"return_per_day": np.nan, "annualized_equivalent": np.nan}
    return_per_day = avg_return / holding_days
    annualized_equivalent = (1.0 + avg_return) ** (252.0 / holding_days) - 1.0
    return {
        "return_per_day": float(return_per_day),
        "annualized_equivalent": float(annualized_equivalent),
    }


def fetch_single_ticker_history(ticker: str, start_date: str, end_date: str) -> pd.DataFrame:
    # 2026-04-28 CST: Added because Yahoo proxy data is the approved free-source
    # boundary for this research contract.
    # Purpose: normalize one market series into a stable daily OHLCV shape.
    frame = yf.download(
        tickers=ticker,
        start=start_date,
        end=end_date,
        auto_adjust=False,
        progress=False,
        threads=False,
    )
    if frame.empty:
        raise ValueError(f"No data returned for ticker: {ticker}")
    if isinstance(frame.columns, pd.MultiIndex):
        frame.columns = [col[0] for col in frame.columns]
    renamed = frame.rename(
        columns={
            "Open": "open",
            "High": "high",
            "Low": "low",
            "Close": "close",
            "Adj Close": "adj_close",
            "Volume": "volume",
        }
    ).reset_index()
    renamed["trade_date"] = pd.to_datetime(renamed["Date"])
    renamed["ticker"] = ticker
    keep_cols = ["trade_date", "ticker", "open", "high", "low", "close", "adj_close", "volume"]
    for column in keep_cols:
        if column not in renamed.columns:
            renamed[column] = np.nan
    return renamed[keep_cols].sort_values("trade_date").reset_index(drop=True)


def build_environment_flags(frame: pd.DataFrame, lookback_days: int = 5) -> pd.DataFrame:
    out = frame.copy()
    out["usd_ret_lb"] = out["usd_close"] / out["usd_close"].shift(lookback_days) - 1.0
    out["oil_ret_lb"] = out["oil_close"] / out["oil_close"].shift(lookback_days) - 1.0
    out["usd_regime"] = np.where(out["usd_ret_lb"] >= 0, "usd_strong", "usd_weak")
    out["oil_regime"] = np.where(out["oil_ret_lb"] >= 0, "oil_strong", "oil_weak")
    out["resonance_regime"] = np.select(
        [
            (out["usd_regime"] == "usd_strong") & (out["oil_regime"] == "oil_strong"),
            (out["usd_regime"] == "usd_strong") & (out["oil_regime"] == "oil_weak"),
            (out["usd_regime"] == "usd_weak") & (out["oil_regime"] == "oil_strong"),
            (out["usd_regime"] == "usd_weak") & (out["oil_regime"] == "oil_weak"),
        ],
        [
            "usd_up_oil_up",
            "usd_up_oil_down",
            "usd_down_oil_up",
            "usd_down_oil_down",
        ],
        default="unknown",
    )
    return out


def build_forward_metrics(frame: pd.DataFrame) -> pd.DataFrame:
    out = frame.copy()
    closes = out["gold_close"].to_numpy()
    highs = out["gold_high"].to_numpy()
    lows = out["gold_low"].to_numpy()
    for holding_days in HOLDING_WINDOWS:
        returns = np.full(len(out), np.nan)
        drawdowns = np.full(len(out), np.nan)
        runups = np.full(len(out), np.nan)
        for idx in range(len(out)):
            if idx + holding_days < len(out):
                entry_price = closes[idx]
                returns[idx] = closes[idx + holding_days] / entry_price - 1.0
                drawdowns[idx] = lows[idx + 1 : idx + holding_days + 1].min() / entry_price - 1.0
                runups[idx] = highs[idx + 1 : idx + holding_days + 1].max() / entry_price - 1.0
        out[f"future_ret_{holding_days}d"] = returns
        out[f"hold_max_drawdown_{holding_days}d"] = drawdowns
        out[f"hold_max_runup_{holding_days}d"] = runups
    return out


def build_gold_features(frame: pd.DataFrame) -> pd.DataFrame:
    out = frame.copy()
    out["ret_3d"] = out["gold_close"] / out["gold_close"].shift(3) - 1.0
    out["ret_5d"] = out["gold_close"] / out["gold_close"].shift(5) - 1.0
    out["ret_10d"] = out["gold_close"] / out["gold_close"].shift(10) - 1.0
    out["ma5"] = out["gold_close"].rolling(5).mean()
    out["ma10"] = out["gold_close"].rolling(10).mean()
    out["ma20"] = out["gold_close"].rolling(20).mean()
    out["ma30"] = out["gold_close"].rolling(30).mean()
    out["avg_volume_5d"] = out["gold_volume"].rolling(5).mean()
    out["avg_volume_20d"] = out["gold_volume"].rolling(20).mean()
    out["volume_ratio_1d_vs_20d"] = out["gold_volume"] / out["avg_volume_20d"]
    out["close_vs_ma10"] = out["gold_close"] / out["ma10"] - 1.0
    out["close_vs_ma20"] = out["gold_close"] / out["ma20"] - 1.0
    out["recent_20d_high"] = out["gold_high"].shift(1).rolling(20).max()
    out["pullback_depth_from_high"] = out["gold_close"] / out["recent_20d_high"] - 1.0
    out["trend_intact_flag"] = ((out["gold_close"] >= out["ma20"]) & (out["ma20"] >= out["ma30"])).astype(int)
    out["reclaim_short_ma_flag"] = (out["gold_close"] >= out["ma10"]).astype(int)
    out["new_high_breakout_flag"] = (out["gold_close"] > out["recent_20d_high"]).astype(int)
    out["oversold_rank_5d"] = out["ret_5d"].rank(method="first", pct=True, ascending=True)
    return out


def tag_structures(frame: pd.DataFrame) -> pd.DataFrame:
    out = frame.copy()
    out["is_trend_continuation"] = (
        (out["ret_10d"] > 0.025)
        & (out["gold_close"] > out["ma20"])
        & (out["new_high_breakout_flag"] == 1)
        & (out["volume_ratio_1d_vs_20d"] >= 0.8)
    ).astype(int)
    out["is_pullback_repair"] = (
        (out["trend_intact_flag"] == 1)
        & (out["pullback_depth_from_high"] <= -0.015)
        & (out["pullback_depth_from_high"] >= -0.07)
        & (out["close_vs_ma10"] >= -0.025)
        & (out["reclaim_short_ma_flag"] == 1)
    ).astype(int)
    out["is_mean_reversion"] = (
        (out["oversold_rank_5d"] <= 0.12)
        & (out["close_vs_ma20"] <= -0.025)
        & (out["volume_ratio_1d_vs_20d"] <= 1.3)
    ).astype(int)
    out["structure_name"] = np.select(
        [out["is_trend_continuation"] == 1, out["is_pullback_repair"] == 1, out["is_mean_reversion"] == 1],
        ["trend_continuation", "pullback_repair", "mean_reversion"],
        default="none",
    )
    return out


def summarize_holding_window(frame: pd.DataFrame, holding_days: int) -> dict[str, float | int]:
    ret_col = f"future_ret_{holding_days}d"
    dd_col = f"hold_max_drawdown_{holding_days}d"
    up_col = f"hold_max_runup_{holding_days}d"
    usable = frame[[ret_col, dd_col, up_col]].dropna().copy()
    avg_return = float(usable[ret_col].mean()) if not usable.empty else np.nan
    metrics = compute_time_efficiency(avg_return=avg_return, holding_days=holding_days)
    return {
        "holding_days": int(holding_days),
        "sample_count": int(len(usable)),
        "win_rate": float((usable[ret_col] > 0).mean()) if not usable.empty else np.nan,
        "avg_return": avg_return,
        "median_return": float(usable[ret_col].median()) if not usable.empty else np.nan,
        "avg_max_drawdown": float(usable[dd_col].mean()) if not usable.empty else np.nan,
        "avg_max_runup": float(usable[up_col].mean()) if not usable.empty else np.nan,
        "return_per_day": metrics["return_per_day"],
        "annualized_equivalent": metrics["annualized_equivalent"],
    }


def summarize_structure_by_environment(frame: pd.DataFrame) -> pd.DataFrame:
    rows: list[dict[str, float | int | str]] = []
    sample = frame[frame["structure_name"] != "none"].copy()
    grouped = sample.groupby(["structure_name", "resonance_regime"], dropna=False)
    for (structure_name, resonance_regime), subset in grouped:
        for holding_days in HOLDING_WINDOWS:
            summary = summarize_holding_window(subset, holding_days)
            summary["structure_name"] = structure_name
            summary["resonance_regime"] = resonance_regime
            rows.append(summary)
    return pd.DataFrame(rows)


def prepare_analysis_frame(start_date: str, end_date: str) -> tuple[pd.DataFrame, dict[str, int]]:
    gold = fetch_single_ticker_history("GC=F", start_date, end_date).rename(
        columns={"open": "gold_open", "high": "gold_high", "low": "gold_low", "close": "gold_close", "volume": "gold_volume"}
    )
    usd = fetch_single_ticker_history("DX-Y.NYB", start_date, end_date).rename(columns={"close": "usd_close"})
    oil = fetch_single_ticker_history("BZ=F", start_date, end_date).rename(columns={"close": "oil_close"})

    merged = gold.merge(usd[["trade_date", "usd_close"]], on="trade_date", how="left")
    merged = merged.merge(oil[["trade_date", "oil_close"]], on="trade_date", how="left")
    merged = merged.sort_values("trade_date").reset_index(drop=True)
    merged[["usd_close", "oil_close"]] = merged[["usd_close", "oil_close"]].ffill()
    merged = merged.dropna(subset=["gold_close", "gold_high", "gold_low", "usd_close", "oil_close"]).copy()

    frame = build_environment_flags(merged)
    frame = build_gold_features(frame)
    frame = build_forward_metrics(frame)
    frame = tag_structures(frame)
    counts = {
        "gold_rows": int(len(gold)),
        "usd_rows": int(len(usd)),
        "oil_rows": int(len(oil)),
        "merged_rows": int(len(frame)),
    }
    return frame, counts


def main() -> int:
    args = parse_args()
    frame, counts = prepare_analysis_frame(args.start_date, args.end_date)
    summary = summarize_structure_by_environment(frame)

    output_root = Path(args.output_root)
    output_root.mkdir(parents=True, exist_ok=True)
    frame.to_csv(output_root / "gold_proxy_flagged_events.csv", index=False, encoding="utf-8-sig")
    summary.to_csv(output_root / "gold_proxy_structure_summary.csv", index=False, encoding="utf-8-sig")

    best_by_win = (
        summary.sort_values(["structure_name", "win_rate", "annualized_equivalent"], ascending=[True, False, False])
        .groupby("structure_name")
        .head(1)
    )
    best_by_efficiency = (
        summary.sort_values(["structure_name", "return_per_day", "win_rate"], ascending=[True, False, False])
        .groupby("structure_name")
        .head(1)
    )
    payload = {
        "data_counts": counts,
        "structure_counts": frame["structure_name"].value_counts(dropna=False).to_dict(),
        "best_by_win_rate": best_by_win.to_dict(orient="records"),
        "best_by_time_efficiency": best_by_efficiency.to_dict(orient="records"),
    }
    (output_root / "summary.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(payload, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
