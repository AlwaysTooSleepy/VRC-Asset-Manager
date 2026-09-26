use crate::models::{Asset, Category, Creator, Library, Model, Site, Version};
use rusqlite::{params, types::Type, Connection, Transaction};
use std::{
    fs,
    path::{Path, PathBuf},
};

const DB_RELATIVE_PATH: &str = "AppData/SQLite/library.sqlite";
const SCHEMA_VERSION: &str = "1";

fn json_error<E: ToString>(error: E) -> String {
    error.to_string()
}

pub fn path(root: &Path) -> PathBuf {
    root.join(DB_RELATIVE_PATH)
}

pub fn open(root: &Path) -> Result<Connection, String> {
    fs::create_dir_all(path(root).parent().expect("database path has a parent"))
        .map_err(json_error)?;
    let conn = Connection::open(path(root)).map_err(json_error)?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(json_error)?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(json_error)?;
    initialize(&conn)?;
    Ok(conn)
}

pub fn initialize(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_meta (
            key TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sites (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL UNIQUE COLLATE NOCASE
        );
        CREATE TABLE IF NOT EXISTS categories (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL UNIQUE COLLATE NOCASE
        );
        CREATE TABLE IF NOT EXISTS creators (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL UNIQUE COLLATE NOCASE,
            icon_image_path TEXT,
            profile_urls_json TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS models (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            creator_id TEXT NOT NULL REFERENCES creators(id),
            site_id TEXT NOT NULL REFERENCES sites(id),
            source_url TEXT,
            folder_path TEXT NOT NULL,
            date_added TEXT NOT NULL,
            date_updated TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS assets (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            creator_id TEXT NOT NULL REFERENCES creators(id),
            site_id TEXT NOT NULL REFERENCES sites(id),
            category_id TEXT NOT NULL REFERENCES categories(id),
            source_url TEXT,
            folder_path TEXT NOT NULL,
            compatible_with_all INTEGER NOT NULL CHECK (compatible_with_all IN (0, 1)),
            date_added TEXT NOT NULL,
            date_updated TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS model_versions (
            model_id TEXT NOT NULL REFERENCES models(id) ON DELETE CASCADE,
            version_label TEXT NOT NULL,
            folder_path TEXT NOT NULL,
            date_added TEXT NOT NULL,
            notes TEXT,
            missing INTEGER NOT NULL CHECK (missing IN (0, 1)),
            PRIMARY KEY (model_id, folder_path),
            UNIQUE (model_id, version_label)
        );
        CREATE TABLE IF NOT EXISTS asset_versions (
            asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
            version_label TEXT NOT NULL,
            folder_path TEXT NOT NULL,
            date_added TEXT NOT NULL,
            notes TEXT,
            missing INTEGER NOT NULL CHECK (missing IN (0, 1)),
            PRIMARY KEY (asset_id, folder_path),
            UNIQUE (asset_id, version_label)
        );
        CREATE TABLE IF NOT EXISTS model_images (
            model_id TEXT NOT NULL REFERENCES models(id) ON DELETE CASCADE,
            path TEXT NOT NULL,
            PRIMARY KEY (model_id, path)
        );
        CREATE TABLE IF NOT EXISTS asset_images (
            asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
            path TEXT NOT NULL,
            PRIMARY KEY (asset_id, path)
        );
        CREATE TABLE IF NOT EXISTS asset_compatible_models (
            asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
            model_id TEXT NOT NULL REFERENCES models(id) ON DELETE CASCADE,
            PRIMARY KEY (asset_id, model_id)
        );",
    )
    .map_err(json_error)?;
    conn.execute(
        "INSERT INTO schema_meta(key, value) VALUES('schema_version', ?1)
         ON CONFLICT(key) DO NOTHING",
        [SCHEMA_VERSION],
    )
    .map_err(json_error)?;
    Ok(())
}

fn insert_library(tx: &Transaction<'_>, library: &Library) -> Result<(), String> {
    for site in &library.sites {
        tx.execute(
            "INSERT INTO sites(id, name) VALUES(?1, ?2)",
            params![site.id, site.name],
        )
        .map_err(json_error)?;
    }
    for category in &library.categories {
        tx.execute(
            "INSERT INTO categories(id, name) VALUES(?1, ?2)",
            params![category.id, category.name],
        )
        .map_err(json_error)?;
    }
    for creator in &library.creators {
        let urls = serde_json::to_string(&creator.profile_urls).map_err(json_error)?;
        tx.execute(
            "INSERT INTO creators(id, name, icon_image_path, profile_urls_json) VALUES(?1, ?2, ?3, ?4)",
            params![creator.id, creator.name, creator.icon_image_path, urls],
        ).map_err(json_error)?;
    }
    for model in &library.models {
        tx.execute(
            "INSERT INTO models(id, name, creator_id, site_id, source_url, folder_path, date_added, date_updated)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![model.id, model.name, model.creator_id, model.site_id, model.source_url, model.folder_path, model.date_added, model.date_updated],
        ).map_err(json_error)?;
        for path in &model.image_paths {
            tx.execute(
                "INSERT INTO model_images(model_id, path) VALUES(?1, ?2)",
                params![model.id, path],
            )
            .map_err(json_error)?;
        }
        for version in &model.versions {
            tx.execute(
                "INSERT INTO model_versions(model_id, version_label, folder_path, date_added, notes, missing)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                params![model.id, version.version_label, version.folder_path, version.date_added, version.notes, model_bool(version.missing)],
            ).map_err(json_error)?;
        }
    }
    for asset in &library.assets {
        tx.execute(
            "INSERT INTO assets(id, name, creator_id, site_id, category_id, source_url, folder_path, compatible_with_all, date_added, date_updated)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![asset.id, asset.name, asset.creator_id, asset.site_id, asset.category_id, asset.source_url, asset.folder_path, model_bool(asset.compatible_with_all), asset.date_added, asset.date_updated],
        ).map_err(json_error)?;
        for path in &asset.image_paths {
            tx.execute(
                "INSERT INTO asset_images(asset_id, path) VALUES(?1, ?2)",
                params![asset.id, path],
            )
            .map_err(json_error)?;
        }
        for model_id in &asset.compatible_model_ids {
            tx.execute(
                "INSERT INTO asset_compatible_models(asset_id, model_id) VALUES(?1, ?2)",
                params![asset.id, model_id],
            )
            .map_err(json_error)?;
        }
        for version in &asset.versions {
            tx.execute(
                "INSERT INTO asset_versions(asset_id, version_label, folder_path, date_added, notes, missing)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                params![asset.id, version.version_label, version.folder_path, version.date_added, version.notes, model_bool(version.missing)],
            ).map_err(json_error)?;
        }
    }
    Ok(())
}

fn model_bool(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn read_versions(conn: &Connection, table: &str, id: &str) -> Result<Vec<Version>, String> {
    let sql = format!("SELECT version_label, folder_path, date_added, notes, missing FROM {table} WHERE {} = ?1 ORDER BY rowid", if table == "model_versions" { "model_id" } else { "asset_id" });
    let mut statement = conn.prepare(&sql).map_err(json_error)?;
    let rows = statement
        .query_map([id], |row| {
            Ok(Version {
                version_label: row.get(0)?,
                folder_path: row.get(1)?,
                date_added: row.get(2)?,
                notes: row.get(3)?,
                missing: row.get::<_, i64>(4)? != 0,
            })
        })
        .map_err(json_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(json_error)
}

fn read_paths(
    conn: &Connection,
    table: &str,
    id_column: &str,
    id: &str,
) -> Result<Vec<String>, String> {
    let sql = format!("SELECT path FROM {table} WHERE {id_column} = ?1 ORDER BY rowid");
    let mut statement = conn.prepare(&sql).map_err(json_error)?;
    let rows = statement
        .query_map([id], |row| row.get(0))
        .map_err(json_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(json_error)
}

pub fn load_library(root: &Path) -> Result<Library, String> {
    let conn = open(root)?;
    let sites = {
        let mut stmt = conn
            .prepare("SELECT id, name FROM sites ORDER BY rowid")
            .map_err(json_error)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Site {
                    id: r.get(0)?,
                    name: r.get(1)?,
                })
            })
            .map_err(json_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(json_error)?;
        rows
    };
    let categories = {
        let mut stmt = conn
            .prepare("SELECT id, name FROM categories ORDER BY rowid")
            .map_err(json_error)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Category {
                    id: r.get(0)?,
                    name: r.get(1)?,
                })
            })
            .map_err(json_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(json_error)?;
        rows
    };
    let creators = {
        let mut stmt = conn
            .prepare(
                "SELECT id, name, icon_image_path, profile_urls_json FROM creators ORDER BY rowid",
            )
            .map_err(json_error)?;
        let rows = stmt
            .query_map([], |r| {
                let urls: String = r.get(3)?;
                let profile_urls = serde_json::from_str(&urls).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(3, Type::Text, Box::new(error))
                })?;
                Ok(Creator {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    icon_image_path: r.get(2)?,
                    profile_urls,
                })
            })
            .map_err(json_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(json_error)?;
        rows
    };
    let mut models = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT id, name, creator_id, site_id, source_url, folder_path, date_added, date_updated FROM models ORDER BY rowid").map_err(json_error)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Model {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    creator_id: r.get(2)?,
                    site_id: r.get(3)?,
                    source_url: r.get(4)?,
                    folder_path: r.get(5)?,
                    image_paths: Vec::new(),
                    versions: Vec::new(),
                    date_added: r.get(6)?,
                    date_updated: r.get(7)?,
                    missing: false,
                })
            })
            .map_err(json_error)?;
        for row in rows {
            let mut model = row.map_err(json_error)?;
            model.image_paths = read_paths(&conn, "model_images", "model_id", &model.id)?;
            model.versions = read_versions(&conn, "model_versions", &model.id)?;
            model.missing =
                !model.versions.is_empty() && model.versions.iter().all(|version| version.missing);
            models.push(model);
        }
    }
    let mut assets = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT id, name, creator_id, site_id, category_id, source_url, folder_path, compatible_with_all, date_added, date_updated FROM assets ORDER BY rowid").map_err(json_error)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Asset {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    creator_id: r.get(2)?,
                    site_id: r.get(3)?,
                    category_id: r.get(4)?,
                    source_url: r.get(5)?,
                    folder_path: r.get(6)?,
                    image_paths: Vec::new(),
                    versions: Vec::new(),
                    compatible_model_ids: Vec::new(),
                    compatible_with_all: r.get::<_, i64>(7)? != 0,
                    date_added: r.get(8)?,
                    date_updated: r.get(9)?,
                    missing: false,
                })
            })
            .map_err(json_error)?;
        for row in rows {
            let mut asset = row.map_err(json_error)?;
            asset.image_paths = read_paths(&conn, "asset_images", "asset_id", &asset.id)?;
            asset.versions = read_versions(&conn, "asset_versions", &asset.id)?;
            asset.missing =
                !asset.versions.is_empty() && asset.versions.iter().all(|version| version.missing);
            let mut links = conn.prepare("SELECT model_id FROM asset_compatible_models WHERE asset_id = ?1 ORDER BY rowid").map_err(json_error)?;
            asset.compatible_model_ids = links
                .query_map([&asset.id], |r| r.get(0))
                .map_err(json_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(json_error)?;
            assets.push(asset);
        }
    }
    Ok(Library {
        schema_version: 1,
        sites,
        creators,
        categories,
        models,
        assets,
    })
}

