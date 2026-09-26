//! Filing a version's files into its folder.
//!
//! Layout of an asset version folder (Assets/<Category>/<Creator>/<Name>_V_<version>/):
//!   * one source folder, no target      -> that folder simply becomes the version folder
//!   * sources with no target            -> their contents go straight into the version folder
//!   * sources with targets              -> one subfolder per target, the only place subfolders
//!                                          appear:  For_<Model>/  and  Shared/
//! Everything is planned and validated first (nothing moves if anything is wrong),
//! then moved; if any step fails, every earlier move is undone.

use crate::{models::*, organizer};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

fn e2s<E: ToString>(e: E) -> String {
    e.to_string()
}

// ---------- subfolder names ----------

/// Subfolder holding a model's files inside an asset version: `For_<Model>`.
/// If another model has the same name, the creator is appended so they stay apart.
pub fn model_subfolder(lib: &Library, model: &Model) -> String {
    let lower = model.name.to_lowercase();
    let clash = lib
        .models
        .iter()
        .any(|m| m.id != model.id && m.name.to_lowercase() == lower);
    if clash {
        let creator = lib
            .creators
            .iter()
            .find(|c| c.id == model.creator_id)
            .map(|c| c.name.as_str())
            .unwrap_or("Unknown");
        format!("For_{}_({})", model.name, creator)
    } else {
        format!("For_{}", model.name)
    }
}

/// (old, new) subfolder names for every model whose subfolder name differs
/// between two library states. Used to rename `For_<Model>` folders when a model
/// is renamed (or starts/stops clashing with another model's name).
pub fn subfolder_renames(before: &Library, after: &Library) -> Vec<(String, String)> {
    after
        .models
        .iter()
        .filter_map(|m| {
            let old = before.models.iter().find(|b| b.id == m.id)?;
            let (o, n) = (model_subfolder(before, old), model_subfolder(after, m));
            if o != n { Some((o, n)) } else { None }
        })
        .collect()
}

// ---------- what was filed (so it can be undone) ----------

#[derive(Default)]
pub struct Filed {
    /// (original location, location inside the library); each is undone by moving back.
    moves: Vec<(PathBuf, PathBuf)>,
    /// Directories created for this version, removed on undo if they're empty.
    dirs: Vec<PathBuf>,
    pub warning: Option<String>,
}

fn rollback(filed: &Filed) -> Vec<String> {
    let mut failures = vec![];
    for (orig, now) in filed.moves.iter().rev() {
        if let Err(e) = organizer::move_path(now, orig) {
            failures.push(format!("{} ({e})", now.display()));
        }
    }
    for dir in filed.dirs.iter().rev() {
        let _ = fs::remove_dir(dir); // only removes it if empty
    }
    failures
}

/// Reverts everything in `filed` after the library failed to save.
pub fn undo(filed: &Filed, save_err: &str) -> String {
    let failures = rollback(filed);
    if !failures.is_empty() {
        return format!(
            "Could not save the library ({save_err}) and some files could not be moved back: {}. They are still inside your library.",
            failures.join(", ")
        );
    }
    if filed.moves.is_empty() {
        format!("Could not save the library: {save_err}")
    } else {
        format!("Could not save the library; the files were moved back: {save_err}")
    }
}

// ---------- validating sources ----------

/// A source as sent by the frontend (see SourceInput in src/types.ts).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInput {
    pub path: String,
    /// "none", "shared" or "model".
    pub target: String,
    #[serde(default)]
    pub model_id: Option<String>,
}

/// Returns the canonical path after checking it is neither inside the library nor an
/// ancestor of it (moving a folder into itself is impossible).
fn check_outside_library(root_c: &Path, raw: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(raw.trim());
    if !path.exists() {
        return Err(format!("'{raw}' does not exist"));
    }
    let canon = path.canonicalize().map_err(e2s)?;
    if root_c.starts_with(&canon) {
        return Err(format!("'{raw}' contains your library. Choose only the files that belong to this item."));
    }
    if canon.starts_with(root_c) {
        return Err(format!("'{raw}' is already inside your library."));
    }
    Ok(canon)
}

/// Single-folder filing used for models: the chosen folder becomes the version
/// folder, or an empty one is created when nothing is chosen.
pub fn file_single(root: &Path, dest: &Path, source_folder: Option<&str>) -> Result<Filed, String> {
    match source_folder.map(str::trim).filter(|s| !s.is_empty()) {
        Some(folder) => {
            let root_c = root.canonicalize().map_err(e2s)?;
            check_outside_library(&root_c, folder)?;
            let src = PathBuf::from(folder);
            if !src.is_dir() {
                return Err(format!("'{folder}' is not an existing folder"));
            }
            let outcome = organizer::move_path(&src, dest)?;
            Ok(Filed { moves: vec![(src, dest.to_path_buf())], dirs: vec![], warning: outcome.warning })
        }
        None => {
            fs::create_dir_all(dest).map_err(e2s)?;
            Ok(Filed { moves: vec![], dirs: vec![dest.to_path_buf()], warning: None })
        }
    }
}

// ---------- planning asset sources ----------

pub struct SourcePlan {
    /// Fast path: this single folder is renamed into place as the version folder.
    whole_folder: Option<PathBuf>,
    /// (item to move, subfolder of the version folder; None = straight into it)
    entries: Vec<(PathBuf, Option<String>)>,
    /// Source folders whose contents are being merged; removed afterwards if empty.
    containers: Vec<PathBuf>,
    /// Subfolders that will be created, in order.
    pub subfolders: Vec<String>,
}

