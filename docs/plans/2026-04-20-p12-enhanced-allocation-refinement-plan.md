# P12 Enhanced Allocation Refinement Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Upgrade `P12` with a bounded residual-cash priority-fill refinement pass while keeping it downstream of `P11`.

**Architecture:** Keep `P11` as the baseline replacement solver and let `P12` consume its output unchanged. First extend CLI tests to lock the enhanced behavior: one case with no refinement because turnover slack is exhausted, and one case where residual cash plus turnover slack allow a higher-priority symbol to scale toward its governed `max_weight_pct`. Then extend the `P12` document to expose baseline-vs-refined fields, implement the deterministic refinement pass, and re-run focused, chain, and full regression checks.

**Tech Stack:** Rust, serde, stock CLI contract tests, Cargo test.

---

### Task 1: Add RED Tests For Enhanced P12

**Files:**
- Modify: `E:/SM/tests/security_portfolio_allocation_decision_cli.rs`

**Step 1: Keep current baseline tests green-intent but add explicit no-refinement assertion**

- assert the current sample keeps `refinement_applied = false`
- assert baseline and final target allocations match when turnover slack is exhausted

**Step 2: Add one new refinement happy-path test**

- use the same governed fixture family
- increase `turnover_limit` so residual-cash deployment is legal
- assert the highest-priority symbol receives additional weight up to available slack/capacity
- assert residual cash decreases
- assert `refinement_applied = true`

**Step 3: Add one cap/constraint test**

- assert enhanced final allocation still respects symbol `max_weight_pct`
- assert turnover slack after refinement is not negative

**Step 4: Run RED**

Run:

```powershell
cargo test --test security_portfolio_allocation_decision_cli -- --nocapture
```

Expected:
- FAIL
- failure is caused by missing enhanced refinement fields/behavior

### Task 2: Implement Residual-Cash Priority Fill In P12

**Files:**
- Modify: `E:/SM/src/ops/security_portfolio_allocation_decision.rs`

**Step 1: Extend the output contract**

- add baseline allocation rows and refinement metadata fields

**Step 2: Implement deterministic priority scoring**

- derive one score from governed return/drawdown facts
- keep sorting deterministic and auditable

**Step 3: Implement the bounded refinement pass**

- compute baseline residual cash and turnover slack
- allocate only residual cash
- stop at symbol `max_weight_pct`, turnover slack, or residual cash exhaustion
- recompute constraints on refined targets

**Step 4: Update rationale and summary**

- explain whether refinement applied
- explain which symbols absorbed extra allocation and why

### Task 3: Run GREEN Verification

**Files:**
- none

**Step 1: Run focused enhanced P12 tests**

Run:

```powershell
cargo test --test security_portfolio_allocation_decision_cli -- --nocapture
```

Expected:
- PASS

**Step 2: Run portfolio-core chain tests**

Run:

```powershell
cargo test --test security_account_objective_contract_cli --test security_portfolio_replacement_plan_cli --test security_portfolio_allocation_decision_cli -- --nocapture
```

Expected:
- PASS

**Step 3: Run repository regression**

Run:

```powershell
cargo test -- --nocapture
```

Expected:
- PASS

### Task 4: Journal And Truth Sync

**Files:**
- Modify: `E:/SM/CHANGELOG_TASK.MD`
- Modify if accepted branch truth changes:
  - `E:/SM/docs/governance/contract_registry.md`
  - `E:/SM/docs/governance/decision_log.md`
  - `E:/SM/docs/handoff/AI_HANDOFF.md`
  - `E:/SM/docs/handoff/CURRENT_STATUS.md`

**Step 1: Append the task journal entry**

- record the enhanced refinement scope
- record why it stayed residual-cash-only
- record verification commands and results

**Step 2: Update truth files if needed**

- document that `P12` is now an enhanced bounded refinement layer

**Step 3: Re-check touched files**

Run:

```powershell
git status --short -- E:/SM/src/ops/security_portfolio_allocation_decision.rs E:/SM/tests/security_portfolio_allocation_decision_cli.rs E:/SM/docs/plans/2026-04-20-p12-enhanced-allocation-refinement-design.md E:/SM/docs/plans/2026-04-20-p12-enhanced-allocation-refinement-plan.md E:/SM/CHANGELOG_TASK.MD E:/SM/docs/governance/contract_registry.md E:/SM/docs/governance/decision_log.md E:/SM/docs/handoff/AI_HANDOFF.md E:/SM/docs/handoff/CURRENT_STATUS.md
```

Expected:
- only the intended enhanced-`P12` files are touched for this slice
