# StockMind Handoff Issues

## Purpose

This file tracks the handoff and standardization problems that are visible right now.

Use it for unresolved gaps, not for finished work history.

## Current Blocking Issues

- latest full regression evidence: `$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_full_regression_20260424_c'; cargo test -- --nocapture`
- result: repository-wide `cargo test` completed green in this worktree after restoring the missing `docs/plans/design/` boundary docs and the `Stock/Foundation Decoupling Baseline` handoff marker
- local dirty-worktree note: the earlier `security_scorecard_training_cli` red state from `C:\Users\tangguokai\AppData\Local\Temp\stockmind_full_rerun_20260423_pass5.log` is no longer an active blocker after `$env:CARGO_TARGET_DIR='E:\SM\target_ps_scorecard_training_full'; cargo test --test security_scorecard_training_cli -- --nocapture` completed `17 passed, 0 failed` on 2026-04-24
- local blocking regression on 2026-04-24 under the standardized isolated runner: `$cargoArgs = @('--','--nocapture'); .\scripts\invoke_isolated_cargo.ps1 -RunLabel repo_full_local_truth_final -CargoCommand test -CargoArguments $cargoArgs` failed at `security_capital_source_factor_audit_cli::security_capital_source_factor_audit_ranks_factor_reports_with_holdout_and_walk_forward`
- isolated confirmation command: `$cargoArgs = @('--test','security_capital_source_factor_audit_cli','security_capital_source_factor_audit_ranks_factor_reports_with_holdout_and_walk_forward','--','--nocapture'); .\scripts\invoke_isolated_cargo.ps1 -RunLabel capital_source_audit_red_confirm -CargoCommand test -CargoArguments $cargoArgs`
- observed failure: `tests/security_capital_source_factor_audit_cli.rs:208` expected `distinct_value_count == 1`, observed `0`
- logs:
  - `E:\SM\.verification\logs\repo_full_local_truth_final_20260424_174837_893_27980.log`
  - `E:\SM\.verification\logs\capital_source_audit_red_confirm_20260424_175956_498_16092.log`

## Current Active Gaps

- the worktree remains intentionally dirty with unrelated runtime artifacts, generated fixtures, and parallel edits, so any Git delivery must keep staging narrowly scoped
- the `docs/plans/` to `docs/plans/design/` migration is still easy to drift; future guard additions must backfill the new-path design docs and handoff markers in the same change
- the current P15 direct adapter is intentionally bounded to governed runtime execution recording through `security_execution_record`; it is not broker execution, broker-fill replay, order-ledger exactness, or cross-symbol rollback
- historical note: older `security_scorecard_training_nikkei_futures_*` artifacts still exist under runtime outputs and may still reflect the pre-futures 19-feature contract, but they do not describe the latest working-tree truth
- the local `E:\SM` worktree has now been freshly focused-reverified on 2026-04-24 through the post-P15 downstream chain and adjacent formal guards (`security_portfolio_execution_reconciliation_bridge_cli`, `security_portfolio_execution_repair_package_cli`, `security_portfolio_core_chain_source_guard`, `stock_formal_boundary_manifest_source_guard`, `stock_entry_layer_source_guard`, `stock_catalog_grouping_source_guard`, `stock_dispatcher_grouping_source_guard`); repository-wide full rerun still belongs to the separate clean verification worktree, not this dirty local tree
- local verification-governance closeout: Windows-local branch-health claims now have a standardized isolated entrypoint at `scripts/invoke_isolated_cargo.ps1`, plus README/CONTRIBUTING guidance that prefers fresh isolated targets over reused `target/` state
- local verification-environment gap: default-target or recycled-target reruns in `E:\SM` should still be treated as untrusted when they disagree with the standardized isolated runner; the environment ambiguity is reduced, but not removed from manual ad-hoc commands
- the capital-source toolchain had already been focused-green on 2026-04-22: `security_capital_flow_backfill`, `security_capital_flow_jpx_weekly_import`, `security_capital_flow_mof_weekly_import`, `security_capital_flow_raw_audit`, `security_capital_source_factor_snapshot`, and `security_capital_source_factor_audit`
- the former JPX long-history blocker had already been closed in local history: runtime `E:\SM\.stockmind_runtime\capital_flow_real_2016_2025_20260422_b` contains 521 JPX weeks across 2016-2025 after adding legacy `TSE 1st Section` parser support
- the current raw-data path can show direct JPX+MOF weekly values through `security_capital_flow_raw_audit`, but one real factor-audit slice still depends on the active price db resolving `NK225.IDX`
- `CHANGELOG_TASK.MD` currently contains at least one `NUL` byte and should remain append-only until a separate approved encoding-cleanup contract exists
- the active capital-source gap is downstream use of the new raw history, not raw JPX collection itself; any future factor re-audit, training merge, or audit-method change must start from a new approved design

## Optional Enhancements

- [ ] if richer cross-document graph coverage is needed later, rerun Graphify with semantic document extraction instead of the current AST-only code audit
- [ ] continue normal historical-doc maintenance as older background files are next touched, but this is no longer a current branch-health issue

## Maintenance Rule

Remove an item from this file only after:

- the problem was actually fixed
- the current status file was refreshed if branch health changed
- the fix was recorded in `CHANGELOG_TASK.MD`
