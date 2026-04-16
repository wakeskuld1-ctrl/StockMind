use std::collections::HashMap;
use std::fs;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::runtime::stock_history_store::{
    StockHistoryImportSummary, StockHistoryRow, StockHistoryStore, StockHistoryStoreError,
};

// 2026-03-28 CST: 这里定义股票历史导入请求，原因是第一刀要把 CSV 导入入口收口成稳定的强类型合同；
// 目的：避免 dispatcher 手工解析分散字段，并为后续 Skill 调用保留清晰的输入边界。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ImportStockPriceHistoryRequest {
    pub csv_path: String,
    pub symbol: String,
    #[serde(default = "default_import_source")]
    pub source: String,
}

// 2026-03-28 CST: 这里定义股票历史导入结果，原因是外部命令行 EXE 需要稳定 JSON 回执；
// 目的：让后续技术咨询 Tool 能知道导入了多少行、覆盖了什么日期范围、数据实际写到哪里。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportStockPriceHistoryResult {
    pub symbol: String,
    pub source: String,
    pub imported_row_count: usize,
    pub database_path: String,
    pub table_name: String,
    pub date_range: ImportDateRange,
}

// 2026-03-28 CST: 这里单独定义日期范围结构，原因是日期边界会被后续技术面咨询直接复用；
// 目的：先把结果合同做成可扩展结构，而不是平铺字符串。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportDateRange {
    pub start_date: String,
    pub end_date: String,
}

// 2026-03-28 CST: 这里集中定义导入层错误，原因是 CSV 文件、列头、日期、数值、SQLite 任一环节都可能失败；
// 目的：把错误翻译成中文、可读、可定位的提示，方便非 IT 用户排障。
#[derive(Debug, Error)]
pub enum ImportStockPriceHistoryError {
    #[error("无法读取 CSV 文件 `{path}`: {message}")]
    ReadCsv { path: String, message: String },
    #[error("CSV 文件为空，未找到表头")]
    EmptyCsv,
    #[error("CSV 文件没有可导入的数据行")]
    EmptyDataRows,
    #[error("缺少必需列: {0}")]
    MissingColumns(String),
    #[error("第 {line_number} 行列数不足，无法读取 `{field}`")]
    MissingFieldValue { line_number: usize, field: String },
    #[error("第 {line_number} 行交易日期 `{value}` 不是可识别的日期格式")]
    InvalidDate { line_number: usize, value: String },
    #[error("第 {line_number} 行字段 `{field}` 的数值 `{value}` 无法解析")]
    InvalidNumber {
        line_number: usize,
        field: String,
        value: String,
    },
    #[error("{0}")]
    Store(#[from] StockHistoryStoreError),
}

// 2026-03-28 CST: 这里提供股票历史导入主入口，原因是当前 S2-1 第一刀只需要打通 “CSV -> SQLite” 主链路；
// 目的：给后续基础技术面 Tool 提供稳定历史数据底座，同时保持当前改造范围最小。
pub fn import_stock_price_history(
    request: &ImportStockPriceHistoryRequest,
) -> Result<ImportStockPriceHistoryResult, ImportStockPriceHistoryError> {
    let csv_content = fs::read_to_string(&request.csv_path).map_err(|error| {
        ImportStockPriceHistoryError::ReadCsv {
            path: request.csv_path.clone(),
            message: error.to_string(),
        }
    })?;
    let rows = parse_stock_price_history_csv(&csv_content)?;
    let store = StockHistoryStore::workspace_default()?;
    let summary = store.import_rows(&request.symbol, &request.source, &rows)?;

    Ok(build_import_result(request, &summary, &store))
}

// 2026-03-28 CST: 这里集中构造导入回执，原因是持久层摘要和对外 Tool JSON 不是同一个层次；
// 目的：把对外合同固定在业务层，避免后续 SQLite 细节泄漏到 CLI 外部。
fn build_import_result(
    request: &ImportStockPriceHistoryRequest,
    summary: &StockHistoryImportSummary,
    store: &StockHistoryStore,
) -> ImportStockPriceHistoryResult {
    ImportStockPriceHistoryResult {
        symbol: request.symbol.clone(),
        source: request.source.clone(),
        imported_row_count: summary.imported_row_count,
        database_path: store.db_path().display().to_string(),
        table_name: "stock_price_history".to_string(),
        date_range: ImportDateRange {
            start_date: summary.start_date.clone(),
            end_date: summary.end_date.clone(),
        },
    }
}

// 2026-03-28 CST: 这里解析 CSV 文本为标准化日线记录，原因是当前第一刀不引入额外 CSV 依赖，先保持交付链路简单；
// 目的：在不扩依赖栈的前提下支持最小可用导入能力，并为后续逐步增强留出口。
fn parse_stock_price_history_csv(
    csv_content: &str,
) -> Result<Vec<StockHistoryRow>, ImportStockPriceHistoryError> {
    let mut non_empty_lines = csv_content.lines().filter(|line| !line.trim().is_empty());
    let header_line = non_empty_lines
        .next()
        .ok_or(ImportStockPriceHistoryError::EmptyCsv)?;
    let header_map = build_header_index_map(header_line)?;

    let mut rows = Vec::new();
    for (index, line) in non_empty_lines.enumerate() {
        let line_number = index + 2;
        rows.push(parse_stock_price_row(line_number, line, &header_map)?);
    }

    if rows.is_empty() {
        return Err(ImportStockPriceHistoryError::EmptyDataRows);
    }

    Ok(rows)
}

// 2026-03-28 CST: 这里统一校验并映射表头，原因是用户手里的 CSV 列头可能有空格、大小写或下划线差异；
// 目的：让第一版先兼容常见列头口径，而不是把用户挡在导入门外。
fn build_header_index_map(
    header_line: &str,
) -> Result<HashMap<&'static str, usize>, ImportStockPriceHistoryError> {
    let raw_headers = parse_csv_line(header_line);
    let mut normalized_positions = HashMap::<String, usize>::new();

