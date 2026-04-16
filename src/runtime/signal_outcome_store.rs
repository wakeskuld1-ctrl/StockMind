use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, params};
use serde::Serialize;
use thiserror::Error;

use crate::runtime::formal_security_runtime_registry::FormalSecurityRuntimeRegistry;

// 2026-04-02 CST：这里定义信号快照研究行，原因是方案C要求先把“当前状态”沉成统一研究资产；
// 目的：让后续 forward returns、analog study、briefing 和投决会都能基于同一份快照事实继续扩展。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecuritySignalSnapshotRow {
    pub symbol: String,
    pub snapshot_date: String,
    pub indicator_digest: String,
    pub resonance_score: f64,
    pub action_bias: String,
    pub snapshot_payload: String,
}

// 2026-04-02 CST：这里定义未来收益研究行，原因是用户明确要求把“历史上类似状态后面怎么走”做成正式平台能力；
// 目的：为后续 1/3/5/10/20 日 forward returns 与回撤/上冲统计提供统一结构。
// 2026-04-02 CST: 这里补充 Serialize 派生，原因是 signal_outcome_research 的结果对象已经开始直接回传 forward return 行；
// 目的：让运行时研究行既能落库，也能被上层 Tool 直接序列化输出，而不需要额外复制一份 DTO。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecuritySignalForwardReturnRow {
    pub symbol: String,
    pub snapshot_date: String,
    pub horizon_days: i64,
    pub forward_return_pct: f64,
    pub max_drawdown_pct: f64,
    pub max_runup_pct: f64,
}

// 2026-04-02 CST：这里定义研究标签行，原因是第一版相似信号检索先走“标签重合 + 结构分组”的可解释路径；
// 目的：避免一开始就引入黑箱相似度算法，同时为后续 analog summary 提供稳定索引。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecuritySignalTagRow {
    pub symbol: String,
    pub snapshot_date: String,
    pub tag_key: String,
    pub tag_value: String,
}

// 2026-04-02 CST：这里定义历史相似信号研究摘要行，原因是投决层需要消费聚合后的样本统计，而不是自己再扫明细；
// 目的：把 sample_count、win_rate、收益统计沉成正式研究结果，供 briefing 和 committee payload 复用。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecuritySignalAnalogStudyRow {
    pub symbol: String,
    pub snapshot_date: String,
    pub study_key: String,
    pub sample_count: i64,
    pub win_rate: f64,
    pub avg_return_pct: f64,
    pub median_return_pct: f64,
    pub summary_payload: String,
}

// 2026-04-02 CST：这里定义信号结果研究平台 SQLite Store，原因是方案C要求把研究层与行情层、共振层分开治理；
// 目的：让快照、future outcomes、tags 和 analog studies 进入单独 runtime 库，保持职责清晰。
#[derive(Debug, Clone)]
pub struct SignalOutcomeStore {
    db_path: PathBuf,
}

// 2026-04-02 CST：这里集中定义研究平台存储层错误，原因是 schema 初始化与后续读写都会有不同失败路径；
// 目的：为上层 Tool 提供清晰中文错误，而不是直接暴露 SQLite 原始异常。
#[derive(Debug, Error)]
pub enum SignalOutcomeStoreError {
    #[error("无法确定信号结果研究 SQLite 所在目录: {0}")]
    ResolveRuntimeDir(String),
    #[error("无法创建信号结果研究 SQLite 目录: {0}")]
    CreateRuntimeDir(String),
    #[error("无法打开信号结果研究 SQLite: {0}")]
    OpenDatabase(String),
    #[error("无法初始化信号结果研究表结构: {0}")]
    BootstrapSchema(String),
    #[error("无法写入信号快照: {0}")]
    WriteSnapshot(String),
    #[error("无法读取信号快照: {0}")]
    ReadSnapshot(String),
    #[error("无法写入未来收益回填结果: {0}")]
    WriteForwardReturns(String),
    #[error("无法读取未来收益回填结果: {0}")]
    ReadForwardReturns(String),
    #[error("无法写入信号标签: {0}")]
    WriteTags(String),
    #[error("无法读取信号标签: {0}")]
    ReadTags(String),
    #[error("无法写入历史相似研究摘要: {0}")]
    WriteAnalogStudies(String),
    #[error("无法读取历史相似研究摘要: {0}")]
    ReadAnalogStudies(String),
}

