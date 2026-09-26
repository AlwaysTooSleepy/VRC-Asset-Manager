//! Hands folders and links to the operating system. These run from the backend
//! (not from the webview), so what can be opened is limited to what we validate here.

use crate::store;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

/// Opens a folder inside the library (path relative to the library root) in the
/// OS file explorer.
#[tauri::command]
pub fn open_folder(app: AppHandle, path: String) -> Result<(), String> {
    let root = store::require_root(&app)?;
    if path.is_empty()
        || path.starts_with('/')
        || path.contains("..")
        || path.contains('\\')
        || path.contains(':')
    {
        return Err(format!("'{path}' is not a valid library path"));
    }
    // Build with the OS's own separators.
    let full: PathBuf = path.split('/').fold(root, |p, part| p.join(part));
    if !full.is_dir() {
        return Err(format!("The folder '{}' no longer exists", full.display()));
    }
    app.opener()
        .open_path(full.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// Opens a web link in the default browser. Only http(s) links are allowed.
#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    let url = url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(format!("'{url}' is not a valid link (it must start with http:// or https://)"));
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}
