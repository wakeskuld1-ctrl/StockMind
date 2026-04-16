use std::fs;
use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;

use crate::runtime::security_execution_store::SecurityExecutionStoreError;
use crate::runtime::security_execution_store_schema::bootstrap_security_execution_schema;

// 2026-04-15 CST: Extracted from security_execution_store.rs because round 2
// plan B now needs connection bootstrap separated from the store facade.
// Purpose: keep runtime directory creation, SQLite opening, and schema bootstrap
// on one governed boundary before later repository methods are split further.
pub(crate) fn open_security_execution_store_connection(
    db_path: &Path,
) -> Result<Connection, SecurityExecutionStoreError> {
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| SecurityExecutionStoreError::CreateRuntimeDir(error.to_string()))?;
    }

    let connection = Connection::open(db_path)
        .map_err(|error| SecurityExecutionStoreError::OpenDatabase(error.to_string()))?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|error| SecurityExecutionStoreError::OpenDatabase(error.to_string()))?;
    bootstrap_security_execution_schema(&connection)?;
    Ok(connection)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::open_security_execution_store_connection;

    #[test]
    fn open_security_execution_store_connection_creates_parent_directory() {
        let root = unique_temp_dir("security-execution-store-connection");
        let db_path = root.join("nested").join("security_execution.db");

        let connection = open_security_execution_store_connection(&db_path)
            .expect("execution store connection should open");
        drop(connection);

        assert!(db_path.exists());
        assert!(db_path.parent().expect("db parent should exist").exists());

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_dir_all(&root);
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let unique = format!(
            "{}-{}-{}",
            prefix,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        );
        std::env::temp_dir().join(unique)
    }
}