impl SignalOutcomeStore {
    // 2026-04-02 CST：这里允许显式指定研究库路径，原因是测试隔离和后续批量研究都可能需要单独落库位置；
    // 目的：保留同一套逻辑在不同 runtime 目录下复用的扩展点。
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    // 2026-04-02 CST：这里提供默认研究库路径，原因是第一版需要沿现有 workspace runtime 自动推导；
    // 目的：让研究层和其他 stock runtime 一样能被 CLI、测试与后续 GUI 统一发现。
    pub fn workspace_default() -> Result<Self, SignalOutcomeStoreError> {
        Ok(Self::new(
            FormalSecurityRuntimeRegistry::signal_outcome_db_path()
                .map_err(SignalOutcomeStoreError::ResolveRuntimeDir)?,
        ))
    }

    // 2026-04-02 CST：这里暴露研究库路径，原因是测试和后续排障都需要确认是否已经真正落盘；
    // 目的：让上层可以直接核对 runtime 文件位置，而不是继续盲猜数据库路径。
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    // 2026-04-02 CST：这里先提供最小连接入口，原因是 Task 1 只需要保证 schema 可以独立初始化；
    // 目的：先把运行时库建立好，后续任务再逐步补快照、回填和研究读写逻辑。
    pub fn open_connection(&self) -> Result<Connection, SignalOutcomeStoreError> {
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| SignalOutcomeStoreError::CreateRuntimeDir(error.to_string()))?;
        }

        let connection = Connection::open(&self.db_path)
            .map_err(|error| SignalOutcomeStoreError::OpenDatabase(error.to_string()))?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|error| SignalOutcomeStoreError::OpenDatabase(error.to_string()))?;
        bootstrap_schema(&connection)?;
        Ok(connection)
    }

    // 2026-04-02 CST: 这里先补最小 signal snapshot upsert，原因是方案C第二步要求把“当下整套指标状态”沉淀到研究库，
    // 目的：让后续 forward returns、analog study 和 briefing 都围绕同一条 snapshot 主键继续扩展，而不是各算各的。
    pub fn upsert_snapshot(
        &self,
        row: &SecuritySignalSnapshotRow,
    ) -> Result<(), SignalOutcomeStoreError> {
        let connection = self.open_connection()?;
        connection
            .execute(
                "INSERT INTO security_signal_snapshots (
                    symbol,
                    snapshot_date,
                    indicator_digest,
                    resonance_score,
                    action_bias,
                    snapshot_payload
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(symbol, snapshot_date) DO UPDATE SET
                    indicator_digest = excluded.indicator_digest,
                    resonance_score = excluded.resonance_score,
                    action_bias = excluded.action_bias,
                    snapshot_payload = excluded.snapshot_payload,
                    updated_at = CURRENT_TIMESTAMP",
                params![
                    row.symbol,
                    row.snapshot_date,
                    row.indicator_digest,
                    row.resonance_score,
                    row.action_bias,
                    row.snapshot_payload,
                ],
            )
            .map_err(|error| SignalOutcomeStoreError::WriteSnapshot(error.to_string()))?;
        Ok(())
    }

    // 2026-04-02 CST: 这里补按 symbol/snapshot_date 读取快照，原因是 forward returns 回填要围绕已落库的 snapshot 主键继续工作，
    // 目的：确保研究平台后续各层都是“先读快照，再扩结果”，而不是重新计算一遍当日状态。
    pub fn load_snapshot(
        &self,
        symbol: &str,
        snapshot_date: &str,
    ) -> Result<Option<SecuritySignalSnapshotRow>, SignalOutcomeStoreError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT symbol, snapshot_date, indicator_digest, resonance_score, action_bias, snapshot_payload
                 FROM security_signal_snapshots
                 WHERE symbol = ?1 AND snapshot_date = ?2
                 LIMIT 1",
            )
            .map_err(|error| SignalOutcomeStoreError::ReadSnapshot(error.to_string()))?;
        let mut rows = statement
            .query(params![symbol, snapshot_date])
            .map_err(|error| SignalOutcomeStoreError::ReadSnapshot(error.to_string()))?;

        let Some(row) = rows
            .next()
            .map_err(|error| SignalOutcomeStoreError::ReadSnapshot(error.to_string()))?
        else {
            return Ok(None);
        };

        Ok(Some(SecuritySignalSnapshotRow {
            symbol: row
                .get(0)
                .map_err(|error| SignalOutcomeStoreError::ReadSnapshot(error.to_string()))?,
            snapshot_date: row
                .get(1)
                .map_err(|error| SignalOutcomeStoreError::ReadSnapshot(error.to_string()))?,
            indicator_digest: row
                .get(2)
                .map_err(|error| SignalOutcomeStoreError::ReadSnapshot(error.to_string()))?,
            resonance_score: row
                .get(3)
                .map_err(|error| SignalOutcomeStoreError::ReadSnapshot(error.to_string()))?,
            action_bias: row
                .get(4)
                .map_err(|error| SignalOutcomeStoreError::ReadSnapshot(error.to_string()))?,
            snapshot_payload: row
                .get(5)
                .map_err(|error| SignalOutcomeStoreError::ReadSnapshot(error.to_string()))?,
        }))
    }

    // 2026-04-02 CST: 这里补最新快照日期查询，原因是 backfill Tool 第一版允许不显式传 snapshot_date，
    // 目的：让上层可以先按 symbol 触发默认回填，再逐步扩成批量/筛选模式。
    pub fn latest_snapshot_date(
        &self,
        symbol: &str,
    ) -> Result<Option<String>, SignalOutcomeStoreError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT snapshot_date
                 FROM security_signal_snapshots
                 WHERE symbol = ?1
                 ORDER BY snapshot_date DESC
                 LIMIT 1",
            )
            .map_err(|error| SignalOutcomeStoreError::ReadSnapshot(error.to_string()))?;
        let mut rows = statement
            .query(params![symbol])
            .map_err(|error| SignalOutcomeStoreError::ReadSnapshot(error.to_string()))?;

        let Some(row) = rows
            .next()
            .map_err(|error| SignalOutcomeStoreError::ReadSnapshot(error.to_string()))?
        else {
            return Ok(None);
        };

        row.get(0)
            .map(Some)
            .map_err(|error| SignalOutcomeStoreError::ReadSnapshot(error.to_string()))
    }

    // 2026-04-02 CST: 这里补 forward returns 批量 upsert，原因是研究平台第二步要把固定 horizons 的收益/回撤/上冲写成正式研究资产，
    // 目的：让后续 analog study 和 briefing 能直接消费持久化结果，而不是每次重新扫描未来窗口。
    pub fn replace_forward_returns(
        &self,
        symbol: &str,
        snapshot_date: &str,
        rows: &[SecuritySignalForwardReturnRow],
    ) -> Result<(), SignalOutcomeStoreError> {
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| SignalOutcomeStoreError::WriteForwardReturns(error.to_string()))?;
        transaction
            .execute(
                "DELETE FROM security_signal_forward_returns
                 WHERE symbol = ?1 AND snapshot_date = ?2",
                params![symbol, snapshot_date],
            )
            .map_err(|error| SignalOutcomeStoreError::WriteForwardReturns(error.to_string()))?;

        for row in rows {
            transaction
                .execute(
                    "INSERT INTO security_signal_forward_returns (
                        symbol,
                        snapshot_date,
                        horizon_days,
                        forward_return_pct,
                        max_drawdown_pct,
                        max_runup_pct
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        row.symbol,
                        row.snapshot_date,
                        row.horizon_days,
                        row.forward_return_pct,
                        row.max_drawdown_pct,
                        row.max_runup_pct,
                    ],
                )
                .map_err(|error| SignalOutcomeStoreError::WriteForwardReturns(error.to_string()))?;
        }

        transaction
            .commit()
            .map_err(|error| SignalOutcomeStoreError::WriteForwardReturns(error.to_string()))?;
        Ok(())
    }

    // 2026-04-02 CST: 这里补充 signal tags 的整批替换写入，原因是 analog study 第一版要先走“标签重合 + 核心数值接近”
    // 的可解释路线，而不是直接引入黑箱相似度索引；目的：让 sector_template、action_bias、MACD/RSRS 状态等快照标签
    // 与 snapshot 主键稳定绑定，供后续研究层和 briefing 层统一复用。
    pub fn replace_tags(
        &self,
        symbol: &str,
        snapshot_date: &str,
        rows: &[SecuritySignalTagRow],
    ) -> Result<(), SignalOutcomeStoreError> {
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| SignalOutcomeStoreError::WriteTags(error.to_string()))?;
        transaction
            .execute(
                "DELETE FROM security_signal_tags
                 WHERE symbol = ?1 AND snapshot_date = ?2",
                params![symbol, snapshot_date],
            )
            .map_err(|error| SignalOutcomeStoreError::WriteTags(error.to_string()))?;

        for row in rows {
            transaction
                .execute(
                    "INSERT INTO security_signal_tags (
                        symbol,
                        snapshot_date,
                        tag_key,
                        tag_value
                    ) VALUES (?1, ?2, ?3, ?4)",
                    params![row.symbol, row.snapshot_date, row.tag_key, row.tag_value],
                )
                .map_err(|error| SignalOutcomeStoreError::WriteTags(error.to_string()))?;
        }

        transaction
            .commit()
            .map_err(|error| SignalOutcomeStoreError::WriteTags(error.to_string()))?;
        Ok(())
    }

    // 2026-04-02 CST: 这里补充按快照主键读取标签集合，原因是 analog study 需要先比较“行业/共振/技术状态标签”
    // 是否同类，再叠加核心数值相近度；目的：让研究层直接消费持久化标签，不重复从 payload 临时推导。
    pub fn load_tags(
        &self,
        symbol: &str,
        snapshot_date: &str,
    ) -> Result<Vec<SecuritySignalTagRow>, SignalOutcomeStoreError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT symbol, snapshot_date, tag_key, tag_value
                 FROM security_signal_tags
                 WHERE symbol = ?1 AND snapshot_date = ?2
                 ORDER BY tag_key ASC",
            )
            .map_err(|error| SignalOutcomeStoreError::ReadTags(error.to_string()))?;
        let mapped_rows = statement
            .query_map(params![symbol, snapshot_date], |row| {
                Ok(SecuritySignalTagRow {
                    symbol: row.get(0)?,
                    snapshot_date: row.get(1)?,
                    tag_key: row.get(2)?,
                    tag_value: row.get(3)?,
                })
            })
            .map_err(|error| SignalOutcomeStoreError::ReadTags(error.to_string()))?;

        let mut rows = Vec::new();
        for row in mapped_rows {
            rows.push(row.map_err(|error| SignalOutcomeStoreError::ReadTags(error.to_string()))?);
        }
        Ok(rows)
    }

    // 2026-04-02 CST: 这里补充按 symbol 批量扫描目标快照日前的历史 snapshot，原因是用户明确要求不能只看单只中信银行，
    // 而要在银行体系内寻找相似状态；目的：把“候选样本集合”读取职责收口在 store，避免上层研究逻辑散写 SQL。
    pub fn load_snapshots_for_symbols_before(
        &self,
        symbols: &[String],
        before_date: &str,
        per_symbol_limit: usize,
    ) -> Result<Vec<SecuritySignalSnapshotRow>, SignalOutcomeStoreError> {
        let connection = self.open_connection()?;
        let mut rows = Vec::new();

        for symbol in symbols {
            let mut statement = connection
                .prepare(
                    "SELECT symbol, snapshot_date, indicator_digest, resonance_score, action_bias, snapshot_payload
                     FROM security_signal_snapshots
                     WHERE symbol = ?1 AND snapshot_date < ?2
                     ORDER BY snapshot_date DESC
                     LIMIT ?3",
                )
                .map_err(|error| SignalOutcomeStoreError::ReadSnapshot(error.to_string()))?;
            let mapped_rows = statement
                .query_map(
                    params![symbol, before_date, per_symbol_limit as i64],
                    |row| {
                        Ok(SecuritySignalSnapshotRow {
                            symbol: row.get(0)?,
                            snapshot_date: row.get(1)?,
                            indicator_digest: row.get(2)?,
                            resonance_score: row.get(3)?,
                            action_bias: row.get(4)?,
                            snapshot_payload: row.get(5)?,
                        })
                    },
                )
                .map_err(|error| SignalOutcomeStoreError::ReadSnapshot(error.to_string()))?;

            for row in mapped_rows {
                rows.push(
                    row.map_err(|error| SignalOutcomeStoreError::ReadSnapshot(error.to_string()))?,
                );
            }
        }

        rows.sort_by(|left, right| {
            left.snapshot_date
                .cmp(&right.snapshot_date)
                .then(left.symbol.cmp(&right.symbol))
        });
        Ok(rows)
    }

    // 2026-04-02 CST: 这里补充读取单个快照的 forward returns 集合，原因是 analog study 需要直接消费
    // 10/20 日收益、回撤与上冲结果；目的：让 briefing 和研究摘要共享同一份持久化 future outcomes。
    pub fn load_forward_returns(
        &self,
        symbol: &str,
        snapshot_date: &str,
    ) -> Result<Vec<SecuritySignalForwardReturnRow>, SignalOutcomeStoreError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT symbol, snapshot_date, horizon_days, forward_return_pct, max_drawdown_pct, max_runup_pct
                 FROM security_signal_forward_returns
                 WHERE symbol = ?1 AND snapshot_date = ?2
                 ORDER BY horizon_days ASC",
            )
            .map_err(|error| SignalOutcomeStoreError::ReadForwardReturns(error.to_string()))?;
        let mapped_rows = statement
            .query_map(params![symbol, snapshot_date], |row| {
                Ok(SecuritySignalForwardReturnRow {
                    symbol: row.get(0)?,
                    snapshot_date: row.get(1)?,
                    horizon_days: row.get(2)?,
                    forward_return_pct: row.get(3)?,
                    max_drawdown_pct: row.get(4)?,
                    max_runup_pct: row.get(5)?,
                })
            })
            .map_err(|error| SignalOutcomeStoreError::ReadForwardReturns(error.to_string()))?;

        let mut rows = Vec::new();
        for row in mapped_rows {
            rows.push(
                row.map_err(|error| {
                    SignalOutcomeStoreError::ReadForwardReturns(error.to_string())
                })?,
            );
        }
        Ok(rows)
    }

    // 2026-04-02 CST: 这里补充 analog study 摘要 upsert，原因是历史相似研究要沉成正式研究资产，供 briefing/committee 复用；
    // 目的：避免每次生成报告都重新扫描全量候选，确保咨询与投决读取同一份历史摘要。
    pub fn upsert_analog_study(
        &self,
        row: &SecuritySignalAnalogStudyRow,
    ) -> Result<(), SignalOutcomeStoreError> {
        let connection = self.open_connection()?;
        connection
            .execute(
                "INSERT INTO security_signal_analog_studies (
                    symbol,
                    snapshot_date,
                    study_key,
                    sample_count,
                    win_rate,
                    avg_return_pct,
                    median_return_pct,
                    summary_payload
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ON CONFLICT(symbol, snapshot_date, study_key) DO UPDATE SET
                    sample_count = excluded.sample_count,
                    win_rate = excluded.win_rate,
                    avg_return_pct = excluded.avg_return_pct,
                    median_return_pct = excluded.median_return_pct,
                    summary_payload = excluded.summary_payload,
                    updated_at = CURRENT_TIMESTAMP",
                params![
                    row.symbol,
                    row.snapshot_date,
                    row.study_key,
                    row.sample_count,
                    row.win_rate,
                    row.avg_return_pct,
                    row.median_return_pct,
                    row.summary_payload,
                ],
            )
            .map_err(|error| SignalOutcomeStoreError::WriteAnalogStudies(error.to_string()))?;
        Ok(())
    }

    // 2026-04-02 CST: 这里补充读取指定 study_key 的 analog study 摘要，原因是 security_decision_briefing
    // 需要在不重扫候选样本的前提下，把历史相似结果接回 committee payload；目的：把研究层与 briefing 层解耦。
    pub fn load_analog_study(
        &self,
        symbol: &str,
        snapshot_date: &str,
        study_key: &str,
    ) -> Result<Option<SecuritySignalAnalogStudyRow>, SignalOutcomeStoreError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT symbol, snapshot_date, study_key, sample_count, win_rate, avg_return_pct, median_return_pct, summary_payload
                 FROM security_signal_analog_studies
                 WHERE symbol = ?1 AND snapshot_date = ?2 AND study_key = ?3
                 LIMIT 1",
            )
            .map_err(|error| SignalOutcomeStoreError::ReadAnalogStudies(error.to_string()))?;
        let mut rows = statement
            .query(params![symbol, snapshot_date, study_key])
            .map_err(|error| SignalOutcomeStoreError::ReadAnalogStudies(error.to_string()))?;

        let Some(row) = rows
            .next()
            .map_err(|error| SignalOutcomeStoreError::ReadAnalogStudies(error.to_string()))?
        else {
            return Ok(None);
        };

        Ok(Some(SecuritySignalAnalogStudyRow {
            symbol: row
                .get(0)
                .map_err(|error| SignalOutcomeStoreError::ReadAnalogStudies(error.to_string()))?,
            snapshot_date: row
                .get(1)
                .map_err(|error| SignalOutcomeStoreError::ReadAnalogStudies(error.to_string()))?,
            study_key: row
                .get(2)
                .map_err(|error| SignalOutcomeStoreError::ReadAnalogStudies(error.to_string()))?,
            sample_count: row
                .get(3)
                .map_err(|error| SignalOutcomeStoreError::ReadAnalogStudies(error.to_string()))?,
            win_rate: row
                .get(4)
                .map_err(|error| SignalOutcomeStoreError::ReadAnalogStudies(error.to_string()))?,
            avg_return_pct: row
                .get(5)
                .map_err(|error| SignalOutcomeStoreError::ReadAnalogStudies(error.to_string()))?,
            median_return_pct: row
                .get(6)
                .map_err(|error| SignalOutcomeStoreError::ReadAnalogStudies(error.to_string()))?,
            summary_payload: row
                .get(7)
                .map_err(|error| SignalOutcomeStoreError::ReadAnalogStudies(error.to_string()))?,
        }))
    }
}

