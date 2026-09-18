//! Process-wide state shared by commands: settings (persisted) and the launch tracker.

use crate::json_store;
use crate::paths;
use crate::settings::Settings;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    pub settings: Mutex<Settings>,
}

impl AppState {
    pub fn load() -> Self {
        let settings = json_store::load::<Settings>(&settings_path()).unwrap_or_else(|e| {
            eprintln!("settings: {e}; using defaults");
            Settings::default()
        });
        AppState { settings: Mutex::new(settings) }
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Resolve game paths from the settings override or Steam auto-detection.
    pub fn game_paths(&self) -> paths::GamePaths {
        let s = self.settings();
        let root = s
            .game_root
            .as_deref()
            .map(PathBuf::from)
            .filter(|p| p.join(paths::EXE_NAME).is_file())
            .or_else(paths::detect_game_root);
        let ws = s.workshop_dir.as_deref().map(PathBuf::from);
        paths::resolve(root.as_deref(), ws.as_deref())
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
