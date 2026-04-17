use std::path::{Path, PathBuf};

use rusqlite::Connection;
use thiserror::Error;

use crate::ops::stock::security_execution_record::SecurityExecutionRecordDocument;
use crate::runtime::formal_security_runtime_registry::FormalSecurityRuntimeRegistry;
use crate::runtime::security_execution_store_connection::open_security_execution_store_connection;
use crate::runtime::security_execution_store_repository_context::SecurityExecutionStoreRepositoryContext;
use crate::runtime::security_execution_store_session::SecurityExecutionStoreSession;
use crate::tools::contracts::{
    SecurityPositionPlanRecordResult, SecurityRecordPositionAdjustmentResult,
};

// 2026-04-08 CST: 这里新增证券执行层 runtime store，原因是仓位计划、调仓事件与投后复盘已经形成正式 ref 链；
// 目的：把 plan / adjustment 的落盘与回读统一收口到独立存储层，避免各个 Tool 各自手写 JSON 文件或重复拼接查询逻辑。
#[derive(Debug, Clone)]
pub struct SecurityExecutionStore {
    db_path: PathBuf,
}

// 2026-04-08 CST: 这里集中定义执行层存储错误，原因是 Task 6 同时涉及建库、写入、回读和 JSON 反序列化；
// 目的：为上层 Tool 返回清晰中文错误，并把执行链存储问题和业务规则问题明确分开。
#[derive(Debug, Error)]
pub enum SecurityExecutionStoreError {
    #[error("无法确定证券执行层 SQLite 所在目录: {0}")]
    ResolveRuntimeDir(String),
    #[error("无法创建证券执行层 SQLite 目录: {0}")]
    CreateRuntimeDir(String),
    #[error("无法打开证券执行层 SQLite: {0}")]
    OpenDatabase(String),
    #[error("无法初始化证券执行层表结构: {0}")]
    BootstrapSchema(String),
    #[error("无法写入仓位计划记录: {0}")]
    WritePositionPlan(String),
    #[error("无法读取仓位计划记录: {0}")]
    ReadPositionPlan(String),
    #[error("无法写入调仓事件记录: {0}")]
    WriteAdjustmentEvent(String),
    #[error("无法读取调仓事件记录: {0}")]
    ReadAdjustmentEvent(String),
    #[error("无法写入执行记录: {0}")]
    WriteExecutionRecord(String),
    #[error("无法读取执行记录: {0}")]
    ReadExecutionRecord(String),
    #[error("鏃犳硶寮€鍚墽琛屽眰浜嬪姟浼氳瘽: {0}")]
    BeginTransaction(String),
    #[error("鏃犳硶鎻愪氦鎵ц灞備簨鍔′細璇? {0}")]
    CommitTransaction(String),
    #[error("鏃犳硶鍥炴粴鎵ц灞備簨鍔′細璇? {0}")]
    RollbackTransaction(String),
    #[error("无法序列化证券执行层对象: {0}")]
    SerializePayload(String),
    #[error("无法反序列化证券执行层对象: {0}")]
    DeserializePayload(String),
}

impl SecurityExecutionStore {
    // 2026-04-08 CST: 这里允许显式指定执行层数据库路径，原因是测试隔离和后续多环境部署都可能需要自定义路径；
    // 目的：让执行层存储既能跟随 workspace 默认 runtime，也能在定向测试里落到独立目录。
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    // 2026-04-08 CST: 这里提供默认执行层数据库入口，原因是 plan / adjustment / review Tool 都需要共享同一份执行链事实源；
    // 目的：让三个 Tool 自动收敛到统一 runtime，而不是每个 Tool 单独维护自己的落盘路径。
    pub fn workspace_default() -> Result<Self, SecurityExecutionStoreError> {
        Ok(Self::new(
            FormalSecurityRuntimeRegistry::execution_store_db_path()
                .map_err(SecurityExecutionStoreError::ResolveRuntimeDir)?,
        ))
    }

    // 2026-04-08 CST: 这里暴露数据库路径，原因是后续测试和排障都需要确认执行链是否真实落盘；
    // 目的：让上层在必要时能直接核对 runtime 文件位置，减少“到底写到哪里去了”的排障成本。
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    // 2026-04-08 CST: 这里统一打开执行层连接并自动建表，原因是当前三个 Tool 都应该共享同一套初始化逻辑；
    // 目的：避免 plan / adjustment / review 各自复制建库代码，确保 schema 只维护一份。
    pub fn open_connection(&self) -> Result<Connection, SecurityExecutionStoreError> {
        open_security_execution_store_connection(&self.db_path)
    }

