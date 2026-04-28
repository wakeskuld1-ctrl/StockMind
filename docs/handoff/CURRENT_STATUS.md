# StockMind Current Status

## Snapshot Date

- Date: 2026-04-28
- Workspace path: `D:\SM`
- Branch: `codex/p10-p11-clean-upload-20260420`
- HEAD: local branch now includes the merged docfix line on top of `8214bc7`

## Working Tree

- this local `D:\SM` worktree continues the same dirty delivery line and now carries the P19D controlled replay commit-writer slice on top of the P19C replay commit-preflight state
- 2026-04-27 update: the Nikkei ETF HGB/RF V3 research chain has been packaged under `docs/research/nikkei-etf-hgb-rf-v3-20260427/`, including full Nikkei training/intermediate artifacts, ETF live-like backtest outputs, daily HGB/RF scoring outputs, an artifact manifest, upload notes, and an algorithm handoff manual.
- For future Nikkei ETF model work, start with `docs/research/nikkei-etf-hgb-rf-v3-20260427/ALGORITHM_HANDOFF_MANUAL.md`; use `live_pre_year` outputs for live-like interpretation and treat `known_labels_asof` as diagnostic only.
- 2026-04-28 update: the current Nikkei operator entrypoint is `python D:\SM\scripts\run_nikkei_hgb_rf_daily_workflow.py --as-of-date <date> --score-start-date <date> --journal-dir D:\SM\docs\trading-journal\nikkei`; the workflow prints `effective_signal_date`, writes `06_daily_workflow_manifest_live_pre_year.json`, and may fall back to the latest live artifact on or before the requested date.
- 2026-04-28 update: the current 2026 `live_pre_year` daily workflow keeps a fixed live split of `train through 2025-09-30` and `validate on 2025Q4`, then scores daily rows; the expanding-window cadence belongs to governed retrain / yearly HGB walk-forward, not to day-by-day operator retraining.
- 2026-04-28 update: the formal ETF Tool boundary is now `src/ops/security_nikkei_etf_position_signal.rs`, which accepts only governed `live_pre_year` HGB artifacts and rejects `known_labels_asof` plus deprecated non-policy filenames.
- 2026-04-28 update: the Nikkei live journal truth now lives under `D:\SM\docs\trading-journal\nikkei\` with `journal.csv`, `journal.md`, and `snapshots\*.json`; rating-change replay uses `python C:\Users\wakes\.codex\skills\nikkei-live-journal\scripts\compare_rating_change.py --journal-dir D:\SM\docs\trading-journal\nikkei --signal-date <date> --etf-symbol <symbol>`.
- 2026-04-28 update: code/tests now enforce the Nikkei weekly `1w` registry/refit token, and the packaged Nikkei research snapshot was rerun on 2026-04-28 so the old `...10d-direction_head` training metadata residue has been cleared from that snapshot.
- 2026-04-28 update: the first offline Nikkei replay-classifier line now exists under `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts/04_replay_classifier_full_snapshot/`; start with `REPLAY_CLASSIFIER_SUMMARY_20260428.md` for signal-quality replay truth.
- 2026-04-28 update: the first offline Nikkei continuation-head line now exists under `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts/05_continuation_head_full_snapshot/`; read it together with `CONTINUATION_HEAD_SUMMARY_20260428.md` as a second-stage refinement layer on top of replay classification.
- 2026-04-28 update: the current continuation-head truth is research-only; the main gap is no longer whether continuation exists, but whether its highly imbalanced labels can be optimized enough for future operator use.
- 2026-04-29 update: the first simulated-action balance experiment now exists under `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts/06_simulated_action_balance_experiment/`; use `SIMULATED_ACTION_BALANCE_SUMMARY_20260429.md` before approving any synthetic augmentation into governed training.
- 2026-04-29 update: the first augmentation did not improve real-validation balance-aware metrics; the active gap is now negative-sample quality, not just negative-sample quantity.
- 2026-04-29 update: the second continuation-balance pass now exists under `docs/research/nikkei-etf-hgb-rf-v3-20260427/artifacts/07_real_failure_event_experiment/`; read `REAL_FAILURE_EVENT_SUMMARY_20260429.md` before approving any real-failure augmentation into governed training.
- 2026-04-29 update: the later prototype-add refinement made the `07_real_failure_event_experiment` add-only and closer to untouched-validation `premature_add` negatives; this improved `1D / 3D` balance-aware metrics versus the prior broad real-failure pass, but still did not beat baseline, so the active gap is now the remaining subtype split inside prototype add failures, especially on `5D`.
- 2026-04-29 update: the next `5D` specialization split the shared add prototype into two `5D` slow-fail subcontexts and slightly lifted `5D balanced_accuracy` above baseline, but the effective mined train count before validation is only `1`, so the active gap has shifted to `5D` sample density under the time-aware split.
- 2026-04-29 update: the dedicated prediction-methods handoff now lives at `docs/research/nikkei-etf-hgb-rf-v3-20260427/PREDICTION_METHODS_HANDOFF_20260429.md`; use it when the immediate question is the `Replay Classifier / Continuation Head` line rather than the base HGB/RF V3 model.
- historical `E:\SM` verification entries below are preserved as background evidence only; current branch-health claims for this continuation must be made from fresh `D:\SM` commands
- the working tree still contains many uncommitted local edits and generated runtime artifacts, including `P16` governance-sync work, capital-source raw-flow/snapshot/audit work, earlier portfolio-core/post-P12 edits, user-local Nikkei training changes, and large runtime fixture/output directories
- the latest branch-truth imported from the docfix line records a fresh 2026-04-24 repository-wide green verification in a separate reconciliation worktree after the missing `docs/plans/design/` backfill and the `Stock/Foundation Decoupling Baseline` handoff marker were restored
- the local `E:\SM` worktree itself should still be treated as dirty and not implicitly equivalent to that clean verification worktree until reverified here

## Verified Commands

### Passed

Fresh Nikkei focused verification run on 2026-04-28 in `D:\SM`:

```powershell
$env:CARGO_TARGET_DIR='D:\SM\.verification\cargo-targets\nikkei_refit_20260428'; cargo test --test security_scorecard_refit_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\.verification\cargo-targets\nikkei_training_20260428'; cargo test --test security_scorecard_training_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\.verification\cargo-targets\nikkei_etf_signal_20260428'; cargo test --test security_nikkei_etf_position_signal_cli -- --nocapture
python D:\SM\scripts\test_run_nikkei_hgb_rf_daily_workflow.py
python C:\Users\wakes\.codex\skills\nikkei-live-journal\scripts\test_upsert_journal.py
$env:CARGO_TARGET_DIR='D:\SM\.verification\cargo-targets\nikkei_check_20260428'; cargo check
python D:\SM\scripts\run_nikkei_hgb_rf_daily_workflow.py --as-of-date 2026-04-27 --score-start-date 2026-04-01 --output-root D:\SM\.verification\nikkei_daily_workflow_20260428
```

Results:

- `security_scorecard_refit_cli`: `4 passed, 0 failed`
- `security_scorecard_training_cli`: `19 passed, 0 failed`
- `security_nikkei_etf_position_signal_cli`: `15 passed, 0 failed`
- `test_run_nikkei_hgb_rf_daily_workflow.py`: `4 passed, 0 failed`
- `test_upsert_journal.py`: `4 passed, 0 failed`
- `cargo check`: completed successfully
- real daily workflow run completed successfully with `train_policy=live_pre_year`, `as_of_date=2026-04-27`, `effective_signal_date=2026-04-24`, `HGB adjustment=-1`, `RF adjustment=0`
- the real daily workflow emitted a non-blocking `joblib/loky` physical-core warning on Windows `wmic`; the workflow still completed with exit code 0 and wrote outputs to `D:\SM\.verification\nikkei_daily_workflow_20260428`

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

Fresh local focused recovery run on 2026-04-25 in `D:\SM` for the rebuilt P17/P18 chain:

```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p17_recovery_green'; cargo test --test security_portfolio_execution_reconciliation_bridge_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p18_recovery_green'; cargo test --test security_portfolio_execution_repair_package_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p17_p18_recovery_guard'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p17_p18_recovery_guard'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p17_p18_recovery_guard'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p17_p18_recovery_final'; cargo test --test security_portfolio_execution_reconciliation_bridge_cli --test security_portfolio_execution_repair_package_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p17_p18_recovery_final'; cargo check
$env:CARGO_TARGET_DIR='D:\SM\target_p17_p18_recovery_verify_after_fmt'; cargo test --test security_portfolio_execution_reconciliation_bridge_cli --test security_portfolio_execution_repair_package_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p17_p18_recovery_verify_after_fmt'; cargo check
$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p18'; cargo test -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p18_confirm'; cargo test -- --nocapture
```

Results:

- P17 focused CLI contract: `4 passed, 0 failed`
- P18 focused CLI contract: `6 passed, 0 failed`
- boundary guard: `4 passed, 0 failed`
- catalog grouping guard: `2 passed, 0 failed`
- dispatcher grouping guard: `1 passed, 0 failed`
- final combined P17/P18 focused run: `10 passed, 0 failed`
- final after-format combined P17/P18 focused run: `10 passed, 0 failed`
- `cargo check` completed successfully before and after `cargo fmt`
- follow-up repository-wide regression in `D:\SM`: `cargo test -- --nocapture` completed with exit code 0 after the P17/P18 recovery
- confirmation repository-wide regression in `D:\SM`: `cargo test -- --nocapture` completed with exit code 0 using `D:\SM\target_repo_full_after_p18_confirm`

Fresh local focused run on 2026-04-25 in `D:\SM` for the new P19A replay-request package:

```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_red'; cargo test --test security_portfolio_execution_replay_request_package_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_green'; cargo test --test security_portfolio_execution_replay_request_package_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_guard'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_guard'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_guard'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_final'; cargo test --test security_portfolio_execution_replay_request_package_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_final'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_final'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_final'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_final'; cargo check
$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p19a'; cargo test -- --nocapture
```

Results:

- P19A RED test first failed with `unsupported tool: security_portfolio_execution_replay_request_package`
- P19A focused CLI contract: `5 passed, 0 failed`
- first formal boundary guard exposed existing dirty-worktree manifest drift for `security_volume_source_manifest`; the guard expectation was aligned to the active stock boundary before final P19A verification
- final boundary guard: `4 passed, 0 failed`
- final catalog grouping guard: `2 passed, 0 failed`
- final dispatcher grouping guard: `1 passed, 0 failed`
- final `cargo check` completed successfully
- follow-up repository-wide regression in `D:\SM`: `cargo test -- --nocapture` completed with exit code 0 using `D:\SM\target_repo_full_after_p19a`

Fresh local focused run on 2026-04-25 in `D:\SM` for the new P19B dry-run replay executor:

```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_red'; cargo test --test security_portfolio_execution_replay_executor_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_green'; cargo test --test security_portfolio_execution_replay_executor_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_final'; cargo test --test security_portfolio_execution_replay_executor_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_final'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_final'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_final'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_final'; cargo check
```

Results:

- P19B RED test first failed with `unsupported tool: security_portfolio_execution_replay_executor`
- P19B focused CLI contract: `7 passed, 0 failed`
- final boundary guard: `4 passed, 0 failed`
- final catalog grouping guard: `2 passed, 0 failed`
- final dispatcher grouping guard: `1 passed, 0 failed`
- final `cargo check` completed successfully
- repository-wide `cargo test -- --nocapture` has not been rerun after P19B; the latest repository-wide pass remains the pre-P19B run after P19A

Fresh repository-wide regression after P19B on 2026-04-25 in `D:\SM`:

```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p19b'; cargo test -- --nocapture
```

Result:

- Failed with exit code 1.
- Reproduced with the focused command:
  `$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p19b'; cargo test --test stock_formal_boundary_manifest_source_guard stock_root_keeps_only_the_frozen_module_manifest -- --nocapture`
