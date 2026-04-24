mod common;

use chrono::{Duration, NaiveDate};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{
    create_test_runtime_db, run_cli_with_json, run_cli_with_json_runtime_and_envs,
};

// 2026-04-01 CST: CSV fixture writer for fullstack regression inputs.
fn create_stock_history_csv(prefix: &str, file_name: &str, rows: &[String]) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let fixture_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("security_analysis_fullstack")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&fixture_dir).expect("security fullstack fixture dir should exist");

    let csv_path = fixture_dir.join(file_name);
    fs::write(&csv_path, rows.join("\n")).expect("security fullstack csv should be written");
    csv_path
}

// 2026-04-01 CST: 闂傚倷绀侀幖顐λ囬锕€鐤炬繝濠傜墛閸嬶繝鏌嶉崫鍕櫣闂傚偆鍨伴—鍐偓锝庝簻椤掋垽鏌涚€ｎ偓鑰块柡灞炬礃瀵板嫰宕煎┑鍡椥ゆ繝纰樻閸ㄤ即顢栨径鎰畺閻熸瑥瀚悷瑙勩亜閺嶃劍鐨戝ù鐙€鍨跺鐑樻姜閹殿喚鐛㈡繛瀛樼矎濞夋盯顢氶敐澶嬪仺闁告挸寮堕弲銏ゆ⒑闁偛鑻晶顕€鏌?HTTP 闂傚倸鍊烽懗鍫曗€﹂崼銉晞闁告侗鍔跺ú顏呮櫇闁逞屽墴閺佸啴濮€閵堝懐鍊炲銈嗗坊閸嬫挾绱掗埀顒佸緞閹邦厾鍘撻梺鑺ッˇ浼村闯閻ｅ瞼纾奸柍褜鍓氬鍕箛椤撶姴骞堟繝纰樻閸ㄦ娊宕㈣瀵憡鎯旈姀銏㈩啎闂佸憡鐟ラˇ閬嶆儗閹烘柡鍋?fullstack Tool 濠电姷鏁搁崑鐐差焽濞嗘挸瑙﹂悗锝庡枟閸ゅ苯螖閿濆懎鏆為柛銈嗗姍閺岋綁寮崒姘闂佺顑嗛…鍥焵椤掆偓閸樻粓宕戦幘缁樼厱闁归偊鍘奸崝銈夋煛閸涚繝鎲炬慨濠傤煼瀹曟帒顫濋璺ㄦ晼闂備礁鎽滄慨鐢告儎椤栨氨鏆﹂柟鐑橆殔鎯熼梺瀹犳〃閼冲爼顢欓崨瀛樷拺闂傚牊渚楀Σ鍫曟煕鎼淬倕鐨洪柍褜鍓氱喊宥夋偂閿熺姴钃熸繛鎴炵矌閻も偓闂佹寧绻傞幊搴綖閳哄懏鈷戦悷娆忓閵堟挳鏌涢幘瀵哥畼缂侇喗鐟﹀鍕箛椤戔敪鍥ㄧ厱闁哄洢鍔岄悘鐘崇箾閸喓绠栫紒缁樼箞婵偓闁挎繂鎳愰崢顐︽⒑缂佹﹩娈樺┑鐐╁亾閻庤娲樺ú鐔肩嵁濮椻偓椤㈡瑩鎳栭埡濠冃濋梺璇查缁犲秹宕曢柆宥嗗亱闁糕剝绋掗崐鍨亜閹捐泛鏋旂紒鈾€鍋撻柣鐔哥矊闁帮絽鐣峰┑濠庢Ь濠?
// 2026-04-01 CST: Local HTTP route server fixture for deterministic upstream responses.
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
        // 2026-04-15 CST: Allow repeated hits because fullstack regressions may
        // retry/fallback across multiple providers and query the same local route
        // more than once during a single test run.
        let max_accepts = route_map.len() * 8;
        for _ in 0..max_accepts {
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
fn tool_catalog_includes_security_analysis_fullstack() {
    let output = run_cli_with_json("");

    // 2026-04-15 CST: Keep the tool catalog regression focused on discoverability.
    // Purpose: ensure the formal tool name remains exposed to skill/runtime callers.
    assert!(
        output["data"]["tool_catalog"]
            .as_array()
            .expect("tool catalog should be an array")
            .iter()
            .any(|tool| tool == "security_analysis_fullstack")
    );
}

#[test]
fn security_analysis_fullstack_aggregates_technical_fundamental_and_disclosures() {
    let runtime_db_path = create_test_runtime_db("security_analysis_fullstack_ok");

    let stock_csv = create_stock_history_csv(
        "security_analysis_fullstack_ok",
        "stock.csv",
        &build_range_bound_rows(220, 35.8, 38.0),
    );
    let market_csv = create_stock_history_csv(
        "security_analysis_fullstack_ok",
        "market.csv",
        &build_confirmed_breakdown_rows(220, 4.9),
    );
    let sector_csv = create_stock_history_csv(
        "security_analysis_fullstack_ok",
        "sector.csv",
        &build_choppy_history_rows(220, 1.02),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "002352.SZ");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "516530.SH");

    let server = spawn_http_route_server(vec![
        (
            "/capital-flow",
            "HTTP/1.1 200 OK",
            r#"{
                "data":{
                    "symbol":"002352.SZ",
                    "main_net_inflow":12500000.0,
                    "super_order_net_inflow":6200000.0,
                    "headline":"濠电姷鏁搁崑鐐哄垂閸洖绠插〒姘ｅ亾妞ゃ垺淇洪ˇ鎶芥煙娓氬灝濮€缂佺姵鐩鎾偄妞嬪孩鐦掗梻鍌欑閹测€趁洪敃鍌氬瀭闁哄绨遍弸宥夋煏婵炑冨閺嬫垿姊虹紒姗嗘當闁绘锕︽禍鎼佹偋閸粎绠氬銈嗗姉閸犳劕鈻嶆繝鍕ㄥ亾鐟欏嫭绀冮柣鐔叉櫊瀹曟椽鍩€椤掍降浜滈柟鍝勬娴滈箖姊婚崶褜妲搁棁澶愭煥濠靛棙绁╅柣鎺戝⒔閳ь剚顔栭崰妤佺箾婵犲洤钃熼柕濞垮劗濡插牊淇婇婵嗕汗闁告棏鍠氱槐鎾存媴閸撳弶笑闂佸鏉垮妞ゆ洏鍎靛畷鐔碱敋閸涱啩鈺呮煟鎼淬値娼愭繛鍙夌矒瀹曚即寮借閺嗭箓鏌曟竟顖氬€归崟鍐磽閸屾瑩妾烽柛銊潐娣囧﹪鏌嗗鍡忔嫼闂佸憡绋戦敃銈夋倶瀹ュ鐓曢悗锝庡亝鐏忔壆绱掔紒妯笺€掔紒杈ㄧ懇閹晛鈻撻幐骞粓姊?
                }
            }"#,
            "application/json",
        ),
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
                        {"notice_date":"2026-03-28","title":"2025婵犲痉鏉库偓妤佹叏閻戣棄纾婚柣鎰▕閻掕姤绻涢崱妯诲碍缂佲偓鐎ｎ喗鐓熼柟浼存涧婢ь垶鏌涙惔锝呮灈闁哄被鍔戝顕€宕堕…鎴烆棄闁荤偞纰嶅畝绋款潖?,"art_code":"AN202603281234567890","columns":[{"column_name":"闂傚倷娴囬褍顫濋敃鍌︾稏濠㈣泛鑻弸鍫⑩偓骞垮劚閹锋垿鎳撻崸妤佺叆闁绘柨鎼瓭缂備胶濯崳锝夊蓟閺囩喎绶為柛顐ｇ箓婵洟姊?}]},
                        {"notice_date":"2026-03-28","title":"2025婵犲痉鏉库偓妤佹叏閻戣棄纾婚柣鎰▕閻掕姤绻涢崱妯虹亶闁稿鎸鹃幉鎾礋椤愮喐鐏嗛柣搴ゎ潐濞叉ê鐣濋幖浣告槬闁跨喓濮寸壕濂告煟閹邦喗鏆╂い锔垮嵆濮婄粯鎷呴崨濠傛殘闁汇埄鍨辩敮鎺曠亱濠电娀娼ч鍛村磼閵娾晜鐓忛柛顐ｇ箖閿涙梻绱掗幉瀣М闁哄本鐩鎾Ω閵夈儮鎷婚梻浣姐€€閸嬫捇鏌熼幆鏉啃撻柣鎾存礋閻擃偊宕舵搴″煂婵犫拃灞芥珝闁?,"art_code":"AN202603281234567891","columns":[{"column_name":"闂傚倸鍊烽懗鍫曗€﹂崼婢濈懓顫濈捄铏诡槶濠电偛妫楀ù姘跺吹濡ゅ懏鐓曢柟浼存涧閺嬫稓绱掗埀顒勫磼濞戞牔绨婚梺鐟版惈濡绂嶆ィ鍐┾拺?}]},
                        {"notice_date":"2026-03-10","title":"闂傚倸鍊烽懗鍫曗€﹂崼銏″床闁圭増婢橀崒銊╂煛瀹ュ骸浜┑顔挎珪閵囧嫰骞掗幋婵愪痪闂佹悶鍊曢崯鎾蓟瀹ュ唯闁挎洍鍋撶紒鎻掝煼閺屸剝鎷呴崷顓犵厜闂佸搫鏈惄顖涗繆閻ゎ垼妲绘繛瀵稿Л閺呯娀寮诲☉銏″亹闁瑰瓨鍔栭崹鐢糕€﹂崶鈺傚珰婵炴潙顑嗛～宥呪攽閳藉棗鐏犻柛姘儑缁辩偛鐣濋崟顑芥嫼濠殿喚鎳撳ú銈嗕繆婵傚憡鍊垫慨妯煎帶婢у鈧娲╃紞浣割嚕閹绢喖顫呴柣妯垮皺閻?,"art_code":"AN202603101234567892","columns":[{"column_name":"闂傚倸鍊烽懗鍫曗€﹂崼婢濈懓顫濈捄铏诡槶濠电偛妫楀ù姘跺吹濡ゅ懏鐓曢柟浼存涧閺嬫稓绱掗埀顒勫磼濞戞牔绨婚梺鐟版惈濡绂嶆ィ鍐┾拺?}]}
                    ]
                }
            }"#,
            "application/json",
        ),
        (
            "/announcements-clean",
            "HTTP/1.1 200 OK",
            r#"{
                "data":{
                    "list":[
                        {"notice_date":"2026-03-28","title":"2025 annual report released","art_code":"AN202603281234567890","columns":[{"column_name":"regular disclosure"}]},
                        {"notice_date":"2026-03-28","title":"2025 annual results summary","art_code":"AN202603281234567891","columns":[{"column_name":"regular disclosure"}]},
                        {"notice_date":"2026-03-10","title":"board approves annual dividend plan","art_code":"AN202603101234567892","columns":[{"column_name":"regular disclosure"}]}
                    ]
                }
            }"#,
            "application/json",
        ),
        (
            "/official-financials",
            "HTTP/1.1 500 Internal Server Error",
            r#"{"error":"official financial fallback should stay unused in this test"}"#,
            "application/json",
        ),
        (
            "/official-announcements",
            "HTTP/1.1 500 Internal Server Error",
            r#"{"error":"official disclosure fallback should stay unused in this test"}"#,
            "application/json",
        ),
        (
            "/sina-financials",
            "HTTP/1.1 500 Internal Server Error",
            r#"{"error":"sina financial fallback should stay unused in this test"}"#,
            "application/json",
        ),
        (
            "/sina-announcements",
            "HTTP/1.1 500 Internal Server Error",
            r#"{"error":"sina disclosure fallback should stay unused in this test"}"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_analysis_fullstack",
        "args": {
            "symbol": "002352.SZ",
            "market_symbol": "510300.SH",
            "sector_symbol": "516530.SH",
            "disclosure_limit": 3
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &[
            // 2026-04-20 CST: Pin every fallback env to deterministic local routes.
            // Reason: this regression must not inherit operator shell envs or external network behavior.
            // Purpose: keep the test focused on the EastMoney happy path and fail loudly if that path drifts.
            (
                "EXCEL_SKILL_EASTMONEY_CAPITAL_FLOW_URL_BASE",
                format!("{server}/capital-flow"),
            ),
            (
                "EXCEL_SKILL_EASTMONEY_FINANCIAL_URL_BASE",
                format!("{server}/financials"),
            ),
            (
                "EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE",
                format!("{server}/announcements-clean"),
            ),
            (
                "EXCEL_SKILL_OFFICIAL_FINANCIAL_URL_BASE",
                format!("{server}/official-financials"),
            ),
            (
                "EXCEL_SKILL_OFFICIAL_ANNOUNCEMENT_URL_BASE",
                format!("{server}/official-announcements"),
            ),
            (
                "EXCEL_SKILL_SINA_FINANCIAL_URL_BASE",
                format!("{server}/sina-financials"),
            ),
            (
                "EXCEL_SKILL_SINA_ANNOUNCEMENT_URL_BASE",
                format!("{server}/sina-announcements"),
            ),
        ],
    );

    // 2026-04-15 CST: Keep this regression focused on the public fullstack aggregation contract.
    // Purpose: verify the current formal fields that upstream callers actually consume.
    assert_eq!(output["status"], "ok");
    assert_eq!(output["data"]["symbol"], "002352.SZ");
    // 2026-04-15 CST: Updated because capital_flow_context is no longer a stable
    // public contract on the aggregated payload; purpose: lock the still-formal
    // contextual conclusion instead of a nullable internal sub-branch.
    assert_eq!(
        output["data"]["technical_context"]["contextual_conclusion"]["alignment"], "mixed",
        "fullstack output: {output}"
    );
    assert_eq!(output["data"]["fundamental_context"]["status"], "available");
    assert_eq!(
        output["data"]["fundamental_context"]["latest_report_period"],
        "2025-12-31"
    );
    assert_eq!(
        output["data"]["fundamental_context"]["profit_signal"],
        "positive"
    );
    assert_eq!(output["data"]["disclosure_context"]["status"], "available");
    assert_eq!(
        output["data"]["disclosure_context"]["announcement_count"],
        3
    );
    assert_eq!(
        output["data"]["industry_context"]["sector_symbol"],
        "516530.SH"
    );
    assert_eq!(
        output["data"]["integrated_conclusion"]["stance"],
        "watchful_positive"
    );
}

