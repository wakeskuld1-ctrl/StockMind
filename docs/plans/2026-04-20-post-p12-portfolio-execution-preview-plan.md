# Post-P12 Portfolio Execution Preview Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build one side-effect-free execution preview bridge downstream of `P12`.

**Architecture:** Add a new formal tool that consumes `SecurityPortfolioAllocationDecisionDocument` and emits preview-only rows without calling runtime stores or execution writers. Keep the current portfolio-core contracts unchanged and verify the new bridge through dedicated CLI coverage plus a focused re-run of the `P10 -> P11 -> P12` chain.

**Tech Stack:** Rust, serde, stock CLI contract tests, Cargo test.

---

### Task 1: Add RED CLI Tests For The Preview Bridge

**Files:**
- Create: `E:/SM/tests/security_portfolio_execution_preview_cli.rs`

**Step 1: Write a catalog-visibility test**

- assert `tool_catalog` includes the new preview tool name

**Step 2: Write one governed happy-path test**

- build `P12` output through the existing `P10 -> P11 -> P12` fixture chain
- call the new preview tool with that document
- assert:
  - status is `ok`
  - document type is the new preview document type
  - account id and `portfolio_allocation_decision_ref` are preserved
  - positive deltas map to `buy`
  - negative deltas map to `sell`
  - zero deltas map to `hold`

**Step 3: Write one malformed-input rejection test**

- mutate the `P12` document so final target totals drift from its frozen residual cash
- assert the new tool fails with an explicit validation error

**Step 4: Run RED**

Run:

```powershell
cargo test --test security_portfolio_execution_preview_cli -- --nocapture
```

Expected:
- FAIL
- failure is caused by the missing tool/module

### Task 2: Implement The Minimum Preview Bridge

**Files:**
- Create: `E:/SM/src/ops/security_portfolio_execution_preview.rs`
- Modify: `E:/SM/src/ops/stock.rs`
- Modify: `E:/SM/src/ops/stock_execution_and_position_management.rs`
- Modify: `E:/SM/src/tools/catalog.rs`
- Modify: `E:/SM/src/tools/dispatcher.rs`
- Modify: `E:/SM/src/tools/dispatcher/stock_ops.rs`

**Step 1: Add the formal request / document / result shell**

- define a request that consumes only `SecurityPortfolioAllocationDecisionDocument`
- define preview rows and a preview document

**Step 2: Implement bounded validation**

- validate account id presence
- validate final-target total plus residual cash closure
- derive preview actions from weight deltas only

**Step 3: Implement summary output**

- emit buy / sell / hold counts
- keep rationale and readiness explicit

**Step 4: Wire public exposure**

- expose the new module on the grouped execution gateway
- add catalog and dispatcher routes

### Task 3: Run GREEN Verification

**Files:**
- none

**Step 1: Run the new CLI tests**

Run:

```powershell
cargo test --test security_portfolio_execution_preview_cli -- --nocapture
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

### Task 4: Truth Sync And Journal

**Files:**
- Modify: `E:/SM/docs/governance/contract_registry.md`
- Modify: `E:/SM/docs/governance/decision_log.md`
- Modify: `E:/SM/docs/handoff/AI_HANDOFF.md`
- Modify: `E:/SM/docs/handoff/CURRENT_STATUS.md`
- Modify: `E:/SM/CHANGELOG_TASK.MD`

**Step 1: Record the new downstream bridge**

- add the preview tool to contract registry
- add one fixed decision for the preview-only boundary

**Step 2: Sync handoff truth**

- record that the branch now includes a post-`P12` execution preview bridge
- list the focused verification command

**Step 3: Append the task journal entry**

- record the preview-only boundary
- record the commands actually run