- Blocking test: `stock_formal_boundary_manifest_source_guard::stock_root_keeps_only_the_frozen_module_manifest`.
- Failure reason: `src/ops/stock.rs` exposes `security_nikkei_turnover_import`, but the frozen public stock-boundary manifest in `tests/stock_formal_boundary_manifest_source_guard.rs` does not yet include that module.
- Current interpretation: this is boundary-manifest drift from the parallel Nikkei official turnover import slice, not a P19B executor contract failure.

Fresh repository-wide recheck after resolving the P19B follow-up boundary drift on 2026-04-26 in `D:\SM`:

```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19b_boundary_recheck'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19b_boundary_recheck'; cargo test --test security_nikkei_turnover_import_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p19b_recheck'; cargo test -- --nocapture
```

Results:

- formal boundary guard: `4 passed, 0 failed`
- Nikkei turnover import CLI: `2 passed, 0 failed`
- P19B follow-up repository-wide regression in `D:\SM`: `cargo test -- --nocapture` completed with exit code 0 using `D:\SM\target_repo_full_after_p19b_recheck`
- current branch-health read for the dirty `D:\SM` worktree is repository-wide green after P19B recheck, subject to the unchanged caveat that the worktree remains dirty with unrelated and generated artifacts

Fresh local focused run on 2026-04-26 in `D:\SM` for the new P19C replay commit-preflight contract:

