# StockMind Current Status

## Snapshot Date

 - Date: 2026-04-24
- Workspace path: `E:\SM`
- Branch: `codex/p10-p11-clean-upload-20260420`
- HEAD: local branch now includes the merged docfix line on top of `8214bc7`

## Working Tree

- this local `E:\SM` worktree now carries a merge of `codex/reconcile-local-features-merged-20260423-docfix` into `codex/p10-p11-clean-upload-20260420`, with the pre-existing local tracked edits reapplied on top
- the working tree still contains many uncommitted local edits and generated runtime artifacts, including `P16` governance-sync work, capital-source raw-flow/snapshot/audit work, earlier portfolio-core/post-P12 edits, user-local Nikkei training changes, and large runtime fixture/output directories
- the latest branch-truth imported from the docfix line records a fresh 2026-04-24 repository-wide green verification in a separate reconciliation worktree after the missing `docs/plans/design/` backfill and the `Stock/Foundation Decoupling Baseline` handoff marker were restored
- the local `E:\SM` worktree itself should still be treated as dirty and not implicitly equivalent to that clean verification worktree until reverified here

## Verified Commands

### Passed

The latest preserved focused verification evidence carried into this merge handoff includes:

```bash
$env:CARGO_TARGET_DIR='E:\SM\target_fix_p15'; cargo test --test security_portfolio_execution_apply_bridge_cli -- --nocapture
$env:CARGO_TARGET_DIR='E:\SM\target_fix_p15'; cargo test --test security_portfolio_execution_status_bridge_cli -- --nocapture
cargo test --test security_portfolio_execution_request_enrichment_cli -- --nocapture
cargo test --test security_execution_record_cli -- --nocapture
cargo test --test security_portfolio_execution_request_package_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p15_finalize_verify'; cargo test --test security_portfolio_execution_apply_bridge_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p15_finalize_verify'; cargo test --test security_execution_record_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p15_finalize_verify'; cargo test --test security_portfolio_execution_preview_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p15_finalize_verify'; cargo test --test security_portfolio_execution_request_package_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p15_finalize_verify'; cargo test --test security_portfolio_execution_request_enrichment_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_chair_fixture_green'; cargo test --test security_chair_resolution_builder_unit -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_adjustment_input_green'; cargo test --test security_adjustment_input_package_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_adjustment_input_verify'; cargo test --test post_open_position_data_flow_guard -- --nocapture
cargo test --test security_analysis_fullstack_fundamental_metrics_source_guard -- --nocapture
cargo test --test security_analysis_fullstack_cli -- --nocapture
cargo test --test security_fundamental_history_live_backfill_cli -- --nocapture
cargo test --test security_stock_history_governance_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_security_approved_packet_green'; cargo test --test security_approved_open_position_packet_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_security_closed_archive_green'; cargo test --test security_closed_position_archive_cli -- --nocapture
$env:CARGO_TARGET_DIR='C:\codex-targets\sm_committee_package_green'; cargo test --test security_committee_decision_package_cli -- --nocapture
$env:CARGO_TARGET_DIR='C:\codex-targets\sm_legacy_freeze_etf_red'; cargo test --test security_chair_resolution_cli security_chair_resolution_does_not_require_stock_only_information_for_gold_etf_when_proxy_history_is_complete -- --nocapture
$env:CARGO_TARGET_DIR='C:\codex-targets\sm_legacy_freeze_guard_green2'; cargo test --test security_decision_committee_legacy_freeze_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p12_enhanced'; cargo test --test security_portfolio_allocation_decision_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p12_enhanced'; cargo test --test security_account_objective_contract_cli --test security_portfolio_replacement_plan_cli --test security_portfolio_allocation_decision_cli -- --nocapture
cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
cargo test --test security_decision_committee_legacy_freeze_source_guard -- --nocapture
cargo test --test security_scorecard_training_cli security_scorecard_training_generates_nikkei_index_artifact -- --nocapture
```

