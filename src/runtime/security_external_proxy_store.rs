use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;

use crate::runtime::formal_security_runtime_registry::FormalSecurityRuntimeRegistry;

// 2026-04-11 CST: Add a dedicated runtime row for historical external proxy
// records, because P4 needs dated macro/ETF proxy history to become auditable
// input data instead of remaining transient manual inputs.
// Purpose: let training and feature snapshots resolve proxy values by
// symbol+date without inventing a second incompatible storage contract.
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityExternalProxyRecordRow {
    pub symbol: String,
    pub as_of_date: String,
    pub instrument_subscope: String,
    pub external_proxy_inputs_json: String,
    pub batch_id: String,
    pub record_ref: String,
    pub created_at: String,
}

// 2026-04-11 CST: Keep the runtime store isolated from ops-layer contracts,
// because runtime should persist dated proxy blobs without depending on stock ops
// types and causing circular module coupling.
// Purpose: store the canonical JSON payload and let the stock ops layer own the
// serde shape for SecurityExternalProxyInputs.
#[derive(Debug, Clone)]
pub struct SecurityExternalProxyStore {
    db_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum SecurityExternalProxyStoreError {
    #[error("failed to resolve external proxy runtime dir: {0}")]
    ResolveRuntimeDir(String),
    #[error("failed to create external proxy runtime dir: {0}")]
    CreateRuntimeDir(String),
    #[error("failed to open external proxy sqlite: {0}")]
    OpenDatabase(String),
    #[error("failed to bootstrap external proxy schema: {0}")]
    BootstrapSchema(String),
    #[error("external proxy backfill rows cannot be empty")]
    EmptyRows,
    #[error("failed to write external proxy rows: {0}")]
    WriteRows(String),
    #[error("failed to read external proxy rows: {0}")]
    ReadRows(String),
}

impl SecurityExternalProxyStore {
    // 2026-04-11 CST: Expose an explicit constructor for tests and custom runtime
    // paths, because the governed backfill tool must support isolated fixture
    // databases without mutating the shared workspace runtime.
    // Purpose: keep runtime-store usage symmetric with stock history storage.
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    // 2026-04-11 CST: Reuse the workspace runtime root for proxy-history storage,
    // because dated proxy backfill is part of the same governed securities runtime
    // and should follow the existing test/runtime override semantics.
    // Purpose: let tests inherit the runtime root from EXCEL_SKILL_RUNTIME_DB while
    // also allowing an explicit override for the dedicated proxy DB.
    pub fn workspace_default() -> Result<Self, SecurityExternalProxyStoreError> {
        Ok(Self::new(
            FormalSecurityRuntimeRegistry::external_proxy_db_path()
                .map_err(SecurityExternalProxyStoreError::ResolveRuntimeDir)?,
        ))
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    // 2026-04-11 CST: Persist backfill rows with symbol/date/subscope as the
    // governed primary key, because P4 wants one dated proxy snapshot per asset pool
    // without letting duplicate manual imports silently diverge.
    // Purpose: make later backfill reruns idempotent and auditable.
    pub fn upsert_rows(
        &self,
        rows: &[SecurityExternalProxyRecordRow],
    ) -> Result<(), SecurityExternalProxyStoreError> {
        if rows.is_empty() {
            return Err(SecurityExternalProxyStoreError::EmptyRows);
        }

        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| SecurityExternalProxyStoreError::WriteRows(error.to_string()))?;

        for row in rows {
            transaction
                .execute(
                    "INSERT INTO security_external_proxy_history (
                        symbol,
                        as_of_date,
                        instrument_subscope,
                        external_proxy_inputs_json,
                        batch_id,
                        record_ref,
                        created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                    ON CONFLICT(symbol, as_of_date, instrument_subscope) DO UPDATE SET
                        external_proxy_inputs_json = excluded.external_proxy_inputs_json,
                        batch_id = excluded.batch_id,
                        record_ref = excluded.record_ref,
                        created_at = excluded.created_at,
                        updated_at = CURRENT_TIMESTAMP",
                    params![
                        row.symbol,
                        row.as_of_date,
                        row.instrument_subscope,
                        row.external_proxy_inputs_json,
                        row.batch_id,
                        row.record_ref,
                        row.created_at,
                    ],
                )
                .map_err(|error| SecurityExternalProxyStoreError::WriteRows(error.to_string()))?;
        }

        transaction
            .commit()
            .map_err(|error| SecurityExternalProxyStoreError::WriteRows(error.to_string()))?;
        Ok(())
    }

