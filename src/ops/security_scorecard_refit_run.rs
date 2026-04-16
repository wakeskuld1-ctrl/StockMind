use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::ops::stock::security_scorecard_model_registry::{
    SecurityScorecardCandidateArtifactInput, SecurityScorecardModelRegistry,
    SecurityScorecardModelRegistryError, build_security_scorecard_model_registry,
    sanitize_identifier,
};

// 2026-04-09 CST: 这里新增 refit Tool 请求合同，原因是 Task 4 需要把“离线重估一次”正式收口成稳定接口，而不是让外层脚本手拼对象；
// 目的：把市场范围、样本窗口、标签版本和 candidate artifact 一次性冻结下来，形成后续训练发布链的最小正式入口。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityScorecardRefitRequest {
    #[serde(default = "default_created_at")]
    pub created_at: String,
    #[serde(default)]
    pub refit_runtime_root: Option<String>,
    pub market_scope: String,
    pub instrument_scope: String,
    pub feature_set_version: String,
    pub label_definition_version: String,
    pub train_range: String,
    pub valid_range: String,
    pub test_range: String,
    pub candidate_artifact: SecurityScorecardCandidateArtifactInput,
    #[serde(default)]
    pub comparison_to_champion_json: Option<Value>,
    #[serde(default)]
    pub promotion_decision: Option<String>,
}

// 2026-04-09 CST: 这里新增 refit_run 正式对象，原因是 Task 4 的核心是把一次重估任务从口头流程升级为可落盘的治理记录；
// 目的：显式记录训练/验证/测试窗口、特征与标签版本、候选 artifact 路径和后续晋级所需预留字段。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityScorecardRefitRun {
    pub refit_run_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub market_scope: String,
    pub instrument_scope: String,
    pub feature_set_version: String,
    pub label_definition_version: String,
    pub train_range: String,
    pub valid_range: String,
    pub test_range: String,
    pub candidate_artifact_path: String,
    #[serde(default)]
    pub candidate_registry_ref: Option<String>,
    #[serde(default)]
    pub comparison_to_champion_json: Option<Value>,
    #[serde(default)]
    pub promotion_decision: Option<String>,
    pub model_grade: String,
    pub created_at: String,
}

// 2026-04-09 CST: 这里定义 refit Tool 聚合返回对象，原因是调用方不仅需要看见 run 与 registry，还需要直接拿到持久化路径；
// 目的：让 CLI / Skill / 后续编排可以在一次调用后立刻继续挂接 decision package 或更下游治理动作。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityScorecardRefitResult {
    pub refit_run: SecurityScorecardRefitRun,
    pub model_registry: SecurityScorecardModelRegistry,
    pub refit_run_path: String,
    pub model_registry_path: String,
}

