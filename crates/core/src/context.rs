//! Process-wide state shared by the UI and the worker threads: settings (persisted), cached game
//! paths, the conflict index cache and the set of game processes being followed.

use crate::conflicts::ConflictCache;
use crate::json_store;
use crate::launch::LaunchTracker;
use crate::packs::ModEntry;
use crate::paths;
use crate::settings::Settings;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub struct AppContext {
    settings: Mutex<Settings>,
    /// Detected game paths. Detection shells out (registry query, exe version), so it runs once
    /// and is reused until the settings change or the exe disappears.
    paths_cache: Mutex<Option<paths::GamePaths>>,
    pub conflicts: ConflictCache,
    pub tracker: LaunchTracker,
}

pub type Ctx = Arc<AppContext>;

impl AppContext {
    pub fn load() -> Ctx {
        let settings = json_store::load::<Settings>(&settings_path()).unwrap_or_else(|e| {
            eprintln!("settings: {e}; using defaults");
            Settings::default()
        });
        Arc::new(AppContext {
            settings: Mutex::new(settings),
            paths_cache: Mutex::new(None),
            conflicts: ConflictCache::default(),
            tracker: LaunchTracker::default(),
        })
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Persist new settings; forgets cached paths so a changed game folder takes effect.
    pub fn set_settings(&self, settings: Settings) -> Result<(), String> {
        json_store::save(&settings_path(), &settings)?;
        if let Ok(mut s) = self.settings.lock() {
            *s = settings;
        }
        self.invalidate_paths();
        Ok(())
    }

    /// Resolve game paths from the settings override or Steam auto-detection (cached).
    pub fn game_paths(&self) -> paths::GamePaths {
        if let Ok(cache) = self.paths_cache.lock() {
            if let Some(p) = cache.as_ref() {
                let exe_ok = p.exe.as_deref().map(|e| Path::new(e).is_file()).unwrap_or(false);
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

    /// Every user pack: data/, Workshop, the game's own `mods\` folder (where the Epic and
    /// Game Pass builds keep downloaded mods), the extra folders from the settings, and the
    /// active files of mods installed from Nexus.
    pub fn scan(&self) -> Vec<ModEntry> {
        let p = self.game_paths();
        let mut extra: Vec<PathBuf> = self.settings().extra_mod_dirs.iter().map(PathBuf::from).collect();
        if let Some(mods) = p.mods_dir.as_deref().map(PathBuf::from) {
            if !extra.iter().any(|d| d == &mods) {
                extra.push(mods);
            }
        }
        let mut out = crate::packs::scan(p.data_dir.as_deref().map(Path::new), p.workshop_dir.as_deref().map(Path::new), &extra);
        out.extend(crate::nexus::scan(&crate::nexus::root()));
        out
    }

    pub fn invalidate_paths(&self) {
        if let Ok(mut cache) = self.paths_cache.lock() {
            *cache = None;
        }
    }
}

pub fn settings_path() -> PathBuf {
    paths::app_data_dir().join("settings.json")
}
/// Profiles are per copy of the game: a Steam profile's Workshop packs do not exist in an
/// Epic install, so each store keeps its own file. Steam (and an unrecognised folder) keep
/// the original name, so existing profiles are untouched.
pub fn profiles_path(store: paths::GameStore) -> PathBuf {
    let name = match store {
        paths::GameStore::Epic => "profiles-epic.json",
        paths::GameStore::GamePass => "profiles-gamepass.json",
        _ => "profiles.json",
    };
    paths::app_data_dir().join(name)
}
pub fn meta_path() -> PathBuf {
    paths::app_data_dir().join("mods.meta.json")
}
