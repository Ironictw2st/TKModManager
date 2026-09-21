//! Profile / list-file operations shared by the UI: loading and saving the JSON documents,
//! importing CA's `used_mods.txt`, and building the `tkmm_mods.txt` text for a profile.

use crate::context::{self, AppContext};
use crate::json_store;
use crate::meta::MetaDoc;
use crate::modlist::{self, ListInput, ListMod, ParsedList};
use crate::packs::{self, ModEntry, ModSource, PackType};
use crate::paths;
use crate::profiles::{Profile, ProfilesDoc};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn load_profiles(store: paths::GameStore) -> Result<ProfilesDoc, String> {
    json_store::load::<ProfilesDoc>(&context::profiles_path(store))
}

pub fn save_profiles(store: paths::GameStore, doc: &ProfilesDoc) -> Result<(), String> {
    json_store::save(&context::profiles_path(store), doc)
}

pub fn load_meta() -> Result<MetaDoc, String> {
    json_store::load::<MetaDoc>(&context::meta_path())
}

pub fn save_meta(doc: &MetaDoc) -> Result<(), String> {
    json_store::save(&context::meta_path(), doc)
}

/// Parse a CA-style list file (`used_mods.txt`) into its directives.
pub fn read_mod_list_file(path: &str) -> Result<ParsedList, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
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
pub fn import_mod_list(ctx: &AppContext, path: &str) -> Result<Vec<ImportedEntry>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let parsed = modlist::parse(&text);
    let scan = ctx.scan();
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

/// Build the list-file input for a profile: enabled packs in order, plus exclusions for every
/// movie pack the profile does not enable that the engine would auto-load anyway - the ones in
/// data/, and the ones sharing a folder we add as a working directory.
pub fn list_input_for(profile: &Profile, scan: &[ModEntry]) -> ListInput {
    let by_key: HashMap<&str, &ModEntry> = scan.iter().map(|m| (m.key.as_str(), m)).collect();
    let mut input = ListInput::default();
    for e in profile.entries.iter().filter(|e| e.enabled && !e.is_separator()) {
        if let Some(m) = by_key.get(e.key.as_str()) {
            input.enabled.push(ListMod {
                file: m.file.clone(),
                dir: (m.source != ModSource::Data).then(|| m.dir.clone()),
                is_movie: m.pack_type == PackType::Movie,
            });
        }
    }
    let enabled_keys: std::collections::HashSet<&str> =
        profile.entries.iter().filter(|e| e.enabled).map(|e| e.key.as_str()).collect();
    // Folders that end up as working directories expose every movie pack inside them, so a
    // disabled movie pack sharing an extra folder with an enabled pack must be excluded too.
    let added_dirs: std::collections::HashSet<String> =
        input.enabled.iter().filter_map(|m| m.dir.as_deref()).map(packs::norm_dir).collect();
    let mut excluded: Vec<String> = scan
        .iter()
        .filter(|m| m.pack_type == PackType::Movie)
        .filter(|m| match m.source {
            ModSource::Data => true,
            // A Workshop item folder becomes a working directory exactly like an extra folder
            // does, so a disabled movie pack sitting next to an enabled pack in the same item
            // would load regardless. Exclude it by name; the file itself is never touched.
            ModSource::Workshop | ModSource::Folder => added_dirs.contains(&packs::norm_dir(&m.dir)),
        })
        .filter(|m| !enabled_keys.contains(m.key.as_str()))
        .map(|m| m.file.clone())
        .collect();
    excluded.sort_by_key(|f| f.to_ascii_lowercase());
    input.excluded_data_movies = excluded;
    input
}

/// The exact text that would be written for this profile (without launch-time extras).
pub fn preview_mod_list(ctx: &AppContext, profile: &Profile) -> String {
    let scan = ctx.scan();
    modlist::build(&list_input_for(profile, &scan))
}

/// Path of the list file inside the game root.
pub fn list_file_path(game_root: &Path) -> PathBuf {
    game_root.join(paths::LIST_FILE_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::ProfileEntry;

    fn pe(key: &str) -> ProfileEntry {
        ProfileEntry { key: key.into(), enabled: true, label: None, collapsed: false }
    }

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
            installed_updated: None,
            latest_updated: None,
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
                pe("ws:3/x.pack"),
                pe("data:on.pack"),
                pe("ws:2/m2.pack"),
                pe("gone:none"),
                pe("sep:1234"),
            ],
            dll: false,
            skip_intro: false,
            last_played: None,
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

    /// A Workshop item that ships a mod pack and a movie pack: enabling only the mod still adds
    /// the item folder as a working directory, so the disabled movie has to be excluded by name.
    #[test]
    fn excludes_disabled_movies_in_an_added_workshop_folder() {
        let scan = vec![
            entry("ws:7/thing.pack", "thing.pack", r"C:\ws\7", ModSource::Workshop, PackType::Mod),
            entry("ws:7/thing_movie.pack", "thing_movie.pack", r"C:\ws\7", ModSource::Workshop, PackType::Movie),
            // Same item disabled entirely: its folder is never added, so nothing to exclude.
            entry("ws:8/other_movie.pack", "other_movie.pack", r"C:\ws\8", ModSource::Workshop, PackType::Movie),
        ];
        let profile = Profile {
            name: "p".into(),
            entries: vec![
                pe("ws:7/thing.pack"),
                ProfileEntry { key: "ws:7/thing_movie.pack".into(), enabled: false, label: None, collapsed: false },
                ProfileEntry { key: "ws:8/other_movie.pack".into(), enabled: false, label: None, collapsed: false },
            ],
            dll: false,
            skip_intro: false,
            last_played: None,
        };
        let input = list_input_for(&profile, &scan);
        assert_eq!(input.excluded_data_movies, vec!["thing_movie.pack"]);
        let text = modlist::build(&input);
        assert!(text.contains("add_working_directory \"C:/ws/7\";"));
        assert!(text.contains("exclude_pack_file \"thing_movie.pack\";"));
        assert!(!text.contains("other_movie.pack"));
    }
}