// 2026-04-09 CST: 这里集中定义 refit Tool 错误边界，原因是该能力同时包含对象构建、registry 登记和文件持久化；
// 目的：向 dispatcher 暴露稳定错误口径，便于后续训练流水线复用与排障。
#[derive(Debug, Error)]
pub enum SecurityScorecardRefitError {
    #[error("security scorecard refit build failed: {0}")]
    Build(String),
    #[error("security scorecard model registry build failed: {0}")]
    Registry(#[from] SecurityScorecardModelRegistryError),
    #[error("security scorecard refit persist failed: {0}")]
    Persist(String),
}

// 2026-04-09 CST: 这里实现 Task 4 的最小正式入口，原因是我们先需要落“治理对象 + 注册表”，而不是提前混入真实训练逻辑；
// 目的：把 candidate artifact 注册、refit_run 生成和磁盘持久化整合成一条稳定主链，直接承接后续 Task 5。
pub fn security_scorecard_refit(
    request: &SecurityScorecardRefitRequest,
) -> Result<SecurityScorecardRefitResult, SecurityScorecardRefitError> {
    validate_request(request)?;

    let model_registry = build_security_scorecard_model_registry(
        &request.market_scope,
        &request.instrument_scope,
        &request.train_range,
        &request.valid_range,
        &request.test_range,
        &build_candidate_with_grade(request),
    )?;
    let refit_run = build_security_scorecard_refit_run(request, &model_registry);

    let runtime_root = resolve_runtime_root(request);
    let refit_run_path = runtime_root.join("scorecard_refit_runs").join(format!(
        "{}.json",
        sanitize_identifier(&refit_run.refit_run_id)
    ));
    let model_registry_path = runtime_root.join("scorecard_model_registry").join(format!(
        "{}__{}.json",
        sanitize_identifier(&model_registry.model_id),
        sanitize_identifier(&model_registry.model_version)
    ));

    persist_json(&refit_run_path, &refit_run)?;
    persist_json(&model_registry_path, &model_registry)?;

    Ok(SecurityScorecardRefitResult {
        refit_run,
        model_registry,
        refit_run_path: refit_run_path.to_string_lossy().to_string(),
        model_registry_path: model_registry_path.to_string_lossy().to_string(),
    })
}

// 2026-04-09 CST: 这里单独构建 refit_run 对象，原因是 run 主对象和 registry 对象虽然相关，但治理语义不同；
// 目的：保持“任务记录”和“模型登记”两个正式对象的职责分离，符合后续 champion/challenger 扩展的对象边界。
fn build_security_scorecard_refit_run(
    request: &SecurityScorecardRefitRequest,
    model_registry: &SecurityScorecardModelRegistry,
) -> SecurityScorecardRefitRun {
    SecurityScorecardRefitRun {
        refit_run_id: format!(
            "refit-{}-{}-{}",
            sanitize_identifier(&request.market_scope),
            sanitize_identifier(&request.instrument_scope),
            sanitize_identifier(&request.created_at),
        ),
        contract_version: "security_scorecard_refit_run.v1".to_string(),
        document_type: "security_scorecard_refit_run".to_string(),
        market_scope: request.market_scope.clone(),
        instrument_scope: request.instrument_scope.clone(),
        feature_set_version: request.feature_set_version.clone(),
        label_definition_version: request.label_definition_version.clone(),
        train_range: request.train_range.clone(),
        valid_range: request.valid_range.clone(),
        test_range: request.test_range.clone(),
        candidate_artifact_path: request.candidate_artifact.artifact_path.clone(),
        candidate_registry_ref: Some(model_registry.registry_id.clone()),
        comparison_to_champion_json: request.comparison_to_champion_json.clone(),
        promotion_decision: request.promotion_decision.clone(),
        model_grade: model_registry.model_grade.clone(),
        created_at: request.created_at.clone(),
    }
}

// 2026-04-11 CST: Normalize model-grade semantics before registry persistence,
// because P5 needs refit to publish explicit candidate/shadow/champion state
// instead of leaving downstream consumers to decode raw promotion strings.
// Purpose: keep registry and refit outputs aligned on one governed grade vocabulary.
fn build_candidate_with_grade(
    request: &SecurityScorecardRefitRequest,
) -> SecurityScorecardCandidateArtifactInput {
    let mut candidate = request.candidate_artifact.clone();
    let (model_grade, grade_reason) = match request.promotion_decision.as_deref().map(str::trim) {
        Some("shadow") => (
            "shadow".to_string(),
            "promoted_by_refit_decision".to_string(),
        ),
        Some("champion") => (
            "champion".to_string(),
            "promoted_by_refit_decision".to_string(),
        ),
        _ => ("candidate".to_string(), "retained_as_candidate".to_string()),
    };
    candidate.model_grade = model_grade;
    candidate.grade_reason = grade_reason;
    candidate
}

// 2026-04-09 CST: 这里在入口层集中做字段校验，原因是 Task 4 先要确保治理对象边界稳定，再谈后续训练填充；
// 目的：避免空窗口、空范围或空版本字段直接写成形式上存在但治理上无意义的正式对象。
fn validate_request(
    request: &SecurityScorecardRefitRequest,
) -> Result<(), SecurityScorecardRefitError> {
    for (field_name, field_value) in [
        ("market_scope", request.market_scope.trim()),
        ("instrument_scope", request.instrument_scope.trim()),
        ("feature_set_version", request.feature_set_version.trim()),
        (
            "label_definition_version",
            request.label_definition_version.trim(),
        ),
        ("train_range", request.train_range.trim()),
        ("valid_range", request.valid_range.trim()),
        ("test_range", request.test_range.trim()),
    ] {
        if field_value.is_empty() {
            return Err(SecurityScorecardRefitError::Build(format!(
                "{field_name} cannot be empty"
            )));
        }
    }
    Ok(())
}

// 2026-04-09 CST: 这里统一持久化 refit 相关正式对象，原因是 run 与 registry 都需要稳定落盘供后续训练发布链消费；
// 目的：避免上层再重复拼目录、建父目录和处理 JSON 序列化错误。
fn persist_json<T: Serialize>(path: &Path, value: &T) -> Result<(), SecurityScorecardRefitError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| SecurityScorecardRefitError::Persist(error.to_string()))?;
    }
    let payload = serde_json::to_vec_pretty(value)
        .map_err(|error| SecurityScorecardRefitError::Persist(error.to_string()))?;
    fs::write(path, payload)
        .map_err(|error| SecurityScorecardRefitError::Persist(error.to_string()))
}

// 2026-04-09 CST: 这里统一解析 refit runtime 根目录，原因是测试、CLI 与未来离线训练批处理都会需要可覆盖的输出根路径；
// 目的：让 Task 4 可以先在显式目录落盘，同时保留与现有 scenes runtime 一致的默认收敛位置。
fn resolve_runtime_root(request: &SecurityScorecardRefitRequest) -> PathBuf {
    request
        .refit_runtime_root
        .as_ref()
        .map(|path| PathBuf::from(path.trim()))
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| {
            PathBuf::from(".worktrees")
                .join("SheetMind-Scenes-inspect")
                .join(".sheetmind_scenes_runtime")
        })
}

fn default_created_at() -> String {
    chrono::Utc::now().to_rfc3339()
}
