use crate::{models::Library, store, sync};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn get_library_root(app: AppHandle) -> Result<Option<String>, String> {
    Ok(store::read_root(&app)?.map(|p| p.to_string_lossy().into_owned()))
}

/// Points the app at a library folder (existing or new), builds the folder
/// skeleton, and returns the library found there. Nothing is moved: switching
/// folders switches libraries.
#[tauri::command]
pub fn set_library_root(app: AppHandle, root: String) -> Result<Library, String> {
    let root = PathBuf::from(root);
    if !root.is_dir() {
        return Err(format!("'{}' is not an existing folder", root.display()));
    }
    store::ensure_structure(&root)?;
    let mut library = store::load_library(&root)?;
    // Pick up whatever is already in this folder (models filed earlier, or by hand).
    sync::reconcile(&root, &mut library);
    store::save_library(&root, &library)?;
    store::write_root(&app, &root)?;
    // Let the webview display images stored inside this library.
    app.asset_protocol_scope()
        .allow_directory(&root, true)
        .map_err(|e| e.to_string())?;
    Ok(library)
}

#[tauri::command]
pub fn get_library(app: AppHandle) -> Result<Library, String> {
    store::load_library(&store::require_root(&app)?)
}

/// Compares the library with the folders on disk (see sync.rs) and returns the result.
#[tauri::command]
pub fn sync_library(app: AppHandle) -> Result<Library, String> {
    let root = store::require_root(&app)?;
    let mut library = store::load_library(&root)?;
    sync::sync_and_save(&root, &mut library)?;
    Ok(library)
}