Fresh focused verification run on 2026-04-23 in `D:\SM_latest_8214bc7d`:

```bash
$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_restore_tail_green3'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_guard_tail'; cargo test --test security_committee_decision_package_cli --test security_adjustment_input_package_cli --test security_closed_position_archive_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_apply_bridge'; cargo test --test security_portfolio_execution_apply_bridge_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_manager_entry_fixed'; cargo test --test security_investment_manager_entry_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_check'; cargo check
```

Fresh boundary and repository verification run on 2026-04-24 in `D:\SM_latest_8214bc7d`:

```bash
$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_boundary_decoupling'; cargo test --test stock_foundation_boundary_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_entry_layer'; cargo test --test stock_entry_layer_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_catalog_grouping'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_dispatcher_grouping'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_modeling_split'; cargo test --test stock_modeling_training_split_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_package_chair'; cargo test --test security_decision_package_chair_node_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_package_verify'; cargo test --test security_decision_verify_package_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_legacy_freeze'; cargo test --test security_decision_committee_legacy_freeze_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_full_regression_20260424_c'; cargo test -- --nocapture
```

Fresh local focused verification run on 2026-04-24 in `E:\SM` after the P17 reconciliation closeout:

```bash
$env:CARGO_TARGET_DIR='E:\SM\target_p17_green'; cargo test --test security_portfolio_execution_reconciliation_bridge_cli -- --nocapture
$env:CARGO_TARGET_DIR='E:\SM\target_p17_green'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='E:\SM\target_p17_green'; cargo test --test security_portfolio_core_chain_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='E:\SM\target_p17_docs'; cargo test --test stock_entry_layer_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='E:\SM\target_p17_docs'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='E:\SM\target_p17_docs'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
```

Fresh local focused verification run on 2026-04-24 in `E:\SM` for the new P18 repair package:

```bash
$env:CARGO_TARGET_DIR='E:\SM\target_p18_green'; cargo test --test security_portfolio_execution_repair_package_cli -- --nocapture
```

Fresh local isolated verification run on 2026-04-24 in `E:\SM` for the scorecard-training truth check:

```bash
$env:CARGO_TARGET_DIR='E:\SM\target_ps_scorecard_training_full'; cargo test --test security_scorecard_training_cli -- --nocapture
```

Fresh local standardized isolated-runner verification on 2026-04-24 in `E:\SM` after environment-governance closeout:

```powershell
$cargoArgs = @('--test','stock_formal_boundary_manifest_source_guard','--','--nocapture'); .\scripts\invoke_isolated_cargo.ps1 -RunLabel smoke_guard_final -CargoCommand test -CargoArguments $cargoArgs
$cargoArgs = @('--','--nocapture'); .\scripts\invoke_isolated_cargo.ps1 -RunLabel repo_full_local_truth_final -CargoCommand test -CargoArguments $cargoArgs
$cargoArgs = @('--test','security_capital_source_factor_audit_cli','security_capital_source_factor_audit_ranks_factor_reports_with_holdout_and_walk_forward','--','--nocapture'); .\scripts\invoke_isolated_cargo.ps1 -RunLabel capital_source_audit_red_confirm -CargoCommand test -CargoArguments $cargoArgs
```

### Failed

Fresh blocking regression is now recorded in this worktree under the standardized isolated runner:

- `security_capital_source_factor_audit_cli::security_capital_source_factor_audit_ranks_factor_reports_with_holdout_and_walk_forward`
- isolated full-rerun command: `$cargoArgs = @('--','--nocapture'); .\scripts\invoke_isolated_cargo.ps1 -RunLabel repo_full_local_truth_final -CargoCommand test -CargoArguments $cargoArgs`
- isolated confirmation command: `$cargoArgs = @('--test','security_capital_source_factor_audit_cli','security_capital_source_factor_audit_ranks_factor_reports_with_holdout_and_walk_forward','--','--nocapture'); .\scripts\invoke_isolated_cargo.ps1 -RunLabel capital_source_audit_red_confirm -CargoCommand test -CargoArguments $cargoArgs`
- failing assertion: `tests/security_capital_source_factor_audit_cli.rs:208` expected `distinct_value_count == 1`, observed `0`
- logs:
  - `E:\SM\.verification\logs\repo_full_local_truth_final_20260424_174837_893_27980.log`
  - `E:\SM\.verification\logs\capital_source_audit_red_confirm_20260424_175956_498_16092.log`

