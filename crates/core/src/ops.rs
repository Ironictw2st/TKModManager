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

/// How recent a copy of a pack is: the newest of its file time, the Workshop publish time and
/// the Nexus upload time. Used to pick between copies that share a file name.
pub fn freshness(m: &ModEntry) -> u64 {
    let nexus = m.nexus.as_ref().map(|n| n.active.uploaded).unwrap_or(0);
    m.mtime.max(m.installed_updated.unwrap_or(0)).max(nexus)
}

/// The enabled packs of a profile in order, one per file name. The game resolves `mod` lines by
/// name only, so two enabled copies of `x.pack` (Workshop + Nexus, two Workshop items, an old
/// Nexus slot) would leave the pick to the engine - usually the older folder, listed first. The
/// newest copy is kept, at the place of the first one.
pub fn enabled_packs<'a>(profile: &Profile, scan: &'a [ModEntry]) -> Vec<&'a ModEntry> {
    let by_key: HashMap<&str, &ModEntry> = scan.iter().map(|m| (m.key.as_str(), m)).collect();
    let mut out: Vec<&ModEntry> = Vec::new();
    for e in profile.entries.iter().filter(|e| e.enabled && !e.is_separator()) {
        let Some(&m) = by_key.get(e.key.as_str()) else { continue };
        match out.iter_mut().find(|o| o.file.eq_ignore_ascii_case(&m.file)) {
            Some(o) if freshness(m) > freshness(o) => *o = m,
            Some(_) => {}
            None => out.push(m),
        }
    }
    out
}

/// A pack the profile loads that has other copies on disk under the same file name.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Clash {
    pub file: String,
    /// The copy that goes into the list file.
    pub loaded: String,
    /// Other copies, with whether each is newer than the loaded one.
    pub others: Vec<ClashCopy>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ClashCopy {
    pub key: String,
    pub source: ModSource,
    pub path: String,
    pub newer: bool,
}

impl Clash {
    /// Copies in data/ can shadow the loaded one: data/ is never a listed folder, so the list
    /// cannot say which copy it means.
    pub fn data_copies(&self) -> impl Iterator<Item = &ClashCopy> {
        self.others.iter().filter(|c| c.source == ModSource::Data)
    }
}

/// Every pack the profile loads that shares its file name with another pack on disk.
pub fn name_clashes(profile: &Profile, scan: &[ModEntry]) -> Vec<Clash> {
    enabled_packs(profile, scan)
        .into_iter()
        .filter_map(|m| {
            let others: Vec<ClashCopy> = scan
                .iter()
                .filter(|o| o.key != m.key && o.file.eq_ignore_ascii_case(&m.file))
                .map(|o| ClashCopy { key: o.key.clone(), source: o.source, path: o.path.clone(), newer: freshness(o) > freshness(m) })
                .collect();
            (!others.is_empty()).then(|| Clash { file: m.file.clone(), loaded: m.key.clone(), others })
        })
        .collect()
}

