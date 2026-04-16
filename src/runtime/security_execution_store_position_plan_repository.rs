use rusqlite::{Connection, params};

use crate::runtime::security_execution_store::SecurityExecutionStoreError;
use crate::tools::contracts::SecurityPositionPlanRecordResult;

// 2026-04-15 CST: Extracted from security_execution_store.rs because round 2
// plan B now needs position-plan persistence isolated from the store facade.
// Purpose: keep formal position-plan record SQL on one dedicated repository
// boundary while the store facade remains only the runtime entry surface.
pub(crate) fn upsert_security_position_plan_record(
    connection: &Connection,
    record: &SecurityPositionPlanRecordResult,
) -> Result<(), SecurityExecutionStoreError> {
    let payload = serde_json::to_string(record)
        .map_err(|error| SecurityExecutionStoreError::SerializePayload(error.to_string()))?;
    connection
        .execute(
            "INSERT INTO security_position_plan_records (
                position_plan_ref,
                symbol,
                analysis_date,
                decision_ref,
                approval_ref,
                evidence_version,
                payload_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(position_plan_ref) DO UPDATE SET
                symbol = excluded.symbol,
                analysis_date = excluded.analysis_date,
                decision_ref = excluded.decision_ref,
                approval_ref = excluded.approval_ref,
                evidence_version = excluded.evidence_version,
                payload_json = excluded.payload_json,
                updated_at = CURRENT_TIMESTAMP",
            params![
                record.position_plan_ref,
                record.symbol,
                record.analysis_date,
                record.decision_ref,
                record.approval_ref,
                record.evidence_version,
                payload,
            ],
        )
        .map_err(|error| SecurityExecutionStoreError::WritePositionPlan(error.to_string()))?;
    Ok(())
}

pub(crate) fn load_security_position_plan_record(
    connection: &Connection,
    position_plan_ref: &str,
) -> Result<Option<SecurityPositionPlanRecordResult>, SecurityExecutionStoreError> {
    let mut statement = connection
        .prepare(
            "SELECT payload_json
             FROM security_position_plan_records
             WHERE position_plan_ref = ?1
             LIMIT 1",
        )
        .map_err(|error| SecurityExecutionStoreError::ReadPositionPlan(error.to_string()))?;
    let mut rows = statement
        .query(params![position_plan_ref])
        .map_err(|error| SecurityExecutionStoreError::ReadPositionPlan(error.to_string()))?;

    let Some(row) = rows
        .next()
        .map_err(|error| SecurityExecutionStoreError::ReadPositionPlan(error.to_string()))?
    else {
        return Ok(None);
    };

    let payload: String = row
        .get(0)
        .map_err(|error| SecurityExecutionStoreError::ReadPositionPlan(error.to_string()))?;
    serde_json::from_str::<SecurityPositionPlanRecordResult>(&payload)
        .map(Some)
        .map_err(|error| SecurityExecutionStoreError::DeserializePayload(error.to_string()))
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::{load_security_position_plan_record, upsert_security_position_plan_record};
    use crate::ops::stock::security_decision_briefing::PositionPlan;
    use crate::runtime::security_execution_store_schema::bootstrap_security_execution_schema;
    use crate::tools::contracts::SecurityPositionPlanRecordResult;

    #[test]
    fn position_plan_repository_round_trips_record() {
        let connection = Connection::open_in_memory().expect("in-memory sqlite should open");
        bootstrap_security_execution_schema(&connection)
            .expect("execution schema should bootstrap");
        let record = SecurityPositionPlanRecordResult {
            position_plan_ref: "plan-ref-1".to_string(),
            decision_ref: "decision-1".to_string(),
            approval_ref: "approval-1".to_string(),
            evidence_version: "v1".to_string(),
            symbol: "601916.SH".to_string(),
            analysis_date: "2026-04-15".to_string(),
            position_action: "build".to_string(),
            starter_position_pct: 0.05,
            max_position_pct: 0.12,
            position_plan: PositionPlan::default(),
        };

        upsert_security_position_plan_record(&connection, &record)
            .expect("position plan record should persist");
        let loaded = load_security_position_plan_record(&connection, "plan-ref-1")
            .expect("position plan record should load")
            .expect("position plan record should exist");

        assert_eq!(loaded, record);
    }
}