```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_red'; cargo test --test security_portfolio_execution_replay_commit_preflight_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_green'; cargo test --test security_portfolio_execution_replay_commit_preflight_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_risk_red'; cargo test --test security_portfolio_execution_replay_commit_preflight_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_risk_green'; cargo test --test security_portfolio_execution_replay_commit_preflight_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_final'; cargo test --test security_portfolio_execution_replay_commit_preflight_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_final'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_final'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_final'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_final'; cargo check
$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p19c'; cargo test -- --nocapture
```

Results:

- P19C RED test first failed with `unsupported tool: security_portfolio_execution_replay_commit_preflight`
- independent P19C risk pass then found missing formal P19B/P14 identity checks, blocked-P14 no-work masking risk, and missing durable source guard
- follow-up RED added those checks and failed as expected with 3 failing tests before implementation
- P19C focused CLI contract then passed with `14 passed, 0 failed`
- P19C is intentionally preflight-only: it consumes P19B dry-run truth plus matching P14 enrichment, freezes future commit idempotency/hash evidence, keeps `runtime_write_count = 0`, rejects runtime refs, rejects P19B commit mode, and does not call `security_execution_record`
- final P19C focused CLI contract passed with `14 passed, 0 failed`
- final formal boundary guard passed with `4 passed, 0 failed`
- final catalog grouping guard passed with `2 passed, 0 failed`
- final dispatcher grouping guard passed with `1 passed, 0 failed`
- final `cargo check` completed successfully
- repository-wide regression after P19C completed with exit code 0 using `D:\SM\target_repo_full_after_p19c`

