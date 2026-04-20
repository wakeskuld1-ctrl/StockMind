# Contributing to StockMind

## Purpose

This repository is maintained as a standalone stock-domain Rust project with explicit boundary, runtime, and handoff rules.

Contributors should optimize for:

- preserving the approved stock-only architecture
- making verification status explicit
- leaving the next engineer or AI with low-context continuation material

## Read This First

Before changing code that affects behavior, contracts, boundaries, testing, or handoff quality, review these files in this order:

1. `README.md`
2. `docs/product/project_intent.md`
3. `docs/governance/contract_registry.md`
4. `docs/governance/decision_log.md`
5. `docs/governance/acceptance_criteria.md`
6. `docs/governance/response_contract.md`
7. `docs/handoff/CURRENT_STATUS.md`
8. `docs/handoff/HANDOFF_ISSUES.md`
9. `docs/AI_HANDOFF.md`

If your task changes the verified state of the branch, update the affected handoff files in the same task.

## Working Rules

- Keep `src/ops/stock.rs` as a frozen formal boundary unless the change is treated as a boundary event with matching design and guard updates.
- Prefer extending existing grouped gateways or adding a new grouped bus over enlarging one hotspot file.
- Do not claim repository health from memory. Re-run the relevant commands and record the real result.
- Keep runtime ownership stock-only and routed through `src/runtime/formal_security_runtime_registry.rs`.
- Treat `docs/handoff/CURRENT_STATUS.md` as the branch-health truth file and `docs/architecture/stockmind-acceptance-checklist.md` as the target acceptance map.

## Expected Workflow

1. Confirm the current branch, commit, and workspace status.
2. Review the governance and handoff documents listed above.
3. If the task changes architecture or flow, update the relevant design document under `docs/plans/` first.
4. Implement the smallest useful slice.
5. Run focused verification first.
6. Run broader acceptance commands when the task changes shared contracts or public flows.
7. Update:
   - `docs/handoff/CURRENT_STATUS.md` when branch health changed
   - `docs/handoff/HANDOFF_ISSUES.md` when new gaps or blockers were found
   - governance docs when contracts, intent, acceptance, or response rules changed
   - `CHANGELOG_TASK.MD` with the task summary

## Verification Ladder

Use the lightest verification that honestly proves the change, then scale up when needed.

### Structure acceptance

Use for boundary, layering, or catalog changes:

```bash
cargo test --test stock_entry_layer_source_guard -- --nocapture
cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
cargo test --test stock_foundation_boundary_gate_v2_source_guard -- --nocapture
cargo test --test stock_modeling_training_split_source_guard -- --nocapture
cargo test --test stock_catalog_grouping_source_guard -- --nocapture
```

### Formal mainline acceptance

Use for governance, package, lifecycle, execution, and post-trade changes:

```bash
cargo test --test security_decision_submit_approval_cli -- --nocapture
cargo test --test security_decision_verify_package_cli -- --nocapture
cargo test --test security_decision_package_revision_cli -- --nocapture
cargo test --test security_lifecycle_validation_cli -- --nocapture
cargo test --test security_post_meeting_conclusion_cli -- --nocapture
cargo test --test security_post_trade_review_cli -- --nocapture
```

### Repository acceptance

Use when the task changes shared contracts, dispatcher wiring, or multiple business families:

```bash
cargo check
cargo test -- --nocapture
```

## Handoff Minimum

Do not consider a task handoff-ready until all of the following are true:

- the relevant verification commands were actually run or explicitly deferred
- the current branch health is written down truthfully
- unresolved blockers are listed in `docs/handoff/HANDOFF_ISSUES.md`
- `CHANGELOG_TASK.MD` records what changed, why, what remains, and what was verified

## Graphify Expectation

When a major feature phase or architecture checkpoint is completed, generate or refresh the repository graph audit and attach the result to the current handoff trail.

If graph output is still missing, keep that gap visible in `docs/handoff/HANDOFF_ISSUES.md` instead of implying the audit already exists.
