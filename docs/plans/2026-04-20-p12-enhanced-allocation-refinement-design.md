# P12 Enhanced Allocation Refinement Design

## Status

- Date: 2026-04-20
- Phase: `P12`
- Approved route: `Option A - constrained second-pass allocation refinement`
- Upstream prerequisite: the minimum `P12` governed decision freeze layer is already implemented and green in this branch
- This document freezes the enhanced execution contract before any refinement code changes continue

## Intent

- Goal: upgrade `P12` from a pure decision-freeze layer into a bounded refinement layer that can improve the final governed allocation without replacing `P11`
- Scope:
  - keep `P10` and `P11` as the only legal upstream sources
  - let `P12` consume baseline `P11.target_weights`
  - allow `P12` to deploy residual cash and limited turnover slack into higher-priority governed symbols
  - emit both baseline and refined allocation truth inside the final governed decision
- Non-goals:
  - do not rewrite `P11` into a compatibility shell
  - do not accept raw upstream fragments
  - do not generate execution requests or persistence side effects
  - do not introduce iterative/global optimization machinery or external solver dependencies
- Success definition: `P12` can materially refine a baseline plan when spare max-weight capacity and turnover slack exist, while staying inside the approved governance boundary
- Delivery form: one enhanced design-freeze document plus one implementation plan, followed by TDD implementation

## Single Source Of Truth

- Current `P12` implementation:
  - `E:/SM/src/ops/security_portfolio_allocation_decision.rs`
- Current `P12` tests:
  - `E:/SM/tests/security_portfolio_allocation_decision_cli.rs`
- Upstream formal contracts:
  - `E:/SM/src/ops/security_account_objective_contract.rs`
  - `E:/SM/src/ops/security_portfolio_replacement_plan.rs`
- Current branch truth:
  - `E:/SM/docs/handoff/CURRENT_STATUS.md`

## Two Approaches

### Approach 1: Residual-Cash Priority Fill

- Core idea:
  - keep `P11.target_weights` as the baseline
  - compute priority scores from governed symbol metadata
  - use only residual cash plus remaining turnover slack to increase target weights on higher-priority symbols up to each symbol's `max_weight_pct`
- Pros:
  - bounded and auditable
  - preserves current `P11` meaning
  - easiest to test and reason about
- Cons:
  - does not rebalance by trimming weak symbols
  - enhancement power is limited when no residual cash remains
- Risks:
  - may look conservative if baseline plans are already fully invested

### Approach 2: Residual-Cash Fill Plus Controlled Trim-and-Reallocate

- Core idea:
  - start from the same baseline
  - first deploy residual cash
  - then optionally trim lower-priority symbols to fund higher-priority symbols when turnover slack still allows
- Pros:
  - stronger enhancement effect
  - can improve plans even when baseline cash is low
- Cons:
  - much easier to blur `P12` into a second solver
  - action semantics become more complex
- Risks:
  - larger regression surface
  - more difficult to explain and audit

## Decision

- Chosen approach: `Approach 1 - Residual-Cash Priority Fill`
- Why:
  - it is the strongest enhancement that still keeps `P12` bounded and obviously downstream of `P11`
  - it changes final allocation meaning without reopening the whole stage split
  - it can be made deterministic and cheap to verify
- Rejected alternative:
  - `Approach 2` is deferred because it materially increases the chance that `P12` becomes a second replacement solver
- Known tradeoffs:
  - enhancement only occurs when baseline residual cash exists and objective turnover still has slack
  - symbols already at max weight or with no turnover slack will remain unchanged

## Execution Contract

- Chosen approach:
  - keep the current `P12` tool name and public request shell
  - add one bounded refinement pass between baseline-plan validation and final allocation freeze
- Allowed change boundary:
  - modify `E:/SM/src/ops/security_portfolio_allocation_decision.rs`
  - extend `E:/SM/tests/security_portfolio_allocation_decision_cli.rs`
  - update governance/handoff docs only if accepted branch truth changes
- Explicit non-goals:
  - no changes to `P10` request/output contracts
  - no changes to `P11` request/output contracts
  - no new runtime stores, files, or external packages
  - no trim-funded second-pass reallocation in this round
- Best-practice path:
  - validate current baseline exactly as before
  - compute deterministic per-symbol priority from governed inputs
  - allocate only residual cash and only inside available turnover slack
  - stop at `max_weight_pct`
  - emit baseline-vs-refined traceability in the final `P12` document
- Route conformance checks:
  - `P12` still consumes only formal `P10` and `P11` documents
  - `P12` refinement is bounded by objective caps and symbol max weights
  - the enhanced result can be explained as baseline-plus-refinement rather than a full re-solve

## Contract

### Request Contract

- Keep existing request type:
  - `SecurityPortfolioAllocationDecisionRequest`
- No new required request fields in this round
- Compatibility rule:
  - callers using the current request shape remain valid
  - the enhanced behavior is the default behavior of the same tool

### Output Contract Changes

- Keep current primary document:
  - `SecurityPortfolioAllocationDecisionDocument`
- Extend it with:
  - `baseline_target_allocations`
  - `baseline_residual_cash_weight_pct`
  - `refinement_applied`
  - `turnover_slack_weight_pct_before_refinement`
  - `turnover_slack_weight_pct_after_refinement`
  - `allocation_refinement_summary`
- Keep existing sections unchanged where possible:
  - `final_target_allocations` now means refined final target allocations
  - current lineage, readiness, blockers, and conflict-resolution sections remain

### Refinement Rules

- Legal symbols:
  - only symbols already present in the governed candidate set
- Priority signal:
  - derive one deterministic priority score from governed expected-return and drawdown facts
  - first round formula may be `expected_annual_return_pct / max(expected_drawdown_pct, 0.01)`
- Funding source:
  - residual cash only
- Capacity limit per symbol:
  - `max_weight_pct - baseline_target_weight_pct`
- Global limits:
  - total refinement must not exceed:
    - baseline residual cash
    - available turnover slack
    - any objective hard limit after recomputation
- Stable tie-breaking:
  - higher priority score first
  - then symbol lexicographic order

## Rejection Boundary

- keep all current `P12` rejection rules
- additionally reject:
  - negative symbol spare capacity caused by malformed baseline over max-weight state
  - refined target weights that exceed any governed symbol `max_weight_pct`
  - refined turnover that exceeds `turnover_limit`
  - refined target risk budget or position count that exceeds account objective limits

## Acceptance

### Pre-Implementation Gate

- this enhanced contract must explicitly freeze:
  - funding source is residual cash only
  - refinement is bounded by turnover slack and max-weight capacity
  - `P12` remains downstream of `P11`, not a replacement for it
- reject before implementation:
  - any proposal that trims baseline targets in this round
  - any proposal that introduces raw-input bypass or iterative optimization loops

### Pre-Completion Gate

- existing `P12` tests remain green
- new tests prove:
  - no refinement occurs when no turnover slack exists
  - refinement increases higher-priority governed symbols when slack exists
  - refined targets still obey `max_weight_pct`, turnover, and objective constraints
- full repository regression remains green

## Next Skill

- Immediate next skill: `writing-plans`
- Implementation workflow after plan approval: `test-driven-development`
