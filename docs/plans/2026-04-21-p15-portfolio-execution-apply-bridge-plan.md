# P15 Portfolio Execution Apply Bridge Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a governed `P15` bridge that consumes the formal `P14` enrichment bundle and applies only ready rows into runtime-backed execution records through the existing `security_execution_record` mainline.

**Architecture:** Add one new public stock tool that consumes only `SecurityPortfolioExecutionRequestEnrichmentDocument`, performs one bundle-level preflight validation pass, applies `ready_for_apply` rows through `security_execution_record`, and emits one batch-level apply document with explicit row statuses and runtime refs, including `apply_status = rejected` when preflight fails before the first runtime write. Keep the bridge `P14 -> execution_record` only: no broker routing, no approval workflow, no hidden rollback semantics, and no bypass of the existing execution runtime path.

**Tech Stack:** Rust, serde, stock CLI contract tests, Cargo test, existing stock runtime stores.

---

> Current blocker: implementation cannot start safely until the missing execution-context sourcing route is approved, because the current `P14` enriched rows do not yet contain enough fields to build a legal `SecurityExecutionRecordRequest`.

### Task 1: Add RED CLI Tests For The P15 Apply Bridge

**Files:**
- Create: `E:/SM/tests/security_portfolio_execution_apply_bridge_cli.rs`
- Test: `E:/SM/tests/security_portfolio_execution_request_enrichment_cli.rs`
- Test: `E:/SM/tests/security_execution_record_cli.rs`

**Step 1: Write a catalog-visibility test**

- assert `tool_catalog` includes `security_portfolio_execution_apply_bridge`

**Step 2: Write one governed happy-path apply test**

- build the upstream enrichment document through the existing `P10 -> P11 -> P12 -> preview -> P13 -> P14` fixture chain
- call the new `P15` tool with that document
- assert:
  - status is `ok`
  - document type is `security_portfolio_execution_apply_bridge`
  - `account_id` and all upstream lineage refs are preserved
  - `ready_for_apply` rows become `applied`
  - each applied row carries `execution_record_ref`
  - bundle-level counts reconcile

**Step 3: Write one hold-row semantics test**

- mutate one upstream enriched row into `non_executable_hold`
- call the new `P15` tool
- assert:
  - the hold row becomes `skipped_non_executable_hold`
  - no `execution_record_ref` is created for that row
  - `skipped_hold_count` increments

**Step 4: Write one blocked-bundle rejection test**

- mutate one upstream enriched row into `blocked`
- call the new `P15` tool
- assert:
  - status is `ok`
  - `apply_status = rejected`
  - the rejection blocker is explicit
  - no runtime-backed execution record is created

**Step 5: Write one malformed-summary rejection test**

- mutate one bundle-level summary count so it no longer matches the enriched rows
- assert:
  - status is `ok`
  - `apply_status = rejected`
  - the rejection blocker is explicit
  - no runtime-backed execution record is created

**Step 6: Write missing rejection-boundary tests**

- mutate one required lineage ref so the enrichment bundle becomes malformed
- mutate one row into an unsupported `enrichment_status`
- mutate one later ready row so `execution_apply_context.as_of_date` is blank
- assert for each case:
  - status is `ok`
  - `apply_status = rejected`
  - the rejection blocker is explicit
  - no runtime-backed execution record is created

**Step 7: Run RED**

Run:

```powershell
cargo test --test security_portfolio_execution_apply_bridge_cli -- --nocapture
```

Expected:
- FAIL
- failure is caused by the missing tool/module

### Task 2: Implement The Minimum P15 Apply Bridge

**Files:**
- Create: `E:/SM/src/ops/security_portfolio_execution_apply_bridge.rs`
- Modify: `E:/SM/src/ops/stock.rs`
- Modify: `E:/SM/src/ops/stock_execution_and_position_management.rs`
- Modify: `E:/SM/src/tools/catalog.rs`
- Modify: `E:/SM/src/tools/dispatcher.rs`
- Modify: `E:/SM/src/tools/dispatcher/stock_ops.rs`

**Step 1: Add the formal request / document / result shell**

- define the request that consumes only `SecurityPortfolioExecutionRequestEnrichmentDocument`
- define the apply row and apply-bridge document
- define the result wrapper and bounded error types

**Step 2: Implement bundle-level preflight validation**

- validate account id presence
- validate required lineage refs
- validate readiness compatibility for apply
- validate enriched-row status support
- validate bundle summary counts

**Step 3: Implement ready-row request mapping**

- derive one minimal `SecurityExecutionRecordRequest` per `ready_for_apply` row
- preserve analysis date and account identity
- keep row-level traceability back to the enrichment bundle and allocation decision

**Step 4: Implement apply execution flow**

- skip `non_executable_hold` rows explicitly
- call `security_execution_record` for each ready row
- collect `execution_record_ref` and `execution_journal_ref`
- surface row-level and bundle-level apply status without claiming hidden rollback

**Step 5: Wire public exposure**

- expose the new module on the grouped execution gateway
- add catalog and dispatcher routes

### Task 3: Freeze The Boundary And Manifest

**Files:**
- Modify: `E:/SM/tests/stock_formal_boundary_manifest_source_guard.rs`

**Step 1: Update the approved stock boundary freeze**

- add the `P15` apply bridge module to the approved stock boundary manifest
- keep the module on the execution / position-management grouping rather than introducing a new gateway

**Step 2: Run the boundary guard**

Run:

```powershell
cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
```

Expected:
- PASS

### Task 4: Run GREEN Verification

**Files:**
- none

**Step 1: Run the new CLI tests**

Run:

```powershell
cargo test --test security_portfolio_execution_apply_bridge_cli -- --nocapture
```

Expected:
- PASS

**Step 2: Re-run enrichment coverage**

Run:

```powershell
cargo test --test security_portfolio_execution_request_enrichment_cli -- --nocapture
```

Expected:
- PASS

**Step 3: Re-run execution-record coverage**

Run:

```powershell
cargo test --test security_execution_record_cli -- --nocapture
```

Expected:
- PASS

**Step 4: Re-run boundary coverage**

Run:

```powershell
cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
```

Expected:
- PASS

**Step 5: Re-run focused portfolio-core coverage**

Run:

```powershell
$env:CARGO_TARGET_DIR='E:\SM\target_p12_enhanced'; cargo test --test security_account_objective_contract_cli --test security_portfolio_replacement_plan_cli --test security_portfolio_allocation_decision_cli -- --nocapture
```

Expected:
- PASS

### Task 5: Truth Sync And Journal

**Files:**
- Modify: `E:/SM/docs/governance/contract_registry.md`
- Modify: `E:/SM/docs/governance/decision_log.md`
- Modify: `E:/SM/docs/handoff/CURRENT_STATUS.md`
- Modify: `E:/SM/docs/handoff/HANDOFF_ISSUES.md`
- Modify: `E:/SM/CHANGELOG_TASK.MD`

**Step 1: Record the new downstream bridge**

- add the `P15` apply bridge to the contract registry
- add one fixed decision that `P15` applies governed enriched requests through `security_execution_record` without becoming broker execution

**Step 2: Sync handoff truth**

- record that the branch now includes the `P15` apply bridge if implementation lands
- list the focused verification commands actually run
- state the explicit non-atomicity boundary if it remains true in the landed implementation

**Step 3: Append the task journal**

- record the `P15` apply-bridge boundary
- record the commands actually run
- record any residual risk around partial runtime apply semantics
