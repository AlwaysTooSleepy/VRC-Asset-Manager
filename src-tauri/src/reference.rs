//! Sites, Creators and Categories. Items reference these by ID, so a rename
//! only changes the name here. Creator and Category names are also folder
//! names, so renaming them renames folders on disk and rewrites the stored
//! paths, all-or-nothing: validate, move, save, and undo the moves on failure.

use crate::{fsops, images, models::*, store};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

// ---------- helpers ----------

pub(crate) fn open(app: &AppHandle) -> Result<(PathBuf, Library), String> {
    let root = store::require_root(app)?;
    let lib = store::load_library(&root)?;
    Ok((root, lib))
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Case-insensitive uniqueness (Windows folders are case-insensitive).
fn ensure_unique<'a>(
    existing: impl Iterator<Item = (&'a str, &'a str)>,
    except_id: Option<&str>,
    name: &str,
    kind: &str,
) -> Result<(), String> {
    let lower = name.to_lowercase();
    for (id, existing_name) in existing {
        if Some(id) != except_id && existing_name.to_lowercase() == lower {
            return Err(format!("A {kind} named '{existing_name}' already exists"));
        }
    }
    Ok(())
}

fn in_use_error(kind: &str, name: &str, count: usize) -> String {
    format!("Cannot delete {kind} '{name}': it is used by {count} item(s). Reassign or remove those first.")
}

fn rewrite(path: &mut String, pairs: &[(String, String)]) {
    for (old, new) in pairs {
        if let Some(updated) = fsops::rewrite_prefix(path, old, new) {
            *path = updated;
            return;
        }
    }
}

fn rewrite_paths(lib: &mut Library, pairs: &[(String, String)]) {
    for m in lib.models.iter_mut() {
        rewrite(&mut m.folder_path, pairs);
        for v in m.versions.iter_mut() {
            rewrite(&mut v.folder_path, pairs);
        }
    }
    for a in lib.assets.iter_mut() {
        rewrite(&mut a.folder_path, pairs);
        for v in a.versions.iter_mut() {
            rewrite(&mut v.folder_path, pairs);
        }
    }
}

/// Renames folders, rewrites stored paths, saves the library. If saving fails
/// the folder renames are undone, so disk and library never disagree.
pub(crate) fn save_with_moves(root: &Path, lib: &mut Library, pairs: &[(String, String)]) -> Result<(), String> {
    let moves: Vec<(PathBuf, PathBuf)> = pairs
        .iter()
        .map(|(from, to)| (root.join(from), root.join(to)))
        .collect();
    let applied = fsops::apply_moves(&moves)?;
    rewrite_paths(lib, pairs);
    if let Err(e) = store::save_library(root, lib) {
        applied.rollback();
        return Err(format!("Could not save the library; folder changes were undone: {e}"));
    }
    Ok(())
}

// ---------- sites (label only, no folders involved) ----------

#[tauri::command]
pub fn add_site(app: AppHandle, name: String) -> Result<Library, String> {
    let (root, mut lib) = open(&app)?;
    let name = fsops::validate_label("Site", &name)?;
    ensure_unique(lib.sites.iter().map(|s| (s.id.as_str(), s.name.as_str())), None, &name, "site")?;
    lib.sites.push(Site { id: new_id(), name });
    store::save_library(&root, &lib)?;
    Ok(lib)
}

#[tauri::command]
pub fn rename_site(app: AppHandle, id: String, name: String) -> Result<Library, String> {
    let (root, mut lib) = open(&app)?;
    let name = fsops::validate_label("Site", &name)?;
    ensure_unique(lib.sites.iter().map(|s| (s.id.as_str(), s.name.as_str())), Some(id.as_str()), &name, "site")?;
    let site = lib.sites.iter_mut().find(|s| s.id == id).ok_or("Site not found")?;
    site.name = name;
    store::save_library(&root, &lib)?;
    Ok(lib)
}

#[tauri::command]
pub fn delete_site(app: AppHandle, id: String) -> Result<Library, String> {
    let (root, mut lib) = open(&app)?;
    let idx = lib.sites.iter().position(|s| s.id == id).ok_or("Site not found")?;
    let count = lib.models.iter().filter(|m| m.site_id == id).count()
        + lib.assets.iter().filter(|a| a.site_id == id).count();
    if count > 0 {
        return Err(in_use_error("site", &lib.sites[idx].name, count));
    }
    lib.sites.remove(idx);
    store::save_library(&root, &lib)?;
    Ok(lib)
}

// ---------- categories (folder name: Assets/<Category>) ----------

#[tauri::command]
pub fn add_category(app: AppHandle, name: String) -> Result<Library, String> {
    let (root, mut lib) = open(&app)?;
    let name = fsops::validate_name("Category", &name)?;
    ensure_unique(lib.categories.iter().map(|c| (c.id.as_str(), c.name.as_str())), None, &name, "category")?;
    lib.categories.push(Category { id: new_id(), name });
    store::save_library(&root, &lib)?;
    Ok(lib)
}

