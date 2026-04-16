use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_analysis_resonance::{
    AppendResonanceFactorSeriesRequest, ResonancePointInput, SecurityAnalysisResonanceError,
    append_resonance_factor_series,
};
use crate::ops::stock::sync_stock_price_history::{
    SyncStockPriceHistoryError, SyncStockPriceHistoryRequest, sync_stock_price_history,
};
use crate::runtime::stock_history_store::{
    StockHistoryRow, StockHistoryStore, StockHistoryStoreError,
};

fn default_adjustment() -> String {
    "qfq".to_string()
}

fn default_sync_providers() -> Vec<String> {
    vec!["tencent".to_string(), "sina".to_string()]
}

// 2026-04-02 CST: 这里定义模板级共振因子同步请求，原因是方案C要求先把“模板因子补数”做成正式 Tool 合同；
// 目的：让银行等行业模板都能沿统一请求结构扩展，而不是每个模板单独写脚本参数。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SyncTemplateResonanceFactorsRequest {
    pub market_regime: String,
    pub template_key: String,
    pub start_date: String,
    pub end_date: String,
    #[serde(default = "default_adjustment")]
    pub adjustment: String,
    #[serde(default = "default_sync_providers")]
    pub providers: Vec<String>,
    #[serde(default)]
    pub factor_keys: Option<Vec<String>>,
    #[serde(default)]
    pub skip_price_sync: bool,
}

// 2026-04-02 CST: 这里定义模板级共振因子同步结果，原因是平台底座同步不应只返回成功/失败；
// 目的：让调用方明确知道本次模板同步了哪些因子、用了哪些代理 symbol、最终写入了多少序列点。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SyncTemplateResonanceFactorsResult {
    pub market_regime: String,
    pub template_key: String,
    pub synced_factor_count: usize,
    pub imported_symbol_count: usize,
    pub total_point_count: usize,
    pub factors: Vec<FactorSyncSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FactorSyncSummary {
    pub factor_key: String,
    pub source_symbols: Vec<String>,
    pub transform: String,
    pub appended_count: usize,
}

// 2026-04-02 CST: 这里集中定义模板级共振因子同步错误，原因是模板配置、行情同步、序列派生和落库都可能失败；
// 目的：把底层异常翻译成可定位的中文错误，方便后续 Agent/Skill 定位到底是配置问题还是数据问题。
#[derive(Debug, Error)]
pub enum SyncTemplateResonanceFactorsError {
    #[error(
        "暂不支持 market_regime=`{market_regime}` template_key=`{template_key}` 的模板因子同步"
    )]
    UnsupportedTemplate {
        market_regime: String,
        template_key: String,
    },
    #[error("模板 `{template_key}` 不包含因子 `{factor_key}`")]
    UnknownFactor {
        template_key: String,
        factor_key: String,
    },
    #[error("因子 `{factor_key}` 缺少可用行情序列，symbol=`{symbol}`")]
    MissingPriceSeries { factor_key: String, symbol: String },
    #[error("因子 `{factor_key}` 在区间内没有可对齐的共同交易日")]
    EmptyIntersection { factor_key: String },
    #[error("因子 `{factor_key}` 的 transform=`{transform}` 缺少足够的输入序列")]
    InvalidTransformInputs {
        factor_key: String,
        transform: String,
    },
    #[error("{0}")]
    StockHistory(#[from] StockHistoryStoreError),
    #[error("{0}")]
    PriceSync(#[from] SyncStockPriceHistoryError),
    #[error("{0}")]
    Resonance(#[from] SecurityAnalysisResonanceError),
}

#[derive(Debug, Clone)]
struct FactorRecipe {
    factor_key: &'static str,
    source_symbols: &'static [&'static str],
    transform: FactorTransform,
}

#[derive(Debug, Clone, Copy)]
enum FactorTransform {
    Close,
    InverseClose,
    Ratio,
    Average,
}

impl FactorTransform {
    fn as_str(self) -> &'static str {
        match self {
            Self::Close => "close",
            Self::InverseClose => "inverse_close",
            Self::Ratio => "ratio",
            Self::Average => "average",
        }
    }
}

