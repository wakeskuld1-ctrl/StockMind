mod common;

use chrono::{Duration, NaiveDate};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{create_test_runtime_db, run_cli_with_json, run_cli_with_json_and_runtime};

const FIXTURE_AS_OF_DATE: &str = "2025-08-08";

// 2026-04-01 CST: 这里补新上层综合 Tool 的专属 CSV 夹具助手，原因是 contextual Tool 需要一次导入个股、大盘代理和板块代理三套日线；
// 目的：让测试继续走真实 `CSV -> SQLite -> Tool` 主链，而不是手工拼装多源上下文 JSON。
fn create_stock_history_csv(prefix: &str, file_name: &str, rows: &[String]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_analysis_contextual")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("security contextual fixture dir should exist");

    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n")).expect("security contextual csv should be written");
    csv_path
}

#[test]
fn security_analysis_contextual_reports_headwind_when_stock_and_environment_conflict() {
    let runtime_db_path = create_test_runtime_db("security_analysis_contextual_headwind");

    let stock_csv = create_stock_history_csv(
        "security_analysis_contextual_headwind",
        "stock_breakout.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_analysis_contextual_headwind",
        "market_breakdown.csv",
        &build_confirmed_breakdown_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_analysis_contextual_headwind",
        "sector_breakdown.csv",
        &build_confirmed_breakdown_rows(220, 950.0),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let request = json!({
        "tool": "security_analysis_contextual",
        "args": {
            "symbol": "601916.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            // 2026-04-17 CST: Added because this integration suite is built on
            // local CSV fixtures and must not drift into live sync on later dates.
            // Purpose: keep contextual assertions pinned to the governed fixture window.
            "as_of_date": FIXTURE_AS_OF_DATE
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-01 CST: 这里补 `headwind` 红测，原因是方案 B 要把“个股与环境双逆向”的逆风路径也锁成正式合同。
    // 目的：避免当前综合 Tool 只有顺风和等待态测试，导致逆风语义只存在于代码分支而没有回归保护。
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["stock_analysis"]["consultation_conclusion"]["bias"],
        "bullish_continuation"
    );
    assert_eq!(
        output["data"]["market_analysis"]["consultation_conclusion"]["bias"],
        "bearish_continuation"
    );
    assert_eq!(
        output["data"]["sector_analysis"]["consultation_conclusion"]["bias"],
        "bearish_continuation"
    );
    assert_eq!(
        output["data"]["contextual_conclusion"]["alignment"],
        "headwind"
    );
    assert_eq!(
        output["data"]["contextual_conclusion"]["risk_flags"]
            .as_array()
            .expect("risk flags should exist")
            .len(),
        1
    );
}

#[test]
fn security_analysis_contextual_uses_proxy_profiles_when_symbols_are_omitted() {
    let runtime_db_path = create_test_runtime_db("security_analysis_contextual_profiles");

    let stock_csv = create_stock_history_csv(
        "security_analysis_contextual_profiles",
        "stock_breakout.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_analysis_contextual_profiles",
        "market_breakout.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_analysis_contextual_profiles",
        "sector_breakout.csv",
        &build_confirmed_breakout_rows(220, 950.0),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let request = json!({
        "tool": "security_analysis_contextual",
        "args": {
            "symbol": "601916.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            // 2026-04-17 CST: Added because profile-based proxy resolution should
            // still read the same local fixture slice instead of syncing to "today".
            // Purpose: make profile-resolution assertions deterministic across dates.
            "as_of_date": FIXTURE_AS_OF_DATE
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-01 CST: 这里补代理配置入口红测，原因是方案 B 需要把 market/sector 代理符号从“纯手填”推进到“可配置调用”。
    // 目的：让真实调用时可以用 profile 收口默认代理，而不是每次都要求外层手工传完整 symbol。
    assert_eq!(output["status"], "ok");
    assert_eq!(output["data"]["market_symbol"], "510300.SH");
    assert_eq!(output["data"]["sector_symbol"], "512800.SH");
    // 2026-04-08 CST: 这里补 contextual 顶层公共合同红测，原因是方案 C 第一批要把 analysis_date / evidence_version 从 briefing 往下收口到环境层；
    // 目的：确保调用方读取 contextual 顶层时就能拿到统一日期与证据版本，而不必再钻进 stock_analysis 内层猜字段。
    assert_eq!(
        output["data"]["analysis_date"],
        output["data"]["stock_analysis"]["as_of_date"]
    );
    assert_eq!(
        output["data"]["evidence_version"],
        format!(
            "security-analysis-contextual:601916.SH:{}:v1",
            output["data"]["analysis_date"]
                .as_str()
                .expect("analysis_date should exist")
        )
    );
    assert_eq!(
        output["data"]["contextual_conclusion"]["alignment"],
        "tailwind"
    );
}

#[test]
fn security_analysis_contextual_requires_proxy_or_symbol_inputs() {
    let request = json!({
        "tool": "security_analysis_contextual",
        "args": {
            "symbol": "601916.SH"
        }
    });

    let output = run_cli_with_json(&request.to_string());

    // 2026-04-01 CST: 这里补错误路径红测，原因是方案 B 不能只关心成功场景，还要给真实调用缺参时稳定、可解释的报错。
    // 目的：避免没有 market/sector symbol 且没有 profile 时，调用方得到模糊解析错误而不是业务层明确提示。
    assert_eq!(output["status"], "error");
    assert!(
        output["error"]
            .as_str()
            .expect("error field should exist")
            .contains("market_symbol")
    );
}

// 2026-04-01 CST: 这里复用股票历史导入助手，原因是新 Tool 当前仍建立在统一 stock_history_store 主表之上；
// 目的：确保综合分析测试验证的是正式落库后的调用链，而不是内存态样本。
fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_analysis_contextual_fixture"
        }
    });

    let output =
        run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path.to_path_buf());
    assert_eq!(output["status"], "ok");
}

// 2026-04-01 CST: 这里补一个稳定上行并在尾部完成有效突破的样本，原因是综合 Tool 第一版要能验证“多头 + 环境顺风”；
// 目的：让 stock / market / sector 三条链都能稳定落到 `bullish_continuation`。
fn build_confirmed_breakout_rows(day_count: usize, start_close: f64) -> Vec<String> {
    let mut rows = vec!["trade_date,open,high,low,close,adj_close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("seed date should be valid");
    let mut close = start_close;

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let (next_close, volume): (f64, i64) = if offset < day_count - 20 {
            (close + 0.78, 880_000 + offset as i64 * 8_000)
        } else {
            let phase = offset - (day_count - 20);
            match phase % 4 {
                0 => (close + 1.35, 1_700_000 + phase as i64 * 26_000),
                1 => (close - 0.18, 420_000),
                2 => (close + 1.08, 1_540_000 + phase as i64 * 22_000),
                _ => (close + 0.42, 1_240_000),
            }
        };

        let open = close;
        // 2026-04-17 CST: Updated because the new decision chain confirms breakout
        // against recent high-derived key levels instead of only close-to-close drift.
        // Purpose: make this fixture close decisively above prior resistance rather than
        // hiding the move under oversized upper shadows that only produce `range_wait`.
        let high = next_close.max(open) + if offset < day_count - 20 { 0.28 } else { 0.14 };
        let low = next_close.min(open) - if offset < day_count - 20 { 0.24 } else { 0.12 };
        let adj_close = next_close;
        rows.push(format!(
            "{},{open:.2},{high:.2},{low:.2},{next_close:.2},{adj_close:.2},{volume}",
            trade_date.format("%Y-%m-%d")
        ));
        close = next_close;
    }

    rows
}

// 2026-04-01 CST: 这里补一个横盘等待样本，原因是综合 Tool 不能只会在顺风环境下输出多头确认；
// 目的：后续可以用它验证“个股尚未选边时，不应被环境信号直接抬成确定方向”。
fn build_choppy_history_rows(day_count: usize) -> Vec<String> {
    let mut rows = vec!["trade_date,open,high,low,close,adj_close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("seed date should be valid");

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let wave = match offset % 6 {
            0 => -0.8,
            1 => 0.7,
            2 => -0.6,
            3 => 0.6,
            4 => -0.7,
            _ => 0.8,
        };
        let base = 100.0 + wave;
        let open = base - 0.15;
        let high = base + 0.85;
        let low = base - 0.85;
        let close = base + 0.1;
        let adj_close = close;
        let volume = 900_000 + (offset % 5) as i64 * 80_000;
        rows.push(format!(
            "{},{open:.2},{high:.2},{low:.2},{close:.2},{adj_close:.2},{volume}",
            trade_date.format("%Y-%m-%d")
        ));
    }

    rows
}

// 2026-04-01 CST: 这里补一组稳定下行并在尾部完成有效跌破的样本，原因是方案 B 需要把综合 Tool 的 `headwind` 场景正式锁成回归合同。
// 目的：让“个股看多、环境双空”的逆风场景走真实 `CSV -> SQLite -> Tool` 主链，而不是只停留在代码规则里。
fn build_confirmed_breakdown_rows(day_count: usize, start_close: f64) -> Vec<String> {
    let mut rows = vec!["trade_date,open,high,low,close,adj_close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("seed date should be valid");
    let mut close = start_close;

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let (next_close, volume): (f64, i64) = if offset < day_count - 20 {
            (close - 0.72, 860_000 + offset as i64 * 7_000)
        } else {
            let phase = offset - (day_count - 20);
            match phase % 4 {
                0 => (close - 1.30, 1_660_000 + phase as i64 * 24_000),
                1 => (close + 0.16, 410_000),
                2 => (close - 1.02, 1_520_000 + phase as i64 * 21_000),
                _ => (close - 0.38, 1_200_000),
            }
        };

        let open = close;
        // 2026-04-17 CST: Updated because the stricter breakdown chain now checks
        // support using low-derived key levels, so this fixture must finish below the
        // prior floor instead of leaving oversized lower shadows that blur the signal.
        // Purpose: keep contextual headwind/tailwind tests anchored to real breakdown samples.
        let high = next_close.max(open) + if offset < day_count - 20 { 0.24 } else { 0.12 };
        let low = next_close.min(open) - if offset < day_count - 20 { 0.28 } else { 0.14 };
        let adj_close = next_close;
        rows.push(format!(
            "{},{open:.2},{high:.2},{low:.2},{next_close:.2},{adj_close:.2},{volume}",
            trade_date.format("%Y-%m-%d")
        ));
        close = next_close;
    }

    rows
}

#[test]
fn tool_catalog_includes_security_analysis_contextual() {
    let output = run_cli_with_json("");

    // 2026-04-01 CST: 这里先锁新综合 Tool 的可发现性，原因是如果 catalog 看不见，后续 CLI / Skill 都无法稳定接它；
    // 目的：先把“能力可发现”钉成正式合同，再补具体上下文聚合行为。
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool_catalog should be an array")
            .iter()
            .filter_map(|value| value.as_str())
            .any(|tool| tool == "security_analysis_contextual")
    );
}

#[test]
fn security_analysis_contextual_reports_tailwind_when_stock_market_sector_align() {
    let runtime_db_path = create_test_runtime_db("security_analysis_contextual_tailwind");

    let stock_csv = create_stock_history_csv(
        "security_analysis_contextual_tailwind",
        "stock_breakout.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_analysis_contextual_tailwind",
        "market_breakout.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_analysis_contextual_tailwind",
        "sector_breakout.csv",
        &build_confirmed_breakout_rows(220, 950.0),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let request = json!({
        "tool": "security_analysis_contextual",
        "args": {
            "symbol": "601916.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            // 2026-04-17 CST: Added because this tailwind regression should stay
            // anchored to the synthetic breakout fixture rather than live market drift.
            // Purpose: preserve a stable local proof for contextual alignment logic.
            "as_of_date": FIXTURE_AS_OF_DATE
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-01 CST: 这里先补综合 Tool 的红测，原因是方案 A 的核心不是再造指标，而是把个股/大盘/板块三层收成一个正式合同；
    // 目的：确保三者同向时，新 Tool 会明确返回顺风环境，而不是只把三份子结果平铺给调用方自己再拼。
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["stock_analysis"]["consultation_conclusion"]["bias"],
        "bullish_continuation"
    );
    assert_eq!(
        output["data"]["market_analysis"]["consultation_conclusion"]["bias"],
        "bullish_continuation"
    );
    assert_eq!(
        output["data"]["sector_analysis"]["consultation_conclusion"]["bias"],
        "bullish_continuation"
    );
    assert_eq!(
        output["data"]["contextual_conclusion"]["alignment"],
        "tailwind"
    );
    assert!(
        output["data"]["contextual_conclusion"]["headline"]
            .as_str()
            .expect("headline should exist")
            .contains("顺风")
    );
    assert!(
        output["data"]["contextual_conclusion"]["rationale"]
            .as_array()
            .expect("rationale should exist")
            .iter()
            .filter_map(|value| value.as_str())
            .any(|text| text.contains("大盘") && text.contains("板块") && text.contains("同向"))
    );
}

#[test]
fn security_analysis_contextual_keeps_mixed_when_stock_is_range_wait() {
    let runtime_db_path = create_test_runtime_db("security_analysis_contextual_mixed");

    let stock_csv = create_stock_history_csv(
        "security_analysis_contextual_mixed",
        "stock_choppy.csv",
        &build_choppy_history_rows(220),
    );
    let market_csv = create_stock_history_csv(
        "security_analysis_contextual_mixed",
        "market_breakout.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_analysis_contextual_mixed",
        "sector_breakout.csv",
        &build_confirmed_breakout_rows(220, 950.0),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let request = json!({
        "tool": "security_analysis_contextual",
        "args": {
            "symbol": "601916.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            // 2026-04-17 CST: Added because the mixed-case fixture should be evaluated
            // inside its own local sample window and not upgraded by live sync drift.
            // Purpose: keep the range-wait regression pinned to the intended fixture date.
            "as_of_date": FIXTURE_AS_OF_DATE
        }
    });

    let output = run_cli_with_json_and_runtime(&request.to_string(), &runtime_db_path);

    // 2026-04-01 CST: 这里补第二条边界红测，原因是综合 Tool 不能因为环境偏强就把个股等待态硬拽成方向确认；
    // 目的：确保个股仍在 `range_wait` 时，上层结论会明确保留 mixed / wait 语义，而不是伪造顺风确认。
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["stock_analysis"]["consultation_conclusion"]["bias"],
        "range_wait"
    );
    assert_eq!(
        output["data"]["contextual_conclusion"]["alignment"],
        "mixed"
    );
    assert!(
        output["data"]["contextual_conclusion"]["headline"]
            .as_str()
            .expect("headline should exist")
            .contains("等待")
    );
    assert!(
        output["data"]["contextual_conclusion"]["risk_flags"]
            .as_array()
            .expect("risk_flags should exist")
            .iter()
            .filter_map(|value| value.as_str())
            .any(|text| text.contains("个股自身") && text.contains("未完成"))
    );
}