Historical note only:

- earlier preserved branch-health records had previously exposed `security_feature_snapshot_cli` and later stock-boundary document guards
- those are no longer the current truth after the 2026-04-24 fresh verification recorded above
- the earlier local full-rerun red record at `C:\Users\tangguokai\AppData\Local\Temp\stockmind_full_rerun_20260423_pass5.log` should not be treated as a stable code regression for `security_scorecard_training_cli`; a fresh isolated target rerun passed `17 passed, 0 failed` on 2026-04-24 and narrowed that red state to Windows target/exe-lock verification pollution

## Current Delivery Read

- the merged local branch contains the P10/P11/P12 portfolio-core slice plus the post-P12 P13 request package, P14 request enrichment, P15 apply-bridge path, and your local carried-forward P16 plus capital-source line
- the new `security_portfolio_allocation_decision` tool is live on the public stock bus as an enhanced bounded P12 refinement layer with baseline-vs-refined allocation traceability
- the new `security_portfolio_execution_preview` tool is live on the public stock bus as a preview-only downstream consumer of governed P12 output, and each preview row now carries a nested execution-request-aligned preview subset
- the new `security_portfolio_execution_request_package` tool is live on the public stock bus as a formal side-effect-free P13 request bridge downstream of the standardized preview document
- the new `security_portfolio_execution_request_enrichment` tool is live on the public stock bus as a formal side-effect-free P14 enrichment bridge downstream of the P13 request package
- the new `security_portfolio_execution_apply_bridge` tool is live on the public stock bus as a formal governed P15 apply bridge downstream of the P14 enrichment bundle, and it surfaces `applied` / `partial_success` / `failed` row outcomes explicitly
- the new `security_portfolio_execution_repair_package` tool is live on the public stock bus as a formal side-effect-free P18 repair-intent package downstream of the P17 reconciliation artifact
- the formal `security_approved_open_position_packet`, `security_closed_position_archive`, `security_committee_decision_package`, and `security_adjustment_input_package` routes had already been restored onto the public stock bus before this merge handoff
- the stock formal-boundary manifest guard docs and frozen module set were reconciled to the current merged mainline on 2026-04-23, and the guard remains green
- the restored lifecycle-tail routes (`security_committee_decision_package`, `security_adjustment_input_package`, `security_closed_position_archive`) and the carried-forward `security_portfolio_execution_apply_bridge` route are fresh-green again on 2026-04-23
- `security_investment_manager_entry_cli` was also repaired from an encoding-broken regression literal and is fresh-green again on 2026-04-23
- the missing design-doc migration slice under `docs/plans/design/` has now been restored for the active stock boundary guards
- the current merged worktree is repository-wide green under the fresh 2026-04-24 `cargo test -- --nocapture` run
- the local `E:\SM` worktree now also has fresh 2026-04-24 focused-green verification for the post-P15 downstream chain through `P17`, including `security_portfolio_execution_reconciliation_bridge_cli`, `security_portfolio_core_chain_source_guard`, `stock_formal_boundary_manifest_source_guard`, `stock_entry_layer_source_guard`, `stock_catalog_grouping_source_guard`, and `stock_dispatcher_grouping_source_guard`
- the local `E:\SM` worktree now also has fresh 2026-04-24 focused-green verification for `security_portfolio_execution_repair_package_cli`
- the local `E:\SM` worktree now also has fresh 2026-04-24 isolated-green verification for `security_scorecard_training_cli`, including the earlier full-rerun red slice that had complained about missing `weekly_spot_return_min`
- the local `E:\SM` worktree now has a standardized isolated verification runner at `scripts/invoke_isolated_cargo.ps1`, and README/CONTRIBUTING now route Windows-local branch-health claims through that entrypoint instead of ad-hoc reused targets
- the local carried-forward post-P15 line should now be treated as freshly re-verified through `P18` focused contract verification, `P17` boundary and grouping guards, and the isolated `security_scorecard_training_cli` truth check in this dirty worktree; however, local branch health is now blocked by the newly isolated-confirmed `security_capital_source_factor_audit_cli` regression and the broader capital-source standalone toolchain should not be called fully green here until that red is resolved

