# Nikkei And Gold Spot Training Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a governed training path for Nikkei first, then gold spot, while deferring Taiwan and all ETF premium work until index-family training is stable.

**Architecture:** First fix the model-identity contract so non-equity subjects stop being emitted as generic equity candidates. Then add an end-to-end Nikkei training slice that proves `history -> snapshot -> forward outcome -> training artifact` works on an index subject. Only after that, stop for explicit user confirmation on the gold spot data contract before wiring any external data source or symbol mapping. ETF premium fields remain out of scope until both index-family artifacts are stable.

**Tech Stack:** Rust, Cargo test, serde/serde_json, governed SQLite runtime stores, existing stock/index sync pipeline, existing scorecard training pipeline.

---

### Task 1: Lock The Non-Equity Training Identity Contract

**Files:**
- Modify: `E:/SM/src/ops/security_scorecard_training.rs`
- Modify: `E:/SM/src/ops/security_feature_snapshot.rs`
- Modify: `E:/SM/src/ops/security_decision_evidence_bundle.rs`
- Test: `E:/SM/tests/security_scorecard_training_cli.rs`
- Test: `E:/SM/tests/security_feature_snapshot_cli.rs`

**Step 1: Write the failing tests**

Add a new training CLI test that asserts a non-equity training request persists an explicit subject identity into the artifact and registry. Use this contract:

- Nikkei request:
  - `market_scope = "GLOBAL"`
  - `instrument_scope = "INDEX"`
  - `instrument_subscope = "nikkei_index"`
- Gold spot request:
  - `market_scope = "GLOBAL"`
  - `instrument_scope = "COMMODITY_SPOT"`
  - `instrument_subscope = "gold_spot"`

Required assertions:

```rust
assert_eq!(artifact_json["instrument_subscope"], "nikkei_index");
assert_eq!(output["data"]["model_registry"]["instrument_subscope"], "nikkei_index");
```

Add a snapshot test that locks non-equity symbol typing:

```rust
assert_eq!(output["data"]["instrument_type"], "INDEX");
assert_eq!(output["data"]["raw_features_json"]["subject_asset_class"], "index");
```

**Step 2: Run tests to verify they fail**

Run:

```powershell
cargo test --test security_scorecard_training_cli -- --nocapture
cargo test --test security_feature_snapshot_cli -- --nocapture
```

Expected:
- FAIL because `SecurityScorecardTrainingRequest` does not yet carry `instrument_subscope`
- FAIL because snapshot/evidence still classify `.IDX` subjects as generic equity

**Step 3: Write the minimal implementation**

Implement these contract changes:

- Add optional `instrument_subscope` to `SecurityScorecardTrainingRequest`
- Thread `instrument_subscope` into:
  - artifact builder
  - candidate artifact input for registry/refit
- Update `derive_instrument_type` so:
  - `.IDX` => `INDEX`
  - `.FX` => `FX`
  - existing ETF logic stays intact
- Update evidence asset classification so:
  - ETF symbols remain `etf`
  - `.IDX` becomes `index`
  - `.FX` becomes `fx`
  - everything else stays `equity`

Implementation note:
- Do not widen ETF premium logic here
- Do not introduce Taiwan handling here
- Keep existing equity and ETF tests green

**Step 4: Run tests to verify they pass**

Run:

```powershell
cargo test --test security_scorecard_training_cli -- --nocapture
cargo test --test security_feature_snapshot_cli -- --nocapture
```

Expected:
- PASS for the new identity-contract assertions
- PASS for pre-existing training and snapshot tests

**Step 5: Commit**

```powershell
git add E:/SM/src/ops/security_scorecard_training.rs E:/SM/src/ops/security_feature_snapshot.rs E:/SM/src/ops/security_decision_evidence_bundle.rs E:/SM/tests/security_scorecard_training_cli.rs E:/SM/tests/security_feature_snapshot_cli.rs
git commit -m "feat: govern non-equity training identity"
```

### Task 2: Add A Nikkei End-To-End Training Slice

**Files:**
- Modify: `E:/SM/tests/security_scorecard_training_cli.rs`
- Modify: `E:/SM/src/ops/security_scorecard_training.rs`
- Modify: `E:/SM/src/ops/security_feature_snapshot.rs`
- Modify: `E:/SM/src/ops/security_forward_outcome.rs`
- Verify: `E:/SM/src/ops/sync_stock_price_history.rs`
- Verify: `E:/SM/tests/stock_price_history_import_cli.rs`

