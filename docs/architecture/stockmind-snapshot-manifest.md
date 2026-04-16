# StockMind Snapshot Manifest

## Goal

Turn the stock-domain chain from `TradingAgents` into one independently clonable repo that still builds and runs the current securities mainline.

## Source Decision

- Primary source: `E:\TradingAgents\TradingAgents`
- Reviewed but not used as migration base:
  - `E:\TradingAgents\worktrees\security-push-20260413`
  - `E:\TradingAgents\worktrees\etf-proxy-import-latest-ready-20260412`
  - `E:\TradingAgents\worktrees\foundation-navigation-kernel-remote`

Reason:
- those worktrees were older than the current main repo for stock/runtime/test surfaces
- the latest bugfixes and boundary closeouts already lived in `TradingAgents`

## Included Surfaces

### Methods / Entry Surfaces

- `src/main.rs`
- `src/tools/dispatcher.rs`
- `src/tools/dispatcher/stock_ops.rs`
- `src/ops/stock.rs`

### Stock Domain Modules

- stock grouped gateways
- stock scenario-entry shells
- security governance / execution / post-trade / training modules
- resonance and signal sidecar modules that are still referenced by the current stock surface

### Runtime / DB Calls

- `src/runtime/formal_security_runtime_registry.rs`
- `src/runtime/stock_history_store.rs`
- `src/runtime/security_*`
- `src/runtime/signal_outcome_store.rs`

Data lands in governed local runtime files under:

```text
STOCKMIND_RUNTIME_DIR
STOCKMIND_RUNTIME_DB
.stockmind_runtime/
```

Legacy `EXCEL_SKILL_*` runtime env names remain accepted for compatibility.

## Excluded Surfaces

- foundation knowledge / metadata / workbook chain
- GUI shell
- original license gate
- workbook/result-ref/session/source helper stack

## Verification Baseline

- `cargo check`
- `cargo test --test security_decision_verify_package_cli -- --nocapture`
- `cargo test --test security_decision_package_cli -- --nocapture`
- `cargo test --test security_decision_package_revision_cli -- --nocapture`

## Current Known Gaps

- package name is `stockmind`, but library crate name remains `excel_skill`
- stock fixtures were copied as snapshot artifacts, not re-generated
- graphify output for this split repo has not been generated inside this repo yet
