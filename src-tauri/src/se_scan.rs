//! Does a pack need the script extender? Two automatic signals:
//!   1. a marker file `script/tkmm/requires_script_extender` (optional `min_version=X.Y.Z` line);
//!      it is outside every mod-loader folder, so the game never runs it;
//!   2. Lua under `script/` that calls the extender API (`se.modify.`, `se.query.`, ...).
//! Results are cached by (path, size, mtime) in `se_scan.cache.json`. The manual override lives
//! in the per-mod metadata and is applied by the frontend.

use crate::json_store;
use crate::packs::ModEntry;
use crate::paths;
use crate::state::AppState;
use rpfm_lib::files::pack::Pack;
use rpfm_lib::files::Container;
use rpfm_lib::games::supported_games::{SupportedGames, KEY_THREE_KINGDOMS};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub const MARKER_PATH: &str = "script/tkmm/requires_script_extender";

/// Patterns that only appear in code using the extender's public Lua API.
const LUA_PATTERNS: [&str; 5] = ["se.modify.", "se.query.", "se.version(", "type(se)", "se.available("];

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeInfo {
    pub required: bool,
    /// "marker" | "lua" | "" (not required)
    pub source: String,
    /// Which file triggered it (marker path or the first matching Lua file).
    pub detail: String,
    pub min_version: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct CacheEntry {
    size: u64,
    mtime: u64,
    info: SeInfo,
}

#[derive(Serialize, Deserialize, Default)]
struct CacheDoc {
    #[serde(default)]
    packs: HashMap<String, CacheEntry>,
}

fn cache_path() -> PathBuf {
    paths::app_data_dir().join("se_scan.cache.json")
}

/// `min_version=0.23.0` anywhere in the marker text.
pub fn parse_marker(text: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let (k, v) = l.split_once('=')?;
        (k.trim() == "min_version").then(|| v.trim().trim_start_matches('v').to_string()).filter(|v| !v.is_empty())
    })
}

/// Strip `--` comments so commented-out calls do not count.
pub fn lua_uses_extender(source: &str) -> bool {
    source.lines().any(|line| {
        let code = line.split("--").next().unwrap_or("");
        LUA_PATTERNS.iter().any(|p| code.contains(p))
    })
}

fn file_text(pack: &mut Pack, path: &str) -> Option<String> {
    let mut rfile = pack.file(path, true)?.clone();
    rfile.load().ok()?;
    let bytes = rfile.cached().ok()?;
    Some(String::from_utf8_lossy(bytes).into_owned())
}

pub fn scan_pack(path: &Path) -> Result<SeInfo, String> {
    let games = SupportedGames::default();
    let game = games.game(KEY_THREE_KINGDOMS).ok_or("rpfm: unknown game key")?;
    let mut pack = Pack::read_and_merge(&[path.to_path_buf()], game, true, false, false).map_err(|e| e.to_string())?;
    let paths: Vec<String> = pack.paths_raw().into_iter().map(|p| p.replace('\\', "/")).collect();

    if let Some(marker) = paths.iter().find(|p| p.eq_ignore_ascii_case(MARKER_PATH)) {
        let text = file_text(&mut pack, marker).unwrap_or_default();
        return Ok(SeInfo { required: true, source: "marker".into(), detail: marker.clone(), min_version: parse_marker(&text) });
    }
    for p in paths.iter().filter(|p| {
        let l = p.to_ascii_lowercase();
        l.starts_with("script/") && l.ends_with(".lua")
    }) {
        if let Some(text) = file_text(&mut pack, p) {
            if lua_uses_extender(&text) {
                return Ok(SeInfo { required: true, source: "lua".into(), detail: p.clone(), min_version: None });
            }
        }
    }
    Ok(SeInfo::default())
}

/// Scan the packs with these keys (all packs when empty). Unreadable packs count as "not required".
#[tauri::command]
pub async fn se_requirements(app: AppHandle, keys: Vec<String>) -> Result<HashMap<String, SeInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let scan = app.state::<AppState>().scan();
        let wanted: Vec<&ModEntry> = if keys.is_empty() { scan.iter().collect() } else { scan.iter().filter(|m| keys.contains(&m.key)).collect() };
        let mut cache: CacheDoc = json_store::load(&cache_path()).unwrap_or_default();
        let mut out = HashMap::new();
        let mut dirty = false;
        for m in wanted {
            let hit = cache.packs.get(&m.path).filter(|c| c.size == m.size && c.mtime == m.mtime).map(|c| c.info.clone());
            let info = match hit {
                Some(i) => i,
                None => {
                    let info = scan_pack(Path::new(&m.path)).unwrap_or_else(|e| {
                        eprintln!("se_scan: {}: {e}", m.path);
                        SeInfo::default()
                    });
                    cache.packs.insert(m.path.clone(), CacheEntry { size: m.size, mtime: m.mtime, info: info.clone() });
                    dirty = true;
                    info
                }
            };
            out.insert(m.key.clone(), info);
        }
        if dirty {
            let _ = json_store::save(&cache_path(), &cache);
        }
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use rpfm_lib::files::{FileType, RFile};
    use rpfm_lib::games::pfh_file_type::PFHFileType;
    use rpfm_lib::games::pfh_version::PFHVersion;

    #[test]
    fn marker_min_version() {
        assert_eq!(parse_marker("# needs SE\nmin_version = v0.23.0\n").as_deref(), Some("0.23.0"));
        assert_eq!(parse_marker(""), None);
        assert_eq!(parse_marker("min_version="), None);
    }

    #[test]
    fn lua_detection_ignores_comments() {
        assert!(lua_uses_extender("local ok = se.modify.recruit(cqi, key)"));
        assert!(lua_uses_extender("if type(se) == \"table\" then"));
        assert!(!lua_uses_extender("-- se.modify.recruit(cqi, key)\nlocal base = 1"));
        assert!(!lua_uses_extender("local house = 3 -- nothing here"));
    }

    fn write_pack(dir: &Path, name: &str, files: &[(&str, &str)]) -> PathBuf {
        let games = SupportedGames::default();
        let game = games.game(KEY_THREE_KINGDOMS).unwrap();
        let mut pack = Pack::new_with_name_and_version(name, PFHVersion::PFH5);
        pack.set_pfh_file_type(PFHFileType::Mod);
        for (p, text) in files {
            pack.insert(RFile::new_from_vec(text.as_bytes(), FileType::Text, 0, p)).unwrap();
        }
        let out = dir.join(name);
        pack.save(Some(&out), game, &None).unwrap();
        out
    }

    #[test]
    fn scans_real_packs() {
        let tmp = tempfile::tempdir().unwrap();
        let marker = write_pack(tmp.path(), "m.pack", &[(MARKER_PATH, "min_version=0.23.0")]);
        let info = scan_pack(&marker).unwrap();
        assert!(info.required && info.source == "marker");
        assert_eq!(info.min_version.as_deref(), Some("0.23.0"));

        let lua = write_pack(tmp.path(), "l.pack", &[("script/campaign/mod/x.lua", "se.query.character(1)")]);
        let info = scan_pack(&lua).unwrap();
        assert!(info.required && info.source == "lua");
        assert_eq!(info.detail, "script/campaign/mod/x.lua");

        let plain = write_pack(tmp.path(), "p.pack", &[("script/campaign/mod/y.lua", "cm:callback(f, 1)")]);
        assert_eq!(scan_pack(&plain).unwrap(), SeInfo::default());
    }
}
