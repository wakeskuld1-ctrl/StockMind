use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::stock::security_analysis_fullstack::{
    GovernedDisclosureHistoryRow, GovernedFundamentalHistoryRow, SecurityAnalysisFullstackError,
    SecurityAnalysisFullstackRequest, fetch_live_disclosure_history_rows_for_governed_history,
    fetch_live_fundamental_history_rows_for_governed_history, security_analysis_fullstack,
};
use crate::ops::stock::security_disclosure_history_backfill::SecurityDisclosureHistoryBackfillRecordInput;
use crate::ops::stock::security_external_proxy_backfill::load_historical_external_proxy_inputs;
use crate::ops::stock::security_external_proxy_history_import::{
    SecurityExternalProxyHistoryImportRequest, security_external_proxy_history_import,
};
use crate::ops::stock::security_fundamental_history_backfill::SecurityFundamentalHistoryBackfillRecordInput;
use crate::ops::stock::sync_stock_price_history::{
    SyncStockPriceHistoryError, SyncStockPriceHistoryFetchedRows, SyncStockPriceHistoryRequest,
    fetch_stock_price_history_rows,
};
use crate::runtime::security_disclosure_history_store::{
    SecurityDisclosureHistoryRecordRow, SecurityDisclosureHistoryStore,
};
use crate::runtime::security_fundamental_history_store::{
    SecurityFundamentalHistoryRecordRow, SecurityFundamentalHistoryStore,
};
use crate::runtime::stock_history_store::{
    StockHistoryImportSummary, StockHistoryStore, StockHistoryStoreError,
};
use crate::runtime_paths::workspace_runtime_dir;

const DEFAULT_DISCLOSURE_LIMIT: usize = 8;
const DEFAULT_LOOKBACK_DAYS: usize = 260;