Fresh local focused run on 2026-04-26 in `D:\SM` for the new P19D controlled replay commit writer:

```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_execution_record_red'; cargo test --test security_execution_record_cli security_execution_record_replay_control -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_execution_record_green'; cargo test --test security_execution_record_cli security_execution_record_replay_control -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_red'; cargo test --test security_portfolio_execution_replay_commit_writer_cli tool_catalog_includes_security_portfolio_execution_replay_commit_writer -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_green'; cargo test --test security_portfolio_execution_replay_commit_writer_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_final'; cargo test --test security_portfolio_execution_replay_commit_writer_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_final'; cargo test --test security_execution_record_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_final'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_final'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_final'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_final'; cargo check
$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p19d'; cargo test -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p19d_fmt'; cargo test -- --nocapture
```

Results:

- P19D first added `SecurityExecutionReplayCommitControl` to `security_execution_record`; RED failed because deterministic replay ids and machine-readable metadata were absent, then GREEN passed with `2 passed, 0 failed`
- P19D writer RED first failed because `security_portfolio_execution_replay_commit_writer` was not in the catalog
- P19D focused CLI contract then passed with `6 passed, 0 failed`
- final P19D focused CLI contract passed with `6 passed, 0 failed`
- final adjacent `security_execution_record_cli` passed with `7 passed, 0 failed`
- final formal boundary guard passed with `4 passed, 0 failed`
- final catalog grouping guard passed with `2 passed, 0 failed`
- final dispatcher grouping guard passed with `1 passed, 0 failed`
- final `cargo check` completed successfully
- repository-wide regression after P19D completed with exit code 0 using `D:\SM\target_repo_full_after_p19d`
- post-format repository-wide regression also completed with exit code 0 using `D:\SM\target_repo_full_after_p19d_fmt`
- P19D is intentionally controlled per-row and non-atomic across rows: it consumes P19C preflight evidence, writes runtime records only through `security_execution_record`, detects `already_committed` from machine-readable replay metadata, rejects idempotency conflicts without overwrite, and keeps broker execution/lifecycle closeout out of scope

