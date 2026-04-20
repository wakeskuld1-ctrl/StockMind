# P12 Governed Portfolio Allocation Decision Design

## Status

- Date: 2026-04-20
- Phase: `P12`
- Approved route: `Option 1 - minimum governed decision freeze layer`
- Upstream prerequisite: `P10` and `P11` are already implemented and verified in this branch
- This document freezes design and execution contract only; it does not claim implementation is live

## Intent

- Goal: add one formal `P12` tool that freezes the final governed portfolio allocation decision after `P11`
- Scope: define the public tool name, request contract, output contract, rejection boundary, non-goals, and minimum acceptance checks
- Non-goals:
  - do not introduce a stronger or second solver
  - do not accept raw upstream fragments that bypass `P10` or `P11`
  - do not emit execution requests, order intents, or persistence side effects
  - do not start `P13+` work inside this slice
- Success definition: the next implementation session can build `P12` without re-opening the stage boundary or re-debating whether `P12` should solve optimization again
- Delivery form: one design-freeze document that acts as the governing contract for the next implementation slice

## Single Source Of Truth

- Historical phase context: `E:/SM/docs/handoff/AI_HANDOFF.md`
- Current branch health: `E:/SM/docs/handoff/CURRENT_STATUS.md`
- Stable upstream contracts:
  - `E:/SM/src/ops/security_account_objective_contract.rs`
  - `E:/SM/src/ops/security_portfolio_replacement_plan.rs`
- Existing upstream CLI acceptance:
  - `E:/SM/tests/security_account_objective_contract_cli.rs`
  - `E:/SM/tests/security_portfolio_replacement_plan_cli.rs`

## Decision

- Chosen approach: `Option 1 - minimum governed decision freeze layer`
- Core idea:
  - `P10` freezes the governed account objective and candidate universe
  - `P11` freezes the unified replacement plan and capital migration summary
  - `P12` freezes the final governed allocation decision that re-states the accepted target allocation, residual cash, constraint checks, decision rationale, and readiness/blockers
- Why this route was approved:
  - it completes the `P10 -> P11 -> P12` mainline without turning `P12` into another solver stage
  - it keeps the public architecture auditable and stage-separated
  - it minimizes delivery risk while still making the portfolio-core chain complete
- Rejected alternative:
  - `Option 2 - enhanced allocation solver`
  - rejected because it would duplicate `P11` responsibilities, widen the implementation boundary, and blur the formal phase decomposition already approved in handoff
- Known tradeoffs:
  - `P12` will validate and freeze, not optimize
  - richer allocation intelligence stays deferred until a later separately approved phase

## Execution Contract

- Chosen approach: implement only the minimum `P12` governed decision-freeze layer described in this document
- Allowed change boundary:
  - add one new `src/ops/security_portfolio_allocation_decision.rs`
  - expose the tool through `src/ops/stock.rs`
  - wire catalog and dispatcher entries
  - add one new CLI contract test file for `P12`
  - update governance and handoff truth only if the implementation actually lands and changes accepted branch truth
- Explicit non-goals:
  - no new optimization math beyond contract validation and deterministic freeze
  - no direct consumption of `active_position_book`, `position_contracts`, `monitoring_evidence_package`, or raw candidates at `P12`
  - no order/execution generation
  - no retrofitting `P10` or `P11` into broader semantic changes
- Best-practice path expected for this route:
  - `P12` must consume formal `P10` and `P11` documents only
  - `P12` must re-check constraint conformance before freezing the decision
  - `P12` must emit one stable decision document plus one named result wrapper
  - `P12` must fail hard on contract drift instead of silently repairing it
- Acceptance checks for route conformance:
  - the public request shell contains only `P10` and `P11` documents plus metadata
  - the output is a governed decision document, not a solver trace or execution request preview
  - happy-path and hard-fail tests prove that `P12` validates upstream drift rather than recomputing from raw fragments

## Proposed Public Tool

- Tool name: `security_portfolio_allocation_decision`
- Stage meaning: `P12` governed portfolio allocation decision
- Tool responsibility: convert governed `P10` and `P11` outputs into one final allocation decision document that is ready for later approval or downstream execution-bridge work

## Contract

### Request Contract

- Proposed type: `SecurityPortfolioAllocationDecisionRequest`
- Required fields:
  - `account_objective_contract: SecurityAccountObjectiveContractDocument`
  - `portfolio_candidate_set: SecurityPortfolioCandidateSet`
  - `portfolio_replacement_plan: SecurityPortfolioReplacementPlanDocument`
  - `created_at: String`
- Compatibility rule:
  - `P12` request must not accept raw account, monitoring, position-contract, or candidate inputs
  - `P12` must treat `P10` and `P11` outputs as the only legal upstream sources

### Output Contract

- Proposed document type: `security_portfolio_allocation_decision`
- Proposed version: `security_portfolio_allocation_decision.v1`
- Proposed primary document: `SecurityPortfolioAllocationDecisionDocument`
- Proposed wrapper: `SecurityPortfolioAllocationDecisionResult`

### Required Output Sections