// 2026-04-02 CST: 这里实现模板级共振因子同步主入口，原因是银行宏观共振底座需要先把“代理行情 -> 因子序列 -> resonance DB”打通；
// 目的：让后续行业模板补数都能复用同一条主链，而不是在 briefing 外层做一次性预处理。
pub fn sync_template_resonance_factors(
    request: &SyncTemplateResonanceFactorsRequest,
) -> Result<SyncTemplateResonanceFactorsResult, SyncTemplateResonanceFactorsError> {
    let recipes = resolve_template_recipes(&request.market_regime, &request.template_key)?;
    let selected_recipes = filter_requested_recipes(
        &request.template_key,
        recipes,
        request.factor_keys.as_deref(),
    )?;

    let unique_symbols = selected_recipes
        .iter()
        .flat_map(|recipe| recipe.source_symbols.iter().copied())
        .collect::<BTreeSet<_>>();

    if !request.skip_price_sync {
        for symbol in &unique_symbols {
            let sync_request = SyncStockPriceHistoryRequest {
                symbol: (*symbol).to_string(),
                start_date: request.start_date.clone(),
                end_date: request.end_date.clone(),
                adjustment: request.adjustment.clone(),
                providers: request.providers.clone(),
            };
            let _ = sync_stock_price_history(&sync_request)?;
        }
    }

    let stock_store = StockHistoryStore::workspace_default()?;
    let mut series_by_symbol = HashMap::new();
    for symbol in &unique_symbols {
        let rows =
            stock_store.load_rows_in_range(symbol, &request.start_date, &request.end_date)?;
        series_by_symbol.insert((*symbol).to_string(), rows);
    }

    let mut factor_summaries = Vec::new();
    let mut total_point_count = 0_usize;
    for recipe in selected_recipes {
        let points = build_factor_points(&recipe, &series_by_symbol)?;
        let append_request = AppendResonanceFactorSeriesRequest {
            factor_key: recipe.factor_key.to_string(),
            source: format!(
                "template_sync:{}:{}:{}",
                request.template_key,
                request.market_regime,
                recipe.transform.as_str()
            ),
            points,
        };
        let append_result = append_resonance_factor_series(&append_request)?;
        total_point_count += append_result.appended_count;
        factor_summaries.push(FactorSyncSummary {
            factor_key: recipe.factor_key.to_string(),
            source_symbols: recipe
                .source_symbols
                .iter()
                .map(|symbol| (*symbol).to_string())
                .collect(),
            transform: recipe.transform.as_str().to_string(),
            appended_count: append_result.appended_count,
        });
    }

    Ok(SyncTemplateResonanceFactorsResult {
        market_regime: request.market_regime.clone(),
        template_key: request.template_key.clone(),
        synced_factor_count: factor_summaries.len(),
        imported_symbol_count: unique_symbols.len(),
        total_point_count,
        factors: factor_summaries,
    })
}

// 2026-04-02 CST: 这里集中维护模板配方，原因是方案C的核心就是先收敛“模板 -> 因子 -> 代理 symbol/变换”映射；
// 目的：后续无论是银行还是别的传统行业，都可以在这里以可审计的方式继续扩模板，而不是散落进分析逻辑里。
fn resolve_template_recipes(
    market_regime: &str,
    template_key: &str,
) -> Result<Vec<FactorRecipe>, SyncTemplateResonanceFactorsError> {
    if market_regime == "a_share" && template_key == "bank" {
        return Ok(vec![
            FactorRecipe {
                factor_key: "bank_sector_proxy",
                source_symbols: &["512800.SH"],
                transform: FactorTransform::Close,
            },
            FactorRecipe {
                factor_key: "cn_bond_10y_yield",
                source_symbols: &["511260.SH"],
                transform: FactorTransform::InverseClose,
            },
            FactorRecipe {
                factor_key: "credit_risk_spread",
                source_symbols: &["511260.SH", "511190.SH"],
                transform: FactorTransform::Ratio,
            },
            FactorRecipe {
                factor_key: "real_estate_proxy",
                source_symbols: &["512200.SH"],
                transform: FactorTransform::Close,
            },
            FactorRecipe {
                factor_key: "resident_consumption_proxy",
                source_symbols: &["159928.SZ"],
                transform: FactorTransform::Close,
            },
            FactorRecipe {
                factor_key: "loan_growth_proxy",
                source_symbols: &["512800.SH", "512200.SH", "159928.SZ"],
                transform: FactorTransform::Average,
            },
            FactorRecipe {
                factor_key: "pmi_proxy",
                source_symbols: &["510300.SH", "511260.SH"],
                transform: FactorTransform::Ratio,
            },
        ]);
    }

    Err(SyncTemplateResonanceFactorsError::UnsupportedTemplate {
        market_regime: market_regime.to_string(),
        template_key: template_key.to_string(),
    })
}

