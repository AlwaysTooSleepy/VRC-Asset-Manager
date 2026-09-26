//! Central image store: everything lives in AppData/Images, named by a hash
//! of its own bytes rather than a random ID. That makes the store
//! content-addressed: importing the same image twice — the same URL
//! fetched again, or the same photo reused across two products — always
//! resolves to the same file, so the library never ends up with duplicate
//! copies of identical bytes. Items refer to images by relative path, and
//! nothing about sharing one path across several items needs special
//! handling: `is_referenced` (below) already checks every creator/model/
//! asset, so a shared image is only ever deleted once *nothing* points at
//! it any more.

use crate::{models::Library, store};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::AppHandle;

const IMAGES_DIR: &str = "AppData/Images";
const THUMBS_DIR: &str = "AppData/Images/Thumbs";
const EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp"];

/// Longest edge of a generated thumbnail, in pixels. Big enough to still
/// look sharp in a card grid, small enough to be trivial to decode.
const THUMB_MAX_DIM: u32 = 384;

/// How much of the SHA-256 digest to use for the filename. 20 hex chars is
/// 80 bits — astronomically collision-safe for a personal image library
/// (the birthday bound is far past any realistic library size) while
/// staying short enough to be readable in a file browser.
const HASH_HEX_LEN: usize = 20;

/// Only paths inside the central image folder are ever accepted or deleted.
pub fn check_ref(rel: &str) -> Result<(), String> {
    let ok = rel.starts_with("AppData/Images/") && !rel.contains("..") && !rel.contains('\\');
    if ok {
        Ok(())
    } else {
        Err(format!("'{rel}' is not a valid library image path"))
    }
}

pub fn delete_file(root: &Path, rel: &str) {
    if check_ref(rel).is_ok() {
        let _ = fs::remove_file(root.join(rel));
        // The thumbnail is a derived artifact of this file, not something
        // anything else references directly — clean it up alongside the
        // original so it doesn't linger as dead weight.
        if let Some(thumb_rel) = thumb_rel_path(rel) {
            let _ = fs::remove_file(root.join(thumb_rel));
        }
    }
}

fn is_referenced(lib: &Library, rel: &str) -> bool {
    lib.creators.iter().any(|c| c.icon_image_path.as_deref() == Some(rel))
        || lib.models.iter().any(|m| m.image_paths.iter().any(|p| p == rel))
        || lib.assets.iter().any(|a| a.image_paths.iter().any(|p| p == rel))
}

/// Content-addressed relative path for some image bytes: a truncated
/// SHA-256 hex digest of `bytes`, plus `ext`. Two calls with the same
/// bytes always produce the same path, regardless of where the bytes came
/// from — that's the whole dedup mechanism.
fn content_rel_path(bytes: &[u8], ext: &str) -> String {
    let digest = Sha256::digest(bytes);
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("{IMAGES_DIR}/{}.{ext}", &hex[..HASH_HEX_LEN])
}

/// Writes `bytes` to `rel` under `root`, unless a file is already there —
/// same content hashes to the same path, so an existing file already *is*
/// these exact bytes and there's nothing to do.
fn write_deduped(root: &Path, rel: &str, bytes: &[u8]) -> Result<(), String> {
    let dest = root.join(rel);
    if dest.is_file() {
        return Ok(());
    }
    fs::create_dir_all(dest.parent().unwrap_or(root)).map_err(|e| e.to_string())?;
    fs::write(dest, bytes).map_err(|e| e.to_string())
}

/// The thumbnail path for an original image at `original_rel` — same
/// content hash, moved into the `Thumbs` subfolder, always `.png`
/// regardless of the original's own format (see `generate_thumbnail`).
/// Returns `None` for anything not directly under `AppData/Images/`
/// (including a thumbnail's own path — a derived file doesn't get a
/// thumbnail of itself).
fn thumb_rel_path(original_rel: &str) -> Option<String> {
    let filename = original_rel.strip_prefix(&format!("{IMAGES_DIR}/"))?;
    if filename.starts_with("Thumbs/") {
        return None;
    }
    let stem = filename.split('.').next()?;
    if stem.is_empty() {
        return None;
    }
    Some(format!("{THUMBS_DIR}/{stem}.png"))
}

