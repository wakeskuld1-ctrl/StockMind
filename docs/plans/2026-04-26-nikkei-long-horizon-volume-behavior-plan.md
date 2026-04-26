# Nikkei Long-Horizon Volume Behavior Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add medium and long-horizon weekly volume behavior features so Nikkei training can inspect quarterly, half-year, and yearly accumulation patterns instead of relying only on a 4-week volume ratio.

**Architecture:** Extend the existing Nikkei weekly feature path inside `security_scorecard_training.rs`. Reuse the current weekly bucket abstraction, keep volume-source priority unchanged, and add leak-free prior-window calculations before model fitting.

**Tech Stack:** Rust, Cargo tests, existing `security_scorecard_training` CLI tests, existing stock-history runtime fixtures.

---

### Risk Synchronization Gate
**Risk subprocess mode:** inline-fresh-pass
**Question asked:** What artifact will drift if the weekly training feature contract is expanded?
**Boundary items:**
- Weekly Nikkei model artifact feature list.
- Weekly Nikkei diagnostic `feature_coverage_summary`.
- Existing tests that assert feature names or feature counts.
**Must-sync files:**
- `D:\SM\src\ops\security_scorecard_training.rs`
- `D:\SM\tests\security_scorecard_training_cli.rs`
- `D:\SM\task_plan.md`
- `D:\SM\progress.md`
- `D:\SM\findings.md`
- `D:\SM\.trae\CHANGELOG_TASK.md`
**Must-run checks:**
- `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_long_volume_red'; cargo test weekly_price_aggregation_emits_distribution_quantiles_for_nikkei_training --test security_scorecard_training_cli -- --nocapture`
- `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_long_volume_green'; cargo test weekly_price_aggregation_emits_distribution_quantiles_for_nikkei_training --test security_scorecard_training_cli -- --nocapture`
- `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_long_volume_proxy_green'; cargo test security_scorecard_training_nikkei_weekly_uses_volume_proxy_without_futures_features --test security_scorecard_training_cli -- --nocapture`
- `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_long_volume_weekly'; cargo test weekly_ --test security_scorecard_training_cli -- --nocapture`
- `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_long_volume_check'; cargo check`
**Blockers:**
- Do not implement if the feature names or neutral defaults are changed without updating this plan.
- Do not claim model improvement until a real `NK225_VOL.YFINANCE` rerun is completed and compared.

### Task 1: Add Failing Contract Test For Long-Horizon Weekly Volume Features
**Files:**
- Modify: `D:\SM\tests\security_scorecard_training_cli.rs`

**Step 1: Extend the weekly aggregation feature-name test**
Add assertions to `weekly_price_aggregation_emits_distribution_quantiles_for_nikkei_training` for:
- `weekly_volume_ratio_13w`
- `weekly_volume_ratio_26w`
- `weekly_volume_ratio_52w`
- `weekly_price_position_52w`
- `weekly_volume_accumulation_26w`
- `weekly_volume_accumulation_52w`
- `weekly_high_volume_low_price_signal`
- `weekly_high_volume_breakout_signal`

**Step 2: Add value variation assertions**
In the same test, collect `weekly_volume_ratio_13w`, `weekly_volume_ratio_26w`, and `weekly_price_position_52w`, and assert that at least one adjacent pair differs when the fixture has enough history.

**Step 3: Run the focused test and confirm RED**
Run: `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_long_volume_red'; cargo test weekly_price_aggregation_emits_distribution_quantiles_for_nikkei_training --test security_scorecard_training_cli -- --nocapture`
Expected: FAIL because the new feature names are not emitted yet.

### Task 2: Register New Weekly Feature Configs
**Files:**
- Modify: `D:\SM\src\ops\security_scorecard_training.rs`

**Step 1: Add feature configs**
Inside `training_feature_configs(...)`, in the `uses_nikkei_weekly_training_contract(request)` block, add numeric `TrainingFeatureConfig` entries for the eight new feature names.

**Step 2: Keep group naming stable**
Use group name `V` for all new volume behavior features.

**Step 3: Do not touch non-weekly feature configs**
No changes to bank, A-share, or 10D non-weekly feature contracts in this task.

### Task 3: Implement Leak-Free Long-Horizon Weekly Calculations
**Files:**
- Modify: `D:\SM\src\ops\security_scorecard_training.rs`

**Step 1: Add small helper functions**
Add private helpers near `build_weekly_price_feature_rows(...)`:
- `prior_weekly_volume_mean(...)`
- `prior_weekly_volume_ratio_mean(...)`
- `prior_weekly_close_position(...)`
- `price_position_in_prior_range(...)`

