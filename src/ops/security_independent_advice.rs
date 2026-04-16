use chrono::Utc;
use serde::{Deserialize, Serialize};

// 2026-04-13 CST: 这里新增独立建议 Tool 请求合同，原因是方案B要求把“大模型独立建议线”从主席请求内嵌字段升级成单独可发现、可复用的正式入口；
// 目的：让后续 Skill、主席裁决和会后治理都消费同一份标准独立建议文档，而不是继续手工拼嵌套对象。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityIndependentAdviceRequest {
    pub symbol: String,
    pub analysis_date: String,
    pub source_type: String,
    pub suggested_stance: String,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub rationale: Option<String>,
    #[serde(default)]
    pub key_risks: Vec<String>,
    #[serde(default)]
    pub evidence_basis: Vec<String>,
    #[serde(default = "default_generated_at")]
    pub generated_at: String,
}

// 2026-04-13 CST: 这里冻结独立建议正式文档合同，原因是主席层后续必须消费标准化对象，而不是自由文本；
// 目的：把“独立建议”沉淀成可落盘、可审计、可回放的正式证券治理对象。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityIndependentAdviceDocument {
    pub advice_id: String,
    pub contract_version: String,
    pub document_type: String,
    pub generated_at: String,
    pub symbol: String,
    pub analysis_date: String,
    pub source_type: String,
    pub suggested_stance: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    pub key_risks: Vec<String>,
    pub evidence_basis: Vec<String>,
    pub advice_summary: String,
}

// 2026-04-13 CST: 这里集中构造独立建议文档，原因是后续主席线、投后线和上层 Skill 都需要同一份稳定对象；
// 目的：把建议摘要、默认时间和列表去重收口到单点，避免各链路重复写映射。
pub fn security_independent_advice(
    request: &SecurityIndependentAdviceRequest,
) -> SecurityIndependentAdviceDocument {
    let mut key_risks = normalize_lines(&request.key_risks);
    let mut evidence_basis = normalize_lines(&request.evidence_basis);
    if key_risks.is_empty() {
        key_risks.push("独立建议未显式提供关键风险，请主席层谨慎采用".to_string());
    }
    if evidence_basis.is_empty() {
        evidence_basis.push("独立建议未附带额外证据清单".to_string());
    }

    let rationale = request
        .rationale
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let summary = build_advice_summary(
        &request.source_type,
        &request.suggested_stance,
        request.confidence,
        rationale.as_deref(),
    );

    SecurityIndependentAdviceDocument {
        advice_id: format!(
            "independent-advice-{}-{}-{}",
            request.symbol,
            request.analysis_date,
            request.source_type.trim()
        ),
        contract_version: "security_independent_advice.v1".to_string(),
        document_type: "security_independent_advice".to_string(),
        generated_at: normalize_generated_at(&request.generated_at),
        symbol: request.symbol.trim().to_string(),
        analysis_date: request.analysis_date.trim().to_string(),
        source_type: request.source_type.trim().to_string(),
        suggested_stance: request.suggested_stance.trim().to_string(),
        confidence: request.confidence.map(|value| value.clamp(0.0, 1.0)),
        rationale,
        key_risks,
        evidence_basis,
        advice_summary: summary,
    }
}

fn build_advice_summary(
    source_type: &str,
    suggested_stance: &str,
    confidence: Option<f64>,
    rationale: Option<&str>,
) -> String {
    let confidence_text = confidence
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "unknown".to_string());
    let rationale_text = rationale.unwrap_or("未提供额外解释");
    format!(
        "独立建议源 `{}` 对当前标的给出 `{}` 立场，置信度 `{}`，理由：{}。",
        source_type.trim(),
        suggested_stance.trim(),
        confidence_text,
        rationale_text
    )
}

fn normalize_lines(values: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for item in values {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !normalized.iter().any(|existing| existing == trimmed) {
            normalized.push(trimmed.to_string());
        }
    }
    normalized
}

fn normalize_generated_at(value: &str) -> String {
    if value.trim().is_empty() {
        Utc::now().to_rfc3339()
    } else {
        value.trim().to_string()
    }
}

fn default_generated_at() -> String {
    Utc::now().to_rfc3339()
}
