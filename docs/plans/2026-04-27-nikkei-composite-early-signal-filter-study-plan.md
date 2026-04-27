# Nikkei Composite Early Signal Filter Study Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Evaluate whether a composite early-signal filter built from short-term price strength, confirmation-window volume expansion, and breadth expansion improves 20MA breakout quality.

**Architecture:** Keep `20MA consecutive breakout` as the base trigger. Build three feature blocks: price strength, volume strength, and breadth expansion. Test simple AND-style composite filters and compare them against the unfiltered breakout set and single-volume filters on later 180-day path quality. No production rule changes are allowed in this phase.

**Tech Stack:** Python, pandas, packaged CSV artifacts, `.verification` outputs

---

## Execution Contract

- **Chosen approach:** Composite event-study evaluation on immutable breakout events.
- **Allowed change boundary:** Analysis-only outputs and tracking docs.
- **Explicit non-goals:**
  - Do not modify production bull logic.
  - Do not promote any tested composite filter to production without a separate approved rule-design step.
- **Acceptance checks:**
  - Feature definitions are explicit.
  - Composite filters are compared to both baseline and single-factor filters.
  - Output includes path-quality and sample-size tradeoffs.

## Feature Blocks

### Price Strength
- confirmation-window cumulative return
- confirmation-end distance above MA20
- confirmation-end breakout against recent high

### Volume Strength
- confirmation-window average volume ratio
- confirmation-window max volume ratio
- confirmation-end price-volume impulse

### Breadth Expansion
- component breadth level near signal date
- breadth improvement across confirmation window
- breakout/breadth participation proxies from packaged daily features when available

## Candidate Composite Filters

- `price_only`
- `price_and_volume`
- `price_and_breadth`
- `volume_and_breadth`
- `price_volume_breadth`

## Expected Outputs

- One composite-filter performance table
- One best-candidate table with sample-size tradeoff
- One concise memo on whether composite early-signal filtering materially improves breakout quality
