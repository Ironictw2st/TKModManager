//! Tauri commands for settings, paths, mod discovery, profiles, metadata and the list file.

use crate::json_store;
use crate::meta::MetaDoc;
use crate::modlist::{self, ListInput, ListMod, ParsedList};
use crate::packs::{self, ModEntry, ModSource, PackType};
use crate::paths::{self, GamePaths};
use crate::profiles::{Profile, ProfilesDoc};
use crate::settings::Settings;
use crate::state::{self, AppState};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::State;

#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub fn app_data_dir() -> String {
    paths::app_data_dir().to_string_lossy().into_owned()
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings()
}

#[tauri::command]
pub fn set_settings(state: State<AppState>, settings: Settings) -> Result<(), String> {
    json_store::save(&state::settings_path(), &settings)?;
    if let Ok(mut s) = state.settings.lock() {
        *s = settings;
    }
    Ok(())
}

#[tauri::command]
pub fn get_paths(state: State<AppState>) -> GamePaths {
    state.game_paths()
}

#[tauri::command]
pub fn scan_mods(state: State<AppState>) -> Vec<ModEntry> {
    let p = state.game_paths();
    packs::scan(p.data_dir.as_deref().map(Path::new), p.workshop_dir.as_deref().map(Path::new))
}

#[tauri::command]
pub fn load_profiles() -> Result<ProfilesDoc, String> {
    json_store::load::<ProfilesDoc>(&state::profiles_path())
}

#[tauri::command]
pub fn save_profiles(doc: ProfilesDoc) -> Result<(), String> {
    json_store::save(&state::profiles_path(), &doc)
}

#[tauri::command]
pub fn load_meta() -> Result<MetaDoc, String> {
    json_store::load::<MetaDoc>(&state::meta_path())
}

#[tauri::command]
pub fn save_meta(doc: MetaDoc) -> Result<(), String> {
    json_store::save(&state::meta_path(), &doc)
}

/// Parse a CA-style list file (`used_mods.txt`) into its directives.
#[tauri::command]
pub fn read_mod_list_file(path: String) -> Result<ParsedList, String> {
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
    Ok(modlist::parse(&text))
}

/// One line of an import: which installed pack (if any) a `mod` line resolved to.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImportedEntry {
    pub file: String,
    pub key: Option<String>,
}

/// Resolve the `mod` lines of a CA list against the current scan. A file name is matched
/// first inside the listed working directories (Workshop items), then in data/.
#[tauri::command]
pub fn import_mod_list(state: State<AppState>, path: String) -> Result<Vec<ImportedEntry>, String> {
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
    let parsed = modlist::parse(&text);
    let p = state.game_paths();
    let scan = packs::scan(p.data_dir.as_deref().map(Path::new), p.workshop_dir.as_deref().map(Path::new));
    Ok(resolve_import(&parsed, &scan))
}

pub fn resolve_import(parsed: &ParsedList, scan: &[ModEntry]) -> Vec<ImportedEntry> {
    let norm = |s: &str| s.replace('\\', "/").trim_end_matches('/').to_ascii_lowercase();
    let dirs: Vec<String> = parsed.dirs.iter().map(|d| norm(d)).collect();
    parsed
        .mods
        .iter()
        .map(|file| {
            let candidates: Vec<&ModEntry> =
                scan.iter().filter(|m| m.file.eq_ignore_ascii_case(file)).collect();
            let in_dir = candidates
                .iter()
                .find(|m| dirs.iter().any(|d| *d == norm(&m.dir)))
                .or_else(|| candidates.iter().find(|m| m.source == ModSource::Data))
                .or_else(|| candidates.first());
            ImportedEntry { file: file.clone(), key: in_dir.map(|m| m.key.clone()) }
        })
        .collect()
}