**Execution contract freeze**

- Chosen route: train the Nikkei baseline first with `NK225.IDX` only, using the already-governed free-history path.
- Allowed change boundary:
  - formal Nikkei training contract
  - `.IDX` subject sample collection and forward-outcome compatibility
  - subscope-aware model identity
  - nearby regression verification already listed in this task
- Explicit non-goals:
  - no World Bank macro integration in Task 2
  - no BOJ/government policy feature integration in Task 2
  - no Japan trade feature integration in Task 2
  - no Nikkei constituent-weight or heavyweight-stock feature integration in Task 2
- Augmentation gate:
  - do not add the deferred data families above unless the baseline Nikkei run first completes and its governed diagnostics show a concrete trigger
  - acceptable triggers are: `production_readiness != "candidate"`, or a repeatable failure cluster that maps to one specific missing data family
  - do not add new data merely because it looks economically relevant
- Expected best-practice path for this route:
  - first prove `history -> snapshot -> forward outcome -> training artifact` on `NK225.IDX`
  - then inspect `feature_coverage_summary`, `correlation_summary`, `drift_summary`, `walk_forward_summary`, and `readiness_assessment`
  - only after that decide whether to stage a separate Nikkei enhancement task
- Acceptance checks for confirming this route was respected:
  - Task 2 passes with `NK225.IDX` as the only subject symbol
  - the Task 2 diff does not introduce World Bank / policy / trade / heavyweight-constituent ingestion code
  - the close-out explicitly states whether augmentation was still deferred after reading the baseline diagnostics

**Step 1: Write the failing test**

Add a new test named similar to:

```rust
fn security_scorecard_training_generates_nikkei_index_artifact() { ... }
```

Test contract:
- Subject symbol: `NK225.IDX`
- Use one synthetic Nikkei-like history series with mixed up/down windows so the train split contains both positive and negative labels
- Request values:

```json
{
  "market_scope": "GLOBAL",
  "instrument_scope": "INDEX",
  "instrument_subscope": "nikkei_index",
  "symbol_list": ["NK225.IDX"],
  "horizon_days": 10,
  "target_head": "direction_head"
}
```

Required assertions:

```rust
assert_eq!(output["status"], "ok");
assert_eq!(artifact_json["model_id"], "global_index_nikkei_index_10d_direction_head");
assert_eq!(artifact_json["instrument_subscope"], "nikkei_index");
assert_eq!(artifact_json["label_definition"], "security_forward_outcome.v1");
assert!(output["data"]["metrics_summary_json"]["sample_count"].as_u64().unwrap() >= 3);
```

If `model_id` is not yet subscope-aware, this test should fail first and force the contract upgrade.

**Step 2: Run the test to verify it fails**

Run:

```powershell
cargo test --test security_scorecard_training_cli security_scorecard_training_generates_nikkei_index_artifact -- --nocapture
```

Expected:
- FAIL because model identity is still too generic or the index path still assumes equity semantics

**Step 3: Write the minimal implementation**

Implement only what is necessary to make Nikkei training formal:

- Make `model_id` include `instrument_subscope` when present
- Ensure sample collection accepts an `.IDX` subject without trying to force ETF/equity-only assumptions
- Keep fundamentals/disclosures degradable instead of fatal for index subjects
- Preserve the existing `direction_head`-only constraint

Recommended `model_id` rule:

```text
{market_scope}_{instrument_scope}_{instrument_subscope}_{horizon}d_{target_head}
```

Examples:
- `global_index_nikkei_index_10d_direction_head`
- `global_commodity_spot_gold_spot_10d_direction_head`

**Step 4: Run the test and the nearby regression suite**

Run:

```powershell
cargo test --test security_scorecard_training_cli security_scorecard_training_generates_nikkei_index_artifact -- --nocapture
cargo test --test stock_price_history_import_cli sync_stock_price_history_imports_fred_index_history_into_sqlite -- --nocapture
cargo test --test security_analysis_fullstack_cli security_analysis_fullstack_auto_maps_159866_cross_border_inputs_and_uses_builtin_etf_facts -- --nocapture
```

Expected:
- PASS for Nikkei training
- PASS for existing Nikkei history sync coverage
- PASS for existing cross-border ETF mapping coverage

