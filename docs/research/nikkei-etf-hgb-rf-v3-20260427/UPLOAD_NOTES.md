# Upload Notes

## Scope

This upload package is intentionally scoped to the Nikkei ETF HGB/RF V3 research line.

Included:

- Full Nikkei training/intermediate research snapshot.
- Full ETF live-like backtest snapshot.
- Full daily HGB/RF scoring snapshot.
- File manifest with SHA256 hashes.
- Chinese model summary for fast review.
- Algorithm handoff manual.
- README with conclusions and reproduction notes.

Excluded:

- `target_*` build directories.
- Runtime fixture spam under `tests/runtime_fixtures/`.
- `.playwright-cli/`.
- A-share/HS300 runtime data because it is approximately 577.64MB and is not part of the active Nikkei ETF execution line.

## 2026-04-29 Prediction-Methods Handoff Addendum

This upload branch additionally packages the prediction-enhancement research line:

- `Replay Classifier`
- `Continuation Head`
- simulated balance experiment
- real failure mining experiments
- prototype-add and `5D` slow-fail follow-up documents

Included for this addendum:

- `PREDICTION_METHODS_HANDOFF_20260429.md`
- `REPLAY_CLASSIFIER_SUMMARY_20260428.md`
- `CONTINUATION_HEAD_SUMMARY_20260428.md`
- `SIMULATED_ACTION_BALANCE_SUMMARY_20260429.md`
- `REAL_FAILURE_EVENT_SUMMARY_20260429.md`
- `artifacts/04_replay_classifier_full_snapshot/`
- `artifacts/05_continuation_head_full_snapshot/`
- `artifacts/06_simulated_action_balance_experiment/`
- `artifacts/07_real_failure_event_experiment/`
- related Python scripts and tests under `scripts/`

Known current research boundary:

- the two prediction methods are enhancement layers on top of HGB/RF V3;
- they are not standalone replacements for the base position-sizing model;
- the active bottleneck is now `5D` pre-validation slow-fail sample density.

Fresh verification used for this handoff branch:

```powershell
python D:\SM\scripts\test_nikkei_real_failure_event_balance.py
python D:\SM\scripts\test_nikkei_continuation_head.py
python D:\SM\scripts\test_nikkei_replay_classifier.py
python D:\SM\scripts\test_run_nikkei_hgb_rf_daily_workflow.py
python C:\Users\wakes\.codex\skills\nikkei-live-journal\scripts\test_upsert_journal.py
```

Results:

- `test_nikkei_real_failure_event_balance.py`: `5 passed, 0 failed`
- `test_nikkei_continuation_head.py`: `3 passed, 0 failed`
- `test_nikkei_replay_classifier.py`: `6 passed, 0 failed`
- `test_run_nikkei_hgb_rf_daily_workflow.py`: `4 passed, 0 failed`
- `test_upsert_journal.py`: `4 passed, 0 failed`

Artifact refresh used for this handoff branch:

- `artifact_manifest.csv` regenerated to include the prediction-methods handoff doc and the `artifacts/04-07` research outputs.

## Fresh Verification Evidence

Commands run during package preparation:

```powershell
$dirs=@('D:\.stockmind_runtime\nikkei_etf_daily_model_scoring_20260427','D:\.stockmind_runtime\nikkei_etf_live_like_backtest_20260426','D:\.stockmind_runtime\nikkei_current_rerun_20260426_direction_head_yfinance_10y_long_volume_behavior','D:\.stockmind_runtime\a_share_dynamic_hs300_backtest_20260426')
```

Result:

- Daily HGB/RF scoring snapshot: 17 files, 0.15MB.
- Live-like ETF backtest snapshot: 12 files, 2.65MB.
- Nikkei training/intermediate snapshot: 181 files, 13.04MB.
- A-share/HS300 side experiment: 67 files, 577.64MB, excluded.

```powershell
artifact policy check passed: 4/4
```

This verified that the new policy-qualified JSON artifacts match their internal `train_policy` values.

```powershell
artifact_manifest.csv generated
```

Result:

- 210 included artifact files.
- 15.84MB total included artifact size.

```powershell
python docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\03_daily_hgb_rf_scoring_full_snapshot\daily_hgb_rf_v3_scoring.py --analysis-root docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\01_training_and_intermediate_full_snapshot\analysis_exports --output-root D:\.stockmind_runtime\nikkei_package_verify_20260427 --train-policy live_pre_year
```

Result:

- HGB latest live policy output on 2026-04-24: adjustment `-1`, target proxy `0.122423`.
- RF latest live policy output on 2026-04-24: adjustment `0`, target proxy `0.372423`.
- The packaged snapshot can reproduce the live scoring results when `--analysis-root` is pointed at the packaged artifact directory.

```powershell
$env:CARGO_TARGET_DIR='D:\.stockmind_runtime\cargo_target_nikkei_upload_tool'; cargo test --test security_nikkei_etf_position_signal_cli -- --nocapture
$env:CARGO_TARGET_DIR='D:\.stockmind_runtime\cargo_target_nikkei_upload_catalog'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\.stockmind_runtime\cargo_target_nikkei_upload_dispatcher'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture
$env:CARGO_TARGET_DIR='D:\.stockmind_runtime\cargo_target_nikkei_upload_check'; cargo check
```

Result:

- `security_nikkei_etf_position_signal_cli`: 11 passed, 0 failed.
- `stock_catalog_grouping_source_guard`: 2 passed, 0 failed.
- `stock_dispatcher_grouping_source_guard`: 1 passed, 0 failed.
- `cargo check`: completed successfully.

```powershell
artifact manifest hash check passed: 210/210
```

## Known Risks

- `known_labels_asof` is diagnostic and must not be used as a live signal.
- Old non-policy JSON files are preserved in the full snapshot for traceability, but they are deprecated.
- The daily scoring script currently has absolute default paths pointing at the original runtime directory; use explicit arguments or patch defaults for cross-machine replay.
- ETF premium backtest uses proxy logic and is not a substitute for real-time IOPV.

## Suggested Review Order

1. `README.md`
2. `MODEL_SUMMARY_20260428.md`
3. `ALGORITHM_HANDOFF_MANUAL.md`
4. `artifact_manifest.csv`
5. `artifacts/03_daily_hgb_rf_scoring_full_snapshot/05_latest_adjustment_artifacts_live_pre_year.csv`
6. `artifacts/01_training_and_intermediate_full_snapshot/analysis_exports/adjustment_point_analysis/64_walk_forward_hgb_backtest_summary.csv`
7. `artifacts/02_live_like_backtest_full_snapshot/08_no_deadband_decision_summary.csv`
