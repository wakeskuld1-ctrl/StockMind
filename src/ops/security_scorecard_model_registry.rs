use std::fs;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

// 2026-04-09 CST: 这里新增 scorecard model registry 正式合同，原因是 Task 4 需要把 candidate artifact 从“临时文件”升级为可治理、可追溯的正式注册对象；
// 目的：让后续训练入口、champion/challenger 比较和线上消费都能基于稳定字段追踪模型来源，而不是依赖外层脚本约定。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityScorecardModelRegistry {
    pub registry_id: String,
    // 2026-04-11 CST: Added serde defaults for backward-compatible registry loading,
    // because P5 submit_approval now consumes historical registry fixtures that may
    // not carry the newly formalized governance metadata yet.
    // Purpose: keep old registry documents readable while the approval chain upgrades
    // to model-grade-aware governance semantics.
    #[serde(default = "default_contract_version")]
    pub contract_version: String,
    pub document_type: String,
    pub model_id: String,
    pub market_scope: String,
    pub instrument_scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument_subscope: Option<String>,
    pub horizon_days: usize,
    pub target_head: String,
    pub model_version: String,
    pub status: String,
    #[serde(default = "default_model_grade")]
    pub model_grade: String,
    #[serde(default = "default_grade_reason")]
    pub grade_reason: String,
    // 2026-04-11 CST: Added empty-window defaults for legacy registry fixtures,
    // because older candidate files were created before training/validation/oot
    // windows became mandatory in the governed registry schema.
    // Purpose: let submit_approval and later promotion tools read legacy fixtures
    // without forcing every old runtime artifact to be rewritten first.
    #[serde(default)]
    pub training_window: String,
    #[serde(default)]
    pub validation_window: String,
    #[serde(default)]
    pub oot_window: String,
    pub artifact_path: String,
    pub artifact_sha256: String,
    pub metrics_summary_json: Value,
    pub published_at: Option<String>,
}

// 2026-04-09 CST: 这里定义 candidate artifact 请求合同，原因是 refit 工具需要显式接收候选模型治理元数据，而不是猜测 artifact 内容或硬编码版本；
// 目的：把 Task 4 的最小正式输入边界固定下来，让 Task 5 训练入口以后只负责填充它。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityScorecardCandidateArtifactInput {
    pub model_id: String,
    pub model_version: String,
    pub horizon_days: usize,
    pub target_head: String,
    #[serde(default = "default_registry_status")]
    pub status: String,
    pub artifact_path: String,
    #[serde(default)]
    pub metrics_summary_json: Value,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub instrument_subscope: Option<String>,
    #[serde(default = "default_model_grade")]
    pub model_grade: String,
    #[serde(default = "default_grade_reason")]
    pub grade_reason: String,
}

// 2026-04-09 CST: 这里集中定义 registry 构建错误，原因是 artifact 哈希登记既涉及字段校验也涉及文件读取；
// 目的：让 refit 工具向 dispatcher 暴露稳定、可定位的错误，而不是把底层 I/O 细节直接泄露到上层。
#[derive(Debug, Error)]
pub enum SecurityScorecardModelRegistryError {
    #[error("security scorecard model registry build failed: {0}")]
    Build(String),
}

// 2026-04-09 CST: 这里集中构建单个 candidate registry 对象，原因是 Task 4 先需要正式登记一份候选 artifact，而不是提前展开完整训练流程；
// 目的：把 artifact 路径、sha、窗口和目标头绑定成一条可落盘、可审计、可升级的注册记录。
pub fn build_security_scorecard_model_registry(
    market_scope: &str,
    instrument_scope: &str,
    training_window: &str,
    validation_window: &str,
    oot_window: &str,
    candidate: &SecurityScorecardCandidateArtifactInput,
) -> Result<SecurityScorecardModelRegistry, SecurityScorecardModelRegistryError> {
    let artifact_path = candidate.artifact_path.trim();
    if artifact_path.is_empty() {
        return Err(SecurityScorecardModelRegistryError::Build(
            "candidate artifact path cannot be empty".to_string(),
        ));
    }

    let artifact_payload = fs::read(artifact_path).map_err(|error| {
        SecurityScorecardModelRegistryError::Build(format!(
            "failed to read candidate artifact `{artifact_path}`: {error}"
        ))
    })?;
    let artifact_sha256 = sha256_for_bytes(&artifact_payload);

    Ok(SecurityScorecardModelRegistry {
        registry_id: format!(
            "registry-{}-{}-{}d-{}",
            sanitize_identifier(&candidate.model_id),
            sanitize_identifier(&candidate.model_version),
            candidate.horizon_days,
            sanitize_identifier(&candidate.target_head),
        ),
        contract_version: "security_scorecard_model_registry.v1".to_string(),
        document_type: "security_scorecard_model_registry".to_string(),
        model_id: candidate.model_id.clone(),
        market_scope: market_scope.to_string(),
        instrument_scope: instrument_scope.to_string(),
        instrument_subscope: candidate.instrument_subscope.clone(),
        horizon_days: candidate.horizon_days,
        target_head: candidate.target_head.clone(),
        model_version: candidate.model_version.clone(),
        status: candidate.status.clone(),
        model_grade: candidate.model_grade.clone(),
        grade_reason: candidate.grade_reason.clone(),
        training_window: training_window.to_string(),
        validation_window: validation_window.to_string(),
        oot_window: oot_window.to_string(),
        artifact_path: artifact_path.to_string(),
        artifact_sha256,
        metrics_summary_json: candidate.metrics_summary_json.clone(),
        published_at: candidate.published_at.clone(),
    })
}

// 2026-04-09 CST: 这里统一做 artifact 哈希，原因是后续 champion/challenger 治理和线上消费都要基于稳定摘要比较模型版本；
// 目的：避免不同调用方重复实现哈希口径，导致同一 artifact 产生不一致登记结果。
fn sha256_for_bytes(payload: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(payload);
    format!("{:x}", hasher.finalize())
}

// 2026-04-09 CST: 这里统一清洗 registry 标识片段，原因是 model_id / version / target_head 未来会进入文件名和 ref 字段；
// 目的：避免路径字符漂移影响持久化或后续引用稳定性。
pub fn sanitize_identifier(raw: &str) -> String {
    raw.chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch,
            _ => '_',
        })
        .collect()
}

fn default_registry_status() -> String {
    "candidate".to_string()
}

fn default_contract_version() -> String {
    "security_scorecard_model_registry.v1".to_string()
}

fn default_model_grade() -> String {
    "candidate".to_string()
}

fn default_grade_reason() -> String {
    "retained_as_candidate".to_string()
}
