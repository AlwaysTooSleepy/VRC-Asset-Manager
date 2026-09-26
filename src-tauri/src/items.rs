//! Models and their versions: creating and editing entries, and filing version
//! folders. Folder names follow Models/<Creator>/<Name>_V_<version>.

use crate::{filing, fsops, images, models::*, reference, store, sync};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::Path,
};
use tauri::AppHandle;

fn e2s<E: ToString>(e: E) -> String {
    e.to_string()
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ---------- shared validation ----------

/// Exact shape the frontend sends for add_model and update_model (see ModelInput in src/types.ts).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInput {
    pub name: String,
    pub creator_id: String,
    pub site_id: String,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub image_paths: Vec<String>,
}

struct CleanModel {
    name: String,
    creator_id: String,
    site_id: String,
    source_url: Option<String>,
    image_paths: Vec<String>,
}

pub(crate) fn clean_url(raw: Option<&str>) -> Result<Option<String>, String> {
    match raw.map(str::trim) {
        None | Some("") => Ok(None),
        Some(u) if u.starts_with("http://") || u.starts_with("https://") => Ok(Some(u.to_string())),
        Some(u) => Err(format!("'{u}' is not a valid link (it must start with http:// or https://)")),
    }
}

fn clean_model(lib: &Library, input: &ModelInput) -> Result<CleanModel, String> {
    let name = fsops::validate_name("Model", &input.name)?;
    if !lib.creators.iter().any(|c| c.id == input.creator_id) {
        return Err("Choose a creator".to_string());
    }
    if !lib.sites.iter().any(|s| s.id == input.site_id) {
        return Err("Choose a site".to_string());
    }
    let mut image_paths: Vec<String> = vec![];
    for p in &input.image_paths {
        images::check_ref(p)?;
        if !image_paths.contains(p) {
            image_paths.push(p.clone());
        }
    }
    Ok(CleanModel {
        name,
        creator_id: input.creator_id.clone(),
        site_id: input.site_id.clone(),
        source_url: clean_url(input.source_url.as_deref())?,
        image_paths,
    })
}

pub(crate) fn creator_name<'a>(lib: &'a Library, id: &str) -> Result<&'a str, String> {
    lib.creators
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.name.as_str())
        .ok_or_else(|| "Choose a creator".to_string())
}

fn duplicate_check(lib: &Library, creator_id: &str, name: &str, except: Option<&str>) -> Result<(), String> {
    let lower = name.to_lowercase();
    if lib.models.iter().any(|m| {
        Some(m.id.as_str()) != except && m.creator_id == creator_id && m.name.to_lowercase() == lower
    }) {
        return Err(format!(
            "'{name}' by {} is already in your library. Open it and use \"Add version\" to add another version.",
            creator_name(lib, creator_id)?
        ));
    }
    Ok(())
}

// ---------- add / edit / remove a model ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddModelResult {
    pub library: Library,
    pub model_id: String,
}

/// Creates the model entry and its creator folder. Files are filed later, one
/// version at a time, with add_version.
#[tauri::command]
pub fn add_model(app: AppHandle, input: ModelInput) -> Result<AddModelResult, String> {
    let (root, mut lib) = reference::open(&app)?;
    let c = clean_model(&lib, &input)?;
    duplicate_check(&lib, &c.creator_id, &c.name, None)?;
    let creator = creator_name(&lib, &c.creator_id)?.to_string();
    fs::create_dir_all(root.join("Models").join(&creator)).map_err(e2s)?;

    let model_id = uuid::Uuid::new_v4().to_string();
    let stamp = now();
    lib.models.push(Model {
        id: model_id.clone(),
        name: c.name,
        creator_id: c.creator_id,
        site_id: c.site_id,
        source_url: c.source_url,
        folder_path: String::new(),
        image_paths: c.image_paths,
        versions: vec![],
        date_added: stamp.clone(),
        date_updated: stamp,
        missing: false,
    });
    store::save_library(&root, &lib)?;
    Ok(AddModelResult { library: lib, model_id })
}

