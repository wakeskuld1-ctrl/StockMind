# StockMind AI Handoff

## Stock/Foundation Decoupling Baseline

StockMind is a stock-only split repo. Stock does not currently depend on generic foundation analytics, and the old foundation boundary is intentionally excluded from this workspace.

## Stock/Foundation Split Manifest Frozen

The split repo keeps a shared/runtime hold zone only for stock-facing tools, contracts, dispatcher wiring, and governed runtime stores. Public tool discovery is stock-only in this repo.

## Stock/Foundation Boundary Gate V2

Shared/runtime hold-zone files remain under `src/tools/*` and `src/runtime/*`, but they now carry only stock-facing ownership in the standalone repo.

## Stock Formal Boundary Manifest Gate

The formal stock boundary is frozen at `src/ops/stock.rs`, and `src/ops/mod.rs` exposes only the `stock` top-level root in this split repo.

## Stock Boundary Expansion Rule

Do not keep extending `src/ops/stock.rs` as the default answer to new stock-domain work.

The current boundary is already the approved formal stock root for this split repo. If a future change adds a new business family or another large capability cluster, prefer one of these paths first:

1. add the capability under an existing grouped gateway when it clearly belongs there
2. add a new grouped bus / scenario shell above the existing modules
3. keep support logic internal behind an existing approved boundary

Do not widen `stock.rs` unless the change is explicitly treated as a formal boundary event with matching design, handoff, and guard updates.

Reason:
- the current repo already has enough public surface on the stock boundary
- repeated `stock.rs` expansion will blur ownership and recreate the flat pre-refactor shape
- future growth should happen by clearer grouped buses, not by inflating the single frozen root

## Current High-Pressure Areas

These are not current release blockers, but they are the easiest places for later sessions to reintroduce coupling:

- `src/tools/dispatcher/stock_ops.rs`
  - application-layer hotspot with many dispatch branches
  - future tool growth should prefer clearer grouped dispatch buses instead of continuing to enlarge one file
- `src/ops/security_execution_record.rs`
  - lifecycle orchestration hotspot connecting package, execution, plan, and review artifacts
  - future changes should prefer helper extraction or existing internal support modules over further mainline accumulation
- `src/ops/security_real_data_validation_backfill.rs`
  - broad validation + import + governed-history orchestration surface
  - future additions should avoid turning it into the default place for all runtime enrichment logic
- `src/ops/signal_outcome_research.rs`
  - broad sidecar analysis surface
  - future research features should be evaluated for a separate sidecar bus before adding more responsibilities here

## Dispatcher Growth Rule

The standalone repo currently routes all stock tools through `src/tools/dispatcher.rs` and `src/tools/dispatcher/stock_ops.rs`.

This is accepted for the current phase, but it is not a license to keep centralizing every new capability there forever.

If future work adds another meaningfully separate stock capability family, prefer:

- a new grouped dispatcher bus
- or a thinner grouped module split under `src/tools/dispatcher/`

Do not treat `stock_ops.rs` as an unlimited sink for every future tool branch.

## legacy committee public discovery

Public discovery must expose the formal committee mainline `security_committee_vote -> security_chair_resolution` while leaving legacy compatibility routes out of `tool_catalog`.

## legacy committee application surface

The remaining legacy committee compatibility path must stay explicitly labeled at the grouped gateway and dispatcher surface until the last downstream caller is migrated.

## Security Decision Committee Legacy Freeze

The legacy `security_decision_committee` implementation is a frozen compatibility zone in this split repo.

Current governance work must stay on the formal mainline `security_committee_vote -> security_chair_resolution`.

Any change to `src/ops/security_decision_committee.rs` should be treated as an approved migration event and reviewed against `docs/plans/2026-04-16-security-legacy-committee-governance-design.md`.

## Acceptance Reminder

Before treating a future stock refactor as complete, re-run all three layers:

1. structure acceptance
2. formal mainline acceptance
3. full repository regression

Use `docs/architecture/stockmind-acceptance-checklist.md` as the current acceptance entry instead of relying on memory.