struct Resolved {
    path: PathBuf,
    canon: PathBuf,
    sub: Option<String>,
}

/// Validates the sources and works out exactly what would be moved where, without
/// touching anything. Shared by the live preview and the real filing.
pub fn plan_sources(root: &Path, lib: &Library, asset: &Asset, sources: &[SourceInput]) -> Result<SourcePlan, String> {
    if sources.is_empty() {
        return Ok(SourcePlan { whole_folder: None, entries: vec![], containers: vec![], subfolders: vec![] });
    }
    let root_c = root.canonicalize().map_err(e2s)?;

    let mut resolved: Vec<Resolved> = vec![];
    for s in sources {
        let canon = check_outside_library(&root_c, &s.path)?;
        let sub = match s.target.as_str() {
            "none" => None,
            "shared" => Some("Shared".to_string()),
            "model" => {
                let id = s.model_id.as_deref().ok_or("Choose a model for every source that targets one")?;
                let model = lib
                    .models
                    .iter()
                    .find(|m| m.id == id)
                    .ok_or("A selected target model no longer exists")?;
                if !asset.compatible_with_all && !asset.compatible_model_ids.iter().any(|x| x == id) {
                    return Err(format!(
                        "'{}' isn't one of this asset's compatible models. Edit the asset to add it first.",
                        model.name
                    ));
                }
                Some(model_subfolder(lib, model))
            }
            other => return Err(format!("Unknown target '{other}'")),
        };
        resolved.push(Resolved { path: PathBuf::from(s.path.trim()), canon, sub });
    }

    let tagged = resolved.iter().filter(|r| r.sub.is_some()).count();
    if tagged != 0 && tagged != resolved.len() {
        return Err("Give every source a target, or leave them all as \"None\" to file them straight into the version folder.".to_string());
    }

    for (i, a) in resolved.iter().enumerate() {
        for b in resolved.iter().skip(i + 1) {
            if a.canon == b.canon {
                return Err(format!("'{}' is attached twice", a.path.display()));
            }
            if a.canon.starts_with(&b.canon) || b.canon.starts_with(&a.canon) {
                return Err(format!(
                    "'{}' and '{}' overlap: one is inside the other. Attach only one of them.",
                    a.path.display(),
                    b.path.display()
                ));
            }
        }
    }

    // One folder and no target: it simply becomes the version folder.
    if resolved.len() == 1 && resolved[0].sub.is_none() && resolved[0].path.is_dir() {
        return Ok(SourcePlan {
            whole_folder: Some(resolved[0].path.clone()),
            entries: vec![],
            containers: vec![],
            subfolders: vec![],
        });
    }

    let mut entries: Vec<(PathBuf, Option<String>)> = vec![];
    let mut containers: Vec<PathBuf> = vec![];
    let mut subfolders: Vec<String> = vec![];
    for r in &resolved {
        if let Some(sub) = &r.sub {
            if !subfolders.contains(sub) {
                subfolders.push(sub.clone());
            }
        }
        if r.path.is_dir() {
            containers.push(r.path.clone());
            for child in fs::read_dir(&r.path).map_err(e2s)? {
                entries.push((child.map_err(e2s)?.path(), r.sub.clone()));
            }
        } else {
            entries.push((r.path.clone(), r.sub.clone()));
        }
    }

    // Nothing may overwrite anything else (compared case-insensitively, as on Windows).
    let mut seen: HashMap<(String, String), PathBuf> = HashMap::new();
    for (path, sub) in &entries {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let key = (sub.clone().unwrap_or_default().to_lowercase(), name.to_lowercase());
        if seen.insert(key, path.clone()).is_some() {
            let place = sub.as_ref().map(|s| format!(" in {s}")).unwrap_or_default();
            return Err(format!("Two of your sources both contain '{name}'{place}; they would overwrite each other."));
        }
    }

    Ok(SourcePlan { whole_folder: None, entries, containers, subfolders })
}

/// Carries out a plan, creating `dest` (which must not exist yet). If any step
/// fails, everything already moved is put back.
pub fn file_sources(dest: &Path, plan: &SourcePlan) -> Result<Filed, String> {
    if let Some(src) = &plan.whole_folder {
        let outcome = organizer::move_path(src, dest)?;
        return Ok(Filed { moves: vec![(src.clone(), dest.to_path_buf())], dirs: vec![], warning: outcome.warning });
    }

    let mut filed = Filed::default();
    let dirs = std::iter::once(dest.to_path_buf()).chain(plan.subfolders.iter().map(|s| dest.join(s)));
    for dir in dirs {
        if let Err(e) = fs::create_dir_all(&dir) {
            rollback(&filed);
            return Err(e2s(e));
        }
        filed.dirs.push(dir);
    }

    let mut warnings: Vec<String> = vec![];
    for (src, sub) in &plan.entries {
        let Some(name) = src.file_name() else { continue };
        let base = match sub {
            Some(s) => dest.join(s),
            None => dest.to_path_buf(),
        };
        let target = base.join(name);
        match organizer::move_path(src, &target) {
            Ok(outcome) => {
                filed.moves.push((src.clone(), target));
                if let Some(w) = outcome.warning {
                    warnings.push(w);
                }
            }
            Err(e) => {
                rollback(&filed);
                return Err(format!("Filing failed and was rolled back: {e}"));
            }
        }
    }

    for container in &plan.containers {
        let _ = fs::remove_dir(container); // tidy up the now-empty source folder
    }
    if !warnings.is_empty() {
        filed.warning = Some(warnings.join("\n"));
    }
    Ok(filed)
}
