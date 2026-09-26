//! Data model. The JSON library file is the source of truth; folders on disk
//! and the UI are representations of it. Items reference Creators, Sites and
//! Categories by ID only, so renaming one never touches an item.
//!
//! All stored paths (folderPath, imagePaths, iconImagePath) are RELATIVE to the
//! library root and use forward slashes, e.g. "Models/Komado/Chocolat_V_1.0.0".
//! This keeps the library portable if the whole folder is moved.
//!
//! Every struct uses camelCase on the wire so it matches src/types.ts exactly.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Creator {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub icon_image_path: Option<String>,
    #[serde(default)]
    pub profile_urls: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub version_label: String,
    pub folder_path: String,
    pub date_added: String,
    #[serde(default)]
    pub notes: Option<String>,
    /// Derived: true when this version's folder isn't on disk. Refreshed by every sync.
    #[serde(default)]
    pub missing: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: String,
    pub name: String,
    pub creator_id: String,
    pub site_id: String,
    #[serde(default)]
    pub source_url: Option<String>,
    /// Folder of the current (highest present) version. Empty until a version exists.
    pub folder_path: String,
    #[serde(default)]
    pub image_paths: Vec<String>,
    #[serde(default)]
    pub versions: Vec<Version>,
    pub date_added: String,
    pub date_updated: String,
    /// Derived: true when the model has versions and none of their folders exist.
    #[serde(default)]
    pub missing: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub name: String,
    pub creator_id: String,
    pub site_id: String,
    pub category_id: String,
    #[serde(default)]
    pub source_url: Option<String>,
    /// Folder of the current (highest present) version. Empty until a version exists.
    pub folder_path: String,
    #[serde(default)]
    pub image_paths: Vec<String>,
    #[serde(default)]
    pub versions: Vec<Version>,
    /// Always present on the wire (see Vision doc section 7).
    #[serde(default)]
    pub compatible_model_ids: Vec<String>,
    #[serde(default)]
    pub compatible_with_all: bool,
    pub date_added: String,
    pub date_updated: String,
    /// Derived: true when the asset has versions and none of their folders exist.
    #[serde(default)]
    pub missing: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Library {
    pub schema_version: u32,
    pub sites: Vec<Site>,
    pub creators: Vec<Creator>,
    pub categories: Vec<Category>,
    pub models: Vec<Model>,
    pub assets: Vec<Asset>,
}

impl Default for Library {
    fn default() -> Self {
        Self {
            schema_version: 1,
            sites: vec![],
            creators: vec![],
            categories: vec![],
            models: vec![],
            assets: vec![],
        }
    }
}
