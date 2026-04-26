# Nikkei Volume Source Manifest Design

## Intent
- Build a governed volume-source manifest before further Nikkei model tuning.
- Make current volume availability explicit instead of relying on implicit `stock_history.db` inspection.
- Keep this phase as data governance only: it must not automatically change training behavior.

## Contract
- Tool name: `security_volume_source_manifest`.
- Primary instrument: `instrument_symbol`, for this task `NK225.IDX`.
- Candidate volume sources: `volume_source_symbols`, for this task at least `NK225.IDX` and `NK225_VOL.PROXY`.
- Optional gate: `minimum_effective_history_days`, defaulting to `750` if omitted.
- Output document:
  - `contract_version`
  - `document_type`
  - `instrument_symbol`
  - `as_of_date`
  - `readiness_gates`
  - `volume_sources`
  - `summary`
- Each source must expose:
  - `symbol`
  - `source_names`
  - `first_trade_date`
  - `last_trade_date`
  - `row_count`
  - `nonzero_volume_rows`
  - `zero_volume_rows`
  - `nonzero_volume_ratio`
  - `min_volume`
  - `max_volume`
  - `coverage_status`
  - `eligible_for_training`
  - `limitations`

## Status Rules
- `missing_history`: no rows exist for the symbol.
- `no_volume`: rows exist, but all volume values are zero.
- `usable_short_proxy`: rows exist and volume is non-zero, but coverage is below `minimum_effective_history_days`.
- `train_ready_volume_proxy`: rows exist, volume is non-zero, and coverage meets `minimum_effective_history_days`.

## Data Flow
- Read `stock_price_history` through the governed `StockHistoryStore`.
- Aggregate per symbol with one store-level query.
- Return JSON through the normal stock dispatcher.
- Do not persist a new database table in this phase; the manifest is a reproducible artifact from the official history store.

## Rejection Conditions
- Reject empty `instrument_symbol`.
- Reject an empty `volume_source_symbols` list.
- Do not infer missing source symbols from naming conventions.
- Do not mark a short proxy as full training-ready.

## Acceptance
- Tool catalog exposes `security_volume_source_manifest`.
- CLI test imports a zero-volume `NK225.IDX` and a shorter non-zero `NK225_VOL.PROXY`.
- Manifest marks `NK225.IDX` as `no_volume`.
- Manifest marks `NK225_VOL.PROXY` as `usable_short_proxy`.
- Manifest reports row count, date range, source names, and non-zero volume ratio.