pub fn save_library(root: &Path, library: &Library) -> Result<(), String> {
    let mut conn = open(root)?;
    let tx = conn.transaction().map_err(json_error)?;
    tx.execute_batch("DELETE FROM asset_compatible_models; DELETE FROM model_images; DELETE FROM asset_images; DELETE FROM model_versions; DELETE FROM asset_versions; DELETE FROM models; DELETE FROM assets; DELETE FROM sites; DELETE FROM categories; DELETE FROM creators;").map_err(json_error)?;
    insert_library(&tx, library)?;
    tx.commit().map_err(json_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_relational_library() {
        let root =
            std::env::temp_dir().join(format!("vrchat-asset-manager-db-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let library = Library {
            sites: vec![Site {
                id: "site".into(),
                name: "Store".into(),
            }],
            categories: vec![Category {
                id: "category".into(),
                name: "Clothes".into(),
            }],
            creators: vec![Creator {
                id: "creator".into(),
                name: "Maker".into(),
                icon_image_path: Some("AppData/Images/icon.png".into()),
                profile_urls: vec!["https://example.test".into()],
            }],
            models: vec![Model {
                id: "model".into(),
                name: "Avatar".into(),
                creator_id: "creator".into(),
                site_id: "site".into(),
                source_url: Some("https://example.test/model".into()),
                folder_path: "Models/Maker/Avatar_V_1".into(),
                image_paths: vec!["AppData/Images/model.png".into()],
                versions: vec![Version {
                    version_label: "1".into(),
                    folder_path: "Models/Maker/Avatar_V_1".into(),
                    date_added: "2026-01-01T00:00:00Z".into(),
                    notes: Some("notes".into()),
                    missing: false,
                }],
                date_added: "2026-01-01T00:00:00Z".into(),
                date_updated: "2026-01-01T00:00:00Z".into(),
                missing: false,
            }],
            assets: vec![Asset {
                id: "asset".into(),
                name: "Outfit".into(),
                creator_id: "creator".into(),
                site_id: "site".into(),
                category_id: "category".into(),
                source_url: None,
                folder_path: "Assets/Clothes/Maker/Outfit_V_1".into(),
                image_paths: vec![],
                versions: vec![],
                compatible_model_ids: vec!["model".into()],
                compatible_with_all: false,
                date_added: "2026-01-01T00:00:00Z".into(),
                date_updated: "2026-01-01T00:00:00Z".into(),
                missing: false,
            }],
            schema_version: 1,
        };
        save_library(&root, &library).unwrap();
        assert_eq!(
            serde_json::to_string(&library).unwrap(),
            serde_json::to_string(&load_library(&root).unwrap()).unwrap()
        );
        fs::remove_dir_all(root).unwrap();
    }
}
