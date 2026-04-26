# Nikkei Volume Proxy Design

## Intent
- Keep `NK225.IDX` price history on the restored FRED contract.
- Add a separate volume proxy source so Nikkei weekly training can compute usable `weekly_volume_*` features instead of reading FRED's `volume=0`.
- Avoid treating a volume proxy as futures data unless the caller explicitly supplies `futures_symbol`.

## Contract
- New request field: `volume_proxy_symbol`.
- Price source remains `symbol_list` / `market_symbol` / `sector_symbol`.
- Volume source priority for weekly features:
  1. `volume_proxy_symbol`, when present and non-empty.
  2. `futures_symbol`, preserving the existing behavior.
  3. spot rows, as the existing fallback.
- `volume_proxy_symbol` only affects weekly volume features:
  - `weekly_volume_ratio_4w`
  - `weekly_up_day_volume_share`
  - `weekly_down_day_volume_share`
  - `weekly_volume_price_confirmation`
- `volume_proxy_symbol` must not enable futures features:
  - `weekly_futures_return_p50`
  - `weekly_basis_pct_p50`
  - `weekly_futures_relative_strength_p50`

## Data Flow
- Import proxy OHLCV rows into the same `stock_history.db` under a distinct symbol such as `NK225_VOL.PROXY`.
- Run training with:
  - `symbol_list=["NK225.IDX"]`
  - `volume_proxy_symbol="NK225_VOL.PROXY"`
  - no `futures_symbol` unless futures factors are explicitly desired.
- The weekly aggregator aligns proxy rows by ISO week and uses proxy `total_volume` for the volume calculations while still using spot `close` for weekly returns and labels.

## Acceptance
- A red test proves weekly volume features vary when spot volume is zero and `volume_proxy_symbol` has varying volume.
- A regression test proves `volume_proxy_symbol` does not add futures feature names.
- A real rerun proves current FRED price + proxy volume training no longer reports the weekly volume fields as zero-variance.

## Risks
- Proxy volume is not identical to exchange official Nikkei spot volume.
- If proxy coverage is shorter than FRED coverage, early weeks fall back to spot zero volume and only overlapping weeks benefit.
- 10D daily `volume_ratio_20` / `mfi_14` remain unchanged in this phase.
