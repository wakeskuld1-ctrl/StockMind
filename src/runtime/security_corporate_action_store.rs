use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, params};
use thiserror::Error;

use crate::runtime::formal_security_runtime_registry::FormalSecurityRuntimeRegistry;

// 2026-04-16 CST: Added because P0-1 needs one dated corporate-action runtime contract
// instead of leaving dividend facts scattered in ad hoc notes or future manual inputs.
// Purpose: make cash dividends and later split-style events a governed runtime fact source.
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityCorporateActionRow {
    pub symbol: String,
    pub effective_date: String,
    pub action_type: String,
    pub cash_dividend_per_share: f64,
    pub split_ratio: f64,
    pub bonus_ratio: f64,
    pub source: String,
    pub payload_json: String,
}

// 2026-04-16 CST: Added because the holding-summary layer needs one formal runtime facade
// parallel to stock history, proxy history, and fundamentals.
// Purpose: keep corporate-action persistence on the governed runtime family instead of
// coupling it to execution or price stores.
#[derive(Debug, Clone)]
pub struct SecurityCorporateActionStore {
    db_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum SecurityCorporateActionStoreError {
    #[error("failed to resolve corporate action runtime dir: {0}")]
    ResolveRuntimeDir(String),
    #[error("failed to create corporate action runtime dir: {0}")]
    CreateRuntimeDir(String),
    #[error("failed to open corporate action sqlite: {0}")]
    OpenDatabase(String),
    #[error("failed to bootstrap corporate action schema: {0}")]
    BootstrapSchema(String),
    #[error("corporate action rows cannot be empty")]
    EmptyRows,
    #[error("failed to write corporate action rows: {0}")]
    WriteRows(String),
    #[error("failed to read corporate action rows: {0}")]
    ReadRows(String),
}

impl SecurityCorporateActionStore {
    // 2026-04-16 CST: Added because tests and isolated runtime slices still need an explicit
    // constructor path.
    // Purpose: mirror the existing governed store pattern without inventing a special setup path.
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    // 2026-04-16 CST: Added because corporate-action storage must follow the same runtime-root
    // policy as the rest of the governed securities family.
    // Purpose: keep one default-path rule for this new formal store.
    pub fn workspace_default() -> Result<Self, SecurityCorporateActionStoreError> {
        Ok(Self::new(
            FormalSecurityRuntimeRegistry::corporate_action_db_path()
                .map_err(SecurityCorporateActionStoreError::ResolveRuntimeDir)?,
        ))
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    // 2026-04-16 CST: Added because backfill and manual reconciliation both need one idempotent
    // write path keyed by symbol plus effective date plus action type.
    // Purpose: let later reruns overwrite the same action fact instead of duplicating it.
    pub fn upsert_rows(
        &self,
        rows: &[SecurityCorporateActionRow],
    ) -> Result<(), SecurityCorporateActionStoreError> {
        if rows.is_empty() {
            return Err(SecurityCorporateActionStoreError::EmptyRows);
        }

        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| SecurityCorporateActionStoreError::WriteRows(error.to_string()))?;

        for row in rows {
            transaction
                .execute(
                    "INSERT INTO security_corporate_actions (
                        symbol,
                        effective_date,
                        action_type,
                        cash_dividend_per_share,
                        split_ratio,
                        bonus_ratio,
                        source,
                        payload_json
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                    ON CONFLICT(symbol, effective_date, action_type) DO UPDATE SET
                        cash_dividend_per_share = excluded.cash_dividend_per_share,
                        split_ratio = excluded.split_ratio,
                        bonus_ratio = excluded.bonus_ratio,
                        source = excluded.source,
                        payload_json = excluded.payload_json,
                        updated_at = CURRENT_TIMESTAMP",
                    params![
                        row.symbol,
                        row.effective_date,
                        row.action_type,
                        row.cash_dividend_per_share,
                        row.split_ratio,
                        row.bonus_ratio,
                        row.source,
                        row.payload_json,
                    ],
                )
                .map_err(|error| SecurityCorporateActionStoreError::WriteRows(error.to_string()))?;
        }

        transaction
            .commit()
            .map_err(|error| SecurityCorporateActionStoreError::WriteRows(error.to_string()))?;
        Ok(())
    }