**Step 5: Commit**

```powershell
git add E:/SM/tests/security_scorecard_training_cli.rs E:/SM/src/ops/security_scorecard_training.rs E:/SM/src/ops/security_feature_snapshot.rs E:/SM/src/ops/security_forward_outcome.rs
git commit -m "feat: add governed nikkei index training slice"
```

### Task 3: Pause Gate Before Gold Spot Data Integration

**Files:**
- Modify: `E:/SM/docs/plans/2026-04-20-nikkei-gold-index-training-plan.md`
- Optional note: `E:/SM/CHANGELOG_TASK.MD`

**Step 1: Prepare the confirmation packet**

Draft the exact questions that must be confirmed with the user before any gold spot source integration begins:

```text
1. Canonical gold spot symbol: XAUUSD.CMD or another agreed symbol?
2. Primary source family: FRED-derived, public commodity API, or manual CSV bootstrap?
3. Daily bar definition: UTC close, New York close, or provider-native close?
4. Quote convention: XAUUSD or CNY-converted gold spot?
```

**Step 2: Stop and request confirmation**

Do not edit source-integration code yet. Send the user the confirmation packet and wait.

Expected:
- No code change beyond plan/journal text
- No new provider mapping added yet

**Step 3: Record the confirmed contract**

After the user responds, append the selected gold spot contract to this plan:

- canonical symbol
- provider family
- bar close convention
- quote currency
- out-of-scope alternatives

**Step 4: Verify the stop gate was honored**

Check:

```powershell
git diff --name-only
```

Expected:
- No changes yet to `E:/SM/src/ops/sync_stock_price_history.rs` for gold spot before user confirmation

**Step 5: Commit the updated plan/journal only if needed**

```powershell
git add E:/SM/docs/plans/2026-04-20-nikkei-gold-index-training-plan.md E:/SM/CHANGELOG_TASK.MD
git commit -m "docs: lock gold spot data confirmation gate"
```

### Task 4: Add The Confirmed Gold Spot History Contract

**Files:**
- Modify: `E:/SM/src/ops/sync_stock_price_history.rs`
- Modify: `E:/SM/tests/stock_price_history_import_cli.rs`
- Optional: `E:/SM/src/runtime/stock_history_store.rs`

**Step 1: Write the failing history-sync test**

After user confirmation, add a dedicated test like:

```rust
fn sync_stock_price_history_imports_gold_spot_history_into_sqlite() { ... }
```

Required assertions:

```rust
assert_eq!(output["status"], "ok");
assert_eq!(output["data"]["symbol"], "XAUUSD.CMD");
assert_eq!(output["data"]["provider_used"], "<confirmed_provider>");
```

Also assert the stored close for one known date.

**Step 2: Run the test to verify it fails**

Run:

```powershell
cargo test --test stock_price_history_import_cli sync_stock_price_history_imports_gold_spot_history_into_sqlite -- --nocapture
```

Expected:
- FAIL because gold spot normalization/provider mapping does not exist yet

**Step 3: Write the minimal implementation**

Implement only the confirmed gold spot path:

- Add normalized provider symbol mapping for the confirmed canonical gold spot symbol
- Add one provider fetch branch
- Convert provider rows into existing `StockHistoryRow`
- Reuse the existing governed SQLite store

Do not:
- add Taiwan
- add ETF premium fields
- add multiple gold sources in this task

**Step 4: Run the history sync regression suite**

Run:

```powershell
cargo test --test stock_price_history_import_cli sync_stock_price_history_imports_gold_spot_history_into_sqlite -- --nocapture
cargo test --test stock_price_history_import_cli sync_stock_price_history_imports_fred_index_history_into_sqlite -- --nocapture
```

Expected:
- PASS for gold spot sync
- PASS for Nikkei sync

**Step 5: Commit**

```powershell
git add E:/SM/src/ops/sync_stock_price_history.rs E:/SM/tests/stock_price_history_import_cli.rs
git commit -m "feat: add governed gold spot history sync"
```

### Task 5: Add Gold Spot Training End-To-End

**Files:**
- Modify: `E:/SM/tests/security_scorecard_training_cli.rs`
- Modify: `E:/SM/src/ops/security_scorecard_training.rs`
- Modify: `E:/SM/src/ops/security_feature_snapshot.rs`
- Modify: `E:/SM/src/ops/security_decision_evidence_bundle.rs`

