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