/// Build the list-file input for a profile: enabled packs in order, plus exclusions for
/// every data/ movie pack the profile does not enable.
pub fn list_input_for(profile: &Profile, scan: &[ModEntry]) -> ListInput {
    let by_key: HashMap<&str, &ModEntry> = scan.iter().map(|m| (m.key.as_str(), m)).collect();
    let mut input = ListInput::default();
    for e in profile.entries.iter().filter(|e| e.enabled) {
        if let Some(m) = by_key.get(e.key.as_str()) {
            input.enabled.push(ListMod {
                file: m.file.clone(),
                dir: (m.source == ModSource::Workshop).then(|| m.dir.clone()),
                is_movie: m.pack_type == PackType::Movie,
            });
        }
    }
    let enabled_keys: std::collections::HashSet<&str> =
        profile.entries.iter().filter(|e| e.enabled).map(|e| e.key.as_str()).collect();
    let mut excluded: Vec<String> = scan
        .iter()
        .filter(|m| m.source == ModSource::Data && m.pack_type == PackType::Movie)
        .filter(|m| !enabled_keys.contains(m.key.as_str()))
        .map(|m| m.file.clone())
        .collect();
    excluded.sort_by_key(|f| f.to_ascii_lowercase());
    input.excluded_data_movies = excluded;
    input
}

/// The exact text that would be written for this profile (without launch-time extras).
#[tauri::command]
pub fn preview_mod_list(state: State<AppState>, profile: Profile) -> String {
    let p = state.game_paths();
    let scan = packs::scan(p.data_dir.as_deref().map(Path::new), p.workshop_dir.as_deref().map(Path::new));
    modlist::build(&list_input_for(&profile, &scan))
}

/// Path of the list file inside the game root.
pub fn list_file_path(game_root: &Path) -> PathBuf {
    game_root.join(paths::LIST_FILE_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::ProfileEntry;

    fn entry(key: &str, file: &str, dir: &str, source: ModSource, t: PackType) -> ModEntry {
        ModEntry {
            key: key.into(),
            file: file.into(),
            path: format!("{dir}/{file}"),
            dir: dir.into(),
            source,
            workshop_id: None,
            pack_type: t,
            size: 0,
            mtime: 0,
            preview_path: None,
        }
    }

    #[test]
    fn import_prefers_listed_dirs_then_data() {
        let scan = vec![
            entry("ws:1/a.pack", "a.pack", r"C:\ws\1", ModSource::Workshop, PackType::Mod),
            entry("data:a.pack", "a.pack", r"C:\g\data", ModSource::Data, PackType::Mod),
            entry("data:b.pack", "b.pack", r"C:\g\data", ModSource::Data, PackType::Mod),
        ];
        let parsed = ParsedList {
            dirs: vec!["C:/ws/1".into()],
            mods: vec!["a.pack".into(), "b.pack".into(), "zz.pack".into()],
            excludes: vec![],
        };
        let r = resolve_import(&parsed, &scan);
        assert_eq!(r[0].key.as_deref(), Some("ws:1/a.pack"));
        assert_eq!(r[1].key.as_deref(), Some("data:b.pack"));
        assert_eq!(r[2].key, None);
    }

    #[test]
    fn excludes_disabled_data_movies_only() {
        let scan = vec![
            entry("data:mv.pack", "mv.pack", r"C:\g\data", ModSource::Data, PackType::Movie),
            entry("data:on.pack", "on.pack", r"C:\g\data", ModSource::Data, PackType::Movie),
            entry("ws:2/m2.pack", "m2.pack", r"C:\ws\2", ModSource::Workshop, PackType::Movie),
            entry("ws:3/x.pack", "x.pack", r"C:\ws\3", ModSource::Workshop, PackType::Mod),
        ];
        let profile = Profile {
            name: "p".into(),
            entries: vec![
                ProfileEntry { key: "ws:3/x.pack".into(), enabled: true },
                ProfileEntry { key: "data:on.pack".into(), enabled: true },
                ProfileEntry { key: "ws:2/m2.pack".into(), enabled: true },
                ProfileEntry { key: "gone:none".into(), enabled: true },
            ],
            dll: false,
            skip_intro: false,
        };
        let input = list_input_for(&profile, &scan);
        assert_eq!(input.enabled.len(), 3);
        assert_eq!(input.excluded_data_movies, vec!["mv.pack"]);
        let text = modlist::build(&input);
        assert!(text.contains("add_working_directory \"C:/ws/2\";"));
        assert!(text.contains("mod \"x.pack\";"));
        assert!(!text.contains("mod \"m2.pack\";"));
        assert!(text.contains("exclude_pack_file \"mv.pack\";"));
    }
}
