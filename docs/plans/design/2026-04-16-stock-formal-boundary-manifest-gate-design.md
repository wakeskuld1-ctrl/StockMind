# Stock Formal Boundary Manifest Gate Design

Use `Option B`.

## Gate 1 - stock root manifest freeze

Freeze the standalone formal boundary at `src/ops/stock.rs`, keep `src/ops/mod.rs` stock-only, and require later sessions to update the manifest deliberately instead of drifting by accident.
