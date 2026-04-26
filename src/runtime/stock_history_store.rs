use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, params};
use thiserror::Error;

use crate::runtime::formal_security_runtime_registry::FormalSecurityRuntimeRegistry;

// 2026-03-28 CST: 这里定义单条股票日线记录，原因是 CSV 导入和后续技术指标计算都会复用同一份标准化历史结构；
// 目的：把“文本 CSV 行”先收口成稳定的 Rust 结构，避免后面每个 Tool 都重复解析字段。
// 2026-04-15 CST: English anchor comment added because older mojibake comments in this
// file became misleading during the second-layer runtime cleanup.
// Purpose: make the key store contracts understandable without expanding this round into
// a full comment rewrite.
#[derive(Debug, Clone, PartialEq)]
pub struct StockHistoryRow {
    pub trade_date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub adj_close: f64,
    pub volume: i64,
}

// 2026-03-28 CST: 这里定义导入摘要，原因是外部 EXE Tool 需要回执导入结果，而不是只做静默落库；
// 目的：让后续 Skill、CLI 和交接链路都能直接消费统一的导入结果合同。
// Purpose: summarize one import run in a stable contract that CLI and later tools can reuse.
#[derive(Debug, Clone, PartialEq)]
pub struct StockHistoryImportSummary {
    pub imported_row_count: usize,
    pub start_date: String,
    pub end_date: String,
}

// 2026-04-14 CST: Added because stock training readiness now needs one stable coverage summary
// reader instead of letting each audit-style tool hand-write COUNT/MIN/MAX SQL.
// Purpose: centralize symbol-level history coverage statistics on the official stock store.
#[derive(Debug, Clone, PartialEq)]
pub struct StockHistoryCoverageSummary {
    pub first_trade_date: String,
    pub last_trade_date: String,
    pub history_days: usize,
}

// 2026-04-25 CST: Added because Nikkei volume governance needs source-level
// volume coverage and provenance before training can consume proxy volume safely.
// Purpose: provide one store contract for volume manifests without duplicating SQL in ops.
#[derive(Debug, Clone, PartialEq)]
pub struct StockHistoryVolumeSourceSummary {
    pub first_trade_date: String,
    pub last_trade_date: String,
    pub row_count: usize,
    pub nonzero_volume_rows: usize,
    pub zero_volume_rows: usize,
    pub min_volume: i64,
    pub max_volume: i64,
    pub source_names: Vec<String>,
}

// 2026-03-28 CST: 这里定义股票历史 SQLite Store，原因是用户已经明确历史数据要走 SQLite；
// 目的：把股票历史表和 session/runtime 记忆分离，既复用同一个 runtime 根目录，又不把两类表硬耦合到一起。
// Purpose: keep stock-history persistence inside the governed runtime family while staying
// separate from session/local-memory state.
#[derive(Debug, Clone)]
pub struct StockHistoryStore {
    db_path: PathBuf,
}

// 2026-03-28 CST: 这里集中定义股票历史存储层错误，原因是 CSV 解析层和 SQLite 层都可能失败；
// 目的：让上层 Tool 能拿到清晰、中文、可定位的问题信息。
// Purpose: keep read/write/bootstrap failures separated so upper layers get diagnosable
// storage errors instead of one generic SQLite failure.
#[derive(Debug, Error)]
pub enum StockHistoryStoreError {
    #[error("无法确定股票历史 SQLite 所在目录: {0}")]
    ResolveRuntimeDir(String),
    #[error("无法创建股票历史 SQLite 目录: {0}")]
    CreateRuntimeDir(String),
    #[error("无法打开股票历史 SQLite: {0}")]
    OpenDatabase(String),
    #[error("无法初始化股票历史表结构: {0}")]
    BootstrapSchema(String),
    #[error("股票历史数据不能为空")]
    EmptyRows,
    #[error("无法写入股票历史数据: {0}")]
    WriteRows(String),
    // 2026-03-28 CST：这里补充历史读取错误类型，原因是技术面咨询 Tool 已经开始直接依赖 SQLite 历史查询；
    // 目的：把“写入失败”和“读取失败”在存储层明确拆开，便于上层返回更准确的中文错误。
    #[error("无法读取股票历史数据: {0}")]
    ReadRows(String),
    // 2026-04-14 CST：这里补 legacy 股票库并入错误，原因是当前仓库已经确认同时存在 `runtime/stock_history.db`
    // 与 `.excel_skill_runtime/stock_history.db` 两套口径；目的：在收口正式 runtime 时能给出清晰失败原因。
    #[error("镜像 legacy 股票历史库失败: {0}")]
    LegacyBootstrap(String),
}