- Identity and lineage:
  - `portfolio_allocation_decision_id`
  - `contract_version`
  - `document_type`
  - `generated_at`
  - `account_id`
  - `account_objective_contract_ref`
  - `portfolio_candidate_set_ref`
  - `portfolio_replacement_plan_ref`
- Final allocation freeze:
  - `final_target_allocations`
  - `residual_cash_weight_pct`
  - `capital_base_amount_before`
  - `capital_base_amount_after`
  - `rebase_context_applied`
- Governance checks:
  - `constraint_checks`
  - `readiness_status`
  - `blockers`
- Traceability and rationale:
  - `decision_rationale`
  - `conflict_resolution_summary`
  - `decision_summary`

### Proposed Supporting Rows

- `SecurityPortfolioAllocationDecisionRow`
  - `symbol`
  - `current_weight_pct`
  - `target_weight_pct`
  - `weight_delta_pct`
  - `decision_action`
  - `allocation_source`
- `SecurityPortfolioAllocationConstraintCheck`
  - `check_name`
  - `status`
  - `observed_value`
  - `limit_value`
  - `detail`

## Rule Layer Separation

- Universal rules:
  - no cross-account contamination
  - no weight non-conservation
  - no hidden repair of malformed upstream contracts
- Project rules:
  - portfolio-core stages must stay decomposed as `P10`, `P11`, `P12`
  - current public stock boundary remains explicit and auditable
  - current branch truth belongs to `CURRENT_STATUS.md`, not to historical handoff notes
- Task rules for this slice:
  - `P12` only freezes a governed allocation decision
  - `P12` may validate upstream consistency, but must not become a new optimizer
- Temporary assumptions:
  - the first `P12` version may derive final allocation rows from `P11.target_weights` plus `P11` action sections
  - richer approval semantics may be added later, but are not required for `v1`

## Rejection Boundary

- `P12` must hard-fail when `portfolio_replacement_plan.account_id` does not match `account_objective_contract.account_id`
- `P12` must hard-fail when `portfolio_candidate_set.account_id` does not match the same account
- `P12` must hard-fail when the final target allocation cannot be mapped back to symbols that exist in the governed candidate set
- `P12` must hard-fail when `portfolio_replacement_plan.target_weights` do not conserve weight or contradict `capital_migration_plan`
- `P12` must hard-fail when recomputed target risk budget exceeds `account_objective_contract.risk_budget_limit`
- `P12` must hard-fail when recomputed target position count exceeds `account_objective_contract.position_count_limit`
- `P12` must hard-fail when recomputed turnover exceeds `account_objective_contract.turnover_limit`
- `P12` must hard-fail when residual cash is negative or inconsistent with the target-weight sum
- `P12` must hard-fail when callers try to bypass formal upstream documents by adding raw fragment fields to the request contract

## Readiness Semantics

- `readiness_status = ready` only when all constraint checks pass and blockers are empty
- `readiness_status = blocked` when any hard-governance check fails
- the first `P12` version should keep readiness binary and deterministic rather than adding a wider approval taxonomy

## Acceptance

### Pre-Implementation Gate

- this document must explicitly define:
  - public tool name
  - legal request inputs
  - legal output sections
  - rejection conditions
  - non-goals
  - minimum verification commands
- invalid work to reject before implementation:
  - any proposal that makes `P12` consume raw upstream fragments
  - any proposal that adds a second solver layer
  - any proposal that directly emits execution-side effects

### Pre-Completion Gate For The Future Implementation

- catalog exposes `security_portfolio_allocation_decision`
- dispatcher accepts the request and returns the named result wrapper
- happy-path CLI test proves `P12` freezes one governed final allocation decision from `P10` and `P11` outputs
- hard-fail tests prove rejection of:
  - cross-account drift
  - malformed replacement-plan allocation non-conservation
  - objective-limit mismatch
  - candidate-set bypass or symbol drift
- branch truth and task journal are updated if implementation changes accepted health or current next-step guidance

## Minimum Verification Entry For The Future Implementation

- focused:
  - `cargo test --test security_portfolio_allocation_decision_cli -- --nocapture`
- portfolio-core chain:
  - `cargo test --test security_account_objective_contract_cli --test security_portfolio_replacement_plan_cli --test security_portfolio_allocation_decision_cli -- --nocapture`
- repository-level confidence after landing:
  - `cargo test -- --nocapture`

## Initial File Plan For The Future Implementation

- Create: `E:/SM/src/ops/security_portfolio_allocation_decision.rs`
- Modify: `E:/SM/src/ops/stock.rs`
- Modify: `E:/SM/src/tools/catalog.rs`
- Modify: `E:/SM/src/tools/dispatcher.rs`
- Modify: `E:/SM/src/tools/dispatcher/stock_ops.rs`
- Create: `E:/SM/tests/security_portfolio_allocation_decision_cli.rs`
- Modify if implementation changes governance truth:
  - `E:/SM/docs/governance/contract_registry.md`
  - `E:/SM/docs/governance/decision_log.md`
  - `E:/SM/docs/handoff/CURRENT_STATUS.md`
  - `E:/SM/CHANGELOG_TASK.MD`

## Next Skill

- Immediate next skill after this design freeze: `writing-plans`
- Immediate next engineering workflow after planning: `test-driven-development`
