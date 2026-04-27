# Nikkei HGB/RF V3 Position Optimization Analysis Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Evaluate whether the current HGB/RF V3 position mapping is too conservative, using approved Plan A + Plan B analysis without changing production trading logic.

**Architecture:** Keep the current HGB/RF V3 scores and existing packaged research artifacts as the signal source of truth. Run two analysis layers: Plan A compares alternative position-mapping variants at portfolio level; Plan B attributes where low exposure, de-risking, and ETF execution frictions caused missed upside or protected downside. All work stays in read-only analysis outputs until a later user-approved implementation route exists.

**Tech Stack:** Python, pandas, packaged CSV artifacts under `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts`, markdown tracking docs under repo root and `docs/plans`

---

## Execution Contract

- **Chosen approach:** Combined Plan A + Plan B.
- **Allowed change boundary:** Analysis-only work, including new plan/docs, read-only scripts or notebooks under verification/staging roots if needed, and regenerated evaluation artifacts outside production runtime roots.
- **Explicit non-goals:**
  - Do not modify production model logic, score thresholds, ETF execution rules, or packaged research artifacts.
  - Do not re-promote the old `positive_return_1w` line to primary decision logic.
  - Do not claim a new recommended position rule before comparative metrics are produced.
- **Best-practice path expected for this route:**
  - Hold signal source constant.
  - Change only position-mapping assumptions in synthetic evaluation variants.
  - Separate portfolio-level comparison from event-level attribution.
  - State degraded assumptions whenever ETF premium/NAV data is unavailable.
- **Acceptance checks:**
  - A variant matrix exists with clearly named exposure rules.
  - Each variant is evaluated on the same date range and same signal inputs.
  - Output includes both risk/return metrics and event attribution metrics.
  - Output explicitly states what is real packaged history vs. what is synthetic position remapping.

## Boundary Contract

| Boundary | Role | Source of Truth | Forbidden Reuse | Fallback Policy |
|---|---|---|---|---|
| Research artifact root | Immutable packaged signal, curve, ledger, and summary inputs | `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts` | Do not overwrite or append rows into packaged artifacts | If a needed field is missing, mark the metric unavailable rather than patching package data |
| Verification output root | Temporary analysis outputs, derived tables, and charts | `.verification/*` | Do not treat derived outputs as production signals | If a run is partial, keep outputs isolated in a dated subdirectory |
| Portfolio comparison layer | Synthetic re-mapping of existing scores/positions | Derived from packaged HGB/RF scores and base curve | Do not mix alternative mapping results back into official backtest summaries | If a mapping cannot be reproduced with current fields, exclude and document why |
| ETF execution attribution layer | Real packaged ETF ledgers and curves for execution-gap analysis | `02_live_like_backtest_full_snapshot/*` | Do not imply refreshed NAV/premium coverage beyond `2026-04-24` package horizon | Use packaged horizon only and label post-`2026-04-24` refresh as unavailable |

## Analysis Questions

1. If the current position mapping is lifted upward, how much incremental return appears, and how much extra drawdown is paid for it?
2. If upside add-on is made more aggressive while downside de-risking is left mostly intact, does the return/drawdown trade improve versus a simple base lift?
3. Which missed-upside periods came from low base exposure, which came from model de-risking, and which came from ETF execution friction?
4. Does the current system meaningfully avoid losses after de-risking, or does it often de-risk immediately before continued upside?

## Variant Matrix (Initial)

- `baseline_current`: current packaged position rule
- `base_lift_light`: raise base exposure center modestly
- `base_lift_medium`: raise base exposure center more clearly
- `upside_asym_light`: keep base near current, enlarge positive adjustment, keep negative adjustment unchanged
- `upside_asym_medium`: keep base near current, enlarge positive adjustment and slightly slow negative cut

Note: exact mapping formulas are not yet approved for production use. For this phase they are evaluation variants only and must be declared explicitly in the output.

## Metrics

