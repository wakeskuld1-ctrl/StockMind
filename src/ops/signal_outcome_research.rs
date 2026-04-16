use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

use crate::ops::stock::security_analysis_resonance::{
    SecurityAnalysisResonanceError, SecurityAnalysisResonanceRequest, security_analysis_resonance,
};
use crate::runtime::signal_outcome_store::{
    SecuritySignalAnalogStudyRow, SecuritySignalForwardReturnRow, SecuritySignalSnapshotRow,
    SecuritySignalTagRow, SignalOutcomeStore, SignalOutcomeStoreError,
};
use crate::runtime::stock_history_store::{StockHistoryStore, StockHistoryStoreError};

// 2026-04-02 CST: 这里先定义 snapshot 记录请求，原因是方案C第一批任务要把“统一研究快照”独立成正式 Tool，
// 目的：让咨询、研究和后续投决都通过同一个 request 合同进入，而不是复用临时脚本参数。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RecordSecuritySignalSnapshotRequest {
    pub symbol: String,
    #[serde(default)]
    pub market_symbol: Option<String>,
    #[serde(default)]
    pub sector_symbol: Option<String>,
    pub market_regime: String,
    pub sector_template: String,
    #[serde(default)]
    pub as_of_date: Option<String>,
    #[serde(default = "default_lookback_days")]
    pub lookback_days: usize,
    #[serde(default = "default_factor_lookback_days")]
    pub factor_lookback_days: usize,
    #[serde(default = "default_disclosure_limit")]
    pub disclosure_limit: usize,
}

// 2026-04-02 CST: 这里把 snapshot Tool 响应收口成结构化结果，原因是研究平台不仅要落库，也要把当前这次入库的事实直接回给上层，
// 目的：让 Skill / Agent 无需额外再去扫库确认刚刚写入了什么。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecordSecuritySignalSnapshotResult {
    pub symbol: String,
    pub snapshot_date: String,
    pub indicator_digest: String,
    pub resonance_score: f64,
    pub action_bias: String,
    pub indicator_snapshot: Value,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BackfillSecuritySignalOutcomesRequest {
    pub symbol: String,
    #[serde(default)]
    pub snapshot_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BackfillSecuritySignalOutcomesResult {
    pub symbol: String,
    pub snapshot_date: String,
    pub forward_returns: Vec<SecuritySignalForwardReturnRow>,
}

// 2026-04-02 CST: 这里定义历史相似研究请求，原因是用户明确要求“银行板块共振 + MACD/RSRS 等核心技术形态”
// 要一起进入历史样本库，而不是只做单只股票复盘；目的：让 Agent/Skill 能用正式 Tool 合同发起银行体系 analog study。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StudySecuritySignalAnalogsRequest {
    pub symbol: String,
    #[serde(default)]
    pub snapshot_date: Option<String>,
    #[serde(default)]
    pub comparison_symbols: Vec<String>,
    #[serde(default = "default_study_key")]
    pub study_key: String,
    #[serde(default = "default_min_similarity_score")]
    pub min_similarity_score: f64,
    #[serde(default = "default_sample_limit")]
    pub sample_limit: usize,
}

