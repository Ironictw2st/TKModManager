//! App settings (`settings.json`). Everything has a default so a missing file is fine.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Explicit game root override; None = auto-detect through Steam.
    pub game_root: Option<String>,
    /// Explicit Workshop content dir override.
    pub workshop_dir: Option<String>,
    /// "dark" | "light" | "system"
    pub theme_mode: String,
    pub accent: String,
    /// Check GitHub for a newer app build on startup.
    pub check_app_updates: bool,
    /// Check GitHub for a newer script-extender DLL on startup.
    pub check_dll_updates: bool,
    /// Optional Steam Web API key (unlocks required-item lookups without scraping).
    pub steam_api_key: String,
    /// Hours before cached Workshop metadata is refreshed.
    pub workshop_cache_hours: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            game_root: None,
            workshop_dir: None,
            theme_mode: "dark".into(),
            accent: "#c9a227".into(),
            check_app_updates: true,
            check_dll_updates: true,
            steam_api_key: String::new(),
            workshop_cache_hours: 24,
        }
    }
}
