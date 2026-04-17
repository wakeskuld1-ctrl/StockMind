# Stock/Foundation Decoupling Design

## Dependency Verdict

Stock does not currently depend on generic foundation analytics.

## Recommended engineering split

Keep StockMind as a stock-only workspace, preserve stock-facing shared/runtime infrastructure, and avoid reintroducing dormant foundation modules into the standalone repo.