Fresh local focused run on 2026-04-26 in `D:\SM` for the new P19E replay commit audit:

```powershell
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_red'; cargo test --test security_portfolio_execution_replay_commit_audit_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_green'; cargo test --test security_portfolio_execution_replay_commit_audit_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_guard'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_guard'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_guard'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_adjacent'; cargo test --test security_portfolio_execution_replay_commit_writer_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_final'; cargo test --test security_portfolio_execution_replay_commit_audit_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_final'; cargo test --test security_portfolio_execution_replay_commit_writer_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_final'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_final'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_final'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_final'; cargo check
$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p19e'; cargo test -- --nocapture
```

Results:

- P19E RED test first failed with `unsupported tool: security_portfolio_execution_replay_commit_audit` and missing source file evidence after the test fixture compile issue was corrected.
- P19E focused CLI contract passed with `9 passed, 0 failed`.
- P19E final focused CLI contract passed with `9 passed, 0 failed`.
- adjacent P19D replay commit writer test passed with `6 passed, 0 failed`.
- final formal boundary guard passed with `4 passed, 0 failed`.
- final catalog grouping guard passed with `2 passed, 0 failed`.
- final dispatcher grouping guard passed with `1 passed, 0 failed`.
- final `cargo check` completed successfully.
- repository-wide regression after P19E completed with exit code 0 using `D:\SM\target_repo_full_after_p19e`.
- P19E is intentionally read-only: it consumes P19D commit-writer truth, reads runtime execution records to verify machine-readable replay metadata, preserves missing/mismatch/failure/conflict audit states, keeps `runtime_write_count = 0`, and does not call `security_execution_record`.

P20A lifecycle closeout readiness is complete in `D:\SM` as the approved side-effect-free consumer of P19E audit truth:

- `security_portfolio_execution_lifecycle_closeout_readiness` maps only P19E `verified` and `already_committed_verified` rows to `eligible_for_closeout_preflight`.
- P19E `missing_runtime_record`, `metadata_mismatch`, `commit_failed_preserved`, `idempotency_conflict_confirmed`, `skipped_no_commit_work_preserved`, `not_auditable`, and unknown states remain explicit P20A blockers.
- P20A keeps `runtime_write_count = 0` and must not call `security_execution_record`, `security_post_trade_review`, or `security_closed_position_archive`.
- P20A is not broker execution, broker-fill replay, position materialization, lifecycle closure, or a closed-position archive writer.
- Focused P20A GREEN passed locally with `9 passed, 0 failed`.
- Final P20A focused verification passed with `9 passed, 0 failed`.
- Final adjacent P19E focused verification passed with `9 passed, 0 failed`.
- Final formal boundary guard passed with `4 passed, 0 failed`.
- Final catalog grouping guard passed with `2 passed, 0 failed`.
- Final dispatcher grouping guard passed with `1 passed, 0 failed`.
- Final `cargo check` completed successfully using `D:\SM\target_p20a_closeout_readiness_final`.
- Repository-wide regression after P20A completed with exit code 0 using `D:\SM\target_repo_full_after_p20a`.
- During full regression, the first run exposed a self-inflicted rustfmt recursion issue: formatting `src/ops/stock.rs` also reformatted frozen `security_decision_committee.rs`; that formatting-only drift was reverted, the legacy freeze guard passed, and the full regression was rerun successfully.