    for (index, header) in raw_headers.iter().enumerate() {
        normalized_positions.insert(normalize_header_name(header), index);
    }

    let required_fields = [
        ("trade_date", ["trade_date", "date", "tradedate"].as_slice()),
        ("open", ["open"].as_slice()),
        ("high", ["high"].as_slice()),
        ("low", ["low"].as_slice()),
        ("close", ["close"].as_slice()),
        ("volume", ["volume"].as_slice()),
    ];

    let mut header_map = HashMap::<&'static str, usize>::new();
    let mut missing_fields = Vec::<String>::new();

    for (canonical_name, aliases) in required_fields {
        if let Some(index) = aliases.iter().find_map(|alias| {
            normalized_positions
                .get(&normalize_header_name(alias))
                .copied()
        }) {
            header_map.insert(canonical_name, index);
        } else {
            missing_fields.push(canonical_name.to_string());
        }
    }

    // 2026-04-12 CST: Keep adj_close as an optional alias group, because some
    // sources provide an explicit adjusted close while validation fixtures may
    // only provide close.
    // Purpose: reuse adjusted prices when available without rejecting lean CSV inputs.
    if let Some(index) = [
        "adj_close",
        "adjclose",
        "adj close",
        "adjusted_close",
        "adjustedclose",
    ]
    .iter()
    .find_map(|alias| {
        normalized_positions
            .get(&normalize_header_name(alias))
            .copied()
    }) {
        header_map.insert("adj_close", index);
    }

    if !missing_fields.is_empty() {
        return Err(ImportStockPriceHistoryError::MissingColumns(
            missing_fields.join(", "),
        ));
    }

    Ok(header_map)
}

// 2026-03-28 CST: 这里解析单行 CSV 记录，原因是历史行情的每个字段都需要在入库前先做类型校验；
// 目的：保证 SQLite 里的日线记录已经是标准化值，避免后续技术指标层再重复清洗。
fn parse_stock_price_row(
    line_number: usize,
    line: &str,
    header_map: &HashMap<&'static str, usize>,
) -> Result<StockHistoryRow, ImportStockPriceHistoryError> {
    let values = parse_csv_line(line);

    Ok(StockHistoryRow {
        trade_date: parse_trade_date(
            line_number,
            field_value(&values, header_map, "trade_date", line_number)?,
        )?,
        open: parse_f64_field(
            line_number,
            "open",
            field_value(&values, header_map, "open", line_number)?,
        )?,
        high: parse_f64_field(
            line_number,
            "high",
            field_value(&values, header_map, "high", line_number)?,
        )?,
        low: parse_f64_field(
            line_number,
            "low",
            field_value(&values, header_map, "low", line_number)?,
        )?,
        close: parse_f64_field(
            line_number,
            "close",
            field_value(&values, header_map, "close", line_number)?,
        )?,
        // 2026-04-12 CST: Treat adj_close as optional, because validation slices
        // and lightweight fixtures often only carry close while still being
        // perfectly valid for governed replay.
        // Purpose: keep price-history import usable when adjusted close is absent by reusing close.
        adj_close: parse_optional_adj_close(
            line_number,
            &values,
            header_map,
            field_value(&values, header_map, "close", line_number)?,
        )?,
        volume: parse_i64_field(
            line_number,
            "volume",
            field_value(&values, header_map, "volume", line_number)?,
        )?,
    })
}