    // 2026-04-16 CST: Added because the P0-1 holding-yield helper needs one deterministic
    // lookup for all actions already effective by the resolved trade date.
    // Purpose: keep date filtering inside the store instead of repeating SQL in business code.
    pub fn load_rows_on_or_before(
        &self,
        symbol: &str,
        effective_date: &str,
    ) -> Result<Vec<SecurityCorporateActionRow>, SecurityCorporateActionStoreError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "
                SELECT symbol, effective_date, action_type, cash_dividend_per_share, split_ratio,
                       bonus_ratio, source, payload_json
                FROM security_corporate_actions
                WHERE symbol = ?1
                  AND effective_date <= ?2
                ORDER BY effective_date ASC, action_type ASC
                ",
            )
            .map_err(|error| SecurityCorporateActionStoreError::ReadRows(error.to_string()))?;

        let mapped_rows = statement
            .query_map(params![symbol, effective_date], |row| {
                Ok(SecurityCorporateActionRow {
                    symbol: row.get(0)?,
                    effective_date: row.get(1)?,
                    action_type: row.get(2)?,
                    cash_dividend_per_share: row.get(3)?,
                    split_ratio: row.get(4)?,
                    bonus_ratio: row.get(5)?,
                    source: row.get(6)?,
                    payload_json: row.get(7)?,
                })
            })
            .map_err(|error| SecurityCorporateActionStoreError::ReadRows(error.to_string()))?;

        let mut rows = Vec::new();
        for row in mapped_rows {
            rows.push(
                row.map_err(|error| {
                    SecurityCorporateActionStoreError::ReadRows(error.to_string())
                })?,
            );
        }
        Ok(rows)
    }

    fn open_connection(&self) -> Result<Connection, SecurityCorporateActionStoreError> {
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                SecurityCorporateActionStoreError::CreateRuntimeDir(error.to_string())
            })?;
        }

        let connection = Connection::open(&self.db_path)
            .map_err(|error| SecurityCorporateActionStoreError::OpenDatabase(error.to_string()))?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|error| SecurityCorporateActionStoreError::OpenDatabase(error.to_string()))?;
        self.bootstrap_schema(&connection)?;
        Ok(connection)
    }

    fn bootstrap_schema(
        &self,
        connection: &Connection,
    ) -> Result<(), SecurityCorporateActionStoreError> {
        connection
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS security_corporate_actions (
                    symbol TEXT NOT NULL,
                    effective_date TEXT NOT NULL,
                    action_type TEXT NOT NULL,
                    cash_dividend_per_share REAL NOT NULL DEFAULT 0,
                    split_ratio REAL NOT NULL DEFAULT 1,
                    bonus_ratio REAL NOT NULL DEFAULT 0,
                    source TEXT NOT NULL,
                    payload_json TEXT NOT NULL DEFAULT '{}',
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    PRIMARY KEY (symbol, effective_date, action_type)
                );
                CREATE INDEX IF NOT EXISTS idx_security_corporate_actions_symbol_date
                ON security_corporate_actions(symbol, effective_date);
                ",
            )
            .map_err(|error| {
                SecurityCorporateActionStoreError::BootstrapSchema(error.to_string())
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db_path(file_name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{file_name}_{nanos}.db"))
    }

    #[test]
    fn corporate_action_store_round_trips_dated_rows() {
        let db_path = temp_db_path("security_corporate_action_store_round_trip");
        let store = SecurityCorporateActionStore::new(db_path.clone());

        store
            .upsert_rows(&[
                SecurityCorporateActionRow {
                    symbol: "002352.SZ".to_string(),
                    effective_date: "2025-05-20".to_string(),
                    action_type: "cash_dividend".to_string(),
                    cash_dividend_per_share: 0.45,
                    split_ratio: 1.0,
                    bonus_ratio: 0.0,
                    source: "fixture".to_string(),
                    payload_json: "{}".to_string(),
                },
                SecurityCorporateActionRow {
                    symbol: "002352.SZ".to_string(),
                    effective_date: "2025-06-20".to_string(),
                    action_type: "bonus".to_string(),
                    cash_dividend_per_share: 0.0,
                    split_ratio: 1.0,
                    bonus_ratio: 0.2,
                    source: "fixture".to_string(),
                    payload_json: "{\"ratio\":0.2}".to_string(),
                },
            ])
            .expect("rows should persist");

        let rows = store
            .load_rows_on_or_before("002352.SZ", "2025-05-31")
            .expect("rows should load");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].cash_dividend_per_share, 0.45);

        let _ = fs::remove_file(db_path);
    }
}
