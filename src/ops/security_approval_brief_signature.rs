use std::collections::BTreeMap;

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::ops::stock::security_decision_approval_brief::SecurityDecisionApprovalBrief;

type HmacSha256 = Hmac<Sha256>;

// 2026-04-02 CST: 这里定义审批简报签名 envelope，原因是正式审批文档需要独立签名工件，而不能把签名直接塞回正文合同；
// 目的：保持正文合同稳定，同时为后续审计与 package 集成提供 detached signature 锚点。
// 2026-04-02 CST: 这里补齐签名 envelope 的反序列化能力，原因是 P0-5 需要从落盘文件读取签名工件并执行验签；
// 目的：让 approval brief 的 detached signature 可以成为真正可验证的治理对象，而不只是可写出的 JSON 文件。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityApprovalBriefSignatureEnvelope {
    pub signature_version: String,
    pub algorithm: String,
    pub key_id: String,
    pub payload_sha256: String,
    pub contract_version: String,
    pub brief_id: String,
    pub signature: String,
}

// 2026-04-02 CST: 这里集中生成 detached signature，原因是 P0-3 需要让审批简报具备“可单独签名”的正式能力；
// 目的：复用 HMAC-SHA256 生成稳定 envelope，给后续 verify / package builder 预留兼容入口。
pub fn sign_security_approval_brief_document(
    brief: &SecurityDecisionApprovalBrief,
    key_id: &str,
    secret: &str,
) -> Result<SecurityApprovalBriefSignatureEnvelope, String> {
    if key_id.trim().is_empty() {
        return Err("approval brief signing key id cannot be empty".to_string());
    }
    if secret.trim().is_empty() {
        return Err("approval brief signing secret cannot be empty".to_string());
    }

    let canonical = canonicalize_json_bytes(brief)
        .map_err(|error| format!("approval brief canonicalization failed: {error}"))?;
    let payload_sha256 = sha256_hex(&canonical);

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|error| format!("approval brief signing init failed: {error}"))?;
    mac.update(&canonical);
    let signature = to_hex(&mac.finalize().into_bytes());

    Ok(SecurityApprovalBriefSignatureEnvelope {
        signature_version: "security_approval_brief_signature.v1".to_string(),
        algorithm: "hmac_sha256".to_string(),
        key_id: key_id.trim().to_string(),
        payload_sha256,
        contract_version: brief.contract_version.clone(),
        brief_id: brief.brief_id.clone(),
        signature,
    })
}

// 2026-04-02 CST: 这里新增 detached signature 验签入口，原因是 P0-5 需要把“可签名”升级成“可校验”；
// 目的：复用与签名阶段相同的 canonicalization 和 HMAC 逻辑，确保 verify Tool 的结论与生成逻辑一致。
pub fn verify_security_approval_brief_document(
    brief: &SecurityDecisionApprovalBrief,
    envelope: &SecurityApprovalBriefSignatureEnvelope,
    secret: &str,
) -> Result<(), String> {
    if secret.trim().is_empty() {
        return Err("approval brief verification secret cannot be empty".to_string());
    }

    let canonical = canonicalize_json_bytes(brief)
        .map_err(|error| format!("approval brief canonicalization failed: {error}"))?;
    let payload_sha256 = sha256_hex(&canonical);
    if payload_sha256 != envelope.payload_sha256 {
        return Err(format!(
            "approval brief payload sha256 mismatch: expected {}, actual {}",
            envelope.payload_sha256, payload_sha256
        ));
    }

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|error| format!("approval brief verification init failed: {error}"))?;
    mac.update(&canonical);
    let actual_signature = to_hex(&mac.finalize().into_bytes());
    if actual_signature != envelope.signature {
        return Err("approval brief detached signature mismatch".to_string());
    }

    Ok(())
}

fn canonicalize_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    let mut raw = serde_json::to_value(value)?;
    canonicalize_json_value(&mut raw);
    serde_json::to_vec(&raw)
}

fn canonicalize_json_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let mut ordered = BTreeMap::new();
            for (key, mut child) in std::mem::take(map) {
                canonicalize_json_value(&mut child);
                ordered.insert(key, child);
            }
            let mut rebuilt = Map::new();
            for (key, child) in ordered {
                rebuilt.insert(key, child);
            }
            *map = rebuilt;
        }
        Value::Array(values) => {
            for child in values {
                canonicalize_json_value(child);
            }
        }
        _ => {}
    }
}

fn sha256_hex(payload: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(payload);
    to_hex(&hasher.finalize())
}

fn to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{:02x}", byte));
    }
    output
}
