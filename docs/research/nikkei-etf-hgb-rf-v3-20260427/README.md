# Nikkei ETF HGB/RF V3 Research Package

## Purpose

This package preserves the full Nikkei ETF research chain used on 2026-04-26 to 2026-04-27:

- Nikkei index and top component data used for V3 adjustment-point research.
- Intermediate breakout, volume, support/resistance, regime, and downside-reduction outputs.
- HGB enhanced V3 and RF enhanced V3 model scoring artifacts.
- Live-like ETF backtest outputs for 159866, 513520, and the dual low-premium ETF execution rule.
- AI algorithm handoff notes explaining how the research evolved and how to resume it.

This is a research snapshot, not a final production model registry.

## Directory Map

| Path | Contents |
|---|---|
| `artifacts/01_training_and_intermediate_full_snapshot/` | Full Nikkei 10-year data, V3 adjustment model dataset, HGB/RF backtest comparison, component breadth, volume behavior, and walk-forward HGB outputs. |
| `artifacts/02_live_like_backtest_full_snapshot/` | ETF execution backtests using 159866 and 513520, including next-open execution, dual low-premium selection, 3bp cost, no-deadband variant, and variant scan outputs. |
| `artifacts/03_daily_hgb_rf_scoring_full_snapshot/` | Daily HGB/RF scoring script and outputs for `live_pre_year` and `known_labels_asof` policies. |
| `artifact_manifest.csv` | File inventory and SHA256 hashes for all included artifacts. |
| `ALGORITHM_HANDOFF_MANUAL.md` | Algorithm handoff manual for the next AI or engineer. |
| `SESSION_HANDOFF_2026-04-27.md` | Session-level handoff for the latest refresh status, blockers, and user-approved questions/answers. |

## Key Research Conclusions

### HGB Enhanced V3

The main walk-forward HGB enhanced V3 result is in:

`artifacts/01_training_and_intermediate_full_snapshot/analysis_exports/adjustment_point_analysis/64_walk_forward_hgb_backtest_summary.csv`

Key row:

| Strategy | Window | Final JPY | Total Return | CAGR | Sharpe | Max Drawdown | Avg Position |
|---|---|---:|---:|---:|---:|---:|---:|
| `WF_HGB_adjusted_V3_2022_2026` | 2022-01-04 to 2026-04-24 | 18,696,346.55 | 86.96% | 16.15% | 1.3653 | -9.58% | 47.88% |

This is why HGB enhanced V3 remains the primary model to understand rather than replacing it prematurely.

### Daily Scoring on 2026-04-24

The live-like daily scoring artifacts are in:

`artifacts/03_daily_hgb_rf_scoring_full_snapshot/05_latest_adjustment_artifacts_live_pre_year.csv`

| Model | Policy | As Of | Adjustment | Base Position | Target Proxy | Down Prob | Neutral Prob | Up Prob |
|---|---|---|---:|---:|---:|---:|---:|---:|
| `hgb_l2_leaf20_live` | `live_pre_year` | 2026-04-24 | -1 | 37.24% | 12.24% | 65.87% | 32.33% | 1.80% |
| `rf_depth4_leaf20_live` | `live_pre_year` | 2026-04-24 | 0 | 37.24% | 37.24% | 39.57% | 41.35% | 19.09% |

Interpretation:

- HGB treated the 2026-04-24 state as high-risk because price was far above support and volume breadth was elevated.
- RF treated the same state as neutral/hold because trend and component breadth still looked supportive.
- Neither model gave a buy/add signal on 2026-04-24.

### 2026-04-27 Refresh Boundary

The `2026-04-27` price rows for Nikkei and both ETFs are available, but the full signal chain is still incomplete:

- `base_position_v3` was not formally refreshed through `2026-04-27`
- Nikkei latest volume proxy row is still zero
- ETF NAV / IOPV premium refresh source is still missing

Practical meaning:

- the latest strict executable HGB action on `2026-04-27` is still the `2026-04-24` close signal
- that strict signal remains `adjustment = -1`, target proxy `12.24%`
- do not call `2026-04-27` a fully refreshed new HGB daily artifact yet

### Live-Like ETF Execution

The no-deadband ETF execution summary is in:

`artifacts/02_live_like_backtest_full_snapshot/08_no_deadband_decision_summary.csv`

Key result:

| Portfolio | Strategy | Final CNY | Return | Annualized | Sharpe | Max Drawdown |
|---|---|---:|---:|---:|---:|---:|
| `dual_low_premium` | `dual_low_premium_buy_no_deadband_3bp` | 1,800,728.04 | 80.07% | 14.65% | 0.9943 | -13.29% |

This variant uses both 159866 and 513520, chooses the lower premium ETF on buy days, applies 3bp cost, and removes the rebalance deadband.

## Important Policy Distinction

Use `live_pre_year` for live-like interpretation.

Do not use `known_labels_asof` as a production signal. It includes rows whose future-label horizon has already completed and is useful only for diagnostics.

Deprecated files in `artifacts/03_daily_hgb_rf_scoring_full_snapshot/`:

- `hgb_l2_leaf20_live_2026-04-24_adjustment.json`
- `rf_depth4_leaf20_live_2026-04-24_adjustment.json`

Those old filenames did not include `train_policy` and were overwritten during paired runs. Use the policy-qualified files instead:

- `hgb_l2_leaf20_live_live_pre_year_2026-04-24_adjustment.json`
- `rf_depth4_leaf20_live_live_pre_year_2026-04-24_adjustment.json`
- `hgb_l2_leaf20_live_known_labels_asof_2026-04-24_adjustment.json`
- `rf_depth4_leaf20_live_known_labels_asof_2026-04-24_adjustment.json`

## Reproduction Commands

From repo root:

```powershell
python docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\03_daily_hgb_rf_scoring_full_snapshot\daily_hgb_rf_v3_scoring.py --train-policy live_pre_year
python docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\03_daily_hgb_rf_scoring_full_snapshot\daily_hgb_rf_v3_scoring.py --train-policy known_labels_asof
```

The script currently has absolute defaults pointing at the original runtime source:

`D:\.stockmind_runtime\nikkei_current_rerun_20260426_direction_head_yfinance_10y_long_volume_behavior`

If running from the packaged Git snapshot on another machine, pass explicit paths or patch the defaults to use:

`docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\01_training_and_intermediate_full_snapshot`

## Exclusions

The A-share/HS300 experimental runtime directory was not included in this package because it is approximately 577.64MB and is not part of the current Nikkei ETF execution line.

The relevant excluded path was:

`D:\.stockmind_runtime\a_share_dynamic_hs300_backtest_20260426`

That work should be packaged separately if A-share ETF deployment becomes active again.
