# Nikkei ETF Live-Like Backtest Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Produce a research backtest that converts the existing HGB V3 Nikkei signal stream into a live-like ETF execution simulation before changing the formal daily Tool.

**Architecture:** Keep the formal Rust Tool unchanged. Use the existing HGB signal log as the signal source, execute each signal on the next available ETF trading day at ETF open, and compare raw next-open execution with cost-aware and premium/deadband-filtered variants. Persist CSV outputs under `D:\.stockmind_runtime` so the temporary research artifact cannot drift into the formal Tool contract.

**Tech Stack:** Python/pandas research script executed from the shell, existing HGB signal CSV, existing ETF premium history CSV files, no public Rust boundary changes.

---

### Risk Synchronization Gate

**Risk subprocess mode:** inline-fresh-pass

**Question asked:** What artifact will drift if this boundary is added, removed, or exposed?

**Boundary items:**
- No new public Tool boundary.
- No Rust dispatcher/catalog/registry change.
- Temporary research outputs only.

**Must-sync files:**
- None for runtime code.
- This plan records the temporary research contract.

**Must-run checks:**
- Verify output CSV row counts and date ranges.
- Verify every execution date is strictly after the signal date.
- Verify filtered strategy never buys when premium is above the configured hard ceiling.
- Verify summary metrics are recomputed from daily equity curves.

**Blockers:**
- Do not claim this is the final live Tool.
- Do not use T-day close as execution price.
- Do not use rows after the signal date to generate the signal.
- Do not treat end-of-day NAV as true intraday IOPV.

### Task 1: Build Research Backtest Outputs

**Files:**
- Read: `D:\.stockmind_runtime\nikkei_current_rerun_20260426_direction_head_yfinance_10y_long_volume_behavior\analysis_exports\adjustment_point_analysis\62_WF_HGB_adjusted_V3_2022_2026_log.csv`
- Read: `D:\.stockmind_runtime\159866_nikkei_etf_premium_history_20260426.csv`
- Read: `D:\.stockmind_runtime\513520_nikkei_etf_premium_history_20260426.csv`
- Write: `D:\.stockmind_runtime\nikkei_etf_live_like_backtest_20260426\*.csv`

**Step 1: Define variants**
- `next_open_no_cost`: next ETF trading day open, no fees, no filters.
- `next_open_3bp`: next ETF trading day open, 3bp single-side fee, no filters.
- `live_filtered_3bp`: next ETF trading day open, 3bp fee, 10% minimum rebalance delta, buy ceiling 1%, hard avoid ceiling 2%, sell not blocked by premium.

**Step 2: Execute the research script**
- Build operation ledger, daily equity curve, summary metrics, and rule audit.

**Step 3: Validate outputs**
- Check execution date is greater than signal date.
- Check premium hard ceiling prevents buys.
- Check summary and operation ledgers are non-empty.

### Task 2: Report Results

**Files:**
- Read generated CSV outputs.

**Step 1: Summarize by ETF and strategy**
- Final capital.
- Total return.
- Annualized return.
- Sharpe ratio using daily returns and 252 trading days.
- Maximum drawdown.
- Trade count.
- Buy count.
- Sell count.
- Skipped buy count.

**Step 2: Explain live-readiness**
- Compare against previous close-based HGB and buy-and-hold baselines.
- Identify whether performance loss is caused by open execution, fees, or filters.
- State limitations before moving to formal Tool implementation.