/// Edits a model. Changing its name or creator also renames/moves the folders
/// of all its versions, all-or-nothing.
#[tauri::command]
pub fn update_model(app: AppHandle, id: String, input: ModelInput) -> Result<Library, String> {
    let (root, mut lib) = reference::open(&app)?;
    let c = clean_model(&lib, &input)?;
    let idx = lib.models.iter().position(|m| m.id == id).ok_or("Model not found")?;
    duplicate_check(&lib, &c.creator_id, &c.name, Some(id.as_str()))?;
    let creator = creator_name(&lib, &c.creator_id)?.to_string();
    let old_images = lib.models[idx].image_paths.clone();
    let before = lib.clone();

    let changed = lib.models[idx].name != c.name || lib.models[idx].creator_id != c.creator_id;
    let mut pairs: Vec<(String, String)> = vec![];
    if changed {
        for v in &lib.models[idx].versions {
            let new_rel = format!("Models/{}/{}_V_{}", creator, c.name, v.version_label);
            if new_rel == v.folder_path {
                continue;
            }
            let same_ci = new_rel.to_lowercase() == v.folder_path.to_lowercase();
            if !same_ci && root.join(&new_rel).exists() {
                return Err(format!("Cannot rename: '{new_rel}' already exists on disk"));
            }
            pairs.push((v.folder_path.clone(), new_rel));
        }
    }

    {
        let m = &mut lib.models[idx];
        m.name = c.name;
        m.creator_id = c.creator_id;
        m.site_id = c.site_id;
        m.source_url = c.source_url;
        m.image_paths = c.image_paths;
        m.date_updated = now();
    }
    // Asset versions keep this model's files in a `For_<Model>` subfolder; rename those too.
    for (old_sub, new_sub) in filing::subfolder_renames(&before, &lib) {
        for a in &lib.assets {
            for v in &a.versions {
                let old_rel = format!("{}/{}", v.folder_path, old_sub);
                if root.join(&old_rel).is_dir() {
                    pairs.push((old_rel, format!("{}/{}", v.folder_path, new_sub)));
                }
            }
        }
    }
    reference::save_with_moves(&root, &mut lib, &pairs)?;
    images::prune_unused(&root, &lib, &old_images);
    Ok(lib)
}

fn all_folders_absent(root: &Path, model: &Model) -> bool {
    model.versions.iter().all(|v| !root.join(&v.folder_path).exists())
}

/// Removes a model's entry. Only allowed once none of its folders exist any
/// more (otherwise the next sync would simply find it again). Never deletes files.
#[tauri::command]
pub fn remove_model(app: AppHandle, id: String) -> Result<Library, String> {
    let (root, mut lib) = reference::open(&app)?;
    let idx = lib.models.iter().position(|m| m.id == id).ok_or("Model not found")?;
    if !all_folders_absent(&root, &lib.models[idx]) {
        return Err("This model's folders still exist on disk. Delete or move them first, then refresh.".to_string());
    }
    let removed = lib.models.remove(idx);
    for a in lib.assets.iter_mut() {
        a.compatible_model_ids.retain(|m| m != &id);
    }
    store::save_library(&root, &lib)?;
    images::prune_unused(&root, &lib, &removed.image_paths);
    Ok(lib)
}

// ---------- versions ----------

fn plan_version(root: &Path, lib: &Library, model_id: &str, version: &str) -> Result<(usize, String, String), String> {
    let label = fsops::validate_name("Version", version)?;
    let idx = lib.models.iter().position(|m| m.id == model_id).ok_or("Model not found")?;
    let model = &lib.models[idx];
    if model.versions.iter().any(|v| v.version_label.to_lowercase() == label.to_lowercase()) {
        return Err(format!("Version '{label}' already exists for this model"));
    }
    let creator = creator_name(lib, &model.creator_id)?;
    let dest_rel = format!("Models/{creator}/{}_V_{label}", model.name);
    if root.join(&dest_rel).exists() {
        return Err(format!("The folder '{dest_rel}' already exists on disk"));
    }
    Ok((idx, label, dest_rel))
}

#[tauri::command]
pub fn preview_version_path(app: AppHandle, model_id: String, version: String) -> Result<String, String> {
    let (root, lib) = reference::open(&app)?;
    Ok(plan_version(&root, &lib, &model_id, &version)?.2)
}

/// Exact shape the frontend sends (see AddVersionInput in src/types.ts).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddVersionInput {
    pub model_id: String,
    pub version: String,
    #[serde(default)]
    pub notes: Option<String>,
    /// Folder to move into the library. If absent, an empty folder is created.
    #[serde(default)]
    pub source_folder: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddVersionResult {
    pub library: Library,
    /// Library-relative folder the version lives in.
    pub destination: String,
    pub warning: Option<String>,
}

pub(crate) fn clean_notes(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim).filter(|n| !n.is_empty()).map(String::from)
}

