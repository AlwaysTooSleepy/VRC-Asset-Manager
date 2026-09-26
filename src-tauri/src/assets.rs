//! Assets: entries, versions, and filing their folders under
//! Assets/<Category>/<Creator>/<Name>_V_<version>.

use crate::{filing, fsops, images, items, models::*, reference, store, sync};
use serde::Deserialize;
use std::{fs, path::Path};
use tauri::AppHandle;

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn asset_rel(category: &str, creator: &str, name: &str, label: &str) -> String {
    format!("Assets/{category}/{creator}/{name}_V_{label}")
}

fn category_name<'a>(lib: &'a Library, id: &str) -> Result<&'a str, String> {
    lib.categories
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.name.as_str())
        .ok_or_else(|| "Choose a category".to_string())
}

// ---------- validation ----------

/// Exact shape the frontend sends for add_asset and update_asset (see AssetInput in
/// src/types.ts). The compatibility fields are always sent (Vision doc section 7).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetInput {
    pub name: String,
    pub creator_id: String,
    pub site_id: String,
    pub category_id: String,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub image_paths: Vec<String>,
    #[serde(default)]
    pub compatible_model_ids: Vec<String>,
    #[serde(default)]
    pub compatible_with_all: bool,
}

struct CleanAsset {
    name: String,
    creator_id: String,
    site_id: String,
    category_id: String,
    source_url: Option<String>,
    image_paths: Vec<String>,
    compatible_model_ids: Vec<String>,
    compatible_with_all: bool,
}

fn clean_asset(lib: &Library, input: &AssetInput) -> Result<CleanAsset, String> {
    let name = fsops::validate_name("Asset", &input.name)?;
    if !lib.creators.iter().any(|c| c.id == input.creator_id) {
        return Err("Choose a creator".to_string());
    }
    if !lib.sites.iter().any(|s| s.id == input.site_id) {
        return Err("Choose a site".to_string());
    }
    if !lib.categories.iter().any(|c| c.id == input.category_id) {
        return Err("Choose a category".to_string());
    }

    let mut image_paths: Vec<String> = vec![];
    for p in &input.image_paths {
        images::check_ref(p)?;
        if !image_paths.contains(p) {
            image_paths.push(p.clone());
        }
    }

    let mut model_ids: Vec<String> = vec![];
    if !input.compatible_with_all {
        for id in &input.compatible_model_ids {
            if !lib.models.iter().any(|m| &m.id == id) {
                return Err("One of the selected models no longer exists".to_string());
            }
            if !model_ids.contains(id) {
                model_ids.push(id.clone());
            }
        }
        if model_ids.is_empty() {
            return Err("Choose which models this asset works with, or turn on \"all models\"".to_string());
        }
    }

    Ok(CleanAsset {
        name,
        creator_id: input.creator_id.clone(),
        site_id: input.site_id.clone(),
        category_id: input.category_id.clone(),
        source_url: items::clean_url(input.source_url.as_deref())?,
        image_paths,
        compatible_model_ids: model_ids,
        compatible_with_all: input.compatible_with_all,
    })
}

fn duplicate_check(lib: &Library, c: &CleanAsset, except: Option<&str>) -> Result<(), String> {
    let lower = c.name.to_lowercase();
    if lib.assets.iter().any(|a| {
        Some(a.id.as_str()) != except
            && a.creator_id == c.creator_id
            && a.category_id == c.category_id
            && a.name.to_lowercase() == lower
    }) {
        return Err(format!(
            "'{}' by {} is already in your library under that category. Open it and use \"Add version\" to add another version.",
            c.name,
            items::creator_name(lib, &c.creator_id)?
        ));
    }
    Ok(())
}

// ---------- add / edit / remove an asset ----------

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddAssetResult {
    pub library: Library,
    pub asset_id: String,
}

/// Creates the asset entry and its Assets/<Category>/<Creator> folder. Files are
/// filed later, one version at a time.
#[tauri::command]
pub fn add_asset(app: AppHandle, input: AssetInput) -> Result<AddAssetResult, String> {
    let (root, mut lib) = reference::open(&app)?;
    let c = clean_asset(&lib, &input)?;
    duplicate_check(&lib, &c, None)?;
    let category = category_name(&lib, &c.category_id)?.to_string();
    let creator = items::creator_name(&lib, &c.creator_id)?.to_string();
    fs::create_dir_all(root.join("Assets").join(&category).join(&creator)).map_err(|e| e.to_string())?;

    let asset_id = uuid::Uuid::new_v4().to_string();
    let stamp = now();
    lib.assets.push(Asset {
        id: asset_id.clone(),
        name: c.name,
        creator_id: c.creator_id,
        site_id: c.site_id,
        category_id: c.category_id,
        source_url: c.source_url,
        folder_path: String::new(),
        image_paths: c.image_paths,
        versions: vec![],
        compatible_model_ids: c.compatible_model_ids,
        compatible_with_all: c.compatible_with_all,
        date_added: stamp.clone(),
        date_updated: stamp,
        missing: false,
    });
    store::save_library(&root, &lib)?;
    Ok(AddAssetResult { library: lib, asset_id })
}