fn bootstrap_schema(connection: &Connection) -> Result<(), SignalOutcomeStoreError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS security_signal_snapshots (
                symbol TEXT NOT NULL,
                snapshot_date TEXT NOT NULL,
                indicator_digest TEXT NOT NULL,
                resonance_score REAL NOT NULL,
                action_bias TEXT NOT NULL,
                snapshot_payload TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY(symbol, snapshot_date)
            );
            CREATE TABLE IF NOT EXISTS security_signal_forward_returns (
                symbol TEXT NOT NULL,
                snapshot_date TEXT NOT NULL,
                horizon_days INTEGER NOT NULL,
                forward_return_pct REAL NOT NULL,
                max_drawdown_pct REAL NOT NULL,
                max_runup_pct REAL NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY(symbol, snapshot_date, horizon_days)
            );
            CREATE TABLE IF NOT EXISTS security_signal_tags (
                symbol TEXT NOT NULL,
                snapshot_date TEXT NOT NULL,
                tag_key TEXT NOT NULL,
                tag_value TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY(symbol, snapshot_date, tag_key)
            );
            CREATE TABLE IF NOT EXISTS security_signal_analog_studies (
                symbol TEXT NOT NULL,
                snapshot_date TEXT NOT NULL,
                study_key TEXT NOT NULL,
                sample_count INTEGER NOT NULL,
                win_rate REAL NOT NULL,
                avg_return_pct REAL NOT NULL,
                median_return_pct REAL NOT NULL,
                summary_payload TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY(symbol, snapshot_date, study_key)
            );",
        )
        .map_err(|error| SignalOutcomeStoreError::BootstrapSchema(error.to_string()))?;
    ensure_column_exists(
        connection,
        "security_signal_analog_studies",
        "summary_payload",
        "TEXT NOT NULL DEFAULT '{}'",
    )?;
    Ok(())
}

