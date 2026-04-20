# Portfolio Core Chain Guard Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add one dedicated source guard that freezes the approved `P10 -> P11 -> P12` formal portfolio-core chain and makes its acceptance path explicit.

**Architecture:** Keep the current portfolio-core implementation unchanged and add one guard-first verification layer. The new guard should prove that `P11` consumes only formal `P10` outputs, `P12` consumes only formal `P10/P11` outputs, and the dedicated guard is recorded in acceptance and handoff truth so later sessions cannot widen the chain silently.

**Tech Stack:** Rust source-guard tests, Markdown governance docs, Cargo test.

---

### Task 1: Add RED Tests For The Dedicated Portfolio-Core Chain Guard

**Files:**
- Create: `E:/SM/tests/security_portfolio_core_chain_source_guard.rs`

**Step 1: Write the failing test file**

- add one test that reads:
  - `E:/SM/src/ops/security_portfolio_replacement_plan.rs`
  - `E:/SM/src/ops/security_portfolio_allocation_decision.rs`
  - `E:/SM/src/tools/catalog.rs`
  - `E:/SM/src/tools/dispatcher.rs`
  - `E:/SM/docs/architecture/stockmind-acceptance-checklist.md`
  - `E:/SM/docs/handoff/AI_HANDOFF.md`
  - `E:/SM/docs/governance/decision_log.md`
- assert:
  - `SecurityPortfolioReplacementPlanRequest` consumes `SecurityAccountObjectiveContractDocument` and `SecurityPortfolioCandidateSet`
  - `SecurityPortfolioAllocationDecisionRequest` consumes `SecurityAccountObjectiveContractDocument`, `SecurityPortfolioCandidateSet`, and `SecurityPortfolioReplacementPlanDocument`
  - catalog and dispatcher expose the three tools in formal order
  - acceptance and handoff docs mention the dedicated portfolio-core chain guard
  - decision log no longer leaves the chain-guard question open as an unresolved future question

**Step 2: Run RED**

Run:

```powershell
cargo test --test security_portfolio_core_chain_source_guard -- --nocapture
```

Expected:
- FAIL
- failure is caused by missing doc markers and unresolved guard registration, not by syntax errors

### Task 2: Implement The Minimum Guard And Truth Sync

**Files:**
- Modify: `E:/SM/tests/security_portfolio_core_chain_source_guard.rs`
- Modify: `E:/SM/docs/architecture/stockmind-acceptance-checklist.md`
- Modify: `E:/SM/docs/governance/decision_log.md`
- Modify: `E:/SM/docs/handoff/AI_HANDOFF.md`
- Modify: `E:/SM/docs/handoff/CURRENT_STATUS.md`

**Step 1: Keep the guard source assertions minimal and explicit**

- normalize newlines once
- assert exact request-shell markers for `P11` and `P12`
- assert exact tool markers for catalog and dispatcher
- keep failure messages specific to portfolio-core chain drift

**Step 2: Register the guard in acceptance**

- add the new test command to level-2 formal mainline acceptance
- explain what the guard proves about the formal portfolio-core chain

**Step 3: Close the decision gap**

- replace the open question about whether a chain-level source guard is needed
- add a fixed decision that the portfolio-core chain now requires its own dedicated guard after `P12`

**Step 4: Sync handoff truth**

- record the new guard in `AI_HANDOFF.md`
- add the focused verification command and observed effect in `CURRENT_STATUS.md`

### Task 3: Run GREEN Verification

**Files:**
- none

**Step 1: Run the dedicated guard**

Run:

```powershell
cargo test --test security_portfolio_core_chain_source_guard -- --nocapture
```

Expected:
- PASS

**Step 2: Re-run the portfolio-core chain tests**

Run:

```powershell
$env:CARGO_TARGET_DIR='E:\SM\target_p12_enhanced'; cargo test --test security_account_objective_contract_cli --test security_portfolio_replacement_plan_cli --test security_portfolio_allocation_decision_cli -- --nocapture
```

Expected:
- PASS

### Task 4: Journal And Closeout

**Files:**
- Modify: `E:/SM/CHANGELOG_TASK.MD`

**Step 1: Append the task journal entry**

- record the dedicated guard scope
- record that this task hardened the chain without changing portfolio-core runtime behavior
- record the verification commands and results

**Step 2: Re-check touched files**

Run:

```powershell
git status --short -- E:/SM/tests/security_portfolio_core_chain_source_guard.rs E:/SM/docs/architecture/stockmind-acceptance-checklist.md E:/SM/docs/governance/decision_log.md E:/SM/docs/handoff/AI_HANDOFF.md E:/SM/docs/handoff/CURRENT_STATUS.md E:/SM/docs/plans/2026-04-20-portfolio-core-chain-guard-plan.md E:/SM/CHANGELOG_TASK.MD
```

Expected:
- only the intended portfolio-core chain-guard files are touched for this slice
