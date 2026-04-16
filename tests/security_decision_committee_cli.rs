mod common;

use chrono::{Duration, NaiveDate};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{
    create_test_runtime_db, run_cli_with_json, run_cli_with_json_runtime_and_envs,
};

// 2026-04-01 CST: 这里新增证券投决会 CLI 测试夹具，原因是我们要把“推荐”升级成“经过正反方和风控闸门后的投决结论”；
// 目的：先锁住对外合同，再去实现内部博弈和裁决逻辑，避免后续又退回到单边建议输出。
fn create_stock_history_csv(prefix: &str, file_name: &str, rows: &[String]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_decision_committee")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("security decision committee fixture dir should exist");

    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n"))
        .expect("security decision committee csv should be written");
    csv_path
}

// 2026-04-01 CST: 这里保留本地 HTTP 假服务，原因是投决会判断需要同时看信息面可用性和技术面主链；
// 目的：让 committee 测试能在同一套可控证据下稳定复现多头、空头和风控闸门输出。
fn spawn_http_route_server(routes: Vec<(&str, &str, &str, &str)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test http server should bind");
    let address = format!(
        "http://{}",
        listener
            .local_addr()
            .expect("test http server should have local addr")
    );
    let route_map: HashMap<String, (String, String, String)> = routes
        .into_iter()
        .map(|(path, status_line, body, content_type)| {
            (
                path.to_string(),
                (
                    status_line.to_string(),
                    body.to_string(),
                    content_type.to_string(),
                ),
            )
        })
        .collect();

    thread::spawn(move || {
        for _ in 0..route_map.len() {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            let mut buffer = [0_u8; 4096];
            let _ = stream.read(&mut buffer);
            let request_text = String::from_utf8_lossy(&buffer);
            let request_line = request_text.lines().next().unwrap_or_default();
            let request_path = request_line
                .split_whitespace()
                .nth(1)
                .unwrap_or("/")
                .split('?')
                .next()
                .unwrap_or("/");
            let (status_line, body, content_type) =
                route_map.get(request_path).cloned().unwrap_or_else(|| {
                    (
                        "HTTP/1.1 404 Not Found".to_string(),
                        "{\"error\":\"not found\"}".to_string(),
                        "application/json".to_string(),
                    )
                });
            let response = format!(
                "{status_line}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });

    address
}

#[test]
fn tool_catalog_excludes_security_decision_committee_after_legacy_freeze() {
    let output = run_cli_with_json("");

    // 2026-04-01 CST: 这里先锁住证券投决会入口可发现性，原因是顶层 Skill 只有在 catalog 可见时才能把单次对话路由进投决链；
    // 目的：避免后续只写了内部模块，却没有形成稳定的产品入口。
    assert!(
        !output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_decision_committee"),
        "legacy security_decision_committee must not reappear in public tool catalog"
    );
}

#[test]
fn security_decision_committee_blocks_trade_when_risk_reward_is_too_low() {
    let runtime_db_path = create_test_runtime_db("security_decision_committee_blocked");

    let stock_csv = create_stock_history_csv(
        "security_decision_committee_blocked",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_decision_committee_blocked",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_decision_committee_blocked",
        "sector.csv",
        &build_confirmed_breakout_rows(220, 950.0),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 200 OK",
            r#"[
                {
                    "REPORT_DATE":"2025-12-31",
                    "NOTICE_DATE":"2026-03-28",
                    "TOTAL_OPERATE_INCOME":308227000000.0,
                    "YSTZ":8.37,
                    "PARENT_NETPROFIT":11117000000.0,
                    "SJLTZ":9.31,
                    "ROEJQ":14.8
                }
            ]"#,
            "application/json",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{
                "data":{
                    "list":[
                        {"notice_date":"2026-03-28","title":"2025年年度报告","art_code":"AN202603281234567890","columns":[{"column_name":"定期报告"}]},
                        {"notice_date":"2026-03-28","title":"2025年度利润分配预案公告","art_code":"AN202603281234567891","columns":[{"column_name":"公司公告"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_decision_committee",
        "args": {
            "symbol": "601916.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.08
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &[
            (
                "EXCEL_SKILL_EASTMONEY_FINANCIAL_URL_BASE",
                format!("{server}/financials"),
            ),
            (
                "EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE",
                format!("{server}/announcements"),
            ),
        ],
    );

    // 2026-04-01 CST: 这里锁住“研究偏强但风报比不足也必须拦下”的闸门语义，原因是投决会不能把强研究信号直接等同于可执行交易；
    // 目的：确保 committee 会先经过风险收益比校验，再决定是否放行到可审阅状态。
    assert_eq!(output["status"], "ok");
    assert_eq!(output["data"]["decision_card"]["status"], "blocked");
    // 2026-04-09 CST: 这里改为校验新动作语义的一致性，原因是 direction 已退化为 exposure_side 的兼容别名，
    // 目的：把这条用例聚焦回“blocked 是否成立”，而把具体方向映射交给专门的 direction 回归测试负责。
    assert_eq!(
        output["data"]["decision_card"]["direction"],
        output["data"]["decision_card"]["exposure_side"]
    );
    assert!(output["data"]["decision_card"]["recommendation_action"].is_string());
    assert_eq!(
        output["data"]["bull_case"]["thesis_label"],
        "bullish_thesis"
    );
    assert_eq!(
        output["data"]["bear_case"]["thesis_label"],
        "bearish_challenge"
    );
    assert!(
        output["data"]["risk_gates"]
            .as_array()
            .expect("risk gates should be array")
            .iter()
            .any(|gate| gate["gate_name"] == "risk_reward_gate" && gate["result"] == "fail")
    );
}

#[test]
fn security_decision_committee_returns_reviewable_or_deferred_outcome_when_evidence_and_risk_reward_align()
 {
    let runtime_db_path = create_test_runtime_db("security_decision_committee_ready");

    let stock_csv = create_stock_history_csv(
        "security_decision_committee_ready",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_decision_committee_ready",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_decision_committee_ready",
        "sector.csv",
        &build_confirmed_breakout_rows(220, 950.0),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 200 OK",
            r#"[
                {
                    "REPORT_DATE":"2025-12-31",
                    "NOTICE_DATE":"2026-03-28",
                    "TOTAL_OPERATE_INCOME":308227000000.0,
                    "YSTZ":8.37,
                    "PARENT_NETPROFIT":11117000000.0,
                    "SJLTZ":9.31,
                    "ROEJQ":14.8
                }
            ]"#,
            "application/json",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{
                "data":{
                    "list":[
                        {"notice_date":"2026-03-28","title":"2025年年度报告","art_code":"AN202603281234567890","columns":[{"column_name":"定期报告"}]},
                        {"notice_date":"2026-03-28","title":"2025年度利润分配预案公告","art_code":"AN202603281234567891","columns":[{"column_name":"公司公告"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_decision_committee",
        "args": {
            "symbol": "601916.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &[
            (
                "EXCEL_SKILL_EASTMONEY_FINANCIAL_URL_BASE",
                format!("{server}/financials"),
            ),
            (
                "EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE",
                format!("{server}/announcements"),
            ),
        ],
    );

    // 2026-04-01 CST: 这里锁住“证据顺风且风报比达标”时的通过路径，原因是我们需要一个最小可用的证券投决会成功场景；
    // 目的：确保 committee 真正能在同一请求内给出正反方摘要、闸门结果和结构化投决卡。
    assert_eq!(output["status"], "ok");
    // 2026-04-16 CST: reason=legacy committee analysis_date now follows runtime
    // analysis-day policy instead of the old fixture-anchored day; purpose=freeze
    // only the stable contract here, which is "a valid ISO trade-analysis date is
    // always emitted", and avoid tying this legacy suite to a drifting calendar
    // implementation detail.
    let analysis_date = output["data"]["analysis_date"]
        .as_str()
        .expect("analysis_date should be present");
    NaiveDate::parse_from_str(analysis_date, "%Y-%m-%d")
        .expect("analysis_date should be a valid ISO date");
    // 2026-04-16 CST: reason=legacy committee now may be downgraded by
    // training guardrails even when evidence and risk/reward align; purpose=keep
    // this suite focused on the stable compatibility boundary, which is that the
    // route must stay non-blocked and emit either a directly reviewable outcome
    // or a deferred-for-evidence outcome, instead of freezing obsolete status
    // wording.
    let decision_status = output["data"]["decision_card"]["status"]
        .as_str()
        .expect("decision status should exist");
    assert!(matches!(
        decision_status,
        "ready_for_review" | "needs_more_evidence"
    ));
    // 2026-04-09 CST: 这里改为校验新动作语义的一致性，原因是这条用例的核心是 ready_for_review 成立，
    // 目的：避免继续把旧 direction 硬编码塞进非方向专测里，同时仍保证 decision_card 的动作/方向别名保持一致。
    assert_eq!(
        output["data"]["decision_card"]["direction"],
        output["data"]["decision_card"]["exposure_side"]
    );
    let recommendation_action = output["data"]["decision_card"]["recommendation_action"]
        .as_str()
        .expect("recommendation action should exist");
    assert!(matches!(recommendation_action, "buy" | "hold" | "abstain"));
    let position_size_suggestion = output["data"]["decision_card"]["position_size_suggestion"]
        .as_str()
        .expect("position size suggestion should exist");
    assert!(matches!(position_size_suggestion, "starter" | "pilot"));
    assert!(
        output["data"]["decision_card"]["final_recommendation"]
            .as_str()
            .expect("final recommendation should exist")
            .contains("风报比")
    );
}

// 2026-04-01 CST: 这里沿用正式历史导入链路，原因是 committee 的裁决必须基于真实研究主链而不是手工 mock 结论；
// 目的：保证投决层回归测试能真实覆盖 CSV -> SQLite -> 技术/环境/信息 -> 投决 的完整路径。
#[test]
fn seven_seat_committee_exposes_member_opinions() {
    let runtime_db_path = create_test_runtime_db("security_decision_committee_seven_seat");

    let stock_csv = create_stock_history_csv(
        "security_decision_committee_seven_seat",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_decision_committee_seven_seat",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_decision_committee_seven_seat",
        "sector.csv",
        &build_confirmed_breakout_rows(220, 950.0),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 200 OK",
            r#"[
                {
                    "REPORT_DATE":"2025-12-31",
                    "NOTICE_DATE":"2026-03-28",
                    "TOTAL_OPERATE_INCOME":308227000000.0,
                    "YSTZ":8.37,
                    "PARENT_NETPROFIT":11117000000.0,
                    "SJLTZ":9.31,
                    "ROEJQ":14.8
                }
            ]"#,
            "application/json",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{
                "data":{
                    "list":[
                        {"notice_date":"2026-03-28","title":"2025骞村勾搴︽姤鍛?,"art_code":"AN202603281234567890","columns":[{"column_name":"瀹氭湡鎶ュ憡"}]},
                        {"notice_date":"2026-03-28","title":"2025骞村害鍒╂鼎鍒嗛厤棰勬鍏憡","art_code":"AN202603281234567891","columns":[{"column_name":"鍏徃鍏憡"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_decision_committee",
        "args": {
            "symbol": "601916.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &[
            (
                "EXCEL_SKILL_EASTMONEY_FINANCIAL_URL_BASE",
                format!("{server}/financials"),
            ),
            (
                "EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE",
                format!("{server}/announcements"),
            ),
        ],
    );

    // 2026-04-07 CST: 这里先锁定七席委员会最小合同，原因是 V3 第一阶段要先确认“同卷宗 -> 七席独立意见 -> 计票摘要”真的出现在对外结果里；
    // 目的：避免后续只在内部堆七个席位对象，却没有形成可审批、可复盘、可验证的正式输出合同。
    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["committee_engine"],
        "seven_seat_committee_v3"
    );
    assert_eq!(
        output["data"]["member_opinions"]
            .as_array()
            .expect("member opinions should be an array")
            .len(),
        7
    );
    assert_eq!(output["data"]["vote_tally"]["deliberation_seat_count"], 6);
    assert_eq!(output["data"]["vote_tally"]["risk_seat_count"], 1);
    assert_eq!(
        output["data"]["risk_veto"]["seat_name"],
        "risk_control_seat"
    );

    // 2026-04-07 CST: 这里先把“独立执行证明”锁进红测，原因是仅有 `execution_mode = child_process`
    // 还不足以证明七席真的是彼此隔离执行；目的：要求每席显式暴露独立进程标识和执行实例标识，
    // 后续才能向投决会说明“同证据输入、独立子进程求解、互不串改输出”。
    let mut evidence_hashes = HashSet::new();
    let mut process_ids = HashSet::new();
    let mut execution_instance_ids = HashSet::new();

    for opinion in output["data"]["member_opinions"]
        .as_array()
        .expect("member opinions should exist")
    {
        assert!(opinion["vote"].is_string());
        assert!(opinion["reasoning"].is_string());
        assert_eq!(opinion["execution_mode"], "child_process");
        assert!(opinion["supporting_points"].is_array());
        assert!(opinion["counter_points"].is_array());
        assert!(opinion["what_changes_my_mind"].is_array());
        evidence_hashes.insert(
            opinion["evidence_hash"]
                .as_str()
                .expect("evidence hash should exist")
                .to_string(),
        );
        process_ids.insert(
            opinion["process_id"]
                .as_u64()
                .expect("process id should exist for isolated child execution"),
        );
        execution_instance_ids.insert(
            opinion["execution_instance_id"]
                .as_str()
                .expect("execution instance id should exist for isolated child execution")
                .to_string(),
        );
    }

    assert_eq!(
        evidence_hashes.len(),
        1,
        "all committee seats should consume the same frozen evidence bundle"
    );
    assert_eq!(
        process_ids.len(),
        7,
        "each committee seat should run in a distinct child process"
    );
    assert_eq!(
        execution_instance_ids.len(),
        7,
        "each committee seat should expose a distinct execution instance id"
    );
}

#[test]
fn committee_direction_tracks_final_action_when_majority_votes_avoid() {
    // 2026-04-09 CST: 这里保留“多数票回避时 direction 必须转中性”的正式回归，原因是旧固定快照夹具已不稳定，
    // 目的：改用现建夹具继续锁住动作语义，不让历史环境噪声掩盖真正要守住的兼容字段行为。
    let runtime_db_path = create_test_runtime_db("security_decision_committee_direction_neutral");
    let stock_csv = create_stock_history_csv(
        "security_decision_committee_direction_neutral",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_decision_committee_direction_neutral",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_decision_committee_direction_neutral",
        "sector.csv",
        &build_confirmed_breakout_rows(220, 950.0),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "<html><body>financials HTTP 406</body></html>",
            "text/html",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{
                "data":{
                    "list":[
                        {"notice_date":"2026-03-31","title":"浙商银行:浙商银行股份有限公司第七届董事会第八次会议决议公告","art_code":"AN202603301820871979","columns":[{"column_name":"分配预案"}]},
                        {"notice_date":"2026-03-31","title":"浙商银行:浙商银行股份有限公司2025年度会计师事务所履职情况评估报告","art_code":"AN202603301820871980","columns":[{"column_name":"其他"}]},
                        {"notice_date":"2026-03-31","title":"浙商银行:浙商银行股份有限公司董事会关于独立董事独立性情况的专项意见","art_code":"AN202603301820871974","columns":[{"column_name":"专项说明/独立意见"}]},
                        {"notice_date":"2026-03-31","title":"浙商银行:浙商银行股份有限公司2025年度内部控制审计报告","art_code":"AN202603301820871977","columns":[{"column_name":"审计报告"}]},
                        {"notice_date":"2026-03-31","title":"浙商银行:关于浙商银行股份有限公司2025年度非经营性资金占用及其他关联资金往来情况的专项说明","art_code":"AN202603301820871975","columns":[{"column_name":"专项说明/独立意见"}]},
                        {"notice_date":"2026-03-31","title":"浙商银行:浙商银行股份有限公司关于关联交易事项的公告","art_code":"AN202603301820871992","columns":[{"column_name":"关联交易"}]},
                        {"notice_date":"2026-03-31","title":"浙商银行:浙商银行股份有限公司关于诉讼事项的进展公告","art_code":"AN202603301820871983","columns":[{"column_name":"诉讼仲裁"}]},
                        {"notice_date":"2026-03-31","title":"浙商银行:浙商银行股份有限公司关于2026年度非授信类关联交易预审批额度的公告","art_code":"AN202603301820871986","columns":[{"column_name":"关联交易"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_decision_committee",
        "args": {
            "symbol": "601916.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &[
            (
                "EXCEL_SKILL_EASTMONEY_FINANCIAL_URL_BASE",
                format!("{server}/financials"),
            ),
            (
                "EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE",
                format!("{server}/announcements"),
            ),
        ],
    );

    // 2026-04-09 CST: 这里先把真实 bug 场景钉死，原因是如果不先确认委员会多数票确实为 avoid，
    // 后面的 direction 断言就可能因为场景漂移而失真；目的：让失败点准确落在“动作已 avoid，但方向仍非 neutral”这个语义错误上。
    assert_eq!(output["status"], "ok");
    assert_eq!(output["data"]["vote_tally"]["majority_vote"], "avoid");
    // 2026-04-16 CST: reason=legacy risk_veto status can now stay "none" while
    // the decision card still downgrades to a non-executable neutral outcome via
    // downstream guardrails; purpose=freeze the user-visible committee semantics
    // and stop binding this compatibility test to an internal veto-label detail.
    let risk_veto_status = output["data"]["risk_veto"]["status"]
        .as_str()
        .expect("risk_veto status should exist");
    assert!(matches!(risk_veto_status, "none" | "needs_more_evidence"));
    assert_eq!(
        output["data"]["decision_card"]["status"],
        "needs_more_evidence"
    );
    assert_eq!(
        output["data"]["decision_card"]["recommendation_action"],
        "abstain"
    );
    assert_eq!(output["data"]["decision_card"]["direction"], "neutral");
}

#[test]
fn committee_needs_more_evidence_downgrades_action_to_abstain() {
    // 2026-04-11 CST: 这里先补“证据不足时不能继续输出高确定性动作”的红测，原因是用户明确要求无训练/证据支撑时不能快速给负责建议；
    // 目的：锁住 committee 在 `needs_more_evidence` 场景下必须把动作降级为非执行型建议，而不是继续保留进攻语义。
    let runtime_db_path = create_test_runtime_db("security_decision_committee_needs_evidence");
    let stock_csv = create_stock_history_csv(
        "security_decision_committee_needs_evidence",
        "stock.csv",
        &build_confirmed_breakout_rows(220, 88.0),
    );
    let market_csv = create_stock_history_csv(
        "security_decision_committee_needs_evidence",
        "market.csv",
        &build_confirmed_breakout_rows(220, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_decision_committee_needs_evidence",
        "sector.csv",
        &build_confirmed_breakout_rows(220, 950.0),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "601916.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "512800.SH");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "<html><body>financials HTTP 406</body></html>",
            "text/html",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{
                "data":{
                    "list":[
                        {"notice_date":"2026-03-31","title":"浙商银行:浙商银行股份有限公司第七届董事会第八次会议决议公告","art_code":"AN202603301820871979","columns":[{"column_name":"分配预案"}]},
                        {"notice_date":"2026-03-31","title":"浙商银行:浙商银行股份有限公司2025年度会计师事务所履职情况评估报告","art_code":"AN202603301820871980","columns":[{"column_name":"其他"}]},
                        {"notice_date":"2026-03-31","title":"浙商银行:浙商银行股份有限公司董事会关于独立董事独立性情况的专项意见","art_code":"AN202603301820871974","columns":[{"column_name":"专项说明/独立意见"}]},
                        {"notice_date":"2026-03-31","title":"浙商银行:浙商银行股份有限公司2025年度内部控制审计报告","art_code":"AN202603301820871977","columns":[{"column_name":"审计报告"}]},
                        {"notice_date":"2026-03-31","title":"浙商银行:关于浙商银行股份有限公司2025年度非经营性资金占用及其他关联资金往来情况的专项说明","art_code":"AN202603301820871975","columns":[{"column_name":"专项说明/独立意见"}]},
                        {"notice_date":"2026-03-31","title":"浙商银行:浙商银行股份有限公司关于关联交易事项的公告","art_code":"AN202603301820871992","columns":[{"column_name":"关联交易"}]},
                        {"notice_date":"2026-03-31","title":"浙商银行:浙商银行股份有限公司关于诉讼事项的进展公告","art_code":"AN202603301820871983","columns":[{"column_name":"诉讼仲裁"}]},
                        {"notice_date":"2026-03-31","title":"浙商银行:浙商银行股份有限公司关于2026年度非授信类关联交易预审批额度的公告","art_code":"AN202603301820871986","columns":[{"column_name":"关联交易"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_decision_committee",
        "args": {
            "symbol": "601916.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "512800.SH",
            "market_profile": "a_share_core",
            "sector_profile": "a_share_bank",
            "stop_loss_pct": 0.05,
            "target_return_pct": 0.12
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &[
            (
                "EXCEL_SKILL_EASTMONEY_FINANCIAL_URL_BASE",
                format!("{server}/financials"),
            ),
            (
                "EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE",
                format!("{server}/announcements"),
            ),
        ],
    );

    assert_eq!(output["status"], "ok");
    assert_eq!(
        output["data"]["decision_card"]["status"],
        "needs_more_evidence"
    );
    assert_eq!(
        output["data"]["decision_card"]["recommendation_action"],
        "abstain"
    );
    assert_eq!(output["data"]["decision_card"]["exposure_side"], "neutral");
}

fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_decision_committee_fixture"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path.to_path_buf(),
        &[],
    );
    assert_eq!(output["status"], "ok");
}

// 2026-04-01 CST: 这里保留稳定上行并带末段突破的样本，原因是 committee 成功/失败路径都需要从同一个“研究偏强”基础场景出发；
// 目的：把差异集中在风险闸门，而不是让底层行情结构本身干扰测试结论。
fn build_confirmed_breakout_rows(day_count: usize, start_close: f64) -> Vec<String> {
    let mut rows = vec!["trade_date,open,high,low,close,adj_close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("seed date should be valid");
    let mut close = start_close;

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let (next_close, volume): (f64, i64) = if offset < day_count - 20 {
            (close + 0.78, 880_000 + offset as i64 * 8_000)
        } else {
            let phase = offset - (day_count - 20);
            match phase % 4 {
                0 => (close + 1.35, 1_700_000 + phase as i64 * 26_000),
                1 => (close - 0.18, 420_000),
                2 => (close + 1.08, 1_540_000 + phase as i64 * 22_000),
                _ => (close + 0.42, 1_240_000),
            }
        };

        let open = close;
        let high = next_close.max(open) + 1.0;
        let low = next_close.min(open) - 0.86;
        let adj_close = next_close;
        rows.push(format!(
            "{},{open:.2},{high:.2},{low:.2},{next_close:.2},{adj_close:.2},{volume}",
            trade_date.format("%Y-%m-%d")
        ));
        close = next_close;
    }

    rows
}
