# Nikkei Consecutive MA Breakout Study Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Study how Nikkei behaves after consecutive breakouts above the 20-day, 50-day, and 100-day moving averages, and compare slow-bull versus fast-bull follow-through.

**Architecture:** Use packaged Nikkei daily history as immutable input. Detect breakout events with explicit consecutive-confirmation rules, then measure forward return and drawdown distributions over multiple windows. Add a simple speed classification layer so the same breakout type can be separated into slow-bull and fast-bull cases. No production rule changes are allowed in this phase.

**Tech Stack:** Python, pandas, packaged CSV artifacts, `.verification` outputs, markdown tracking docs

---

## Execution Contract

- **Chosen approach:** Event study of consecutive MA breakouts with forward-performance measurement.
- **Allowed change boundary:** Analysis-only artifacts and tracking docs.
- **Explicit non-goals:**
  - Do not change production bull/bear/range logic.
  - Do not convert any tested breakout rule into production without a later approved implementation contract.
- **Acceptance checks:**
  - Breakout event definitions are explicit and reproducible.
  - Output includes both average outcome and risk outcome.
  - Slow-bull and fast-bull are compared on the same base event family.

## Event Families

- `break_above_ma20_consecutive`
- `break_above_ma50_consecutive`
- `break_above_ma100_consecutive`

Initial rule for this round:
- event start = first day price moves from not-above to above the MA
- confirmation = stays above the same MA for at least `N` consecutive trading days
- default `N` tested: `3` and `5`

## Required Metrics

- Event count
- Forward `5d/10d/20d/40d` average return
- Forward `5d/10d/20d/40d` win rate
- Forward `5d/10d/20d/40d` average max drawdown
- Median return and median drawdown
- Large-failure ratio (forward window drawdown worse than threshold)

## Slow Bull vs Fast Bull

Initial classification for this round:
- `fast_bull`: breakout confirmation followed by stronger short-window acceleration
- `slow_bull`: breakout confirmation followed by positive but milder continuation

Exact thresholds are evaluation-only and will be declared in outputs.

## Expected Outputs

- One breakout event summary table by MA and confirmation length
- One slow-bull vs fast-bull comparison table
- One event-level detail table for manual inspection
- One concise memo on which MA breakout family is earlier and which is more reliable