struct LegacyStockHistoryRow {
    symbol: String,
    trade_date: String,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    adj_close: f64,
    volume: i64,
    source: String,
}

impl StockHistoryStore {
    // 2026-03-28 CST: 这里允许显式指定数据库路径，原因是测试和后续命令行打包都可能需要自定义落盘位置；
    // 目的：保留“同一逻辑，不同落盘目录”的扩展点，同时不增加当前业务复杂度。
    // 2026-04-15 CST: English anchor for the explicit-constructor path.
    // Purpose: keep tests and isolated runtime slices readable while the older
    // historical comments are still being cleaned in smaller steps.
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    // 2026-03-28 CST: 这里提供工作区默认数据库路径，原因是当前第一刀要和现有 runtime 根目录保持一致；
    // 目的：让 `EXCEL_SKILL_RUNTIME_DB` 的测试隔离能力能自动覆盖股票历史落盘，而不再新增一套测试环境变量。
    // 2026-04-15 CST: Resolve the default stock-history path through the formal runtime
    // registry.
    // Purpose: keep stock history on the same governed default-path policy as the rest
    // of the securities runtime family.
    pub fn workspace_default() -> Result<Self, StockHistoryStoreError> {
        let store = Self::new(
            FormalSecurityRuntimeRegistry::stock_history_db_path()
                .map_err(StockHistoryStoreError::ResolveRuntimeDir)?,
        );

        // 2026-04-14 CST：这里把 legacy `runtime/stock_history.db` 自动并入正式 `.excel_skill_runtime`，
        // 原因是当前工作区已经出现 CLI 与研究链各读各库的情况；目的：把正式运行时唯一口径收口到 `.excel_skill_runtime`。
        // 2026-04-15 CST: Keep the legacy bootstrap as the only intentional exception in
        // this store.
        // Purpose: merge the old workspace `runtime/stock_history.db` into the governed
        // runtime root without reintroducing a second default-path rule.
        if should_bootstrap_workspace_stock_history_from_legacy() {
            store.bootstrap_workspace_stock_history_from_legacy()?;
        }

        Ok(store)
    }

    // 2026-03-28 CST: 这里暴露数据库路径，原因是导入 Tool 回执需要告诉上层数据实际落到哪里；
    // 目的：方便后续 Skill/交接/排障定位 SQLite 文件。
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    // 2026-03-28 CST: 这里集中执行 upsert 导入，原因是“同一 symbol + trade_date 覆盖”是历史补数和重导入的核心规则；
    // 目的：保证技术指标计算读取到的总是一份去重后的日线历史，而不是重复交易日。
    pub fn import_rows(
        &self,
        symbol: &str,
        source: &str,
        rows: &[StockHistoryRow],
    ) -> Result<StockHistoryImportSummary, StockHistoryStoreError> {
        if rows.is_empty() {
            return Err(StockHistoryStoreError::EmptyRows);
        }

        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| StockHistoryStoreError::WriteRows(error.to_string()))?;

        for row in rows {
            transaction
                .execute(
                    "INSERT INTO stock_price_history (
                        symbol,
                        trade_date,
                        open,
                        high,
                        low,
                        close,
                        adj_close,
                        volume,
                        source
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                    ON CONFLICT(symbol, trade_date) DO UPDATE SET
                        open = excluded.open,
                        high = excluded.high,
                        low = excluded.low,
                        close = excluded.close,
                        adj_close = excluded.adj_close,
                        volume = excluded.volume,
                        source = excluded.source,
                        updated_at = CURRENT_TIMESTAMP",
                    params![
                        symbol,
                        row.trade_date,
                        row.open,
                        row.high,
                        row.low,
                        row.close,
                        row.adj_close,
                        row.volume,
                        source,
                    ],
                )
                .map_err(|error| StockHistoryStoreError::WriteRows(error.to_string()))?;
        }

