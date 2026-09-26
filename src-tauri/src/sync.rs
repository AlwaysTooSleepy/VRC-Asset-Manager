//! Keeps the library file and the folders on disk in agreement. The disk is
//! scanned for
//!     Models/<Creator>/<Name>_V_<version>
//!     Assets/<Category>/<Creator>/<Name>_V_<version>
//! and the library is reconciled with what is found:
//!   * folders the library doesn't know about are added (this is also how an
//!     existing library folder is picked up),
//!   * versions whose folder disappeared are flagged `missing`,
//!   * exactly one missing version + exactly one new folder for the same item
//!     is treated as a rename (V_1.0.0 -> V_1.0.3), keeping notes and dates.

use crate::{filing, models::*, store};
use std::{cmp::Ordering, collections::HashSet, fs, path::Path};

const UNKNOWN_SITE: &str = "Unknown";

struct DiskVersion {
    label: String,
    rel: String,
}

/// One model or asset found on disk, with the folders above it
/// (`dirs` = [creator] for models, [category, creator] for assets).
struct DiskItem {
    dirs: Vec<String>,
    name: String,
    versions: Vec<DiskVersion>,
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn subdirs(path: &Path) -> Vec<String> {
    let Ok(read) = fs::read_dir(path) else { return vec![] };
    let mut names: Vec<String> = read
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| !n.starts_with('.'))
        .collect();
    names.sort();
    names
}

/// "Chocolat_V_1.0.2" -> ("Chocolat", "1.0.2"). Splits at the last "_V_".
fn split_versioned(name: &str) -> Option<(String, String)> {
    let idx = name.to_ascii_lowercase().rfind("_v_")?;
    let (item, version) = (&name[..idx], &name[idx + 3..]);
    if item.trim().is_empty() || version.trim().is_empty() {
        return None;
    }
    Some((item.to_string(), version.to_string()))
}

fn walk(dir: &Path, rel_prefix: String, levels: usize, dirs: &mut Vec<String>, out: &mut Vec<DiskItem>) {
    if levels > 0 {
        for name in subdirs(dir) {
            dirs.push(name.clone());
            walk(&dir.join(&name), format!("{rel_prefix}/{name}"), levels - 1, dirs, out);
            dirs.pop();
        }
        return;
    }
    for entry in subdirs(dir) {
        let Some((item_name, label)) = split_versioned(&entry) else { continue };
        let version = DiskVersion { label, rel: format!("{rel_prefix}/{entry}") };
        let lower = item_name.to_lowercase();
        match out.iter_mut().find(|d| d.dirs == *dirs && d.name.to_lowercase() == lower) {
            Some(d) => d.versions.push(version),
            None => out.push(DiskItem { dirs: dirs.clone(), name: item_name, versions: vec![version] }),
        }
    }
}

/// Finds versioned folders below `base`, `levels` folders deep (creator, or category + creator).
fn discover(root: &Path, base: &str, levels: usize) -> Vec<DiskItem> {
    let mut out = vec![];
    walk(&root.join(base), base.to_string(), levels, &mut vec![], &mut out);
    out
}

/// "1.10.0" > "1.9.0": compare the digit runs numerically.
pub fn version_key(label: &str) -> Vec<u64> {
    label
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<u64>().unwrap_or(0))
        .collect()
}

fn cmp_versions(a: &Version, b: &Version) -> Ordering {
    version_key(&a.version_label)
        .cmp(&version_key(&b.version_label))
        .then_with(|| a.date_added.cmp(&b.date_added))
}

/// (folder of the current version, whether every version is missing).
/// "Current" is the highest version that is present.
fn current_state(versions: &[Version]) -> (String, bool) {
    let best = versions
        .iter()
        .filter(|v| !v.missing)
        .max_by(|a, b| cmp_versions(a, b))
        .or_else(|| versions.iter().max_by(|a, b| cmp_versions(a, b)));
    let path = best.map(|v| v.folder_path.clone()).unwrap_or_default();
    (path, !versions.is_empty() && versions.iter().all(|v| v.missing))
}

