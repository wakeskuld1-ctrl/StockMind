# Nikkei ETF Position Signal Tool Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a formal daily Rust Tool that produces an auditable Nikkei ETF target-position signal from Nikkei index data, component breadth, volume confirmation, and optional V3+HGB adjustment.

**Architecture:** Add one side-effect-free stock Tool named `security_nikkei_etf_position_signal`. The Tool reads governed stock-history rows from the existing runtime store plus optional component CSV files, computes V3 regime and position rules without future data, and returns a machine-readable decision trace. Public exposure must stay synchronized across ops exports, catalog, dispatcher, tests, and contract docs.

**Tech Stack:** Rust, serde/serde_json, chrono, existing `StockHistoryStore`, existing JSON CLI Tool bus, cargo integration tests.

---

### Risk Synchronization Gate
**Risk subprocess mode:** inline-fresh-pass
**Question asked:** What artifact will drift if this boundary is added, removed, or exposed?
**Boundary items:**
- Public Tool name: `security_nikkei_etf_position_signal`
- Ops module: `src/ops/security_nikkei_etf_position_signal.rs`
- Grouped stock surface: likely `stock_governance_and_positioning` because the Tool decides position state, not raw data import.
- CLI dispatcher branch and stock dispatcher helper.
- Tool catalog item.
- Contract registry entry.
- CLI integration test and any relevant boundary/source guard if the new Tool touches an existing frozen list.

**Must-sync files:**
- `src/ops/security_nikkei_etf_position_signal.rs`
- `src/ops/stock.rs`
- `src/ops/stock_governance_and_positioning.rs`
- `src/tools/catalog.rs`
- `src/tools/dispatcher.rs`
- `src/tools/dispatcher/stock_ops.rs`
- `tests/security_nikkei_etf_position_signal_cli.rs`
- `docs/governance/contract_registry.md`
- `.trae/CHANGELOG_TASK.md`

**Must-run checks:**
- `cargo test --test security_nikkei_etf_position_signal_cli -- --nocapture`
- `cargo test --test stock_catalog_grouping_source_guard -- --nocapture`
- `cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture`
- `cargo check`

**Blockers:**
- Do not implement model training inside this Tool.
- Do not use future labels or rows after `as_of_date`.
- Do not trade or score component stocks directly; component data only confirms index ETF timing.
- If `model_mode = "v3_hgb"` and no governed model artifact is provided, return an error instead of guessing.

### Task 1: Add Failing Catalog And CLI Contract Tests

**Files:**
- Create: `tests/security_nikkei_etf_position_signal_cli.rs`

**Step 1: Write the failing tests**
- Test catalog discovery includes `security_nikkei_etf_position_signal`.
- Test a happy-path `rule_only` request returns:
  - `document_type = "security_nikkei_etf_position_signal"`
  - `contract_version = "security_nikkei_etf_position_signal.v1"`
  - `instrument_symbol = "NK225.IDX"`
  - `etf_symbol` echoed from request
  - `model_mode = "rule_only"`
  - `market_regime`
  - `v3_base_position`
  - `target_position`
  - non-empty `reason_codes`
  - `data_coverage.index_rows_used <= rows through as_of_date`
- Test `v3_hgb` without `model_artifact_path` returns an error.

**Step 2: Run RED**
- Run: `cargo test --test security_nikkei_etf_position_signal_cli -- --nocapture`
- Expected: fails because the test file or Tool is not implemented.

### Task 2: Implement Minimal Ops Contract

**Files:**
- Create: `src/ops/security_nikkei_etf_position_signal.rs`

**Step 1: Add request/result structs**
- Request fields:
  - `as_of_date: String`
  - `instrument_symbol: String`
  - `etf_symbol: String`
  - `volume_proxy_symbol: Option<String>`
  - `model_mode: Option<String>`
  - `model_artifact_path: Option<String>`
  - `minimum_index_history_days: Option<usize>`
  - `component_weights_path: Option<String>`
  - `component_history_dir: Option<String>`
- Result fields:
  - `contract_version`
  - `document_type`
  - `as_of_date`
  - `instrument_symbol`
  - `etf_symbol`
  - `model_mode`
  - `market_regime`
  - `position_cap`
  - `v3_base_position`
  - `hgb_adjustment`
  - `target_position`
  - `entry_signal`
  - `exit_signal`
  - `breadth_signal`
  - `volume_signal`
  - `volume_metrics`
  - `reason_codes`
  - `risk_flags`
  - `data_coverage`
  - `decision_trace`