// 2026-04-02 CST: 这里补一个轻量 schema 迁移助手，原因是 signal_outcome_research.db 可能已经被上一轮运行创建，
// 如果直接修改 CREATE TABLE 并不能自动补旧库列；目的：保证新增历史摘要载荷字段可以平滑落到已存在的 runtime 库中。
fn ensure_column_exists(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
    column_definition: &str,
) -> Result<(), SignalOutcomeStoreError> {
    let pragma_sql = format!("PRAGMA table_info({table_name})");
    let mut statement = connection
        .prepare(&pragma_sql)
        .map_err(|error| SignalOutcomeStoreError::BootstrapSchema(error.to_string()))?;
    let mapped_rows = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| SignalOutcomeStoreError::BootstrapSchema(error.to_string()))?;
    let mut existing_columns = Vec::new();
    for row in mapped_rows {
        existing_columns.push(
            row.map_err(|error| SignalOutcomeStoreError::BootstrapSchema(error.to_string()))?,
        );
    }
    if existing_columns
        .iter()
        .any(|existing| existing == column_name)
    {
        return Ok(());
    }

    let alter_sql =
        format!("ALTER TABLE {table_name} ADD COLUMN {column_name} {column_definition}");
    connection
        .execute(&alter_sql, [])
        .map_err(|error| SignalOutcomeStoreError::BootstrapSchema(error.to_string()))?;
    Ok(())
}
