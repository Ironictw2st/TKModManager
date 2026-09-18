//! Does a pack need the script extender? A pack declares it by shipping
//! `SE/script_extender.json`:
//!
//! ```text
//! { "author": "Ironic", "minimum_version": 0.28, "maximum_version": 0.28, "notes": "" }
//! ```
//!
//! Every field is optional. The file is read tolerantly (trailing commas, BOM) and version
//! numbers keep their source text, so `0.30` is minor version 30, never `0.3`. Results are
//! cached by (path, size, mtime) in `se_manifest.cache.json`. The manual override lives in the
//! per-mod metadata and is applied by the frontend.

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

pub const MANIFEST_PATH: &str = "SE/script_extender.json";

/// Keys whose numeric values are versions (kept as text).
const VERSION_KEYS: [&str; 2] = ["minimum_version", "maximum_version"];

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeInfo {
    pub required: bool,
    /// Path of the manifest inside the pack, as stored.
    pub path: String,
    pub author: Option<String>,
    pub min_version: Option<String>,
    pub max_version: Option<String>,
    pub notes: Option<String>,
    /// Set when the manifest exists but could not be read.
    pub error: Option<String>,
}

#[derive(Deserialize, Default)]
struct RawManifest {
    #[serde(default)]
    author: Option<serde_json::Value>,
    #[serde(default)]
    minimum_version: Option<serde_json::Value>,
    #[serde(default)]
    maximum_version: Option<serde_json::Value>,
    #[serde(default)]
    notes: Option<serde_json::Value>,
}

/// Make the manifest acceptable to strict JSON: drop a BOM and trailing commas, and turn the bare
/// numbers of the version keys into strings so their exact text survives (`0.30` != `0.3`).
pub fn normalize(text: &str) -> String {
    let chars: Vec<char> = text.trim_start_matches('\u{feff}').chars().collect();
    let mut out = String::with_capacity(chars.len() + 8);
    let mut i = 0;
    let mut last_key: Option<String> = None;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            // Copy the whole string literal, remembering it as a potential key.
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i = (i + 1).min(chars.len());
            let lit: String = chars[start..i].iter().collect();
            last_key = Some(lit.trim_matches('"').to_string());
            out.push_str(&lit);
            continue;
        }
        if c == ':' {
            out.push(c);
            i += 1;
            let is_version = last_key.as_deref().map(|k| VERSION_KEYS.contains(&k)).unwrap_or(false);
            if is_version {
                while i < chars.len() && chars[i].is_whitespace() {
                    out.push(chars[i]);
                    i += 1;
                }
                if i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    let start = i;
                    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                        i += 1;
                    }
                    let num: String = chars[start..i].iter().collect();
                    out.push('"');
                    out.push_str(&num);
                    out.push('"');
                }
            }
            continue;
        }
        if c == ',' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                i += 1; // trailing comma
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

fn as_text(v: Option<serde_json::Value>) -> Option<String> {
    let s = match v? {
        serde_json::Value::String(s) => s,
        serde_json::Value::Number(n) => n.to_string(),
        _ => return None,
    };
    let s = s.trim().to_string();
    (!s.is_empty()).then_some(s)
}

fn as_version(v: Option<serde_json::Value>) -> Option<String> {
    as_text(v).map(|s| s.trim_start_matches(['v', 'V']).to_string()).filter(|s| !s.is_empty())
}

/// Parse the manifest text. A broken file still means "requires the extender".
pub fn parse_manifest(text: &str, path: &str) -> SeInfo {
    let mut info = SeInfo { required: true, path: path.to_string(), ..Default::default() };
    match serde_json::from_str::<RawManifest>(&normalize(text)) {
        Ok(m) => {
            info.author = as_text(m.author);
            info.min_version = as_version(m.minimum_version);
            info.max_version = as_version(m.maximum_version);
            info.notes = as_text(m.notes);
        }
        Err(e) => info.error = Some(format!("{MANIFEST_PATH} is not valid JSON: {e}")),
    }
    info
}

pub fn scan_pack(path: &Path) -> Result<SeInfo, String> {
    let games = SupportedGames::default();
    let game = games.game(KEY_THREE_KINGDOMS).ok_or("rpfm: unknown game key")?;
    let pack = Pack::read_and_merge(&[path.to_path_buf()], game, true, false, false).map_err(|e| e.to_string())?;
    let found = pack
        .paths_raw()
        .into_iter()
        .find(|p| p.replace('\\', "/").eq_ignore_ascii_case(MANIFEST_PATH))
        .map(str::to_string);
    let Some(inner) = found else { return Ok(SeInfo::default()) };
    let text = pack
        .file(&inner, true)
        .cloned()
        .and_then(|mut f| {
            f.load().ok()?;
            f.cached().ok().map(|b| String::from_utf8_lossy(b).into_owned())
        });
    Ok(match text {
        Some(t) => parse_manifest(&t, &inner.replace('\\', "/")),
        None => SeInfo {
            required: true,
            path: inner.replace('\\', "/"),
            error: Some(format!("could not read {MANIFEST_PATH} from the pack")),
            ..Default::default()
        },
    })
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
    paths::app_data_dir().join("se_manifest.cache.json")
}