/// Edits an asset. Changing its name, creator or category also moves the
/// folders of all its versions, all-or-nothing.
#[tauri::command]
pub fn update_asset(app: AppHandle, id: String, input: AssetInput) -> Result<Library, String> {
    let (root, mut lib) = reference::open(&app)?;
    let c = clean_asset(&lib, &input)?;
    let idx = lib.assets.iter().position(|a| a.id == id).ok_or("Asset not found")?;
    duplicate_check(&lib, &c, Some(id.as_str()))?;
    let category = category_name(&lib, &c.category_id)?.to_string();
    let creator = items::creator_name(&lib, &c.creator_id)?.to_string();
    let old_images = lib.assets[idx].image_paths.clone();

    let a = &lib.assets[idx];
    let changed = a.name != c.name || a.creator_id != c.creator_id || a.category_id != c.category_id;
    let mut pairs: Vec<(String, String)> = vec![];
    if changed {
        for v in &a.versions {
            let new_rel = asset_rel(&category, &creator, &c.name, &v.version_label);
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
        let a = &mut lib.assets[idx];
        a.name = c.name;
        a.creator_id = c.creator_id;
        a.site_id = c.site_id;
        a.category_id = c.category_id;
        a.source_url = c.source_url;
        a.image_paths = c.image_paths;
        a.compatible_model_ids = c.compatible_model_ids;
        a.compatible_with_all = c.compatible_with_all;
        a.date_updated = now();
    }
    reference::save_with_moves(&root, &mut lib, &pairs)?;
    images::prune_unused(&root, &lib, &old_images);
    Ok(lib)
}

fn all_absent(root: &Path, versions: &[Version]) -> bool {
    versions.iter().all(|v| !root.join(&v.folder_path).exists())
}

/// Removes an asset's entry. Only allowed once none of its folders exist any
/// more (otherwise the next sync would simply find it again). Never deletes files.
#[tauri::command]
pub fn remove_asset(app: AppHandle, id: String) -> Result<Library, String> {
    let (root, mut lib) = reference::open(&app)?;
    let idx = lib.assets.iter().position(|a| a.id == id).ok_or("Asset not found")?;
    if !all_absent(&root, &lib.assets[idx].versions) {
        return Err("This asset's folders still exist on disk. Delete or move them first, then refresh.".to_string());
    }
    let removed = lib.assets.remove(idx);
    store::save_library(&root, &lib)?;
    images::prune_unused(&root, &lib, &removed.image_paths);
    Ok(lib)
}

// ---------- versions ----------

fn plan_version(root: &Path, lib: &Library, asset_id: &str, version: &str) -> Result<(usize, String, String), String> {
    let label = fsops::validate_name("Version", version)?;
    let idx = lib.assets.iter().position(|a| a.id == asset_id).ok_or("Asset not found")?;
    let asset = &lib.assets[idx];
    if asset.versions.iter().any(|v| v.version_label.to_lowercase() == label.to_lowercase()) {
        return Err(format!("Version '{label}' already exists for this asset"));
    }
    let category = category_name(lib, &asset.category_id)?;
    let creator = items::creator_name(lib, &asset.creator_id)?;
    let dest_rel = asset_rel(category, creator, &asset.name, &label);
    if root.join(&dest_rel).exists() {
        return Err(format!("The folder '{dest_rel}' already exists on disk"));
    }
    Ok((idx, label, dest_rel))
}

/// What adding a version would create, for the live preview.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetVersionPreview {
    /// Library-relative version folder.
    pub path: String,
    /// Subfolders that would be created inside it (empty for a flat layout).
    pub subfolders: Vec<String>,
}

/// Dry run of add_asset_version: validates the label and every source, moves nothing.
#[tauri::command]
pub fn preview_asset_version(
    app: AppHandle,
    asset_id: String,
    version: String,
    sources: Vec<filing::SourceInput>,
) -> Result<AssetVersionPreview, String> {
    let (root, lib) = reference::open(&app)?;
    let (idx, _label, dest_rel) = plan_version(&root, &lib, &asset_id, &version)?;
    let plan = filing::plan_sources(&root, &lib, &lib.assets[idx], &sources)?;
    Ok(AssetVersionPreview { path: dest_rel, subfolders: plan.subfolders.clone() })
}

/// Exact shape the frontend sends (see AddAssetVersionInput in src/types.ts).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddAssetVersionInput {
    pub asset_id: String,
    pub version: String,
    #[serde(default)]
    pub notes: Option<String>,
    /// Files and folders to move in, each optionally aimed at a model or "Shared".
    /// Empty creates an empty version folder.
    #[serde(default)]
    pub sources: Vec<filing::SourceInput>,
}