// 2026-04-12 CST: Parse adj_close when present, because some upstream CSV files
// provide explicit adjusted close while lighter fixtures only provide close.
// Purpose: preserve adjusted prices when available and fall back to close otherwise.
fn parse_optional_adj_close(
    line_number: usize,
    values: &[String],
    header_map: &HashMap<&'static str, usize>,
    close_value: &str,
) -> Result<f64, ImportStockPriceHistoryError> {
    if header_map.contains_key("adj_close") {
        return parse_f64_field(
            line_number,
            "adj_close",
            field_value(values, header_map, "adj_close", line_number)?,
        );
    }

    parse_f64_field(line_number, "close", close_value)
}

// 2026-03-28 CST: 这里统一读取字段值，原因是 CSV 行列数不足时需要返回清晰的列名和行号；
// 目的：避免报出笼统的“index out of bounds”之类对业务方无意义的错误。
fn field_value<'a>(
    values: &'a [String],
    header_map: &HashMap<&'static str, usize>,
    field: &'static str,
    line_number: usize,
) -> Result<&'a str, ImportStockPriceHistoryError> {
    let index = *header_map
        .get(field)
        .expect("required fields should have been validated before row parsing");
    values.get(index).map(String::as_str).ok_or_else(|| {
        ImportStockPriceHistoryError::MissingFieldValue {
            line_number,
            field: field.to_string(),
        }
    })
}

// 2026-03-28 CST: 这里统一做 CSV 行切分，原因是第一刀虽然不引入 csv crate，也要最少支持引号包裹字段；
// 目的：避免遇到简单引号场景就把整条导入链路打断。
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::<String>::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    current.push('"');
                    let _ = chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                fields.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    fields.push(current.trim().to_string());
    fields
}

// 2026-03-28 CST: 这里统一标准化列头名，原因是 CSV 表头经常混有空格、下划线、横杠和大小写差异；
// 目的：让第一版导入对常见口径更宽容，但仍保持最终映射字段明确。
fn normalize_header_name(value: &str) -> String {
    value.trim().to_lowercase().replace([' ', '_', '-'], "")
}

// 2026-03-28 CST: 这里统一解析交易日期，原因是后续技术指标和咨询层都依赖稳定的日期排序；
// 目的：把常见 CSV 日期格式收口为标准 `YYYY-MM-DD`。
fn parse_trade_date(
    line_number: usize,
    value: &str,
) -> Result<String, ImportStockPriceHistoryError> {
    let trimmed = value.trim();
    let parsed = ["%Y-%m-%d", "%Y/%m/%d", "%Y%m%d"]
        .iter()
        .find_map(|pattern| NaiveDate::parse_from_str(trimmed, pattern).ok());

    parsed
        .map(|date| date.format("%Y-%m-%d").to_string())
        .ok_or_else(|| ImportStockPriceHistoryError::InvalidDate {
            line_number,
            value: trimmed.to_string(),
        })
}

// 2026-03-28 CST: 这里统一解析浮点字段，原因是价格列既需要类型校验，也要兼容带千分位的文本数字；
// 目的：保证入库前价格字段都是可直接用于技术指标计算的标准数值。
fn parse_f64_field(
    line_number: usize,
    field: &'static str,
    value: &str,
) -> Result<f64, ImportStockPriceHistoryError> {
    let normalized = normalize_number_text(value);
    normalized
        .parse::<f64>()
        .map_err(|_| ImportStockPriceHistoryError::InvalidNumber {
            line_number,
            field: field.to_string(),
            value: value.to_string(),
        })
}

// 2026-03-28 CST: 这里统一解析成交量字段，原因是 volume 在数据库里更适合用整数存储；
// 目的：避免后续查询和技术指标逻辑再重复判断类型。
fn parse_i64_field(
    line_number: usize,
    field: &'static str,
    value: &str,
) -> Result<i64, ImportStockPriceHistoryError> {
    let normalized = normalize_number_text(value);
    if let Ok(integer_value) = normalized.parse::<i64>() {
        return Ok(integer_value);
    }

    normalized
        .parse::<f64>()
        .map(|value| value.round() as i64)
        .map_err(|_| ImportStockPriceHistoryError::InvalidNumber {
            line_number,
            field: field.to_string(),
            value: value.to_string(),
        })
}

// 2026-03-28 CST: 这里统一清洗数字文本，原因是一些 CSV 导出会带千分位或空白字符；
// 目的：把最常见的脏格式在入库前先处理掉，减少误报。
fn normalize_number_text(value: &str) -> String {
    value.trim().replace(',', "")
}

// 2026-03-28 CST: 这里给 source 提供默认值，原因是手工 CSV 导入是当前第一刀的主场景；
// 目的：在调用方没显式传 source 时，也能得到稳定可追踪的数据来源字段。
fn default_import_source() -> String {
    "csv_import".to_string()
}