P20B lifecycle closeout evidence package is complete in `D:\SM` as the approved read-only consumer of P20A readiness truth:

- `security_portfolio_execution_lifecycle_closeout_evidence_package` point-reads target runtime execution records only for P20A `eligible_for_closeout_preflight` rows.
- P20B maps closed, metadata-matching runtime records to `evidence_ready_for_closeout_archive_preflight`.
- P20B preserves P20A blocked rows without runtime reads as `blocked_p20a_not_eligible`.
- P20B keeps `runtime_write_count = 0` and must not call `security_execution_record`, `security_post_trade_review`, or `security_closed_position_archive`.
- P20B is not broker execution, broker-fill replay, position materialization, archive production, lifecycle closure, or a closed-position archive writer.
- P20B RED failed with unsupported tool/source missing as expected, and focused P20B GREEN passed locally with `11 passed, 0 failed`.
- Final P20B focused verification passed with `11 passed, 0 failed`.
- Final adjacent P20A verification passed with `9 passed, 0 failed`.
- Final formal boundary guard passed with `4 passed, 0 failed`.
- Final catalog grouping guard passed with `2 passed, 0 failed`.
- Final dispatcher grouping guard passed with `1 passed, 0 failed`.
- Final `cargo check` completed successfully using `D:\SM\target_p20b_closeout_evidence_final`.
- Repository-wide regression after P20B completed with exit code 0 using `D:\SM\target_repo_full_after_p20b`.

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

- the Nikkei live operator entrypoint is now `scripts/run_nikkei_hgb_rf_daily_workflow.py`, and the approved live interpretation must read both the requested `as_of_date` and the resolved `effective_signal_date`
- the formal `security_nikkei_etf_position_signal` boundary now consumes only governed `live_pre_year` HGB artifacts and rejects `known_labels_asof`, stale-date filenames, and deprecated non-policy filenames
- the Nikkei live journal path under `docs/trading-journal/nikkei/` is now part of the governed delivery line, with signal-fact persistence owned by the daily workflow and rating-change replay owned by the journal skill script
- the merged local branch contains the P10/P11/P12 portfolio-core slice plus the post-P12 P13 request package, P14 request enrichment, P15 apply-bridge path, and your local carried-forward P16 plus capital-source line
- the new `security_portfolio_allocation_decision` tool is live on the public stock bus as an enhanced bounded P12 refinement layer with baseline-vs-refined allocation traceability
- the new `security_portfolio_execution_preview` tool is live on the public stock bus as a preview-only downstream consumer of governed P12 output, and each preview row now carries a nested execution-request-aligned preview subset
- the new `security_portfolio_execution_request_package` tool is live on the public stock bus as a formal side-effect-free P13 request bridge downstream of the standardized preview document
- the new `security_portfolio_execution_request_enrichment` tool is live on the public stock bus as a formal side-effect-free P14 enrichment bridge downstream of the P13 request package
- the new `security_portfolio_execution_apply_bridge` tool is live on the public stock bus as a formal governed P15 apply bridge downstream of the P14 enrichment bundle, and it surfaces `applied` / `partial_success` / `failed` row outcomes explicitly
- the new `security_portfolio_execution_repair_package` tool is live on the public stock bus as a formal side-effect-free P18 repair-intent package downstream of the P17 reconciliation artifact
- the new `security_portfolio_execution_replay_request_package` tool is live on the public stock bus as a formal side-effect-free P19A replay-request package downstream of the P18 repair package
- the new `security_portfolio_execution_replay_executor` tool is live on the public stock bus as a formal dry-run-only P19B replay executor downstream of the P19A replay request package
- the new `security_portfolio_execution_replay_commit_preflight` tool is live on the public stock bus as a formal side-effect-free P19C preflight layer downstream of the P19B dry-run executor; it freezes future commit payload hashes and idempotency candidates but does not write runtime facts
- the new `security_portfolio_execution_replay_commit_writer` tool is live on the public stock bus as a formal P19D controlled per-row replay commit writer downstream of P19C; it writes only through `security_execution_record`, uses deterministic replay refs, and exposes non-atomic per-row commit status
- the new `security_portfolio_execution_replay_commit_audit` tool is live on the public stock bus as a formal read-only P19E runtime verification layer downstream of P19D; it verifies machine-readable replay metadata and keeps `runtime_write_count = 0`
- the new `security_portfolio_execution_lifecycle_closeout_readiness` tool is live on the public stock bus as a formal side-effect-free P20A readiness layer downstream of P19E; it emits closeout preflight eligibility only and does not close lifecycle or write archive/runtime/post-trade facts
- the new `security_portfolio_execution_lifecycle_closeout_evidence_package` tool is live on the public stock bus as a formal read-only P20B evidence layer downstream of P20A; it emits closeout evidence readiness only and does not close lifecycle or write archive/runtime/post-trade facts
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
- the local carried-forward post-P15 line should now be treated as freshly re-verified through `P18` focused contract verification, `P17` boundary and grouping guards, and a 2026-04-25 `D:\SM` repository-wide `cargo test -- --nocapture` pass after the P17/P18 recovery