/// Read the manifests of the packs with these keys (all packs when empty). Unreadable packs
/// count as "not required".
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
    fn parses_the_documented_sample_with_trailing_comma() {
        let text = "{    \"author\": \"Ironic\",    \"minimum_version\": 0.28,    \"maximum_version\": 0.28,    \"notes\": \"\",}";
        let info = parse_manifest(text, MANIFEST_PATH);
        assert!(info.required);
        assert_eq!(info.author.as_deref(), Some("Ironic"));
        assert_eq!(info.min_version.as_deref(), Some("0.28"));
        assert_eq!(info.max_version.as_deref(), Some("0.28"));
        assert_eq!(info.notes, None);
        assert_eq!(info.error, None);
    }

    #[test]
    fn keeps_version_text_exactly() {
        let info = parse_manifest("{\"minimum_version\": 0.30, \"maximum_version\": \"v0.31.2\"}", MANIFEST_PATH);
        assert_eq!(info.min_version.as_deref(), Some("0.30"));
        assert_eq!(info.max_version.as_deref(), Some("0.31.2"));
    }

    #[test]
    fn optional_fields_bom_and_strings_with_commas() {
        let info = parse_manifest("\u{feff}{ \"notes\": \"needs SE, v1 era\", }", MANIFEST_PATH);
        assert!(info.required && info.error.is_none());
        assert_eq!(info.notes.as_deref(), Some("needs SE, v1 era"));
        assert_eq!(info.min_version, None);
        let empty = parse_manifest("{}", MANIFEST_PATH);
        assert!(empty.required && empty.error.is_none());
    }

    #[test]
    fn broken_json_still_requires_with_error() {
        let info = parse_manifest("{ author: Ironic }", MANIFEST_PATH);
        assert!(info.required);
        assert!(info.error.is_some());
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

    /// Minimal PFH5 mod pack written by hand (no rpfm), like third-party tools produce.
    fn raw_pack(dir: &Path, name: &str, files: &[(&str, &str)]) -> PathBuf {
        let mut index = Vec::new();
        let mut data = Vec::new();
        for (p, body) in files {
            index.extend_from_slice(&(body.len() as u32).to_le_bytes());
            index.push(0); // not compressed
            index.extend_from_slice(p.replace('/', "\\").as_bytes());
            index.push(0);
            data.extend_from_slice(body.as_bytes());
        }
        let mut out = b"PFH5".to_vec();
        for v in [3u32, 0, 0, files.len() as u32, index.len() as u32, 0x7fff_ffff] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&index);
        out.extend_from_slice(&data);
        let path = dir.join(name);
        std::fs::write(&path, out).unwrap();
        path
    }

    #[test]
    fn scans_hand_written_packs() {
        let tmp = tempfile::tempdir().unwrap();
        let sample = "{    \"author\": \"Ironic\",    \"minimum_version\": 0.28,    \"maximum_version\": 0.28,    \"notes\": \"\",}";
        let info = scan_pack(&raw_pack(tmp.path(), "s.pack", &[(MANIFEST_PATH, sample)])).unwrap();
        assert_eq!((info.required, info.author.as_deref(), info.min_version.as_deref(), info.max_version.as_deref()), (true, Some("Ironic"), Some("0.28"), Some("0.28")));
        assert_eq!(info.path, MANIFEST_PATH);

        let upper = scan_pack(&raw_pack(tmp.path(), "u.pack", &[("se/SCRIPT_EXTENDER.JSON", "{\"maximum_version\":0.20}")])).unwrap();
        assert!(upper.required);
        assert_eq!(upper.max_version.as_deref(), Some("0.20"));

        let broken = scan_pack(&raw_pack(tmp.path(), "b.pack", &[(MANIFEST_PATH, "{ author: Ironic ")])).unwrap();
        assert!(broken.required && broken.error.is_some());

        let lua = scan_pack(&raw_pack(tmp.path(), "l.pack", &[("script/campaign/mod/x.lua", "se.query.character(1)")])).unwrap();
        assert!(!lua.required);
    }

    #[test]
    fn scans_real_packs() {
        let tmp = tempfile::tempdir().unwrap();
        let declared = write_pack(tmp.path(), "d.pack", &[(MANIFEST_PATH, "{\"author\":\"Ironic\",\"minimum_version\":0.28,}")]);
        let info = scan_pack(&declared).unwrap();
        assert!(info.required);
        assert_eq!(info.author.as_deref(), Some("Ironic"));
        assert_eq!(info.min_version.as_deref(), Some("0.28"));

        // Lua that calls the API is no longer a signal on its own.
        let lua = write_pack(tmp.path(), "l.pack", &[("script/campaign/mod/x.lua", "se.query.character(1)")]);
        assert_eq!(scan_pack(&lua).unwrap(), SeInfo::default());
    }
}