// 2026-04-02 CST: 这里补 `SecuritySignalAnalogMatch` 的反序列化能力，原因是 analog study summary 需要把持久化 JSON 重新读回 research summary / briefing 主链；
// 目的：让历史研究层可以稳定从数据库摘要反解出 matched_analogs，而不是只支持写入不支持回读。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecuritySignalAnalogMatch {
    pub symbol: String,
    pub snapshot_date: String,
    pub similarity_score: f64,
    pub matched_tag_count: usize,
    pub matched_tags: Vec<String>,
    pub forward_return_10d: f64,
    pub max_drawdown_10d: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StudySecuritySignalAnalogsResult {
    pub symbol: String,
    pub snapshot_date: String,
    pub study_key: String,
    pub sample_count: usize,
    pub win_rate_10d: f64,
    pub avg_return_10d: f64,
    pub median_return_10d: f64,
    pub expected_return_window: String,
    pub expected_drawdown_window: String,
    pub matched_analogs: Vec<SecuritySignalAnalogMatch>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SignalOutcomeResearchSummaryRequest {
    pub symbol: String,
    #[serde(default)]
    pub snapshot_date: Option<String>,
    #[serde(default = "default_study_key")]
    pub study_key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SignalOutcomeResearchSummaryResult {
    pub symbol: String,
    pub snapshot_date: String,
    pub study_key: String,
    pub status: String,
    pub historical_confidence: String,
    pub analog_sample_count: usize,
    pub analog_win_rate_10d: Option<f64>,
    pub analog_loss_rate_10d: Option<f64>,
    pub analog_flat_rate_10d: Option<f64>,
    pub analog_avg_return_10d: Option<f64>,
    pub analog_median_return_10d: Option<f64>,
    pub analog_avg_win_return_10d: Option<f64>,
    pub analog_avg_loss_return_10d: Option<f64>,
    pub analog_payoff_ratio_10d: Option<f64>,
    pub analog_expectancy_10d: Option<f64>,
    pub expected_return_window: Option<String>,
    pub expected_drawdown_window: Option<String>,
    pub research_limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
struct AnalogStudySummaryPayload {
    pub expected_return_window: String,
    pub expected_drawdown_window: String,
    pub matched_analogs: Vec<SecuritySignalAnalogMatch>,
}

#[derive(Debug, Error)]
pub enum SignalOutcomeResearchError {
    #[error("{0}")]
    Resonance(#[from] SecurityAnalysisResonanceError),
    #[error("{0}")]
    Store(#[from] SignalOutcomeStoreError),
    #[error("{0}")]
    StockHistory(#[from] StockHistoryStoreError),
    #[error("无法序列化信号快照: {0}")]
    SerializeSnapshot(String),
    #[error("证券 `{symbol}` 不存在可回填的信号快照")]
    MissingSnapshot { symbol: String },
    #[error("证券 `{symbol}` 在快照日 `{snapshot_date}` 之后缺少足够历史数据")]
    MissingForwardHistory {
        symbol: String,
        snapshot_date: String,
    },
    #[error("证券 `{symbol}` 在快照日 `{snapshot_date}` 缺少可用标签")]
    MissingTags {
        symbol: String,
        snapshot_date: String,
    },
    #[error("证券 `{symbol}` 在快照日 `{snapshot_date}` 缺少 10 日未来收益")]
    MissingTenDayOutcome {
        symbol: String,
        snapshot_date: String,
    },
    #[error("证券 `{symbol}` 在快照日 `{snapshot_date}` 缺少可用历史相似研究 `{study_key}`")]
    MissingAnalogStudy {
        symbol: String,
        snapshot_date: String,
        study_key: String,
    },
}

// 2026-04-02 CST: 这里先实现 Task 2 的最小 research 入口，原因是用户明确要求高阶指标、RSRS 和共振层必须一起进入统一底稿，
// 目的：复用已存在的 resonance/fullstack 主链，一次性产出“技术面完整快照 + 共振评分 + 动作偏向”并落到研究库。
pub fn record_security_signal_snapshot(
    request: &RecordSecuritySignalSnapshotRequest,
) -> Result<RecordSecuritySignalSnapshotResult, SignalOutcomeResearchError> {
    let analysis = security_analysis_resonance(&SecurityAnalysisResonanceRequest {
        symbol: request.symbol.clone(),
        market_symbol: request.market_symbol.clone(),
        sector_symbol: request.sector_symbol.clone(),
        market_regime: request.market_regime.clone(),
        sector_template: request.sector_template.clone(),
        as_of_date: request.as_of_date.clone(),
        lookback_days: request.lookback_days,
        factor_lookback_days: request.factor_lookback_days,
        disclosure_limit: request.disclosure_limit,
    })?;

    let snapshot_date = analysis.resonance_context.snapshot_date.clone();
    let resonance_score = analysis.resonance_context.resonance_score;
    let action_bias = analysis.resonance_context.action_bias.clone();

    let base_snapshot = serde_json::to_value(
        &analysis
            .base_analysis
            .technical_context
            .stock_analysis
            .indicator_snapshot,
    )
    .map_err(|error| SignalOutcomeResearchError::SerializeSnapshot(error.to_string()))?;
    let indicator_snapshot =
        enrich_indicator_snapshot(base_snapshot, resonance_score, &action_bias);
    let indicator_digest = build_indicator_digest(&indicator_snapshot);
    let snapshot_payload = serde_json::to_string(&indicator_snapshot)
        .map_err(|error| SignalOutcomeResearchError::SerializeSnapshot(error.to_string()))?;

    let store = SignalOutcomeStore::workspace_default()?;
    store.upsert_snapshot(&SecuritySignalSnapshotRow {
        symbol: request.symbol.clone(),
        snapshot_date: snapshot_date.clone(),
        indicator_digest: indicator_digest.clone(),
        resonance_score,
        action_bias: action_bias.clone(),
        snapshot_payload,
    })?;
    store.replace_tags(
        &request.symbol,
        &snapshot_date,
        &build_signal_tags(
            &request.symbol,
            &snapshot_date,
            &indicator_snapshot,
            &request.market_regime,
            &request.sector_template,
        ),
    )?;

    Ok(RecordSecuritySignalSnapshotResult {
        symbol: request.symbol.clone(),
        snapshot_date,
        indicator_digest,
        resonance_score,
        action_bias,
        indicator_snapshot,
    })
}

// 2026-04-02 CST: 这里实现 forward returns 最小回填入口，原因是方案C要求平台不仅解释“现在”，还要沉淀“类似状态后面通常怎么走”，
// 目的：先把固定 1/3/5/10/20 日收益、最大回撤和最大上冲回填出来，作为 analog study 和 briefing 的事实底座。
pub fn backfill_security_signal_outcomes(
    request: &BackfillSecuritySignalOutcomesRequest,
) -> Result<BackfillSecuritySignalOutcomesResult, SignalOutcomeResearchError> {
    let store = SignalOutcomeStore::workspace_default()?;
    let snapshot_date = match request.snapshot_date.as_deref() {
        Some(snapshot_date) => snapshot_date.to_string(),
        None => store
            .latest_snapshot_date(&request.symbol)?
            .ok_or_else(|| SignalOutcomeResearchError::MissingSnapshot {
                symbol: request.symbol.clone(),
            })?,
    };
    let snapshot = store
        .load_snapshot(&request.symbol, &snapshot_date)?
        .ok_or_else(|| SignalOutcomeResearchError::MissingSnapshot {
            symbol: request.symbol.clone(),
        })?;

    let stock_store = StockHistoryStore::workspace_default()?;
    let baseline_rows =
        stock_store.load_recent_rows(&request.symbol, Some(&snapshot.snapshot_date), 1)?;
    let Some(base_row) = baseline_rows.last() else {
        return Err(SignalOutcomeResearchError::MissingSnapshot {
            symbol: request.symbol.clone(),
        });
    };
    let future_rows = stock_store.load_rows_after(&request.symbol, &snapshot.snapshot_date, 20)?;
    if future_rows.is_empty() {
        return Err(SignalOutcomeResearchError::MissingForwardHistory {
            symbol: request.symbol.clone(),
            snapshot_date: snapshot.snapshot_date.clone(),
        });
    }

    let forward_returns = [1_i64, 3, 5, 10, 20]
        .into_iter()
        .filter_map(|horizon_days| {
            build_forward_return_row(
                &request.symbol,
                &snapshot.snapshot_date,
                base_row.close,
                &future_rows,
                horizon_days,
            )
        })
        .collect::<Vec<_>>();
    if forward_returns.is_empty() {
        return Err(SignalOutcomeResearchError::MissingForwardHistory {
            symbol: request.symbol.clone(),
            snapshot_date: snapshot.snapshot_date.clone(),
        });
    }

    store.replace_forward_returns(&request.symbol, &snapshot.snapshot_date, &forward_returns)?;

    Ok(BackfillSecuritySignalOutcomesResult {
        symbol: request.symbol.clone(),
        snapshot_date: snapshot.snapshot_date,
        forward_returns,
    })
}

// 2026-04-02 CST: 这里实现历史相似研究主入口，原因是用户明确要求银行板块层面要把宏观共振与 MACD/RSRS 等核心技术
// 一起做成历史样本库，而不是停留在“当前状态解释”；目的：把银行体系内相似状态后的 10 日收益/回撤统计沉成正式研究资产。
pub fn study_security_signal_analogs(
    request: &StudySecuritySignalAnalogsRequest,
) -> Result<StudySecuritySignalAnalogsResult, SignalOutcomeResearchError> {
    let store = SignalOutcomeStore::workspace_default()?;
    let snapshot_date = match request.snapshot_date.as_deref() {
        Some(snapshot_date) => snapshot_date.to_string(),
        None => store
            .latest_snapshot_date(&request.symbol)?
            .ok_or_else(|| SignalOutcomeResearchError::MissingSnapshot {
                symbol: request.symbol.clone(),
            })?,
    };
    let target_snapshot = store
        .load_snapshot(&request.symbol, &snapshot_date)?
        .ok_or_else(|| SignalOutcomeResearchError::MissingSnapshot {
            symbol: request.symbol.clone(),
        })?;
    let target_tags = store.load_tags(&request.symbol, &snapshot_date)?;
    if target_tags.is_empty() {
        return Err(SignalOutcomeResearchError::MissingTags {
            symbol: request.symbol.clone(),
            snapshot_date,
        });
    }

    let comparison_symbols = if request.comparison_symbols.is_empty() {
        vec![request.symbol.clone()]
    } else {
        request.comparison_symbols.clone()
    };
    let candidate_snapshots = store.load_snapshots_for_symbols_before(
        &comparison_symbols,
        &target_snapshot.snapshot_date,
        request.sample_limit.max(12) * 6,
    )?;

    let mut matched = Vec::new();
    for candidate_snapshot in candidate_snapshots {
        let candidate_tags = store.load_tags(
            &candidate_snapshot.symbol,
            &candidate_snapshot.snapshot_date,
        )?;
        if candidate_tags.is_empty() {
            continue;
        }
        let Some(candidate_outcome_10d) = store
            .load_forward_returns(
                &candidate_snapshot.symbol,
                &candidate_snapshot.snapshot_date,
            )?
            .into_iter()
            .find(|row| row.horizon_days == 10)
        else {
            continue;
        };

        let similarity = compute_similarity_score(
            &target_snapshot.snapshot_payload,
            &target_tags,
            &candidate_snapshot.snapshot_payload,
            &candidate_tags,
        )?;
        if similarity.score < request.min_similarity_score {
            continue;
        }

        matched.push(SecuritySignalAnalogMatch {
            symbol: candidate_snapshot.symbol,
            snapshot_date: candidate_snapshot.snapshot_date,
            similarity_score: round_ratio(similarity.score),
            matched_tag_count: similarity.matched_tags.len(),
            matched_tags: similarity.matched_tags,
            forward_return_10d: candidate_outcome_10d.forward_return_pct,
            max_drawdown_10d: candidate_outcome_10d.max_drawdown_pct,
        });
    }

    matched.sort_by(|left, right| right.similarity_score.total_cmp(&left.similarity_score));
    matched.truncate(request.sample_limit.max(1));

    let forward_returns = matched
        .iter()
        .map(|item| item.forward_return_10d)
        .collect::<Vec<_>>();
    let drawdowns = matched
        .iter()
        .map(|item| item.max_drawdown_10d)
        .collect::<Vec<_>>();
    let sample_count = matched.len();
    let win_rate_10d = if sample_count == 0 {
        0.0
    } else {
        matched
            .iter()
            .filter(|item| item.forward_return_10d > 0.0)
            .count() as f64
            / sample_count as f64
    };
    let avg_return_10d = average(&forward_returns);
    let median_return_10d = median(&forward_returns);
    let expected_return_window =
        format_return_window("10日收益", avg_return_10d, median_return_10d);
    let expected_drawdown_window =
        format_return_window("10日回撤", average(&drawdowns), median(&drawdowns));

    let summary_payload = AnalogStudySummaryPayload {
        expected_return_window: expected_return_window.clone(),
        expected_drawdown_window: expected_drawdown_window.clone(),
        matched_analogs: matched.clone(),
    };
    store.upsert_analog_study(&SecuritySignalAnalogStudyRow {
        symbol: request.symbol.clone(),
        snapshot_date: target_snapshot.snapshot_date.clone(),
        study_key: request.study_key.clone(),
        sample_count: sample_count as i64,
        win_rate: win_rate_10d,
        avg_return_pct: avg_return_10d,
        median_return_pct: median_return_10d,
        summary_payload: serde_json::to_string(&summary_payload)
            .map_err(|error| SignalOutcomeResearchError::SerializeSnapshot(error.to_string()))?,
    })?;

    Ok(StudySecuritySignalAnalogsResult {
        symbol: request.symbol.clone(),
        snapshot_date: target_snapshot.snapshot_date,
        study_key: request.study_key.clone(),
        sample_count,
        win_rate_10d,
        avg_return_10d,
        median_return_10d,
        expected_return_window,
        expected_drawdown_window,
        matched_analogs: matched,
    })
}

// 2026-04-02 CST: 这里补充研究摘要读取入口，原因是 security_decision_briefing 和 committee payload
// 需要复用正式研究库里的历史统计，而不是在装配阶段重新扫描候选样本；目的：让咨询与投决完全共用同一份历史摘要。
pub fn signal_outcome_research_summary(
    request: &SignalOutcomeResearchSummaryRequest,
) -> Result<SignalOutcomeResearchSummaryResult, SignalOutcomeResearchError> {
    let store = SignalOutcomeStore::workspace_default()?;
    let snapshot_date = match request.snapshot_date.as_deref() {
        Some(snapshot_date) => snapshot_date.to_string(),
        None => store
            .latest_snapshot_date(&request.symbol)?
            .ok_or_else(|| SignalOutcomeResearchError::MissingSnapshot {
                symbol: request.symbol.clone(),
            })?,
    };

    let Some(study_row) =
        store.load_analog_study(&request.symbol, &snapshot_date, &request.study_key)?
    else {
        return Ok(SignalOutcomeResearchSummaryResult {
            symbol: request.symbol.clone(),
            snapshot_date,
            study_key: request.study_key.clone(),
            status: "unavailable".to_string(),
            historical_confidence: "unknown".to_string(),
            analog_sample_count: 0,
            analog_win_rate_10d: None,
            analog_loss_rate_10d: None,
            analog_flat_rate_10d: None,
            analog_avg_return_10d: None,
            analog_median_return_10d: None,
            analog_avg_win_return_10d: None,
            analog_avg_loss_return_10d: None,
            analog_payoff_ratio_10d: None,
            analog_expectancy_10d: None,
            expected_return_window: None,
            expected_drawdown_window: None,
            research_limitations: vec!["历史相似研究尚未生成或样本不足。".to_string()],
        });
    };

    let summary_payload: AnalogStudySummaryPayload =
        serde_json::from_str(&study_row.summary_payload)
            .map_err(|error| SignalOutcomeResearchError::SerializeSnapshot(error.to_string()))?;
    // 2026-04-08 CST: 这里从 matched_analogs 回推赔率层数值，原因是本轮赔率系统要求直接复用正式研究摘要入口，
    // 目的：让 briefing/committee 都通过同一份 summary 获得 win/loss/flat、payoff 与 expectancy，而不是各层自己再扫库重算。
    let forward_returns = summary_payload
        .matched_analogs
        .iter()
        .map(|item| item.forward_return_10d)
        .collect::<Vec<_>>();
    let sample_count = study_row.sample_count.max(0) as usize;
    let win_returns = forward_returns
        .iter()
        .copied()
        .filter(|value| *value > 0.0)
        .collect::<Vec<_>>();
    let loss_returns = forward_returns
        .iter()
        .copied()
        .filter(|value| *value < 0.0)
        .collect::<Vec<_>>();
    let flat_count = forward_returns
        .iter()
        .filter(|value| value.abs() <= f64::EPSILON)
        .count();
    let analog_loss_rate_10d = ratio_option(loss_returns.len(), sample_count);
    let analog_flat_rate_10d = ratio_option(flat_count, sample_count);
    let analog_avg_win_return_10d = average_option(&win_returns);
    let analog_avg_loss_return_10d = average_option(&loss_returns);
    let analog_payoff_ratio_10d = match (analog_avg_win_return_10d, analog_avg_loss_return_10d) {
        (Some(avg_win), Some(avg_loss)) if avg_loss < 0.0 => Some(avg_win / avg_loss.abs()),
        _ => None,
    };
    let analog_expectancy_10d = match (study_row.win_rate, analog_avg_win_return_10d) {
        // 2026-04-08 CST: 这里把平盘收益视为 0，原因是 V1 赔率系统先按 win/loss 两端估算期望值；
        // 目的：在不引入更复杂收益分布模型的前提下，先给上层一个稳定、可解释的期望回报数值。
        (win_rate, Some(avg_win)) => Some(
            win_rate * avg_win
                + analog_loss_rate_10d.unwrap_or(0.0) * analog_avg_loss_return_10d.unwrap_or(0.0),
        ),
        _ => None,
    };
    Ok(SignalOutcomeResearchSummaryResult {
        symbol: study_row.symbol,
        snapshot_date: study_row.snapshot_date,
        study_key: study_row.study_key,
        status: if study_row.sample_count > 0 {
            "available".to_string()
        } else {
            "unavailable".to_string()
        },
        historical_confidence: classify_historical_confidence(study_row.sample_count as usize),
        analog_sample_count: sample_count,
        analog_win_rate_10d: Some(study_row.win_rate),
        analog_loss_rate_10d,
        analog_flat_rate_10d,
        analog_avg_return_10d: Some(study_row.avg_return_pct),
        analog_median_return_10d: Some(study_row.median_return_pct),
        analog_avg_win_return_10d,
        analog_avg_loss_return_10d,
        analog_payoff_ratio_10d,
        analog_expectancy_10d,
        expected_return_window: Some(summary_payload.expected_return_window),
        expected_drawdown_window: Some(summary_payload.expected_drawdown_window),
        research_limitations: if study_row.sample_count > 0 {
            Vec::new()
        } else {
            vec!["历史相似样本为空，暂不输出胜率区间。".to_string()]
        },
    })
}

// 2026-04-02 CST: 这里把共振层字段并入统一 indicator_snapshot，原因是用户明确要求输出不能只像基本面摘要，必须把技术面与共振共同交付，
// 目的：让研究层与 briefing 层都能消费同一份结构，不再出现“技术指标一份、共振解释一份”的双口径。
fn enrich_indicator_snapshot(
    base_snapshot: Value,
    resonance_score: f64,
    action_bias: &str,
) -> Value {
    let mut object = match base_snapshot {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    object.insert("resonance_score".to_string(), Value::from(resonance_score));
    object.insert(
        "action_bias".to_string(),
        Value::String(action_bias.to_string()),
    );
    Value::Object(object)
}

// 2026-04-02 CST: 这里把研究快照压成可检索的 digest，原因是第一版 analog 检索和审计都会先依赖字段名+数值摘要，
// 目的：在不提前引入复杂向量索引的前提下，先提供稳定、可读、可落库的轻量表示。
fn build_indicator_digest(indicator_snapshot: &Value) -> String {
    let Some(object) = indicator_snapshot.as_object() else {
        return String::new();
    };

    let mut keys = object.keys().cloned().collect::<Vec<_>>();
    keys.sort_unstable();
    keys.into_iter()
        .filter_map(|key| object.get(&key).map(|value| (key, value)))
        .map(|(key, value)| format!("{key}={}", digest_value(value)))
        .collect::<Vec<_>>()
        .join("|")
}

fn digest_value(value: &Value) -> String {
    match value {
        Value::Number(number) => {
            let numeric = number.as_f64().unwrap_or_default();
            format!("{numeric:.6}")
        }
        Value::String(text) => text.clone(),
        Value::Bool(flag) => flag.to_string(),
        _ => value.to_string(),
    }
}

fn build_forward_return_row(
    symbol: &str,
    snapshot_date: &str,
    base_close: f64,
    future_rows: &[crate::runtime::stock_history_store::StockHistoryRow],
    horizon_days: i64,
) -> Option<SecuritySignalForwardReturnRow> {
    if base_close.abs() <= f64::EPSILON {
        return None;
    }

    let window_length = horizon_days as usize;
    if future_rows.len() < window_length {
        return None;
    }

    let window = &future_rows[..window_length];
    let end_close = window.last()?.close;
    let min_close = window
        .iter()
        .map(|row| row.close)
        .fold(f64::INFINITY, f64::min);
    let max_close = window
        .iter()
        .map(|row| row.close)
        .fold(f64::NEG_INFINITY, f64::max);

    Some(SecuritySignalForwardReturnRow {
        symbol: symbol.to_string(),
        snapshot_date: snapshot_date.to_string(),
        horizon_days,
        forward_return_pct: (end_close - base_close) / base_close,
        max_drawdown_pct: (min_close - base_close) / base_close,
        max_runup_pct: (max_close - base_close) / base_close,
    })
}

fn default_lookback_days() -> usize {
    180
}

fn default_factor_lookback_days() -> usize {
    120
}

fn default_disclosure_limit() -> usize {
    6
}

fn default_study_key() -> String {
    "bank_resonance_core_technical_v1".to_string()
}

fn default_min_similarity_score() -> f64 {
    0.58
}

fn default_sample_limit() -> usize {
    12
}

// 2026-04-02 CST: 这里把研究快照压成可解释标签集合，原因是用户强调共振不能只给几个英文指标名，
// 要能落成“银行板块强/利率偏正/信用偏弱/MACD偏强/RSRS强化”这种可解释形态；目的：为 analog study 提供稳定标签底座。
fn build_signal_tags(
    symbol: &str,
    snapshot_date: &str,
    indicator_snapshot: &Value,
    market_regime: &str,
    sector_template: &str,
) -> Vec<SecuritySignalTagRow> {
    let mut tags = BTreeMap::new();
    tags.insert("market_regime".to_string(), market_regime.to_string());
    tags.insert("sector_template".to_string(), sector_template.to_string());
    tags.insert(
        "action_bias".to_string(),
        indicator_snapshot
            .get("action_bias")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
    );
    let resonance_score = numeric_field(indicator_snapshot, "resonance_score");
    tags.insert(
        "resonance_bucket".to_string(),
        if resonance_score >= 0.70 {
            "strong_positive".to_string()
        } else if resonance_score <= 0.42 {
            "defensive".to_string()
        } else {
            "balanced".to_string()
        },
    );
    tags.insert(
        "macd_state".to_string(),
        classify_band(
            numeric_field(indicator_snapshot, "macd_histogram"),
            0.05,
            -0.05,
            "bullish",
            "bearish",
            "neutral",
        ),
    );
    tags.insert(
        "rsrs_state".to_string(),
        classify_band(
            numeric_field(indicator_snapshot, "rsrs_zscore_18_60"),
            0.70,
            -0.70,
            "trend_up",
            "trend_down",
            "trend_neutral",
        ),
    );
    tags.insert(
        "trend_strength".to_string(),
        if numeric_field(indicator_snapshot, "adx_14") >= 25.0 {
            "strong".to_string()
        } else if numeric_field(indicator_snapshot, "adx_14") < 20.0 {
            "weak".to_string()
        } else {
            "moderate".to_string()
        },
    );
    tags.insert(
        "volume_state".to_string(),
        if numeric_field(indicator_snapshot, "volume_ratio_20") >= 1.05 {
            "confirmed".to_string()
        } else if numeric_field(indicator_snapshot, "volume_ratio_20") < 0.95 {
            "weak".to_string()
        } else {
            "neutral".to_string()
        },
    );
    tags.insert(
        "breakout_state".to_string(),
        if numeric_field(indicator_snapshot, "close")
            >= numeric_field(indicator_snapshot, "resistance_level_20")
        {
            "at_or_above_resistance".to_string()
        } else if numeric_field(indicator_snapshot, "close")
            >= numeric_field(indicator_snapshot, "ema_10")
        {
            "pullback_confirmation".to_string()
        } else {
            "below_confirmation".to_string()
        },
    );
    tags.insert(
        "money_flow_state".to_string(),
        if numeric_field(indicator_snapshot, "mfi_14") >= 80.0 {
            "overbought".to_string()
        } else if numeric_field(indicator_snapshot, "mfi_14") <= 20.0 {
            "oversold".to_string()
        } else {
            "neutral".to_string()
        },
    );

    tags.into_iter()
        .map(|(tag_key, tag_value)| SecuritySignalTagRow {
            symbol: symbol.to_string(),
            snapshot_date: snapshot_date.to_string(),
            tag_key,
            tag_value,
        })
        .collect()
}

struct SimilarityScore {
    score: f64,
    matched_tags: Vec<String>,
}

fn compute_similarity_score(
    target_payload: &str,
    target_tags: &[SecuritySignalTagRow],
    candidate_payload: &str,
    candidate_tags: &[SecuritySignalTagRow],
) -> Result<SimilarityScore, SignalOutcomeResearchError> {
    let target_value: Value = serde_json::from_str(target_payload)
        .map_err(|error| SignalOutcomeResearchError::SerializeSnapshot(error.to_string()))?;
    let candidate_value: Value = serde_json::from_str(candidate_payload)
        .map_err(|error| SignalOutcomeResearchError::SerializeSnapshot(error.to_string()))?;

    let target_tag_map = target_tags
        .iter()
        .map(|row| (row.tag_key.as_str(), row.tag_value.as_str()))
        .collect::<BTreeMap<_, _>>();
    let candidate_tag_map = candidate_tags
        .iter()
        .map(|row| (row.tag_key.as_str(), row.tag_value.as_str()))
        .collect::<BTreeMap<_, _>>();

    let matched_tags = target_tag_map
        .iter()
        .filter_map(|(key, value)| {
            candidate_tag_map
                .get(key)
                .filter(|candidate_value| *candidate_value == value)
                .map(|_| format!("{key}={value}"))
        })
        .collect::<Vec<_>>();
    let tag_score = if target_tag_map.is_empty() {
        0.0
    } else {
        matched_tags.len() as f64 / target_tag_map.len() as f64
    };

    let numeric_score = [
        closeness(
            numeric_field(&target_value, "resonance_score"),
            numeric_field(&candidate_value, "resonance_score"),
            0.18,
        ),
        closeness(
            numeric_field(&target_value, "macd_histogram"),
            numeric_field(&candidate_value, "macd_histogram"),
            0.25,
        ),
        closeness(
            numeric_field(&target_value, "rsrs_zscore_18_60"),
            numeric_field(&candidate_value, "rsrs_zscore_18_60"),
            0.80,
        ),
        closeness(
            numeric_field(&target_value, "adx_14"),
            numeric_field(&candidate_value, "adx_14"),
            12.0,
        ),
        closeness(
            numeric_field(&target_value, "volume_ratio_20"),
            numeric_field(&candidate_value, "volume_ratio_20"),
            0.28,
        ),
        closeness(
            numeric_field(&target_value, "rsi_14"),
            numeric_field(&candidate_value, "rsi_14"),
            12.0,
        ),
        closeness(
            numeric_field(&target_value, "mfi_14"),
            numeric_field(&candidate_value, "mfi_14"),
            18.0,
        ),
    ]
    .into_iter()
    .sum::<f64>()
        / 7.0;

    Ok(SimilarityScore {
        score: 0.55 * tag_score + 0.45 * numeric_score,
        matched_tags,
    })
}

fn numeric_field(value: &Value, field_name: &str) -> f64 {
    value
        .get(field_name)
        .and_then(Value::as_f64)
        .unwrap_or_default()
}

fn classify_band(
    value: f64,
    positive_threshold: f64,
    negative_threshold: f64,
    positive_label: &str,
    negative_label: &str,
    neutral_label: &str,
) -> String {
    if value >= positive_threshold {
        positive_label.to_string()
    } else if value <= negative_threshold {
        negative_label.to_string()
    } else {
        neutral_label.to_string()
    }
}

fn closeness(left: f64, right: f64, scale: f64) -> f64 {
    if scale.abs() <= f64::EPSILON {
        return 0.0;
    }
    (1.0 - ((left - right).abs() / scale)).clamp(0.0, 1.0)
}

fn average(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

// 2026-04-08 CST: 这里补一个可选平均值助手，原因是赔率层要明确区分“没有样本”和“平均值为 0”；
// 目的：让 summary 层在 win/loss 两端样本缺失时返回 None，而不是误把 0 当成真实统计结果。
fn average_option(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(average(values))
    }
}

// 2026-04-08 CST: 这里补一个可选比例助手，原因是赔率层需要在 sample_count 为 0 时显式返回缺失而不是 0；
// 目的：避免 unavailable 状态下把“没有研究”误说成“有 0% 的胜率/败率/平率”。
fn ratio_option(numerator: usize, denominator: usize) -> Option<f64> {
    if denominator == 0 {
        None
    } else {
        Some(numerator as f64 / denominator as f64)
    }
}

fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let midpoint = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[midpoint - 1] + sorted[midpoint]) / 2.0
    } else {
        sorted[midpoint]
    }
}

fn format_return_window(label: &str, average_value: f64, median_value: f64) -> String {
    format!(
        "{label}均值 {:.2}%，中位数 {:.2}%",
        average_value * 100.0,
        median_value * 100.0
    )
}

fn classify_historical_confidence(sample_count: usize) -> String {
    if sample_count >= 12 {
        "high".to_string()
    } else if sample_count >= 6 {
        "medium".to_string()
    } else if sample_count >= 1 {
        "low".to_string()
    } else {
        "unknown".to_string()
    }
}

fn round_ratio(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}
