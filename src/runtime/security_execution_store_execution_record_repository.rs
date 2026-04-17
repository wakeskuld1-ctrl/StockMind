use rusqlite::{Connection, params};

use crate::ops::stock::security_execution_record::SecurityExecutionRecordDocument;
use crate::runtime::security_execution_store::SecurityExecutionStoreError;

// 2026-04-15 CST: Extracted from security_execution_store.rs because round 2
// plan B now needs execution-record persistence isolated from the store facade.
// Purpose: keep formal execution-record SQL on one dedicated repository
// boundary while the store facade remains only the runtime entry surface.
pub(crate) fn upsert_security_execution_record(
    connection: &Connection,
    record: &SecurityExecutionRecordDocument,
) -> Result<(), SecurityExecutionStoreError> {
    let payload = serde_json::to_string(record)
        .map_err(|error| SecurityExecutionStoreError::SerializePayload(error.to_string()))?;
    connection
        .execute(
            "INSERT INTO security_execution_records (
                execution_record_id,
                account_id,
                symbol,
                analysis_date,
                position_state,
                current_position_pct,
                sector_tag,
                payload_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(execution_record_id) DO UPDATE SET
                account_id = excluded.account_id,
                symbol = excluded.symbol,
                analysis_date = excluded.analysis_date,
                position_state = excluded.position_state,
                current_position_pct = excluded.current_position_pct,
                sector_tag = excluded.sector_tag,
                payload_json = excluded.payload_json,
                updated_at = CURRENT_TIMESTAMP",
            params![
                record.execution_record_id,
                record.account_id,
                record.symbol,
                record.analysis_date,
                record.position_state,
                record.current_position_pct,
                record.sector_tag,
                payload,
            ],
        )
        .map_err(|error| SecurityExecutionStoreError::WriteExecutionRecord(error.to_string()))?;
    Ok(())
}

pub(crate) fn load_latest_open_security_execution_records(
    connection: &Connection,
    account_id: &str,
) -> Result<Vec<SecurityExecutionRecordDocument>, SecurityExecutionStoreError> {
    let mut statement = connection
        .prepare(
            "SELECT payload_json
             FROM security_execution_records
             WHERE account_id = ?1
               AND position_state = 'open'
             ORDER BY analysis_date DESC, updated_at DESC",
        )
        .map_err(|error| SecurityExecutionStoreError::ReadExecutionRecord(error.to_string()))?;
    let rows = statement
        .query_map(params![account_id], |row| row.get::<_, String>(0))
        .map_err(|error| SecurityExecutionStoreError::ReadExecutionRecord(error.to_string()))?;

    let mut records = Vec::new();
    for row in rows {
        let payload = row
            .map_err(|error| SecurityExecutionStoreError::ReadExecutionRecord(error.to_string()))?;
        let record = serde_json::from_str::<SecurityExecutionRecordDocument>(&payload)
            .map_err(|error| SecurityExecutionStoreError::DeserializePayload(error.to_string()))?;
        records.push(record);
    }
    Ok(records)
}

