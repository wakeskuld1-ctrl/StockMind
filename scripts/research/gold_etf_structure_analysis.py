#!/usr/bin/env python
# 2026-04-28 CST: Added because the approved research pivot now needs one
# governed script to test trend, pullback, and mean-reversion structures on
# A-share gold ETFs across 3D~30D windows with time-efficiency outputs.
# Purpose: compare win rate, absolute return, drawdown, and capital-efficiency
# across medium-short holding windows before deciding whether the ETF path is
# more rule-like than A-share stock short-term trading.

from __future__ import annotations

import argparse
import json
import math
import sqlite3
from pathlib import Path

import numpy as np
import pandas as pd


DEFAULT_DB_PATH = Path(r"E:\SM\.stockmind_runtime\monster_stock_research_20260427\monster_stock_research.db")
DEFAULT_OUTPUT_ROOT = Path(r"E:\SM\docs\research\gold_etf_structure_10y_20260428")
DEFAULT_SYMBOLS = ["518880.SH", "518800.SH", "159934.SZ"]
HOLDING_WINDOWS = [3, 5, 10, 15, 20, 30]


def compute_time_efficiency(avg_return: float, holding_days: int) -> dict[str, float]:
    return_per_day = avg_return / holding_days if holding_days > 0 else 0.0
    annualized_equivalent = (1.0 + avg_return) ** (252.0 / holding_days) - 1.0 if holding_days > 0 else 0.0
    return {
        "return_per_day": float(return_per_day),
        "annualized_equivalent": float(annualized_equivalent),
    }


def summarize_holding_window(frame: pd.DataFrame, holding_days: int) -> dict[str, float | int]:
    ret_col = f"future_ret_{holding_days}d"
    dd_col = f"hold_max_drawdown_{holding_days}d"
    up_col = f"hold_max_runup_{holding_days}d"
    usable = frame[[ret_col, dd_col, up_col]].dropna().copy()
    avg_return = float(usable[ret_col].mean()) if not usable.empty else np.nan
    metrics = compute_time_efficiency(avg_return=avg_return, holding_days=holding_days) if not usable.empty else {
        "return_per_day": np.nan,
        "annualized_equivalent": np.nan,
    }
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


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--db-path", default=str(DEFAULT_DB_PATH))
    parser.add_argument("--output-root", default=str(DEFAULT_OUTPUT_ROOT))
    parser.add_argument("--symbols", default=",".join(DEFAULT_SYMBOLS))
    return parser.parse_args()


def load_price_history(db_path: Path, symbols: list[str]) -> pd.DataFrame:
    conn = sqlite3.connect(db_path)
    placeholders = ",".join(["?"] * len(symbols))
    query = f"""
    select symbol, trade_date, open, high, low, close, amount_cny, turnover_rate_pct, pct_change
    from a_share_daily_enriched
    where symbol in ({placeholders})
    order by symbol, trade_date
    """
    frame = pd.read_sql_query(query, conn, params=symbols)
    conn.close()
    frame["trade_date"] = pd.to_datetime(frame["trade_date"])
    frame = frame[(frame["close"] > 0) & (frame["high"] > 0) & (frame["low"] > 0)].copy()
    return frame.sort_values(["symbol", "trade_date"]).reset_index(drop=True)


def build_features(frame: pd.DataFrame) -> pd.DataFrame:
    grouped = frame.groupby("symbol", group_keys=False)
    frame["ret_3d"] = grouped["close"].transform(lambda s: s / s.shift(3) - 1.0)
    frame["ret_5d"] = grouped["close"].transform(lambda s: s / s.shift(5) - 1.0)
    frame["ret_10d"] = grouped["close"].transform(lambda s: s / s.shift(10) - 1.0)
    frame["ma5"] = grouped["close"].transform(lambda s: s.rolling(5).mean())
    frame["ma10"] = grouped["close"].transform(lambda s: s.rolling(10).mean())
    frame["ma20"] = grouped["close"].transform(lambda s: s.rolling(20).mean())
    frame["ma30"] = grouped["close"].transform(lambda s: s.rolling(30).mean())
    frame["avg_amt_5d"] = grouped["amount_cny"].transform(lambda s: s.rolling(5).mean())
    frame["avg_amt_20d"] = grouped["amount_cny"].transform(lambda s: s.rolling(20).mean())
    frame["turnover_rate_5d"] = grouped["turnover_rate_pct"].transform(lambda s: s.rolling(5).mean())
    frame["turnover_rate_20d"] = grouped["turnover_rate_pct"].transform(lambda s: s.rolling(20).mean())
    frame["amount_ratio_1d_vs_20d"] = frame["amount_cny"] / frame["avg_amt_20d"]
    frame["turnover_lift_1d_vs_20d"] = frame["turnover_rate_pct"] / frame["turnover_rate_20d"]
    frame["close_vs_ma20"] = frame["close"] / frame["ma20"] - 1.0
    frame["close_vs_ma10"] = frame["close"] / frame["ma10"] - 1.0
    frame["new_high_breakout_flag"] = (
        frame["close"] > grouped["high"].transform(lambda s: s.shift(1).rolling(20).max())
    ).astype(int)
    frame["trend_intact_flag"] = ((frame["close"] >= frame["ma20"]) & (frame["ma20"] >= frame["ma30"])).astype(int)
    frame["pullback_depth_from_high"] = frame["close"] / grouped["high"].transform(lambda s: s.shift(1).rolling(20).max()) - 1.0
    frame["reclaim_short_ma_flag"] = (frame["close"] >= frame["ma10"]).astype(int)
    frame["oversold_rank_5d"] = grouped["ret_5d"].transform(lambda s: s.rank(method="first", pct=True, ascending=True))

    for holding_days in HOLDING_WINDOWS:
        ret_col = f"future_ret_{holding_days}d"
        dd_col = f"hold_max_drawdown_{holding_days}d"
        up_col = f"hold_max_runup_{holding_days}d"
        returns = np.full(len(frame), np.nan)
        drawdowns = np.full(len(frame), np.nan)
        runups = np.full(len(frame), np.nan)
        for sym, df in frame.groupby("symbol"):
            idx = df.index.to_numpy()
            closes = df["close"].to_numpy()
            highs = df["high"].to_numpy()
            lows = df["low"].to_numpy()
            for local_i, global_i in enumerate(idx):
                if local_i + holding_days < len(df):
                    returns[global_i] = closes[local_i + holding_days] / closes[local_i] - 1.0
                    drawdowns[global_i] = lows[local_i + 1 : local_i + holding_days + 1].min() / closes[local_i] - 1.0
                    runups[global_i] = highs[local_i + 1 : local_i + holding_days + 1].max() / closes[local_i] - 1.0
        frame[ret_col] = returns
        frame[dd_col] = drawdowns
        frame[up_col] = runups
    return frame