fn add_version_blocking(app: AppHandle, input: AddAssetVersionInput) -> Result<items::AddVersionResult, String> {
    let (root, mut lib) = reference::open(&app)?;
    let (idx, label, dest_rel) = plan_version(&root, &lib, &input.asset_id, &input.version)?;
    let dest = root.join(&dest_rel);
    let plan = filing::plan_sources(&root, &lib, &lib.assets[idx], &input.sources)?;
    let filed = filing::file_sources(&dest, &plan)?;

    let stamp = now();
    {
        let a = &mut lib.assets[idx];
        a.versions.push(Version {
            version_label: label,
            folder_path: dest_rel.clone(),
            date_added: stamp.clone(),
            notes: items::clean_notes(input.notes.as_deref()),
            missing: false,
        });
        a.date_updated = stamp;
        sync::refresh_asset_current(a);
    }

    if let Err(e) = store::save_library(&root, &lib) {
        return Err(filing::undo(&filed, &e));
    }
    Ok(items::AddVersionResult { library: lib, destination: dest_rel, warning: filed.warning })
}

/// Async + spawn_blocking so moving a large folder never freezes the window.
#[tauri::command]
pub async fn add_asset_version(app: AppHandle, input: AddAssetVersionInput) -> Result<items::AddVersionResult, String> {
    tauri::async_runtime::spawn_blocking(move || add_version_blocking(app, input))
        .await
        .map_err(|e| e.to_string())?
}

/// Exact shape the frontend sends (see UpdateAssetVersionInput in src/types.ts).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAssetVersionInput {
    pub asset_id: String,
    /// Identifies the version: its current folder path.
    pub folder_path: String,
    pub version: String,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Edits a version's label (which renames its folder) and notes.
#[tauri::command]
pub fn update_asset_version(app: AppHandle, input: UpdateAssetVersionInput) -> Result<Library, String> {
    let (root, mut lib) = reference::open(&app)?;
    let aidx = lib.assets.iter().position(|a| a.id == input.asset_id).ok_or("Asset not found")?;
    let vidx = lib.assets[aidx]
        .versions
        .iter()
        .position(|v| v.folder_path == input.folder_path)
        .ok_or("Version not found")?;
    let label = fsops::validate_name("Version", &input.version)?;

    let old_label = lib.assets[aidx].versions[vidx].version_label.clone();
    let mut pairs: Vec<(String, String)> = vec![];
    if label != old_label {
        if lib.assets[aidx].versions[vidx].missing {
            return Err("This version's folder is missing, so it can't be renamed. Restore the folder or remove the entry.".to_string());
        }
        let lower = label.to_lowercase();
        if lib.assets[aidx]
            .versions
            .iter()
            .enumerate()
            .any(|(i, v)| i != vidx && v.version_label.to_lowercase() == lower)
        {
            return Err(format!("Version '{label}' already exists for this asset"));
        }
        let old_path = input.folder_path.clone();
        let parent = old_path.rsplit_once('/').map(|(p, _)| p.to_string()).ok_or("Invalid version path")?;
        let new_rel = format!("{parent}/{}_V_{label}", lib.assets[aidx].name);
        if new_rel != old_path {
            let same_ci = new_rel.to_lowercase() == old_path.to_lowercase();
            if !same_ci && root.join(&new_rel).exists() {
                return Err(format!("Cannot rename: '{new_rel}' already exists on disk"));
            }
            pairs.push((old_path, new_rel));
        }
    }

    {
        let a = &mut lib.assets[aidx];
        a.versions[vidx].version_label = label;
        a.versions[vidx].notes = items::clean_notes(input.notes.as_deref());
        a.date_updated = now();
        sync::refresh_asset_current(a); // before the path rewrite, which then fixes up folder_path too
    }
    reference::save_with_moves(&root, &mut lib, &pairs)?;
    Ok(lib)
}

/// Removes a version's entry. Only allowed when its folder is already gone.
/// Never deletes files.
#[tauri::command]
pub fn remove_asset_version(app: AppHandle, asset_id: String, folder_path: String) -> Result<Library, String> {
    let (root, mut lib) = reference::open(&app)?;
    let aidx = lib.assets.iter().position(|a| a.id == asset_id).ok_or("Asset not found")?;
    let vidx = lib.assets[aidx]
        .versions
        .iter()
        .position(|v| v.folder_path == folder_path)
        .ok_or("Version not found")?;
    if root.join(&folder_path).exists() {
        return Err("This version's folder still exists on disk. Delete or move it first, then refresh.".to_string());
    }
    let a = &mut lib.assets[aidx];
    a.versions.remove(vidx);
    a.date_updated = now();
    sync::refresh_asset_current(a);
    store::save_library(&root, &lib)?;
    Ok(lib)
}