#[test]
fn security_analysis_fullstack_degrades_gracefully_when_info_sources_fail() {
    let runtime_db_path = create_test_runtime_db("security_analysis_fullstack_degraded");

    let stock_csv = create_stock_history_csv(
        "security_analysis_fullstack_degraded",
        "stock.csv",
        &build_range_bound_rows(220, 35.8, 38.0),
    );
    let market_csv = create_stock_history_csv(
        "security_analysis_fullstack_degraded",
        "market.csv",
        &build_confirmed_breakdown_rows(220, 4.9),
    );
    let sector_csv = create_stock_history_csv(
        "security_analysis_fullstack_degraded",
        "sector.csv",
        &build_choppy_history_rows(220, 1.02),
    );
    import_history_csv(&runtime_db_path, &stock_csv, "002352.SZ");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "516530.SH");

    let server = spawn_http_route_server(vec![
        (
            "/capital-flow",
            "HTTP/1.1 200 OK",
            r#"{
                "data":{
                    "symbol":"002352.SZ",
                    "main_net_inflow":12500000.0,
                    "super_order_net_inflow":6200000.0,
                    "headline":"濠电姷鏁搁崑鐐哄垂閸洖绠插〒姘ｅ亾妞ゃ垺淇洪ˇ鎶芥煙娓氬灝濮€缂佺姵鐩鎾偄妞嬪孩鐦掗梻鍌欑閹测€趁洪敃鍌氬瀭闁哄绨遍弸宥夋煏婵炑冨閺嬫垿姊虹紒姗嗘當闁绘锕︽禍鎼佹偋閸粎绠氬銈嗗姉閸犳劕鈻嶆繝鍕ㄥ亾鐟欏嫭绀冮柣鐔叉櫊瀹曟椽鍩€椤掍降浜滈柟鍝勬娴滈箖姊婚崶褜妲搁棁澶愭煥濠靛棙绁╅柣鎺戝⒔閳ь剚顔栭崰妤佺箾婵犲洤钃熼柕濞垮劗濡插牊淇婇婵嗕汗闁告棏鍠氱槐鎾存媴閸撳弶笑闂佸鏉垮妞ゆ洏鍎靛畷鐔碱敋閸涱啩鈺呮煟鎼淬値娼愭繛鍙夌矒瀹曚即寮借閺嗭箓鏌曟竟顖氬€归崟鍐磽閸屾瑩妾烽柛銊潐娣囧﹪鏌嗗鍡忔嫼闂佸憡绋戦敃銈夋倶瀹ュ鐓曢悗锝庡亝鐏忔壆绱掔紒妯笺€掔紒杈ㄧ懇閹晛鈻撻幐骞粓姊?
                }
            }"#,
            "application/json",
        ),
        (
            "/financials",
            "HTTP/1.1 500 Internal Server Error",
            r#"{"error":"financial upstream failed"}"#,
            "application/json",
        ),
        (
            "/announcements",
            "HTTP/1.1 500 Internal Server Error",
            r#"{"error":"announcement upstream failed"}"#,
            "application/json",
        ),
        (
            "/official-financials",
            "HTTP/1.1 200 OK",
            r#"{
                "source":"official_financials",
                "latest_report_period":"2025-12-31",
                "report_notice_date":"2026-03-28",
                "report_metrics":{
                    "revenue":308227000000.0,
                    "revenue_yoy_pct":8.37,
                    "net_profit":11117000000.0,
                    "net_profit_yoy_pct":9.31,
                    "roe_pct":14.8
                }
            }"#,
            "application/json",
        ),
        (
            "/official-announcements",
            "HTTP/1.1 200 OK",
            r#"{
                "source":"official_announcements",
                "recent_announcements":[
                    {
                        "published_at":"2026-03-28",
                        "title":"2025 annual report released",
                        "article_code":"AN202603281234567890",
                        "category":"regular disclosure"
                    },
                    {
                        "published_at":"2026-03-10",
                        "title":"board approves annual dividend plan",
                        "article_code":"AN202603101234567892",
                        "category":"regular disclosure"
                    }
                ]
            }"#,
            "application/json",
        ),
        (
            "/sina-financials",
            "HTTP/1.1 500 Internal Server Error",
            r#"{"error":"sina financial fallback should stay unused in this test"}"#,
            "application/json",
        ),
        (
            "/sina-announcements",
            "HTTP/1.1 500 Internal Server Error",
            r#"{"error":"sina disclosure fallback should stay unused in this test"}"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_analysis_fullstack",
        "args": {
            "symbol": "002352.SZ",
            "market_symbol": "510300.SH",
            "sector_symbol": "516530.SH"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path,
        &[
            // 2026-04-20 CST: Keep the entire provider chain local and explicit.
            // Reason: this degraded-path regression is about controlled fallback order, not ambient env leakage.
            // Purpose: prove EastMoney may fail while official fallback recovers the formal information layers.
            ("EXCEL_SKILL_EASTMONEY_DAILY_LIMIT", "0".to_string()),
            (
                "EXCEL_SKILL_EASTMONEY_CAPITAL_FLOW_URL_BASE",
                format!("{server}/capital-flow"),
            ),
            (
                "EXCEL_SKILL_EASTMONEY_FINANCIAL_URL_BASE",
                format!("{server}/financials"),
            ),
            (
                "EXCEL_SKILL_EASTMONEY_ANNOUNCEMENT_URL_BASE",
                format!("{server}/announcements"),
            ),
            (
                "EXCEL_SKILL_OFFICIAL_FINANCIAL_URL_BASE",
                format!("{server}/official-financials"),
            ),
            (
                "EXCEL_SKILL_OFFICIAL_ANNOUNCEMENT_URL_BASE",
                format!("{server}/official-announcements"),
            ),
            (
                "EXCEL_SKILL_SINA_FINANCIAL_URL_BASE",
                format!("{server}/sina-financials"),
            ),
            (
                "EXCEL_SKILL_SINA_ANNOUNCEMENT_URL_BASE",
                format!("{server}/sina-announcements"),
            ),
        ],
    );
    // 2026-04-15 CST: Keep this degraded-path regression focused on current public fallback behavior.
    // Purpose: verify that the tool stays usable even when preferred info providers fail.
    assert_eq!(output["status"], "ok");
    // 2026-04-15 CST: Updated because degraded fullstack can now recover formal
    // information layers from fallback providers; purpose: verify the current
    // public behavior rather than the retired budget_exhausted branch.
    assert_eq!(
        output["data"]["technical_context"]["contextual_conclusion"]["alignment"], "mixed",
        "fullstack output: {output}"
    );
    assert_eq!(output["data"]["fundamental_context"]["status"], "available");
    assert_eq!(output["data"]["disclosure_context"]["status"], "available");
    assert_eq!(
        output["data"]["integrated_conclusion"]["stance"],
        "watchful_positive"
    );
    assert_eq!(
        output["data"]["integrated_conclusion"]["risk_flags"]
            .as_array()
            .expect("risk flags should exist")
            .len(),
        0,
        "fullstack output: {output}"
    );
}

