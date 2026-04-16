mod common;

use serde_json::json;

use crate::common::run_cli_with_json;

// 2026-04-09 CST: 这里先锁账户级仓位 Tool 的可发现性，原因是方案A要求把账户级仓位管理正式对象化；
// 目的：确保上层可以像调用单标的 position_plan 一样发现 portfolio_position_plan。
#[test]
fn tool_catalog_includes_security_portfolio_position_plan() {
    let output = run_cli_with_json("");
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_portfolio_position_plan"),
        "tool catalog should include security_portfolio_position_plan"
    );
}

// 2026-04-09 CST: 这里先锁账户级仓位输出合同，原因是方案A不是做口头建议，而是正式回答“这笔钱先给谁、给多少”；
// 目的：确保输出里同时包含现金底线、组合摘要和逐标的建议项。
#[test]
fn security_portfolio_position_plan_outputs_formal_account_level_allocation() {
    let candidate_1 = json!({
        "symbol": "601916.SH",
        "sector_tag": "bank",
        "position_plan_document": {
            "position_plan_id": "position-plan-601916.SH-2025-10-15",
            "contract_version": "security_position_plan.v1",
            "document_type": "security_position_plan",
            "generated_at": "2026-04-09T18:00:00+08:00",
            "symbol": "601916.SH",
            "analysis_date": "2025-10-15",
            "analysis_date_guard": {
                "requested_as_of_date": "2025-10-15",
                "effective_analysis_date": "2025-10-15",
                "effective_trade_date": "2025-10-15",
                "local_data_last_date": "2025-10-15",
                "data_freshness_status": "local_exact_requested_date",
                "sync_attempted": false,
                "sync_result": null,
                "date_fallback_reason": null
            },
            "evidence_version": "evidence-v1",
            "briefing_ref": "evidence-v1",
            "committee_payload_ref": "committee-payload:601916.SH:2025-10-15",
            "recommended_action": "buy",
            "confidence": "high",
            "odds_grade": "favorable",
            "historical_confidence": "high",
            "confidence_grade": "strong",
            "position_action": "build",
            "entry_mode": "breakout_confirmation",
            "starter_position_pct": 0.06,
            "max_position_pct": 0.15,
            "add_on_trigger": "volume_up",
            "reduce_on_trigger": "break_support",
            "hard_stop_trigger": "close_below_stop",
            "liquidity_cap": "单次执行不超过计划仓位的 30%",
            "position_risk_grade": "medium",
            "regime_adjustment": "normal",
            "execution_notes": ["只在确认后加仓"],
            "rationale": ["赔率较优"]
        }
    });
    let candidate_2 = json!({
        "symbol": "600919.SH",
        "sector_tag": "bank",
        "position_plan_document": {
            "position_plan_id": "position-plan-600919.SH-2025-10-15",
            "contract_version": "security_position_plan.v1",
            "document_type": "security_position_plan",
            "generated_at": "2026-04-09T18:00:00+08:00",
            "symbol": "600919.SH",
            "analysis_date": "2025-10-15",
            "analysis_date_guard": {
                "requested_as_of_date": "2025-10-15",
                "effective_analysis_date": "2025-10-15",
                "effective_trade_date": "2025-10-15",
                "local_data_last_date": "2025-10-15",
                "data_freshness_status": "local_exact_requested_date",
                "sync_attempted": false,
                "sync_result": null,
                "date_fallback_reason": null
            },
            "evidence_version": "evidence-v2",
            "briefing_ref": "evidence-v2",
            "committee_payload_ref": "committee-payload:600919.SH:2025-10-15",
            "recommended_action": "buy",
            "confidence": "medium",
            "odds_grade": "balanced",
            "historical_confidence": "medium",
            "confidence_grade": "stable",
            "position_action": "build",
            "entry_mode": "pullback_confirmation",
            "starter_position_pct": 0.05,
            "max_position_pct": 0.12,
            "add_on_trigger": "trend_follow",
            "reduce_on_trigger": "break_support",
            "hard_stop_trigger": "close_below_stop",
            "liquidity_cap": "单次执行不超过计划仓位的 30%",
            "position_risk_grade": "high",
            "regime_adjustment": "tight",
            "execution_notes": ["只做观察仓"],
            "rationale": ["风险较高"]
        }
    });
    let request = json!({
        "tool": "security_portfolio_position_plan",
        "args": {
            "account_id": "acct-demo-001",
            "total_equity": 100000.0,
            "available_cash": 30000.0,
            "min_cash_reserve_pct": 0.20,
            "max_single_position_pct": 0.20,
            "max_sector_exposure_pct": 0.35,
            "max_portfolio_risk_budget_pct": 0.05,
            "current_portfolio_risk_budget_pct": 0.035,
            "max_single_trade_risk_budget_pct": 0.02,
            "holdings": [
                {
                    "symbol": "601916.SH",
                    "market_value": 12000.0,
                    "sector_tag": "bank"
                },
                {
                    "symbol": "159866.SZ",
                    "market_value": 18000.0,
                    "sector_tag": "etf_japan"
                },
                {
                    "symbol": "601916.SH",
                    "market_value": 0.0,
                    "sector_tag": "bank"
                }
            ],
            "candidates": [candidate_1, candidate_2],
            "created_at": "2026-04-09T18:00:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["portfolio_position_plan"]["document_type"],
        "security_portfolio_position_plan"
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["account_id"],
        "acct-demo-001"
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["current_cash_pct"],
        json!(0.30)
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["deployable_cash_amount"],
        json!(10000.0)
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["remaining_portfolio_risk_budget_pct"],
        json!(0.015)
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["estimated_new_risk_budget_pct"],
        json!(0.015)
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["total_portfolio_risk_budget_pct"],
        json!(0.05)
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["allocations"][0]["symbol"],
        "601916.SH"
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["allocations"][0]["action"],
        "add"
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["allocations"][0]["current_position_pct"],
        json!(0.12)
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["allocations"][0]["target_position_pct"],
        json!(0.15)
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["allocations"][0]["recommended_trade_amount"],
        json!(3000.0)
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["allocations"][0]["estimated_risk_budget_pct"],
        json!(0.015)
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["allocations"][0]["suggested_tranche_action"],
        "add_tranche"
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["allocations"][0]["suggested_tranche_pct"],
        json!(0.03)
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["allocations"][0]["remaining_tranche_count"],
        json!(0)
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["allocations"][1]["symbol"],
        "600919.SH"
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["allocations"][1]["action"],
        "hold"
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["allocations"][1]["recommended_trade_amount"],
        json!(0.0)
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["allocations"][1]["estimated_risk_budget_pct"],
        json!(0.0)
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["allocations"][1]["suggested_tranche_action"],
        "hold"
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["allocations"][1]["remaining_tranche_count"],
        json!(3)
    );
    assert!(
        output["data"]["portfolio_position_plan"]["allocations"][1]["constraint_flags"]
            .as_array()
            .expect("constraint flags should be array")
            .iter()
            .any(|flag| flag == "portfolio_risk_budget_reached"),
        "second candidate should be blocked by portfolio risk budget"
    );
}

#[test]
fn security_portfolio_position_plan_accepts_open_position_snapshots_as_holdings_input() {
    let candidate = json!({
        "symbol": "601916.SH",
        "sector_tag": "bank",
        "position_plan_document": {
            "position_plan_id": "position-plan-601916.SH-2025-10-15",
            "contract_version": "security_position_plan.v1",
            "document_type": "security_position_plan",
            "generated_at": "2026-04-10T12:00:00+08:00",
            "symbol": "601916.SH",
            "analysis_date": "2025-10-15",
            "analysis_date_guard": {
                "requested_as_of_date": "2025-10-15",
                "effective_analysis_date": "2025-10-15",
                "effective_trade_date": "2025-10-15",
                "local_data_last_date": "2025-10-15",
                "data_freshness_status": "local_exact_requested_date",
                "sync_attempted": false,
                "sync_result": null,
                "date_fallback_reason": null
            },
            "evidence_version": "evidence-v1",
            "briefing_ref": "evidence-v1",
            "committee_payload_ref": "committee-payload:601916.SH:2025-10-15",
            "recommended_action": "buy",
            "confidence": "high",
            "odds_grade": "favorable",
            "historical_confidence": "high",
            "confidence_grade": "strong",
            "position_action": "build",
            "entry_mode": "breakout_confirmation",
            "starter_position_pct": 0.06,
            "max_position_pct": 0.15,
            "add_on_trigger": "volume_up",
            "reduce_on_trigger": "break_support",
            "hard_stop_trigger": "close_below_stop",
            "liquidity_cap": "单次执行不超过计划仓位的 30%",
            "position_risk_grade": "medium",
            "regime_adjustment": "normal",
            "execution_notes": ["只在确认后加仓"],
            "rationale": ["赔率较优"]
        }
    });
    let request = json!({
        "tool": "security_portfolio_position_plan",
        "args": {
            "account_id": "acct-demo-open-snapshot",
            "total_equity": 100000.0,
            "available_cash": 30000.0,
            "min_cash_reserve_pct": 0.20,
            "max_single_position_pct": 0.20,
            "max_sector_exposure_pct": 0.35,
            "max_portfolio_risk_budget_pct": 0.05,
            "current_portfolio_risk_budget_pct": 0.02,
            "max_single_trade_risk_budget_pct": 0.02,
            "holdings": [],
            "open_position_snapshots": [
                {
                    "symbol": "601916.SH",
                    "position_state": "open",
                    "current_position_pct": 0.12,
                    "sector_tag": "bank",
                    "source_execution_record_ref": "execution-record-601916.SH-open"
                },
                {
                    "symbol": "600919.SH",
                    "position_state": "flat",
                    "current_position_pct": 0.08,
                    "sector_tag": "bank",
                    "source_execution_record_ref": "execution-record-600919.SH-flat"
                }
            ],
            "candidates": [candidate],
            "created_at": "2026-04-10T12:00:00+08:00"
        }
    });

    let output = run_cli_with_json(&request.to_string());
    assert_eq!(output["status"], "ok", "output={output}");
    // 2026-04-10 CST: 这里锁定 open snapshot 会被折算成当前持仓，原因是账户层连续状态不能再依赖手工补 holdings；
    // 目的：确保 open snapshot 能直接进入下一轮账户计划，而 flat snapshot 不会被错误算成持仓暴露。
    assert_eq!(
        output["data"]["portfolio_position_plan"]["allocations"][0]["current_position_pct"],
        json!(0.12)
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["allocations"][0]["target_position_pct"],
        json!(0.15)
    );
    assert_eq!(
        output["data"]["portfolio_position_plan"]["allocations"][0]["recommended_trade_amount"],
        json!(3000.0)
    );
}