pub fn refresh_current(model: &mut Model) {
    let (path, missing) = current_state(&model.versions);
    model.folder_path = path;
    model.missing = missing;
}

pub fn refresh_asset_current(asset: &mut Asset) {
    let (path, missing) = current_state(&asset.versions);
    asset.folder_path = path;
    asset.missing = missing;
}

fn find_or_create_creator(lib: &mut Library, dir_name: &str) -> String {
    let lower = dir_name.to_lowercase();
    if let Some(c) = lib.creators.iter().find(|c| c.name.to_lowercase() == lower) {
        return c.id.clone();
    }
    let id = new_id();
    lib.creators.push(Creator {
        id: id.clone(),
        name: dir_name.to_string(),
        icon_image_path: None,
        profile_urls: vec![],
    });
    id
}

fn find_or_create_category(lib: &mut Library, dir_name: &str) -> String {
    let lower = dir_name.to_lowercase();
    if let Some(c) = lib.categories.iter().find(|c| c.name.to_lowercase() == lower) {
        return c.id.clone();
    }
    let id = new_id();
    lib.categories.push(Category { id: id.clone(), name: dir_name.to_string() });
    id
}

fn unknown_site_id(lib: &mut Library) -> String {
    if let Some(s) = lib.sites.iter().find(|s| s.name.eq_ignore_ascii_case(UNKNOWN_SITE)) {
        return s.id.clone();
    }
    let id = new_id();
    lib.sites.push(Site { id: id.clone(), name: UNKNOWN_SITE.to_string() });
    id
}

/// Reconciles one item's version list with the folders found on disk.
/// Returns true if versions were added or a rename was detected.
fn reconcile_versions(versions: &mut Vec<Version>, disk: &[DiskVersion]) -> bool {
    let stamp = now();
    let mut present = vec![false; versions.len()];
    let mut new_disk: Vec<&DiskVersion> = vec![];

    for d in disk {
        let found = versions.iter().position(|v| v.folder_path.to_lowercase() == d.rel.to_lowercase());
        match found {
            Some(i) => {
                present[i] = true;
                versions[i].folder_path = d.rel.clone(); // adopt on-disk casing
                versions[i].missing = false;
            }
            None => new_disk.push(d),
        }
    }

    let missing_idx: Vec<usize> = (0..versions.len()).filter(|i| !present[*i]).collect();

    if missing_idx.len() == 1 && new_disk.len() == 1 {
        // Unambiguous rename done outside the app.
        let v = &mut versions[missing_idx[0]];
        v.version_label = new_disk[0].label.clone();
        v.folder_path = new_disk[0].rel.clone();
        v.missing = false;
        return true;
    }

    for i in &missing_idx {
        versions[*i].missing = true;
    }
    let added = !new_disk.is_empty();
    for d in new_disk {
        versions.push(Version {
            version_label: d.label.clone(),
            folder_path: d.rel.clone(),
            date_added: stamp.clone(),
            notes: None,
            missing: false,
        });
    }
    added
}

fn reconcile_models(root: &Path, lib: &mut Library) {
    let mut touched: HashSet<String> = HashSet::new();

    for group in discover(root, "Models", 1) {
        let creator_id = find_or_create_creator(lib, &group.dirs[0]);
        let lower = group.name.to_lowercase();
        let existing = lib
            .models
            .iter()
            .position(|m| m.creator_id == creator_id && m.name.to_lowercase() == lower);
        let idx = match existing {
            Some(i) => i,
            None => {
                let site_id = unknown_site_id(lib);
                let stamp = now();
                lib.models.push(Model {
                    id: new_id(),
                    name: group.name.clone(),
                    creator_id,
                    site_id,
                    source_url: None,
                    folder_path: String::new(),
                    image_paths: vec![],
                    versions: vec![],
                    date_added: stamp.clone(),
                    date_updated: stamp,
                    missing: false,
                });
                lib.models.len() - 1
            }
        };
        touched.insert(lib.models[idx].id.clone());
        let m = &mut lib.models[idx];
        if reconcile_versions(&mut m.versions, &group.versions) {
            m.date_updated = now();
        }
        refresh_current(m);
    }

    // Models with nothing on disk any more: all their versions are missing.
    for m in lib.models.iter_mut() {
        if !touched.contains(&m.id) {
            for v in m.versions.iter_mut() {
                v.missing = true;
            }
            refresh_current(m);
        }
    }
}

