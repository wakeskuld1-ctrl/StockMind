// 2026-04-16 CST: Modified because StockMind is a stock-only snapshot repo.
// Reason: the migration should keep the securities chain independently buildable without
// dragging foundation, GUI, and license surfaces into the new crate root.
// Purpose: publish only the stock-facing modules that are required by the current tests and CLI.
pub mod ops;
pub mod runtime;
pub mod runtime_paths;
#[cfg(test)]
pub(crate) mod test_support;
pub mod tools;

use tools::contracts::ToolResponse;

pub fn tool_catalog_json() -> String {
    serde_json::to_string(&ToolResponse::tool_catalog())
        .expect("tool catalog serialization should succeed")
}