    // 2026-04-15 CST: Added because the next verified bottom-layer step needs
    // one explicit transaction/session boundary without changing caller-facing
    // store naming.
    // Purpose: let future multi-step writes share one governed SQLite
    // transaction while keeping the facade as the only public runtime entry.
    pub(crate) fn open_session(
        &self,
    ) -> Result<SecurityExecutionStoreSession, SecurityExecutionStoreError> {
        let connection = self.open_connection()?;
        SecurityExecutionStoreSession::new(connection)
    }

    // 2026-04-15 CST: Added because round 2 plan B now needs a formal internal
    // repository boundary after the facade opens SQLite.
    // Purpose: keep repository dispatch centralized here so later transaction or
    // session abstractions can extend one path without breaking callers.
    fn with_repository_context<T, F>(&self, action: F) -> Result<T, SecurityExecutionStoreError>
    where
        F: FnOnce(
            &SecurityExecutionStoreRepositoryContext<'_>,
        ) -> Result<T, SecurityExecutionStoreError>,
    {
        let connection = self.open_connection()?;
        let context = SecurityExecutionStoreRepositoryContext::new(&connection);
        action(&context)
    }

    // 2026-04-08 CST: 这里补仓位计划记录落盘，原因是投后复盘必须能只凭 position_plan_ref 回读正式计划对象；
    // 目的：让 position_plan_record 不再只是回声式 Tool，而是真正成为后续执行与复盘的锚点。
    pub fn upsert_position_plan(
        &self,
        record: &SecurityPositionPlanRecordResult,
    ) -> Result<(), SecurityExecutionStoreError> {
        self.with_repository_context(|context| context.upsert_position_plan(record))
    }

    // 2026-04-08 CST: 这里补仓位计划回读，原因是 post_trade_review 需要只凭 ref 从执行层恢复正式计划事实；
    // 目的：让复盘链路不依赖调用方重复携带完整计划 payload，保持正式引用对象语义。
    pub fn load_position_plan(
        &self,
        position_plan_ref: &str,
    ) -> Result<Option<SecurityPositionPlanRecordResult>, SecurityExecutionStoreError> {
        self.with_repository_context(|context| context.load_position_plan(position_plan_ref))
    }

    // 2026-04-08 CST: 这里补调仓事件落盘，原因是投后复盘要从 adjustment_event_ref 链接回每次实际执行动作；
    // 目的：让 security_record_position_adjustment 产出的正式事件对象能被后续聚合与审计反查，而不是停留在单次响应里。
    pub fn upsert_adjustment_event(
        &self,
        record: &SecurityRecordPositionAdjustmentResult,
    ) -> Result<(), SecurityExecutionStoreError> {
        self.with_repository_context(|context| context.upsert_adjustment_event(record))
    }

    // 2026-04-08 CST: 这里补调仓事件回读，原因是复盘 Tool 需要顺着多条 adjustment_event_ref 回收完整事件链；
    // 目的：把事件恢复逻辑集中在存储层，避免复盘 Tool 再自己拼 SQL 或手工解析落盘 JSON。
    pub fn load_adjustment_event(
        &self,
        adjustment_event_ref: &str,
    ) -> Result<Option<SecurityRecordPositionAdjustmentResult>, SecurityExecutionStoreError> {
        self.with_repository_context(|context| context.load_adjustment_event(adjustment_event_ref))
    }

    // 2026-04-14 CST: 这里补回 execution_record 落盘入口，原因是 execution_record 主链已经正式将聚合执行事实写入 runtime；
    // 目的：让账户 open snapshot 与 review/package 可以继续共享同一份执行事实源。
    pub fn upsert_execution_record(
        &self,
        record: &SecurityExecutionRecordDocument,
    ) -> Result<(), SecurityExecutionStoreError> {
        self.with_repository_context(|context| context.upsert_execution_record(record))
    }

    // 2026-04-14 CST: 这里补回账户层 open execution 查询，原因是 account_open_position_snapshot 已经正式依赖该 runtime 读取；
    // 目的：让账户层只读取聚合执行事实，不再自己重扫 journal 或额外拼装临时状态。
    pub fn load_latest_open_execution_records(
        &self,
        account_id: &str,
    ) -> Result<Vec<SecurityExecutionRecordDocument>, SecurityExecutionStoreError> {
        self.with_repository_context(|context| {
            context.load_latest_open_execution_records(account_id)
        })
    }