#[test]
fn security_analysis_fullstack_synthesizes_etf_information_from_governed_proxy_history() {
    let runtime_db_path = create_test_runtime_db("security_analysis_fullstack_etf_info");
    let external_proxy_db_path = runtime_db_path
        .parent()
        .expect("runtime db should have parent")
        .join("security_external_proxy.db");

    let etf_csv = create_stock_history_csv(
        "security_analysis_fullstack_etf_info",
        "gold_etf.csv",
        &build_confirmed_breakout_rows(260, 101.0),
    );
    let market_csv = create_stock_history_csv(
        "security_analysis_fullstack_etf_info",
        "market.csv",
        &build_confirmed_breakout_rows(260, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_analysis_fullstack_etf_info",
        "sector.csv",
        &build_confirmed_breakout_rows(260, 99.0),
    );
    import_history_csv(&runtime_db_path, &etf_csv, "518880.SH");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "518800.SH");

    let backfill_request = json!({
        "tool": "security_external_proxy_backfill",
        "args": {
            "batch_id": "fullstack-etf-info",
            "created_at": "2026-04-13T10:00:00+08:00",
            "records": [{
                "symbol": "518880.SH",
                "as_of_date": "2025-08-08",
                "instrument_subscope": "gold_etf",
                "external_proxy_inputs": {
                    "gold_spot_proxy_status": "manual_bound",
                    "gold_spot_proxy_return_5d": 0.021019,
                    "usd_index_proxy_status": "manual_bound",
                    "usd_index_proxy_return_5d": -0.003841,
                    "real_rate_proxy_status": "manual_bound",
                    "real_rate_proxy_delta_bp_5d": -2.0
                }
            }]
        }
    });
    let backfill_output = run_cli_with_json_runtime_and_envs(
        &backfill_request.to_string(),
        &runtime_db_path,
        &[(
            "EXCEL_SKILL_EXTERNAL_PROXY_DB",
            external_proxy_db_path.to_string_lossy().to_string(),
        )],
    );
    assert_eq!(backfill_output["status"], "ok");

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "<html><body>financials unavailable for ETF fixture</body></html>",
            "text/html",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{"data":{"list":[]}}"#,
            "application/json",
        ),
    ]);

    let request = json!({
        "tool": "security_analysis_fullstack",
        "args": {
            "symbol": "518880.SH",
            "market_symbol": "510300.SH",
            "sector_symbol": "518800.SH",
            "market_profile": "a_share_core",
            "sector_profile": "gold_etf_peer",
            "as_of_date": "2025-08-08"
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
            (
                "EXCEL_SKILL_EXTERNAL_PROXY_DB",
                external_proxy_db_path.to_string_lossy().to_string(),
            ),
        ],
    );

    assert_eq!(
        output["status"], "ok",
        "unexpected ETF fullstack output: {output}"
    );
    // 2026-04-20 CST: Updated because the frozen P10/P11 ETF contract now promotes
    // a complete governed proxy family into the formal ETF information surface.
    // Reason: chair/committee closeout now depends on fullstack exposing governed
    // ETF info instead of preserving the older stock-only downgrade behavior.
    // Purpose: lock the public fullstack contract to the governed ETF proxy source.
    assert_eq!(
        output["data"]["fundamental_context"]["status"], "available",
        "fullstack output: {output}"
    );
    assert_eq!(
        output["data"]["fundamental_context"]["source"],
        "governed_etf_proxy_information"
    );
    assert_eq!(
        output["data"]["disclosure_context"]["status"], "available",
        "fullstack output: {output}"
    );
    assert_eq!(
        output["data"]["disclosure_context"]["source"],
        "governed_etf_proxy_information"
    );
    assert_eq!(
        output["data"]["etf_context"]["status"], "available",
        "fullstack output: {output}"
    );
    assert_eq!(
        output["data"]["etf_context"]["source"], "governed_etf_proxy_information",
        "fullstack output: {output}"
    );
    assert_eq!(
        output["data"]["integrated_conclusion"]["stance"],
        "constructive"
    );
}

