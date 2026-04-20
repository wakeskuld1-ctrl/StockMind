# P12 Governed Portfolio Allocation Decision Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement the minimum `P12` governed portfolio allocation decision layer without expanding it into a second solver.

**Architecture:** Reuse the already-accepted `P10` and `P11` formal contracts as the only legal upstream inputs. First add CLI tests that prove catalog visibility, happy-path decision freeze, and hard-fail drift/constraint boundaries. Then add the new `security_portfolio_allocation_decision` module plus stock boundary, dispatcher, and catalog wiring. Keep the implementation deterministic: validate, freeze, and summarize; do not optimize or emit execution side effects.

**Tech Stack:** Rust, serde, thiserror, stock CLI contract tests, Cargo test.

---

### Task 1: Add RED CLI Tests For P12

**Files:**
- Create: `E:/SM/tests/security_portfolio_allocation_decision_cli.rs`

**Step 1: Write the failing catalog-discovery test**

- assert `tool_catalog` contains `security_portfolio_allocation_decision`

**Step 2: Write the failing happy-path decision-freeze test**

- build `P10` outputs
- build `P11` output from them
- call `security_portfolio_allocation_decision`
- assert final allocation, residual cash, readiness, and lineage fields exist

**Step 3: Write the failing hard-boundary tests**

- cross-account drift
- target-weight non-conservation or capital-migration contradiction
- objective-limit mismatch
- candidate-set symbol drift

**Step 4: Run RED**

Run:

```powershell
cargo test --test security_portfolio_allocation_decision_cli -- --nocapture
```

Expected:
- FAIL
- failure is caused by missing `P12` tool wiring and implementation

### Task 2: Implement The Minimum P12 Contract

**Files:**
- Create: `E:/SM/src/ops/security_portfolio_allocation_decision.rs`

**Step 1: Add request, output, helper row, and error contracts**

- define the public request shell that consumes only `P10` and `P11` documents
- define the final decision document and named result wrapper
- define explicit hard-fail errors for account drift, symbol drift, constraint mismatch, and non-conservation

**Step 2: Implement the deterministic decision freeze**

- validate upstream identity and closure
- recompute key checks from governed inputs
- freeze final allocation rows from `P11`
- emit readiness, blockers, constraint checks, rationale, and summary

**Step 3: Keep comments aligned with project rules**

- every new or modified code block keeps nearby English comments for reason, purpose, and time

### Task 3: Wire P12 Onto The Formal Stock Surface

**Files:**
- Modify: `E:/SM/src/ops/stock.rs`
- Modify: `E:/SM/src/ops/stock_execution_and_position_management.rs`
- Modify: `E:/SM/src/tools/catalog.rs`
- Modify: `E:/SM/src/tools/dispatcher.rs`
- Modify: `E:/SM/src/tools/dispatcher/stock_ops.rs`

**Step 1: Expose the new module on the stock boundary**

- add the new `P12` module next to `P10` and `P11`

**Step 2: Add grouped-gateway export**

- expose the module through `stock_execution_and_position_management`

**Step 3: Wire catalog and dispatcher**

- add tool name to catalog
- add `dispatcher.rs` branch
- add `stock_ops.rs` request parsing and routing

### Task 4: Run GREEN Verification

**Files:**
- none

**Step 1: Run focused P12 tests**

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

### Task 5: Journal And Close The Slice

**Files:**
- Modify: `E:/SM/CHANGELOG_TASK.MD`

**Step 1: Append the task journal entry**

- record P12 implementation scope
- record why it stayed a decision-freeze layer
- record verification commands and results

**Step 2: Re-check touched files**

Run:

```powershell
git status --short -- E:/SM/src/ops/security_portfolio_allocation_decision.rs E:/SM/src/ops/stock.rs E:/SM/src/ops/stock_execution_and_position_management.rs E:/SM/src/tools/catalog.rs E:/SM/src/tools/dispatcher.rs E:/SM/src/tools/dispatcher/stock_ops.rs E:/SM/tests/security_portfolio_allocation_decision_cli.rs E:/SM/docs/plans/2026-04-20-p12-governed-portfolio-allocation-decision-plan.md E:/SM/CHANGELOG_TASK.MD
```

Expected:
- only the intended `P12` implementation, plan, and journal files are touched for this slice