/// Models an asset works with, judging by `For_<Model>` subfolders inside its versions.
fn detect_compat(root: &Path, lib: &Library, asset: &Asset) -> Vec<String> {
    let mut found: HashSet<String> = HashSet::new();
    for v in asset.versions.iter().filter(|v| !v.missing) {
        for name in subdirs(&root.join(&v.folder_path)) {
            if name.to_lowercase().starts_with("for_") {
                found.insert(name.to_lowercase());
            }
        }
    }
    if found.is_empty() {
        return vec![];
    }
    lib.models
        .iter()
        .filter(|m| found.contains(&filing::model_subfolder(lib, m).to_lowercase()))
        .map(|m| m.id.clone())
        .collect()
}

fn reconcile_assets(root: &Path, lib: &mut Library) {
    let mut touched: HashSet<String> = HashSet::new();

    for group in discover(root, "Assets", 2) {
        let category_id = find_or_create_category(lib, &group.dirs[0]);
        let creator_id = find_or_create_creator(lib, &group.dirs[1]);
        let lower = group.name.to_lowercase();
        let existing = lib.assets.iter().position(|a| {
            a.category_id == category_id && a.creator_id == creator_id && a.name.to_lowercase() == lower
        });
        let idx = match existing {
            Some(i) => i,
            None => {
                let site_id = unknown_site_id(lib);
                let stamp = now();
                lib.assets.push(Asset {
                    id: new_id(),
                    name: group.name.clone(),
                    creator_id,
                    site_id,
                    category_id,
                    source_url: None,
                    folder_path: String::new(),
                    image_paths: vec![],
                    versions: vec![],
                    compatible_model_ids: vec![],
                    compatible_with_all: false,
                    date_added: stamp.clone(),
                    date_updated: stamp,
                    missing: false,
                });
                lib.assets.len() - 1
            }
        };
        touched.insert(lib.assets[idx].id.clone());
        let a = &mut lib.assets[idx];
        if reconcile_versions(&mut a.versions, &group.versions) {
            a.date_updated = now();
        }
        refresh_asset_current(a);
    }

    for a in lib.assets.iter_mut() {
        if !touched.contains(&a.id) {
            for v in a.versions.iter_mut() {
                v.missing = true;
            }
            refresh_asset_current(a);
        }
    }

    // An asset with no compatibility chosen was found on disk (the UI never allows
    // saving one like that). Pre-select the models its `For_<Model>` folders point to.
    let libr: &Library = &*lib;
    let fills: Vec<(usize, Vec<String>)> = libr
        .assets
        .iter()
        .enumerate()
        .filter(|(_, a)| !a.compatible_with_all && a.compatible_model_ids.is_empty())
        .filter_map(|(i, a)| {
            let ids = detect_compat(root, libr, a);
            if ids.is_empty() { None } else { Some((i, ids)) }
        })
        .collect();
    for (i, ids) in fills {
        lib.assets[i].compatible_model_ids = ids;
    }
}

pub fn reconcile(root: &Path, lib: &mut Library) {
    reconcile_models(root, lib);
    reconcile_assets(root, lib);
}

/// Reconciles and writes the library only if something actually changed.
pub fn sync_and_save(root: &Path, lib: &mut Library) -> Result<(), String> {
    let before = serde_json::to_string(lib).map_err(|e| e.to_string())?;
    reconcile(root, lib);
    let after = serde_json::to_string(lib).map_err(|e| e.to_string())?;
    if before != after {
        store::save_library(root, lib)?;
    }
    Ok(())
}
