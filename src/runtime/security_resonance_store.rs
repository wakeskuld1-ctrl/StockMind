use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, params};
use thiserror::Error;

use crate::runtime::formal_security_runtime_registry::FormalSecurityRuntimeRegistry;

// 2026-04-02 CST：这里定义共振因子注册项，原因是方案 3 已确认先做“可注册、可扩展、可评估”的平台底层；
// 目的：把行业模板候选因子和后续新增想法统一沉淀成正式数据资产，而不是继续散落在代码常量里。
#[derive(Debug, Clone, PartialEq)]
pub struct ResonanceFactorDefinition {
    pub factor_key: String,
    pub display_name: String,
    pub market_regime: String,
    pub template_key: String,
    pub factor_type: String,
    pub source_kind: String,
    pub expected_relation: String,
    pub source_symbol: Option<String>,
    pub enabled: bool,
    pub notes: Option<String>,
}

// 2026-04-02 CST：这里定义因子日度序列点，原因是平台第一版需要把商品、运价、汇率、利率等统一落成可计算序列；
// 目的：为后续滚动相关、beta、领先滞后和稳定性评估提供统一输入结构。
#[derive(Debug, Clone, PartialEq)]
pub struct ResonanceFactorPoint {
    pub trade_date: String,
    pub value: f64,
}

// 2026-04-02 CST：这里定义事件标签，原因是用户已明确要求“事件标签也纳入第一版平台”；
// 目的：让地缘、政策、运输瓶颈等非纯价格驱动也能进入正式存储和后续评估链路。
#[derive(Debug, Clone, PartialEq)]
pub struct ResonanceEventTag {
    pub event_key: String,
    pub event_date: String,
    pub title: String,
    pub market_regime: String,
    pub template_key: String,
    pub symbol_scope: Option<String>,
    pub polarity: String,
    pub strength: f64,
    pub notes: Option<String>,
}

// 2026-04-02 CST：这里定义共振快照行，原因是平台不能只临时算结论，还要把“当日最强驱动”写回库里供后续评估；
// 目的：把每只证券在某个分析时点的因子关系沉淀成可复盘、可排序、可比对的研究记录。
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityResonanceSnapshotRow {
    pub symbol: String,
    pub snapshot_date: String,
    pub factor_key: String,
    pub display_name: String,
    pub relation_kind: String,
    pub expected_relation: String,
    pub correlation: f64,
    pub beta: f64,
    pub direction_alignment: f64,
    pub stability_score: f64,
    pub lag_days: i32,
    pub divergence_score: f64,
    pub resonance_score: f64,
    pub driver_side: String,
}

// 2026-04-02 CST：这里定义共振平台 SQLite Store，原因是用户要求“算出来以后写到数据库里，后边再把相关性强的拉出来评估”；
// 目的：把共振平台和现有 runtime 根目录接起来，同时保持股票历史库与共振研究库职责分离。
#[derive(Debug, Clone)]
pub struct SecurityResonanceStore {
    db_path: PathBuf,
}

// 2026-04-02 CST：这里集中定义共振平台存储层错误，原因是平台注册、写库、读库和快照替换都会产生不同失败路径；
// 目的：让上层 Tool 能拿到清晰、可定位的中文错误，而不是只看到笼统的 SQLite 异常。
#[derive(Debug, Error)]
pub enum SecurityResonanceStoreError {
    #[error("无法确定共振平台 SQLite 所在目录: {0}")]
    ResolveRuntimeDir(String),
    #[error("无法创建共振平台 SQLite 目录: {0}")]
    CreateRuntimeDir(String),
    #[error("无法打开共振平台 SQLite: {0}")]
    OpenDatabase(String),
    #[error("无法初始化共振平台表结构: {0}")]
    BootstrapSchema(String),
    #[error("无法写入共振因子定义: {0}")]
    WriteFactorRegistry(String),
    #[error("无法写入共振因子序列: {0}")]
    WriteFactorSeries(String),
    #[error("无法写入共振事件标签: {0}")]
    WriteEventTags(String),
    #[error("无法写入共振快照: {0}")]
    WriteSnapshots(String),
    #[error("无法读取共振因子定义: {0}")]
    ReadFactorRegistry(String),
    #[error("无法读取共振因子序列: {0}")]
    ReadFactorSeries(String),
    #[error("无法读取共振事件标签: {0}")]
    ReadEventTags(String),
}