#[test]
fn security_analysis_fullstack_auto_maps_159866_cross_border_inputs_and_uses_builtin_etf_facts() {
    let runtime_db_path =
        create_test_runtime_db("security_analysis_fullstack_cross_border_defaults");

    let etf_csv = create_stock_history_csv(
        "security_analysis_fullstack_cross_border_defaults",
        "etf.csv",
        &build_range_bound_rows(260, 1.02, 1.48),
    );
    let market_csv = create_stock_history_csv(
        "security_analysis_fullstack_cross_border_defaults",
        "market.csv",
        &build_confirmed_breakout_rows(260, 3200.0),
    );
    let sector_csv = create_stock_history_csv(
        "security_analysis_fullstack_cross_border_defaults",
        "sector.csv",
        &build_confirmed_breakout_rows(260, 42.0),
    );
    import_history_csv(&runtime_db_path, &etf_csv, "159866.SZ");
    import_history_csv(&runtime_db_path, &market_csv, "510300.SH");
    import_history_csv(&runtime_db_path, &sector_csv, "513520.SH");

    // 2026-04-15 CST: Added because scheme B-2 requires cross-border ETF tests
    // to own their FRED fixtures locally instead of leaking them from other tests.
    // Purpose: keep the contract deterministic and provide enough history for
    // technical windows on NK225.IDX and JPYCNY.FX.
    let fred_csv = build_fred_csv_ending_at("2026-04-14", 320, 52000.0, 18.0);
    let fred_cny_csv = build_fred_csv_ending_at("2026-04-14", 320, 6.4, 0.0015);

    let server = spawn_http_route_server(vec![
        (
            "/financials",
            "HTTP/1.1 406 Not Acceptable",
            "{\"error\":\"financials unavailable for etf\"}",
            "application/json",
        ),
        (
            "/announcements",
            "HTTP/1.1 200 OK",
            r#"{"data":{"list":[{"notice_date":"2026-04-14","title":"ETF routine disclosure","art_code":"ETF2026041401","columns":[{"column_name":"ETF notice"}]}]}}"#,
            "application/json",
        ),
        ("/fred", "HTTP/1.1 200 OK", &fred_csv, "text/csv"),
        ("/fred-cny", "HTTP/1.1 200 OK", &fred_cny_csv, "text/csv"),
    ]);

    let request = json!({
        "tool": "security_analysis_fullstack",
        "args": {
            "symbol": "159866.SZ",
            "market_symbol": "510300.SH",
            "sector_symbol": "513520.SH",
            "market_profile": "cross_border_qdii",
            "sector_profile": "nikkei_qdii_cross_border_peer",
            "as_of_date": "2026-04-15",
            "disclosure_limit": 1
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
            (
                "EXCEL_SKILL_OFFICIAL_FINANCIAL_URL_BASE",
                format!("{server}/financials"),
            ),
            (
                "EXCEL_SKILL_OFFICIAL_ANNOUNCEMENT_URL_BASE",
                format!("{server}/announcements"),
            ),
            (
                "EXCEL_SKILL_SINA_FINANCIAL_URL_BASE",
                format!("{server}/financials"),
            ),
            (
                "EXCEL_SKILL_SINA_ANNOUNCEMENT_URL_BASE",
                format!("{server}/announcements"),
            ),
            ("EXCEL_SKILL_FRED_CSV_URL_BASE", format!("{server}/fred")),
            (
                "EXCEL_SKILL_FRED_DERIVED_DEXCHUS_URL_BASE",
                format!("{server}/fred-cny"),
            ),
        ],
    );

    // 2026-04-15 CST: Added because scheme B-2 must stop requiring operators
    // to hand-pass Nikkei cross-border mapping inputs for 159866.SZ.
    // Purpose: prove the formal fullstack path can auto-bind underlying/FX defaults
    // and still expose ETF facts when no ETF facts URL env is configured.
    assert_eq!(output["status"], "ok", "fullstack output: {output}");
    assert_eq!(output["data"]["etf_context"]["status"], "available");
    // 2026-04-15 CST: Updated because the builtin ETF facts fallback now returns
    // normalized UTF-8 benchmark text; purpose: guard the real customer-facing
    // string instead of a historical mojibake artifact.
    assert_eq!(
        output["data"]["etf_context"]["benchmark"],
        "\u{65e5}\u{7ecf}225\u{6307}\u{6570}"
    );
    assert_eq!(
        output["data"]["cross_border_context"]["underlying_market"]["symbol"],
        "NK225.IDX"
    );
    assert_eq!(
        output["data"]["cross_border_context"]["fx_market"]["symbol"],
        "JPYCNY.FX"
    );
    assert_eq!(
        output["data"]["cross_border_context"]["underlying_market"]["status"], "available",
        "fullstack output: {output}"
    );
    assert_eq!(
        output["data"]["cross_border_context"]["fx_market"]["status"], "available",
        "fullstack output: {output}"
    );
    assert_eq!(
        output["data"]["cross_border_context"]["status"], "available",
        "fullstack output: {output}"
    );
}

// 2026-04-01 CST: Shared history import helper for fullstack regressions.
fn import_history_csv(runtime_db_path: &Path, csv_path: &Path, symbol: &str) {
    let request = json!({
        "tool": "import_stock_price_history",
        "args": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": symbol,
            "source": "security_analysis_fullstack_fixture"
        }
    });

    let output = run_cli_with_json_runtime_and_envs(
        &request.to_string(),
        &runtime_db_path.to_path_buf(),
        &[],
    );
    assert_eq!(output["status"], "ok");
}

