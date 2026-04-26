use std::path::PathBuf;

use rusqlite::{Connection, OptionalExtension};
use serde_json::Value;
use thiserror::Error;

// 2026-04-25 CST: Added because the merged stock runtime registry references
// the governed capital-flow store while the implementation file is absent.
// Reason: training and capital-source boundaries need a compile-stable store facade.
// Purpose: provide the narrow read surface currently used by training without widening runtime behavior.
#[derive(Debug, Clone)]
pub struct SecurityCapitalFlowStore {
    db_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum SecurityCapitalFlowStoreError {
    #[error("security capital flow store sqlite failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("security capital flow store parent directory is missing")]
    MissingParentDirectory,
}

impl SecurityCapitalFlowStore {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    pub fn insert_records(
        &self,
        records: &[SecurityCapitalFlowRecord],
    ) -> Result<usize, SecurityCapitalFlowStoreError> {
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|_| SecurityCapitalFlowStoreError::MissingParentDirectory)?;
        }
        let mut connection = Connection::open(&self.db_path)?;
        bootstrap_schema(&connection)?;
        let transaction = connection.transaction()?;
        for record in records {
            transaction.execute(
                "insert into capital_flow_observations (
                    dataset_id, frequency, observation_date, series_key, value, source, payload_json
                ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    record.dataset_id,
                    record.frequency,
                    record.metric_date,
                    record.series_key,
                    record.value,
                    record.source,
                    record.payload_json.to_string(),
                ],
            )?;
        }
        transaction.commit()?;
        Ok(records.len())
    }

    pub fn load_metric_dates_in_range(
        &self,
        dataset_id: &str,
        frequency: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<String>, SecurityCapitalFlowStoreError> {
        if !self.db_path.exists() {
            return Ok(Vec::new());
        }

        let connection = Connection::open(&self.db_path)?;
        let table_exists: Option<i64> = connection
            .query_row(
                "select 1 from sqlite_master where type = 'table' and name = 'capital_flow_observations'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if table_exists.is_none() {
            return Ok(Vec::new());
        }

        let mut statement = connection.prepare(
            "select distinct observation_date
             from capital_flow_observations
             where dataset_id = ?1 and frequency = ?2 and observation_date >= ?3 and observation_date <= ?4
             order by observation_date",
        )?;
        let rows = statement.query_map(
            rusqlite::params![dataset_id, frequency, start_date, end_date],
            |row| row.get::<_, String>(0),
        )?;

        let mut dates = Vec::new();
        for row in rows {
            dates.push(row?);
        }
        Ok(dates)
    }

    pub fn load_records_until(
        &self,
        as_of_date: &str,
    ) -> Result<Vec<SecurityCapitalFlowRecord>, SecurityCapitalFlowStoreError> {
        if !self.db_path.exists() {
            return Ok(Vec::new());
        }
        let connection = Connection::open(&self.db_path)?;
        let table_exists: Option<i64> = connection
            .query_row(
                "select 1 from sqlite_master where type = 'table' and name = 'capital_flow_observations'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if table_exists.is_none() {
            return Ok(Vec::new());
        }

        let mut statement = connection.prepare(
            "select dataset_id, frequency, observation_date, series_key, value, source, payload_json
             from capital_flow_observations
             where observation_date <= ?1
             order by observation_date",
        )?;
        let rows = statement.query_map([as_of_date], |row| {
            let payload_text: String = row.get(6)?;
            Ok(SecurityCapitalFlowRecord {
                dataset_id: row.get(0)?,
                frequency: row.get(1)?,
                metric_date: row.get(2)?,
                series_key: row.get(3)?,
                value: row.get(4)?,
                source: row.get(5)?,
                payload_json: serde_json::from_str(&payload_text).unwrap_or(Value::Null),
            })
        })?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SecurityCapitalFlowRecord {
    pub dataset_id: String,
    pub frequency: String,
    pub metric_date: String,
    pub series_key: String,
    pub value: f64,
    pub source: String,
    pub payload_json: Value,
}

fn bootstrap_schema(connection: &Connection) -> Result<(), SecurityCapitalFlowStoreError> {
    connection.execute_batch(
        "create table if not exists capital_flow_observations (
            id integer primary key autoincrement,
            dataset_id text not null,
            frequency text not null,
            observation_date text not null,
            series_key text not null,
            value real not null,
            source text not null,
            payload_json text not null
        );",
    )?;
    Ok(())
}