// 2026-04-12 CST: Add a governed real-data validation request, because P11 starts
// refreshing validation slices with live-compatible prices and public disclosure context.
// Purpose: keep real-data verification narrow and reproducible instead of scripting it ad hoc.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityRealDataValidationBackfillRequest {
    pub slice_id: String,
    pub symbol: String,
    #[serde(default)]
    pub market_symbol: Option<String>,
    #[serde(default)]
    pub sector_symbol: Option<String>,
    pub start_date: String,
    pub end_date: String,
    #[serde(default = "default_adjustment")]
    pub adjustment: String,
    #[serde(default = "default_sync_providers")]
    pub providers: Vec<String>,
    #[serde(default)]
    pub validation_runtime_root: Option<String>,
    #[serde(default)]
    pub market_profile: Option<String>,
    #[serde(default)]
    pub sector_profile: Option<String>,
    #[serde(default = "default_lookback_days")]
    pub lookback_days: usize,
    #[serde(default = "default_disclosure_limit")]
    pub disclosure_limit: usize,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityRealDataValidationPriceSyncSummary {
    pub symbol: String,
    pub provider_used: String,
    pub imported_row_count: usize,
    pub date_range: SecurityRealDataValidationDateRange,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityRealDataValidationDateRange {
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SecurityRealDataValidationBackfillResult {
    pub contract_version: String,
    pub document_type: String,
    pub slice_id: String,
    pub primary_symbol: String,
    pub validation_runtime_root: String,
    pub runtime_db_path: String,
    pub fundamental_history_db_path: String,
    pub disclosure_history_db_path: String,
    #[serde(default)]
    pub external_proxy_db_path: Option<String>,
    #[serde(default)]
    pub external_proxy_import_result_paths: Vec<String>,
    pub price_sync_summaries: Vec<SecurityRealDataValidationPriceSyncSummary>,
    pub fullstack_context_path: String,
    pub manifest_path: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
struct SecurityRealDataValidationManifest {
    pub contract_version: String,
    pub document_type: String,
    pub slice_id: String,
    pub primary_symbol: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sector_symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sector_profile: Option<String>,
    pub created_at: String,
    pub validation_runtime_root: String,
    pub runtime_db_path: String,
    pub fundamental_history_db_path: String,
    pub disclosure_history_db_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_proxy_db_path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_proxy_import_result_paths: Vec<String>,
    pub fullstack_context_path: String,
    pub price_sync_summaries: Vec<SecurityRealDataValidationPriceSyncSummary>,
}

#[derive(Debug, Error)]
pub enum SecurityRealDataValidationBackfillError {
    #[error("security real-data validation backfill build failed: {0}")]
    Build(String),
    #[error("security real-data validation backfill price sync failed: {0}")]
    PriceSync(#[from] SyncStockPriceHistoryError),
    #[error("security real-data validation backfill stock storage failed: {0}")]
    StockStorage(#[from] StockHistoryStoreError),
    #[error("security real-data validation backfill fundamental storage failed: {0}")]
    FundamentalStorage(String),
    #[error("security real-data validation backfill disclosure storage failed: {0}")]
    DisclosureStorage(String),
    #[error("security real-data validation backfill external proxy import failed: {0}")]
    ExternalProxyImport(String),
    #[error("security real-data validation backfill fullstack failed: {0}")]
    Fullstack(#[from] SecurityAnalysisFullstackError),
    #[error("security real-data validation backfill persist failed: {0}")]
    Persist(String),
}

#[derive(Debug, Clone, PartialEq)]
struct EffectiveValidationProxyContext {
    market_symbol: Option<String>,
    sector_symbol: Option<String>,
    market_profile: Option<String>,
    sector_profile: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct ValidationExternalProxyImportSummary {
    external_proxy_db_path: String,
    import_result_paths: Vec<String>,
}

// 2026-04-12 CST: Refresh one governed validation slice with live-compatible
// price history plus public disclosure context, because the securities mainline
// now needs repeatable real-data verification inputs after P8-P10 lifecycle closure.
// Purpose: keep real-data verification on the formal stock tool chain, not in shell recipes.
pub fn security_real_data_validation_backfill(
    request: &SecurityRealDataValidationBackfillRequest,
) -> Result<SecurityRealDataValidationBackfillResult, SecurityRealDataValidationBackfillError> {
    validate_request(request)?;
    let effective_proxy_context = resolve_effective_proxy_context(request)?;

    let validation_root = resolve_validation_runtime_root(request)?;
    fs::create_dir_all(&validation_root).map_err(|error| {
        SecurityRealDataValidationBackfillError::Persist(format!(
            "failed to create validation runtime root `{}`: {error}",
            validation_root.display()
        ))
    })?;

    let runtime_db_path = validation_root.join("stock_history.db");
    let fundamental_history_db_path = validation_root.join("security_fundamental_history.db");
    let disclosure_history_db_path = validation_root.join("security_disclosure_history.db");
    let store = StockHistoryStore::new(runtime_db_path.clone());
    let mut price_sync_summaries = Vec::new();
    let external_proxy_import_summary =
        import_required_etf_proxy_history(request, &effective_proxy_context)?;

    for symbol in collect_sync_symbols(request, &effective_proxy_context) {
        let sync_request = SyncStockPriceHistoryRequest {
            symbol: symbol.clone(),
            start_date: request.start_date.clone(),
            end_date: request.end_date.clone(),
            adjustment: request.adjustment.clone(),
            providers: request.providers.clone(),
        };
        let fetched_rows = fetch_stock_price_history_rows(&sync_request)?;
        let import_summary = store.import_rows(
            &symbol,
            &format!("{}_http_{}", fetched_rows.provider_used, request.adjustment),
            &fetched_rows.rows,
        )?;
        price_sync_summaries.push(build_price_sync_summary(
            &symbol,
            &fetched_rows,
            &import_summary,
        ));
    }

    let fullstack_context_path = validation_root.join("fullstack_context.json");
    let manifest_path = validation_root.join("real_data_validation_manifest.json");
    let fullstack_request = SecurityAnalysisFullstackRequest {
        symbol: request.symbol.clone(),
        market_symbol: effective_proxy_context.market_symbol.clone(),
        sector_symbol: effective_proxy_context.sector_symbol.clone(),
        market_profile: effective_proxy_context.market_profile.clone(),
        sector_profile: effective_proxy_context.sector_profile.clone(),
        as_of_date: Some(request.end_date.clone()),
        // 2026-04-16 CST: Added because SecurityAnalysisFullstackRequest now explicitly models
        // cross-border ETF legs.
        // Reason: this validation tool does not supply those legs itself, so it must pass
        // governed empty values instead of compiling against an outdated struct shape.
        // Purpose: keep this compatibility path compiling without changing current slice behavior.
        underlying_symbol: None,
        fx_symbol: None,
        lookback_days: request.lookback_days,
        disclosure_limit: request.disclosure_limit,
    };
    if let Ok(fundamental_context) =
        fetch_live_fundamental_history_rows_for_governed_history(&request.symbol)
    {
        persist_slice_fundamental_history(
            &fundamental_history_db_path,
            request,
            &fundamental_context,
        )?;
    }
    if let Ok(disclosure_context) = fetch_live_disclosure_history_rows_for_governed_history(
        &request.symbol,
        request.disclosure_limit.max(1),
        3,
    ) {
        persist_slice_disclosure_history(
            &disclosure_history_db_path,
            request,
            &disclosure_context,
        )?;
    }
    let fullstack_result = with_validation_history_overrides(
        &runtime_db_path,
        &fundamental_history_db_path,
        &disclosure_history_db_path,
        || security_analysis_fullstack(&fullstack_request),
    )?;
    persist_json(&fullstack_context_path, &fullstack_result)?;

    let result = SecurityRealDataValidationBackfillResult {
        contract_version: "security_real_data_validation_backfill.v1".to_string(),
        document_type: "security_real_data_validation_backfill_result".to_string(),
        slice_id: request.slice_id.trim().to_string(),
        primary_symbol: request.symbol.trim().to_string(),
        validation_runtime_root: validation_root.to_string_lossy().to_string(),
        runtime_db_path: runtime_db_path.to_string_lossy().to_string(),
        fundamental_history_db_path: fundamental_history_db_path.to_string_lossy().to_string(),
        disclosure_history_db_path: disclosure_history_db_path.to_string_lossy().to_string(),
        external_proxy_db_path: external_proxy_import_summary
            .as_ref()
            .map(|summary| summary.external_proxy_db_path.clone()),
        external_proxy_import_result_paths: external_proxy_import_summary
            .as_ref()
            .map(|summary| summary.import_result_paths.clone())
            .unwrap_or_default(),
        price_sync_summaries: price_sync_summaries.clone(),
        fullstack_context_path: fullstack_context_path.to_string_lossy().to_string(),
        manifest_path: manifest_path.to_string_lossy().to_string(),
    };
    let manifest = SecurityRealDataValidationManifest {
        contract_version: result.contract_version.clone(),
        document_type: "security_real_data_validation_manifest".to_string(),
        slice_id: result.slice_id.clone(),
        primary_symbol: result.primary_symbol.clone(),
        market_symbol: effective_proxy_context.market_symbol.clone(),
        sector_symbol: effective_proxy_context.sector_symbol.clone(),
        market_profile: effective_proxy_context.market_profile.clone(),
        sector_profile: effective_proxy_context.sector_profile.clone(),
        created_at: request.created_at.trim().to_string(),
        validation_runtime_root: result.validation_runtime_root.clone(),
        runtime_db_path: result.runtime_db_path.clone(),
        fundamental_history_db_path: result.fundamental_history_db_path.clone(),
        disclosure_history_db_path: result.disclosure_history_db_path.clone(),
        external_proxy_db_path: result.external_proxy_db_path.clone(),
        external_proxy_import_result_paths: result.external_proxy_import_result_paths.clone(),
        fullstack_context_path: result.fullstack_context_path.clone(),
        price_sync_summaries,
    };
    persist_json(&manifest_path, &manifest)?;

    Ok(result)
}

// 2026-04-12 CST: Resolve one ETF-native proxy context before syncing prices,
// because governed validation slices now need to preserve ETF semantics while
// still enriching the slice with the peer environment symbols required by fullstack.
// Purpose: keep manifest, price sync, and fullstack all aligned on one auditable
// market/sector context instead of mixing raw request fields with ad-hoc defaults.
fn resolve_effective_proxy_context(
    request: &SecurityRealDataValidationBackfillRequest,
) -> Result<EffectiveValidationProxyContext, SecurityRealDataValidationBackfillError> {
    let market_profile = normalize_optional_field(request.market_profile.as_deref());
    let sector_profile = normalize_optional_field(request.sector_profile.as_deref());
    let market_symbol = normalize_optional_field(request.market_symbol.as_deref())
        .or_else(|| resolve_market_symbol_from_profile(market_profile.as_deref()));
    let sector_symbol = normalize_optional_field(request.sector_symbol.as_deref()).or_else(|| {
        resolve_sector_symbol_from_profile(request.symbol.trim(), sector_profile.as_deref())
    });

    Ok(EffectiveValidationProxyContext {
        market_symbol,
        sector_symbol,
        market_profile,
        sector_profile,
    })
}

// 2026-04-12 CST: Keep request validation local, because this tool introduces a
// new governed validation-slice contract and should fail early on missing identifiers.
// Purpose: avoid partial writes when operators supply incomplete slice coordinates.
fn validate_request(
    request: &SecurityRealDataValidationBackfillRequest,
) -> Result<(), SecurityRealDataValidationBackfillError> {
    if request.slice_id.trim().is_empty() {
        return Err(SecurityRealDataValidationBackfillError::Build(
            "slice_id cannot be empty".to_string(),
        ));
    }
    if request.symbol.trim().is_empty() {
        return Err(SecurityRealDataValidationBackfillError::Build(
            "symbol cannot be empty".to_string(),
        ));
    }
    if request.start_date.trim().is_empty() {
        return Err(SecurityRealDataValidationBackfillError::Build(
            "start_date cannot be empty".to_string(),
        ));
    }
    if request.end_date.trim().is_empty() {
        return Err(SecurityRealDataValidationBackfillError::Build(
            "end_date cannot be empty".to_string(),
        ));
    }
    if request.created_at.trim().is_empty() {
        return Err(SecurityRealDataValidationBackfillError::Build(
            "created_at cannot be empty".to_string(),
        ));
    }
    if request.providers.is_empty() {
        return Err(SecurityRealDataValidationBackfillError::Build(
            "providers cannot be empty".to_string(),
        ));
    }

    Ok(())
}

// 2026-04-12 CST: Resolve one stable validation root, because operators need a
// predictable place to inspect the refreshed slice after the tool finishes.
// Purpose: keep explicit roots honored while also giving a deterministic default location.
fn resolve_validation_runtime_root(
    request: &SecurityRealDataValidationBackfillRequest,
) -> Result<PathBuf, SecurityRealDataValidationBackfillError> {
    if let Some(root) = request
        .validation_runtime_root
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return Ok(PathBuf::from(root));
    }

    let runtime_root =
        workspace_runtime_dir().map_err(SecurityRealDataValidationBackfillError::Build)?;
    Ok(runtime_root
        .join("validation_real_data_slices")
        .join(sanitize_identifier(request.slice_id.trim())))
}

// 2026-04-12 CST: Keep the symbol list deterministic, because the manifest and
// operator inspection should not change order across reruns of the same slice.
// Purpose: preserve stable primary/market/sector ordering for later verification.
fn collect_sync_symbols(
    request: &SecurityRealDataValidationBackfillRequest,
    effective_proxy_context: &EffectiveValidationProxyContext,
) -> Vec<String> {
    let mut symbols = vec![request.symbol.trim().to_string()];
    if let Some(symbol) = effective_proxy_context.market_symbol.as_ref() {
        symbols.push(symbol.to_string());
    }
    if let Some(symbol) = effective_proxy_context.sector_symbol.as_ref() {
        symbols.push(symbol.to_string());
    }
    symbols.sort();
    symbols.dedup();
    symbols
}

// 2026-04-12 CST: Normalize optional request strings in one helper, because the
// validation slice now resolves explicit symbols and profile-derived symbols together.
// Purpose: avoid repeating trim/empty handling across market and sector resolution.
fn normalize_optional_field(value: Option<&str>) -> Option<String> {
    value
        .map(|candidate| candidate.trim().to_string())
        .filter(|candidate| !candidate.is_empty())
}

// 2026-04-12 CST: Mirror the governed market-profile mapping locally, because the
// validation slicer needs to decide which peer prices to sync before fullstack runs.
// Purpose: keep one deterministic default for slice enrichment when callers supply
// only market_profile instead of an explicit market symbol.
fn resolve_market_symbol_from_profile(market_profile: Option<&str>) -> Option<String> {
    match market_profile {
        Some("a_share_core") => Some("510300.SH".to_string()),
        _ => None,
    }
}

// 2026-04-12 CST: Add ETF-native sector peer resolution here, because governed
// validation slices must stay ETF-native instead of forcing operators back to
// industry-only profiles such as a_share_bank.
// Purpose: enrich ETF slices with the peer environment symbols needed by technical
// context, manifest replay, and later chair-level validation.
fn resolve_sector_symbol_from_profile(
    primary_symbol: &str,
    sector_profile: Option<&str>,
) -> Option<String> {
    match sector_profile {
        Some("a_share_bank") | Some("equity_etf") | Some("equity_etf_peer") => {
            Some("512800.SH".to_string())
        }
        Some("treasury_etf") | Some("bond_etf_peer") => Some("511060.SH".to_string()),
        Some("gold_etf") | Some("gold_etf_peer") => Some("518800.SH".to_string()),
        Some("cross_border_etf") | Some("cross_border_etf_peer") => {
            Some(primary_symbol.to_string())
        }
        _ => None,
    }
}

// 2026-04-12 CST: Import ETF pool proxy history before fullstack persists the slice,
// because latest validation reruns need governed proxy rows to exist before chair-level
// scoring resolves dated ETF information.
// Purpose: make pool-style ETF validations fail fast when proxy history is absent instead of
// silently producing slices that later degrade to placeholder_unbound.
fn import_required_etf_proxy_history(
    request: &SecurityRealDataValidationBackfillRequest,
    effective_proxy_context: &EffectiveValidationProxyContext,
) -> Result<Option<ValidationExternalProxyImportSummary>, SecurityRealDataValidationBackfillError> {
    let Some(proxy_file_names) =
        expected_proxy_history_file_names(effective_proxy_context.sector_profile.as_deref())
    else {
        return Ok(None);
    };

    let runtime_root =
        workspace_runtime_dir().map_err(SecurityRealDataValidationBackfillError::Build)?;
    let external_proxy_db_path = resolve_external_proxy_db_path();
    let candidate_files =
        discover_pool_proxy_history_files(&runtime_root, &proxy_file_names, request.symbol.trim())?;
    if candidate_files.is_empty() {
        return Err(
            SecurityRealDataValidationBackfillError::ExternalProxyImport(format!(
                "missing required etf proxy history for `{}` under `{}`",
                request.symbol.trim(),
                runtime_root.display()
            )),
        );
    }

    let mut import_result_paths = Vec::new();
    for file_path in candidate_files {
        let import_result =
            security_external_proxy_history_import(&SecurityExternalProxyHistoryImportRequest {
                batch_id: format!(
                    "validation-slice-proxy:{}:{}",
                    request.slice_id.trim(),
                    sanitize_identifier(
                        file_path
                            .file_stem()
                            .and_then(|value| value.to_str())
                            .unwrap_or("proxy_history"),
                    )
                ),
                created_at: request.created_at.trim().to_string(),
                file_path: file_path.to_string_lossy().to_string(),
            })
            .map_err(|error| {
                SecurityRealDataValidationBackfillError::ExternalProxyImport(error.to_string())
            })?;
        import_result_paths.push(import_result.backfill_result_path);
    }

    if load_historical_external_proxy_inputs(request.symbol.trim(), request.end_date.trim())
        .map_err(|error| {
            SecurityRealDataValidationBackfillError::ExternalProxyImport(error.to_string())
        })?
        .is_none()
    {
        return Err(
            SecurityRealDataValidationBackfillError::ExternalProxyImport(format!(
                "missing required etf proxy history for `{}` on or before `{}` after import",
                request.symbol.trim(),
                request.end_date.trim()
            )),
        );
    }

    Ok(Some(ValidationExternalProxyImportSummary {
        external_proxy_db_path: external_proxy_db_path.to_string_lossy().to_string(),
        import_result_paths,
    }))
}

// 2026-04-12 CST: Centralize ETF pool-history file-name expectations here, because
// validation slices should discover the same governed CSV artifacts that pooled training
// jobs already emit for each ETF subscope.
// Purpose: keep auto-discovery deterministic instead of relying on loosely matched filenames.
fn expected_proxy_history_file_names(sector_profile: Option<&str>) -> Option<Vec<&'static str>> {
    match sector_profile {
        Some("treasury_etf") | Some("bond_etf_peer") => Some(vec![
            "treasury_pool_proxy_history.csv",
            "bond_pool_proxy_history.csv",
        ]),
        Some("gold_etf") | Some("gold_etf_peer") => Some(vec!["gold_pool_proxy_history.csv"]),
        Some("cross_border_etf") | Some("cross_border_etf_peer") => {
            Some(vec!["cross_border_pool_proxy_history.csv"])
        }
        Some("equity_etf") | Some("equity_etf_peer") => Some(vec!["equity_pool_proxy_history.csv"]),
        _ => None,
    }
}

// 2026-04-12 CST: Discover pool proxy CSV files under the governed runtime root, because
// validation reruns should reuse previously prepared pool artifacts instead of asking operators
// to hand-wire file paths for every ETF symbol.
// Purpose: keep the fix workflow reproducible for future ETF validation batches.
fn discover_pool_proxy_history_files(
    runtime_root: &Path,
    expected_file_names: &[&str],
    symbol: &str,
) -> Result<Vec<PathBuf>, SecurityRealDataValidationBackfillError> {
    let expected_names = expected_file_names
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let mut pending = vec![runtime_root.to_path_buf()];
    let mut matches = BTreeSet::new();

    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory).map_err(|error| {
            SecurityRealDataValidationBackfillError::ExternalProxyImport(format!(
                "failed to scan proxy history directory `{}`: {error}",
                directory.display()
            ))
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| {
                SecurityRealDataValidationBackfillError::ExternalProxyImport(format!(
                    "failed to read proxy history directory entry under `{}`: {error}",
                    directory.display()
                ))
            })?;
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if !expected_names.contains(&file_name.to_ascii_lowercase()) {
                continue;
            }
            let payload = fs::read_to_string(&path).map_err(|error| {
                SecurityRealDataValidationBackfillError::ExternalProxyImport(format!(
                    "failed to read proxy history candidate `{}`: {error}",
                    path.display()
                ))
            })?;
            if payload.contains(symbol) {
                matches.insert(path);
            }
        }
    }

    Ok(matches.into_iter().collect())
}

// 2026-04-12 CST: Mirror the workspace-default proxy-store resolution here, because
// validation manifests should report the exact governed external-proxy SQLite path that
// received the imported ETF pool history.
// Purpose: keep downstream latest-chair reruns auditable from one result document.
fn resolve_external_proxy_db_path() -> PathBuf {
    if let Ok(path) = std::env::var("EXCEL_SKILL_EXTERNAL_PROXY_DB") {
        return PathBuf::from(path);
    }

    workspace_runtime_dir()
        .map(|runtime_root| runtime_root.join("security_external_proxy.db"))
        .unwrap_or_else(|_| PathBuf::from("security_external_proxy.db"))
}

// 2026-04-12 CST: Freeze one per-symbol sync summary, because the validation
// slice manifest should show what landed for each imported symbol without querying SQLite.
// Purpose: make replay/debug output operator-readable from one JSON manifest.
fn build_price_sync_summary(
    symbol: &str,
    fetched_rows: &SyncStockPriceHistoryFetchedRows,
    import_summary: &StockHistoryImportSummary,
) -> SecurityRealDataValidationPriceSyncSummary {
    SecurityRealDataValidationPriceSyncSummary {
        symbol: symbol.to_string(),
        provider_used: fetched_rows.provider_used.clone(),
        imported_row_count: import_summary.imported_row_count,
        date_range: SecurityRealDataValidationDateRange {
            start_date: import_summary.start_date.clone(),
            end_date: import_summary.end_date.clone(),
        },
    }
}

// 2026-04-12 CST: Extend the scoped runtime override to all governed history
// stores, because validation replay now persists price, fundamental, and
// disclosure history side by side inside one slice-local runtime root.
// Purpose: force fullstack to prefer the slice-local governed history bundle without leaking env changes.
fn with_validation_history_overrides<T>(
    stock_db_path: &Path,
    fundamental_history_db_path: &Path,
    disclosure_history_db_path: &Path,
    callback: impl FnOnce() -> Result<T, SecurityAnalysisFullstackError>,
) -> Result<T, SecurityRealDataValidationBackfillError> {
    #[cfg(test)]
    let _env_lock = crate::test_support::lock_test_env();

    let previous_stock_db = std::env::var_os("EXCEL_SKILL_STOCK_DB");
    let previous_fundamental_history_db = std::env::var_os("EXCEL_SKILL_FUNDAMENTAL_HISTORY_DB");
    let previous_disclosure_history_db = std::env::var_os("EXCEL_SKILL_DISCLOSURE_HISTORY_DB");

    // 2026-04-12 CST: Scope all three governed-history overrides together,
    // because fullstack now resolves stock, fundamental, and disclosure stores
    // independently through runtime env hooks.
    // Purpose: make one validation slice behave like a self-contained governed runtime.
    unsafe {
        std::env::set_var(
            "EXCEL_SKILL_STOCK_DB",
            OsString::from(stock_db_path.as_os_str()),
        );
        std::env::set_var(
            "EXCEL_SKILL_FUNDAMENTAL_HISTORY_DB",
            OsString::from(fundamental_history_db_path.as_os_str()),
        );
        std::env::set_var(
            "EXCEL_SKILL_DISCLOSURE_HISTORY_DB",
            OsString::from(disclosure_history_db_path.as_os_str()),
        );
    }

    let callback_result = callback().map_err(SecurityRealDataValidationBackfillError::Fullstack);

    // 2026-04-12 CST: Restore every previous env override after the callback,
    // because later tool calls in the same CLI process must not inherit this
    // slice-local validation runtime accidentally.
    // Purpose: keep governed validation deterministic and side-effect scoped.
    restore_env_override("EXCEL_SKILL_STOCK_DB", previous_stock_db);
    restore_env_override(
        "EXCEL_SKILL_FUNDAMENTAL_HISTORY_DB",
        previous_fundamental_history_db,
    );
    restore_env_override(
        "EXCEL_SKILL_DISCLOSURE_HISTORY_DB",
        previous_disclosure_history_db,
    );

    callback_result
}

// 2026-04-12 CST: Centralize env restoration, because the validation override
// path now touches multiple governed-history variables and should restore them consistently.
// Purpose: avoid duplicate unsafe restore blocks when the validation runtime grows.
fn restore_env_override(key: &str, previous_value: Option<OsString>) {
    match previous_value {
        Some(previous_value) => unsafe {
            std::env::set_var(key, previous_value);
        },
        None => unsafe {
            std::env::remove_var(key);
        },
    }
}

// 2026-04-12 CST: Persist slice-local governed fundamental history, because
// validation slices should capture replayable financial context together with
// price history once fullstack has already fetched or reconstructed it.
// Purpose: let validation replay prefer slice-local stock information history.
fn persist_slice_fundamental_history(
    db_path: &Path,
    request: &SecurityRealDataValidationBackfillRequest,
    rows: &[GovernedFundamentalHistoryRow],
) -> Result<(), SecurityRealDataValidationBackfillError> {
    if rows.is_empty() {
        return Ok(());
    }

    let batch_id = format!("validation-slice:{}", request.slice_id.trim());
    let persisted_rows = rows
        .iter()
        .map(|context| {
            let input = SecurityFundamentalHistoryBackfillRecordInput {
                symbol: request.symbol.trim().to_string(),
                report_period: context.report_period.clone(),
                notice_date: context.notice_date.clone(),
                source: context.source.clone(),
                report_metrics: context.report_metrics.clone(),
            };
            let report_period = input.report_period.clone();
            Ok(SecurityFundamentalHistoryRecordRow {
                symbol: input.symbol,
                report_period: input.report_period,
                notice_date: input.notice_date,
                source: input.source,
                report_metrics_json: serde_json::to_string(&input.report_metrics).map_err(
                    |error| {
                        SecurityRealDataValidationBackfillError::FundamentalStorage(format!(
                            "failed to serialize slice-local report metrics: {error}"
                        ))
                    },
                )?,
                batch_id: batch_id.clone(),
                record_ref: format!(
                    "fundamental-history:{}:{}:v1",
                    request.symbol.trim(),
                    report_period
                ),
                created_at: request.created_at.trim().to_string(),
            })
        })
        .collect::<Result<Vec<_>, SecurityRealDataValidationBackfillError>>()?;
    let store = SecurityFundamentalHistoryStore::new(db_path.to_path_buf());
    store.upsert_rows(&persisted_rows).map_err(|error| {
        SecurityRealDataValidationBackfillError::FundamentalStorage(error.to_string())
    })
}

// 2026-04-12 CST: Persist slice-local governed disclosure history, because
// validation slices should capture replayable announcement context together with
// price history once fullstack has already fetched or reconstructed it.
// Purpose: let validation replay prefer slice-local stock event history.
fn persist_slice_disclosure_history(
    db_path: &Path,
    request: &SecurityRealDataValidationBackfillRequest,
    announcements: &[GovernedDisclosureHistoryRow],
) -> Result<(), SecurityRealDataValidationBackfillError> {
    if announcements.is_empty() {
        return Ok(());
    }

    let batch_id = format!("validation-slice:{}", request.slice_id.trim());
    let rows = announcements
        .iter()
        .map(|announcement| {
            let input = SecurityDisclosureHistoryBackfillRecordInput {
                symbol: request.symbol.trim().to_string(),
                published_at: announcement.published_at.clone(),
                title: announcement.title.clone(),
                article_code: announcement.article_code.clone(),
                category: announcement.category.clone(),
                source: "validation_slice_fullstack".to_string(),
            };
            SecurityDisclosureHistoryRecordRow {
                symbol: input.symbol,
                published_at: input.published_at,
                title: input.title.clone(),
                article_code: input.article_code.clone(),
                category: input.category,
                source: input.source,
                batch_id: batch_id.clone(),
                record_ref: input.article_code.unwrap_or_else(|| {
                    format!(
                        "disclosure-history:{}:{}:{}:v1",
                        request.symbol.trim(),
                        announcement.published_at.trim(),
                        sanitize_identifier(announcement.title.trim())
                    )
                }),
                created_at: request.created_at.trim().to_string(),
            }
        })
        .collect::<Vec<_>>();
    let store = SecurityDisclosureHistoryStore::new(db_path.to_path_buf());
    store.upsert_rows(&rows).map_err(|error| {
        SecurityRealDataValidationBackfillError::DisclosureStorage(error.to_string())
    })
}

// 2026-04-12 CST: Keep persisted JSON writes shared and deterministic, because
// both the fullstack context and manifest are operator-facing audit artifacts.
// Purpose: avoid duplicate fs/serde glue across the new validation-slice outputs.
fn persist_json(
    path: &Path,
    value: &impl Serialize,
) -> Result<(), SecurityRealDataValidationBackfillError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            SecurityRealDataValidationBackfillError::Persist(format!(
                "failed to create parent directory `{}`: {error}",
                parent.display()
            ))
        })?;
    }

    let payload = serde_json::to_vec_pretty(value).map_err(|error| {
        SecurityRealDataValidationBackfillError::Persist(format!(
            "failed to serialize json for `{}`: {error}",
            path.display()
        ))
    })?;
    fs::write(path, payload).map_err(|error| {
        SecurityRealDataValidationBackfillError::Persist(format!(
            "failed to write json `{}`: {error}",
            path.display()
        ))
    })
}

fn sanitize_identifier(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' => character,
            _ => '_',
        })
        .collect()
}

fn default_adjustment() -> String {
    "qfq".to_string()
}

fn default_sync_providers() -> Vec<String> {
    vec!["tencent".to_string(), "sina".to_string()]
}

fn default_lookback_days() -> usize {
    DEFAULT_LOOKBACK_DAYS
}

fn default_disclosure_limit() -> usize {
    DEFAULT_DISCLOSURE_LIMIT
}