// 2026-04-15 CST: Added because cross-border ETF fullstack tests now need a
// long-enough FRED-style observation series for technical indicators to compute.
// Purpose: generate deterministic CSV bodies for official free-source fixtures.
fn build_fred_csv_ending_at(
    end_date: &str,
    day_count: usize,
    start_value: f64,
    step: f64,
) -> String {
    let end_date =
        NaiveDate::parse_from_str(end_date, "%Y-%m-%d").expect("end_date should be valid");
    let start_date = end_date - Duration::days((day_count.saturating_sub(1)) as i64);
    let mut lines = vec!["DATE,VALUE".to_string()];

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let value = start_value + offset as f64 * step;
        lines.push(format!("{},{}", trade_date.format("%Y-%m-%d"), value));
    }

    lines.join("\n")
}

// 2026-04-01 CST: 闂傚倷绀侀幖顐λ囬锕€鐤炬繝濠傜墛閸嬶繝鏌嶉崫鍕櫣闂傚偆鍨伴—鍐偓锝庝簻椤掋垽鏌涚€ｎ偓鑰块柡灞炬礃瀵板嫰宕煎┑鍡楊潛闂傚倸鍊搁幊蹇涙晝閵忕媭娼栧┑鐘宠壘绾惧吋绻涢幋鐐垫噧闁哥偛顦埞鎴︽倷鐎涙ê纰嶅┑鐐点€嬬换婵嬬嵁婵犲洦鏅搁柣妯垮皺閸旀挳姊洪崨濠傚Е濞存粍绮撹棢閹兼番鍔嶉埛鎴︽煕濠靛棗顏褔浜堕弻娑㈠箻閸楃偛顬嬪┑鈥冲级閸旀洟鍩為幋鐘亾閿濆簼绨婚柛瀣Ч濮婅櫣绱掑Ο娲绘闂佽　鍋撻弶鍫氭櫇閻棝鏌涢鐘插姕闁绘挻鐩弻娑㈠即閵忊剝閿梺璇查獜缁犳捇寮婚弴銏犲耿婵°倐鍋撴い鎺嬪灮缁辨帞鈧綆浜跺Ο鈧悗娈垮枛婢у酣骞戦崟顖毼╂い顒夊櫘娴滎亜顫忓ú顏勫窛濠电姴鍟喊宥囩磽娴ｅ壊妲兼繛澶嬬洴閺佸啴濮€閵堝棙娅栭梺鍛婃磸閸斿本绂嶆ィ鍐╃厱闁挎棁顕ч獮鏍归悩宕囩煁缂佺粯绻嗛ˇ顕€鏌涚€ｎ偅宕屾慨濠冩そ瀹曨偊宕熼鈧娑㈡⒑閹肩偛濡肩€规洦鍓涢崣鍛存⒑缁夊棗瀚峰▓鏃傜磼閻樺啿鈻曢柡灞炬礃瀵板嫰宕煎┑鍡╃€风紓鍌欒兌婵參宕抽敐澶婅摕鐎广儱顦敮闂佹寧鏌ㄦ晶浠嬫偂閳ь剟姊虹拠鑼婵炲瓨宀稿畷鍦崉娓氼垱缍庨梺鐓庮潟閸婃绮堥崟顖涚厽婵☆垰鍚嬮弳鈺呮煙瀹割喕绨婚柍瑙勫灴閹瑩鎳犵捄渚純闂備浇顕х换鎴犳崲閸曨偀鏋庨柕蹇嬪€栭崑鍕煕韫囨艾浜归柛鏂跨仛缁绘繈濮€閿濆棛銆愬┑鐐差嚟閸忔﹢骞冮悽绋块唶闁哄洨鍠撻崢鎾绘⒑闂堟侗妾у┑鈥虫川缁顫濋懜鐢靛帗閻熸粍绮撳畷婊冣枎閹邦噣妾梺鍝勫暙閸婅崵绮婚弬娆剧唵闁兼悂娼ф慨鍥煕濞嗗骏鏀婚柕鍥у瀵粙濡搁妸銉缂傚倷璁查崑鎾绘倵閿濆骸鏋涚紒鐙欏洦鐓曟い鎰剁悼缁犳牠鎮归幇鍓佺瘈闁哄矉绲介埥澶婎潨閸絽鎯堥梻浣筋嚃閸ㄦ壆绮旈棃娑辩劷闊洦绋戠粈鍫㈡喐瀹ュ鍤€?
// 2026-04-01 CST: Range-bound stock fixture helper for fullstack regressions.
fn build_range_bound_rows(day_count: usize, support: f64, resistance: f64) -> Vec<String> {
    let mut rows = vec!["trade_date,open,high,low,close,adj_close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("seed date should be valid");
    let span = resistance - support;

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let phase = offset % 12;
        let anchor = match phase {
            0 => support + span * 0.24,
            1 => support + span * 0.37,
            2 => support + span * 0.44,
            3 => support + span * 0.58,
            4 => support + span * 0.66,
            5 => support + span * 0.73,
            6 => support + span * 0.62,
            7 => support + span * 0.55,
            8 => support + span * 0.68,
            9 => support + span * 0.81,
            10 => support + span * 0.93,
            _ => support + span * 0.88,
        };
        let open = anchor - 0.18;
        let close = anchor + if phase >= 9 { 0.12 } else { 0.05 };
        let high = resistance.min(close + 0.16);
        let low = support.max(open - 0.18);
        let volume = 180_000 + (offset % 7) as i64 * 12_000;
        rows.push(format!(
            "{},{open:.2},{high:.2},{low:.2},{close:.2},{close:.2},{volume}",
            trade_date.format("%Y-%m-%d")
        ));
    }

    rows
}