def flag_structures(frame: pd.DataFrame) -> pd.DataFrame:
    out = frame.copy()
    # 2026-04-28 CST: Added because the first ETF study must separate trend,
    # pullback, and mean-reversion structures instead of blending them into one score.
    # Purpose: compare which structure has the strongest win rate and time efficiency.
    out["is_trend_continuation"] = (
        (out["ret_10d"] > 0.03)
        & (out["close"] > out["ma20"])
        & (out["new_high_breakout_flag"] == 1)
        & (out["amount_ratio_1d_vs_20d"] >= 0.9)
    ).astype(int)
    out["is_pullback_repair"] = (
        (out["trend_intact_flag"] == 1)
        & (out["pullback_depth_from_high"] <= -0.02)
        & (out["pullback_depth_from_high"] >= -0.08)
        & (out["close_vs_ma10"] >= -0.03)
        & (out["reclaim_short_ma_flag"] == 1)
    ).astype(int)
    out["is_mean_reversion"] = (
        (out["oversold_rank_5d"] <= 0.10)
        & (out["close_vs_ma20"] <= -0.03)
        & (out["amount_ratio_1d_vs_20d"] <= 1.2)
    ).astype(int)
    return out


def summarize_structure(frame: pd.DataFrame, structure_flag: str, structure_name: str) -> pd.DataFrame:
    sample = frame[frame[structure_flag] == 1].copy()
    rows = []
    for holding_days in HOLDING_WINDOWS:
        summary = summarize_holding_window(sample, holding_days)
        summary["structure_name"] = structure_name
        rows.append(summary)
    return pd.DataFrame(rows)


def main() -> int:
    args = parse_args()
    symbols = [item.strip() for item in args.symbols.split(",") if item.strip()]
    history = load_price_history(Path(args.db_path), symbols)
    featured = build_features(history)
    flagged = flag_structures(featured)

    summaries = pd.concat(
        [
            summarize_structure(flagged, "is_trend_continuation", "trend_continuation"),
            summarize_structure(flagged, "is_pullback_repair", "pullback_repair"),
            summarize_structure(flagged, "is_mean_reversion", "mean_reversion"),
        ],
        ignore_index=True,
    )

    overall_counts = {
        "trend_continuation": int(flagged["is_trend_continuation"].sum()),
        "pullback_repair": int(flagged["is_pullback_repair"].sum()),
        "mean_reversion": int(flagged["is_mean_reversion"].sum()),
    }

    output_root = Path(args.output_root)
    output_root.mkdir(parents=True, exist_ok=True)
    flagged.to_csv(output_root / "gold_etf_flagged_events.csv", index=False, encoding="utf-8-sig")
    summaries.to_csv(output_root / "gold_etf_structure_summary.csv", index=False, encoding="utf-8-sig")

    best_by_win = summaries.sort_values(["structure_name", "win_rate", "annualized_equivalent"], ascending=[True, False, False]).groupby("structure_name").head(1)
    best_by_efficiency = summaries.sort_values(["structure_name", "return_per_day", "win_rate"], ascending=[True, False, False]).groupby("structure_name").head(1)
    payload = {
        "symbols": symbols,
        "sample_counts": overall_counts,
        "best_by_win_rate": best_by_win.to_dict(orient="records"),
        "best_by_time_efficiency": best_by_efficiency.to_dict(orient="records"),
    }
    (output_root / "summary.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")

    print(json.dumps(payload, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
