// 2026-04-15 CST: Added because the current refactor now treats execution and position
// management as one explicit in-trade closed loop.
// Reason: the older flat boundary made it too easy to mistake position handling for a
// passive snapshot instead of the center of the mid-trade feedback loop.
// Purpose: group execution facts, account-position views, and adjustment flow without
// changing the verified runtime facade mainline.

pub use super::security_account_open_position_snapshot;
// 2026-04-18 CST: Added because the approved intake packet is now the formal
// first document inside the post-open execution-and-position loop.
// Reason: grouping it here keeps the packet aligned with the mid-trade data path
// rather than letting it drift back into pre-trade governance modules.
// Purpose: expose the new intake contract on the explicit in-trade grouped gateway.
pub use super::security_approved_open_position_packet;
// 2026-04-19 CST: Added because P10 now introduces the account-level objective
// shell at the front of the portfolio-core expansion stage.
// Reason: the grouped in-trade gateway should expose the new account objective
// contract beside the other post-open mathematical artifacts.
// Purpose: make the P10 objective builder reachable through the grouped execution gateway.
pub use super::security_account_objective_contract;
// 2026-04-19 CST: Added because Task 3 now introduces the unified replacement
// solver as the first P11 contract in the portfolio-core expansion.
// Reason: the grouped in-trade gateway should expose the replacement plan next
// to the P10 account objective boundary it consumes.
// Purpose: make the P11 replacement plan reachable through the grouped execution gateway.
pub use super::security_portfolio_replacement_plan;
// 2026-04-20 CST: Added because P12 now introduces the final governed
// allocation decision freeze after the implemented replacement plan stage.
// Reason: the grouped in-trade gateway should expose the whole P10 -> P11 ->
// P12 portfolio-core chain on one explicit execution-and-position surface.
// Purpose: make the P12 allocation decision reachable through the grouped gateway.
pub use super::security_portfolio_allocation_decision;
// 2026-04-20 CST: Added because the approved next step after P12 is one
// side-effect-free execution preview bridge.
// Reason: grouping it here keeps the downstream preview on the same explicit
// execution-and-position surface without bypassing the portfolio-core chain.
// Purpose: make the post-P12 preview bridge reachable through the grouped gateway.
pub use super::security_portfolio_execution_preview;
// 2026-04-20 CST: Added because P13 now extends the explicit post-P12 path
// from preview into one formal request-package bridge.
// Reason: grouping it here keeps the request bridge on the same execution-and-
// position surface without implying runtime execution facts.
// Purpose: make the P13 request bridge reachable through the grouped gateway.
pub use super::security_portfolio_execution_request_package;
// 2026-04-21 CST: Added because P14 now extends the explicit post-P12 path
// from request packaging into one enrichment bridge while still stopping short of execution.
// Reason: grouping it here keeps the enrichment bridge on the same execution-and-
// position surface without implying runtime execution facts.
// Purpose: make the P14 request enrichment bridge reachable through the grouped gateway.
pub use super::security_execution_journal;
pub use super::security_execution_record;
pub use super::security_portfolio_execution_request_enrichment;
// 2026-04-21 CST: Added because P15 now extends the explicit post-P12 path
// from request enrichment into one governed apply bridge.
// Reason: grouping it here keeps the new apply stage on the same execution-and-
// position surface while reusing the existing execution-record runtime path.
// Purpose: make the P15 apply bridge reachable through the grouped gateway.
pub use super::security_portfolio_execution_apply_bridge;
// 2026-04-22 CST: Added because P16 now freezes the governed P15 apply result
// into one explicit execution-status artifact on the same execution-and-position surface.
// Reason: later reconciliation should start from one named status bridge, not raw apply rows.
// Purpose: make the P16 execution-status bridge reachable through the grouped gateway.
pub use super::security_portfolio_execution_status_bridge;
// 2026-04-25 CST: Added because P17 recovery now extends the post-P15 chain
// from P16 status freezing into one side-effect-free reconciliation truth layer.
// Reason: grouping keeps reconciliation on the same execution-and-position surface
// without implying repair, replay, or position materialization.
// Purpose: make the P17 reconciliation bridge reachable through the grouped gateway.
pub use super::security_portfolio_execution_reconciliation_bridge;
// 2026-04-25 CST: Added because P18 recovery now freezes repair intent
// downstream of the recovered P17 reconciliation truth.
// Reason: grouping keeps repair-intent packaging on the same execution surface
// without implying retry execution or lifecycle closeout.
// Purpose: make the P18 repair package reachable through the grouped gateway.
pub use super::security_portfolio_execution_repair_package;
// 2026-04-25 CST: Added because P19A now freezes P18 governed retry candidates
// as replay request rows without becoming a replay executor.
// Reason: replay request packaging must remain separate from runtime writes and broker-fill replay.
// Purpose: make the P19A replay request package reachable through the grouped gateway.
pub use super::security_portfolio_execution_replay_request_package;
// 2026-04-25 CST: Added because P19B now validates replay requests as dry-run
// executor truth without writing runtime facts.
// Reason: commit-mode replay remains out of scope until a later approved contract.
// Purpose: make the P19B dry-run replay executor reachable through the grouped gateway.
pub use super::security_portfolio_execution_replay_executor;
// 2026-04-26 CST: Added because P19C freezes commit payload readiness after
// P19B while remaining side-effect free.
// Reason: runtime write authority must wait for P19D and a durable idempotency contract.
// Purpose: make the P19C replay commit preflight reachable through the grouped gateway.
pub use super::security_portfolio_execution_replay_commit_preflight;
// 2026-04-26 CST: Added because P19D follows P19C as the controlled runtime replay writer.
// Purpose: keep replay commit authority grouped with execution management and not pre-trade flow.
pub use super::security_portfolio_execution_replay_commit_writer;
// 2026-04-26 CST: Added because P19E audits P19D runtime replay commit results.
// Purpose: keep read-only replay verification on the same grouped execution gateway.
pub use super::security_portfolio_execution_replay_commit_audit;
// 2026-04-26 CST: Added because P20A consumes P19E audit truth for closeout preflight readiness.
// Purpose: keep side-effect-free readiness grouped with execution management without closing lifecycle.
pub use super::security_portfolio_execution_lifecycle_closeout_readiness;
// 2026-04-26 CST: Added because P20B verifies closed runtime evidence after P20A.
// Purpose: keep read-only closeout evidence grouped with execution management without archive writes.
pub use super::security_portfolio_execution_lifecycle_closeout_evidence_package;
// 2026-04-18 CST: Added because Task 2 introduces the live contract object that
// stands between approved intake and later active-position state.
// Reason: the user approved keeping the live contract on the post-open data path.
// Purpose: expose `PositionContract` from the grouped in-trade gateway.
pub use super::security_position_contract;
// 2026-04-18 CST: Added because Task 4 inserts one explicit single-position
// evaluation layer into the in-trade monitoring loop.
// Reason: later account evidence should consume a named evaluation object rather
// than rebuilding actionability directly from snapshots and contracts.
// Purpose: expose the per-position evaluation tool from the grouped in-trade gateway.
pub use super::security_per_position_evaluation;
// 2026-04-18 CST: Added because Task 5 now exposes one standardized monitoring
// evidence package built on top of active positions and evaluations.
// Reason: the in-trade monitoring loop needs a named account-level evidence artifact.
// Purpose: expose the monitoring evidence package from the grouped gateway.
pub use super::security_monitoring_evidence_package;
// 2026-04-19 CST: Added because Task 6 now exposes the account-level capital
// rebase chain on the same post-open management surface.
// Reason: capital events are governed live-state operations, not pre-trade planning artifacts.
// Purpose: expose the capital rebase module from the grouped gateway.
pub use super::security_capital_rebase;
pub use super::security_record_position_adjustment;
