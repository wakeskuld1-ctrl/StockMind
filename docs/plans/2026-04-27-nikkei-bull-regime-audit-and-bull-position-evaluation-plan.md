# Nikkei Bull Regime Audit And Bull Position Evaluation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Determine, with explicit metrics, whether the current bull-regime label is reliable enough and whether more aggressive bull-market positioning is justified.

**Architecture:** Keep packaged history and current regime labels as immutable inputs. First audit the predictive usefulness and error cost of existing `bull` labels. Then evaluate synthetic "bull-only more aggressive" position variants on the same historical sample. No production logic changes are allowed in this phase.

**Tech Stack:** Python, pandas, packaged CSV artifacts under `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts`, markdown tracking docs, `.verification` outputs

---

## Execution Contract

- **Chosen approach:** Audit bull-regime quality first, then evaluate bull-only aggressive position variants.
- **Allowed change boundary:** Analysis-only outputs, tracking docs, and temporary verification artifacts.
- **Explicit non-goals:**
  - Do not change production bull/bear/range rules.
  - Do not change production position rules.
  - Do not recommend rule changes without explicit metric results.
- **Acceptance checks:**
  - Bull-regime quality metrics are reported with clear sample windows.
  - Bull-only aggressive variants are evaluated against the same baseline history.
  - Decision output includes both benefit metrics and misclassification-cost metrics.

## Core Questions

1. Are current `bull` labels followed by meaningfully positive forward returns often enough to be trusted?
2. How costly are false-positive bull labels?
3. If bull periods are given higher exposure, does the return increase justify the added drawdown?

## Required Metrics

### Bull Regime Quality
- Bull sample count and coverage ratio
- Bull forward `5d/10d/20d` average return
- Bull forward `5d/10d/20d` win rate
- Bull-period realized max drawdown
- Bull false-positive count:
  - Bull label followed by non-positive forward return
  - Bull label followed by drawdown worse than threshold
- Bull false-positive loss severity

### Bull-Only Aggressive Position Evaluation
- Total return delta vs baseline
- Bull-sample return delta vs baseline
- Max drawdown delta vs baseline
- Calmar delta vs baseline
- Sharpe delta vs baseline
- Return-per-extra-drawdown ratio
- False-positive bull sample extra loss vs baseline

## Variant Matrix (Evaluation Only)

- `baseline_current`
- `bull_plus_10pct`
- `bull_plus_20pct`
- `bull_plus_30pct`

Rule: only dates whose packaged regime is `bull` get the extra exposure. All other dates keep the packaged baseline target unchanged.

## Expected Outputs

- One bull-regime quality summary table
- One bull false-positive severity table
- One bull-only aggressive variant comparison table
- One final decision memo stating whether data supports changing rules
