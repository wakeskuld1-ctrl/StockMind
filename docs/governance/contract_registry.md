# StockMind Contract Registry

## Scope

This registry is intentionally compact.

It names the formal request and output contracts that currently matter for architecture review, handoff, and acceptance work in this repository.

It is not a replacement for reading the implementation files when exact field-level behavior matters.

## Registry

| Tool / flow | Request contract | Primary output contract(s) | Boundary notes | File anchor |
| --- | --- | --- | --- | --- |
| `security_committee_vote` | `SecurityCommitteeVoteRequest` | `SecurityCommitteeVoteResult` | formal committee mainline; do not treat legacy committee as default public route | `src/ops/security_committee_vote.rs` |
| `security_chair_resolution` | `SecurityChairResolutionRequest` | `SecurityChairResolutionDocument`, `SecurityChairResolutionResult` | formal governance follow-on to committee vote | `src/ops/security_chair_resolution.rs` |
| `security_position_plan` | `SecurityPositionPlanRequest` | `SecurityPositionPlanDocument`, `SecurityPositionPlanResult` | position plan remains one formal governance-to-execution bridge | `src/ops/security_position_plan.rs` |
| `security_position_contract` | `SecurityPositionContractRequest` | `SecurityPositionContract`, `SecurityPositionContractResult` | contract builder must consume formal upstream planning artifacts | `src/ops/security_position_contract.rs` |
| `security_execution_record` | `SecurityExecutionRecordRequest` | `SecurityExecutionRecordDocument`, `SecurityExecutionRecordResult` | execution object anchors the real lifecycle slice and later post-trade review | `src/ops/security_execution_record.rs` |
| `security_post_trade_review` | `SecurityPostTradeReviewRequest` | `SecurityPostTradeReviewDocument`, `SecurityPostTradeReviewResult` | post-trade output must stay bound to the formal execution chain | `src/ops/security_post_trade_review.rs` |
| `security_approved_open_position_packet` | `SecurityApprovedOpenPositionPacketRequest` | `SecurityApprovedOpenPositionPacketDocument` | post-open management starts from an approved packet, not a raw research candidate | `src/ops/security_approved_open_position_packet.rs` |
| `security_monitoring_evidence_package` | `SecurityMonitoringEvidencePackageRequest` | `SecurityMonitoringEvidencePackage`, `SecurityMonitoringEvidencePackageResult` | monitoring evidence is a governed post-open evidence object | `src/ops/security_monitoring_evidence_package.rs` |
| `security_capital_rebase` | `SecurityCapitalRebaseRequest` | `SecurityAccountRebaseSnapshot`, `SecurityCapitalRebalanceEvidencePackage`, `SecurityCapitalRebaseResult` | capital events are first-class rebasing events, not ordinary add/trim aliases | `src/ops/security_capital_rebase.rs` |
| `security_account_objective_contract` | `SecurityAccountObjectiveContractRequest` | `SecurityAccountObjectiveContractDocument`, `SecurityPortfolioCandidateSet`, `SecurityAccountObjectiveContractResult` | P10 consumes only governed account and candidate inputs | `src/ops/security_account_objective_contract.rs` |
| `security_portfolio_replacement_plan` | `SecurityPortfolioReplacementPlanRequest` | `SecurityPortfolioReplacementPlanDocument`, `SecurityPortfolioReplacementPlanResult` | P11 consumes formal P10 outputs instead of raw upstream fragments | `src/ops/security_portfolio_replacement_plan.rs` |
| `security_portfolio_allocation_decision` | `SecurityPortfolioAllocationDecisionRequest` | `SecurityPortfolioAllocationDecisionDocument`, `SecurityPortfolioAllocationDecisionResult` | P12 consumes only formal P10/P11 outputs, emits baseline-vs-refined allocation truth, and may apply bounded residual-cash refinement inside turnover slack and symbol caps; it must not behave like a second full solver | `src/ops/security_portfolio_allocation_decision.rs` |
| `security_portfolio_execution_preview` | `SecurityPortfolioExecutionPreviewRequest` | `SecurityPortfolioExecutionPreviewDocument`, `SecurityPortfolioExecutionPreviewResult` | post-P12 bridge consumes only the governed allocation decision and emits preview-only execution rows plus one nested execution-request preview subset per symbol; it must not execute, persist, or bypass P12 | `src/ops/security_portfolio_execution_preview.rs` |
| `security_portfolio_execution_request_package` | `SecurityPortfolioExecutionRequestPackageRequest` | `SecurityPortfolioExecutionRequestPackageDocument`, `SecurityPortfolioExecutionRequestPackageResult` | P13 consumes only the standardized preview document and emits a formal side-effect-free execution request package; it must not execute, persist, or bypass preview/P12 lineage | `src/ops/security_portfolio_execution_request_package.rs` |

## Runtime Ownership Contracts

These runtime families are currently part of the accepted stock-only ownership surface:

- `security_execution.db`
- `stock_history.db`
- `security_external_proxy.db`
- `security_fundamental_history.db`
- `security_disclosure_history.db`
- `security_corporate_action.db`
- `security_resonance.db`
- `signal_outcome_research.db`

Resolution order is defined by:

1. `STOCKMIND_RUNTIME_DIR`
2. parent of `STOCKMIND_RUNTIME_DB`
3. `EXCEL_SKILL_RUNTIME_DIR`
4. parent of `EXCEL_SKILL_RUNTIME_DB`
5. `.stockmind_runtime/`

## Hard-Fail Rules For Governance Work

When a task changes a formal contract, the task is incomplete unless all of the following are updated together:

- the implementation files
- the dispatcher or catalog surface, if public behavior changed
- the verification commands that prove the contract still works
- the current handoff status, if branch health changed

## Known Registry Limits

- The repository now includes a generated graph audit under `graphify-out/`, but that bundle is structural and AST-only rather than a full semantic contract graph.
- This registry is a stable governance reference and should be rechecked when large migrations land.