**Step 2: Implement rule-only minimum**
- Load index rows from `StockHistoryStore::workspace_default()` through `as_of_date`.
- Reject if index rows are fewer than `minimum_index_history_days`, default `220`.
- Compute moving averages using only rows through `as_of_date`.
- Classify:
  - bull if close > 200D MA, 50D MA > 200D MA, and 200D slope > 0.
  - bear if close < 200D MA, 50D MA < 200D MA, and 200D slope < 0.
  - otherwise neutral.
- Position cap:
  - bull `1.0`
  - neutral `0.75`
  - bear `0.35`
- Base position:
  - bull `1.0`
  - neutral `0.5`
  - bear `0.25`
- Clamp target to cap.
- For `model_mode = "v3_hgb"`, reject if `model_artifact_path` is empty; do not fake model inference.
- For `model_mode = "v3_hgb"`, consume only a daily `nikkei_v3_hgb_adjustment.v1` artifact with matching `as_of_date` and `adjustment` in `-1/0/1`.
- When `volume_proxy_symbol` is supplied, compute `volume_ratio_3d_vs_prev20` from rows through `as_of_date` and combine it with 20D price breakout confirmation.

**Step 3: Run GREEN**
- Run: `cargo test --test security_nikkei_etf_position_signal_cli -- --nocapture`
- Expected: passes once dispatcher/catalog sync is complete.

### Task 3: Expose Public Tool Boundary

**Files:**
- Modify: `src/ops/stock.rs`
- Modify: `src/ops/stock_governance_and_positioning.rs`
- Modify: `src/tools/catalog.rs`
- Modify: `src/tools/dispatcher.rs`
- Modify: `src/tools/dispatcher/stock_ops.rs`

**Step 1: Add stock boundary exports**
- Add the new ops file to `stock.rs`.
- Re-export it from the governance/positioning grouped module.

**Step 2: Add dispatcher helper**
- Parse `SecurityNikkeiEtfPositionSignalRequest`.
- Return `ToolResponse::ok(json!(result))` on success.
- Return `ToolResponse::error(error.to_string())` on failure.

**Step 3: Add catalog and dispatcher branch**
- Add `security_nikkei_etf_position_signal` near governance/positioning Tools.
- Add the main dispatcher branch.

**Step 4: Verify boundary**
- Run: `cargo test --test security_nikkei_etf_position_signal_cli -- --nocapture`
- Expected: pass.
- Run: `cargo test --test stock_catalog_grouping_source_guard -- --nocapture`
- Expected: pass or expose a frozen-manifest sync need.
- Run: `cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture`
- Expected: pass or expose a dispatcher guard sync need.

### Task 4: Add Data-Coverage And Rejection Tests

**Files:**
- Modify: `tests/security_nikkei_etf_position_signal_cli.rs`
- Modify: `src/ops/security_nikkei_etf_position_signal.rs`

**Step 1: Write failing rejection tests**
- Missing `as_of_date` rows for index returns an error.
- Insufficient index history returns an error.
- Future leakage guard: rows after `as_of_date` do not change the decision trace for the same `as_of_date`.

**Step 2: Run RED**
- Run: `cargo test --test security_nikkei_etf_position_signal_cli -- --nocapture`
- Expected: new tests fail before validation/leakage handling is complete.

**Step 3: Implement minimal validation**
- Reject empty symbols and empty `as_of_date`.
- Count and expose rows used.
- Always query through `as_of_date`.
- Include latest row date in `data_coverage`.

**Step 4: Run GREEN**
- Run: `cargo test --test security_nikkei_etf_position_signal_cli -- --nocapture`
- Expected: pass.

### Task 5: Update Contract Registry And Journal

**Files:**
- Modify: `docs/governance/contract_registry.md`
- Modify: `.trae/CHANGELOG_TASK.md`

**Step 1: Update contract registry**
- Add one row for `security_nikkei_etf_position_signal`.
- Boundary note must state:
  - side-effect-free daily ETF position signal.
  - component stocks are only breadth evidence.
  - no future labels.
  - `v3_hgb` must reject missing model artifacts.

**Step 2: Append task journal**
- Add a new dated entry with:
  - modified content.
  - reason.
  - remaining work.
  - risks.
  - closed items.

### Task 6: Final Verification

**Files:**
- No production edits unless verification exposes a defect.

**Step 1: Run focused tests**
- `cargo test --test security_nikkei_etf_position_signal_cli -- --nocapture`

**Step 2: Run boundary guards**
- `cargo test --test stock_catalog_grouping_source_guard -- --nocapture`
- `cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture`

**Step 3: Run compile check**
- `cargo check`

**Step 4: Summarize verification**
- Report exact commands run.
- Report failures if any.
- Do not claim completion unless all required checks pass or limitations are explicitly stated.