    // 2026-04-17 CST: Reason=lifecycle follow-up tools need a stable ref-based
    // lookup path after execution_record persists its governed output.
    // Purpose=let post_trade_review and later lifecycle consumers reopen one record by id.
    pub fn load_execution_record(
        &self,
        execution_record_id: &str,
    ) -> Result<Option<SecurityExecutionRecordDocument>, SecurityExecutionStoreError> {
        self.with_repository_context(|context| context.load_execution_record(execution_record_id))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::SecurityExecutionStore;
    use crate::ops::stock::security_decision_briefing::PositionPlan;
    use crate::ops::stock::security_execution_record::SecurityExecutionRecordDocument;
    use crate::tools::contracts::{
        PositionAdjustmentEventType, PositionPlanAlignment, SecurityPositionPlanRecordResult,
        SecurityRecordPositionAdjustmentResult,
    };

    // 2026-04-15 CST: Added because the facade layer now routes through a
    // repository context and still needs one file-backed verification path.
    // Purpose: keep runtime facade tests isolated without introducing a new
    // dev dependency just for temporary directories.
    fn build_temp_db_path(test_name: &str) -> PathBuf {
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let base_dir = std::env::temp_dir()
            .join("excel_skill_security_execution_store_tests")
            .join(format!("{test_name}_{unique_suffix}"));
        fs::create_dir_all(&base_dir).expect("temporary test directory should be created");
        base_dir.join("security_execution.db")
    }

    #[test]
    fn security_execution_store_facade_round_trips_plan_and_adjustment_records() {
        let db_path = build_temp_db_path("facade_plan_adjustment");
        let store = SecurityExecutionStore::new(db_path.clone());
        let position_plan = SecurityPositionPlanRecordResult {
            position_plan_ref: "plan-facade-1".to_string(),
            decision_ref: "decision-facade-1".to_string(),
            approval_ref: "approval-facade-1".to_string(),
            evidence_version: "v1".to_string(),
            symbol: "601916.SH".to_string(),
            analysis_date: "2026-04-15".to_string(),
            position_action: "build".to_string(),
            starter_position_pct: 0.05,
            max_position_pct: 0.12,
            position_plan: PositionPlan::default(),
        };
        let adjustment = SecurityRecordPositionAdjustmentResult {
            adjustment_event_ref: "adjustment-facade-1".to_string(),
            decision_ref: "decision-facade-1".to_string(),
            approval_ref: "approval-facade-1".to_string(),
            evidence_version: "v1".to_string(),
            position_plan_ref: "plan-facade-1".to_string(),
            symbol: "601916.SH".to_string(),
            event_type: PositionAdjustmentEventType::Add,
            event_date: "2026-04-15".to_string(),
            before_position_pct: 0.05,
            after_position_pct: 0.08,
            trigger_reason: "trend_confirmation".to_string(),
            plan_alignment: PositionPlanAlignment::OnPlan,
        };

        store
            .upsert_position_plan(&position_plan)
            .expect("position plan should persist through facade");
        store
            .upsert_adjustment_event(&adjustment)
            .expect("adjustment event should persist through facade");

        assert_eq!(
            store
                .load_position_plan("plan-facade-1")
                .expect("position plan should load through facade"),
            Some(position_plan)
        );
        assert_eq!(
            store
                .load_adjustment_event("adjustment-facade-1")
                .expect("adjustment event should load through facade"),
            Some(adjustment)
        );
        assert!(
            db_path.exists(),
            "facade test should create the SQLite file"
        );

        let _ = fs::remove_file(&db_path);
        let _ = db_path.parent().map(fs::remove_dir_all).transpose();
    }

    #[test]
    fn security_execution_store_facade_round_trips_open_execution_records() {
        let db_path = build_temp_db_path("facade_execution_record");
        let store = SecurityExecutionStore::new(db_path.clone());
        let record = SecurityExecutionRecordDocument {
            execution_record_id: "record-facade-1".to_string(),
            contract_version: "security_execution_record.v1".to_string(),
            document_type: "security_execution_record".to_string(),
            generated_at: "2026-04-15T10:00:00+08:00".to_string(),
            symbol: "601916.SH".to_string(),
            analysis_date: "2026-04-15".to_string(),
            account_id: Some("acct-facade-1".to_string()),
            sector_tag: Some("bank".to_string()),
            position_state: "open".to_string(),
            portfolio_position_plan_ref: Some("portfolio-plan-facade-1".to_string()),
            execution_journal_ref: "journal-facade-1".to_string(),
            position_plan_ref: "plan-facade-1".to_string(),
            snapshot_ref: "snapshot-facade-1".to_string(),
            outcome_ref: "outcome-facade-1".to_string(),
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

        store
            .upsert_execution_record(&record)
            .expect("execution record should persist through facade");

        assert_eq!(
            store
                .load_latest_open_execution_records("acct-facade-1")
                .expect("execution records should load through facade"),
            vec![record]
        );
        assert!(
            db_path.exists(),
            "facade test should create the SQLite file"
        );

        let _ = fs::remove_file(&db_path);
        let _ = db_path.parent().map(fs::remove_dir_all).transpose();
    }
}

// 2026-04-08 CST: 这里集中维护执行层 schema，原因是仓位计划与调仓事件已经构成一条稳定执行链；
// 目的：让后续复盘聚合和审计回查都复用同一份表结构，而不是临时新增零散文件存储。