## Current Gaps Still Visible

- the fresh 2026-04-28 governed daily workflow still falls back to `effective_signal_date=2026-04-24`; the active gap is now the daily live artifact freshness, not the old `10d` registry/refit residue
- the current 2026 Nikkei daily workflow still depends on an external Python preprocessing/scoring step rather than an internal Rust training subsystem
- the Windows `joblib/loky` physical-core warning remains a local environment nuisance during the real workflow run; it did not block execution in the 2026-04-28 fresh run
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
- the new P17 reconciliation bridge is intentionally a reconciliation-truth freeze layer; it must not be mistaken for repair execution, broker-fill replay, position materialization, or lifecycle closure
- the new P18 repair package is intentionally a repair-intent freeze layer; it must not be mistaken for runtime replay, broker execution, position materialization, or lifecycle closure
- the new P19A replay request package is intentionally a request-freeze layer; it must not be mistaken for runtime replay, broker execution, position materialization, or lifecycle closure
- the new P19B replay executor is intentionally dry-run-only; it rejects commit mode and must not be mistaken for runtime replay, broker execution, position materialization, or lifecycle closure
- the new P19C replay commit preflight is intentionally preflight-only; it must not be mistaken for P19B commit mode, `security_execution_record`, runtime replay, broker execution, position materialization, or lifecycle closure
- the new P19D replay commit writer is intentionally a controlled runtime writer, not a broker executor or lifecycle closeout; it must keep P19B commit mode and P19C runtime writes forbidden, and future changes must preserve machine-readable replay metadata plus source guards against direct runtime writes
- the new P19E replay commit audit is intentionally read-only; it must not call `security_execution_record`, write runtime facts, replay broker fills, materialize positions, or claim lifecycle closure
- the new P20A lifecycle closeout readiness layer is intentionally preflight-only; it must not call `security_execution_record`, `security_post_trade_review`, or `security_closed_position_archive`, write runtime facts, replay broker fills, materialize positions, or claim lifecycle closure
- the new P20B lifecycle closeout evidence layer is intentionally read-only and pre-archive; it must not call `security_execution_record`, `security_post_trade_review`, or `security_closed_position_archive`, write runtime facts, replay broker fills, materialize positions, produce archives, or claim lifecycle closure
- the earlier P19B repository-wide regression blocker for `security_nikkei_turnover_import` formal boundary drift was resolved by the 2026-04-26 focused boundary recheck and repository-wide rerun in `D:\SM`
- the new standardized isolated Cargo runner closed the earlier `security_scorecard_training_cli` verification-pollution ambiguity; the later P20A repository-wide regression in `D:\SM\target_repo_full_after_p20a` completed with exit code 0, so current branch-health claims should cite that newer run rather than the older capital-source blocker note
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