#[tauri::command]
pub fn rename_category(app: AppHandle, id: String, name: String) -> Result<Library, String> {
    let (root, mut lib) = open(&app)?;
    let name = fsops::validate_name("Category", &name)?;
    ensure_unique(lib.categories.iter().map(|c| (c.id.as_str(), c.name.as_str())), Some(id.as_str()), &name, "category")?;
    let idx = lib.categories.iter().position(|c| c.id == id).ok_or("Category not found")?;
    let old = lib.categories[idx].name.clone();
    if old == name {
        return Ok(lib);
    }
    let pairs = vec![(format!("Assets/{old}"), format!("Assets/{name}"))];
    lib.categories[idx].name = name;
    save_with_moves(&root, &mut lib, &pairs)?;
    Ok(lib)
}

#[tauri::command]
pub fn delete_category(app: AppHandle, id: String) -> Result<Library, String> {
    let (root, mut lib) = open(&app)?;
    let idx = lib.categories.iter().position(|c| c.id == id).ok_or("Category not found")?;
    let count = lib.assets.iter().filter(|a| a.category_id == id).count();
    if count > 0 {
        return Err(in_use_error("category", &lib.categories[idx].name, count));
    }
    lib.categories.remove(idx);
    store::save_library(&root, &lib)?;
    Ok(lib)
}

// ---------- creators (folder name: Models/<Creator>, Assets/<Category>/<Creator>) ----------

/// Exact shape the frontend sends (see CreatorInput in src/types.ts).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatorInput {
    pub name: String,
    #[serde(default)]
    pub icon_image_path: Option<String>,
    #[serde(default)]
    pub profile_urls: Vec<String>,
}

fn clean_urls(urls: &[String]) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = vec![];
    for raw in urls {
        let url = raw.trim();
        if url.is_empty() {
            continue;
        }
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err(format!("'{url}' is not a valid link (it must start with http:// or https://)"));
        }
        if !out.iter().any(|u| u == url) {
            out.push(url.to_string());
        }
    }
    Ok(out)
}

fn creator_pairs(lib: &Library, old: &str, new: &str) -> Vec<(String, String)> {
    let mut pairs = vec![(format!("Models/{old}"), format!("Models/{new}"))];
    for c in &lib.categories {
        pairs.push((
            format!("Assets/{}/{old}", c.name),
            format!("Assets/{}/{new}", c.name),
        ));
    }
    pairs
}

#[tauri::command]
pub fn add_creator(app: AppHandle, input: CreatorInput) -> Result<Library, String> {
    let (root, mut lib) = open(&app)?;
    let name = fsops::validate_name("Creator", &input.name)?;
    let urls = clean_urls(&input.profile_urls)?;
    if let Some(icon) = &input.icon_image_path {
        images::check_ref(icon)?;
    }
    ensure_unique(lib.creators.iter().map(|c| (c.id.as_str(), c.name.as_str())), None, &name, "creator")?;
    lib.creators.push(Creator {
        id: new_id(),
        name,
        icon_image_path: input.icon_image_path,
        profile_urls: urls,
    });
    store::save_library(&root, &lib)?;
    Ok(lib)
}

#[tauri::command]
pub fn update_creator(app: AppHandle, id: String, input: CreatorInput) -> Result<Library, String> {
    let (root, mut lib) = open(&app)?;
    let name = fsops::validate_name("Creator", &input.name)?;
    let urls = clean_urls(&input.profile_urls)?;
    if let Some(icon) = &input.icon_image_path {
        images::check_ref(icon)?;
    }
    ensure_unique(lib.creators.iter().map(|c| (c.id.as_str(), c.name.as_str())), Some(id.as_str()), &name, "creator")?;

    let idx = lib.creators.iter().position(|c| c.id == id).ok_or("Creator not found")?;
    let old_name = lib.creators[idx].name.clone();
    let old_icon = lib.creators[idx].icon_image_path.clone();
    let pairs = if old_name != name {
        creator_pairs(&lib, &old_name, &name)
    } else {
        vec![]
    };

    {
        let c = &mut lib.creators[idx];
        c.name = name;
        c.profile_urls = urls;
        c.icon_image_path = input.icon_image_path.clone();
    }
    save_with_moves(&root, &mut lib, &pairs)?;

    // Only after a successful save: drop the icon that was replaced.
    if old_icon != input.icon_image_path {
        if let Some(old) = old_icon {
            images::delete_file(&root, &old);
        }
    }
    Ok(lib)
}

#[tauri::command]
pub fn delete_creator(app: AppHandle, id: String) -> Result<Library, String> {
    let (root, mut lib) = open(&app)?;
    let idx = lib.creators.iter().position(|c| c.id == id).ok_or("Creator not found")?;
    let count = lib.models.iter().filter(|m| m.creator_id == id).count()
        + lib.assets.iter().filter(|a| a.creator_id == id).count();
    if count > 0 {
        return Err(in_use_error("creator", &lib.creators[idx].name, count));
    }
    let removed = lib.creators.remove(idx);
    store::save_library(&root, &lib)?;
    if let Some(icon) = removed.icon_image_path {
        images::delete_file(&root, &icon);
    }
    Ok(lib)
}
