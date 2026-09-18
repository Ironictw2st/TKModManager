mod cli;
mod commands;
mod conflicts;
mod dll;
mod empty_vp8;
mod fingerprint;
mod hash;
mod history;
mod json_store;
mod launch;
mod logs;
mod meta;
mod modlist;
mod options_pack;
mod packs;
mod paths;
mod profiles;
mod settings;
mod state;
mod tray;
mod update;
mod winproc;
mod workshop;

use state::AppState;
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let startup = cli::parse(std::env::args());
    let start_minimized = startup.minimized;
    tauri::Builder::default()
        // Must be first: a second copy (e.g. a profile shortcut) hands its args to this one.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let args = cli::parse(argv);
            if !args.minimized {
                tray::show_main(app);
            }
            let _ = app.emit("cli-args", args);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .manage(AppState::load())
        .manage(conflicts::ConflictCache::default())
        .manage(launch::LaunchTracker::default())
        .manage(cli::Startup(std::sync::Mutex::new(Some(startup))))
        .setup(move |app| {
            let handle = app.handle().clone();
            let to_tray = app.state::<AppState>().settings().minimize_to_tray;
            tray::build(&handle, to_tray || start_minimized)?;
            if start_minimized {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }
            launch::start_external_watcher(handle);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.app_handle().state::<AppState>().settings().minimize_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
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
            history::launch_history,
            history::crash_report,
            logs::log_sources,
            logs::log_tail,
            hash::hash_packs,
            cli::startup_args,
            cli::create_profile_shortcut,
            dll::dll_status,
            dll::dll_check_update,
            dll::dll_install,
            dll::dll_import_local,
            dll::dll_remove,
            dll::dll_read_log,
            dll::dll_read_cfg,
            dll::dll_write_cfg,
            workshop::workshop_cached,
            workshop::workshop_fetch,
            workshop::workshop_collection,
            conflicts::conflicts_for,
            update::check_update,
            update::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
