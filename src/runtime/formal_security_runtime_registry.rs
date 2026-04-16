use std::path::PathBuf;

use crate::runtime_paths::workspace_runtime_dir;

// 2026-04-14 CST: Added because round 2 plan B needs one formal registry for
// securities runtime SQLite locations instead of leaving each formal store to
// re-resolve its own path inline.
// Purpose: freeze the formal execution-store landing path before later formal
// stores join the same registry.
pub struct FormalSecurityRuntimeRegistry;

impl FormalSecurityRuntimeRegistry {
    // 2026-04-15 CST: Added because the second-layer runtime cleanup now needs one
    // governed resolver for every formal SQLite path instead of letting each store
    // rebuild env + fallback rules independently.
    // Purpose: keep the official runtime family on one path policy and prevent
    // future AI changes from reintroducing parallel default-path logic.
    fn runtime_dir_from_direct_db_env(env_var: &str) -> Result<Option<PathBuf>, String> {
        if let Ok(path) = std::env::var(env_var) {
            let db_path = PathBuf::from(path);
            let parent = db_path
                .parent()
                .map(|value| value.to_path_buf())
                .ok_or_else(|| format!("{env_var} path has no parent directory"))?;
            return Ok(Some(parent));
        }

        Ok(None)
    }

    // 2026-04-15 CST: Added because execution store should stop being the only
    // runtime DB that benefits from the formal registry.
    // Purpose: reuse one canonical env-override + runtime-root fallback helper for
    // every governed SQLite database in the runtime family.
    fn db_path_from_env_or_runtime_root(
        env_var: &str,
        fallback_file_name: &str,
    ) -> Result<PathBuf, String> {
        if let Ok(path) = std::env::var(env_var) {
            return Ok(PathBuf::from(path));
        }

        let runtime_dir = workspace_runtime_dir()?;
        Ok(runtime_dir.join(fallback_file_name))
    }

    pub fn execution_store_runtime_dir() -> Result<PathBuf, String> {
        if let Some(runtime_dir) =
            Self::runtime_dir_from_direct_db_env("EXCEL_SKILL_SECURITY_EXECUTION_DB")?
        {
            return Ok(runtime_dir);
        }

        workspace_runtime_dir()
    }

    pub fn execution_store_db_path() -> Result<PathBuf, String> {
        Self::db_path_from_env_or_runtime_root(
            "EXCEL_SKILL_SECURITY_EXECUTION_DB",
            "security_execution.db",
        )
    }

    pub fn stock_history_db_path() -> Result<PathBuf, String> {
        Self::db_path_from_env_or_runtime_root("EXCEL_SKILL_STOCK_DB", "stock_history.db")
    }

    pub fn external_proxy_db_path() -> Result<PathBuf, String> {
        Self::db_path_from_env_or_runtime_root(
            "EXCEL_SKILL_EXTERNAL_PROXY_DB",
            "security_external_proxy.db",
        )
    }

    pub fn fundamental_history_db_path() -> Result<PathBuf, String> {
        Self::db_path_from_env_or_runtime_root(
            "EXCEL_SKILL_FUNDAMENTAL_HISTORY_DB",
            "security_fundamental_history.db",
        )
    }

    pub fn disclosure_history_db_path() -> Result<PathBuf, String> {
        Self::db_path_from_env_or_runtime_root(
            "EXCEL_SKILL_DISCLOSURE_HISTORY_DB",
            "security_disclosure_history.db",
        )
    }

    // 2026-04-16 CST: Added because P0-1 needs one governed SQLite landing path for
    // dated corporate actions instead of letting holding-yield helpers invent a file
    // location ad hoc.
    // Purpose: keep corporate-action storage inside the same formal runtime registry
    // used by the rest of the governed securities runtime family.
    pub fn corporate_action_db_path() -> Result<PathBuf, String> {
        Self::db_path_from_env_or_runtime_root(
            "EXCEL_SKILL_CORPORATE_ACTION_DB",
            "security_corporate_action.db",
        )
    }

    pub fn resonance_db_path() -> Result<PathBuf, String> {
        Self::db_path_from_env_or_runtime_root("EXCEL_SKILL_RESONANCE_DB", "security_resonance.db")
    }

