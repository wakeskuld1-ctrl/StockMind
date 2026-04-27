# Nikkei Fast Bull Standalone Prototype Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Evaluate a standalone fast-bull trading prototype and compare it against the existing packaged strategy outcomes.

**Architecture:** Use the previously studied `5-day confirmed fast_bull` event family as the signal backbone. Simulate an independent execution prototype with explicit entry, add, and failure-exit behavior, then compare the result to packaged baseline/HGB/ETF outcomes. No production rule changes are allowed in this phase.

**Tech Stack:** Python, pandas, packaged artifacts, `.verification` outputs

---

## Execution Contract

- **Chosen approach:** Standalone fast-bull prototype backtest only.
- **Allowed change boundary:** Analysis-only outputs and tracking docs.
- **Explicit non-goals:**
  - Do not write the prototype back into production strategy code.
  - Do not alter packaged research artifacts.
- **Acceptance checks:**
  - Entry / add / exit rules are explicit in the output.
  - Prototype metrics are reported side-by-side with existing strategy metrics.
  - Output states sample-size limits and rule fragility if present.
