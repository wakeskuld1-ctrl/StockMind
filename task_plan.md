# HGB/RF/XGBoost Compare Task Plan

## Goal
Keep the Nikkei model comparison on an explicit, traceable boundary and separate "package alignment" from "shared rigorous comparison".

## Current Approved Analysis Route
- Route: Plan B for model comparison reporting.
- Scope: analysis and temporary verification scripts only; no production strategy or packaged artifact changes.
- Objective: keep two surfaces in parallel:
  - package-compatible replay for calibration against the formal research package
  - shared yearly walk-forward for fair `HGB / RF / XGBoost` comparison
- Non-goal: do not present "closest to packaged" as the same thing as "most rigorous common comparison".

## Execution Contract
- Chosen approach: keep package-compatible strict replay as a reference table, but treat the shared yearly walk-forward table as the main strict comparison surface.
- Allowed change boundary:
  - `.verification/nikkei_xgb_model_compare_20260427/*`
  - `.verification/nikkei_xgb_model_compare_20260427_wf/*`
  - `progress.md`
  - `findings.md`
  - `CHANGELOG_TASK.MD`
- Explicit non-goals:
  - no production logic changes
  - no edits to the packaged research artifact tree
  - no claim that the recreated walk-forward line is the new official truth unless it first matches the packaged HGB walk-forward tightly
- Acceptance checks:
  - output one common-surface table for `HGB / RF / XGBoost`
  - include return, max drawdown, Sharpe, average position, and execution count
  - quantify the recreated HGB walk-forward gap versus packaged `64`

## Boundary Contract
| Boundary | Role | Current Known Location | Forbidden Reuse |
|---|---|---|---|
| Packaged research artifact root | Immutable snapshot for review and calibration | `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts` | Do not rewrite or treat as this run's output root |
| Temporary compare script root | Verification-only implementation | `.verification/nikkei_xgb_model_compare_20260427/` | Do not promote directly into production or packaged research |
| Temporary output root | This round's comparison outputs | `.verification/nikkei_xgb_model_compare_20260427_wf/` | Do not mix with packaged official truth tables |
| Common feature/data root | Shared training and scoring inputs | `adjustment_point_analysis/55_v3_adjustment_model_dataset.csv` and packaged scorer inputs | Do not switch one model to a different feature family |

## Phases
- [completed] Phase 1: Inventory packaged artifacts and replay scripts.
- [completed] Phase 2: Identify exact data families required for latest daily scoring and ETF backtest.
- [completed] Phase 3: Build package-compatible strict compare outputs for HGB/RF/XGBoost.
- [completed] Phase 4: Freeze the dual-surface contract after the user corrected the comparison standard.
- [completed] Phase 5: Add a shared yearly walk-forward compare surface under `.verification`.
- [in_progress] Phase 6: Decide whether to spend effort on official-grade HGB walk-forward alignment or return to position/refill logic.

## Errors Encountered
| Error | Attempt | Resolution |
|---|---|---|
| Existing `.verification` CSV was locked | Tried to rerun into the prior compare output directory | Moved this round's outputs to `.verification/nikkei_xgb_model_compare_20260427_wf` |
| Shared walk-forward HGB did not match packaged official HGB tightly | Reused yearly expanding training plus package-compatible next-open execution | Treat the new walk-forward table as a fair common surface, but not as a replacement for packaged official truth |