        transaction
            .commit()
            .map_err(|error| StockHistoryStoreError::WriteRows(error.to_string()))?;

        let start_date = rows
            .iter()
            .map(|row| row.trade_date.as_str())
            .min()
            .expect("rows should not be empty")
            .to_string();
        let end_date = rows
            .iter()
            .map(|row| row.trade_date.as_str())
            .max()
            .expect("rows should not be empty")
            .to_string();

        Ok(StockHistoryImportSummary {
            imported_row_count: rows.len(),
            start_date,
            end_date,
        })
    }

    // 2026-03-28 CST: 这里统一打开并初始化股票历史 SQLite，原因是上层 Tool 不应该关心建库和建表细节；
    // 目的：让导入 Tool 和后续技术咨询 Tool 共用同一套持久层入口。
    // 2026-03-28 CST：这里新增最近历史读取方法，原因是 `technical_consultation_basic` 需要沿现有 SQLite 主线直接取最近窗口数据；
    // 目的：统一收口“按 symbol + 截止日期 + 回看窗口读取升序历史”的 SQL，避免上层 Tool 重复拼接查询。
    pub fn load_recent_rows(
        &self,
        symbol: &str,
        as_of_date: Option<&str>,
        lookback_days: usize,
    ) -> Result<Vec<StockHistoryRow>, StockHistoryStoreError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "
                SELECT trade_date, open, high, low, close, adj_close, volume
                FROM stock_price_history
                WHERE symbol = ?1
                  AND (?2 IS NULL OR trade_date <= ?2)
                ORDER BY trade_date DESC
                LIMIT ?3
                ",
            )
            .map_err(|error| StockHistoryStoreError::ReadRows(error.to_string()))?;

        let mapped_rows = statement
            .query_map(params![symbol, as_of_date, lookback_days as i64], |row| {
                Ok(StockHistoryRow {
                    trade_date: row.get(0)?,
                    open: row.get(1)?,
                    high: row.get(2)?,
                    low: row.get(3)?,
                    close: row.get(4)?,
                    adj_close: row.get(5)?,
                    volume: row.get(6)?,
                })
            })
            .map_err(|error| StockHistoryStoreError::ReadRows(error.to_string()))?;

        let mut rows = Vec::new();
        for row in mapped_rows {
            rows.push(row.map_err(|error| StockHistoryStoreError::ReadRows(error.to_string()))?);
        }

        // 2026-03-28 CST：这里反转成升序结果，原因是 SQLite 倒序取最近窗口最直接；
        // 目的：保证上层指标计算永远面对“从旧到新”的稳定输入，减少重复排序逻辑。
        rows.reverse();
        Ok(rows)
    }

    // 2026-04-09 CST: 这里新增向未来窗口读取方法，原因是 Task 3 的 forward_outcome 需要基于同一份 SQLite 历史表回填未来多期限标签；
    // 目的：把“按 symbol + 截止日之后 + 固定窗口读取升序未来行情”的 SQL 统一收口，避免标签层和后续训练层重复拼接查询。
    pub fn load_forward_rows(
        &self,
        symbol: &str,
        after_date: &str,
        forward_days: usize,
    ) -> Result<Vec<StockHistoryRow>, StockHistoryStoreError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "
                SELECT trade_date, open, high, low, close, adj_close, volume
                FROM stock_price_history
                WHERE symbol = ?1
                  AND trade_date > ?2
                ORDER BY trade_date ASC
                LIMIT ?3
                ",
            )
            .map_err(|error| StockHistoryStoreError::ReadRows(error.to_string()))?;

        let mapped_rows = statement
            .query_map(params![symbol, after_date, forward_days as i64], |row| {
                Ok(StockHistoryRow {
                    trade_date: row.get(0)?,
                    open: row.get(1)?,
                    high: row.get(2)?,
                    low: row.get(3)?,
                    close: row.get(4)?,
                    adj_close: row.get(5)?,
                    volume: row.get(6)?,
                })
            })
            .map_err(|error| StockHistoryStoreError::ReadRows(error.to_string()))?;

        let mut rows = Vec::new();
        for row in mapped_rows {
            rows.push(row.map_err(|error| StockHistoryStoreError::ReadRows(error.to_string()))?);
        }

        Ok(rows)
    }

    // 2026-04-14 CST: 这里补旧接口兼容层，原因是信号研究链仍按 `load_rows_after`
    // 命名读取未来窗口，而当前 store 已经收口为 `load_forward_rows`。
    // 目的：先恢复现有研究链编译，不在这一轮同时推倒重命名所有调用面。
    pub fn load_rows_after(
        &self,
        symbol: &str,
        after_date: &str,
        limit: usize,
    ) -> Result<Vec<StockHistoryRow>, StockHistoryStoreError> {
        self.load_forward_rows(symbol, after_date, limit)
    }

    // 2026-04-14 CST: 这里补按日期区间读取兼容接口，原因是共振模板同步仍直接调用
    // `load_rows_in_range`，而当前工作区文件里该接口在合并后丢失。
    // 目的：把日期区间查询继续收口在 store 层，避免上层重复拼 SQL。
    pub fn load_rows_in_range(
        &self,
        symbol: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<StockHistoryRow>, StockHistoryStoreError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "
                SELECT trade_date, open, high, low, close, adj_close, volume
                FROM stock_price_history
                WHERE symbol = ?1
                  AND trade_date >= ?2
                  AND trade_date <= ?3
                ORDER BY trade_date ASC
                ",
            )
            .map_err(|error| StockHistoryStoreError::ReadRows(error.to_string()))?;

        let mapped_rows = statement
            .query_map(params![symbol, start_date, end_date], |row| {
                Ok(StockHistoryRow {
                    trade_date: row.get(0)?,
                    open: row.get(1)?,
                    high: row.get(2)?,
                    low: row.get(3)?,
                    close: row.get(4)?,
                    adj_close: row.get(5)?,
                    volume: row.get(6)?,
                })
            })
            .map_err(|error| StockHistoryStoreError::ReadRows(error.to_string()))?;

        let mut rows = Vec::new();
        for row in mapped_rows {
            rows.push(row.map_err(|error| StockHistoryStoreError::ReadRows(error.to_string()))?);
        }
        Ok(rows)
    }

    // 2026-04-14 CST: 这里补本地最后交易日查询，原因是日期门禁与自动补数链已经把
    // “本地数据最晚覆盖到哪天”作为正式判断条件。
    // 目的：把最后交易日读取统一收口在 store 层，避免 guard 自己拼 SQL。
    pub fn latest_trade_date(
        &self,
        symbol: &str,
    ) -> Result<Option<String>, StockHistoryStoreError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "
                SELECT trade_date
                FROM stock_price_history
                WHERE symbol = ?1
                ORDER BY trade_date DESC
                LIMIT 1
                ",
            )
            .map_err(|error| StockHistoryStoreError::ReadRows(error.to_string()))?;
        statement
            .query_row(params![symbol], |row| row.get::<_, String>(0))
            .map(Some)
            .or_else(|error| {
                if matches!(error, rusqlite::Error::QueryReturnedNoRows) {
                    Ok(None)
                } else {
                    Err(StockHistoryStoreError::ReadRows(error.to_string()))
                }
            })
    }

    // 2026-04-14 CST: 这里补“请求日及之前最近交易日”查询，原因是用户已经要求
    // 分析链必须自动回退到最近有效交易日。
    // 目的：继续让日期回退逻辑以 store + guard 的正式组合实现，而不是散落在 Tool 层。
    pub fn latest_trade_date_on_or_before(
        &self,
        symbol: &str,
        as_of_date: &str,
    ) -> Result<Option<String>, StockHistoryStoreError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "
                SELECT trade_date
                FROM stock_price_history
                WHERE symbol = ?1
                  AND trade_date <= ?2
                ORDER BY trade_date DESC
                LIMIT 1
                ",
            )
            .map_err(|error| StockHistoryStoreError::ReadRows(error.to_string()))?;
        statement
            .query_row(params![symbol, as_of_date], |row| row.get::<_, String>(0))
            .map(Some)
            .or_else(|error| {
                if matches!(error, rusqlite::Error::QueryReturnedNoRows) {
                    Ok(None)
                } else {
                    Err(StockHistoryStoreError::ReadRows(error.to_string()))
                }
            })
    }

    // 2026-04-14 CST: Added because the new stock training coverage audit must answer
    // "how much usable history exists for this symbol" from the official runtime store.
    // Purpose: expose one store-level coverage query that returns start date, end date,
    // and effective row count without forcing business ops to duplicate SQL.
    pub fn load_coverage_summary(
        &self,
        symbol: &str,
        as_of_date: Option<&str>,
    ) -> Result<Option<StockHistoryCoverageSummary>, StockHistoryStoreError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "
                SELECT
                    MIN(trade_date) AS first_trade_date,
                    MAX(trade_date) AS last_trade_date,
                    COUNT(*) AS history_days
                FROM stock_price_history
                WHERE symbol = ?1
                  AND (?2 IS NULL OR trade_date <= ?2)
                ",
            )
            .map_err(|error| StockHistoryStoreError::ReadRows(error.to_string()))?;

        statement
            .query_row(params![symbol, as_of_date], |row| {
                let first_trade_date: Option<String> = row.get(0)?;
                let last_trade_date: Option<String> = row.get(1)?;
                let history_days: i64 = row.get(2)?;
                Ok((first_trade_date, last_trade_date, history_days))
            })
            .map_err(|error| StockHistoryStoreError::ReadRows(error.to_string()))
            .and_then(|(first_trade_date, last_trade_date, history_days)| {
                match (first_trade_date, last_trade_date) {
                    (Some(first_trade_date), Some(last_trade_date)) if history_days > 0 => {
                        Ok(Some(StockHistoryCoverageSummary {
                            first_trade_date,
                            last_trade_date,
                            history_days: history_days as usize,
                        }))
                    }
                    _ => Ok(None),
                }
            })
    }

    // 2026-04-25 CST: Added because the volume-source manifest must distinguish
    // "has price rows but no volume" from "has a short non-zero volume proxy".
    // Purpose: centralize source/date/volume coverage statistics on the official store.
    pub fn load_volume_source_summary(
        &self,
        symbol: &str,
        as_of_date: Option<&str>,
    ) -> Result<Option<StockHistoryVolumeSourceSummary>, StockHistoryStoreError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "
                SELECT
                    MIN(trade_date) AS first_trade_date,
                    MAX(trade_date) AS last_trade_date,
                    COUNT(*) AS row_count,
                    SUM(CASE WHEN volume > 0 THEN 1 ELSE 0 END) AS nonzero_volume_rows,
                    SUM(CASE WHEN volume = 0 THEN 1 ELSE 0 END) AS zero_volume_rows,
                    MIN(volume) AS min_volume,
                    MAX(volume) AS max_volume,
                    GROUP_CONCAT(DISTINCT source) AS source_names
                FROM stock_price_history
                WHERE symbol = ?1
                  AND (?2 IS NULL OR trade_date <= ?2)
                ",
            )
            .map_err(|error| StockHistoryStoreError::ReadRows(error.to_string()))?;

        statement
            .query_row(params![symbol, as_of_date], |row| {
                let first_trade_date: Option<String> = row.get(0)?;
                let last_trade_date: Option<String> = row.get(1)?;
                let row_count: i64 = row.get(2)?;
                let nonzero_volume_rows: Option<i64> = row.get(3)?;
                let zero_volume_rows: Option<i64> = row.get(4)?;
                let min_volume: Option<i64> = row.get(5)?;
                let max_volume: Option<i64> = row.get(6)?;
                let source_names: Option<String> = row.get(7)?;
                Ok((
                    first_trade_date,
                    last_trade_date,
                    row_count,
                    nonzero_volume_rows,
                    zero_volume_rows,
                    min_volume,
                    max_volume,
                    source_names,
                ))
            })
            .map_err(|error| StockHistoryStoreError::ReadRows(error.to_string()))
            .and_then(
                |(
                    first_trade_date,
                    last_trade_date,
                    row_count,
                    nonzero_volume_rows,
                    zero_volume_rows,
                    min_volume,
                    max_volume,
                    source_names,
                )| match (first_trade_date, last_trade_date) {
                    (Some(first_trade_date), Some(last_trade_date)) if row_count > 0 => {
                        Ok(Some(StockHistoryVolumeSourceSummary {
                            first_trade_date,
                            last_trade_date,
                            row_count: row_count as usize,
                            nonzero_volume_rows: nonzero_volume_rows.unwrap_or_default() as usize,
                            zero_volume_rows: zero_volume_rows.unwrap_or_default() as usize,
                            min_volume: min_volume.unwrap_or_default(),
                            max_volume: max_volume.unwrap_or_default(),
                            source_names: source_names
                                .unwrap_or_default()
                                .split(',')
                                .map(str::trim)
                                .filter(|source| !source.is_empty())
                                .map(str::to_string)
                                .collect(),
                        }))
                    }
                    _ => Ok(None),
                },
            )
    }

    fn open_connection(&self) -> Result<Connection, StockHistoryStoreError> {
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| StockHistoryStoreError::CreateRuntimeDir(error.to_string()))?;
        }

        let connection = Connection::open(&self.db_path)
            .map_err(|error| StockHistoryStoreError::OpenDatabase(error.to_string()))?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|error| StockHistoryStoreError::OpenDatabase(error.to_string()))?;
        self.bootstrap_schema(&connection)?;
        Ok(connection)
    }

    // 2026-03-28 CST: 这里初始化股票历史表，原因是第一刀只需要最小历史表，不应把指标缓存和咨询结果表一起硬塞进来；
    // 目的：先稳住 `stock_price_history` 主表，再逐步往上叠技术面能力。
    // 2026-04-14 CST：这里新增 legacy 股票库自动并入入口，原因是用户已经明确要求统一正式 runtime 口径，
    // 不再允许 `runtime/` 与 `.excel_skill_runtime/` 各自积累不同证券数据；目的：让正式 CLI 与研究链最终回到同一份行情库。
    fn bootstrap_workspace_stock_history_from_legacy(&self) -> Result<(), StockHistoryStoreError> {
        let Some(legacy_db_path) = legacy_workspace_stock_history_db_path()? else {
            return Ok(());
        };

        if legacy_db_path == self.db_path {
            return Ok(());
        }

        if !should_refresh_from_legacy(&self.db_path, &legacy_db_path)? {
            return Ok(());
        }

        eprintln!(
            "warning: 检测到 legacy 股票库 `{}`，正在并入正式 runtime `{}`",
            legacy_db_path.display(),
            self.db_path.display()
        );

        let legacy_rows = load_legacy_stock_history_rows(&legacy_db_path)?;
        if legacy_rows.is_empty() {
            return Ok(());
        }

        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| StockHistoryStoreError::LegacyBootstrap(error.to_string()))?;

        for row in legacy_rows {
            transaction
                .execute(
                    "INSERT INTO stock_price_history (
                        symbol,
                        trade_date,
                        open,
                        high,
                        low,
                        close,
                        adj_close,
                        volume,
                        source
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                    ON CONFLICT(symbol, trade_date) DO UPDATE SET
                        open = excluded.open,
                        high = excluded.high,
                        low = excluded.low,
                        close = excluded.close,
                        adj_close = excluded.adj_close,
                        volume = excluded.volume,
                        source = excluded.source,
                        updated_at = CURRENT_TIMESTAMP",
                    params![
                        row.symbol,
                        row.trade_date,
                        row.open,
                        row.high,
                        row.low,
                        row.close,
                        row.adj_close,
                        row.volume,
                        row.source,
                    ],
                )
                .map_err(|error| StockHistoryStoreError::LegacyBootstrap(error.to_string()))?;
        }

        transaction
            .commit()
            .map_err(|error| StockHistoryStoreError::LegacyBootstrap(error.to_string()))?;
        Ok(())
    }

    fn bootstrap_schema(&self, connection: &Connection) -> Result<(), StockHistoryStoreError> {
        connection
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS stock_price_history (
                    symbol TEXT NOT NULL,
                    trade_date TEXT NOT NULL,
                    open REAL NOT NULL,
                    high REAL NOT NULL,
                    low REAL NOT NULL,
                    close REAL NOT NULL,
                    adj_close REAL NOT NULL,
                    volume INTEGER NOT NULL,
                    source TEXT NOT NULL,
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    PRIMARY KEY(symbol, trade_date)
                );

                CREATE INDEX IF NOT EXISTS idx_stock_price_history_symbol_date
                ON stock_price_history(symbol, trade_date);
                ",
            )
            .map_err(|error| StockHistoryStoreError::BootstrapSchema(error.to_string()))?;
        Ok(())
    }
}