/// Generates and stores a small cached thumbnail for `bytes` (the same
/// bytes just written to `original_rel` by the caller), so grid/card
/// views can load a few KB instead of decoding the full original on every
/// render. Always encoded as PNG regardless of the source format — one
/// simple code path, and safe for images that rely on transparency (e.g.
/// creator icons). Best-effort: if anything here fails (corrupt bytes, a
/// variant the decoder doesn't support), the import itself still
/// succeeds — that image just falls back to loading its full original
/// everywhere it's shown (see the frontend's `thumbUrl` helper).
fn generate_thumbnail(root: &Path, bytes: &[u8], original_rel: &str) {
    let Some(thumb_rel) = thumb_rel_path(original_rel) else { return };
    if root.join(&thumb_rel).is_file() {
        return; // this exact content already has a cached thumbnail
    }
    let Ok(img) = image::load_from_memory(bytes) else { return };
    let thumb = img.resize(THUMB_MAX_DIM, THUMB_MAX_DIM, image::imageops::FilterType::Triangle);
    let mut png_bytes: Vec<u8> = Vec::new();
    if thumb
        .write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .is_err()
    {
        return;
    }
    let _ = write_deduped(root, &thumb_rel, &png_bytes);
}

/// Copies an image chosen by the user into the library. The original file is
/// left untouched. Returns the new relative path.
#[tauri::command]
pub fn import_image(app: AppHandle, source_path: String) -> Result<String, String> {
    let root = store::require_root(&app)?;
    let src = PathBuf::from(&source_path);
    if !src.is_file() {
        return Err(format!("'{source_path}' is not a file"));
    }
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    if !EXTENSIONS.contains(&ext.as_str()) {
        return Err(format!("Unsupported image type '.{ext}' (use png, jpg, gif, webp or bmp)"));
    }
    let bytes = fs::read(&src).map_err(|e| e.to_string())?;
    let rel = content_rel_path(&bytes, &ext);
    write_deduped(&root, &rel, &bytes)?;
    generate_thumbnail(&root, &bytes, &rel);
    Ok(rel)
}

/// Cleans up an imported image that ended up not being used (form cancelled).
/// Does nothing if anything in the library still references it.
#[tauri::command]
pub fn remove_image_if_unused(app: AppHandle, path: String) -> Result<(), String> {
    check_ref(&path)?;
    let root = store::require_root(&app)?;
    let lib = store::load_library(&root)?;
    if !is_referenced(&lib, &path) {
        delete_file(&root, &path);
    }
    Ok(())
}

/// Deletes any of `candidates` that nothing in `lib` references any more.
pub fn prune_unused(root: &Path, lib: &Library, candidates: &[String]) {
    for rel in candidates {
        if !is_referenced(lib, rel) {
            delete_file(root, rel);
        }
    }
}

const MAX_DOWNLOAD: usize = 25 * 1024 * 1024;

/// Identifies the image type from its first bytes (more reliable than headers or URLs).
fn sniff_ext(b: &[u8]) -> Option<&'static str> {
    if b.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        Some("png")
    } else if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if b.starts_with(b"GIF8") {
        Some("gif")
    } else if b.len() >= 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        Some("webp")
    } else if b.starts_with(b"BM") {
        Some("bmp")
    } else {
        None
    }
}