/// Build the list-file input for a profile: enabled packs in order, plus exclusions for every
/// movie pack the profile does not enable that the engine would auto-load anyway - the ones in
/// data/, and the ones sharing a folder we add as a working directory.
pub fn list_input_for(profile: &Profile, scan: &[ModEntry]) -> ListInput {
    let mut input = ListInput::default();
    let enabled = enabled_packs(profile, scan);
    for m in &enabled {
        input.enabled.push(ListMod {
            file: m.file.clone(),
            dir: (m.source != ModSource::Data).then(|| m.dir.clone()),
            is_movie: m.pack_type == PackType::Movie,
        });
    }
    let enabled_keys: std::collections::HashSet<&str> = enabled.iter().map(|m| m.key.as_str()).collect();
    // Exclusions are by name, so a stale copy must not exclude the enabled one.
    let enabled_files: std::collections::HashSet<String> = enabled.iter().map(|m| m.file.to_ascii_lowercase()).collect();
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
            ModSource::Workshop | ModSource::Folder | ModSource::Nexus => added_dirs.contains(&packs::norm_dir(&m.dir)),
        })
        .filter(|m| !enabled_keys.contains(m.key.as_str()) && !enabled_files.contains(&m.file.to_ascii_lowercase()))
        .map(|m| m.file.clone())
        .collect();
    excluded.sort_by_key(|f| f.to_ascii_lowercase());
    excluded.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
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
            nexus: None,
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

    /// Nexus installs load in place from the manager's folder, like extra folders.
    #[test]
    fn nexus_packs_load_from_their_folder() {
        let dir = r"C:\appdata\TKModManager\nexus\77\200";
        let scan = vec![
            entry("nx:77/cool.pack", "cool.pack", dir, ModSource::Nexus, PackType::Mod),
            entry("nx:77/cool_movie.pack", "cool_movie.pack", dir, ModSource::Nexus, PackType::Movie),
        ];
        let profile = Profile {
            name: "p".into(),
            entries: vec![pe("nx:77/cool.pack"), ProfileEntry { key: "nx:77/cool_movie.pack".into(), enabled: false, label: None, collapsed: false }],
            dll: false,
            skip_intro: false,
            last_played: None,
        };
        let text = modlist::build(&list_input_for(&profile, &scan));
        assert!(text.contains("add_working_directory \"C:/appdata/TKModManager/nexus/77/200\";"), "{text}");
        assert!(text.contains("mod \"cool.pack\";"));
        assert!(text.contains("exclude_pack_file \"cool_movie.pack\";"));
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

    fn profile_of(entries: Vec<ProfileEntry>) -> Profile {
        Profile { name: "p".into(), entries, dll: false, skip_intro: false, last_played: None }
    }

    /// Two enabled copies of one pack name (an old Nexus slot and the Workshop item): only the
    /// newest is listed, at the first copy's place, and the stale copy's folder is not added.
    #[test]
    fn two_enabled_copies_keep_only_the_newest() {
        let mut old = entry("nx:local-190/!!190.pack", "!!190.pack", r"C:\nx\local-190\l1", ModSource::Nexus, PackType::Mod);
        old.mtime = 100;
        let mut new = entry("ws:9/!!190.PACK", "!!190.PACK", r"C:\ws\9", ModSource::Workshop, PackType::Mod);
        new.mtime = 50;
        new.installed_updated = Some(500);
        let other = entry("ws:3/x.pack", "x.pack", r"C:\ws\3", ModSource::Workshop, PackType::Mod);
        let scan = vec![old, new, other];
        let profile = profile_of(vec![pe("nx:local-190/!!190.pack"), pe("ws:3/x.pack"), pe("ws:9/!!190.PACK")]);
        let text = modlist::build(&list_input_for(&profile, &scan));
        let expected = [
            "add_working_directory \"C:/ws/9\";",
            "add_working_directory \"C:/ws/3\";",
            "mod \"!!190.PACK\";",
            "mod \"x.pack\";",
            "",
        ]
        .join("\n");
        assert_eq!(text, expected);
    }

    /// A stale disabled movie copy must not exclude, by name, the enabled copy elsewhere.
    #[test]
    fn stale_movie_copy_does_not_exclude_the_enabled_one() {
        let scan = vec![
            entry("data:mv.pack", "mv.pack", r"C:\g\data", ModSource::Data, PackType::Movie),
            entry("ws:4/mv.pack", "mv.pack", r"C:\ws\4", ModSource::Workshop, PackType::Movie),
        ];
        let profile = profile_of(vec![ProfileEntry { key: "data:mv.pack".into(), enabled: false, label: None, collapsed: false }, pe("ws:4/mv.pack")]);
        let text = modlist::build(&list_input_for(&profile, &scan));
        assert!(text.contains("add_working_directory \"C:/ws/4\";"), "{text}");
        assert!(!text.contains("exclude_pack_file"), "{text}");
    }

    #[test]
    fn clashes_report_data_copies_of_loaded_packs() {
        let mut data = entry("data:a.pack", "a.pack", r"C:\g\data", ModSource::Data, PackType::Mod);
        data.mtime = 10;
        let mut ws = entry("ws:1/a.pack", "a.pack", r"C:\ws\1", ModSource::Workshop, PackType::Mod);
        ws.mtime = 20;
        let lone = entry("ws:2/b.pack", "b.pack", r"C:\ws\2", ModSource::Workshop, PackType::Mod);
        let unused = entry("data:c.pack", "c.pack", r"C:\g\data", ModSource::Data, PackType::Mod);
        let scan = vec![data, ws, lone, unused.clone(), ModEntry { key: "ws:3/c.pack".into(), ..unused }];
        let profile = profile_of(vec![pe("ws:1/a.pack"), pe("ws:2/b.pack")]);
        let clashes = name_clashes(&profile, &scan);
        assert_eq!(clashes.len(), 1);
        assert_eq!(clashes[0].loaded, "ws:1/a.pack");
        let data: Vec<_> = clashes[0].data_copies().collect();
        assert_eq!(data.len(), 1);
        assert!(!data[0].newer);
    }
}
