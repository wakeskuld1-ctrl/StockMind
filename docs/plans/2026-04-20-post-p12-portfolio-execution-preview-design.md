# Post-P12 Portfolio Execution Preview Design

## Status

- Date: 2026-04-20
- Stage: post-`P12` downstream bridge
- Approved route: `Option 1 - side-effect-free execution preview bridge`
- Upstream prerequisite: `P10 -> P11 -> P12` portfolio-core chain is already implemented, guarded, and green in this branch
- This document freezes the execution-preview contract before implementation starts

## Intent

- Goal: add one downstream bridge that turns a governed `P12` allocation decision into a side-effect-free execution preview
- Scope:
  - consume only `SecurityPortfolioAllocationDecisionDocument`
  - emit one formal preview document with per-symbol execution preview rows
  - preserve the current no-side-effect boundary
  - make the new bridge discoverable on the public stock tool surface
- Non-goals:
  - do not call `security_execution_record`
  - do not persist preview rows or execution requests
  - do not fetch runtime prices, dates, or market data
  - do not reopen `P10/P11/P12` portfolio-core math

## Two Approaches

### Approach 1: Minimal Execution Preview Bridge

- Core idea:
  - take final `P12` allocation rows
  - derive portfolio-level preview actions such as `buy`, `sell`, and `hold`
  - output one formal preview document plus compact request-preview rows
- Pros:
  - smallest downstream step after `P12`
  - side-effect boundary stays obvious
  - easy to test from the existing `P10 -> P11 -> P12` fixture chain
- Cons:
  - preview rows are still preparation artifacts, not executable facts
  - runtime-specific execution fields remain intentionally blank
- Risks:
  - if naming is sloppy, users may confuse preview rows with real execution requests

### Approach 2: Full Portfolio Rebalance Package

- Core idea:
  - aggregate `P12` into a larger rebalance package with grouped entry, trim, exit, and hold bundles
  - make that package the new downstream portfolio orchestration object
- Pros:
  - richer portfolio-level representation
  - could support later batching and approval workflows
- Cons:
  - much larger design surface
  - overlaps heavily with `P11` action summary and `P12` final decision semantics
- Risks:
  - easy to rebuild a second orchestration stage instead of a small preview bridge

## Decision

- Chosen approach: `Approach 1 - Minimal Execution Preview Bridge`
- Why:
  - it is the strongest next step that still preserves the current bounded delivery rhythm
  - it keeps `P12` as the final allocation decision while adding one explicit downstream consumer
  - it avoids reopening solver logic or runtime execution semantics
- Rejected alternative:
  - `Approach 2` is deferred because it expands scope too aggressively right after `P12` hardening

## Execution Contract

- Chosen approach:
  - add one new formal tool downstream of `P12`
  - consume only `SecurityPortfolioAllocationDecisionDocument`
  - emit preview-only rows and summary metadata
- Allowed change boundary:
  - create one new ops module for the preview bridge
  - wire it into `stock.rs`, `stock_execution_and_position_management.rs`, catalog, and dispatcher
  - add one dedicated CLI test file
  - update governance / handoff / acceptance docs if branch truth changes
- Explicit non-goals:
  - no runtime writes
  - no execution store integration
  - no live-price or actual-trade enrichment
  - no changes to `P12` request or output contracts in this round
- Route conformance checks:
  - the new request shell must contain only `portfolio_allocation_decision` plus `created_at`
  - preview output must clearly identify itself as preview, not execution fact
  - the new bridge must not produce `SecurityExecutionRecordDocument`

## Proposed Contract

### Request

- New request:
  - `SecurityPortfolioExecutionPreviewRequest`
- Required fields:
  - `portfolio_allocation_decision: SecurityPortfolioAllocationDecisionDocument`
  - `created_at: String` with the same normalized default pattern used in `P10/P11/P12`

### Output

- New output document:
  - `SecurityPortfolioExecutionPreviewDocument`
- Minimum fields:
  - preview document id, contract version, document type, generated time, account id
  - `portfolio_allocation_decision_ref`
  - per-symbol preview rows
  - preview summary counts for buy / sell / hold
  - blockers and readiness
  - preview rationale

### Preview Row Shape

- one row per final allocation symbol
- fields:
  - `symbol`
  - `current_weight_pct`
  - `target_weight_pct`
  - `weight_delta_pct`
  - `preview_action`
  - `preview_trade_gross_pct`
  - `execution_request_preview_summary`
- action rules:
  - positive delta -> `buy`
  - negative delta -> `sell`
  - zero delta -> `hold`

## Rejection Boundary

- reject account-less or malformed `P12` documents
- reject non-conserving `P12.final_target_allocations` if totals drift from the frozen `P12` summary
- reject unsupported preview actions derived from malformed deltas

## Acceptance

### Pre-Implementation Gate

- the bridge must stay preview-only
- it must consume only `P12`
- it must not bypass `P12` and consume raw portfolio-core fragments

### Pre-Completion Gate

- new CLI tests prove:
  - tool catalog includes the preview bridge
  - preview rows derive from a governed `P12` document
  - hold rows stay explicit when weight delta is zero
  - malformed `P12` inputs are rejected
- existing `P10/P11/P12` chain tests remain green

