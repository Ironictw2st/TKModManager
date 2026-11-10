//! File-path collisions between enabled packs. Each pack's file index is read lazily through
//! rpfm_lib (header + index only) and cached by (path, size, mtime).
//!
//! Precedence model: vanilla < mod packs in list order < movie packs; the last pack wins.

use crate::packs::{ModEntry, PackType};
use rpfm_lib::files::pack::Pack;
use rpfm_lib::files::Container;
use rpfm_lib::games::supported_games::{SupportedGames, KEY_THREE_KINGDOMS};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
    /// Path inside the packs (forward slashes, lowercase).
    pub path: String,
    /// Keys of the packs that contain it, in precedence order (last wins).
    pub packs: Vec<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConflictReport {
    pub conflicts: Vec<Conflict>,
    /// Per pack key: number of conflicting paths it takes part in.
    pub per_pack: BTreeMap<String, usize>,
    /// Packs that could not be read (key -> error).
    pub errors: BTreeMap<String, String>,
}

#[derive(Clone)]
struct IndexEntry {
    size: u64,
    mtime: u64,
    paths: Vec<String>,
}

#[derive(Default)]
pub struct ConflictCache(Mutex<HashMap<String, IndexEntry>>);

fn read_paths(path: &Path) -> Result<Vec<String>, String> {
    let games = SupportedGames::default();
    let game = games.game(KEY_THREE_KINGDOMS).ok_or("rpfm: unknown game key")?;
    let pack = Pack::read_and_merge(&[PathBuf::from(path)], game, true, false, false).map_err(|e| e.to_string())?;
    let mut out: Vec<String> = pack
        .paths_raw()
        .into_iter()
        .map(|p| p.replace('\\', "/").to_ascii_lowercase())
        .filter(|p| !p.contains(".rpfm_reserved"))
        .collect();
    out.sort();
    out.dedup();
    Ok(out)
}

/// Compute collisions among `ordered` (already in load order, movies last).
pub fn report(cache: &ConflictCache, ordered: &[ModEntry]) -> ConflictReport {
    let mut owners: HashMap<String, Vec<String>> = HashMap::new();
    let mut rep = ConflictReport::default();
    for m in ordered {
        let paths = {
            let mut c = cache.0.lock().unwrap();
            let fresh = c.get(&m.key).filter(|e| e.size == m.size && e.mtime == m.mtime).cloned();
            match fresh {
                Some(e) => e.paths,
                None => match read_paths(Path::new(&m.path)) {
                    Ok(paths) => {
                        c.insert(m.key.clone(), IndexEntry { size: m.size, mtime: m.mtime, paths: paths.clone() });
                        paths
                    }
                    Err(e) => {
                        rep.errors.insert(m.key.clone(), e);
                        continue;
                    }
                },
            }
        };
        for p in paths {
            owners.entry(p).or_default().push(m.key.clone());
        }
    }
    let mut conflicts: Vec<Conflict> = owners
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(path, packs)| Conflict { path, packs })
        .collect();
    conflicts.sort_by(|a, b| a.path.cmp(&b.path));
    for c in &conflicts {
        for k in &c.packs {
            *rep.per_pack.entry(k.clone()).or_default() += 1;
        }
    }
    rep.conflicts = conflicts;
    rep
}

/// What kind of game file a path is, from its top folder. Decides how much a conflict matters.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FileKind {
    /// `db/`: game data tables. The losing mod's rows in that table file are gone.
    Data,
    /// `script/`: Lua. The losing mod's script is replaced.
    Scripts,
    /// `ui/`: layouts and interface art.
    Interface,
    /// `text/`: names and descriptions.
    Text,
    /// Models, animations, textures, effects, maps.
    Art,
    Other,
}

impl FileKind {
    pub fn of(path: &str) -> FileKind {
        match path.split('/').next().unwrap_or("") {
            "db" => FileKind::Data,
            "script" => FileKind::Scripts,
            "ui" => FileKind::Interface,
            "text" => FileKind::Text,
            "variantmeshes" | "animations" | "models" | "terrain" | "vfx" | "prefabs" | "shaders" | "particles" | "audio" | "movies" => FileKind::Art,
            _ => FileKind::Other,
        }
    }

    pub fn label(self) -> String {
        match self {
            FileKind::Data => crate::t!("conflicts-kind-data"),
            FileKind::Scripts => crate::t!("conflicts-kind-scripts"),
            FileKind::Interface => crate::t!("conflicts-kind-interface"),
            FileKind::Text => crate::t!("conflicts-kind-text"),
            FileKind::Art => crate::t!("conflicts-kind-art"),
            FileKind::Other => crate::t!("conflicts-kind-other"),
        }
    }

