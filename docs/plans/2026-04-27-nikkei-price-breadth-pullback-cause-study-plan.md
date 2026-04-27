# Nikkei Price And Breadth Pullback Cause Study Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Diagnose why `price_and_breadth` early bull candidate signals still show weak 30-day path quality, with focus on whether pullbacks come from confirmation retests or higher-level resistance / consolidation.

**Architecture:** Use the existing `price_and_breadth` event set as immutable input. For each event, compare price location versus MA20/50/100/200 and recent resistance levels, then inspect the following 30-day path. Classify weak short-term outcomes into likely retest / incomplete breakout / higher-level consolidation buckets. No production rule changes are allowed in this phase.

**Tech Stack:** Python, pandas, packaged daily history, `.verification` outputs

---

## Execution Contract

- **Chosen approach:** Event-level decomposition of weak 30-day outcomes for `price_and_breadth`.
- **Allowed change boundary:** Analysis-only outputs and tracking docs.
- **Explicit non-goals:**
  - Do not change the filter definition.
  - Do not promote any new MA hierarchy rule to production in this phase.
- **Acceptance checks:**
  - Each event is labeled with MA hierarchy state at trigger time.
  - Weak 30-day outcomes are grouped into interpretable cause buckets.
  - Output distinguishes simple MA20 breakout from broader multi-MA breakout context.

## Required Comparisons

- price vs `MA20 / MA50 / MA100 / MA200`
- `MA20 vs MA50`, `MA50 vs MA100`, `MA100 vs MA200`
- price vs recent `20d / 60d / 120d` highs
- next-30-day retest depth and recovery behavior

## Expected Outputs

- One event detail table with MA hierarchy fields
- One weak-30-day sample table
- One cause summary table
- One concise memo on whether short-term weakness is mostly retest behavior or higher-level resistance/consolidation