    // 2026-04-11 CST: Load the dated proxy record by symbol and analysis date,
    // because feature snapshot and training need one deterministic historical proxy
    // view for a past as-of date.
    // Purpose: let the ops layer merge historical proxy context before it builds the
    // evidence hash and raw snapshot fields.
    pub fn load_record(
        &self,
        symbol: &str,
        as_of_date: &str,
    ) -> Result<Option<SecurityExternalProxyRecordRow>, SecurityExternalProxyStoreError> {
        let connection = self.open_connection()?;
        connection
            .query_row(
                "SELECT symbol, as_of_date, instrument_subscope, external_proxy_inputs_json, batch_id, record_ref, created_at
                 FROM security_external_proxy_history
                 WHERE symbol = ?1 AND as_of_date = ?2
                 ORDER BY updated_at DESC, created_at DESC
                 LIMIT 1",
                params![symbol, as_of_date],
                |row| {
                    Ok(SecurityExternalProxyRecordRow {
                        symbol: row.get(0)?,
                        as_of_date: row.get(1)?,
                        instrument_subscope: row.get(2)?,
                        external_proxy_inputs_json: row.get(3)?,
                        batch_id: row.get(4)?,
                        record_ref: row.get(5)?,
                        created_at: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(|error| SecurityExternalProxyStoreError::ReadRows(error.to_string()))
    }

    // 2026-04-12 UTC+08: Add a nearest-prior lookup for dated ETF proxy history,
    // because prediction-mode requests can arrive on non-trading dates such as weekends
    // while the governed proxy snapshots are still keyed by the latest trading day.
    // Purpose: let the final decision chain reuse the freshest auditable proxy inputs
    // on or before the requested date instead of degrading back to unavailable evidence.
    pub fn load_latest_record_on_or_before(
        &self,
        symbol: &str,
        as_of_date: &str,
    ) -> Result<Option<SecurityExternalProxyRecordRow>, SecurityExternalProxyStoreError> {
        let connection = self.open_connection()?;
        connection
            .query_row(
                "SELECT symbol, as_of_date, instrument_subscope, external_proxy_inputs_json, batch_id, record_ref, created_at
                 FROM security_external_proxy_history
                 WHERE symbol = ?1 AND as_of_date <= ?2
                 ORDER BY as_of_date DESC, updated_at DESC, created_at DESC
                 LIMIT 1",
                params![symbol, as_of_date],
                |row| {
                    Ok(SecurityExternalProxyRecordRow {
                        symbol: row.get(0)?,
                        as_of_date: row.get(1)?,
                        instrument_subscope: row.get(2)?,
                        external_proxy_inputs_json: row.get(3)?,
                        batch_id: row.get(4)?,
                        record_ref: row.get(5)?,
                        created_at: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(|error| SecurityExternalProxyStoreError::ReadRows(error.to_string()))
    }

    // 2026-04-17 CST: Added because ETF latest-proxy requests can omit an explicit
    // as_of_date while still expecting the governed runtime to anchor on the freshest
    // auditable proxy snapshot.
    // Reason: only supporting exact-date and on-or-before lookups leaves the no-date
    // chair/evidence path blind to already-imported proxy history.
    // Purpose: give upstream evidence resolution one canonical "latest known proxy row"
    // lookup without duplicating SQL in the ops layer.
    pub fn load_latest_record(
        &self,
        symbol: &str,
    ) -> Result<Option<SecurityExternalProxyRecordRow>, SecurityExternalProxyStoreError> {
        let connection = self.open_connection()?;
        connection
            .query_row(
                "SELECT symbol, as_of_date, instrument_subscope, external_proxy_inputs_json, batch_id, record_ref, created_at
                 FROM security_external_proxy_history
                 WHERE symbol = ?1
                 ORDER BY as_of_date DESC, updated_at DESC, created_at DESC
                 LIMIT 1",
                params![symbol],
                |row| {
                    Ok(SecurityExternalProxyRecordRow {
                        symbol: row.get(0)?,
                        as_of_date: row.get(1)?,
                        instrument_subscope: row.get(2)?,
                        external_proxy_inputs_json: row.get(3)?,
                        batch_id: row.get(4)?,
                        record_ref: row.get(5)?,
                        created_at: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(|error| SecurityExternalProxyStoreError::ReadRows(error.to_string()))
    }

    fn open_connection(&self) -> Result<Connection, SecurityExternalProxyStoreError> {
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                SecurityExternalProxyStoreError::CreateRuntimeDir(error.to_string())
            })?;
        }

        let connection = Connection::open(&self.db_path)
            .map_err(|error| SecurityExternalProxyStoreError::OpenDatabase(error.to_string()))?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|error| SecurityExternalProxyStoreError::OpenDatabase(error.to_string()))?;
        self.bootstrap_schema(&connection)?;
        Ok(connection)
    }

    fn bootstrap_schema(
        &self,
        connection: &Connection,
    ) -> Result<(), SecurityExternalProxyStoreError> {
        connection
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS security_external_proxy_history (
                    symbol TEXT NOT NULL,
                    as_of_date TEXT NOT NULL,
                    instrument_subscope TEXT NOT NULL,
                    external_proxy_inputs_json TEXT NOT NULL,
                    batch_id TEXT NOT NULL,
                    record_ref TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    PRIMARY KEY(symbol, as_of_date, instrument_subscope)
                );

                CREATE INDEX IF NOT EXISTS idx_security_external_proxy_history_symbol_date
                ON security_external_proxy_history(symbol, as_of_date);
                ",
            )
            .map_err(|error| SecurityExternalProxyStoreError::BootstrapSchema(error.to_string()))?;
        Ok(())
    }
}
