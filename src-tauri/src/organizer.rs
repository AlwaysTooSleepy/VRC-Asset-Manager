//! Moves files and folders into the library. The app MOVES (never copies);
//! a copy-then-delete fallback is used only when source and destination are on
//! different drives, where a plain rename is impossible.

use std::{fs, io, path::Path};

// OS error code for "cannot rename across devices".
#[cfg(windows)]
const CROSS_DEVICE: i32 = 17; // ERROR_NOT_SAME_DEVICE
#[cfg(not(windows))]
const CROSS_DEVICE: i32 = 18; // EXDEV

pub struct MoveOutcome {
    /// Set when the item arrived safely but the original could not be fully removed.
    pub warning: Option<String>,
}

fn copy_dir(src: &Path, dest: &Path) -> io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Moves a file or folder so that it becomes `dest` (the item itself, with no
/// extra nesting). Fails without touching anything if `dest` already exists.
pub fn move_path(src: &Path, dest: &Path) -> Result<MoveOutcome, String> {
    if dest.exists() {
        return Err(format!("'{}' already exists", dest.display()));
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    match fs::rename(src, dest) {
        Ok(()) => Ok(MoveOutcome { warning: None }),
        Err(e) if e.raw_os_error() == Some(CROSS_DEVICE) => {
            let is_dir = src.is_dir();
            let copied = if is_dir { copy_dir(src, dest) } else { fs::copy(src, dest).map(|_| ()) };
            if let Err(err) = copied {
                let _ = if is_dir { fs::remove_dir_all(dest) } else { fs::remove_file(dest) };
                return Err(format!(
                    "Copying '{}' into the library failed; the original is untouched: {err}",
                    src.display()
                ));
            }
            let removed = if is_dir { fs::remove_dir_all(src) } else { fs::remove_file(src) };
            match removed {
                Ok(()) => Ok(MoveOutcome { warning: None }),
                Err(err) => Ok(MoveOutcome {
                    warning: Some(format!(
                        "'{}' was copied into your library, but the original could not be fully removed: {err}",
                        src.display()
                    )),
                }),
            }
        }
        Err(e) => Err(format!(
            "Could not move '{}' (is it open in another program?): {e}",
            src.display()
        )),
    }
}
