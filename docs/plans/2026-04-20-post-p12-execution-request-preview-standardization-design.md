# Post-P12 Execution-Request Preview Standardization Design

## Status

- Date: 2026-04-20
- Stage: post-`P12` preview enhancement
- Approved route: `Option 1 - standardize preview rows as execution-request preview`
- Upstream prerequisite: the minimal post-`P12` preview-only bridge already exists in this branch

## Intent

- Goal: upgrade the existing post-`P12` preview rows into a standardized execution-request preview shape
- Scope:
  - keep the current preview-only bridge
  - preserve the existing human-readable preview fields
  - add one explicit request-aligned preview object per symbol
  - freeze the alignment boundary against future real execution work
- Non-goals:
  - do not call `security_execution_record`
  - do not persist execution requests
  - do not rename this tool into real execution
  - do not reopen `P10/P11/P12` portfolio-core logic

## Two Approaches

### Option 1: Add One Nested Standardized Execution-Request Preview Object

- Core idea:
  - keep the current preview row fields for readability
  - add one nested object that aligns to the safe subset of `SecurityExecutionRecordRequest`
- Pros:
  - smallest delta on top of the shipped preview bridge
  - future execution bridge can reuse a stable preview-aligned shell
  - avoids breaking the existing preview row readability
- Cons:
  - row shape becomes wider
  - one request-aligned object now coexists with legacy preview summary text
- Risks:
  - if naming is weak, callers may confuse request preview with executable request

### Option 2: Replace The Row Shape Entirely With One Request-Like Row

- Core idea:
  - remove most current preview row fields
  - emit only one request-like row contract
- Pros:
  - strongest schema purity
  - least duplication long-term
- Cons:
  - higher compatibility risk for current preview consumers
  - loses the currently explicit target/current/delta readability unless rebuilt elsewhere
- Risks:
  - easy to create avoidable downstream churn for little immediate value

## Decision

- Chosen approach: `Option 1`
- Why:
  - it upgrades the bridge without breaking the current preview interpretation surface
  - it creates one explicit migration path toward a later real execution bridge
  - it keeps the preview-only boundary highly visible

## Execution Contract

- Allowed change boundary:
  - modify `src/ops/security_portfolio_execution_preview.rs`
  - modify `tests/security_portfolio_execution_preview_cli.rs`
  - sync governance / handoff truth only if branch truth changes
- Explicit non-goals:
  - no runtime write
  - no execution store
  - no change to `security_execution_record`
  - no unrelated training or fixture cleanup
- Route conformance checks:
  - every preview row still exposes `symbol/current/target/delta/action`
  - every preview row gains one nested request-aligned preview object
  - the nested object remains clearly marked as preview-only
  - the tool still emits preview documents, not execution documents

## Proposed Standardized Preview Subset

- per-row nested object name:
  - `execution_record_request_preview`
- aligned fields:
  - `symbol`
  - `account_id`
  - `decision_ref`
  - `execution_action`
  - `execution_status`
  - `executed_gross_pct`
  - `execution_summary`
- preview-only freeze:
  - `execution_status` must be `preview_only`
  - the nested object exists only inside the preview document
  - no consumer may treat it as proof of execution

## Acceptance

- new CLI tests prove:
  - the happy path exposes the nested request-aligned preview object
  - the nested object preserves `account_id` and `decision_ref`
  - `buy/sell/hold` map into the aligned `execution_action`
  - malformed `P12` input is still rejected
- existing focused preview and `P10 -> P11 -> P12` coverage remain green
