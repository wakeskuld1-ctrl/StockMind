# Nikkei Long-Horizon Volume Behavior Design

## Intent
- Add long-horizon volume behavior features for Nikkei weekly training.
- Current weekly volume fields only compare the current week with the prior 4 weeks.
- The user clarified that large index-level accumulation can be yearly rather than weekly, so the model needs medium/long horizon volume context before judging volume as bullish or bearish.

## Approved Direction
- Use Scheme B: long-horizon volume behavior features.
- Keep `NK225.IDX` as the FRED spot price source.
- Keep `volume_proxy_symbol` as the explicit volume source.
- Do not overwrite spot OHLCV rows with proxy rows.

## Feature Contract
### Existing Short-Horizon Features
- `weekly_volume_ratio_4w`: current week total volume divided by the previous 4-week average.
- `weekly_up_day_volume_share`: volume share on daily up-close rows within the same week.
- `weekly_down_day_volume_share`: volume share on daily down-close rows within the same week.
- `weekly_volume_price_confirmation`: `1` when weekly return is positive and volume ratio is at least `1.05`, `-1` when weekly return is negative and volume ratio is at least `1.05`, otherwise `0`.

### New Medium/Long-Horizon Features
- `weekly_volume_ratio_13w`: current week total volume divided by previous 13-week average.
- `weekly_volume_ratio_26w`: current week total volume divided by previous 26-week average.
- `weekly_volume_ratio_52w`: current week total volume divided by previous 52-week average.
- `weekly_price_position_52w`: current weekly close position inside the prior 52-week close range, from `0.0` low to `1.0` high.
- `weekly_volume_accumulation_26w`: average 26-week volume ratio multiplied by low/mid price-position pressure, intended to detect heavy volume while price is not yet extended.
- `weekly_volume_accumulation_52w`: average 52-week volume ratio multiplied by low/mid price-position pressure, intended to detect yearly accumulation.
- `weekly_high_volume_low_price_signal`: `1` when long-horizon volume is elevated and 52-week price position is low or mid-low, otherwise `0`.
- `weekly_high_volume_breakout_signal`: `1` when long-horizon volume is elevated and 52-week price position is high, otherwise `0`.

## Calculation Rules
- Use weekly buckets already produced by `build_weekly_price_buckets`.
- Use the selected volume source bucket first: `volume_proxy_symbol`, then `futures_symbol`, then spot rows.
- Use spot close prices for price-position features, because price labels and weekly return features are anchored to `NK225.IDX`.
- Use historical windows strictly before the current week to avoid leakage.
- If a prior window does not have enough weeks, emit neutral defaults:
  - ratio fields: `1.0`
  - accumulation fields: `0.0`
  - binary signals: `0.0`
  - price position: existing current-week close position if 52-week history is unavailable, otherwise `0.5`

## Rationale
- `4w` volume detects short-term volume bursts.
- `13w` approximates quarterly capital behavior.
- `26w` approximates half-year positioning.
- `52w` approximates yearly accumulation or distribution behavior.
- Price position is required because high volume has different meaning at low, mid, and high price locations.

## Out Of Scope
- No new public CLI tool.
- No new provider.
- No official turnover import changes.
- No market-regime classifier in this slice.
- No production promotion decision in this slice.

## Acceptance
- Weekly Nikkei training artifacts include the new long-horizon volume feature names.
- Existing short-horizon fields remain unchanged.
- The new features vary when a volume proxy has non-constant long history.
- The new features do not appear in non-weekly training unless already governed by the weekly Nikkei feature contract.
- A real rerun with `NK225_VOL.YFINANCE` produces a comparable metrics table against the previous no-proxy, short-proxy, and 10Y-proxy runs.

## Risks
- More features can overfit if the sample count remains low.
- Long-horizon features may be highly correlated with existing price-position features.
- yfinance volume remains a proxy, not official Nikkei turnover truth.
- If yearly volume behavior is structurally different after 2025, the new fields may still need a later regime split.
