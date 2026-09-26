mod assets;
mod commands;
mod db;
mod filing;
mod fsops;
mod images;
mod items;
mod models;
mod organizer;
mod reference;
mod scraper;
mod store;
mod sync;
mod system;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Re-allow image access to the saved library folder on every launch.
            if let Ok(Some(root)) = store::read_root(app.handle()) {
                let _ = app.asset_protocol_scope().allow_directory(&root, true);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_library_root,
            commands::set_library_root,
            commands::get_library,
            images::import_image,
            images::remove_image_if_unused,
            system::open_folder,
            system::open_url,
            commands::sync_library,
            images::import_image_from_url,
            items::add_model,
            items::update_model,
            items::remove_model,
            items::preview_version_path,
            items::add_version,
            items::update_version,
            items::remove_version,
            assets::add_asset,
            assets::update_asset,
            assets::remove_asset,
            assets::preview_asset_version,
            assets::add_asset_version,
            assets::update_asset_version,
            assets::remove_asset_version,
            reference::add_site,
            reference::rename_site,
            reference::delete_site,
            reference::add_category,
            reference::rename_category,
            reference::delete_category,
            reference::add_creator,
            reference::update_creator,
            reference::delete_creator,
            scraper::detect_supported_site,
            scraper::fetch_page_meta,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
