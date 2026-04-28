# Scripts

## `invoke_isolated_cargo.ps1`

Use this script for Windows-local verification when reused `target/` directories or stale test binaries may pollute the result.

### Examples

```powershell
.\scripts\invoke_isolated_cargo.ps1 -RunLabel smoke_guard -CargoCommand test -CargoArguments @('--test','stock_formal_boundary_manifest_source_guard','--','--nocapture')
.\scripts\invoke_isolated_cargo.ps1 -RunLabel scorecard_full -CargoCommand test -CargoArguments @('--test','security_scorecard_training_cli','--','--nocapture')
.\scripts\invoke_isolated_cargo.ps1 -RunLabel repo_full -CargoCommand test -CargoArguments @('--','--nocapture')
.\scripts\invoke_isolated_cargo.ps1 -RunLabel repo_check -CargoCommand check
```

By default the script writes:

- isolated targets under `.verification/cargo-targets/`
- logs under `.verification/logs/`

Use `-NoLog` if you only need stdout/stderr passthrough.

## `run_nikkei_hgb_rf_daily_workflow.py`

2026-04-28 CST: Added because the Nikkei live workflow now separates expand-window retrain from daily walk-forward scoring.
Purpose: run one governed `live_pre_year` scoring batch, then print a stable HGB/RF summary from policy-qualified artifacts only.

### Example

```powershell
python D:\SM\scripts\run_nikkei_hgb_rf_daily_workflow.py --as-of-date 2026-04-27 --score-start-date 2026-04-01
```

Supported options:

- `--as-of-date`
- `--score-start-date`
- `--analysis-root`
- `--output-root`
- `--journal-dir`

## Prediction-Enhancement Scripts

These scripts belong to the Nikkei `Replay Classifier / Continuation Head`
research line.

Core entry points:

- `build_nikkei_replay_samples.py`
- `train_nikkei_replay_classifier.py`
- `train_nikkei_continuation_head.py`
- `run_nikkei_simulated_action_balance.py`
- `run_nikkei_real_failure_event_balance.py`

Primary tests:

- `test_nikkei_replay_classifier.py`
- `test_nikkei_continuation_head.py`
- `test_nikkei_simulated_action_balance.py`
- `test_nikkei_real_failure_event_balance.py`

These scripts are research-layer tooling. They do not replace the governed
HGB/RF V3 adjustment workflow.