### Plan A: Portfolio Comparison

- Total return
- Annualized return
- Max drawdown
- Calmar ratio
- Sharpe ratio
- Win rate
- Drawdown duration
- Upside capture ratio
- Downside protection ratio
- Incremental return vs. baseline
- Incremental max drawdown vs. baseline
- Return-per-extra-drawdown ratio

### Plan B: Event Attribution

- Low-exposure missed-upside count and cumulative missed return
- De-risk-then-rally event count
- De-risk-then-protect event count
- Post-reduction forward 5/10/20 day index return distribution
- High-score but low-position event count
- Gap contribution split:
  - base exposure gap
  - model de-risk gap
  - ETF execution friction gap

## Task Breakdown

### Task 1: Freeze analysis assumptions

**Files:**
- Modify: `task_plan.md`
- Modify: `progress.md`
- Create/Modify: `findings.md`
- Create: `docs/plans/2026-04-27-nikkei-hgb-rf-v3-position-optimization-analysis-plan.md`

**Step 1:** Record the approved route as analysis-only Plan A + Plan B.

**Step 2:** Write the evaluation boundary and non-goals into tracking docs.

**Step 3:** State the baseline date range and packaged-data ceiling explicitly.

### Task 2: Build portfolio comparison spec

**Files:**
- Read: `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts/03_daily_hgb_rf_scoring_full_snapshot/01_daily_model_scores_live_pre_year.csv`
- Read: `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts/01_adjustment_point_analysis/61_WF_HGB_adjusted_V3_2022_2026_curve.csv`
- Read: `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts/01_adjustment_point_analysis/61_V3_base_2022_2026_curve.csv`
- Read: `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts/01_adjustment_point_analysis/64_walk_forward_hgb_backtest_summary.csv`

**Step 1:** Confirm which packaged fields are sufficient to recreate baseline position and synthetic variants.

**Step 2:** Define exact comparison horizon, likely `2022-01-04` to `2026-04-24`, and a shorter `2026 YTD` slice.

**Step 3:** Define how each synthetic variant changes only the mapping layer.

**Step 4:** Define the output table schema for risk/return comparison.

### Task 3: Build event attribution spec

**Files:**
- Read: `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts/02_live_like_backtest_full_snapshot/06_dual_low_premium_no_deadband_ledger.csv`
- Read: `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts/02_live_like_backtest_full_snapshot/07_dual_low_premium_no_deadband_curve.csv`
- Read: `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts/02_live_like_backtest_full_snapshot/08_no_deadband_decision_summary.csv`
- Read: `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts/01_adjustment_point_analysis/62_WF_HGB_adjusted_V3_2022_2026_log.csv`

**Step 1:** Define event categories for missed upside and protected downside.

**Step 2:** Define the daily joined table schema linking signal date, execute date, action, score, target position, ETF chosen, premium, and equity gap.

**Step 3:** Define attribution rules that split gap into base, model, and execution layers.

**Step 4:** Define the final summary output structure for user review.

### Task 4: Prepare implementation gate

**Files:**
- Modify: `progress.md`
- Modify: `findings.md`

**Step 1:** Summarize whether current packaged fields are enough to run the analysis directly.

**Step 2:** If enough, propose the next implementation contract for running the analysis.

**Step 3:** If not enough, state the blocker precisely and stop before any production change.

## Expected Outputs

- One portfolio variant comparison table
- One attribution event table
- One concise conclusion memo answering:
  - how much extra return each variant adds
  - how much extra drawdown each variant costs
  - whether current conservatism is justified by avoided losses
  - whether the main issue is base exposure, model sensitivity, or ETF execution friction

## Verification Gate

- Do not call any variant "better" without reporting both incremental return and incremental drawdown.
- Do not interpret synthetic variant results as deployable rules without a separate approved implementation contract.
- Do not use post-`2026-04-24` ETF premium conclusions unless a premium/NAV source is added and explicitly approved.
