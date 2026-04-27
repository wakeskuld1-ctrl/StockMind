# Nikkei HGB/RF V3 Data Completion Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete the minimum data path needed to continue Nikkei HGB/RF V3 daily scoring and ETF execution research without reviving the old `1w` direction-prediction route as the main line.

**Architecture:** Keep the packaged research snapshot immutable, stage refreshed inputs under a new runtime/output root, regenerate policy-qualified daily HGB/RF artifacts, and only then feed selected artifacts into the formal ETF position signal tool. ETF execution replay must keep price data and premium/NAV data distinct.

**Tech Stack:** Python, pandas, scikit-learn, yfinance proxy data, existing Rust `security_nikkei_etf_position_signal` tool.

---

## Boundary Contract

| Boundary | Role | Field Or Path | Single Source Of Truth | Forbidden Reuse | Fallback Policy |
|---|---|---|---|---|---|
| Packaged artifact root | Read-only baseline snapshot | `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts` | 2026-04-27 package manifest | Do not overwrite while refreshing data. | Use only for replay and diff. |
| Refreshed analysis root | New generated analysis inputs | `.verification/nikkei_hgb_rf_v3_data_update_YYYYMMDD/analysis_exports` or later approved runtime root | Explicit refreshed files and manifest | Do not pretend it is the original package. | Mark degraded if any required input remains stale. |
| Daily scoring output root | Generated HGB/RF CSV and JSON artifacts | `.verification/nikkei_hgb_rf_v3_data_update_YYYYMMDD/daily_scoring` | Policy-qualified scorer outputs | Do not use `known_labels_asof` as live signal. | `live_pre_year` only for live interpretation. |
| Index and component source | Nikkei index, volume proxy, top30 components | yfinance proxy files | yfinance pull manifest with date range and ticker list | Do not mix stale top30 files with refreshed index silently. | Fail closed for missing ticker rows unless explicitly degraded. |
| ETF execution source | 159866/513520 open/close/NAV or premium proxy | TBD ETF quote/NAV source | Separate ETF execution manifest | Do not infer NAV from ETF price alone. | Price-only refresh may proceed, but premium selection must be marked unverified. |

## Current Baseline

- Packaged daily scoring data is complete through `2026-04-24`.
- Top30 component files are complete through `2026-04-24`.
- Daily `live_pre_year` scorer replays successfully from the package.
- HGB/RF latest `2026-04-24` signal:
  - HGB: reduce risk, target proxy about `12.24%`.
  - RF: hold, target proxy about `37.24%`.
- ETF equity curves already run through `2026-04-24`; operation ledgers have last trade execution on `2026-03-10`, which is not by itself a data gap because the curve continues.

## Task 1: Refresh Market Inputs In A Staging Root

**Files:**
- Read: `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts/01_training_and_intermediate_full_snapshot/analysis_exports/11_stock_history_NK225_VOL_YFINANCE.csv`
- Read: `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts/01_training_and_intermediate_full_snapshot/analysis_exports/adjustment_point_analysis/component_yfinance_raw_top30_retry/*.csv`
- Create: `.verification/nikkei_hgb_rf_v3_data_update_YYYYMMDD/manifest.json`
- Create: `.verification/nikkei_hgb_rf_v3_data_update_YYYYMMDD/analysis_exports/...`

**Steps:**
1. Copy the packaged `analysis_exports` tree into the staging root.
2. Pull yfinance rows after `2026-04-24` for `^N225` and top30 component tickers.
3. Append only strictly newer rows.
4. Write a manifest with source, ticker, row counts, start/end date, and missing tickers.
5. Reject the refresh if index and components do not share the same latest market date.

## Task 2: Regenerate HGB/RF Daily Scoring

**Files:**
- Read: staged `analysis_exports`.
- Execute: `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts/03_daily_hgb_rf_scoring_full_snapshot/daily_hgb_rf_v3_scoring.py`
- Create: staged `daily_scoring` output root.

**Steps:**
1. Run the scorer with explicit `--analysis-root`, `--output-root`, `--train-policy live_pre_year`, and latest valid `--as-of-date`.
2. Verify latest JSON filenames include `live_pre_year`.
3. Verify latest artifact `as_of_date` equals the intended market date.
4. Record HGB/RF adjustment, target proxy, and disagreement state.

## Task 3: ETF Execution Data Check

**Files:**
- Read: packaged ETF curve and ledger CSVs.
- Create: staged ETF execution gap report.

**Steps:**
1. Confirm whether ETF price rows already cover the latest scorer date.
2. Confirm whether NAV/premium proxy rows cover the planned execution date.
3. If only ETF prices are available, mark premium selection as unverified.
4. Do not rerun dual low-premium backtest unless both ETF open prices and NAV/premium proxy are available.

## Task 4: Formal Tool Dry Run

**Files:**
- Read: latest `hgb_l2_leaf20_live_live_pre_year_<date>_adjustment.json`.
- Use: `security_nikkei_etf_position_signal`.

**Steps:**
1. Run the tool in `v3_hgb` mode with the latest HGB artifact.
2. Provide execution quotes only if ETF open/NAV data is available.
3. Verify the tool rejects missing artifact or mismatched artifact date.
4. Save result JSON under the staging root.

## Completion Gate

- Data补全 can be called complete only when:
  - The staged manifest exists.
  - Latest HGB/RF `live_pre_year` artifacts exist.
  - Missing data is listed explicitly.
  - ETF premium/NAV status is either verified or clearly degraded.
  - No package baseline files are overwritten without a separate approval.
