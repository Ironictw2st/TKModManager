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
    /// Same for the app itself. Pre-releases are only offered when the user opts in.
    pub app_channel: String,
    /// Closing the window hides it to the tray instead of quitting.
    pub minimize_to_tray: bool,
    /// Before a launch, ask Steam to bring the enabled Workshop items up to date (what CA's
    /// launcher does). Starting the exe directly skips Steam's own pre-launch update.
    pub update_workshop_on_launch: bool,
    /// Unix seconds: Workshop mods last updated before this count as "older than the game
    /// patch". None = the game exe's build date.
    pub outdated_before: Option<u64>,
    /// App release the user chose "Skip this version" for; not offered again at startup.
    pub skipped_app_version: String,
    /// Same for the script-extender DLL.
    pub skipped_dll_version: String,
    /// Check GitHub for a newer Three Kingdoms Coop Setup on startup (only when Coop is installed).
    pub check_coop_updates: bool,
    /// Coop release the user chose "Skip this version" for.
    pub skipped_coop_version: String,
    /// Mod list row size: "compact" | "normal" | "large".
    pub list_density: String,
    /// Personal Nexus Mods API key (nexusmods.com > Site preferences > API Access).
    pub nexus_api_key: String,
    /// Modified time (unix seconds) of the newest crash.log the user has been told about.
    pub last_crash_seen: u64,
    /// Script-extender settings for profiles without their own. None = not set up yet: taken
    /// from an existing `dll\script_extender.cfg` (`dll::default_se_config`).
    pub se_config_default: Option<crate::se_config::SeConfig>,
    /// UI language: a folder name under `locales/` ("en", "de", "pt-BR"). "" = not chosen yet,
    /// so the first start asks.
    pub language: String,
    /// The first-time setup / tour version this user has been through or offered. 0 = never.
    pub setup_done: u32,
}

/// Bump when the setup or tour gains something existing users should be offered again.
pub const SETUP_VERSION: u32 = 1;

/// No profile file was ever saved in `dir`: the manager's first start on this PC. Existing
/// users have one (`profiles.json`, or per store `profiles-epic.json`).
pub fn first_start_in(dir: &std::path::Path) -> bool {
    !std::fs::read_dir(dir)
        .map(|rd| rd.flatten().any(|e| {
            let n = e.file_name().to_string_lossy().to_ascii_lowercase();
            n.starts_with("profiles") && n.ends_with(".json")
        }))
        .unwrap_or(false)
}

pub fn first_start() -> bool {
    first_start_in(&crate::paths::app_data_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_start_means_no_profile_file() {
        let dir = tempfile::tempdir().unwrap();
        assert!(first_start_in(dir.path()));
        assert!(first_start_in(&dir.path().join("missing")));
        std::fs::write(dir.path().join("settings.json"), "{}").unwrap();
        assert!(first_start_in(dir.path()), "settings alone (language picked) is still a first start");
        std::fs::write(dir.path().join("profiles-epic.json"), "{}").unwrap();
        assert!(!first_start_in(dir.path()));
        // Older settings files have no setupDone: they read as 0.
        let s: Settings = serde_json::from_str("{\"language\":\"en\"}").unwrap();
        assert_eq!(s.setup_done, 0);
    }
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
            app_channel: "stable".into(),
            minimize_to_tray: false,
            update_workshop_on_launch: true,
            outdated_before: None,
            skipped_app_version: String::new(),
            skipped_dll_version: String::new(),
            check_coop_updates: true,
            skipped_coop_version: String::new(),
            list_density: "normal".into(),
            nexus_api_key: String::new(),
            last_crash_seen: 0,
            se_config_default: None,
            language: String::new(),
            setup_done: 0,
        }
    }
}
