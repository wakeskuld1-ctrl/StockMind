# P17/P18 Execution Recovery Design

## Intent
- Goal: recover the missing post-P16 execution chain in `D:\SM` by rebuilding P17 reconciliation and P18 repair-intent package contracts from the current P16 implementation and preserved handoff records.
- Scope: add P17 and P18 Rust modules, CLI tests, public stock boundary wiring, catalog and dispatcher routes, governance registry updates, handoff notes, and focused verification evidence.
- Non-goals: do not implement P19 replay executor, broker execution, order-ledger exactness, position materialization, or lifecycle closeout.
- Success definition: P17 consumes only the P16 status artifact and freezes reconciliation truth; P18 consumes only the P17 reconciliation artifact and freezes repair intent. Both tools are visible on the public stock bus and verified by focused tests plus boundary/grouping guards.
- Delivery form: code, tests, governance documentation, current-status updates, and task journal entry in the existing `D:\SM` worktree.

## Contract
- P17 core object: `SecurityPortfolioExecutionReconciliationBridgeDocument`.
- P17 input: `SecurityPortfolioExecutionStatusBridgeDocument`.
- P17 output: one side-effect-free reconciliation artifact with row-level reconciliation status, unresolved rows, blockers, lineage refs, summary counts, and rationale.
- P17 state boundary: may classify status rows as settled, skipped hold, reconciliation required, or blocked by rejected upstream state; must not write runtime facts, replay execution records, infer broker fills, or create positions.
- P17 rejection conditions:
  - unsupported P16 `execution_status`
  - status-row count drift against P16 summary counts
  - missing required P16 lineage refs
- P18 core object: `SecurityPortfolioExecutionRepairPackageDocument`.
- P18 input: `SecurityPortfolioExecutionReconciliationBridgeDocument`.
- P18 output: one repair-intent artifact with rows classified as `manual_follow_up`, `governed_retry_candidate`, or `blocked_pending_decision`.
- P18 state boundary: freezes what kind of repair is needed; must not execute repair, retry writes, broker-fill replay, materialize positions, or close lifecycle.
- P18 rejection conditions:
  - missing P17 lineage
  - reconciliation summary drift
  - unsupported P17 reconciliation status
  - unresolved row with insufficient evidence for a repair class
- Traceability requirements: P17 must preserve P16 -> P15 -> P14 -> P13 -> preview -> P12 refs; P18 must preserve P17 and inherited upstream refs.
- Compatibility zones: `src/ops/stock.rs` remains the formal public module manifest; catalog and dispatcher ordering must keep P17/P18 immediately after P16 in execution-and-position-management.

## Decision
- Chosen approach: rebuild P17 first, then P18.
- Why: P18 depends on P17 unresolved execution truth, so implementing P18 without P17 would create a fake downstream contract.
- Rejected alternative: P18-only recovery with a thin P17 stub. It is faster but hides reconciliation drift and weakens any future P19 replay executor.
- Rejected alternative: documentation-only recovery. It preserves handoff clarity but leaves `D:\SM` functionally behind the recorded `E:\SM` line.
- Known tradeoff: the original `E:\SM` files are unavailable, so field names and helper internals are reconstructed from current P16 style and changelog/handoff evidence rather than copied.
- Open question: future P19 replay executor contract remains unapproved and must start a separate design cycle.

## Acceptance
- Before implementation starts:
  - design document exists under `docs/plans/`
  - implementation plan exists under `docs/plans/`
  - P16 code and tests have been inspected as the upstream style source
- Before completion can be claimed:
  - P17 tests were written first and observed red for missing tool/module behavior
  - P17 implementation makes focused P17 tests green
  - P18 tests were written first and observed red for missing tool/module behavior
  - P18 implementation makes focused P18 tests green
  - public surface and guard tests are updated and green
  - `docs/governance/contract_registry.md`, `docs/governance/decision_log.md`, `docs/handoff/CURRENT_STATUS.md`, and `docs/handoff/HANDOFF_ISSUES.md` reflect current `D:\SM` truth
  - `.trae/CHANGELOG_TASK.md` receives an append-only task entry
- Completion must be refused or softened if full repository regression is not run; focused-green through P18 is not repository-wide green.