fn filter_requested_recipes(
    template_key: &str,
    recipes: Vec<FactorRecipe>,
    factor_keys: Option<&[String]>,
) -> Result<Vec<FactorRecipe>, SyncTemplateResonanceFactorsError> {
    let Some(factor_keys) = factor_keys else {
        return Ok(recipes);
    };

    let mut recipe_map = recipes
        .into_iter()
        .map(|recipe| (recipe.factor_key.to_string(), recipe))
        .collect::<HashMap<_, _>>();
    let mut selected = Vec::new();
    for factor_key in factor_keys {
        let Some(recipe) = recipe_map.remove(factor_key) else {
            return Err(SyncTemplateResonanceFactorsError::UnknownFactor {
                template_key: template_key.to_string(),
                factor_key: factor_key.clone(),
            });
        };
        selected.push(recipe);
    }
    Ok(selected)
}

fn build_factor_points(
    recipe: &FactorRecipe,
    series_by_symbol: &HashMap<String, Vec<StockHistoryRow>>,
) -> Result<Vec<ResonancePointInput>, SyncTemplateResonanceFactorsError> {
    let aligned_series = recipe
        .source_symbols
        .iter()
        .map(|symbol| {
            let rows = series_by_symbol.get(*symbol).ok_or_else(|| {
                SyncTemplateResonanceFactorsError::MissingPriceSeries {
                    factor_key: recipe.factor_key.to_string(),
                    symbol: (*symbol).to_string(),
                }
            })?;
            if rows.is_empty() {
                return Err(SyncTemplateResonanceFactorsError::MissingPriceSeries {
                    factor_key: recipe.factor_key.to_string(),
                    symbol: (*symbol).to_string(),
                });
            }
            Ok(rows_to_map(rows))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let date_intersection = intersect_dates(&aligned_series);
    if date_intersection.is_empty() {
        return Err(SyncTemplateResonanceFactorsError::EmptyIntersection {
            factor_key: recipe.factor_key.to_string(),
        });
    }

    let mut points = Vec::new();
    for trade_date in date_intersection {
        let value = match recipe.transform {
            FactorTransform::Close => first_series_value(recipe, &aligned_series, &trade_date)?,
            FactorTransform::InverseClose => {
                let close = first_series_value(recipe, &aligned_series, &trade_date)?;
                if close.abs() <= f64::EPSILON {
                    0.0
                } else {
                    1.0 / close
                }
            }
            FactorTransform::Ratio => {
                if aligned_series.len() < 2 {
                    return Err(SyncTemplateResonanceFactorsError::InvalidTransformInputs {
                        factor_key: recipe.factor_key.to_string(),
                        transform: recipe.transform.as_str().to_string(),
                    });
                }
                let numerator = *aligned_series[0]
                    .get(&trade_date)
                    .expect("intersection should ensure numerator exists");
                let denominator = *aligned_series[1]
                    .get(&trade_date)
                    .expect("intersection should ensure denominator exists");
                if denominator.abs() <= f64::EPSILON {
                    0.0
                } else {
                    numerator / denominator
                }
            }
            FactorTransform::Average => {
                if aligned_series.is_empty() {
                    return Err(SyncTemplateResonanceFactorsError::InvalidTransformInputs {
                        factor_key: recipe.factor_key.to_string(),
                        transform: recipe.transform.as_str().to_string(),
                    });
                }
                let sum = aligned_series
                    .iter()
                    .map(|series| {
                        *series
                            .get(&trade_date)
                            .expect("intersection should ensure source exists")
                    })
                    .sum::<f64>();
                sum / aligned_series.len() as f64
            }
        };
        points.push(ResonancePointInput { trade_date, value });
    }

    Ok(points)
}

fn first_series_value(
    recipe: &FactorRecipe,
    aligned_series: &[BTreeMap<String, f64>],
    trade_date: &str,
) -> Result<f64, SyncTemplateResonanceFactorsError> {
    let source = aligned_series.first().ok_or_else(|| {
        SyncTemplateResonanceFactorsError::InvalidTransformInputs {
            factor_key: recipe.factor_key.to_string(),
            transform: recipe.transform.as_str().to_string(),
        }
    })?;
    Ok(*source
        .get(trade_date)
        .expect("intersection should ensure trade_date exists"))
}

fn rows_to_map(rows: &[StockHistoryRow]) -> BTreeMap<String, f64> {
    rows.iter()
        .map(|row| (row.trade_date.clone(), row.close))
        .collect::<BTreeMap<_, _>>()
}

fn intersect_dates(series_maps: &[BTreeMap<String, f64>]) -> Vec<String> {
    let Some(first) = series_maps.first() else {
        return Vec::new();
    };

    first
        .keys()
        .filter(|trade_date| {
            series_maps
                .iter()
                .all(|series| series.contains_key(*trade_date))
        })
        .cloned()
        .collect()
}