impl SecurityResonanceStore {
    // 2026-04-02 CST：这里允许显式指定共振库路径，原因是测试隔离和后续批量研究都可能需要单独落盘位置；
    // 目的：保留“同一逻辑，不同 runtime 根目录”的扩展点，同时不强绑死固定路径。
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    // 2026-04-02 CST：这里提供默认共振库路径，原因是平台第一版要沿现有 runtime 根目录统一落盘；
    // 目的：让 `EXCEL_SKILL_RUNTIME_DB` 和 `EXCEL_SKILL_RUNTIME_DIR` 可以自动推导出共振平台数据库位置。
    pub fn workspace_default() -> Result<Self, SecurityResonanceStoreError> {
        Ok(Self::new(
            FormalSecurityRuntimeRegistry::resonance_db_path()
                .map_err(SecurityResonanceStoreError::ResolveRuntimeDir)?,
        ))
    }

    // 2026-04-02 CST：这里暴露数据库路径，原因是测试和后续研究工具需要直接确认共振库是否真正落盘；
    // 目的：避免上层只能盲猜 SQLite 文件位置，提升排查和验证效率。
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    // 2026-04-02 CST：这里提供因子注册 upsert，原因是“新想法先注册因子，再评估”是平台化演进的第一步；
    // 目的：让行业模板因子与个股特例因子都能统一进入 registry，而不是继续走硬编码。
    pub fn upsert_factor_definition(
        &self,
        definition: &ResonanceFactorDefinition,
    ) -> Result<(), SecurityResonanceStoreError> {
        let connection = self.open_connection()?;
        connection
            .execute(
                "INSERT INTO resonance_factor_registry (
                    factor_key,
                    display_name,
                    market_regime,
                    template_key,
                    factor_type,
                    source_kind,
                    expected_relation,
                    source_symbol,
                    enabled,
                    notes
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                ON CONFLICT(factor_key) DO UPDATE SET
                    display_name = excluded.display_name,
                    market_regime = excluded.market_regime,
                    template_key = excluded.template_key,
                    factor_type = excluded.factor_type,
                    source_kind = excluded.source_kind,
                    expected_relation = excluded.expected_relation,
                    source_symbol = excluded.source_symbol,
                    enabled = excluded.enabled,
                    notes = excluded.notes,
                    updated_at = CURRENT_TIMESTAMP",
                params![
                    definition.factor_key,
                    definition.display_name,
                    definition.market_regime,
                    definition.template_key,
                    definition.factor_type,
                    definition.source_kind,
                    definition.expected_relation,
                    definition.source_symbol,
                    bool_to_sqlite_int(definition.enabled),
                    definition.notes,
                ],
            )
            .map_err(|error| SecurityResonanceStoreError::WriteFactorRegistry(error.to_string()))?;
        Ok(())
    }

    // 2026-04-02 CST：这里批量写入因子序列，原因是平台后续会持续追加价格、运价、汇率等外部因子；
    // 目的：在落原始值的同时顺手沉淀标准化值和日收益，给后续评估层减少重复计算成本。
    pub fn upsert_factor_series(
        &self,
        factor_key: &str,
        source: &str,
        points: &[ResonanceFactorPoint],
    ) -> Result<usize, SecurityResonanceStoreError> {
        let mut sorted_points = points.to_vec();
        sorted_points.sort_by(|left, right| left.trade_date.cmp(&right.trade_date));

        let values = sorted_points
            .iter()
            .map(|point| point.value)
            .collect::<Vec<_>>();
        let mean = arithmetic_mean(&values);
        let std_dev = standard_deviation(&values, mean);

        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| SecurityResonanceStoreError::WriteFactorSeries(error.to_string()))?;

        for (index, point) in sorted_points.iter().enumerate() {
            let normalized_value = if std_dev > f64::EPSILON {
                (point.value - mean) / std_dev
            } else {
                0.0
            };
            let daily_return = if index == 0 {
                0.0
            } else {
                let previous_value = sorted_points[index - 1].value;
                if previous_value.abs() > f64::EPSILON {
                    (point.value - previous_value) / previous_value
                } else {
                    0.0
                }
            };

            transaction
                .execute(
                    "INSERT INTO resonance_factor_series (
                        factor_key,
                        trade_date,
                        value,
                        normalized_value,
                        daily_return,
                        source
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                    ON CONFLICT(factor_key, trade_date) DO UPDATE SET
                        value = excluded.value,
                        normalized_value = excluded.normalized_value,
                        daily_return = excluded.daily_return,
                        source = excluded.source,
                        updated_at = CURRENT_TIMESTAMP",
                    params![
                        factor_key,
                        point.trade_date,
                        point.value,
                        normalized_value,
                        daily_return,
                        source,
                    ],
                )
                .map_err(|error| {
                    SecurityResonanceStoreError::WriteFactorSeries(error.to_string())
                })?;
        }

