mod commands;
mod conflicts;
mod dll;
mod empty_vp8;
mod fingerprint;
mod json_store;
mod launch;
mod meta;
mod modlist;
mod options_pack;
mod packs;
mod paths;
mod profiles;
mod settings;
mod state;
mod update;
mod workshop;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .manage(AppState::load())
        .manage(conflicts::ConflictCache::default())
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::app_data_dir,
            commands::get_settings,
            commands::set_settings,
            commands::get_paths,
            commands::scan_mods,
            commands::load_profiles,
            commands::save_profiles,
            commands::load_meta,
            commands::save_meta,
            commands::read_mod_list_file,
            commands::import_mod_list,
            commands::preview_mod_list,
            launch::game_running,
            launch::launch_game,
            launch::list_saves,
            dll::dll_status,
            dll::dll_check_update,
            dll::dll_install,
            dll::dll_import_local,
            dll::dll_remove,
            dll::dll_read_log,
            workshop::workshop_cached,
            workshop::workshop_fetch,
            conflicts::conflicts_for,
            update::check_update,
            update::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