// 2026-04-14 CST：这里把“是否允许触发 legacy 股票库并入”单独收口，原因是测试和显式 runtime 路径不应被工作区默认逻辑污染。
// 目的：只在正式工作区默认 `.excel_skill_runtime` 路径下触发自动迁移，不影响测试隔离库和人工指定库。
fn should_bootstrap_workspace_stock_history_from_legacy() -> bool {
    std::env::var("EXCEL_SKILL_RUNTIME_DIR").is_err()
        && std::env::var("EXCEL_SKILL_RUNTIME_DB").is_err()
}

// 2026-04-14 CST：这里集中解析 legacy `runtime/stock_history.db` 路径，原因是历史研究阶段残留的旧口径已经明确存在；
// 目的：统一识别旧证券行情库，而不是在多个调用点手工拼接路径。
fn legacy_workspace_stock_history_db_path() -> Result<Option<PathBuf>, StockHistoryStoreError> {
    let current_dir = std::env::current_dir()
        .map_err(|error| StockHistoryStoreError::ResolveRuntimeDir(error.to_string()))?;
    let candidate = current_dir.join("runtime").join("stock_history.db");
    if candidate.is_file() {
        return Ok(Some(candidate));
    }
    Ok(None)
}

// 2026-04-14 CST：这里按文件修改时间决定是否重新并入 legacy 股票库，原因是自动迁移不能每次打开库都全量重放；
// 目的：在保证一轮收口可落地的同时，把额外开销控制在旧库更新后才触发。
fn should_refresh_from_legacy(
    target_db_path: &Path,
    legacy_db_path: &Path,
) -> Result<bool, StockHistoryStoreError> {
    if !target_db_path.exists() {
        return Ok(true);
    }

    let legacy_modified = fs::metadata(legacy_db_path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| StockHistoryStoreError::LegacyBootstrap(error.to_string()))?;
    let target_modified = fs::metadata(target_db_path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| StockHistoryStoreError::LegacyBootstrap(error.to_string()))?;
    Ok(legacy_modified > target_modified)
}