**Step 1: Write the failing test**

Add a dedicated training test:

```rust
fn security_scorecard_training_generates_gold_spot_artifact() { ... }
```

Training request contract:

```json
{
  "market_scope": "GLOBAL",
  "instrument_scope": "COMMODITY_SPOT",
  "instrument_subscope": "gold_spot",
  "symbol_list": ["XAUUSD.CMD"],
  "horizon_days": 10,
  "target_head": "direction_head"
}
```

Required assertions:

```rust
assert_eq!(output["status"], "ok");
assert_eq!(artifact_json["instrument_subscope"], "gold_spot");
assert_eq!(artifact_json["model_id"], "global_commodity_spot_gold_spot_10d_direction_head");
```

**Step 2: Run the test to verify it fails**

Run:

```powershell
cargo test --test security_scorecard_training_cli security_scorecard_training_generates_gold_spot_artifact -- --nocapture
```

Expected:
- FAIL until gold spot identity and training collection are both wired

**Step 3: Write the minimal implementation**

Implement only the gold-spot-specific pieces needed for formal training:

- ensure gold spot subjects classify as non-equity
- ensure snapshot can freeze technical-only or degraded research context without fatal failure
- ensure training artifact/registry emit `gold_spot`

Do not introduce ETF premium logic here.

**Step 4: Run the focused regression suite**

Run:

```powershell
cargo test --test security_scorecard_training_cli security_scorecard_training_generates_gold_spot_artifact -- --nocapture
cargo test --test security_scorecard_training_cli security_scorecard_training_generates_nikkei_index_artifact -- --nocapture
```

Expected:
- PASS for gold spot training
- PASS for Nikkei training

**Step 5: Commit**

```powershell
git add E:/SM/tests/security_scorecard_training_cli.rs E:/SM/src/ops/security_scorecard_training.rs E:/SM/src/ops/security_feature_snapshot.rs E:/SM/src/ops/security_decision_evidence_bundle.rs
git commit -m "feat: add governed gold spot training slice"
```

### Task 6: Final Verification And Handoff

**Files:**
- Modify: `E:/SM/CHANGELOG_TASK.MD`
- Optional note: `E:/SM/docs/AI_HANDOFF.md`

**Step 1: Run the required verification suite**

```powershell
cargo test --test security_scorecard_training_cli -- --nocapture
cargo test --test stock_price_history_import_cli -- --nocapture
cargo test --test security_feature_snapshot_cli -- --nocapture
```

Expected:
- PASS for Nikkei and gold spot training coverage
- PASS for history sync regressions
- PASS for snapshot identity regressions

**Step 2: Manually inspect artifact identity**

Inspect the generated artifact JSONs and confirm:

```text
instrument_subscope == nikkei_index / gold_spot
model_id includes subscope
registry entry carries the same subscope
```

**Step 3: Append task journal**

Record:
- what changed
- why it changed
- what remains deferred
- ETF premium explicitly still deferred
- Taiwan explicitly still deferred

**Step 4: Write the handoff summary**

Summarize:
- Nikkei status
- gold spot status
- current risks
- next phase = ETF premium

**Step 5: Commit**

```powershell
git add E:/SM/CHANGELOG_TASK.MD E:/SM/docs/AI_HANDOFF.md
git commit -m "docs: record nikkei and gold spot training handoff"
```

## Acceptance Gates

### Pre-Implementation Gate
- User-approved route remains:
  - Nikkei first
  - gold spot second
  - Taiwan later
  - ETF premium later
- Gold spot data integration must not start before explicit user confirmation at Task 3
- No implementation step may skip the identity-contract fix

### Pre-Completion Gate
- Nikkei training artifact exists and is identity-correct
- Gold spot training artifact exists and is identity-correct
- Registry/artifact/runtime no longer flatten these subjects into generic equity candidates
- ETF premium logic remains untouched in this plan’s implementation scope

## Explicit Rejection Rules
- Reject any attempt to start gold source integration before Task 3 user confirmation
- Reject any shortcut that reuses `gold_etf` as if it were `gold_spot`
- Reject any claim that Taiwan is included in this delivery
- Reject any implementation that leaves `instrument_subscope` absent for Nikkei or gold spot