    pub fn signal_outcome_db_path() -> Result<PathBuf, String> {
        Self::db_path_from_env_or_runtime_root(
            "EXCEL_SKILL_SIGNAL_OUTCOME_DB",
            "signal_outcome_research.db",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RuntimeEnvGuard {
        pairs: Vec<(&'static str, Option<String>)>,
    }

    impl RuntimeEnvGuard {
        fn capture(keys: &[&'static str]) -> Self {
            Self {
                pairs: keys
                    .iter()
                    .map(|key| (*key, std::env::var(key).ok()))
                    .collect(),
            }
        }
    }

    impl Drop for RuntimeEnvGuard {
        fn drop(&mut self) {
            unsafe {
                for (key, value) in &self.pairs {
                    match value {
                        Some(original) => std::env::set_var(key, original),
                        None => std::env::remove_var(key),
                    }
                }
            }
        }
    }

    #[test]
    fn execution_store_db_path_prefers_direct_env_override() {
        let _guard = RuntimeEnvGuard::capture(&[
            "EXCEL_SKILL_SECURITY_EXECUTION_DB",
            "EXCEL_SKILL_RUNTIME_DIR",
            "EXCEL_SKILL_RUNTIME_DB",
        ]);

        unsafe {
            std::env::set_var(
                "EXCEL_SKILL_SECURITY_EXECUTION_DB",
                r"E:\tmp\custom_security_execution.db",
            );
            std::env::remove_var("EXCEL_SKILL_RUNTIME_DIR");
            std::env::remove_var("EXCEL_SKILL_RUNTIME_DB");
        }

        let resolved = FormalSecurityRuntimeRegistry::execution_store_db_path()
            .expect("execution db path should resolve");
        assert_eq!(
            resolved,
            PathBuf::from(r"E:\tmp\custom_security_execution.db")
        );
    }

    #[test]
    fn execution_store_db_path_falls_back_to_runtime_root() {
        let _guard = RuntimeEnvGuard::capture(&[
            "EXCEL_SKILL_SECURITY_EXECUTION_DB",
            "EXCEL_SKILL_RUNTIME_DIR",
            "EXCEL_SKILL_RUNTIME_DB",
        ]);

        unsafe {
            std::env::remove_var("EXCEL_SKILL_SECURITY_EXECUTION_DB");
            std::env::set_var("EXCEL_SKILL_RUNTIME_DIR", r"E:\tmp\runtime-root");
            std::env::remove_var("EXCEL_SKILL_RUNTIME_DB");
        }

        let resolved = FormalSecurityRuntimeRegistry::execution_store_db_path()
            .expect("runtime-root fallback should resolve");
        assert_eq!(
            resolved,
            PathBuf::from(r"E:\tmp\runtime-root\security_execution.db")
        );
    }

    #[test]
    fn execution_store_runtime_dir_uses_execution_db_parent_when_overridden() {
        let _guard = RuntimeEnvGuard::capture(&[
            "EXCEL_SKILL_SECURITY_EXECUTION_DB",
            "EXCEL_SKILL_RUNTIME_DIR",
            "EXCEL_SKILL_RUNTIME_DB",
        ]);

        unsafe {
            std::env::set_var(
                "EXCEL_SKILL_SECURITY_EXECUTION_DB",
                r"E:\tmp\formal-runtime\security_execution.db",
            );
            std::env::remove_var("EXCEL_SKILL_RUNTIME_DIR");
            std::env::remove_var("EXCEL_SKILL_RUNTIME_DB");
        }

        let resolved = FormalSecurityRuntimeRegistry::execution_store_runtime_dir()
            .expect("execution runtime dir should resolve");
        assert_eq!(resolved, PathBuf::from(r"E:\tmp\formal-runtime"));
    }

    #[test]
    fn governed_runtime_store_db_paths_prefer_direct_env_overrides() {
        let _guard = RuntimeEnvGuard::capture(&[
            "EXCEL_SKILL_STOCK_DB",
            "EXCEL_SKILL_EXTERNAL_PROXY_DB",
            "EXCEL_SKILL_FUNDAMENTAL_HISTORY_DB",
            "EXCEL_SKILL_DISCLOSURE_HISTORY_DB",
            "EXCEL_SKILL_CORPORATE_ACTION_DB",
            "EXCEL_SKILL_RESONANCE_DB",
            "EXCEL_SKILL_SIGNAL_OUTCOME_DB",
            "EXCEL_SKILL_RUNTIME_DIR",
            "EXCEL_SKILL_RUNTIME_DB",
        ]);

        unsafe {
            std::env::set_var("EXCEL_SKILL_STOCK_DB", r"E:\tmp\stock\stock_history.db");
            std::env::set_var(
                "EXCEL_SKILL_EXTERNAL_PROXY_DB",
                r"E:\tmp\proxy\security_external_proxy.db",
            );
            std::env::set_var(
                "EXCEL_SKILL_FUNDAMENTAL_HISTORY_DB",
                r"E:\tmp\fundamental\security_fundamental_history.db",
            );
            std::env::set_var(
                "EXCEL_SKILL_DISCLOSURE_HISTORY_DB",
                r"E:\tmp\disclosure\security_disclosure_history.db",
            );
            std::env::set_var(
                "EXCEL_SKILL_CORPORATE_ACTION_DB",
                r"E:\tmp\corporate\security_corporate_action.db",
            );
            std::env::set_var(
                "EXCEL_SKILL_RESONANCE_DB",
                r"E:\tmp\resonance\security_resonance.db",
            );
            std::env::set_var(
                "EXCEL_SKILL_SIGNAL_OUTCOME_DB",
                r"E:\tmp\signal\signal_outcome_research.db",
            );
            std::env::remove_var("EXCEL_SKILL_RUNTIME_DIR");
            std::env::remove_var("EXCEL_SKILL_RUNTIME_DB");
        }

        assert_eq!(
            FormalSecurityRuntimeRegistry::stock_history_db_path().unwrap(),
            PathBuf::from(r"E:\tmp\stock\stock_history.db")
        );
        assert_eq!(
            FormalSecurityRuntimeRegistry::external_proxy_db_path().unwrap(),
            PathBuf::from(r"E:\tmp\proxy\security_external_proxy.db")
        );
        assert_eq!(
            FormalSecurityRuntimeRegistry::fundamental_history_db_path().unwrap(),
            PathBuf::from(r"E:\tmp\fundamental\security_fundamental_history.db")
        );
        assert_eq!(
            FormalSecurityRuntimeRegistry::disclosure_history_db_path().unwrap(),
            PathBuf::from(r"E:\tmp\disclosure\security_disclosure_history.db")
        );
        assert_eq!(
            FormalSecurityRuntimeRegistry::corporate_action_db_path().unwrap(),
            PathBuf::from(r"E:\tmp\corporate\security_corporate_action.db")
        );
        assert_eq!(
            FormalSecurityRuntimeRegistry::resonance_db_path().unwrap(),
            PathBuf::from(r"E:\tmp\resonance\security_resonance.db")
        );
        assert_eq!(
            FormalSecurityRuntimeRegistry::signal_outcome_db_path().unwrap(),
            PathBuf::from(r"E:\tmp\signal\signal_outcome_research.db")
        );
    }

    #[test]
    fn governed_runtime_store_db_paths_fall_back_to_shared_runtime_root() {
        let _guard = RuntimeEnvGuard::capture(&[
            "EXCEL_SKILL_STOCK_DB",
            "EXCEL_SKILL_EXTERNAL_PROXY_DB",
            "EXCEL_SKILL_FUNDAMENTAL_HISTORY_DB",
            "EXCEL_SKILL_DISCLOSURE_HISTORY_DB",
            "EXCEL_SKILL_RESONANCE_DB",
            "EXCEL_SKILL_SIGNAL_OUTCOME_DB",
            "EXCEL_SKILL_RUNTIME_DIR",
            "EXCEL_SKILL_RUNTIME_DB",
        ]);

        unsafe {
            std::env::remove_var("EXCEL_SKILL_STOCK_DB");
            std::env::remove_var("EXCEL_SKILL_EXTERNAL_PROXY_DB");
            std::env::remove_var("EXCEL_SKILL_FUNDAMENTAL_HISTORY_DB");
            std::env::remove_var("EXCEL_SKILL_DISCLOSURE_HISTORY_DB");
            std::env::remove_var("EXCEL_SKILL_RESONANCE_DB");
            std::env::remove_var("EXCEL_SKILL_SIGNAL_OUTCOME_DB");
            std::env::set_var("EXCEL_SKILL_RUNTIME_DIR", r"E:\tmp\family-runtime");
            std::env::remove_var("EXCEL_SKILL_RUNTIME_DB");
        }

        assert_eq!(
            FormalSecurityRuntimeRegistry::stock_history_db_path().unwrap(),
            PathBuf::from(r"E:\tmp\family-runtime\stock_history.db")
        );
        assert_eq!(
            FormalSecurityRuntimeRegistry::external_proxy_db_path().unwrap(),
            PathBuf::from(r"E:\tmp\family-runtime\security_external_proxy.db")
        );
        assert_eq!(
            FormalSecurityRuntimeRegistry::fundamental_history_db_path().unwrap(),
            PathBuf::from(r"E:\tmp\family-runtime\security_fundamental_history.db")
        );
        assert_eq!(
            FormalSecurityRuntimeRegistry::disclosure_history_db_path().unwrap(),
            PathBuf::from(r"E:\tmp\family-runtime\security_disclosure_history.db")
        );
        assert_eq!(
            FormalSecurityRuntimeRegistry::resonance_db_path().unwrap(),
            PathBuf::from(r"E:\tmp\family-runtime\security_resonance.db")
        );
        assert_eq!(
            FormalSecurityRuntimeRegistry::signal_outcome_db_path().unwrap(),
            PathBuf::from(r"E:\tmp\family-runtime\signal_outcome_research.db")
        );
    }
}
