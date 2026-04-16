mod common;

use serde_json::json;

use crate::common::run_cli_with_json;

// 2026-04-13 CST: 这里补独立建议 Tool 的 catalog 红测，原因是方案B要求把“大模型独立建议线”升级成正式可发现入口；
// 目的：避免代码实现已落地，但 CLI / Skill 仍无法稳定发现这条独立建议主链。
#[test]
fn security_independent_advice_is_cataloged() {
    let output = run_cli_with_json(r#"{"tool":"tool_catalog","args":{}}"#);
    let tool_catalog = output["data"]["tool_catalog"]
        .as_array()
        .expect("tool_catalog should be an array");
    let stock_catalog = output["data"]["tool_catalog_modules"]["stock"]
        .as_array()
        .expect("stock tool group should be an array");

    assert!(
        tool_catalog
            .iter()
            .filter_map(|item| item.as_str())
            .any(|item| item == "security_independent_advice")
    );
    assert!(
        stock_catalog
            .iter()
            .filter_map(|item| item.as_str())
            .any(|item| item == "security_independent_advice")
    );
}

// 2026-04-13 CST: 这里补独立建议 Tool 的结构输出测试，原因是主席层后续要消费标准文档而不是散乱字段；
// 目的：锁住 document_type / contract_version / source_type / suggested_stance 等最小正式合同。
#[test]
fn security_independent_advice_cli_returns_structured_document() {
    let request = json!({
        "tool": "security_independent_advice",
        "args": {
            "symbol": "601916.SH",
            "analysis_date": "2026-04-13",
            "source_type": "llm_independent_review",
            "suggested_stance": "avoid",
            "confidence": 0.88,
            "rationale": "信息面与数据面存在解释冲突",
            "key_risks": ["独立建议与委员会方向冲突"],
            "evidence_basis": ["committee_package_v1"],
            "generated_at": "2026-04-13T09:00:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["document_type"],
        "security_independent_advice"
    );
    assert_eq!(
        output["data"]["contract_version"],
        "security_independent_advice.v1"
    );
    assert_eq!(output["data"]["source_type"], "llm_independent_review");
    assert_eq!(output["data"]["suggested_stance"], "avoid");
}
