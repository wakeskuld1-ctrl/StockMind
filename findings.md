# HGB/RF V3 Data Completion Findings

## Confirmed Context
- User approved Plan A: HGB/RF V3 is the main Nikkei line.
- The old `1w positive_return` route is now historical/companion evaluation, not the main route.

## Evidence To Collect
- Packaged HGB/RF V3 daily scoring inputs and outputs.
- ETF live-like backtest inputs and outputs.
- Missing or absolute runtime path dependencies.
- Data families needed to update the package beyond 2026-04-24.

## Findings
- The daily HGB/RF scoring script is `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts/03_daily_hgb_rf_scoring_full_snapshot/daily_hgb_rf_v3_scoring.py`.
- The script still has absolute defaults under `D:\.stockmind_runtime\...`, but supports explicit `--analysis-root`, `--output-root`, `--as-of-date`, `--score-start-date`, and `--train-policy`.
- The live policy remains `live_pre_year`; `known_labels_asof` must stay diagnostic.
- The script reads at least these packaged inputs: `11_stock_history_NK225_VOL_YFINANCE.csv`, V3 base/adjustment files, top30 official weights, and `component_yfinance_raw_top30_retry`.
- ETF live-like snapshot contains ledgers, curves, summaries, rule audits, and dual low-premium variant scan outputs.
- Exact daily scoring inputs confirmed from the script:
  - `adjustment_point_analysis/55_v3_adjustment_model_dataset.csv`
  - `11_stock_history_NK225_VOL_YFINANCE.csv`
  - `adjustment_point_analysis/61_V3_base_2022_2026_curve.csv`
  - `adjustment_point_analysis/15_nikkei_top30_official_weights.csv`
  - `adjustment_point_analysis/component_yfinance_raw_top30_retry/*.csv`
- The script writes five CSV families plus policy-qualified JSON artifacts.
- Packaged coverage:
  - `11_stock_history_NK225_VOL_YFINANCE.csv`: 2016-04-25 through 2026-04-24.
  - Top30 component raw CSVs: all inspected files run 2016-04-25 through 2026-04-24.
  - `55_v3_adjustment_model_dataset.csv`: 2017-03-21 through 2026-03-27.
  - `61_V3_base_2022_2026_curve.csv`: 2022-01-04 through 2026-04-24.
  - Daily HGB/RF scores: 2026-03-30 through 2026-04-24 for both `live_pre_year` and `known_labels_asof`.
  - ETF equity curves: 2022-01-04 through 2026-04-24.
  - ETF trade ledgers: last execution date 2026-03-10, while curves continue through 2026-04-24.
- ETF backtest audit confirms lower-premium selection and non-empty ledgers, but premium is a proxy, not real-time IOPV.
- Replayed the packaged `live_pre_year` daily scorer successfully into `.verification/nikkei_hgb_rf_v3_replay_20260427`.
- Replay evidence for 2026-04-24:
  - HGB: `adjustment=-1`, target proxy `0.122423`.
  - RF: `adjustment=0`, target proxy `0.372423`.
  - Validation basis: previous Q4 out-of-sample.
- Local Python environment has `yfinance`, `pandas`, and `sklearn` available.
- Staged yfinance refresh root: `.verification/nikkei_hgb_rf_v3_data_update_20260427_094746`.
- After timezone normalization, all 31 yfinance data families have a latest local-market date of `2026-04-27`.
- Data quality blocker: the new `^N225` row has `volume = 0`, so latest `volume_ratio60` would be degraded.
- Scorer run with `--as-of-date 2026-04-27` still emitted latest artifacts for `2026-04-24`.
- Scorer blocker: `adjustment_point_analysis/61_V3_base_2022_2026_curve.csv` is still latest `2026-04-24`; without `base_position_v3` for `2026-04-27`, the scorer drops the new date.
- ETF execution gap report written to `.verification/nikkei_hgb_rf_v3_data_update_20260427_094746/etf_execution_gap_report.json`.
- ETF probe found price rows for `159866.SZ` and `513520.SS` on `2026-04-27`, but no NAV/IOPV source; therefore low-premium execution cannot be called refreshed/verified.
- Artifact filename risk: the staged scorer created files named `*_2026-04-27_adjustment.json`, but their internal `as_of_date` is still `2026-04-24`. Treat the internal field as truth; do not feed those files as 2026-04-27 artifacts.
- Latest strict action conclusion for `2026-04-27`:
  - do not claim a new formal HGB daily signal exists on `2026-04-27`
  - the latest executable strict HGB action is still the `2026-04-24` close signal executed on `2026-04-27` next open
  - that action remains `adjustment=-1`, `target_position_proxy=12.24%`