        transaction
            .commit()
            .map_err(|error| SecurityResonanceStoreError::WriteFactorSeries(error.to_string()))?;

        Ok(sorted_points.len())
    }

    // 2026-04-02 CST：这里批量写入事件标签，原因是事件因子已经被纳入第一版平台而不是后补；
    // 目的：让地缘、政策、运输风险等标签拥有和价格因子一样的正式落库入口。
    pub fn upsert_event_tags(
        &self,
        tags: &[ResonanceEventTag],
    ) -> Result<usize, SecurityResonanceStoreError> {
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| SecurityResonanceStoreError::WriteEventTags(error.to_string()))?;

        for tag in tags {
            transaction
                .execute(
                    "INSERT INTO resonance_event_tags (
                        event_key,
                        event_date,
                        title,
                        market_regime,
                        template_key,
                        symbol_scope,
                        polarity,
                        strength,
                        notes
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                    ON CONFLICT(event_key, event_date, symbol_scope) DO UPDATE SET
                        title = excluded.title,
                        market_regime = excluded.market_regime,
                        template_key = excluded.template_key,
                        polarity = excluded.polarity,
                        strength = excluded.strength,
                        notes = excluded.notes,
                        updated_at = CURRENT_TIMESTAMP",
                    params![
                        tag.event_key,
                        tag.event_date,
                        tag.title,
                        tag.market_regime,
                        tag.template_key,
                        tag.symbol_scope.clone().unwrap_or_default(),
                        tag.polarity,
                        tag.strength,
                        tag.notes,
                    ],
                )
                .map_err(|error| SecurityResonanceStoreError::WriteEventTags(error.to_string()))?;
        }

        transaction
            .commit()
            .map_err(|error| SecurityResonanceStoreError::WriteEventTags(error.to_string()))?;
        Ok(tags.len())
    }

    // 2026-04-02 CST：这里按市场和行业模板读取启用因子，原因是平台要先从候选因子池筛出当前分析场景可用的研究对象；
    // 目的：让“行业模板 + 个股补充因子”都能通过统一 registry 管理，而不是散落在上层逻辑里。
    pub fn list_factors(
        &self,
        market_regime: &str,
        template_key: &str,
    ) -> Result<Vec<ResonanceFactorDefinition>, SecurityResonanceStoreError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT
                    factor_key,
                    display_name,
                    market_regime,
                    template_key,
                    factor_type,
                    source_kind,
                    expected_relation,
                    source_symbol,
                    enabled,
                    notes
                 FROM resonance_factor_registry
                 WHERE enabled = 1
                   AND market_regime = ?1
                   AND template_key = ?2
                 ORDER BY factor_key",
            )
            .map_err(|error| SecurityResonanceStoreError::ReadFactorRegistry(error.to_string()))?;

        let mapped = statement
            .query_map(params![market_regime, template_key], |row| {
                let enabled: i64 = row.get(8)?;
                Ok(ResonanceFactorDefinition {
                    factor_key: row.get(0)?,
                    display_name: row.get(1)?,
                    market_regime: row.get(2)?,
                    template_key: row.get(3)?,
                    factor_type: row.get(4)?,
                    source_kind: row.get(5)?,
                    expected_relation: row.get(6)?,
                    source_symbol: row.get(7)?,
                    enabled: enabled == 1,
                    notes: row.get(9)?,
                })
            })
            .map_err(|error| SecurityResonanceStoreError::ReadFactorRegistry(error.to_string()))?;

        let mut results = Vec::new();
        for row in mapped {
            results.push(row.map_err(|error| {
                SecurityResonanceStoreError::ReadFactorRegistry(error.to_string())
            })?);
        }
        Ok(results)
    }

    // 2026-04-02 CST：这里读取最近因子序列，原因是滚动相关和稳定性评估只需要近期窗口数据；
    // 目的：保持平台评估路径和现有股票历史读取方式一致，避免上层重复拼 SQL。
    pub fn load_factor_series_recent(
        &self,
        factor_key: &str,
        as_of_date: Option<&str>,
        lookback_days: usize,
    ) -> Result<Vec<ResonanceFactorPoint>, SecurityResonanceStoreError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT trade_date, value
                 FROM resonance_factor_series
                 WHERE factor_key = ?1
                   AND (?2 IS NULL OR trade_date <= ?2)
                 ORDER BY trade_date DESC
                 LIMIT ?3",
            )
            .map_err(|error| SecurityResonanceStoreError::ReadFactorSeries(error.to_string()))?;

        let mapped = statement
            .query_map(
                params![factor_key, as_of_date, lookback_days as i64],
                |row| {
                    Ok(ResonanceFactorPoint {
                        trade_date: row.get(0)?,
                        value: row.get(1)?,
                    })
                },
            )
            .map_err(|error| SecurityResonanceStoreError::ReadFactorSeries(error.to_string()))?;

        let mut points = Vec::new();
        for row in mapped {
            points.push(row.map_err(|error| {
                SecurityResonanceStoreError::ReadFactorSeries(error.to_string())
            })?);
        }
        points.reverse();
        Ok(points)
    }

    // 2026-04-02 CST：这里读取最近事件标签，原因是平台分析要把近期有效事件一起带进结果和评分；
    // 目的：让事件读取规则也集中在存储层，避免上层每次手工拼不同过滤条件。
    pub fn load_event_tags_recent(
        &self,
        market_regime: &str,
        template_key: &str,
        symbol_scope: Option<&str>,
        as_of_date: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ResonanceEventTag>, SecurityResonanceStoreError> {
        let connection = self.open_connection()?;
        let scope_key = symbol_scope.unwrap_or_default();
        let mut statement = connection
            .prepare(
                "SELECT
                    event_key,
                    event_date,
                    title,
                    market_regime,
                    template_key,
                    symbol_scope,
                    polarity,
                    strength,
                    notes
                 FROM resonance_event_tags
                 WHERE market_regime = ?1
                   AND template_key = ?2
                   AND (?3 = '' OR symbol_scope = '' OR symbol_scope = ?3)
                   AND (?4 IS NULL OR event_date <= ?4)
                 ORDER BY event_date DESC, strength DESC
                 LIMIT ?5",
            )
            .map_err(|error| SecurityResonanceStoreError::ReadEventTags(error.to_string()))?;

        let mapped = statement
            .query_map(
                params![
                    market_regime,
                    template_key,
                    scope_key,
                    as_of_date,
                    limit as i64
                ],
                |row| {
                    let symbol_scope_value: String = row.get(5)?;
                    Ok(ResonanceEventTag {
                        event_key: row.get(0)?,
                        event_date: row.get(1)?,
                        title: row.get(2)?,
                        market_regime: row.get(3)?,
                        template_key: row.get(4)?,
                        symbol_scope: if symbol_scope_value.is_empty() {
                            None
                        } else {
                            Some(symbol_scope_value)
                        },
                        polarity: row.get(6)?,
                        strength: row.get(7)?,
                        notes: row.get(8)?,
                    })
                },
            )
            .map_err(|error| SecurityResonanceStoreError::ReadEventTags(error.to_string()))?;

        let mut tags = Vec::new();
        for row in mapped {
            tags.push(
                row.map_err(|error| SecurityResonanceStoreError::ReadEventTags(error.to_string()))?,
            );
        }
        Ok(tags)
    }

    // 2026-04-02 CST：这里采用“按证券 + 日期整批替换”写快照，原因是一次分析可能会重新评估同一批因子；
    // 目的：确保数据库里保留的是该时点最新的研究结果，而不是同一天多份互相冲突的旧快照。
    pub fn replace_snapshots(
        &self,
        symbol: &str,
        snapshot_date: &str,
        snapshots: &[SecurityResonanceSnapshotRow],
    ) -> Result<(), SecurityResonanceStoreError> {
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| SecurityResonanceStoreError::WriteSnapshots(error.to_string()))?;

        transaction
            .execute(
                "DELETE FROM security_resonance_snapshots WHERE symbol = ?1 AND snapshot_date = ?2",
                params![symbol, snapshot_date],
            )
            .map_err(|error| SecurityResonanceStoreError::WriteSnapshots(error.to_string()))?;

        for snapshot in snapshots {
            transaction
                .execute(
                    "INSERT INTO security_resonance_snapshots (
                        symbol,
                        snapshot_date,
                        factor_key,
                        display_name,
                        relation_kind,
                        expected_relation,
                        correlation,
                        beta,
                        direction_alignment,
                        stability_score,
                        lag_days,
                        divergence_score,
                        resonance_score,
                        driver_side
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                    params![
                        snapshot.symbol,
                        snapshot.snapshot_date,
                        snapshot.factor_key,
                        snapshot.display_name,
                        snapshot.relation_kind,
                        snapshot.expected_relation,
                        snapshot.correlation,
                        snapshot.beta,
                        snapshot.direction_alignment,
                        snapshot.stability_score,
                        snapshot.lag_days,
                        snapshot.divergence_score,
                        snapshot.resonance_score,
                        snapshot.driver_side,
                    ],
                )
                .map_err(|error| SecurityResonanceStoreError::WriteSnapshots(error.to_string()))?;
        }

        transaction
            .commit()
            .map_err(|error| SecurityResonanceStoreError::WriteSnapshots(error.to_string()))?;
        Ok(())
    }

    fn open_connection(&self) -> Result<Connection, SecurityResonanceStoreError> {
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                SecurityResonanceStoreError::CreateRuntimeDir(error.to_string())
            })?;
        }

        let connection = Connection::open(&self.db_path)
            .map_err(|error| SecurityResonanceStoreError::OpenDatabase(error.to_string()))?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|error| SecurityResonanceStoreError::OpenDatabase(error.to_string()))?;
        self.bootstrap_schema(&connection)?;
        Ok(connection)
    }

    // 2026-04-02 CST：这里初始化共振平台表结构，原因是第一版要把因子、事件和快照都正式落成研究资产；
    // 目的：先把最小但完整的平台骨架建起来，为后续持续扩因子、扩模板和回溯评估留稳定地基。
    fn bootstrap_schema(&self, connection: &Connection) -> Result<(), SecurityResonanceStoreError> {
        connection
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS resonance_factor_registry (
                    factor_key TEXT NOT NULL PRIMARY KEY,
                    display_name TEXT NOT NULL,
                    market_regime TEXT NOT NULL,
                    template_key TEXT NOT NULL,
                    factor_type TEXT NOT NULL,
                    source_kind TEXT NOT NULL,
                    expected_relation TEXT NOT NULL,
                    source_symbol TEXT,
                    enabled INTEGER NOT NULL DEFAULT 1,
                    notes TEXT,
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );

                CREATE INDEX IF NOT EXISTS idx_resonance_factor_registry_template
                ON resonance_factor_registry(market_regime, template_key, enabled);

                CREATE TABLE IF NOT EXISTS resonance_factor_series (
                    factor_key TEXT NOT NULL,
                    trade_date TEXT NOT NULL,
                    value REAL NOT NULL,
                    normalized_value REAL NOT NULL,
                    daily_return REAL NOT NULL,
                    source TEXT NOT NULL,
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    PRIMARY KEY(factor_key, trade_date)
                );

                CREATE INDEX IF NOT EXISTS idx_resonance_factor_series_lookup
                ON resonance_factor_series(factor_key, trade_date);

                CREATE TABLE IF NOT EXISTS resonance_event_tags (
                    event_key TEXT NOT NULL,
                    event_date TEXT NOT NULL,
                    title TEXT NOT NULL,
                    market_regime TEXT NOT NULL,
                    template_key TEXT NOT NULL,
                    symbol_scope TEXT NOT NULL DEFAULT '',
                    polarity TEXT NOT NULL,
                    strength REAL NOT NULL,
                    notes TEXT,
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    PRIMARY KEY(event_key, event_date, symbol_scope)
                );

                CREATE INDEX IF NOT EXISTS idx_resonance_event_tags_lookup
                ON resonance_event_tags(market_regime, template_key, symbol_scope, event_date);

                CREATE TABLE IF NOT EXISTS security_resonance_snapshots (
                    symbol TEXT NOT NULL,
                    snapshot_date TEXT NOT NULL,
                    factor_key TEXT NOT NULL,
                    display_name TEXT NOT NULL,
                    relation_kind TEXT NOT NULL,
                    expected_relation TEXT NOT NULL,
                    correlation REAL NOT NULL,
                    beta REAL NOT NULL,
                    direction_alignment REAL NOT NULL,
                    stability_score REAL NOT NULL,
                    lag_days INTEGER NOT NULL,
                    divergence_score REAL NOT NULL,
                    resonance_score REAL NOT NULL,
                    driver_side TEXT NOT NULL,
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    PRIMARY KEY(symbol, snapshot_date, factor_key)
                );

                CREATE INDEX IF NOT EXISTS idx_security_resonance_snapshots_lookup
                ON security_resonance_snapshots(symbol, snapshot_date, driver_side, resonance_score);
                ",
            )
            .map_err(|error| SecurityResonanceStoreError::BootstrapSchema(error.to_string()))?;
        Ok(())
    }
}

fn bool_to_sqlite_int(value: bool) -> i64 {
    if value { 1 } else { 0 }
}

fn arithmetic_mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn standard_deviation(values: &[f64], mean: f64) -> f64 {
    if values.len() < 2 {
        0.0
    } else {
        let variance = values
            .iter()
            .map(|value| {
                let delta = value - mean;
                delta * delta
            })
            .sum::<f64>()
            / values.len() as f64;
        variance.sqrt()
    }
}