**Step 2: Compute ratios using only prior weeks**
For each weekly row:
- `weekly_volume_ratio_13w = current_total_volume / prior_13w_average_volume`
- `weekly_volume_ratio_26w = current_total_volume / prior_26w_average_volume`
- `weekly_volume_ratio_52w = current_total_volume / prior_52w_average_volume`

**Step 3: Compute yearly price position using only prior spot buckets**
Use the current spot weekly close against the prior 52-week low/high close range.

**Step 4: Compute accumulation fields**
Use conservative formulas:
- `weekly_volume_accumulation_26w = max(0, weekly_volume_ratio_26w - 1.0) * (1.0 - weekly_price_position_52w)`
- `weekly_volume_accumulation_52w = max(0, weekly_volume_ratio_52w - 1.0) * (1.0 - weekly_price_position_52w)`

**Step 5: Compute binary signals**
Use:
- `weekly_high_volume_low_price_signal = 1.0` when `weekly_volume_ratio_52w >= 1.10` and `weekly_price_position_52w <= 0.40`, else `0.0`
- `weekly_high_volume_breakout_signal = 1.0` when `weekly_volume_ratio_52w >= 1.10` and `weekly_price_position_52w >= 0.80`, else `0.0`

**Step 6: Preserve neutral defaults**
If the prior window is unavailable or prior average volume is zero:
- ratio fields default to `1.0`
- price position defaults to `0.5`
- accumulation and binary signals default to `0.0`

### Task 4: Run Focused GREEN Tests
**Files:**
- Test: `D:\SM\tests\security_scorecard_training_cli.rs`

**Step 1: Run the previously failing aggregation test**
Run: `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_long_volume_green'; cargo test weekly_price_aggregation_emits_distribution_quantiles_for_nikkei_training --test security_scorecard_training_cli -- --nocapture`
Expected: PASS.

**Step 2: Run volume-proxy regression**
Run: `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_long_volume_proxy_green'; cargo test security_scorecard_training_nikkei_weekly_uses_volume_proxy_without_futures_features --test security_scorecard_training_cli -- --nocapture`
Expected: PASS and still prove the proxy does not enable futures price features.

**Step 3: Run weekly group**
Run: `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_long_volume_weekly'; cargo test weekly_ --test security_scorecard_training_cli -- --nocapture`
Expected: PASS.

### Task 5: Run Real Nikkei Rerun With 10Y Volume Proxy
**Files:**
- Create runtime output under: `D:\.stockmind_runtime\nikkei_current_rerun_20260426_direction_head_yfinance_10y_long_volume_behavior`

**Step 1: Use the existing weekly request contract**
Use the same request as the previous yfinance 10Y proxy rerun, changing:
- `created_at = 2026-04-26T<current-time>+08:00`
- `artifact_runtime_root = D:\.stockmind_runtime\nikkei_current_rerun_20260426_direction_head_yfinance_10y_long_volume_behavior`
- `feature_set_version = nikkei_current_rerun_20260426_yfinance_10y_long_volume_behavior`

**Step 2: Keep the same data boundary**
Set:
- `EXCEL_SKILL_RUNTIME_DB = D:\.stockmind_runtime\nikkei_10y_market_20260425\runtime.db`
- `volume_proxy_symbol = NK225_VOL.YFINANCE`

**Step 3: Persist request and result**
Write `request.json` and `training_result.json` into the runtime output directory.

**Step 4: Extract comparison metrics**
Compare against:
- `D:\.stockmind_runtime\nikkei_current_rerun_20260425_direction_head\training_result.json`
- `D:\.stockmind_runtime\nikkei_current_rerun_20260425_direction_head_volume_proxy\training_result.json`
- `D:\.stockmind_runtime\nikkei_current_rerun_20260425_direction_head_yfinance_10y_volume_proxy\training_result.json`

### Task 6: Update Documentation And Journal
**Files:**
- Modify: `D:\SM\task_plan.md`
- Modify: `D:\SM\progress.md`
- Modify: `D:\SM\findings.md`
- Modify: `D:\SM\.trae\CHANGELOG_TASK.md`

**Step 1: Record implemented fields**
Document formulas, neutral defaults, and source-boundary rules.

**Step 2: Record real rerun metrics**
Add valid/test/holdout/walk-forward comparison.

**Step 3: Record risks**
Explicitly state whether long-horizon fields improved holdout and whether readiness changed.

### Task 7: Final Verification
**Files:**
- All modified source and test files.

**Step 1: Run cargo check**
Run: `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_long_volume_check'; cargo check`
Expected: successful completion.

**Step 2: Inspect git status**
Run: `git status --short`
Expected: only the intended source, test, docs, and journal files are modified, plus intended runtime artifacts outside git if any.

**Step 3: Final response**
Report:
- What changed.
- Which tests passed.
- Real rerun metrics.
- Whether readiness improved.
- Any remaining risk.
