# Nikkei ETF HGB/RF V3 Session Handoff - 2026-04-27

## Current Objective

- Preserve the 2026-04-27 Nikkei ETF research status in a resumable form.
- Separate the latest strict conclusion from degraded or incomplete refresh attempts.
- Record the user-approved routes and the key questions raised during this round so the next worker does not need the hidden chat history.

## Contract And Decision State

- Main model line:
  - HGB enhanced V3 remains the primary Nikkei risk-position model.
  - RF enhanced V3 remains a comparison / disagreement detector, not the main replacement line.
- Comparison contract:
  - "closest to packaged" and "strict common comparison" must stay separate.
  - shared yearly walk-forward is valid for fair `HGB / RF / XGBoost` ranking.
  - packaged official HGB truth is still a separate calibration surface.
- ETF execution contract:
  - signal uses `T-1 close`
  - execution uses `next open`
  - buy rule uses the lower ETF open premium proxy
  - no rebalance deadband in the main dual-ETF research line
- 2026-04-27 refresh contract:
  - if `base_position_v3` is missing for the intended date, do not pretend a new formal HGB daily artifact exists
  - if index volume proxy is zero, the date must be labeled volume-degraded
  - if ETF NAV / IOPV is missing, do not claim that low-premium ETF selection was fully refreshed

## Evidence And Verification

### Verified Facts

- `2026-04-27` Nikkei index price row exists.
- `2026-04-27` ETF price rows exist for both:
  - `159866.SZ`
  - `513520.SS`
- staged refresh root:
  - `.verification/nikkei_hgb_rf_v3_data_update_20260427_094746`
- latest strict HGB daily artifact is still:
  - `as_of_date = 2026-04-24`
  - `adjustment = -1`
  - `base_position_v3 = 37.24%`
  - `target_position_proxy = 12.24%`
- latest strict RF daily artifact is still:
  - `as_of_date = 2026-04-24`
  - `adjustment = 0`
  - `target_position_proxy = 37.24%`
- packaged ETF research explicitly confirms:
  - lower-premium buy logic exists
  - dual low-premium no-deadband variant is the main ETF execution reference

### Verified 2026-04-27 Price Layer

| Symbol | Open | Close | Note |
|---|---:|---:|---|
| `^N225` | 59,880.71 | 60,537.36 | yfinance volume still `0` |
| `159866.SZ` | 1.499 | 1.524 | price row complete |
| `513520.SS` | 2.050 | 2.048 | price row complete |

### Strict Latest Executable Conclusion

The latest formal HGB action that can be executed on `2026-04-27` is still the `2026-04-24` close signal:

- HGB says reduce
- target position proxy is `12.24%`
- this is a next-open execution instruction, not a same-day close instruction

### Unverified Or Incomplete

- `61_V3_base_2022_2026_curve.csv` has not been formally refreshed to `2026-04-27`
- `55_v3_adjustment_model_dataset.csv` still stops earlier than the intended refreshed daily scoring date
- `2026-04-27` ETF NAV / IOPV / premium refresh source is still missing
- low-premium buy refresh beyond the packaged snapshot is therefore not yet formally verified

## Open Risks And Blockers

- blocker: `v3_base_position_missing_for_intended_date`
  - impact: no formal `2026-04-27` HGB artifact can be claimed
- blocker: `latest_index_volume_proxy_zero`
  - impact: `volume_ratio60` is degraded for the latest date
- blocker: `etf_nav_or_iopv_source_missing`
  - impact: dual low-premium execution cannot be refreshed as a verified premium-based result
- delivery risk: the workspace contains many unrelated dirty files, so Git staging must stay narrow

## Truth File Routing

- Current research truth:
  - this file
  - `README.md`
  - `ALGORITHM_HANDOFF_MANUAL.md`
- Historical / broader branch truth:
  - `docs/handoff/CURRENT_STATUS.md`
  - `docs/handoff/HANDOFF_ISSUES.md`
- Verification detail:
  - `task_plan.md`
  - `findings.md`
  - `progress.md`
  - local-only staged gap report under `.verification/.../etf_execution_gap_report.json` when the same workspace is available

## Key User Decisions Recorded This Round

### Approved Directions

- keep HGB/RF V3 as the main Nikkei line
- choose Plan A when moving the weekly line
- later accept combined Plan A + Plan B for position optimization analysis
- later accept XGBoost comparison, but only under a strict common comparison route
- later reject "closest to packaged" as the only ranking criterion
- later return the main decision surface to HGB after shared walk-forward comparison
- require data-backed evaluation for any bull/position-rule change
- require future Git upload to include the day's content and the assistant-raised questions

### Questions Raised During The Round And Their Practical Answers

1. Which line should remain the main Nikkei decision line?
   - Answer: HGB enhanced V3, not RF, not XGBoost.

2. Should model swapping be prioritized over position/refill logic?
   - Answer: no; after strict common comparison, HGB stayed strongest overall.

3. Can `2026-04-27` be treated as a fully refreshed new HGB signal date?
   - Answer: no; only the price layer refreshed fully, not the formal HGB signal chain.

4. Does the ETF lower-premium rule change the `2026-04-27` action?
   - Answer: not materially for this date, because the strict HGB instruction is a sell / reduce action, not a buy-selection action.

5. What is the strict `2026-04-27` action suggestion under HGB?
   - Answer: reduce toward `12.24%` target position.

## Resume Guide

Read these first:

1. `README.md`
2. `ALGORITHM_HANDOFF_MANUAL.md`
3. this file
4. `artifacts/03_daily_hgb_rf_scoring_full_snapshot/05_latest_adjustment_artifacts_live_pre_year.csv`
5. `artifacts/02_live_like_backtest_full_snapshot/08_no_deadband_decision_summary.csv`

Then inspect:

1. `task_plan.md`
2. `findings.md`
3. `progress.md`

## Recommended Next Action

- If the goal is the next strict daily action:
  - refresh / recompute `61_V3_base_2022_2026_curve.csv` through the intended date first
- If the goal is ETF execution refinement:
  - add an approved NAV / IOPV / premium source for `159866` and `513520`
- If the goal is model research:
  - keep HGB as the primary line and work on position mapping / refill logic before attempting another model swap
