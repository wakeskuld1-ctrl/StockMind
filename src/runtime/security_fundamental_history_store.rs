use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;

use crate::runtime::formal_security_runtime_registry::FormalSecurityRuntimeRegistry;

// 2026-04-12 CST: Add a dedicated runtime row for stock fundamental history,
// because governed validation should replay persisted financial snapshots instead
// of depending only on one-off live fetches.
// Purpose: keep stock information history auditable and queryable by symbol/date.
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityFundamentalHistoryRecordRow {
    pub symbol: String,
    pub report_period: String,
    pub notice_date: Option<String>,
    pub source: String,
    pub report_metrics_json: String,
    pub batch_id: String,
    pub record_ref: String,
    pub created_at: String,
}

// 2026-04-12 CST: Keep the runtime store isolated from ops-layer contracts,
// because runtime should persist durable financial rows without pulling the
// product-layer context types into SQLite bootstrapping.
// Purpose: avoid circular coupling between history persistence and fullstack ops.
#[derive(Debug, Clone)]
pub struct SecurityFundamentalHistoryStore {
    db_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum SecurityFundamentalHistoryStoreError {
    #[error("failed to resolve fundamental history runtime dir: {0}")]
    ResolveRuntimeDir(String),
    #[error("failed to create fundamental history runtime dir: {0}")]
    CreateRuntimeDir(String),
    #[error("failed to open fundamental history sqlite: {0}")]
    OpenDatabase(String),
    #[error("failed to bootstrap fundamental history schema: {0}")]
    BootstrapSchema(String),
    #[error("fundamental history rows cannot be empty")]
    EmptyRows,
    #[error("failed to write fundamental history rows: {0}")]
    WriteRows(String),
    #[error("failed to read fundamental history rows: {0}")]
    ReadRows(String),
}

impl SecurityFundamentalHistoryStore {
    // 2026-04-12 CST: Keep an explicit constructor for tests and validation slices,
    // because history replay must support isolated runtime roots outside the shared
    // workspace database location.
    // Purpose: let validation tools persist stock information history per slice.
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    // 2026-04-12 CST: Reuse workspace runtime semantics, because governed stock
    // information history should follow the same runtime-root override behavior as
    // stock price history and external proxy history.
    // Purpose: make tests and isolated runtime roots predictable.
    pub fn workspace_default() -> Result<Self, SecurityFundamentalHistoryStoreError> {
        Ok(Self::new(
            FormalSecurityRuntimeRegistry::fundamental_history_db_path()
                .map_err(SecurityFundamentalHistoryStoreError::ResolveRuntimeDir)?,
        ))
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    // 2026-04-12 CST: Upsert by symbol+report period, because financial snapshots
    // should remain idempotent across reruns while still allowing a later notice-date
    // correction to replace an older payload.
    // Purpose: keep governed financial history stable across repeated imports.
    pub fn upsert_rows(
        &self,
        rows: &[SecurityFundamentalHistoryRecordRow],
    ) -> Result<(), SecurityFundamentalHistoryStoreError> {
        if rows.is_empty() {
            return Err(SecurityFundamentalHistoryStoreError::EmptyRows);
        }

        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| SecurityFundamentalHistoryStoreError::WriteRows(error.to_string()))?;

        for row in rows {
            transaction
                .execute(
                    "INSERT INTO security_fundamental_history (
                        symbol,
                        report_period,
                        notice_date,
                        source,
                        report_metrics_json,
                        batch_id,
                        record_ref,
                        created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                    ON CONFLICT(symbol, report_period) DO UPDATE SET
                        notice_date = excluded.notice_date,
                        source = excluded.source,
                        report_metrics_json = excluded.report_metrics_json,
                        batch_id = excluded.batch_id,
                        record_ref = excluded.record_ref,
                        created_at = excluded.created_at,
                        updated_at = CURRENT_TIMESTAMP",
                    params![
                        row.symbol,
                        row.report_period,
                        row.notice_date,
                        row.source,
                        row.report_metrics_json,
                        row.batch_id,
                        row.record_ref,
                        row.created_at,
                    ],
                )
                .map_err(|error| {
                    SecurityFundamentalHistoryStoreError::WriteRows(error.to_string())
                })?;
        }

        transaction
            .commit()
            .map_err(|error| SecurityFundamentalHistoryStoreError::WriteRows(error.to_string()))?;
        Ok(())
    }

    // 2026-04-12 CST: Resolve the latest governed fundamental row at or before an
    // analysis date, because fullstack replay should consume the same persisted
    // financial snapshot that validation recorded.
    // Purpose: centralize “latest by as-of-date” SQL for stock financial history.
    pub fn load_latest_record(
        &self,
        symbol: &str,
        as_of_date: Option<&str>,
    ) -> Result<Option<SecurityFundamentalHistoryRecordRow>, SecurityFundamentalHistoryStoreError>
    {
        let connection = self.open_connection()?;
        connection
            .query_row(
                "SELECT symbol, report_period, notice_date, source, report_metrics_json, batch_id, record_ref, created_at
                 FROM security_fundamental_history
                 WHERE symbol = ?1
                   AND (
                        ?2 IS NULL
                        OR COALESCE(notice_date, report_period) <= ?2
                   )
                 ORDER BY COALESCE(notice_date, report_period) DESC, report_period DESC
                 LIMIT 1",
                params![symbol, as_of_date],
                |row| {
                    Ok(SecurityFundamentalHistoryRecordRow {
                        symbol: row.get(0)?,
                        report_period: row.get(1)?,
                        notice_date: row.get(2)?,
                        source: row.get(3)?,
                        report_metrics_json: row.get(4)?,
                        batch_id: row.get(5)?,
                        record_ref: row.get(6)?,
                        created_at: row.get(7)?,
                    })
                },
            )
            .optional()
            .map_err(|error| SecurityFundamentalHistoryStoreError::ReadRows(error.to_string()))
    }

    fn open_connection(&self) -> Result<Connection, SecurityFundamentalHistoryStoreError> {
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                SecurityFundamentalHistoryStoreError::CreateRuntimeDir(error.to_string())
            })?;
        }

        let connection = Connection::open(&self.db_path).map_err(|error| {
            SecurityFundamentalHistoryStoreError::OpenDatabase(error.to_string())
        })?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|error| {
                SecurityFundamentalHistoryStoreError::OpenDatabase(error.to_string())
            })?;
        self.bootstrap_schema(&connection)?;
        Ok(connection)
    }

    fn bootstrap_schema(
        &self,
        connection: &Connection,
    ) -> Result<(), SecurityFundamentalHistoryStoreError> {
        connection
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS security_fundamental_history (
                    symbol TEXT NOT NULL,
                    report_period TEXT NOT NULL,
                    notice_date TEXT,
                    source TEXT NOT NULL,
                    report_metrics_json TEXT NOT NULL,
                    batch_id TEXT NOT NULL,
                    record_ref TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    PRIMARY KEY(symbol, report_period)
                );

                CREATE INDEX IF NOT EXISTS idx_security_fundamental_history_symbol_notice
                ON security_fundamental_history(symbol, notice_date, report_period);
                ",
            )
            .map_err(|error| {
                SecurityFundamentalHistoryStoreError::BootstrapSchema(error.to_string())
            })?;
        Ok(())
    }
}