// 2026-04-01 CST: Choppy sector proxy fixture helper for fullstack regressions.
fn build_choppy_history_rows(day_count: usize, base: f64) -> Vec<String> {
    let mut rows = vec!["trade_date,open,high,low,close,adj_close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("seed date should be valid");

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let wave = match offset % 6 {
            0 => -0.008,
            1 => 0.007,
            2 => -0.006,
            3 => 0.006,
            4 => -0.007,
            _ => 0.008,
        };
        let close = base + wave;
        let open = close - 0.003;
        let high = close + 0.012;
        let low = close - 0.012;
        let volume = 4_000_000 + (offset % 5) as i64 * 180_000;
        rows.push(format!(
            "{},{open:.3},{high:.3},{low:.3},{close:.3},{close:.3},{volume}",
            trade_date.format("%Y-%m-%d")
        ));
    }

    rows
}

// 2026-04-01 CST: Historical market breakdown fixture helper for fullstack regressions.
fn build_confirmed_breakdown_rows(day_count: usize, start_close: f64) -> Vec<String> {
    let mut rows = vec!["trade_date,open,high,low,close,adj_close,volume".to_string()];
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("seed date should be valid");
    let mut close = start_close;

    for offset in 0..day_count {
        let trade_date = start_date + Duration::days(offset as i64);
        let (next_close, volume): (f64, i64) = if offset < day_count - 20 {
            (close - 0.007, 8_600_000 + offset as i64 * 18_000)
        } else {
            let phase = offset - (day_count - 20);
            match phase % 4 {
                0 => (close - 0.013, 16_600_000 + phase as i64 * 40_000),
                1 => (close + 0.002, 4_100_000),
                2 => (close - 0.011, 15_200_000 + phase as i64 * 36_000),
                _ => (close - 0.004, 12_000_000),
            }
        };

        let open = close;
        let high = next_close.max(open) + 0.008;
        let low = next_close.min(open) - 0.012;
        rows.push(format!(
            "{},{open:.3},{high:.3},{low:.3},{next_close:.3},{next_close:.3},{volume}",
            trade_date.format("%Y-%m-%d")
        ));
        close = next_close;
    }

    rows
}

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
