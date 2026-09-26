//! Persistence: where the library root lives, folder skeleton, and SQLite store.

use crate::{db, models::Library};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};

fn e2s<E: ToString>(e: E) -> String {
    e.to_string()
}

/// Small app-level config. Stored in the OS config dir because we must know
/// the library root *before* we can find the library file inside it.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AppConfig {
    library_root: Option<String>,
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(e2s)?;
    fs::create_dir_all(&dir).map_err(e2s)?;
    Ok(dir.join("config.json"))
}

pub fn read_root(app: &AppHandle) -> Result<Option<PathBuf>, String> {
    let path = config_path(app)?;
    if !path.exists() {
        return Ok(None);
    }
    let cfg: AppConfig =
        serde_json::from_str(&fs::read_to_string(&path).map_err(e2s)?).map_err(e2s)?;
    Ok(cfg.library_root.map(PathBuf::from))
}

pub fn require_root(app: &AppHandle) -> Result<PathBuf, String> {
    read_root(app)?.ok_or_else(|| "Library root has not been set".to_string())
}

pub fn write_root(app: &AppHandle, root: &Path) -> Result<(), String> {
    let cfg = AppConfig {
        library_root: Some(root.to_string_lossy().into_owned()),
    };
    fs::write(
        config_path(app)?,
        serde_json::to_string_pretty(&cfg).map_err(e2s)?,
    )
    .map_err(e2s)
}

/// Creates Models\, Assets\, AppData\Images\ if missing.
pub fn ensure_structure(root: &Path) -> Result<(), String> {
    for sub in ["Models", "Assets", "AppData/Images"] {
        fs::create_dir_all(root.join(sub)).map_err(e2s)?;
    }
    Ok(())
}

pub fn load_library(root: &Path) -> Result<Library, String> {
    ensure_structure(root)?;
    db::load_library(root)
}

pub fn save_library(root: &Path, lib: &Library) -> Result<(), String> {
    ensure_structure(root)?;
    db::save_library(root, lib)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_folder_gets_a_fresh_sqlite_store() {
        let root = std::env::temp_dir().join(format!(
            "vrchat-asset-manager-empty-{}",
            uuid::Uuid::new_v4()
        ));
        let library = load_library(&root).unwrap();
        assert_eq!(library.schema_version, 1);
        assert!(db::path(&root).is_file());
        fs::remove_dir_all(root).unwrap();
    }
}
