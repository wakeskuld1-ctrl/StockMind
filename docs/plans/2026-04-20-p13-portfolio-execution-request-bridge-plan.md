# P13 Portfolio Execution Request Bridge Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a formal side-effect-free `P13` bridge that converts the standardized post-`P12` preview document into a portfolio execution request package.

**Architecture:** Add one new public stock tool that consumes only `SecurityPortfolioExecutionPreviewDocument` and emits a package-level request document plus per-symbol request rows. Keep the bridge preview-to-request only: no runtime writes, no execution facts, no bypass of preview or portfolio-core contracts.

**Tech Stack:** Rust, serde, stock CLI contract tests, Cargo test.

---

### Task 1: Add RED CLI Tests For The P13 Request Bridge

**Files:**
- Create: `E:/SM/tests/security_portfolio_execution_request_package_cli.rs`

**Step 1: Write a catalog-visibility test**

- assert `tool_catalog` includes `security_portfolio_execution_request_package`

**Step 2: Write one governed happy-path test**

- build the upstream preview document through the existing `P10 -> P11 -> P12 -> preview` fixture chain
- call the new `P13` tool with that document
- assert:
  - status is `ok`
  - document type is `security_portfolio_execution_request_package`
  - `account_id`, `portfolio_execution_preview_ref`, and `portfolio_allocation_decision_ref` are preserved
  - `buy` and `sell` rows map to `ready_request`
  - `hold` rows stay explicit and map to `non_executable_hold`

**Step 3: Write one malformed-preview rejection test**

- mutate the upstream preview rows to introduce unsupported action drift or broken lineage
- assert the new tool fails with an explicit validation error

**Step 4: Run RED**

Run:

```powershell
cargo test --test security_portfolio_execution_request_package_cli -- --nocapture
```

Expected:
- FAIL
- failure is caused by the missing tool/module

### Task 2: Implement The Minimum P13 Request Bridge

**Files:**
- Create: `E:/SM/src/ops/security_portfolio_execution_request_package.rs`
- Modify: `E:/SM/src/ops/stock.rs`
- Modify: `E:/SM/src/ops/stock_execution_and_position_management.rs`
- Modify: `E:/SM/src/tools/catalog.rs`
- Modify: `E:/SM/src/tools/dispatcher.rs`
- Modify: `E:/SM/src/tools/dispatcher/stock_ops.rs`

**Step 1: Add the formal request / document / result shell**

- define the request that consumes only `SecurityPortfolioExecutionPreviewDocument`
- define the request row and request package document
- define the result wrapper and bounded error types

**Step 2: Implement bounded validation**

- validate account id presence
- validate preview lineage refs
- validate preview-action support
- validate package summary counts

**Step 3: Implement row and package derivation**

- map `buy` and `sell` preview rows to `ready_request`
- map `hold` rows to explicit `non_executable_hold`
- emit package-level readiness, blockers, rationale, and summary

**Step 4: Wire public exposure**

- expose the new module on the grouped execution gateway
- add catalog and dispatcher routes

### Task 3: Run GREEN Verification

**Files:**
- none

**Step 1: Run the new CLI tests**

Run:

```powershell
cargo test --test security_portfolio_execution_request_package_cli -- --nocapture
```

Expected:
- PASS

**Step 2: Re-run downstream preview coverage**

Run:

```powershell
cargo test --test security_portfolio_execution_preview_cli -- --nocapture
```

Expected:
- PASS

**Step 3: Re-run focused portfolio-core coverage**

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
- Modify: `E:/SM/docs/handoff/CURRENT_STATUS.md`
- Modify: `E:/SM/docs/handoff/HANDOFF_ISSUES.md`
- Modify: `E:/SM/CHANGELOG_TASK.MD`

**Step 1: Record the new downstream bridge**

- add the `P13` request bridge to contract registry
- add one fixed decision that `P13` packages standardized preview into formal request documents without becoming real execution

**Step 2: Sync handoff truth**

- record that the branch now includes the `P13` request bridge if implementation lands
- list the focused verification commands actually run

**Step 3: Append the task journal**

- record the `P13` request-bridge boundary
- record the commands actually run