- Created session handoff packet at `docs/research/nikkei-etf-hgb-rf-v3-20260427/SESSION_HANDOFF_2026-04-27.md` to preserve current objective, blockers, strict conclusion, and the user-approved question/answer trail.
# HGB/RF V3 Findings

## 2026-04-27 - Position Optimization Analysis Contract
- User concern is no longer qualitative "仓位保守" only; decision gate is quantitative: extra return must be compared against extra drawdown, not discussed abstractly.
- Approved route is combined Plan A + Plan B:
  - Plan A evaluates alternative position mappings on the same historical signal stream.
  - Plan B attributes where low base exposure, model de-risking, and ETF execution friction created gap versus the model anchor.
- Current packaged-data ceiling for trustworthy ETF execution analysis remains `2026-04-24`.
- Any next recommendation must report both `incremental return` and `incremental max drawdown`; otherwise the conclusion is incomplete.

## 2026-04-27 - First Position Optimization Readout
- Analysis output root: `.verification/nikkei_position_optimization_analysis_20260427`
- Synthetic variant definitions for this round are evaluation-only deltas on top of packaged `baseline_target_position`; they are not deployable rules yet.
- Full-period (`2022-01-04` to `2026-04-24`) observations:
  - `base_lift_light`: about `+8.45pct` return vs baseline HGB, with about `-2.45pct` extra max drawdown.
  - `base_lift_medium`: about `+17.12pct` return vs baseline HGB, with about `-4.88pct` extra max drawdown.
  - `upside_asym_light`: about `+3.80pct` return vs baseline HGB, with drawdown almost unchanged.
- 2026 YTD (`2026-01-05` to `2026-04-24`) observations:
  - baseline HGB anchor: about `+7.94pct`
  - real ETF execution: about `+4.67pct`
  - `base_lift_light`: about `+1.00pct` return vs baseline HGB, with about `-1.26pct` extra max drawdown
  - `base_lift_medium`: about `+2.04pct` return vs baseline HGB, with about `-2.55pct` extra max drawdown
  - `upside_asym_medium`: about `+0.24pct` return vs baseline HGB, with drawdown roughly unchanged in this slice
- 2026 YTD attribution against broader benchmarks:
  - buy-and-hold minus base exposure gap is large, indicating low base exposure is the dominant upside miss.
  - HGB anchor is slightly better than base V3 in 2026 YTD, so this slice does not support the claim that HGB de-risking is the main drag.
  - ETF execution remains meaningfully below HGB anchor, confirming a separate execution-layer drag.
- De-risk event follow-up in 2026 YTD:
  - reduction-event forward `5d` mean return is positive
  - forward `10d` mean return is also positive
  - this suggests a non-trivial share of de-risk events were followed by continued upside rather than clear protection

## 2026-04-27 - Bull Regime Audit
- Output root: `.verification/nikkei_bull_regime_audit_20260427`
- Current packaged `bull` label has positive forward-return value, but not clean enough to justify blind aggression:
  - forward `5d` win rate about `59.2%`
  - forward `10d` win rate about `62.4%`
  - forward `20d` win rate about `67.1%`
  - but non-positive forward `10d` outcomes still occur in about `37.6%` of bull-labeled samples
  - forward `10d` drawdown worse than `-5%` still occurs in about `10.2%` of bull-labeled samples
- Bull-only aggressive variants improve return, but drawdown rises in a measurable and monotonic way:
  - `bull_plus_10pct`: about `+6.40pct` full-period return vs baseline with about `+2.45pct` extra max drawdown
  - `bull_plus_20pct`: about `+12.97pct` full-period return vs baseline with about `+4.88pct` extra max drawdown
  - `bull_plus_30pct`: about `+15.41pct` full-period return vs baseline with about `+6.88pct` extra max drawdown
