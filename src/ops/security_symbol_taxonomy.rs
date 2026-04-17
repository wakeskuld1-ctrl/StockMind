use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

// 2026-04-16 CST: Added because P0 data thickening needs symbol-level industry routing instead of
// reusing one blended sector proxy for the whole mixed A-share pool.
// Reason: without a governed taxonomy, snapshot/training/runtime keep collapsing sector semantics
// back into request placeholders and the model sees near-constant industry fields.
// Purpose: provide one cached symbol taxonomy contract that all three links can reuse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecuritySymbolRouting {
    pub market_symbol: Option<String>,
    pub market_profile: Option<String>,
    pub sector_symbol: Option<String>,
    pub sector_profile: Option<String>,
    pub industry_bucket: Option<String>,
    pub subindustry_bucket: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveSecurityRouting {
    pub market_symbol: Option<String>,
    pub market_profile: Option<String>,
    pub sector_symbol: Option<String>,
    pub sector_profile: Option<String>,
    pub industry_bucket: Option<String>,
    pub subindustry_bucket: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SymbolTaxonomyDocument {
    profiles: Vec<SymbolTaxonomyRow>,
}

#[derive(Debug, Deserialize)]
struct SymbolTaxonomyRow {
    symbol: String,
    market_symbol: Option<String>,
    market_profile: Option<String>,
    sector_symbol: Option<String>,
    sector_profile: Option<String>,
    industry_bucket: Option<String>,
    subindustry_bucket: Option<String>,
}

static SYMBOL_TAXONOMY: OnceLock<BTreeMap<String, SecuritySymbolRouting>> = OnceLock::new();

pub fn resolve_security_symbol_routing(symbol: &str) -> Option<SecuritySymbolRouting> {
    let normalized_symbol = normalize_symbol(symbol);
    load_symbol_taxonomy().get(&normalized_symbol).cloned()
}

pub fn resolve_effective_security_routing(
    symbol: &str,
    explicit_market_symbol: Option<&str>,
    explicit_sector_symbol: Option<&str>,
    explicit_market_profile: Option<&str>,
    explicit_sector_profile: Option<&str>,
) -> EffectiveSecurityRouting {
    let symbol_routing = resolve_security_symbol_routing(symbol);
    let taxonomy_market_symbol = symbol_routing
        .as_ref()
        .and_then(|item| item.market_symbol.clone());
    let taxonomy_market_profile = symbol_routing
        .as_ref()
        .and_then(|item| item.market_profile.clone());
    let taxonomy_sector_symbol = symbol_routing
        .as_ref()
        .and_then(|item| item.sector_symbol.clone());
    let taxonomy_sector_profile = symbol_routing
        .as_ref()
        .and_then(|item| item.sector_profile.clone());
    let industry_bucket = symbol_routing
        .as_ref()
        .and_then(|item| item.industry_bucket.clone());
    let subindustry_bucket = symbol_routing
        .as_ref()
        .and_then(|item| item.subindustry_bucket.clone());

    let explicit_market_symbol = normalized_non_empty(explicit_market_symbol);
    let explicit_market_profile = normalized_non_empty(explicit_market_profile);
    let explicit_sector_symbol = normalized_non_empty(explicit_sector_symbol);
    let explicit_sector_profile = normalized_non_empty(explicit_sector_profile);

    let market_symbol = explicit_market_symbol.or(taxonomy_market_symbol);
    let market_profile = explicit_market_profile
        .filter(|value| !is_blended_profile(value))
        .or(taxonomy_market_profile);
    let sector_profile = explicit_sector_profile
        .filter(|value| !is_blended_profile(value))
        .or(taxonomy_sector_profile);
    let sector_symbol = if sector_profile.is_some() {
        if explicit_sector_symbol
            .as_deref()
            .zip(market_symbol.as_deref())
            .is_some_and(|(sector_symbol, market_symbol)| sector_symbol == market_symbol)
        {
            taxonomy_sector_symbol.or(explicit_sector_symbol)
        } else {
            explicit_sector_symbol.or(taxonomy_sector_symbol)
        }
    } else {
        explicit_sector_symbol.or(taxonomy_sector_symbol)
    };

    EffectiveSecurityRouting {
        market_symbol,
        market_profile,
        sector_symbol,
        sector_profile,
        industry_bucket,
        subindustry_bucket,
    }
}

fn load_symbol_taxonomy() -> &'static BTreeMap<String, SecuritySymbolRouting> {
    SYMBOL_TAXONOMY.get_or_init(|| {
        let payload =
            include_str!("../../config/real_trading_stock_pools/a_share_symbol_taxonomy_v1.json");
        let document = serde_json::from_str::<SymbolTaxonomyDocument>(payload)
            .expect("security symbol taxonomy should be valid json");
        document
            .profiles
            .into_iter()
            .map(|row| {
                (
                    normalize_symbol(&row.symbol),
                    SecuritySymbolRouting {
                        market_symbol: row.market_symbol.and_then(|value| normalized_owned(value)),
                        market_profile: row
                            .market_profile
                            .and_then(|value| normalized_owned(value)),
                        sector_symbol: row.sector_symbol.and_then(|value| normalized_owned(value)),
                        sector_profile: row
                            .sector_profile
                            .and_then(|value| normalized_owned(value)),
                        industry_bucket: row
                            .industry_bucket
                            .and_then(|value| normalized_owned(value)),
                        subindustry_bucket: row
                            .subindustry_bucket
                            .and_then(|value| normalized_owned(value)),
                    },
                )
            })
            .collect()
    })
}

fn normalize_symbol(symbol: &str) -> String {
    symbol.trim().to_uppercase()
}

fn normalized_non_empty(value: Option<&str>) -> Option<String> {
    value.and_then(|item| normalized_owned(item.to_string()))
}

fn normalized_owned(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn is_blended_profile(profile: &str) -> bool {
    let normalized = profile.trim().to_ascii_lowercase();
    normalized.contains("blended") || normalized == "unknown"
}

#[cfg(test)]
mod tests {
    use super::{resolve_effective_security_routing, resolve_security_symbol_routing};

    #[test]
    fn symbol_taxonomy_returns_joint_stock_bank_for_601916() {
        let routing = resolve_security_symbol_routing("601916.SH")
            .expect("601916 should exist in the governed symbol taxonomy");
        assert_eq!(routing.industry_bucket.as_deref(), Some("bank"));
        assert_eq!(
            routing.subindustry_bucket.as_deref(),
            Some("joint_stock_bank")
        );
        assert_eq!(routing.sector_symbol.as_deref(), Some("512800.SH"));
    }

    #[test]
    fn effective_routing_replaces_blended_sector_proxy_with_symbol_taxonomy() {
        let routing = resolve_effective_security_routing(
            "601318.SH",
            Some("510300.SH"),
            Some("510300.SH"),
            Some("a_share_core_v1"),
            Some("a_share_core_blended"),
        );
        assert_eq!(routing.sector_symbol.as_deref(), Some("512070.SH"));
        assert_eq!(routing.sector_profile.as_deref(), Some("a_share_insurance"));
        assert_eq!(routing.industry_bucket.as_deref(), Some("insurance"));
        assert_eq!(
            routing.subindustry_bucket.as_deref(),
            Some("life_property_insurance")
        );
    }
}
