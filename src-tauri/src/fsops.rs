//! Filesystem safety helpers: name validation, stored-path rewriting, and
//! multi-folder renames that roll back if anything fails.

use std::{fs, path::PathBuf};

fn e2s<E: ToString>(e: E) -> String {
    e.to_string()
}

const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// For names that are only labels (Sites).
pub fn validate_label(kind: &str, raw: &str) -> Result<String, String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(format!("{kind} name cannot be empty"));
    }
    if name.chars().count() > 100 {
        return Err(format!("{kind} name is too long (max 100 characters)"));
    }
    Ok(name.to_string())
}

/// For names that become folder names (Creators, Categories).
pub fn validate_name(kind: &str, raw: &str) -> Result<String, String> {
    let name = validate_label(kind, raw)?;
    if let Some(c) = name.chars().find(|c| "\\/:*?\"<>|".contains(*c) || c.is_control()) {
        return Err(format!(
            "{kind} name cannot contain '{c}' because it is used as a folder name"
        ));
    }
    if name.ends_with('.') {
        return Err(format!("{kind} name cannot end with a period"));
    }
    let stem = name.split('.').next().unwrap_or("").to_uppercase();
    if RESERVED.contains(&stem.as_str()) {
        return Err(format!("'{name}' is a reserved name on Windows"));
    }
    Ok(name)
}

/// If `path` is `old` or inside `old`, returns it re-based onto `new`.
pub fn rewrite_prefix(path: &str, old: &str, new: &str) -> Option<String> {
    if path == old {
        return Some(new.to_string());
    }
    let rest = path.strip_prefix(old)?.strip_prefix('/')?;
    Some(format!("{new}/{rest}"))
}

/// Folder renames that have been applied and can be undone.
pub struct AppliedMoves {
    steps: Vec<(PathBuf, PathBuf)>,
}

impl AppliedMoves {
    pub fn rollback(&self) {
        for (from, to) in self.steps.iter().rev() {
            let _ = fs::rename(to, from);
        }
    }
}

fn same_ignoring_case(a: &PathBuf, b: &PathBuf) -> bool {
    a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
}

fn step(applied: &mut AppliedMoves, from: &PathBuf, to: &PathBuf) -> Result<(), String> {
    fs::rename(from, to).map_err(e2s)?;
    applied.steps.push((from.clone(), to.clone()));
    Ok(())
}

/// Validates every move first, then performs them. If any rename fails, all
/// earlier ones are undone. Moves whose source folder does not exist yet are
/// skipped (nothing has been filed there).
pub fn apply_moves(moves: &[(PathBuf, PathBuf)]) -> Result<AppliedMoves, String> {
    for (from, to) in moves {
        if from.exists() && to.exists() && !same_ignoring_case(from, to) {
            return Err(format!(
                "Cannot rename: '{}' already exists on disk",
                to.display()
            ));
        }
    }

    let mut applied = AppliedMoves { steps: vec![] };
    for (from, to) in moves {
        if !from.exists() {
            continue;
        }
        if let Some(parent) = to.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                applied.rollback();
                return Err(format!("Rename failed and was rolled back: {e}"));
            }
        }
        let result = if same_ignoring_case(from, to) {
            // Case-only rename (Komado -> komado): go through a temp name so it
            // also works on case-insensitive filesystems.
            let tmp = from.with_file_name(format!(".rename_tmp_{}", uuid::Uuid::new_v4()));
            step(&mut applied, from, &tmp).and_then(|_| step(&mut applied, &tmp, to))
        } else {
            step(&mut applied, from, to)
        };
        if let Err(e) = result {
            applied.rollback();
            return Err(format!(
                "Rename failed and was rolled back (is a folder open in another program?): {e}"
            ));
        }
    }
    Ok(applied)
}