## Current Gaps Still Visible

- the first repository-level graph audit is AST-only and code-structural; document/image semantic extraction remains an optional future enhancement, not a branch-health blocker
- P12 is still intentionally not a trim-funded reallocation solver; this route only refines with baseline residual cash and remaining turnover slack
- the post-P12 preview bridge is intentionally preview-only and must not be mistaken for real execution or persistence, even though it now exposes one nested execution-request-aligned preview subset
- the restored `security_approved_open_position_packet` route is intentionally intake-only normalization and validation; it must not be mistaken for live contract creation, execution, or persistence
- the restored `security_closed_position_archive` route is intentionally lifecycle-archive-only; it must not be mistaken for committee governance packaging, execution, or persistence writes
- the restored `security_adjustment_input_package` route is intentionally side-effect-free packaging only and must not be mistaken for real execution or persistence even though it may preview downstream requests
- the new P13 request bridge is intentionally request-only and must not be mistaken for real execution, persistence, or approval closeout
- the new P14 enrichment bridge is intentionally request-enrichment-only and must not be mistaken for real execution, runtime persistence, or `security_execution_record`
- the new P15 apply bridge is intentionally a governed first apply layer that writes runtime-backed execution records through `security_execution_record`; it should not be mistaken for broker execution, cross-symbol rollback, broker-fill replay, or order-ledger exactness
- the worktree is still intentionally dirty with unrelated runtime artifacts and parallel edits, so future Git upload must continue to stage only the current task slice
- the `docs/plans/` to `docs/plans/design/` migration remains a traceability risk if future boundary guards add new doc dependencies without backfilling the new path in the same change
- the new P16 status bridge is intentionally a pure status-freeze layer; it must not be mistaken for reconciliation, position materialization, or post-trade lifecycle closure
- the new P18 repair package is intentionally a repair-intent freeze layer; it must not be mistaken for runtime replay, broker execution, position materialization, or lifecycle closure
- the new standardized isolated Cargo runner closes the earlier `security_scorecard_training_cli` verification-pollution ambiguity, but it also proves that the current dirty local worktree is not repository-green because `security_capital_source_factor_audit_cli` now fails under the same isolated truth path
- the standalone factor-audit branch history still includes an older thin real-audit result with only one JPX observation date, but the raw capital-flow coverage blocker itself is now closed for 2016-2025 JPX + long-history MOF
- the current capital-source audit still relies on the active workspace/default price store or explicit `EXCEL_SKILL_STOCK_DB` to resolve `NK225.IDX`; if the price db is mispointed, the audit can look empty even though raw flow import succeeded
- Windows test-process residue can still poison local default-target or reused-target reruns, especially around `security_scorecard_training_cli`; future truth checks should prefer a fresh explicit `CARGO_TARGET_DIR` instead of treating reused-target red runs as immediate code truth
- the remaining capital-source gap is no longer raw JPX coverage but downstream integration: if the branch later needs factor re-audit, training merge, or audit-method changes on top of the new raw history, open a new design/approval cycle first
- older historical handoff notes remain background context only; current branch truth still belongs to this file plus `docs/handoff/HANDOFF_ISSUES.md`

## Update Rule

Update this file whenever any of the following change:

- branch health
- first blocking regression failure
- branch or commit used as the active delivery line
