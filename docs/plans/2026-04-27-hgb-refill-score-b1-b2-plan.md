# HGB Refill Score B1/B2 Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Design a refill-position framework for HGB that uses a bounded refill score in B1, then optimizes coefficients in B2 without changing the score structure.

**Architecture:** Keep HGB as the direction and risk-control backbone. Build a refill score that activates only after de-risking and maps recovery evidence into incremental position restoration. B1 fixes score components, penalties, and position bands. B2 optimizes coefficients and thresholds inside that fixed structure using backtest evidence.

**Tech Stack:** Python, pandas, packaged research artifacts, `.verification` outputs, markdown design docs

---

## Execution Contract

- **Chosen approach:** `B1 manual structure` first, then `B2 coefficient optimization`.
- **Allowed change boundary:** Analysis-only design and backtest prototypes. No production rule changes yet.
- **Explicit non-goals:**
  - Do not redesign HGB signal logic.
  - Do not reopen bull/bear regime redesign as the main route.
  - Do not let B2 change feature families or break monotonic risk-control boundaries.
- **Best-practice path:**
  - HGB keeps control of baseline direction and de-risking.
  - Refill score only controls how fast reduced exposure is restored.
  - B1 defines score structure and position mapping bands.
  - B2 only tunes numeric coefficients, thresholds, and caps inside B1.
- **Acceptance checks:**
  - Score terms are explicit and interpretable.
  - Position mapping is bounded and monotonic.
  - Risk penalties can override aggressive refill.
  - B2 parameter search space is defined before optimization starts.

## Problem Definition

Current evidence suggests the main weakness is not de-risking itself, but:

1. after de-risking, refill is too slow
2. trial position to full add-on lacks a smooth path
3. early refill can fail if recovery is fake

So the target is not "reduce less", but:

**rebuild exposure faster when recovery is real, and cut refill quickly when recovery fails**

## B1: Fixed Score Structure

### Activation Gate

Refill score is only evaluated when all of the following hold:

- HGB baseline position is below the user's intended full-risk ceiling
- recent strategy state includes a prior de-risk / low-exposure phase
- no hard-stop failure condition is active

This prevents refill logic from replacing the main HGB strategy.

### Positive Score Blocks

#### 1. HGB Repair

- `hgb_adjustment_repair_1`: HGB state improves from `-1` to `0`
- `hgb_adjustment_repair_2`: HGB state improves from `0` to `+1`

Purpose: reward internal model repair before aggressive refill.

#### 2. Price Repair

- price reclaims `MA20`
- price reclaims `MA50`
- distance above `MA50` improves
- distance above recent local pivot improves

Purpose: confirm that price is rebuilding structure, not only bouncing intraday.

#### 3. Breakout Strength

- re-break `prior_high20`
- re-break `prior_high60`
- breakout persistence over short confirmation window

Purpose: distinguish real continuation from weak rebound.

#### 4. Breadth Repair

- breadth level above threshold
- breadth `3d/5d` improvement
- weighted breadth not deteriorating

Purpose: confirm that recovery is not driven by a narrow subset only.

### Negative Score Blocks

#### 1. Failure Risk

- consecutive closes back below `MA50`
- failed reclaim after refill
- repeated break-and-fail behavior

Purpose: stop refill from snowballing into a false recovery.

#### 2. Volatility / Disorder Penalty

- abnormal short-term downside volatility
- recovery with unstable price behavior

Purpose: slow refill when path quality is poor.

#### 3. Breadth Relapse

- breadth reverses lower immediately after recovery
- diffusion fails to continue

Purpose: punish weak internal participation.

## B1 Position Mapping

The refill score maps to **incremental refill**, not total strategy position.

### Mapping Bands

- `score <= S0`: no refill
- `S0 < score <= S1`: trial refill band, roughly `+10% ~ +25%`
- `S1 < score <= S2`: recovery refill band, roughly `+25% ~ +45%`
- `S2 < score <= S3`: continuation refill band, roughly `+45% ~ +70%`
- `score > S3`: aggressive refill band, roughly `+70% ~ +95%`

### Hard Boundaries

- refill must never push total position above configured ceiling
- one-step refill increase per rebalance is capped
- hard-stop failure can override score and cut refill directly

These boundaries are fixed in B1 and cannot be broken by B2.

## B2: Optimization Scope

B2 may optimize:

- coefficient weights for each score term
- score thresholds `S0/S1/S2/S3`
- decay window for repair signals
- penalty weights
- refill cap inside allowed band

B2 may NOT optimize:

- feature families
- monotonic direction of a signal
- hard failure override existence
- max total position ceiling

## Candidate B1 Feature Set For First Pass

- HGB repair state change
- close above `MA20`
- close above `MA50`
- break `prior_high20`
- break `prior_high60`
- breadth level
- breadth `3d` change
- consecutive closes below `MA50` penalty

This first pass is intentionally compact. More features can be added only if they show incremental value.

## Verification Metrics

### Portfolio Metrics

- total return
- annualized return
- max drawdown
- Sharpe
- Calmar
- return delta vs HGB baseline
- drawdown delta vs HGB baseline

### Refill-Specific Metrics

- time from de-risk to first refill
- time from trial refill to higher refill band
- refill failure rate
- missed-upside reduction after de-risk
- extra drawdown caused by refill

## Expected Outputs

- one B1 score-definition table
- one B1 position-mapping table
- one B2 optimization-boundary table
- one implementation-ready backtest contract for the next step