/// Downloads an image from a web address into the library. Returns the relative path.
#[tauri::command]
pub async fn import_image_from_url(app: AppHandle, url: String) -> Result<String, String> {
    let root = store::require_root(&app)?;
    let url = url.trim().to_string();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(format!("'{url}' is not a valid link (it must start with http:// or https://)"));
    }
    let parsed = reqwest::Url::parse(&url).map_err(|e| format!("'{url}' is not a valid link: {e}"))?;
    let referer = format!("{}/", parsed.origin().ascii_serialization());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (VRChatAssetManager)")
        .build()
        .map_err(|e| e.to_string())?;
    let mut resp = client
        .get(parsed)
        .header(reqwest::header::REFERER, referer)
        .send()
        .await
        .map_err(|e| format!("Could not download '{url}': {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Could not download '{url}': the server replied {}", resp.status()));
    }

    let mut bytes: Vec<u8> = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(|e| format!("Download of '{url}' failed: {e}"))? {
        bytes.extend_from_slice(&chunk);
        if bytes.len() > MAX_DOWNLOAD {
            return Err(format!("'{url}' is larger than 25 MB"));
        }
    }
    let ext = sniff_ext(&bytes)
        .ok_or_else(|| format!("'{url}' is not a supported image (png, jpg, gif, webp or bmp)"))?;

    let rel = content_rel_path(&bytes, ext);
    write_deduped(&root, &rel, &bytes)?;
    generate_thumbnail(&root, &bytes, &rel);
    Ok(rel)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real, decodable PNG with actual pixel variation (not a blank/solid
    /// color, which would compress away to near-nothing and make size
    /// comparisons in the thumbnail tests meaningless).
    fn sample_png(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbaImage::from_fn(w, h, |x, y| {
            image::Rgba([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8, 255])
        });
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        bytes
    }

    #[test]
    fn content_rel_path_is_stable_and_content_dependent() {
        let a = content_rel_path(b"same bytes", "png");
        let b = content_rel_path(b"same bytes", "png");
        let c = content_rel_path(b"different bytes", "png");
        assert_eq!(a, b, "identical bytes must hash to the identical path");
        assert_ne!(a, c, "different bytes must not collide");
        assert!(a.starts_with("AppData/Images/") && a.ends_with(".png"));
    }

    #[test]
    fn content_rel_path_changes_with_extension_even_for_same_bytes() {
        // Same bytes imported under two different sniffed/declared
        // extensions must not collide on disk — the extension is part of
        // the path, not just decoration.
        let png = content_rel_path(b"same bytes", "png");
        let jpg = content_rel_path(b"same bytes", "jpg");
        assert_ne!(png, jpg);
    }

    #[test]
    fn write_deduped_skips_existing_file_without_touching_it() {
        let tmp = std::env::temp_dir().join(format!("vcam-image-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(tmp.join(IMAGES_DIR)).unwrap();
        let rel = content_rel_path(b"hello", "png");

        write_deduped(&tmp, &rel, b"hello").unwrap();
        let first_write = fs::metadata(tmp.join(&rel)).unwrap().modified().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));
        write_deduped(&tmp, &rel, b"hello").unwrap();
        let second_write = fs::metadata(tmp.join(&rel)).unwrap().modified().unwrap();

        assert_eq!(first_write, second_write, "second call must not rewrite the file");
        assert_eq!(fs::read(tmp.join(&rel)).unwrap(), b"hello");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn thumb_rel_path_derives_from_the_same_hash_stem() {
        assert_eq!(
            thumb_rel_path("AppData/Images/abc123.png").as_deref(),
            Some("AppData/Images/Thumbs/abc123.png")
        );
        assert_eq!(
            thumb_rel_path("AppData/Images/abc123.jpeg").as_deref(),
            Some("AppData/Images/Thumbs/abc123.png"),
            "a thumbnail is always .png, regardless of the original's own format"
        );
    }

    #[test]
    fn thumb_rel_path_refuses_paths_outside_the_image_dir_or_a_thumbnail_of_a_thumbnail() {
        assert!(thumb_rel_path("AppData/Images/Thumbs/abc123.png").is_none());
        assert!(thumb_rel_path("AppData/Other/abc123.png").is_none());
    }

    #[test]
    fn generate_thumbnail_writes_a_smaller_readable_png_within_the_size_cap() {
        let original = sample_png(800, 600);
        let tmp = std::env::temp_dir().join(format!("vcam-thumb-test-{}", uuid::Uuid::new_v4()));
        let rel = content_rel_path(&original, "png");
        write_deduped(&tmp, &rel, &original).unwrap();

        generate_thumbnail(&tmp, &original, &rel);

        let thumb_path = tmp.join(thumb_rel_path(&rel).unwrap());
        assert!(thumb_path.is_file(), "thumbnail should have been written");

        let thumb_img = image::open(&thumb_path).unwrap();
        assert!(thumb_img.width() <= THUMB_MAX_DIM && thumb_img.height() <= THUMB_MAX_DIM);
        // 800x600 -> at most 384 long edge keeps the 4:3 ratio, so 384x288.
        assert_eq!((thumb_img.width(), thumb_img.height()), (384, 288));
        assert!(
            thumb_path.metadata().unwrap().len() < original.len() as u64,
            "a downsized thumbnail should be smaller than the original"
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn generate_thumbnail_skips_regeneration_when_already_present() {
        let original = sample_png(800, 600);
        let tmp = std::env::temp_dir().join(format!("vcam-thumb-test-{}", uuid::Uuid::new_v4()));
        let rel = content_rel_path(&original, "png");
        write_deduped(&tmp, &rel, &original).unwrap();
        generate_thumbnail(&tmp, &original, &rel);

        let thumb_path = tmp.join(thumb_rel_path(&rel).unwrap());
        let first_write = fs::metadata(&thumb_path).unwrap().modified().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));
        generate_thumbnail(&tmp, &original, &rel);
        let second_write = fs::metadata(&thumb_path).unwrap().modified().unwrap();

        assert_eq!(first_write, second_write, "an existing thumbnail must not be regenerated");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn delete_file_also_removes_the_thumbnail() {
        let original = sample_png(800, 600);
        let tmp = std::env::temp_dir().join(format!("vcam-thumb-test-{}", uuid::Uuid::new_v4()));
        let rel = content_rel_path(&original, "png");
        write_deduped(&tmp, &rel, &original).unwrap();
        generate_thumbnail(&tmp, &original, &rel);
        let thumb_path = tmp.join(thumb_rel_path(&rel).unwrap());
        assert!(tmp.join(&rel).is_file());
        assert!(thumb_path.is_file());

        delete_file(&tmp, &rel);

        assert!(!tmp.join(&rel).is_file(), "original should be gone");
        assert!(!thumb_path.is_file(), "thumbnail should be gone too");

        let _ = fs::remove_dir_all(&tmp);
    }
}

