//! Process-wide state shared by commands: settings (persisted) and the launch tracker.

use crate::json_store;
use crate::paths;
use crate::settings::Settings;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    pub settings: Mutex<Settings>,
    /// Detected game paths. Detection shells out (registry query, exe version), so it runs
    /// once and is reused until the settings change or the exe disappears.
    paths_cache: Mutex<Option<paths::GamePaths>>,
}

impl AppState {
    pub fn load() -> Self {
        let settings = json_store::load::<Settings>(&settings_path()).unwrap_or_else(|e| {
            eprintln!("settings: {e}; using defaults");
            Settings::default()
        });
        AppState { settings: Mutex::new(settings), paths_cache: Mutex::new(None) }
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Resolve game paths from the settings override or Steam auto-detection (cached).
    pub fn game_paths(&self) -> paths::GamePaths {
        if let Ok(cache) = self.paths_cache.lock() {
            if let Some(p) = cache.as_ref() {
                let exe_ok = p.exe.as_deref().map(|e| std::path::Path::new(e).is_file()).unwrap_or(false);
                if exe_ok {
                    return p.clone();
                }
            }
        }
        let s = self.settings();
        let root = s
            .game_root
            .as_deref()
            .map(PathBuf::from)
            .filter(|p| p.join(paths::EXE_NAME).is_file())
            .or_else(paths::detect_game_root);
        let ws = s.workshop_dir.as_deref().map(PathBuf::from);
        let resolved = paths::resolve(root.as_deref(), ws.as_deref());
        if resolved.game_root.is_some() {
            if let Ok(mut cache) = self.paths_cache.lock() {
                *cache = Some(resolved.clone());
            }
        }
        resolved
    }

    /// Forget the cached paths (settings changed).
    pub fn invalidate_paths(&self) {
        if let Ok(mut cache) = self.paths_cache.lock() {
            *cache = None;
        }
    }
}

pub fn settings_path() -> PathBuf {
    paths::app_data_dir().join("settings.json")
}
pub fn profiles_path() -> PathBuf {
    paths::app_data_dir().join("profiles.json")
}
pub fn meta_path() -> PathBuf {
    paths::app_data_dir().join("mods.meta.json")
}
