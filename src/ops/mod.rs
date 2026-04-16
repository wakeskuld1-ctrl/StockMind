// 2026-04-16 CST: Modified because StockMind is the stock-only split snapshot.
// Reason: the standalone repo should expose just one domain boundary instead of carrying
// dormant foundation exports that no longer ship with this crate.
// Purpose: keep `crate::ops::stock::*` as the single formal application surface.
pub mod stock;
