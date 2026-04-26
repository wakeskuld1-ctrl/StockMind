mod common;

use chrono::{Duration, NaiveDate};
use excel_skill::runtime::stock_history_store::{StockHistoryRow, StockHistoryStore};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{create_test_runtime_db, run_cli_with_json, run_cli_with_json_and_runtime};

// 2026-04-26 CST: Added because the approved Nikkei ETF daily signal must be
// discoverable before it can become an operator-run daily Tool.
// Purpose: lock the public Tool catalog boundary for the Nikkei ETF position signal.
#[test]
fn tool_catalog_includes_security_nikkei_etf_position_signal() {
    let output = run_cli_with_json("");

    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_nikkei_etf_position_signal")
    );
}

// 2026-04-26 CST: Added because the daily Nikkei ETF signal must produce one
// auditable rule-only position without reading beyond the requested date.
// Purpose: prove the minimum daily-run contract before adding model inference.
#[test]
fn security_nikkei_etf_position_signal_builds_rule_only_daily_decision() {
    let runtime_db_path = create_test_runtime_db("security_nikkei_etf_position_signal");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have a parent directory");
    let store = StockHistoryStore::new(runtime_root.join("stock_history.db"));
    store
        .import_rows(
            "NK225.IDX",
            "nikkei_index_fixture",
            &build_trending_rows(240),
        )
        .expect("index rows should import");

    let request = json!({
        "tool": "security_nikkei_etf_position_signal",
        "args": {
            "as_of_date": "2026-04-24",
            "instrument_symbol": "NK225.IDX",
            "etf_symbol": "1321.T",
            "model_mode": "rule_only",
            "minimum_index_history_days": 220
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["document_type"],
        "security_nikkei_etf_position_signal"
    );
    assert_eq!(
        output["data"]["contract_version"],
        "security_nikkei_etf_position_signal.v1"
    );
    assert_eq!(output["data"]["instrument_symbol"], "NK225.IDX");
    assert_eq!(output["data"]["etf_symbol"], "1321.T");
    assert_eq!(output["data"]["model_mode"], "rule_only");
    assert_eq!(output["data"]["market_regime"], "bull");
    assert_eq!(output["data"]["v3_base_position"], 1.0);
    assert_eq!(output["data"]["target_position"], 1.0);
    assert_eq!(output["data"]["data_coverage"]["index_rows_used"], 240);
    assert_eq!(
        output["data"]["data_coverage"]["latest_index_trade_date"],
        "2026-04-24"
    );
    assert!(
        output["data"]["reason_codes"]
            .as_array()
            .expect("reason codes should be an array")
            .contains(&json!("bull_regime_confirmed"))
    );
}

// 2026-04-26 CST: Added because HGB adjustment is allowed only when a governed
// artifact is explicitly supplied, otherwise the daily Tool would guess model state.
// Purpose: reject incomplete model-mode requests instead of silently falling back.
#[test]
fn security_nikkei_etf_position_signal_rejects_hgb_without_artifact() {
    let runtime_db_path = create_test_runtime_db("security_nikkei_etf_position_signal_hgb");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have a parent directory");
    let store = StockHistoryStore::new(runtime_root.join("stock_history.db"));
    store
        .import_rows(
            "NK225.IDX",
            "nikkei_index_fixture",
            &build_trending_rows(240),
        )
        .expect("index rows should import");

    let request = json!({
        "tool": "security_nikkei_etf_position_signal",
        "args": {
            "as_of_date": "2026-04-24",
            "instrument_symbol": "NK225.IDX",
            "etf_symbol": "1321.T",
            "model_mode": "v3_hgb",
            "minimum_index_history_days": 220
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .expect("error message should be a string")
            .contains("model_artifact_path is required")
    );
}

// 2026-04-26 CST: Added because HGB mode must consume an explicit governed
// daily adjustment artifact and then apply the approved 0.25 position step.
// Purpose: connect V3+HGB without replaying backtest logs or guessing model state.
#[test]
fn security_nikkei_etf_position_signal_applies_hgb_adjustment_artifact() {
    let runtime_db_path = create_test_runtime_db("security_nikkei_etf_position_signal_hgb_apply");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have a parent directory");
    let store = StockHistoryStore::new(runtime_root.join("stock_history.db"));
    store
        .import_rows(
            "NK225.IDX",
            "nikkei_index_fixture",
            &build_trending_rows(240),
        )
        .expect("index rows should import");
    let artifact_path = create_hgb_adjustment_artifact(
        "security_nikkei_etf_position_signal_hgb_apply",
        "2026-04-24",
        -1,
    );

    let request = json!({
        "tool": "security_nikkei_etf_position_signal",
        "args": {
            "as_of_date": "2026-04-24",
            "instrument_symbol": "NK225.IDX",
            "etf_symbol": "1321.T",
            "model_mode": "v3_hgb",
            "model_artifact_path": artifact_path.to_string_lossy(),
            "minimum_index_history_days": 220
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(output["data"]["model_mode"], "v3_hgb");
    assert_eq!(output["data"]["v3_base_position"], 1.0);
    assert_eq!(output["data"]["hgb_adjustment"], -1.0);
    assert_eq!(output["data"]["target_position"], 0.75);
    assert!(
        output["data"]["reason_codes"]
            .as_array()
            .expect("reason codes should be an array")
            .contains(&json!("hgb_adjustment_down"))
    );
}

// 2026-04-26 CST: Added because the V3 regime contract depends on a long enough
// 200D window and must reject thin history instead of emitting false confidence.
// Purpose: lock the daily Tool's minimum-history gate.
#[test]
fn security_nikkei_etf_position_signal_rejects_insufficient_index_history() {
    let runtime_db_path = create_test_runtime_db("security_nikkei_etf_position_signal_short");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have a parent directory");
    let store = StockHistoryStore::new(runtime_root.join("stock_history.db"));
    store
        .import_rows(
            "NK225.IDX",
            "nikkei_index_fixture",
            &build_trending_rows(120),
        )
        .expect("index rows should import");

    let request = json!({
        "tool": "security_nikkei_etf_position_signal",
        "args": {
            "as_of_date": "2025-12-25",
            "instrument_symbol": "NK225.IDX",
            "etf_symbol": "1321.T",
            "model_mode": "rule_only",
            "minimum_index_history_days": 220
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    assert_eq!(output["status"], "error", "output={output}");
    assert!(
        output["error"]
            .as_str()
            .expect("error message should be a string")
            .contains("insufficient index history")
    );
}

// 2026-04-26 CST: Added because the daily operator Tool must be safe against
// accidental future rows already present in the local runtime database.
// Purpose: prove that rows after `as_of_date` do not change same-day decisions.
#[test]
fn security_nikkei_etf_position_signal_ignores_rows_after_as_of_date() {
    let runtime_db_path = create_test_runtime_db("security_nikkei_etf_position_signal_leakage");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have a parent directory");
    let store = StockHistoryStore::new(runtime_root.join("stock_history.db"));
    store
        .import_rows(
            "NK225.IDX",
            "nikkei_index_fixture",
            &build_trending_rows(240),
        )
        .expect("index rows should import");

    let request = json!({
        "tool": "security_nikkei_etf_position_signal",
        "args": {
            "as_of_date": "2026-04-24",
            "instrument_symbol": "NK225.IDX",
            "etf_symbol": "1321.T",
            "model_mode": "rule_only",
            "minimum_index_history_days": 220
        }
    });

    let before = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);
    store
        .import_rows(
            "NK225.IDX",
            "nikkei_index_fixture_future",
            &build_trending_rows_with_future_crash(260),
        )
        .expect("future rows should import");
    let after = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    assert_eq!(before["status"], "ok", "output={before}");
    assert_eq!(after["status"], "ok", "output={after}");
    assert_eq!(
        before["data"]["market_regime"],
        after["data"]["market_regime"]
    );
    assert_eq!(
        before["data"]["target_position"],
        after["data"]["target_position"]
    );
    assert_eq!(after["data"]["data_coverage"]["index_rows_used"], 240);
    assert_eq!(
        after["data"]["data_coverage"]["latest_index_trade_date"],
        "2026-04-24"
    );
}

// 2026-04-26 CST: Added because component stocks are not traded directly but
// must confirm whether the Nikkei ETF entry signal has enough weighted breadth.
// Purpose: lock the approved index-ETF strategy distinction between trade target
// and component evidence.
#[test]
fn security_nikkei_etf_position_signal_uses_component_breadth_when_supplied() {
    let runtime_db_path = create_test_runtime_db("security_nikkei_etf_position_signal_breadth");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have a parent directory");
    let store = StockHistoryStore::new(runtime_root.join("stock_history.db"));
    store
        .import_rows(
            "NK225.IDX",
            "nikkei_index_fixture",
            &build_trending_rows(240),
        )
        .expect("index rows should import");
    let fixture_dir = create_component_fixture(
        "security_nikkei_etf_position_signal_breadth",
        &[
            ("7203.T", 0.35, true),
            ("6758.T", 0.25, true),
            ("9984.T", 0.20, false),
        ],
    );
    let weights_path = fixture_dir.join("weights.csv");
    let history_dir = fixture_dir.join("history");

    let request = json!({
        "tool": "security_nikkei_etf_position_signal",
        "args": {
            "as_of_date": "2026-04-24",
            "instrument_symbol": "NK225.IDX",
            "etf_symbol": "1321.T",
            "model_mode": "rule_only",
            "minimum_index_history_days": 220,
            "component_weights_path": weights_path.to_string_lossy(),
            "component_history_dir": history_dir.to_string_lossy()
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["breadth_signal"],
        "component_bull_breadth_confirmed"
    );
    assert_eq!(
        output["data"]["data_coverage"]["component_history_coverage_count"],
        3
    );
    assert_eq!(
        output["data"]["data_coverage"]["component_weight_coverage_ratio"],
        1.0
    );
    assert!(
        output["data"]["reason_codes"]
            .as_array()
            .expect("reason codes should be an array")
            .contains(&json!("component_breadth_bull_confirmed"))
    );
    assert!(
        !output["data"]["risk_flags"]
            .as_array()
            .expect("risk flags should be an array")
            .contains(&json!("component_breadth_not_supplied"))
    );
}

// 2026-04-26 CST: Added because the approved ETF timing rule requires volume
// confirmation after breakout instead of relying on price or breadth alone.
// Purpose: lock the 3D-vs-20D volume expansion signal without future rows.
#[test]
fn security_nikkei_etf_position_signal_confirms_volume_backed_breakout() {
    let runtime_db_path = create_test_runtime_db("security_nikkei_etf_position_signal_volume");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have a parent directory");
    let store = StockHistoryStore::new(runtime_root.join("stock_history.db"));
    store
        .import_rows(
            "NK225.IDX",
            "nikkei_index_fixture",
            &build_breakout_rows(240),
        )
        .expect("index rows should import");
    store
        .import_rows(
            "NK225_VOL.YFINANCE",
            "nikkei_volume_fixture",
            &build_volume_proxy_rows(240),
        )
        .expect("volume rows should import");

    let request = json!({
        "tool": "security_nikkei_etf_position_signal",
        "args": {
            "as_of_date": "2026-04-24",
            "instrument_symbol": "NK225.IDX",
            "etf_symbol": "1321.T",
            "model_mode": "rule_only",
            "minimum_index_history_days": 220,
            "volume_proxy_symbol": "NK225_VOL.YFINANCE"
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["volume_signal"],
        "volume_backed_20d_breakout_confirmed"
    );
    assert_eq!(output["data"]["volume_metrics"]["price_breakout_20d"], true);
    assert!(
        output["data"]["volume_metrics"]["volume_ratio_3d_vs_prev20"]
            .as_f64()
            .expect("volume ratio should be numeric")
            > 1.2
    );
    assert!(
        output["data"]["reason_codes"]
            .as_array()
            .expect("reason codes should be an array")
            .contains(&json!("volume_backed_breakout_confirmed"))
    );
    assert!(
        !output["data"]["risk_flags"]
            .as_array()
            .expect("risk flags should be an array")
            .contains(&json!("volume_proxy_not_supplied"))
    );
}

// 2026-04-26 CST: Added because the approved live Nikkei ETF execution layer
// must buy the lower-premium ETF instead of treating premium as a broad block.
// Purpose: lock the Scheme A live-open plan before implementing the Tool fields.
#[test]
fn security_nikkei_etf_position_signal_live_plan_buys_lower_premium_etf() {
    let runtime_db_path =
        create_test_runtime_db("security_nikkei_etf_position_signal_live_buy_low_premium");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have a parent directory");
    let store = StockHistoryStore::new(runtime_root.join("stock_history.db"));
    store
        .import_rows(
            "NK225.IDX",
            "nikkei_index_fixture",
            &build_trending_rows(240),
        )
        .expect("index rows should import");

    let request = json!({
        "tool": "security_nikkei_etf_position_signal",
        "args": {
            "as_of_date": "2026-04-24",
            "instrument_symbol": "NK225.IDX",
            "etf_symbol": "159866.SZ",
            "model_mode": "rule_only",
            "minimum_index_history_days": 220,
            "planned_execution_date": "2026-04-27",
            "current_cash_cny": 1_000_000.0,
            "current_positions": [],
            "execution_quotes": [
                {
                    "etf_symbol": "159866.SZ",
                    "execution_date": "2026-04-27",
                    "open_price": 1.50,
                    "nav": 1.50
                },
                {
                    "etf_symbol": "513520.SS",
                    "execution_date": "2026-04-27",
                    "open_price": 2.10,
                    "nav": 2.00
                }
            ],
            "commission_rate": 0.0003,
            "extreme_premium_block_pct": 5.0
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(output["data"]["execution_plan"]["action"], "buy_to_target");
    assert_eq!(
        output["data"]["execution_plan"]["selected_buy_etf_symbol"],
        "159866.SZ"
    );
    assert_eq!(
        output["data"]["execution_plan"]["execution_price_basis"],
        "next_open"
    );
    assert_eq!(
        output["data"]["execution_plan"]["minimum_rebalance_delta"],
        0.0
    );
    assert!(
        output["data"]["execution_plan"]["trade_gross_value"]
            .as_f64()
            .expect("trade gross should be numeric")
            > 999_000.0
    );
}

// 2026-04-26 CST: Added because ordinary premium should choose the cheaper ETF,
// while extreme premium on every ETF remains a live execution risk block.
// Purpose: prevent the live Tool from chasing both ETF wrappers when both are too expensive.
#[test]
fn security_nikkei_etf_position_signal_live_plan_blocks_extreme_premium_buy() {
    let runtime_db_path =
        create_test_runtime_db("security_nikkei_etf_position_signal_live_block_extreme");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have a parent directory");
    let store = StockHistoryStore::new(runtime_root.join("stock_history.db"));
    store
        .import_rows(
            "NK225.IDX",
            "nikkei_index_fixture",
            &build_trending_rows(240),
        )
        .expect("index rows should import");

    let request = json!({
        "tool": "security_nikkei_etf_position_signal",
        "args": {
            "as_of_date": "2026-04-24",
            "instrument_symbol": "NK225.IDX",
            "etf_symbol": "159866.SZ",
            "model_mode": "rule_only",
            "minimum_index_history_days": 220,
            "planned_execution_date": "2026-04-27",
            "current_cash_cny": 1_000_000.0,
            "current_positions": [],
            "execution_quotes": [
                {
                    "etf_symbol": "159866.SZ",
                    "execution_date": "2026-04-27",
                    "open_price": 1.60,
                    "nav": 1.50
                },
                {
                    "etf_symbol": "513520.SS",
                    "execution_date": "2026-04-27",
                    "open_price": 2.12,
                    "nav": 2.00
                }
            ],
            "commission_rate": 0.0003,
            "extreme_premium_block_pct": 5.0
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(
        output["data"]["execution_plan"]["action"],
        "delay_buy_extreme_premium"
    );
    assert_eq!(output["data"]["execution_plan"]["trade_gross_value"], 0.0);
    assert!(
        output["data"]["execution_plan"]["risk_flags"]
            .as_array()
            .expect("execution risk flags should be an array")
            .contains(&json!("all_etf_open_premium_above_extreme_block"))
    );
}

// 2026-04-26 CST: Added because the approved live rule sells high-premium ETF
// wrappers first when reducing exposure, preserving cheaper wrapper exposure.
// Purpose: lock sell-side premium harvesting before implementation.
#[test]
fn security_nikkei_etf_position_signal_live_plan_sells_high_premium_etf_first() {
    let runtime_db_path =
        create_test_runtime_db("security_nikkei_etf_position_signal_live_sell_high_premium");
    let runtime_root = runtime_db_path
        .parent()
        .expect("runtime db should have a parent directory");
    let store = StockHistoryStore::new(runtime_root.join("stock_history.db"));
    store
        .import_rows(
            "NK225.IDX",
            "nikkei_index_fixture",
            &build_trending_rows(240),
        )
        .expect("index rows should import");
    let artifact_path = create_hgb_adjustment_artifact(
        "security_nikkei_etf_position_signal_live_sell_high_premium",
        "2026-04-24",
        -1,
    );

    let request = json!({
        "tool": "security_nikkei_etf_position_signal",
        "args": {
            "as_of_date": "2026-04-24",
            "instrument_symbol": "NK225.IDX",
            "etf_symbol": "159866.SZ",
            "model_mode": "v3_hgb",
            "model_artifact_path": artifact_path.to_string_lossy(),
            "minimum_index_history_days": 220,
            "planned_execution_date": "2026-04-27",
            "current_cash_cny": 100_000.0,
            "current_positions": [
                {
                    "etf_symbol": "159866.SZ",
                    "shares": 300_000.0
                },
                {
                    "etf_symbol": "513520.SS",
                    "shares": 300_000.0
                }
            ],
            "execution_quotes": [
                {
                    "etf_symbol": "159866.SZ",
                    "execution_date": "2026-04-27",
                    "open_price": 1.50,
                    "nav": 1.50
                },
                {
                    "etf_symbol": "513520.SS",
                    "execution_date": "2026-04-27",
                    "open_price": 2.10,
                    "nav": 2.00
                }
            ],
            "commission_rate": 0.0003,
            "extreme_premium_block_pct": 5.0
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    assert_eq!(output["status"], "ok", "output={output}");
    assert_eq!(output["data"]["execution_plan"]["action"], "sell_to_target");
    assert_eq!(
        output["data"]["execution_plan"]["trade_legs"][0]["etf_symbol"],
        "513520.SS"
    );
    assert!(
        output["data"]["execution_plan"]["trade_legs"][0]["gross_value"]
            .as_f64()
            .expect("first sell leg gross should be numeric")
            > 0.0
    );
}

fn build_trending_rows(row_count: usize) -> Vec<StockHistoryRow> {
    let start_date = NaiveDate::from_ymd_opt(2025, 8, 28).expect("fixture date should be valid");

    (0..row_count)
        .map(|index| {
            let trade_date = start_date + Duration::days(index as i64);
            let close = 30_000.0 + index as f64 * 20.0;
            StockHistoryRow {
                trade_date: trade_date.format("%Y-%m-%d").to_string(),
                open: close - 10.0,
                high: close + 30.0,
                low: close - 30.0,
                close,
                adj_close: close,
                volume: 1_000_000 + index as i64,
            }
        })
        .collect()
}

fn build_breakout_rows(row_count: usize) -> Vec<StockHistoryRow> {
    let mut rows = build_trending_rows(row_count);
    for (index, row) in rows.iter_mut().enumerate() {
        if index < row_count - 1 {
            row.close = 30_000.0 + (index as f64 * 2.0);
            row.high = row.close + 10.0;
            row.low = row.close - 10.0;
            row.open = row.close - 2.0;
            row.adj_close = row.close;
        } else {
            row.close = 36_000.0;
            row.high = 36_100.0;
            row.low = 35_800.0;
            row.open = 35_900.0;
            row.adj_close = row.close;
        }
    }
    rows
}

fn build_volume_proxy_rows(row_count: usize) -> Vec<StockHistoryRow> {
    let mut rows = build_trending_rows(row_count);
    for (index, row) in rows.iter_mut().enumerate() {
        let volume = if index >= row_count - 3 {
            2_000_000
        } else {
            1_000_000
        };
        row.open = volume as f64;
        row.high = volume as f64;
        row.low = volume as f64;
        row.close = volume as f64;
        row.adj_close = volume as f64;
        row.volume = volume;
    }
    rows
}

fn build_trending_rows_with_future_crash(row_count: usize) -> Vec<StockHistoryRow> {
    let mut rows = build_trending_rows(row_count);
    for row in rows.iter_mut().skip(240) {
        row.close = 20_000.0;
        row.adj_close = 20_000.0;
        row.open = 20_100.0;
        row.high = 20_200.0;
        row.low = 19_800.0;
    }
    rows
}

fn create_component_fixture(prefix: &str, components: &[(&str, f64, bool)]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_nikkei_etf_position_signal")
        .join(format!("{prefix}_{unique_suffix}"));
    let history_dir = fixture_dir.join("history");
    fs::create_dir_all(&history_dir).expect("component fixture dir should exist");

    let weights = components
        .iter()
        .map(|(symbol, weight, _)| format!("{symbol},{weight}"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(
        fixture_dir.join("weights.csv"),
        format!("symbol,weight\n{weights}\n"),
    )
    .expect("weights fixture should be written");

    for (symbol, _, above_200d) in components {
        fs::write(
            history_dir.join(format!("{symbol}.csv")),
            component_csv_body(*above_200d),
        )
        .expect("component history fixture should be written");
    }

    fixture_dir
}

fn create_hgb_adjustment_artifact(prefix: &str, as_of_date: &str, adjustment: i64) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_nikkei_etf_position_signal")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("artifact fixture dir should exist");
    let artifact_path = fixture_dir.join("hgb_adjustment.json");
    fs::write(
        &artifact_path,
        json!({
            "contract_version": "nikkei_v3_hgb_adjustment.v1",
            "as_of_date": as_of_date,
            "adjustment": adjustment,
            "model_id": "hgb_l2_leaf20_fixture"
        })
        .to_string(),
    )
    .expect("artifact fixture should be written");
    artifact_path
}

fn component_csv_body(above_200d: bool) -> String {
    let header = "Date,Open,High,Low,Close,Adj Close,Volume\n";
    let rows = build_component_rows(240, above_200d)
        .into_iter()
        .map(|row| {
            format!(
                "{},{},{},{},{},{},{}",
                row.trade_date, row.open, row.high, row.low, row.close, row.adj_close, row.volume
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("{header}{rows}\n")
}

fn build_component_rows(row_count: usize, above_200d: bool) -> Vec<StockHistoryRow> {
    let start_date = NaiveDate::from_ymd_opt(2025, 8, 28).expect("fixture date should be valid");

    (0..row_count)
        .map(|index| {
            let trade_date = start_date + Duration::days(index as i64);
            let close = if above_200d {
                1_000.0 + index as f64
            } else {
                1_500.0 - index as f64
            };
            StockHistoryRow {
                trade_date: trade_date.format("%Y-%m-%d").to_string(),
                open: close,
                high: close + 5.0,
                low: close - 5.0,
                close,
                adj_close: close,
                volume: 100_000 + index as i64,
            }
        })
        .collect()
}
