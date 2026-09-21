//! Everything the UI shows, owned by the GUI thread. Persistence goes through `tkmm_core::ops`.

use std::collections::HashMap;
use tkmm_core::dll::DllStatus;
use tkmm_core::launch::{LaunchStatus, SaveGame};
use tkmm_core::meta::{MetaDoc, ModMeta};
use tkmm_core::packs::{ModEntry, ModSource};
use tkmm_core::paths::{GameInstall, GamePaths, GameStore};
use tkmm_core::profiles::{Profile, ProfilesDoc};
use tkmm_core::se_scan::SeInfo;
use tkmm_core::status::{self, SeRequirement};
use tkmm_core::workshop::WorkshopItem;
use tkmm_core::{ops, profile_ops};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortKey {
    Status,
    Title,
    Source,
    Type,
    Size,
    Updated,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Filters {
    pub search: String,
    /// "all" | "workshop" | "data" | "folder"
    pub source: String,
    /// "all" | "mod" | "movie"
    pub pack_type: String,
    /// "all" | "enabled" | "disabled" | "updated"
    pub enabled: String,
    /// "all" | "pending" | "old" | "se"
    pub status: String,
    pub tag: Option<String>,
    pub show_hidden: bool,
    /// Show entries whose pack is not installed (unsubscribed or deleted). Off by default: the
    /// row disappears, while the profile entry stays so the load order survives a re-subscribe.
    pub show_missing: bool,
}

impl Default for Filters {
    fn default() -> Self {
        Filters {
            search: String::new(),
            source: "all".into(),
            pack_type: "all".into(),
            enabled: "all".into(),
            status: "all".into(),
            tag: None,
            show_hidden: false,
            show_missing: false,
        }
    }
}

impl Filters {
    /// Groups/drag-and-drop only make sense in load order without narrowing filters.
    pub fn is_narrowing(&self) -> bool {
        !self.search.trim().is_empty() || self.source != "all" || self.pack_type != "all" || self.enabled != "all" || self.status != "all" || self.tag.is_some()
    }
}

#[derive(Default)]
pub struct State {
    pub paths: GamePaths,
    /// Every copy of the game found on this PC (Steam, Epic, Game Pass), for the switcher.
    pub installs: Vec<GameInstall>,
    /// Which copy's profile file is loaded (profiles are per store).
    pub profiles_store: GameStore,
    /// The profiles file was read successfully (or was simply missing). False after a corrupt
    /// or unreadable file, which blocks saving so a bad read cannot overwrite the real thing.
    pub profiles_ok: bool,
    pub mods: Vec<ModEntry>,
    pub by_key: HashMap<String, usize>,
    pub profiles: ProfilesDoc,
    pub meta: MetaDoc,
    pub workshop: HashMap<String, WorkshopItem>,
    pub se_scan: HashMap<String, SeInfo>,
    pub dll: Option<DllStatus>,
    pub launch: Option<LaunchStatus>,
    pub game_running: bool,
    pub saves: Vec<SaveGame>,
    pub load_save: String,
    /// Last abnormal exit: (history id, exit code).
    pub crash: Option<(Option<u64>, Option<u32>)>,
    pub filters: Filters,
    pub sort: (Option<SortKey>, bool),
    pub selected: Vec<String>,
    pub focused: Option<String>,
    pub error: Option<String>,
}

impl State {
    pub fn load() -> State {
        let mut s = State { sort: (None, true), ..Default::default() };
        s.load_profiles_for(GameStore::default());
        s.meta = ops::load_meta().unwrap_or_default();
        s
    }

    /// Switch to another copy's profiles (each store has its own file).
    ///
    /// A failed read is remembered rather than papered over: `self.profiles` would otherwise keep
    /// the previous store's document (or the empty default), and the next scan's `reconcile` would
    /// save that straight over the file the user still has on disk.
    pub fn load_profiles_for(&mut self, store: GameStore) {
        self.profiles_store = store;
        match ops::load_profiles(store) {
            Ok(p) => {
                self.profiles = p;
                self.profiles_ok = true;
            }
            Err(e) => {
                self.profiles = ProfilesDoc::default();
                self.profiles_ok = false;
                self.error = Some(format!("{e}. Fix or move that file; nothing will be saved over it until then."));
            }
        }
    }

    pub fn set_mods(&mut self, mods: Vec<ModEntry>) {
        self.by_key = mods.iter().enumerate().map(|(i, m)| (m.key.clone(), i)).collect();
        self.mods = mods;
        if profile_ops::reconcile_all(&mut self.profiles, &self.mods) {
            self.save_profiles();
        }
    }

    pub fn module(&self, key: &str) -> Option<&ModEntry> {
        self.by_key.get(key).map(|i| &self.mods[*i])
    }

    /// Where a pack came from, telling the game's own `mods\` folder (Epic, Game Pass) apart
    /// from folders the user added.
    pub fn source_label(&self, m: &ModEntry) -> &'static str {
        match m.source {
            ModSource::Workshop => "Workshop",
            ModSource::Data => "data/",
            ModSource::Folder if self.paths.mods_dir.as_deref().map(|d| d.eq_ignore_ascii_case(&m.dir)).unwrap_or(false) => "mods/",
            ModSource::Folder => "Extra folder",
        }
    }

    pub fn ws_of(&self, m: &ModEntry) -> Option<&WorkshopItem> {
        m.workshop_id.as_ref().and_then(|id| self.workshop.get(id))
    }

    pub fn title_of(&self, key: &str) -> String {
        match self.module(key) {
            Some(m) => self.ws_of(m).map(|w| w.title.clone()).filter(|t| !t.is_empty()).unwrap_or_else(|| m.file.trim_end_matches(".pack").to_string()),
            None => key.to_string(),
        }
    }

    pub fn meta_of(&self, key: &str) -> ModMeta {
        self.meta.mods.get(key).cloned().unwrap_or_default()
    }

    pub fn active(&self) -> Profile {
        profile_ops::active(&self.profiles).cloned().unwrap_or_else(|| Profile {
            name: "Default".into(),
            entries: vec![],
            dll: false,
            skip_intro: false,
            last_played: None,
        })
    }

    /// Apply `f` to the active profile and save.
    pub fn update_active(&mut self, f: impl FnOnce(&mut Profile)) {
        if let Some(p) = profile_ops::active_mut(&mut self.profiles) {
            f(p);
        }
        self.save_profiles();
    }

    pub fn save_profiles(&mut self) {
        if !self.profiles_ok {
            return; // the file on disk could not be read; never write over it blind
        }
        if let Err(e) = ops::save_profiles(self.profiles_store, &self.profiles) {
            self.error = Some(e);
        }
    }

    pub fn set_meta(&mut self, key: &str, f: impl FnOnce(&mut ModMeta)) {
        let entry = self.meta.mods.entry(key.to_string()).or_default();
        f(entry);
        if let Err(e) = ops::save_meta(&self.meta) {
            self.error = Some(e);
        }
    }

    pub fn se_req(&self, key: &str) -> SeRequirement {
        status::se_requirement(self.se_scan.get(key))
    }

    pub fn dll_version(&self) -> Option<String> {
        self.dll.as_ref().and_then(|d| d.selected.as_ref()).map(|d| d.version.clone())
    }

    pub fn cutoff(&self, settings_cutoff: Option<u64>) -> u64 {
        settings_cutoff.or_else(|| self.dll.as_ref().and_then(|d| d.game_fingerprint).map(|f| f.timestamp as u64)).unwrap_or(0)
    }

    /// Enabled packs that actually exist. Entries for uninstalled packs are skipped at launch
    /// (`ops::list_input_for`), so counting them would overstate what the game will load.
    pub fn enabled_count(&self) -> usize {
        self.count_enabled(&self.active())
    }

    pub fn count_enabled(&self, p: &Profile) -> usize {
        p.entries.iter().filter(|e| e.enabled && !e.is_separator() && self.by_key.contains_key(&e.key)).count()
    }
}