- False-positive bull samples become more expensive under aggressive variants:
  - average extra loss over false-positive `10d` bull samples is about `-0.22pct` for `bull_plus_10pct`
  - about `-0.45pct` for `bull_plus_20pct`
  - about `-0.64pct` for `bull_plus_30pct`
- Interim implication:
  - data supports considering a mild bull-only exposure increase
  - data does not support a large aggressive jump without first tightening bull-label precision or adding secondary guards

## 2026-04-27 - Consecutive MA Breakout Study
- Output root: `.verification/nikkei_consecutive_ma_breakout_study_20260427`
- Pure consecutive breakouts above `20/50/100` day MAs are earlier than current confirmed bull segments, but average follow-through is not strong enough by itself.
- On the broad event average:
  - `20MA` with `3d` confirmation is the most usable of the simple breakout families, but even there forward `20d` average return is only around `+0.40pct`.
  - `50MA` and `100MA` simple confirmed breakouts are weaker on average, especially under `5d` confirmation.
- The mean result is heavily mixed by two very different paths:
  - `fast_bull` follow-through events show clearly positive forward returns and shallower drawdowns.
  - `slow_bull` or failed follow-through events often have flat-to-negative forward returns and worse drawdowns.
- Relative to confirmed bull-segment starts:
  - `20MA` consecutive breakout events lead bull-segment starts by about `10.7` calendar days on average.
  - `50MA` and `100MA` breakout events also tend to appear earlier than confirmed bull starts, but their reliability is not obviously better from the current simple rule.
- Interim implication:
  - simple `price above MA for N days` is a useful early-warning family
  - it is not yet a sufficient standalone bull definition
  - if used later, it likely needs an additional speed/strength filter to distinguish slow continuation from genuine stronger bull launches

## 2026-04-27 - XGBoost Same-Contract Compare
- Output root: `.verification/nikkei_xgb_model_compare_20260427`
- Approved route stayed inside the temporary experiment boundary:
  - packaged dataset for classification
  - packaged scorer live feature frame for strict backtest/latest snapshot
  - `.verification` only for outputs
- Classification replay result:
  - HGB and RF are close to packaged strict metrics, but not bit-identical.
  - XGBoost baseline is very close to HGB on `valid/test` classification quality:
    - valid accuracy about `47.96%` vs HGB `47.76%`
    - valid balanced accuracy about `43.86%` vs HGB `42.59%`
    - test accuracy about `47.61%` vs HGB `47.79%`
    - test balanced accuracy about `39.72%` vs HGB `39.87%`
- Verification-run strict backtest result:
  - XGBoost has the best return/risk among the three models under the current reproduction:
    - `xgb_depth4_lr005`: return about `+48.10%`, max drawdown about `-13.48%`, Sharpe about `1.38`
    - `rf_depth4_leaf20`: return about `+41.66%`, max drawdown about `-23.24%`, Sharpe about `1.14`
    - `hgb_l2_leaf20`: return about `+36.49%`, max drawdown about `-14.93%`, Sharpe about `1.12`
- Diagnostic full-sample backtest result:
  - XGBoost also slightly beats HGB on the temporary full-sample replay:
    - XGBoost total return about `+102.60%`
    - HGB total return about `+90.30%`
    - both have max drawdown around `-21%`
- Latest packaged comparable snapshot (`2026-04-24`):
  - HGB / RF / XGBoost all predict `adjustment = 0`
  - target position remains about `37.24%`
- Important verification limit:
  - The temporary strict backtest still undershoots packaged `60_v3_adjustment_model_strict_test_backtest.csv` materially for HGB/RF.
  - Therefore, the current XGBoost conclusion is valid for the reproduced experiment surface, but not yet strong enough to replace the packaged HGB baseline without one more replay-layer audit.

## 2026-04-27 - Package-Compatible Strict Replay Alignment
- The main strict replay mismatch was not price data and not core V3 feature selection.
- Two execution-layer differences mattered:
  1. packaged replay is not a daily close-to-close full rebalance;
  2. packaged replay does not fire on every tiny continuous target drift.
