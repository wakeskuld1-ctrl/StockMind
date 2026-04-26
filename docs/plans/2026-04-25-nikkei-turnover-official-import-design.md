# Nikkei Official Turnover Import Design

## Intent
- Accept Nikkei official `Total Trading Value` exports after browser-side Cloudflare verification blocks automated download.
- Convert the official turnover table into a governed stock-history proxy symbol for training and manifest use.
- Keep FRED `NK225.IDX` as the price source and do not pretend turnover is share volume.

## Contract
- Tool name: `security_nikkei_turnover_import`.
- Input file: `source_path`.
- Price source symbol: `price_symbol`, normally `NK225.IDX`.
- Output proxy symbol: `turnover_symbol`, normally `NK225_TURNOVER.NIKKEI`.
- Source name: default `nikkei_official_total_trading_value`.
- Accepted file shapes:
  - CSV/TSV/copy text with a date column and `Total Trading Value` column.
  - Dates may use `YYYY-MM-DD`, `Apr/01/2026`, or `Apr 01 2026`.
  - Values are interpreted as trillion yen unless an explicit `scale` is later added.
- Storage convention:
  - `volume = Total Trading Value(Tril.Yen) * 1_000_000`.
  - OHLC/adj_close are copied from `price_symbol` rows on the same date.
  - Missing price dates are skipped and reported, not guessed.

## Output
- `document_type = security_nikkei_turnover_import_result`.
- `turnover_symbol`
- `price_symbol`
- `imported_row_count`
- `skipped_missing_price_count`
- `date_range`
- `database_path`
- `source`
- `unit = total_trading_value_trillion_yen_scaled_1e6`

## Rejection Conditions
- Empty `source_path`, `price_symbol`, or `turnover_symbol`.
- Missing source file.
- Missing date or total trading value column.
- No rows can be imported after date/value parsing and price-date alignment.

## Acceptance
- Tool catalog exposes `security_nikkei_turnover_import`.
- A test fixture with official-style turnover rows imports into `NK225_TURNOVER.NIKKEI`.
- Imported rows reuse `NK225.IDX` prices and store scaled turnover in `volume`.
- `security_volume_source_manifest` marks the turnover symbol as a usable or train-ready proxy according to the configured history gate.
