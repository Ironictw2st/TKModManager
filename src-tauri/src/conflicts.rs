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
use tauri::State;

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

/// `keys` = enabled pack keys in profile order. Movie packs are moved last (engine order).
#[tauri::command]
pub fn conflicts_for(state: State<crate::state::AppState>, cache: State<ConflictCache>, keys: Vec<String>) -> ConflictReport {
    let scan = state.scan();
    let by_key: HashMap<&str, &ModEntry> = scan.iter().map(|m| (m.key.as_str(), m)).collect();
    let mut ordered: Vec<ModEntry> = keys.iter().filter_map(|k| by_key.get(k.as_str()).map(|m| (*m).clone())).collect();
    let (mut mods, mut movies): (Vec<ModEntry>, Vec<ModEntry>) =
        ordered.drain(..).partition(|m| m.pack_type != PackType::Movie);
    movies.sort_by_key(|m| m.file.to_ascii_lowercase());
    mods.append(&mut movies);
    report(&cache, &mods)
}
