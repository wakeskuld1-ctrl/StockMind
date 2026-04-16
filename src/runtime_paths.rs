use std::path::PathBuf;

pub fn workspace_runtime_dir() -> Result<PathBuf, String> {
    // 2026-04-16 CST: Added because StockMind should prefer its own runtime env names
    // while still accepting inherited EXCEL_SKILL_* wiring from migrated tests.
    // Purpose: make the split repo independently runnable without breaking compatibility.
    if let Ok(path) = std::env::var("STOCKMIND_RUNTIME_DIR") {
        return Ok(PathBuf::from(path));
    }

    if let Ok(db_path) = std::env::var("STOCKMIND_RUNTIME_DB") {
        let db_path = PathBuf::from(db_path);
        return db_path.parent().map(PathBuf::from).ok_or_else(|| {
            format!(
                "STOCKMIND_RUNTIME_DB `{}` missing parent directory",
                db_path.display()
            )
        });
    }

    if let Ok(path) = std::env::var("EXCEL_SKILL_RUNTIME_DIR") {
        return Ok(PathBuf::from(path));
    }

    if let Ok(db_path) = std::env::var("EXCEL_SKILL_RUNTIME_DB") {
        let db_path = PathBuf::from(db_path);
        return db_path.parent().map(PathBuf::from).ok_or_else(|| {
            format!(
                "EXCEL_SKILL_RUNTIME_DB `{}` missing parent directory",
                db_path.display()
            )
        });
    }

    let current_dir = std::env::current_dir().map_err(|error| error.to_string())?;
    Ok(current_dir.join(".stockmind_runtime"))
}
