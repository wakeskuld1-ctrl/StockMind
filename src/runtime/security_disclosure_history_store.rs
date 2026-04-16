use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, params};
use thiserror::Error;

use crate::runtime::formal_security_runtime_registry::FormalSecurityRuntimeRegistry;

// 2026-04-12 CST: Add a dedicated runtime row for stock disclosure history,
// because governed replay should persist recent announcements instead of rebuilding
// disclosure context from one-off live requests every time.
// Purpose: make stock event history auditable and replayable by symbol/date.
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityDisclosureHistoryRecordRow {
    pub symbol: String,
    pub published_at: String,
    pub title: String,
    pub article_code: Option<String>,
    pub category: Option<String>,
    pub source: String,
    pub batch_id: String,
    pub record_ref: String,
    pub created_at: String,
}

// 2026-04-12 CST: Keep disclosure history isolated inside runtime storage,
// because the persistence layer should not depend on full disclosure-context
// aggregation semantics.
// Purpose: let ops rebuild keyword/risk summaries at read time without coupling SQLite rows to UI text.
#[derive(Debug, Clone)]
pub struct SecurityDisclosureHistoryStore {
    db_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum SecurityDisclosureHistoryStoreError {
    #[error("failed to resolve disclosure history runtime dir: {0}")]
    ResolveRuntimeDir(String),
    #[error("failed to create disclosure history runtime dir: {0}")]
    CreateRuntimeDir(String),
    #[error("failed to open disclosure history sqlite: {0}")]
    OpenDatabase(String),
    #[error("failed to bootstrap disclosure history schema: {0}")]
    BootstrapSchema(String),
    #[error("disclosure history rows cannot be empty")]
    EmptyRows,
    #[error("failed to write disclosure history rows: {0}")]
    WriteRows(String),
    #[error("failed to read disclosure history rows: {0}")]
    ReadRows(String),
}

impl SecurityDisclosureHistoryStore {
    // 2026-04-12 CST: Keep an explicit constructor for tests and validation
    // slices, because disclosure replay must support isolated runtime roots.
    // Purpose: let validation tools persist slice-local event history.
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    // 2026-04-12 CST: Reuse workspace runtime semantics, because disclosure
    // history should follow the same override behavior as other governed stores.
    // Purpose: keep tests and isolated runtime roots predictable.
    pub fn workspace_default() -> Result<Self, SecurityDisclosureHistoryStoreError> {
        Ok(Self::new(
            FormalSecurityRuntimeRegistry::disclosure_history_db_path()
                .map_err(SecurityDisclosureHistoryStoreError::ResolveRuntimeDir)?,
        ))
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    // 2026-04-12 CST: Upsert by record_ref, because one announcement should have
    // one durable governed identity even if the same source is imported multiple times.
    // Purpose: keep event history idempotent across reruns.
    pub fn upsert_rows(
        &self,
        rows: &[SecurityDisclosureHistoryRecordRow],
    ) -> Result<(), SecurityDisclosureHistoryStoreError> {
        if rows.is_empty() {
            return Err(SecurityDisclosureHistoryStoreError::EmptyRows);
        }

        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| SecurityDisclosureHistoryStoreError::WriteRows(error.to_string()))?;

        for row in rows {
            transaction
                .execute(
                    "INSERT INTO security_disclosure_history (
                        symbol,
                        published_at,
                        title,
                        article_code,
                        category,
                        source,
                        batch_id,
                        record_ref,
                        created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                    ON CONFLICT(record_ref) DO UPDATE SET
                        symbol = excluded.symbol,
                        published_at = excluded.published_at,
                        title = excluded.title,
                        article_code = excluded.article_code,
                        category = excluded.category,
                        source = excluded.source,
                        batch_id = excluded.batch_id,
                        created_at = excluded.created_at,
                        updated_at = CURRENT_TIMESTAMP",
                    params![
                        row.symbol,
                        row.published_at,
                        row.title,
                        row.article_code,
                        row.category,
                        row.source,
                        row.batch_id,
                        row.record_ref,
                        row.created_at,
                    ],
                )
                .map_err(|error| {
                    SecurityDisclosureHistoryStoreError::WriteRows(error.to_string())
                })?;
        }

        transaction
            .commit()
            .map_err(|error| SecurityDisclosureHistoryStoreError::WriteRows(error.to_string()))?;
        Ok(())
    }

    // 2026-04-12 CST: Load the most recent governed announcements at or before an
    // analysis date, because fullstack replay should derive disclosure context from
    // the same persisted rows that validation recorded.
    // Purpose: centralize “latest recent announcements” SQL for stock event history.
    pub fn load_recent_records(
        &self,
        symbol: &str,
        as_of_date: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SecurityDisclosureHistoryRecordRow>, SecurityDisclosureHistoryStoreError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT symbol, published_at, title, article_code, category, source, batch_id, record_ref, created_at
                 FROM security_disclosure_history
                 WHERE symbol = ?1
                   AND (?2 IS NULL OR published_at <= ?2)
                 ORDER BY published_at DESC, rowid DESC
                 LIMIT ?3",
            )
            .map_err(|error| SecurityDisclosureHistoryStoreError::ReadRows(error.to_string()))?;

        let mapped_rows = statement
            .query_map(params![symbol, as_of_date, limit as i64], |row| {
                Ok(SecurityDisclosureHistoryRecordRow {
                    symbol: row.get(0)?,
                    published_at: row.get(1)?,
                    title: row.get(2)?,
                    article_code: row.get(3)?,
                    category: row.get(4)?,
                    source: row.get(5)?,
                    batch_id: row.get(6)?,
                    record_ref: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })
            .map_err(|error| SecurityDisclosureHistoryStoreError::ReadRows(error.to_string()))?;

        let mut rows = Vec::new();
        for row in mapped_rows {
            rows.push(row.map_err(|error| {
                SecurityDisclosureHistoryStoreError::ReadRows(error.to_string())
            })?);
        }
        Ok(rows)
    }

    fn open_connection(&self) -> Result<Connection, SecurityDisclosureHistoryStoreError> {
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                SecurityDisclosureHistoryStoreError::CreateRuntimeDir(error.to_string())
            })?;
        }

        let connection = Connection::open(&self.db_path).map_err(|error| {
            SecurityDisclosureHistoryStoreError::OpenDatabase(error.to_string())
        })?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|error| {
                SecurityDisclosureHistoryStoreError::OpenDatabase(error.to_string())
            })?;
        self.bootstrap_schema(&connection)?;
        Ok(connection)
    }

    fn bootstrap_schema(
        &self,
        connection: &Connection,
    ) -> Result<(), SecurityDisclosureHistoryStoreError> {
        connection
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS security_disclosure_history (
                    symbol TEXT NOT NULL,
                    published_at TEXT NOT NULL,
                    title TEXT NOT NULL,
                    article_code TEXT,
                    category TEXT,
                    source TEXT NOT NULL,
                    batch_id TEXT NOT NULL,
                    record_ref TEXT NOT NULL PRIMARY KEY,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );

                CREATE INDEX IF NOT EXISTS idx_security_disclosure_history_symbol_date
                ON security_disclosure_history(symbol, published_at);
                ",
            )
            .map_err(|error| {
                SecurityDisclosureHistoryStoreError::BootstrapSchema(error.to_string())
            })?;
        Ok(())
    }
}
