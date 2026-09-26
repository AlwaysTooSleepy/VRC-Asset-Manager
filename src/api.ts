// The only place the frontend talks to the backend. Every Rust command gets
// one typed wrapper here, so payload shapes are defined in a single spot.

import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { confirm, open } from "@tauri-apps/plugin-dialog";
import type {
  AddAssetResult,
  AddAssetVersionInput,
  AddModelResult,
  AddVersionInput,
  AddVersionResult,
  AssetInput,
  AssetVersionPreview,
  CreatorInput,
  Library,
  ModelInput,
  ScrapedMeta,
  SourceInput,
  UpdateAssetVersionInput,
  UpdateVersionInput,
} from "./types";

const IMAGE_EXTENSIONS = ["png", "jpg", "jpeg", "gif", "webp", "bmp"];

/** Library-relative path -> absolute path. */
export const resolvePath = (root: string, rel: string) =>
  `${root.replace(/[\\/]+$/, "")}/${rel}`;

/** URL the webview can use in <img src> for a library-relative image path. */
export const imageUrl = (root: string, rel: string) =>
  convertFileSrc(resolvePath(root, rel));

/**
 * Library-relative path of an image -> its cached thumbnail's relative
 * path. Mirrors the backend's `thumb_rel_path` in images.rs exactly (same
 * folder swap, same hash-stem, always `.png`) — if that naming ever
 * changes on one side, it must change on the other. Returns null for
 * anything not under AppData/Images (nothing to derive a thumbnail path
 * from).
 */
export const thumbRelPath = (rel: string): string | null => {
  const prefix = "AppData/Images/";
  if (!rel.startsWith(prefix)) return null;
  const stem = rel.slice(prefix.length).split(".")[0];
  return stem ? `${prefix}Thumbs/${stem}.png` : null;
};

/**
 * URL for a cached thumbnail. The backend generates one for every image at
 * import time, but it can be missing for images imported before this
 * existed (or if generation failed for that file) — callers should render
 * this with an onError handler that falls back to `imageUrl`, since that's
 * the only reliable way to detect "the file isn't actually there" from the
 * webview.
 */
export const thumbUrl = (root: string, rel: string) => {
  const thumbRel = thumbRelPath(rel);
  return thumbRel ? imageUrl(root, thumbRel) : imageUrl(root, rel);
};

/** Library-relative path -> absolute path shown to the user, with their OS separators. */
export const displayPath = (root: string, rel: string) => {
  const sep = root.includes("\\") ? "\\" : "/";
  return root.replace(/[\\/]+$/, "") + sep + rel.split("/").join(sep);
};

export const api = {
  getLibraryRoot: () => invoke<string | null>("get_library_root"),
  setLibraryRoot: (root: string) => invoke<Library>("set_library_root", { root }),
  getLibrary: () => invoke<Library>("get_library"),
  /** Compares the library with the folders on disk and returns the reconciled library. */
  syncLibrary: () => invoke<Library>("sync_library"),

  // Sites
  addSite: (name: string) => invoke<Library>("add_site", { name }),
  renameSite: (id: string, name: string) => invoke<Library>("rename_site", { id, name }),
  deleteSite: (id: string) => invoke<Library>("delete_site", { id }),

  // Categories (renaming also renames folders on disk)
  addCategory: (name: string) => invoke<Library>("add_category", { name }),
  renameCategory: (id: string, name: string) => invoke<Library>("rename_category", { id, name }),
  deleteCategory: (id: string) => invoke<Library>("delete_category", { id }),

  // Creators (renaming also renames folders on disk)
  addCreator: (input: CreatorInput) => invoke<Library>("add_creator", { input }),
  updateCreator: (id: string, input: CreatorInput) =>
    invoke<Library>("update_creator", { id, input }),
  deleteCreator: (id: string) => invoke<Library>("delete_creator", { id }),

  // Images
  importImage: (sourcePath: string) => invoke<string>("import_image", { sourcePath }),
  importImageFromUrl: (url: string) => invoke<string>("import_image_from_url", { url }),
  removeImageIfUnused: (path: string) => invoke<void>("remove_image_if_unused", { path }),

  // Operating system
  openFolder: (path: string) => invoke<void>("open_folder", { path }),
  openUrl: (url: string) => invoke<void>("open_url", { url }),

  // Models (renaming or changing the creator also moves version folders)
  addModel: (input: ModelInput) => invoke<AddModelResult>("add_model", { input }),
  updateModel: (id: string, input: ModelInput) => invoke<Library>("update_model", { id, input }),
  removeModel: (id: string) => invoke<Library>("remove_model", { id }),

  // Versions
  previewVersionPath: (modelId: string, version: string) =>
    invoke<string>("preview_version_path", { modelId, version }),
  addVersion: (input: AddVersionInput) => invoke<AddVersionResult>("add_version", { input }),
  updateVersion: (input: UpdateVersionInput) => invoke<Library>("update_version", { input }),
  removeVersion: (modelId: string, folderPath: string) =>
    invoke<Library>("remove_version", { modelId, folderPath }),

  // Assets (renaming or changing creator/category also moves version folders)
  addAsset: (input: AssetInput) => invoke<AddAssetResult>("add_asset", { input }),
  updateAsset: (id: string, input: AssetInput) => invoke<Library>("update_asset", { id, input }),
  removeAsset: (id: string) => invoke<Library>("remove_asset", { id }),
  /** Dry run: validates the label and every source, and lists the subfolders that would be created. */
  previewAssetVersion: (assetId: string, version: string, sources: SourceInput[]) =>
    invoke<AssetVersionPreview>("preview_asset_version", { assetId, version, sources }),
  addAssetVersion: (input: AddAssetVersionInput) =>
    invoke<AddVersionResult>("add_asset_version", { input }),
  updateAssetVersion: (input: UpdateAssetVersionInput) =>
    invoke<Library>("update_asset_version", { input }),
  removeAssetVersion: (assetId: string, folderPath: string) =>
    invoke<Library>("remove_asset_version", { assetId, folderPath }),

  // Scraper
  /** Returns the site display name (e.g. "Booth") if this URL is supported, null otherwise. No network call. */
  detectSupportedSite: (url: string) => invoke<string | null>("detect_supported_site", { url }),
  /** Fetches and parses metadata from a supported product page URL. */
  fetchPageMeta: (url: string) => invoke<ScrapedMeta>("fetch_page_meta", { url }),

  // Native dialogs. Pickers return null if the user cancels.
  confirmAction: (message: string) => confirm(message, { title: "Please confirm", kind: "warning" }),
  pickFolder: async (title?: string): Promise<string | null> => {
    const result = await open({ directory: true, multiple: false, title });
    return typeof result === "string" ? result : null;
  },
  pickFiles: async (): Promise<string[]> => {
    const result = await open({ multiple: true, directory: false, title: "Choose files" });
    return Array.isArray(result) ? result : typeof result === "string" ? [result] : [];
  },
  pickFolders: async (): Promise<string[]> => {
    const result = await open({ multiple: true, directory: true, title: "Choose folders" });
    return Array.isArray(result) ? result : typeof result === "string" ? [result] : [];
  },
  pickImages: async (): Promise<string[]> => {
    const result = await open({
      multiple: true,
      directory: false,
      filters: [{ name: "Images", extensions: IMAGE_EXTENSIONS }],
    });
    return Array.isArray(result) ? result : typeof result === "string" ? [result] : [];
  },
  pickImage: async (): Promise<string | null> => {
    const result = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "Images", extensions: IMAGE_EXTENSIONS }],
    });
    return typeof result === "string" ? result : null;
  },
};