// 2026-04-17 CST: Reason=lifecycle review tools now need to reopen one governed
// execution record by ref instead of rebuilding everything from ad hoc caller state.
// Purpose=keep lifecycle follow-up tools anchored on the persisted execution fact.
pub(crate) fn load_security_execution_record(
    connection: &Connection,
    execution_record_id: &str,
) -> Result<Option<SecurityExecutionRecordDocument>, SecurityExecutionStoreError> {
    let mut statement = connection
        .prepare(
            "SELECT payload_json
             FROM security_execution_records
             WHERE execution_record_id = ?1
             LIMIT 1",
        )
        .map_err(|error| SecurityExecutionStoreError::ReadExecutionRecord(error.to_string()))?;
    let mut rows = statement
        .query(params![execution_record_id])
        .map_err(|error| SecurityExecutionStoreError::ReadExecutionRecord(error.to_string()))?;
    let Some(row) = rows
        .next()
        .map_err(|error| SecurityExecutionStoreError::ReadExecutionRecord(error.to_string()))?
    else {
        return Ok(None);
    };
    let payload: String = row
        .get(0)
        .map_err(|error| SecurityExecutionStoreError::ReadExecutionRecord(error.to_string()))?;
    let record = serde_json::from_str::<SecurityExecutionRecordDocument>(&payload)
        .map_err(|error| SecurityExecutionStoreError::DeserializePayload(error.to_string()))?;
    Ok(Some(record))
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::{
        load_latest_open_security_execution_records, load_security_execution_record,
        upsert_security_execution_record,
    };
    use crate::ops::stock::security_execution_record::SecurityExecutionRecordDocument;
    use crate::runtime::security_execution_store_schema::bootstrap_security_execution_schema;

    #[test]
    fn execution_record_repository_round_trips_open_records() {
        let connection = Connection::open_in_memory().expect("in-memory sqlite should open");
        bootstrap_security_execution_schema(&connection)
            .expect("execution schema should bootstrap");
        let record = SecurityExecutionRecordDocument {
            execution_record_id: "record-1".to_string(),
            contract_version: "security_execution_record.v1".to_string(),
            document_type: "security_execution_record".to_string(),
            generated_at: "2026-04-15T10:00:00+08:00".to_string(),
            symbol: "601916.SH".to_string(),
            analysis_date: "2026-04-15".to_string(),
            account_id: Some("acct-1".to_string()),
            sector_tag: Some("bank".to_string()),
            position_state: "open".to_string(),
            portfolio_position_plan_ref: Some("portfolio-plan-1".to_string()),
            execution_journal_ref: "journal-1".to_string(),
            position_plan_ref: "plan-1".to_string(),
            snapshot_ref: "snapshot-1".to_string(),
            outcome_ref: "outcome-1".to_string(),
            planned_entry_date: "2026-04-14".to_string(),
            planned_entry_price: 10.0,
            planned_position_pct: 0.05,
            planned_max_position_pct: 0.12,
            actual_entry_date: "2026-04-15".to_string(),
            actual_entry_price: 10.1,
            actual_position_pct: 0.08,
            current_position_pct: 0.08,
            actual_exit_date: String::new(),
            actual_exit_price: 0.0,
            exit_reason: "position_still_open".to_string(),
            holding_days: 1,
            planned_forward_return: 0.06,
            actual_return: 0.0,
            entry_slippage_pct: 0.01,
            position_size_gap_pct: 0.03,
            planned_tranche_action: Some("entry_tranche".to_string()),
            planned_tranche_pct: Some(0.05),
            planned_peak_position_pct: Some(0.12),
            actual_tranche_action: Some("entry_tranche".to_string()),
            actual_tranche_pct: Some(0.08),
            actual_peak_position_pct: Some(0.08),
            tranche_count_drift: Some(0),
            account_budget_alignment: Some("aligned".to_string()),
            execution_return_gap: -0.06,
            execution_quality: "open_position_pending".to_string(),
            price_as_of_date: None,
            resolved_trade_date: None,
            current_price: None,
            share_adjustment_factor: None,
            cumulative_cash_dividend_per_share: None,
            dividend_adjusted_cost_basis: None,
            holding_total_return_pct: None,
            breakeven_price: None,
            corporate_action_summary: None,
            execution_record_notes: vec!["fixture".to_string()],
            attribution_summary: "fixture".to_string(),
        };

        upsert_security_execution_record(&connection, &record)
            .expect("execution record should persist");
        let loaded = load_latest_open_security_execution_records(&connection, "acct-1")
            .expect("execution records should load");
        let loaded_by_ref = load_security_execution_record(&connection, "record-1")
            .expect("execution record should load by ref");

        // 2026-04-17 CST: Reason=the repository test now checks both list and point lookup
        // against the same fixture instance. Purpose=avoid moving the record before the
        // second assertion while keeping both round-trip checks intact.
        assert_eq!(loaded, vec![record.clone()]);
        assert_eq!(loaded_by_ref, Some(record));
    }
}
