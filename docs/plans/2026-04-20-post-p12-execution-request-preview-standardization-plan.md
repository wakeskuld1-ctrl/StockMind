# Post-P12 Execution-Request Preview Standardization Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Upgrade the post-`P12` preview bridge so each row carries a standardized execution-request preview subset while staying preview-only.

**Architecture:** Keep the shipped preview document and row readability fields, then add one nested request-aligned preview object that mirrors the safe subset of `SecurityExecutionRecordRequest`. Verification stays focused on the dedicated preview CLI surface plus the existing portfolio-core chain.

**Tech Stack:** Rust, serde, stock CLI contract tests, Cargo test.

---

### Task 1: Add RED Assertions For Standardized Request Preview

**Files:**
- Modify: `E:/SM/tests/security_portfolio_execution_preview_cli.rs`

**Step 1: Extend the happy-path test**

- assert each preview row now exposes `execution_record_request_preview`
- assert the nested preview contains `account_id`, `decision_ref`, `execution_action`, `execution_status`, `executed_gross_pct`, and `execution_summary`

**Step 2: Extend the hold-row test**

- assert zero-delta rows map to `execution_action = hold`
- assert the nested `execution_status` stays `preview_only`

**Step 3: Run RED**

Run:

```powershell
cargo test --test security_portfolio_execution_preview_cli -- --nocapture
```

Expected:
- FAIL
- failure is caused by the missing nested standardized preview object

### Task 2: Implement The Standardized Request Preview Subset

**Files:**
- Modify: `E:/SM/src/ops/security_portfolio_execution_preview.rs`

**Step 1: Add the nested preview contract**

- define a new serializable struct for the safe subset of `SecurityExecutionRecordRequest`

**Step 2: Derive the nested preview data**

- map preview rows into request-aligned fields
- freeze `execution_status` to `preview_only`
- reuse the governed `portfolio_allocation_decision_ref` as `decision_ref`

**Step 3: Preserve compatibility**

- keep the existing preview row readability fields
- keep malformed-input rejection unchanged

### Task 3: Run GREEN Verification

**Files:**
- none

**Step 1: Run preview CLI**

```powershell
cargo test --test security_portfolio_execution_preview_cli -- --nocapture
```

Expected:
- PASS

**Step 2: Re-run focused portfolio-core coverage**

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

**Step 1: Sync the standardized preview boundary**

- record that the preview bridge now carries one nested request-aligned subset

**Step 2: Append verification truth**

- record the focused commands actually run

**Step 3: Append task journal**

- record the enhancement scope, preserved boundary, and remaining non-goals