    /// Losing these can break part of the losing mod.
    pub fn matters(self) -> bool {
        matches!(self, FileKind::Data | FileKind::Scripts)
    }
}

/// One mod replacing another's files: `winner` loads later, so the game uses its copies.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct PairSummary {
    pub winner: String,
    pub loser: String,
    pub files: usize,
    /// Files per kind, most first.
    pub kinds: Vec<(FileKind, usize)>,
    /// Game data or scripts are involved: worth a look.
    pub check: bool,
}

/// Conflicts grouped by (winner, loser) pair: pairs worth a look first, then by file count.
/// A file in three packs counts once for each pack it overrides.
pub fn summarize(rep: &ConflictReport) -> Vec<PairSummary> {
    let mut pairs: HashMap<(&str, &str), BTreeMap<FileKind, usize>> = HashMap::new();
    for c in &rep.conflicts {
        let Some((winner, losers)) = c.packs.split_last() else { continue };
        let kind = FileKind::of(&c.path);
        for loser in losers {
            *pairs.entry((winner.as_str(), loser.as_str())).or_default().entry(kind).or_default() += 1;
        }
    }
    let mut out: Vec<PairSummary> = pairs
        .into_iter()
        .map(|((w, l), kinds)| {
            let files = kinds.values().sum();
            let check = kinds.keys().any(|k| k.matters());
            let mut kinds: Vec<(FileKind, usize)> = kinds.into_iter().collect();
            kinds.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            PairSummary { winner: w.to_string(), loser: l.to_string(), files, kinds, check }
        })
        .collect();
    out.sort_by(|a, b| b.check.cmp(&a.check).then(b.files.cmp(&a.files)).then(a.winner.cmp(&b.winner)).then(a.loser.cmp(&b.loser)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(path: &str, packs: &[&str]) -> Conflict {
        Conflict { path: path.into(), packs: packs.iter().map(|s| s.to_string()).collect() }
    }

    #[test]
    fn pairs_by_winner_and_loser_data_first() {
        let rep = ConflictReport {
            conflicts: vec![
                c("animations/a.anim", &["big", "fix"]),
                c("animations/b.anim", &["big", "fix"]),
                c("variantmeshes/c.rigid", &["big", "fix"]),
                c("db/units_tables/x", &["base", "big", "fix"]),
                c("text/db/names.loc", &["base", "fix"]),
            ],
            ..Default::default()
        };
        let s = summarize(&rep);
        // Only the final winner counts: fix > big (three art files + the db table) and
        // fix > base (the db table + one text file). big never wins a file here.
        assert_eq!(s.len(), 2);
        assert_eq!((s[0].winner.as_str(), s[0].loser.as_str(), s[0].files, s[0].check), ("fix", "big", 4, true));
        assert_eq!(s[0].kinds, vec![(FileKind::Art, 3), (FileKind::Data, 1)]);
        assert_eq!((s[1].winner.as_str(), s[1].loser.as_str(), s[1].files), ("fix", "base", 2));
        assert!(s[1].check);
    }

    #[test]
    fn art_only_is_usually_fine() {
        let rep = ConflictReport { conflicts: vec![c("models/x.dds", &["a", "b"]), c("ui/skins/y.png", &["a", "b"])], ..Default::default() };
        let s = summarize(&rep);
        assert_eq!(s.len(), 1);
        assert!(!s[0].check);
        assert_eq!(FileKind::of("script/campaign/mod/x.lua"), FileKind::Scripts);
        assert_eq!(FileKind::of("readme.txt"), FileKind::Other);
    }
}

/// `keys` = enabled pack keys in profile order. Movie packs are moved last (engine order).
pub fn conflicts_for(ctx: &crate::context::AppContext, keys: &[String]) -> ConflictReport {
    let cache = &ctx.conflicts;
    let scan = ctx.scan();
    let by_key: HashMap<&str, &ModEntry> = scan.iter().map(|m| (m.key.as_str(), m)).collect();
    let mut ordered: Vec<ModEntry> = keys.iter().filter_map(|k| by_key.get(k.as_str()).map(|m| (*m).clone())).collect();
    let (mut mods, mut movies): (Vec<ModEntry>, Vec<ModEntry>) =
        ordered.drain(..).partition(|m| m.pack_type != PackType::Movie);
    movies.sort_by_key(|m| m.file.to_ascii_lowercase());
    mods.append(&mut movies);
    report(cache, &mods)
}
