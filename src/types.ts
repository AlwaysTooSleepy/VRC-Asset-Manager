// Mirrors src-tauri/src/models.rs exactly (camelCase on the wire).
// If you change one, change the other.
// All paths are relative to the library root, with forward slashes.

export interface Site { id: string; name: string }
export interface Category { id: string; name: string }
export interface Creator {
  id: string;
  name: string;
  iconImagePath?: string | null;
  profileUrls: string[];
}
export interface Version {
  versionLabel: string;
  folderPath: string;
  dateAdded: string;
  notes?: string | null;
  /** Derived by the backend: this version's folder isn't on disk. */
  missing: boolean;
}
export interface Model {
  id: string;
  name: string;
  creatorId: string;
  siteId: string;
  sourceUrl?: string | null;
  folderPath: string;
  imagePaths: string[];
  versions: Version[];
  dateAdded: string;
  dateUpdated: string;
  /** Derived by the backend: the model has versions and none of their folders exist. */
  missing: boolean;
}
export interface Asset {
  id: string;
  name: string;
  creatorId: string;
  siteId: string;
  categoryId: string;
  folderPath: string;
  imagePaths: string[];
  versions: Version[];
  compatibleModelIds: string[];
  compatibleWithAll: boolean;
  sourceUrl?: string | null;
  dateAdded: string;
  dateUpdated: string;
  /** Derived by the backend: the asset has versions and none of their folders exist. */
  missing: boolean;
}
export interface Library {
  schemaVersion: number;
  sites: Site[];
  creators: Creator[];
  categories: Category[];
  models: Model[];
  assets: Asset[];
}

/** Payload for add_creator / update_creator (mirrors CreatorInput in reference.rs). */
export interface CreatorInput {
  name: string;
  iconImagePath: string | null;
  profileUrls: string[];
}

/** Payload for add_model and update_model (mirrors ModelInput in items.rs). */
export interface ModelInput {
  name: string;
  creatorId: string;
  siteId: string;
  sourceUrl: string | null;
  imagePaths: string[];
}

export interface AddModelResult {
  library: Library;
  modelId: string;
}

/** Payload for add_version (mirrors AddVersionInput in items.rs). */
export interface AddVersionInput {
  modelId: string;
  version: string;
  notes: string | null;
  /** Folder to move into the library; null creates an empty folder instead. */
  sourceFolder: string | null;
}

export interface AddVersionResult {
  library: Library;
  /** Library-relative folder the version lives in. */
  destination: string;
  warning: string | null;
}

/** Payload for update_version (mirrors UpdateVersionInput in items.rs). */
export interface UpdateVersionInput {
  modelId: string;
  /** Identifies the version: its current folder path. */
  folderPath: string;
  version: string;
  notes: string | null;
}

/** Payload for add_asset and update_asset (mirrors AssetInput in assets.rs).
 *  Both compatibility fields are always sent. */
export interface AssetInput {
  name: string;
  creatorId: string;
  siteId: string;
  categoryId: string;
  sourceUrl: string | null;
  imagePaths: string[];
  compatibleModelIds: string[];
  compatibleWithAll: boolean;
}

export interface AddAssetResult {
  library: Library;
  assetId: string;
}

/** One file or folder being filed into an asset version (mirrors SourceInput in filing.rs). */
export interface SourceInput {
  path: string;
  /** "none" = straight into the version folder, "shared" = Shared/, "model" = For_<Model>/ */
  target: "none" | "shared" | "model";
  modelId: string | null;
}

/** Payload for add_asset_version (mirrors AddAssetVersionInput in assets.rs). */
export interface AddAssetVersionInput {
  assetId: string;
  version: string;
  notes: string | null;
  /** Files and folders to move in. Empty creates an empty version folder. */
  sources: SourceInput[];
}

/** What adding an asset version would create (dry run). */
export interface AssetVersionPreview {
  /** Library-relative version folder. */
  path: string;
  /** Subfolders that would be created inside it (empty for a flat layout). */
  subfolders: string[];
}

/** Payload for update_asset_version (mirrors UpdateAssetVersionInput in assets.rs). */
export interface UpdateAssetVersionInput {
  assetId: string;
  /** Identifies the version: its current folder path. */
  folderPath: string;
  version: string;
  notes: string | null;
}

/** A dismissible message shown at the top of the page. */
export interface Notice {
  text: string;
  warning: string | null;
}

/** Metadata returned by the page scraper (all fields optional). */
export interface ScrapedMeta {
  name: string | null;
  creatorName: string | null;
  /** Creator/shop icon found on the page, if any — used when auto-creating a new creator. */
  creatorIconUrl: string | null;
  /** The creator's own shop/profile page, if one could be determined — used when auto-creating a new creator. */
  creatorProfileUrl: string | null;
  /** Every product image found on the page, in page order. Empty if none were found. */
  imageUrls: string[];
  siteName: string | null;
}