- New temporary replay path now uses:
  - next-trading-day open execution
  - sparse signal days defined by `adjustment change OR V3 base signal day`
- Result after alignment:
  - `hgb_l2_leaf20`
    - executor-aligned replay return about `+59.16%`
    - packaged truth return about `+61.43%`
    - gap narrowed to about `-2.27pct`
    - avg position now about `50.36%` vs packaged `50.47%`
  - `rf_depth4_leaf20`
    - executor-aligned replay return about `+55.56%`
    - packaged truth return about `+57.41%`
    - gap narrowed to about `-1.85pct`
    - avg position now about `52.06%` vs packaged `50.18%`
- Classification-side training difference remains small and stable:
  - HGB / RF valid/test accuracy gaps vs packaged are still within about `0.2` to `0.8` percentage points
  - so the remaining replay gap is now mostly an execution-detail tail, not a training-data misalignment
- Updated XGBoost readout under the same aligned replay surface:
  - executor-aligned strict replay return about `+57.46%`
  - max drawdown about `-13.26%`
  - Sharpe about `1.56`
- Current implication:
  - the data boundary is now aligned enough to resume model comparison
  - XGBoost no longer relies on the earlier wrong daily-rebalance executor
  - finer tuning can now be evaluated on the aligned surface instead of the old biased one

## 2026-04-27 - Shared Yearly Walk-Forward Compare
- User correction is valid: "closest to packaged" and "most rigorous common comparison" are different questions and must not share one headline conclusion.
- New strict common-surface output root: `.verification/nikkei_xgb_model_compare_20260427_wf`
- Shared walk-forward definition used in this round:
  - train on all labeled rows strictly before each test year
  - test on that calendar year's labeled rows
  - combine yearly predictions from `2022` through label-complete `2026-03-27`
  - execute on the next trading-day open
  - keep the same `base_position_v3 +/- 0.25` mapping
- Shared walk-forward result (`2022-01-04` to `2026-04-24`):
  - `HGB`: total return about `+93.90%`, max drawdown about `-12.56%`, Sharpe about `1.40`, execution count `294`
  - `RF`: total return about `+86.19%`, max drawdown about `-16.62%`, Sharpe about `1.19`, execution count `141`
  - `XGBoost`: total return about `+85.14%`, max drawdown about `-12.84%`, Sharpe about `1.28`, execution count `251`
- Interim conclusion under the shared strict surface:
  - `HGB` remains the strongest overall model
  - `XGBoost` does not beat HGB on return, Sharpe, or drawdown-adjusted quality
  - `RF` remains the weakest on return/drawdown pair
- Important limit:
  - recreated `HGB` walk-forward still does not match official `WF_HGB_adjusted_V3_2022_2026` tightly enough
  - current gap:
    - return about `+6.93pct`
    - max drawdown about `-2.99pct`
    - execution count `+55`
    - curve max absolute equity gap about `0.835M JPY`
  - implication: this shared walk-forward surface is good enough to answer "which model is stronger under one common leak-free route", but not yet good enough to replace the packaged HGB walk-forward as the published truth line
- Practical interpretation:
  - once the comparison is moved back to a strict common route, the earlier idea that "XGBoost may be better than HGB" is not supported by the current data
  - if the next goal is production relevance, higher-value work is no longer model swapping first; it is either:
    - finish official-grade walk-forward alignment, or
    - return to position-mapping / refill logic where upside capture was already shown to be the dominant gap source
## 2026-04-27 - Handoff Manual Check For Walk-Forward Replication
- The handoff manual is useful, but it does not fully define the official HGB walk-forward generator.
- What it does confirm:
  - `base_position_v3 +/- 0.25` is a research proxy, not necessarily the final execution-layer contract.
  - the formal tool boundary exists at `src/ops/security_nikkei_etf_position_signal.rs`.
  - ETF execution research uses T-1 close for signal, next available open for execution, and no rebalance deadband.
- What it does not confirm:
  - the exact official walk-forward signal-throttling rule
  - whether signal emission uses rounded target/base values, sparse trigger days, or another cadence filter
- Practical implication:
  - the manual supports using the formal Rust tool as the next source of truth for target mapping constraints
  - but the remaining `294 vs 239` execution-count gap still needs log-level cadence reverse engineering
