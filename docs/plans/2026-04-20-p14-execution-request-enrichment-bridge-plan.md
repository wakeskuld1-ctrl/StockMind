# P14 Execution Request Enrichment Bridge Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a formal side-effect-free `P14` bridge that enriches the `P13` request package into an execution-record-aligned request bundle without invoking runtime execution.

**Architecture:** Add one new public stock tool that consumes only `SecurityPortfolioExecutionRequestPackageDocument` and emits a bundle of enriched execution request rows with preserved lineage, deterministic analysis-date enrichment, and explicit hold-row semantics. Keep the bridge request-to-enrichment only: no runtime writes, no execution facts, no bypass of `P13`.

**Tech Stack:** Rust, serde, stock CLI contract tests, Cargo test.

---

### Task 1: Add RED CLI Tests For The P14 Enrichment Bridge

**Files:**
- Create: `E:/SM/tests/security_portfolio_execution_request_enrichment_cli.rs`

**Step 1: Write a catalog-visibility test**

- assert `tool_catalog` includes `security_portfolio_execution_request_enrichment`

**Step 2: Write one governed happy-path test**

- build the upstream request package through the existing `P10 -> P11 -> P12 -> preview -> P13` fixture chain
- call the new `P14` tool with that document and one `analysis_date`
- assert:
  - status is `ok`
  - document type is `security_portfolio_execution_request_enrichment`
  - `account_id`, `portfolio_execution_request_package_ref`, `portfolio_execution_preview_ref`, and `portfolio_allocation_decision_ref` are preserved
  - `ready_request` rows become `ready_for_apply`
  - `execution_action`, `execution_status`, `executed_gross_pct`, and `execution_summary` are populated deterministically

**Step 3: Write one hold-row semantics test**

- mutate one upstream request row into `non_executable_hold`
- assert the enriched row remains explicit and does not become `ready_for_apply`

**Step 4: Write one malformed-input rejection test**

- break one required lineage ref or blank out `analysis_date`
- assert the new tool fails with an explicit validation error

**Step 5: Run RED**

Run:

```powershell
cargo test --test security_portfolio_execution_request_enrichment_cli -- --nocapture
```

Expected:
- FAIL
- failure is caused by the missing tool/module

### Task 2: Implement The Minimum P14 Enrichment Bridge

**Files:**
- Create: `E:/SM/src/ops/security_portfolio_execution_request_enrichment.rs`
- Modify: `E:/SM/src/ops/stock.rs`
- Modify: `E:/SM/src/ops/stock_execution_and_position_management.rs`
- Modify: `E:/SM/src/tools/catalog.rs`
- Modify: `E:/SM/src/tools/dispatcher.rs`
- Modify: `E:/SM/src/tools/dispatcher/stock_ops.rs`

**Step 1: Add the formal request / document / result shell**

- define the request that consumes only `SecurityPortfolioExecutionRequestPackageDocument`
- define the enriched row and enrichment bundle document
- define the result wrapper and bounded error types

**Step 2: Implement bounded validation**

- validate account id presence
- validate package lineage refs
- validate request action / status support
- validate non-blank `analysis_date`
- validate bundle summary counts

**Step 3: Implement row and bundle derivation**

- map `ready_request` rows into `ready_for_apply`
- populate deterministic execution-record-aligned enrichment fields
- keep `non_executable_hold` rows explicit
- emit bundle-level readiness, blockers, rationale, and summary

**Step 4: Wire public exposure**

- expose the new module on the grouped execution gateway
- add catalog and dispatcher routes

### Task 3: Run GREEN Verification

**Files:**
- none

**Step 1: Run the new CLI tests**

Run:

```powershell
cargo test --test security_portfolio_execution_request_enrichment_cli -- --nocapture
```

Expected:
- PASS

**Step 2: Re-run downstream request-bridge coverage**

Run:

```powershell
cargo test --test security_portfolio_execution_request_package_cli -- --nocapture
```

Expected:
- PASS

**Step 3: Re-run preview coverage**

Run:

```powershell
cargo test --test security_portfolio_execution_preview_cli -- --nocapture
```

Expected:
- PASS

**Step 4: Re-run focused portfolio-core coverage**

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

- add the `P14` enrichment bridge to contract registry
- add one fixed decision that `P14` enriches request packages without becoming real execution

**Step 2: Sync handoff truth**

- record that the branch now includes the `P14` enrichment bridge if implementation lands
- list the focused verification commands actually run

**Step 3: Append the task journal**

- record the `P14` enrichment-bridge boundary
- record the commands actually run
