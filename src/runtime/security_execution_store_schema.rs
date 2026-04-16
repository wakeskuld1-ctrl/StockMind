use rusqlite::Connection;

use crate::runtime::security_execution_store::SecurityExecutionStoreError;

// 2026-04-15 CST: Extracted from security_execution_store.rs because round 2
// plan B now needs schema ownership separated from the store facade.
// Purpose: keep formal execution-store table bootstrap on one dedicated
// boundary before later store responsibilities are split further.
pub(crate) fn bootstrap_security_execution_schema(
    connection: &Connection,
) -> Result<(), SecurityExecutionStoreError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS security_position_plan_records (
                position_plan_ref TEXT PRIMARY KEY,
                symbol TEXT NOT NULL,
                analysis_date TEXT NOT NULL,
                decision_ref TEXT NOT NULL,
                approval_ref TEXT NOT NULL,
                evidence_version TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS security_position_adjustment_events (
                adjustment_event_ref TEXT PRIMARY KEY,
                position_plan_ref TEXT NOT NULL,
                symbol TEXT NOT NULL,
                event_date TEXT NOT NULL,
                decision_ref TEXT NOT NULL,
                approval_ref TEXT NOT NULL,
                evidence_version TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_security_position_plan_symbol_date
                ON security_position_plan_records(symbol, analysis_date);
            CREATE INDEX IF NOT EXISTS idx_security_adjustment_plan_date
                ON security_position_adjustment_events(position_plan_ref, event_date);
            CREATE TABLE IF NOT EXISTS security_execution_records (
                execution_record_id TEXT PRIMARY KEY,
                account_id TEXT,
                symbol TEXT NOT NULL,
                analysis_date TEXT NOT NULL,
                position_state TEXT NOT NULL,
                current_position_pct REAL NOT NULL,
                sector_tag TEXT,
                payload_json TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_security_execution_account_state
                ON security_execution_records(account_id, position_state, analysis_date);",
        )
        .map_err(|error| SecurityExecutionStoreError::BootstrapSchema(error.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::bootstrap_security_execution_schema;

    #[test]
    fn bootstrap_security_execution_schema_creates_execution_tables() {
        let connection = Connection::open_in_memory().expect("in-memory sqlite should open");

        bootstrap_security_execution_schema(&connection)
            .expect("execution store schema should bootstrap");

        let mut statement = connection
            .prepare(
                "SELECT name
                 FROM sqlite_master
                 WHERE type = 'table'
                   AND name IN (
                       'security_position_plan_records',
                       'security_position_adjustment_events',
                       'security_execution_records'
                   )
                 ORDER BY name",
            )
            .expect("sqlite_master query should prepare");
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .expect("sqlite_master query should run");
        let table_names = rows
            .map(|row| row.expect("table name row should load"))
            .collect::<Vec<_>>();

        assert_eq!(
            table_names,
            vec![
                "security_execution_records".to_string(),
                "security_position_adjustment_events".to_string(),
                "security_position_plan_records".to_string(),
            ]
        );
    }
}
