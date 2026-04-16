use rusqlite::{Connection, params};

use crate::runtime::security_execution_store::SecurityExecutionStoreError;
use crate::tools::contracts::SecurityRecordPositionAdjustmentResult;

// 2026-04-15 CST: Extracted from security_execution_store.rs because round 2
// plan B now needs adjustment-event persistence isolated from the store facade.
// Purpose: keep formal adjustment-event SQL on one dedicated repository
// boundary while the store facade remains only the runtime entry surface.
pub(crate) fn upsert_security_adjustment_event(
    connection: &Connection,
    record: &SecurityRecordPositionAdjustmentResult,
) -> Result<(), SecurityExecutionStoreError> {
    let payload = serde_json::to_string(record)
        .map_err(|error| SecurityExecutionStoreError::SerializePayload(error.to_string()))?;
    connection
        .execute(
            "INSERT INTO security_position_adjustment_events (
                adjustment_event_ref,
                position_plan_ref,
                symbol,
                event_date,
                decision_ref,
                approval_ref,
                evidence_version,
                payload_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(adjustment_event_ref) DO UPDATE SET
                position_plan_ref = excluded.position_plan_ref,
                symbol = excluded.symbol,
                event_date = excluded.event_date,
                decision_ref = excluded.decision_ref,
                approval_ref = excluded.approval_ref,
                evidence_version = excluded.evidence_version,
                payload_json = excluded.payload_json,
                updated_at = CURRENT_TIMESTAMP",
            params![
                record.adjustment_event_ref,
                record.position_plan_ref,
                record.symbol,
                record.event_date,
                record.decision_ref,
                record.approval_ref,
                record.evidence_version,
                payload,
            ],
        )
        .map_err(|error| SecurityExecutionStoreError::WriteAdjustmentEvent(error.to_string()))?;
    Ok(())
}

pub(crate) fn load_security_adjustment_event(
    connection: &Connection,
    adjustment_event_ref: &str,
) -> Result<Option<SecurityRecordPositionAdjustmentResult>, SecurityExecutionStoreError> {
    let mut statement = connection
        .prepare(
            "SELECT payload_json
             FROM security_position_adjustment_events
             WHERE adjustment_event_ref = ?1
             LIMIT 1",
        )
        .map_err(|error| SecurityExecutionStoreError::ReadAdjustmentEvent(error.to_string()))?;
    let mut rows = statement
        .query(params![adjustment_event_ref])
        .map_err(|error| SecurityExecutionStoreError::ReadAdjustmentEvent(error.to_string()))?;

    let Some(row) = rows
        .next()
        .map_err(|error| SecurityExecutionStoreError::ReadAdjustmentEvent(error.to_string()))?
    else {
        return Ok(None);
    };

    let payload: String = row
        .get(0)
        .map_err(|error| SecurityExecutionStoreError::ReadAdjustmentEvent(error.to_string()))?;
    serde_json::from_str::<SecurityRecordPositionAdjustmentResult>(&payload)
        .map(Some)
        .map_err(|error| SecurityExecutionStoreError::DeserializePayload(error.to_string()))
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::{load_security_adjustment_event, upsert_security_adjustment_event};
    use crate::runtime::security_execution_store_schema::bootstrap_security_execution_schema;
    use crate::tools::contracts::{
        PositionAdjustmentEventType, PositionPlanAlignment, SecurityRecordPositionAdjustmentResult,
    };

    #[test]
    fn adjustment_event_repository_round_trips_record() {
        let connection = Connection::open_in_memory().expect("in-memory sqlite should open");
        bootstrap_security_execution_schema(&connection)
            .expect("execution schema should bootstrap");
        let record = SecurityRecordPositionAdjustmentResult {
            adjustment_event_ref: "adjustment-1".to_string(),
            decision_ref: "decision-1".to_string(),
            approval_ref: "approval-1".to_string(),
            evidence_version: "v1".to_string(),
            position_plan_ref: "plan-ref-1".to_string(),
            symbol: "601916.SH".to_string(),
            event_type: PositionAdjustmentEventType::Add,
            event_date: "2026-04-15".to_string(),
            before_position_pct: 0.05,
            after_position_pct: 0.08,
            trigger_reason: "trend_confirmation".to_string(),
            plan_alignment: PositionPlanAlignment::OnPlan,
        };

        upsert_security_adjustment_event(&connection, &record)
            .expect("adjustment event should persist");
        let loaded = load_security_adjustment_event(&connection, "adjustment-1")
            .expect("adjustment event should load")
            .expect("adjustment event should exist");

        assert_eq!(loaded, record);
    }
}
