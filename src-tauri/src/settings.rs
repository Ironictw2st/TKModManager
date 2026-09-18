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
    /// Extra folders whose `*.pack` files are listed and loaded in place (e.g. RPFM MyMods).
    pub extra_mod_dirs: Vec<String>,
    /// Inject the script extender into a game that was started outside the manager.
    pub auto_inject_external: bool,
    /// "stable" | "prerelease": which script-extender releases are offered.
    pub dll_channel: String,
    /// Closing the window hides it to the tray instead of quitting.
    pub minimize_to_tray: bool,
    /// Unix seconds: Workshop mods last updated before this count as "older than the game
    /// patch". None = the game exe's build date.
    pub outdated_before: Option<u64>,
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
            extra_mod_dirs: Vec::new(),
            auto_inject_external: false,
            dll_channel: "stable".into(),
            minimize_to_tray: false,
            outdated_before: None,
        }
    }
}