// 2026-04-14 CST：这里集中读取 legacy 股票历史行，原因是迁移逻辑需要保留旧库里的 `source` 与逐行 upsert 能力；
// 目的：避免把“读取旧库”和“写入正式库”耦合在同一段事务代码里，降低后续维护成本。
fn load_legacy_stock_history_rows(
    legacy_db_path: &Path,
) -> Result<Vec<LegacyStockHistoryRow>, StockHistoryStoreError> {
    let connection = Connection::open(legacy_db_path)
        .map_err(|error| StockHistoryStoreError::LegacyBootstrap(error.to_string()))?;
    let mut statement = connection
        .prepare(
            "
            SELECT symbol, trade_date, open, high, low, close, adj_close, volume, source
            FROM stock_price_history
            ORDER BY symbol ASC, trade_date ASC
            ",
        )
        .map_err(|error| StockHistoryStoreError::LegacyBootstrap(error.to_string()))?;
    let mapped_rows = statement
        .query_map([], |row| {
            Ok(LegacyStockHistoryRow {
                symbol: row.get(0)?,
                trade_date: row.get(1)?,
                open: row.get(2)?,
                high: row.get(3)?,
                low: row.get(4)?,
                close: row.get(5)?,
                adj_close: row.get(6)?,
                volume: row.get(7)?,
                source: row.get(8)?,
            })
        })
        .map_err(|error| StockHistoryStoreError::LegacyBootstrap(error.to_string()))?;

    let mut rows = Vec::new();
    for row in mapped_rows {
        rows.push(row.map_err(|error| StockHistoryStoreError::LegacyBootstrap(error.to_string()))?);
    }
    Ok(rows)
}