fn add_version_blocking(app: AppHandle, input: AddVersionInput) -> Result<AddVersionResult, String> {
    let (root, mut lib) = reference::open(&app)?;
    let (idx, label, dest_rel) = plan_version(&root, &lib, &input.model_id, &input.version)?;
    let dest = root.join(&dest_rel);
    let filed = filing::file_single(&root, &dest, input.source_folder.as_deref())?;

    let stamp = now();
    {
        let m = &mut lib.models[idx];
        m.versions.push(Version {
            version_label: label,
            folder_path: dest_rel.clone(),
            date_added: stamp.clone(),
            notes: clean_notes(input.notes.as_deref()),
            missing: false,
        });
        m.date_updated = stamp;
        sync::refresh_current(m);
    }

    if let Err(e) = store::save_library(&root, &lib) {
        return Err(filing::undo(&filed, &e));
    }
    Ok(AddVersionResult { library: lib, destination: dest_rel, warning: filed.warning })
}

/// Async + spawn_blocking so moving a large folder never freezes the window.
#[tauri::command]
pub async fn add_version(app: AppHandle, input: AddVersionInput) -> Result<AddVersionResult, String> {
    tauri::async_runtime::spawn_blocking(move || add_version_blocking(app, input))
        .await
        .map_err(|e| e.to_string())?
}

/// Exact shape the frontend sends (see UpdateVersionInput in src/types.ts).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateVersionInput {
    pub model_id: String,
    /// Identifies the version: its current folder path.
    pub folder_path: String,
    pub version: String,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Edits a version's label (which renames its folder) and notes.
#[tauri::command]
pub fn update_version(app: AppHandle, input: UpdateVersionInput) -> Result<Library, String> {
    let (root, mut lib) = reference::open(&app)?;
    let midx = lib.models.iter().position(|m| m.id == input.model_id).ok_or("Model not found")?;
    let vidx = lib.models[midx]
        .versions
        .iter()
        .position(|v| v.folder_path == input.folder_path)
        .ok_or("Version not found")?;
    let label = fsops::validate_name("Version", &input.version)?;

    let old_label = lib.models[midx].versions[vidx].version_label.clone();
    let mut pairs: Vec<(String, String)> = vec![];
    if label != old_label {
        if lib.models[midx].versions[vidx].missing {
            return Err("This version's folder is missing, so it can't be renamed. Restore the folder or remove the entry.".to_string());
        }
        let lower = label.to_lowercase();
        if lib.models[midx]
            .versions
            .iter()
            .enumerate()
            .any(|(i, v)| i != vidx && v.version_label.to_lowercase() == lower)
        {
            return Err(format!("Version '{label}' already exists for this model"));
        }
        let old_path = input.folder_path.clone();
        let parent = old_path.rsplit_once('/').map(|(p, _)| p.to_string()).ok_or("Invalid version path")?;
        let new_rel = format!("{parent}/{}_V_{label}", lib.models[midx].name);
        if new_rel != old_path {
            let same_ci = new_rel.to_lowercase() == old_path.to_lowercase();
            if !same_ci && root.join(&new_rel).exists() {
                return Err(format!("Cannot rename: '{new_rel}' already exists on disk"));
            }
            pairs.push((old_path, new_rel));
        }
    }

    {
        let m = &mut lib.models[midx];
        m.versions[vidx].version_label = label;
        m.versions[vidx].notes = clean_notes(input.notes.as_deref());
        m.date_updated = now();
        sync::refresh_current(m); // before the path rewrite, which then fixes up folder_path too
    }
    reference::save_with_moves(&root, &mut lib, &pairs)?;
    Ok(lib)
}

/// Removes a version's entry. Only allowed when its folder is already gone.
/// Never deletes files.
#[tauri::command]
pub fn remove_version(app: AppHandle, model_id: String, folder_path: String) -> Result<Library, String> {
    let (root, mut lib) = reference::open(&app)?;
    let midx = lib.models.iter().position(|m| m.id == model_id).ok_or("Model not found")?;
    let vidx = lib.models[midx]
        .versions
        .iter()
        .position(|v| v.folder_path == folder_path)
        .ok_or("Version not found")?;
    if root.join(&folder_path).exists() {
        return Err("This version's folder still exists on disk. Delete or move it first, then refresh.".to_string());
    }
    let m = &mut lib.models[midx];
    m.versions.remove(vidx);
    m.date_updated = now();
    sync::refresh_current(m);
    store::save_library(&root, &lib)?;
    Ok(lib)
}
