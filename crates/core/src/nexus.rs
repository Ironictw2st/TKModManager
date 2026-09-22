//! Nexus Mods: `nxm://` links, the v1 REST API (personal API key), and installs.
//!
//! Installed files live in the manager's own folder, never in the game:
//!
//!   %APPDATA%\TKModManager\nexus\<slot>\index.json      which files are installed, which is active
//!   %APPDATA%\TKModManager\nexus\<slot>\<file dir>\*.pack
//!
//! `<slot>` is the Nexus mod id (or `local-<name>` for an archive with no Nexus origin) and
//! `<file dir>` one downloaded file of it. Only the active file dir is scanned and loaded
//! (through `add_working_directory`), and its packs are keyed `nx:<slot>/<pack>`, so switching
//! between installed versions keeps the profile's order and enabled state.

use crate::json_store;
use crate::packs::{self, ModEntry, ModSource, PackType};
use crate::paths;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const GAME_DOMAIN: &str = "totalwarthreekingdoms";
const API: &str = "https://api.nexusmods.com/v1";
const USER_AGENT: &str = concat!("TKModManager/", env!("CARGO_PKG_VERSION"));
const INDEX: &str = "index.json";

pub fn root() -> PathBuf {
    paths::app_data_dir().join("nexus")
}

pub fn mod_page(mod_id: u64) -> String {
    format!("https://www.nexusmods.com/{GAME_DOMAIN}/mods/{mod_id}")
}

pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

// ---------------------------------------------------------------------------------------------
// nxm:// links

/// `nxm://<game>/mods/<mod>/files/<file>?key=..&expires=..&user_id=..` (the "Mod Manager
/// Download" button). `key`/`expires` let non-Premium accounts create a download link.
#[derive(Clone, Debug, PartialEq)]
pub struct NxmLink {
    pub game: String,
    pub mod_id: u64,
    pub file_id: u64,
    pub key: Option<String>,
    pub expires: Option<u64>,
}

pub fn parse_nxm(url: &str) -> Result<NxmLink, String> {
    let rest = url.trim().strip_prefix("nxm://").ok_or("not an nxm:// link")?;
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
    let parts: Vec<&str> = path.trim_end_matches('/').split('/').collect();
    let [game, "mods", mod_id, "files", file_id] = parts.as_slice() else {
        return Err(format!("unrecognised nxm link: {url}"));
    };
    let mut link = NxmLink {
        game: game.to_ascii_lowercase(),
        mod_id: mod_id.parse().map_err(|_| format!("bad mod id in {url}"))?,
        file_id: file_id.parse().map_err(|_| format!("bad file id in {url}"))?,
        key: None,
        expires: None,
    };
    for pair in query.split('&') {
        match pair.split_once('=') {
            Some(("key", v)) if !v.is_empty() => link.key = Some(v.to_string()),
            Some(("expires", v)) => link.expires = v.parse().ok(),
            _ => {}
        }
    }
    if link.game != GAME_DOMAIN {
        return Err(format!("this link is for another game ({}), not Three Kingdoms", link.game));
    }
    Ok(link)
}

// ---------------------------------------------------------------------------------------------
// Installed files

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct InstalledFile {
    /// Folder name under the slot.
    pub dir: String,
    pub file_id: Option<u64>,
    /// Nexus file title (or the archive name for local installs).
    pub name: String,
    pub version: String,
    /// Archive the file came from.
    pub archive: String,
    /// When Nexus says it was uploaded (unix seconds; 0 = unknown).
    pub uploaded: u64,
    pub installed: u64,
    /// Pack files it put in its folder.
    pub packs: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SlotIndex {
    pub mod_id: Option<u64>,
    pub mod_name: String,
    pub author: String,
    /// `dir` of the active file.
    pub active: String,
    pub files: Vec<InstalledFile>,
}

impl SlotIndex {
    pub fn active_file(&self) -> Option<&InstalledFile> {
        self.files.iter().find(|f| f.dir == self.active)
    }
}

/// What a scanned Nexus pack carries into the UI.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NexusRef {
    pub slot: String,
    pub mod_id: Option<u64>,
    pub mod_name: String,
    pub author: String,
    pub active: InstalledFile,
    /// Every installed file of this mod, newest install first.
    pub versions: Vec<InstalledFile>,
    /// A newer file on Nexus (from the cache), when there is one.
    pub newer: Option<RemoteFile>,
}

pub fn load_index(slot_dir: &Path) -> SlotIndex {
    json_store::load::<SlotIndex>(&slot_dir.join(INDEX)).unwrap_or_default()
}

fn save_index(slot_dir: &Path, index: &SlotIndex) -> Result<(), String> {
    json_store::save(&slot_dir.join(INDEX), index)
}

/// Every slot folder with an index, by slot name.
pub fn slots(root: &Path) -> BTreeMap<String, SlotIndex> {
    let mut out = BTreeMap::new();
    let Ok(rd) = std::fs::read_dir(root) else { return out };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || !e.path().join(INDEX).is_file() {
            continue;
        }
        out.insert(name, load_index(&e.path()));
    }
    out
}

/// The active file of every installed Nexus mod, as scan entries.
pub fn scan(root: &Path) -> Vec<ModEntry> {
    let cache = load_cache();
    let mut out = Vec::new();
    for (slot, index) in slots(root) {
        let Some(active) = index.active_file() else { continue };
        let dir = root.join(&slot).join(&active.dir);
        let Ok(files) = std::fs::read_dir(&dir) else { continue };
        let mut versions = index.files.clone();
        versions.sort_by_key(|f| std::cmp::Reverse(f.installed));
        let newer = index.mod_id.and_then(|id| cache.mods.get(&id)).and_then(|c| newer_file(active, c).cloned());
        let nx = NexusRef {
            slot: slot.clone(),
            mod_id: index.mod_id,
            mod_name: index.mod_name.clone(),
            author: index.author.clone(),
            active: active.clone(),
            versions,
            newer,
        };
        for f in files.flatten() {
            let p = f.path();
            if !p.is_file() || !p.extension().map(|e| e.eq_ignore_ascii_case("pack")).unwrap_or(false) {
                continue;
            }
            if let Some(mut entry) = packs::entry_for(&p, ModSource::Nexus, Some(&slot)) {
                entry.nexus = Some(nx.clone());
                out.push(entry);
            }
        }
    }
    out
}

/// Make `dir` the loaded file of `slot`.
pub fn set_active(root: &Path, slot: &str, dir: &str) -> Result<(), String> {
    let slot_dir = root.join(slot);
    let mut index = load_index(&slot_dir);
    if !index.files.iter().any(|f| f.dir == dir) {
        return Err(format!("{dir} is not installed for {slot}"));
    }
    index.active = dir.to_string();
    save_index(&slot_dir, &index)
}

/// Delete one installed file. When it was the active one, the newest remaining install takes
/// over; when none is left the whole slot goes.
pub fn remove_version(root: &Path, slot: &str, dir: &str) -> Result<(), String> {
    let slot_dir = root.join(slot);
    let mut index = load_index(&slot_dir);
    let target = slot_dir.join(dir);
    if target.exists() {
        std::fs::remove_dir_all(&target).map_err(|e| format!("cannot delete {}: {e} (is the game running?)", target.display()))?;
    }
    index.files.retain(|f| f.dir != dir);
    if index.files.is_empty() {
        return std::fs::remove_dir_all(&slot_dir).map_err(|e| format!("cannot delete {}: {e}", slot_dir.display()));
    }
    if index.active == dir {
        index.active = index.files.iter().max_by_key(|f| f.installed).map(|f| f.dir.clone()).unwrap_or_default();
    }
    save_index(&slot_dir, &index)
}

/// What to install and where.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InstallRequest {
    pub slot: String,
    pub mod_id: Option<u64>,
    pub mod_name: String,
    pub author: String,
    /// `dir` is the folder to create; `packs` / `installed` are filled in by the install.
    pub file: InstalledFile,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InstallError {
    Failed(String),
    /// The archive holds the same pack name in several folders (alternative versions):
    /// the caller asks which folder to use and installs again with it.
    Variants(Vec<String>),
}

impl From<String> for InstallError {
    fn from(s: String) -> Self {
        InstallError::Failed(s)
    }
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallError::Failed(s) => f.write_str(s),
            InstallError::Variants(v) => write!(f, "the archive has several variants: {}", v.join(", ")),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Installed {
    pub slot: String,
    pub dir: String,
    pub packs: Vec<String>,
    pub mod_name: String,
}

/// Unpack the mod and movie packs of `archive` into the slot and make them the active file.
/// `variant` picks a folder inside the archive when it ships alternatives.
pub fn install_archive(root: &Path, archive: &Path, req: &InstallRequest, variant: Option<&str>) -> Result<Installed, InstallError> {
    let staging = root.join(".staging").join(format!("{}-{}", req.slot, now()));
    let result = install_from_staging(root, archive, &staging, req, variant);
    let _ = std::fs::remove_dir_all(&staging);
    result
}

fn install_from_staging(root: &Path, archive: &Path, staging: &Path, req: &InstallRequest, variant: Option<&str>) -> Result<Installed, InstallError> {
    let extracted = crate::archive::extract_filtered(archive, staging, &|n| n.ends_with(".pack") || n.ends_with(".png"))?;
    // (folder inside the archive, pack path)
    let mut found: Vec<(String, PathBuf)> = extracted
        .iter()
        .filter(|p| p.extension().map(|e| e.eq_ignore_ascii_case("pack")).unwrap_or(false))
        .filter(|p| matches!(packs::read_pack_type(p), Ok(PackType::Mod | PackType::Movie)))
        .map(|p| {
            let rel = p.parent().and_then(|d| d.strip_prefix(staging).ok()).map(|d| d.to_string_lossy().replace('\\', "/")).unwrap_or_default();
            (rel, p.clone())
        })
        .collect();
    if found.is_empty() {
        return Err(InstallError::Failed("the archive contains no mod packs".into()));
    }
    if let Some(v) = variant {
        found.retain(|(rel, _)| rel == v);
        if found.is_empty() {
            return Err(InstallError::Failed(format!("the archive has no packs in {v}")));
        }
    }
    let lower = |p: &PathBuf| p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    let mut names: Vec<String> = found.iter().map(|(_, p)| lower(p)).collect();
    names.sort();
    if names.windows(2).any(|w| w[0] == w[1]) {
        let mut folders: Vec<String> = found.iter().map(|(rel, _)| rel.clone()).collect();
        folders.sort();
        folders.dedup();
        return Err(InstallError::Variants(folders));
    }

    let slot_dir = root.join(&req.slot);
    let target = slot_dir.join(&req.file.dir);
    let _ = std::fs::remove_dir_all(&target);
    std::fs::create_dir_all(&target).map_err(|e| format!("{}: {e}", target.display()))?;
    let mut copied = Vec::new();
    for (_, p) in &found {
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        std::fs::copy(p, target.join(&name)).map_err(|e| format!("cannot copy {name}: {e}"))?;
        let png = p.with_extension("png");
        if png.is_file() {
            let _ = std::fs::copy(&png, target.join(&name).with_extension("png"));
        }
        copied.push(name);
    }
    copied.sort_by_key(|n| n.to_lowercase());

    let mut index = load_index(&slot_dir);
    index.mod_id = req.mod_id.or(index.mod_id);
    if !req.mod_name.is_empty() {
        index.mod_name = req.mod_name.clone();
    }
    if !req.author.is_empty() {
        index.author = req.author.clone();
    }
    let mut file = req.file.clone();
    file.packs = copied.clone();
    file.installed = now();
    index.files.retain(|f| f.dir != file.dir);
    index.files.push(file);
    index.active = req.file.dir.clone();
    save_index(&slot_dir, &index)?;
    Ok(Installed { slot: req.slot.clone(), dir: req.file.dir.clone(), packs: copied, mod_name: index.mod_name })
}

/// Slot/folder names are file names: keep them short and plain.
fn safe_name(s: &str) -> String {
    let cleaned: String = s.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' }).collect();
    let t = cleaned.trim_matches(['_', '.']).to_string();
    let t: String = t.chars().take(60).collect();
    if t.is_empty() { "mod".into() } else { t }
}

/// Nexus names manual downloads `<name>-<mod id>-<version>-<upload time>.<ext>`
/// (version dots become dashes). Returns (name, mod id, version, upload time).
pub fn parse_nexus_archive_name(file_name: &str) -> Option<(String, u64, String, u64)> {
    let stem = Path::new(file_name).file_stem()?.to_string_lossy().into_owned();
    let parts: Vec<&str> = stem.split('-').collect();
    if parts.len() < 3 {
        return None;
    }
    let last = parts[parts.len() - 1];
    let uploaded: u64 = last.parse().ok().filter(|_| last.len() >= 9)?;
    let i = (1..parts.len() - 1).find(|&i| !parts[i].is_empty() && parts[i].chars().all(|c| c.is_ascii_digit()))?;
    let mod_id: u64 = parts[i].parse().ok()?;
    let version = parts[i + 1..parts.len() - 1].join(".");
    Some((parts[..i].join("-").trim().to_string(), mod_id, version, uploaded))
}

/// Install request for an archive picked from disk. A Nexus-named archive joins that mod's
/// slot (so update checks and version switching work); anything else gets a local slot.
pub fn request_for_local(archive: &Path) -> InstallRequest {
    let file_name = archive.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if let Some((name, mod_id, version, uploaded)) = parse_nexus_archive_name(&file_name) {
        return InstallRequest {
            slot: mod_id.to_string(),
            mod_id: Some(mod_id),
            mod_name: name.clone(),
            author: String::new(),
            file: InstalledFile { dir: format!("t{uploaded}"), name, version, archive: file_name, uploaded, ..Default::default() },
        };
    }
    let stem = archive.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    InstallRequest {
        slot: format!("local-{}", safe_name(&stem)),
        mod_id: None,
        mod_name: stem.clone(),
        author: String::new(),
        file: InstalledFile { dir: format!("l{}", now()), name: stem, archive: file_name, ..Default::default() },
    }
}

// ---------------------------------------------------------------------------------------------
// API

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct RemoteFile {
    pub file_id: u64,
    pub name: String,
    pub version: String,
    /// MAIN, UPDATE, OPTIONAL, OLD_VERSION, MISCELLANEOUS, ARCHIVED
    pub category: String,
    pub uploaded: u64,
    pub file_name: String,
    pub size_kb: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ModInfo {
    pub fetched_at: u64,
    pub name: String,
    pub author: String,
    pub version: String,
    pub files: Vec<RemoteFile>,
    /// (old file id, new file id): the uploader marked the new file as the old one's update.
    pub updates: Vec<(u64, u64)>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Cache {
    pub mods: BTreeMap<u64, ModInfo>,
}

fn cache_path() -> PathBuf {
    paths::app_data_dir().join("nexus.cache.json")
}

pub fn load_cache() -> Cache {
    json_store::load::<Cache>(&cache_path()).unwrap_or_default()
}

fn store_in_cache(mod_id: u64, info: &ModInfo) {
    let mut c = load_cache();
    c.mods.insert(mod_id, info.clone());
    if let Err(e) = json_store::save(&cache_path(), &c) {
        eprintln!("nexus cache: {e}");
    }
}

/// The file that replaces `active`, if Nexus has one. The uploader's own "update of" chain wins;
/// without a file id (manual installs) the newest MAIN file uploaded after it counts.
pub fn newer_file<'a>(active: &InstalledFile, info: &'a ModInfo) -> Option<&'a RemoteFile> {
    if let Some(start) = active.file_id {
        let mut cur = start;
        for _ in 0..64 {
            match info.updates.iter().find(|(old, _)| *old == cur) {
                Some((_, new)) => cur = *new,
                None => break,
            }
        }
        if cur != start {
            if let Some(f) = info.files.iter().find(|f| f.file_id == cur) {
                return Some(f);
            }
        }
        if info.files.iter().any(|f| f.file_id == start && f.category != "OLD_VERSION" && f.category != "ARCHIVED") {
            return None;
        }
    }
    if active.uploaded == 0 {
        return None;
    }
    info.files.iter().filter(|f| f.category == "MAIN" && f.uploaded > active.uploaded).max_by_key(|f| f.uploaded)
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())
}

fn get(key: &str, path: &str) -> Result<Value, String> {
    if key.trim().is_empty() {
        return Err("no Nexus API key: add yours in Settings > Nexus Mods".into());
    }
    let resp = client()?
        .get(format!("{API}/{path}"))
        .header("apikey", key.trim())
        .header("Application-Name", "TKModManager")
        .header("Application-Version", env!("CARGO_PKG_VERSION"))
        .header("Accept", "application/json")
        .send()
        .map_err(|e| format!("Nexus Mods: {e}"))?;
    let status = resp.status();
    let body: Value = resp.json().unwrap_or(Value::Null);
    if status.is_success() {
        return Ok(body);
    }
    let msg = body["message"].as_str().or(body["error"].as_str()).unwrap_or("").to_string();
    Err(match status.as_u16() {
        401 => "Nexus Mods rejected the API key (check it in Settings > Nexus Mods)".into(),
        403 if path.contains("download_link") => {
            "Nexus Mods only gives direct downloads to Premium members. Use the \"Mod Manager Download\" button on the mod's Files tab instead.".into()
        }
        404 => format!("not found on Nexus Mods ({path})"),
        429 => "Nexus Mods rate limit reached; try again later".into(),
        code => format!("Nexus Mods returned {code}: {msg}"),
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct User {
    pub name: String,
    pub premium: bool,
}

pub fn validate(key: &str) -> Result<User, String> {
    let v = get(key, "users/validate.json")?;
    Ok(User { name: v["name"].as_str().unwrap_or("?").to_string(), premium: v["is_premium"].as_bool().unwrap_or(false) })
}

pub fn parse_mod_info(mod_json: &Value, files_json: &Value) -> ModInfo {
    let s = |v: &Value| v.as_str().unwrap_or("").to_string();
    let files = files_json["files"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|f| {
                    Some(RemoteFile {
                        file_id: f["file_id"].as_u64()?,
                        name: s(&f["name"]),
                        version: s(&f["version"]),
                        category: s(&f["category_name"]),
                        uploaded: f["uploaded_timestamp"].as_u64().unwrap_or(0),
                        file_name: s(&f["file_name"]),
                        size_kb: f["size_kb"].as_u64().unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let updates = files_json["file_updates"]
        .as_array()
        .map(|a| a.iter().filter_map(|u| Some((u["old_file_id"].as_u64()?, u["new_file_id"].as_u64()?))).collect())
        .unwrap_or_default();
    ModInfo {
        fetched_at: now(),
        name: s(&mod_json["name"]),
        author: s(&mod_json["author"]),
        version: s(&mod_json["version"]),
        files,
        updates,
    }
}

/// Mod page + file list (cached for `max_age` seconds). Blocking.
pub fn fetch_mod(key: &str, mod_id: u64, max_age: u64) -> Result<ModInfo, String> {
    if let Some(c) = load_cache().mods.get(&mod_id) {
        if max_age > 0 && now().saturating_sub(c.fetched_at) < max_age {
            return Ok(c.clone());
        }
    }
    let m = get(key, &format!("games/{GAME_DOMAIN}/mods/{mod_id}.json"))?;
    let f = get(key, &format!("games/{GAME_DOMAIN}/mods/{mod_id}/files.json"))?;
    let info = parse_mod_info(&m, &f);
    store_in_cache(mod_id, &info);
    Ok(info)
}

/// Refresh the cached file lists of every installed Nexus mod (update check). Returns how many
/// mods have a newer file. Blocking.
pub fn check_updates(key: &str, max_age: u64) -> Result<usize, String> {
    let mut newer = 0;
    let mut last_err = None;
    for (_, index) in slots(&root()) {
        let (Some(id), Some(active)) = (index.mod_id, index.active_file()) else { continue };
        match fetch_mod(key, id, max_age) {
            Ok(info) => newer += usize::from(newer_file(active, &info).is_some()),
            Err(e) => last_err = Some(e),
        }
    }
    match (newer, last_err) {
        (0, Some(e)) => Err(e),
        (n, _) => Ok(n),
    }
}

fn download_link(key: &str, link: &NxmLink) -> Result<String, String> {
    let mut path = format!("games/{GAME_DOMAIN}/mods/{}/files/{}/download_link.json", link.mod_id, link.file_id);
    if let (Some(k), Some(exp)) = (&link.key, link.expires) {
        path.push_str(&format!("?key={k}&expires={exp}"));
    }
    let v = get(key, &path)?;
    v.as_array()
        .and_then(|a| a.iter().find_map(|l| l["URI"].as_str()))
        .map(str::to_string)
        .ok_or_else(|| "Nexus Mods returned no download link".into())
}

/// A downloaded archive waiting to be installed.
#[derive(Clone, Debug)]
pub struct Downloaded {
    pub archive: PathBuf,
    pub request: InstallRequest,
}

/// Resolve an nxm link, download the file into `nexus\.downloads`. Blocking.
pub fn download(key: &str, link: &NxmLink, progress: &dyn Fn(f64)) -> Result<Downloaded, String> {
    let info = fetch_mod(key, link.mod_id, 0)?;
    let file = info.files.iter().find(|f| f.file_id == link.file_id).cloned().unwrap_or(RemoteFile { file_id: link.file_id, ..Default::default() });
    let url = download_link(key, link)?;
    let dir = root().join(".downloads");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let archive_name = if file.file_name.is_empty() { format!("{}-{}.archive", link.mod_id, link.file_id) } else { file.file_name.clone() };
    let archive = dir.join(safe_name(&archive_name));
    crate::update::download_to(&url, &archive, progress)?;
    Ok(Downloaded {
        archive,
        request: InstallRequest {
            slot: link.mod_id.to_string(),
            mod_id: Some(link.mod_id),
            mod_name: info.name.clone(),
            author: info.author.clone(),
            file: InstalledFile {
                dir: link.file_id.to_string(),
                file_id: Some(link.file_id),
                name: file.name,
                version: file.version,
                archive: archive_name,
                uploaded: file.uploaded,
                ..Default::default()
            },
        },
    })
}

// ---------------------------------------------------------------------------------------------
// nxm:// handler registration (HKCU, no admin rights)

fn reg(args: &[&str]) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("reg").args(args).creation_flags(CREATE_NO_WINDOW).output().map_err(|e| format!("reg: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

const NXM_COMMAND_KEY: &str = r"HKCU\Software\Classes\nxm\shell\open\command";

/// The command Windows runs for nxm:// links (any manager: Vortex, MO2, us), if set.
pub fn nxm_handler() -> Option<String> {
    let text = reg(&["query", NXM_COMMAND_KEY, "/ve"]).ok()?;
    let line = text.lines().find(|l| l.contains("REG_SZ"))?;
    let v = line.split("REG_SZ").nth(1)?.trim().to_string();
    (!v.is_empty()).then_some(v)
}

pub fn handler_command(exe: &Path) -> String {
    format!("\"{}\" \"%1\"", exe.display())
}

/// Point nxm:// links at `exe`. Only called when the user asks for it.
pub fn register_nxm_handler(exe: &Path) -> Result<(), String> {
    reg(&["add", r"HKCU\Software\Classes\nxm", "/ve", "/d", "URL:NXM Protocol", "/f"])?;
    reg(&["add", r"HKCU\Software\Classes\nxm", "/v", "URL Protocol", "/d", "", "/f"])?;
    reg(&["add", NXM_COMMAND_KEY, "/ve", "/d", &handler_command(exe), "/f"])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack_bytes(kind: u32) -> Vec<u8> {
        let mut b = b"PFH5".to_vec();
        b.extend_from_slice(&kind.to_le_bytes());
        b.extend_from_slice(&[0; 20]);
        b
    }

    fn zip_with(path: &Path, entries: &[(&str, Vec<u8>)]) {
        use std::io::Write;
        let mut w = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (name, bytes) in entries {
            w.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            w.write_all(bytes).unwrap();
        }
        w.finish().unwrap();
    }

    #[test]
    fn parses_nxm_links() {
        let l = parse_nxm("nxm://totalwarthreekingdoms/mods/249/files/8005809?key=abc&expires=1700000000&user_id=5").unwrap();
        assert_eq!((l.mod_id, l.file_id), (249, 8005809));
        assert_eq!(l.key.as_deref(), Some("abc"));
        assert_eq!(l.expires, Some(1700000000));
        let l = parse_nxm("nxm://TotalWarThreeKingdoms/mods/1/files/2").unwrap();
        assert_eq!((l.key, l.expires), (None, None));
        assert!(parse_nxm("nxm://skyrimspecialedition/mods/1/files/2").unwrap_err().contains("another game"));
        assert!(parse_nxm("https://x").is_err());
        assert!(parse_nxm("nxm://totalwarthreekingdoms/collections/x").is_err());
    }

    #[test]
    fn parses_manual_archive_names() {
        assert_eq!(
            parse_nexus_archive_name("TK Mod Manager-249-0-5-1-1726800000.zip"),
            Some(("TK Mod Manager".into(), 249, "0.5.1".into(), 1726800000))
        );
        assert_eq!(parse_nexus_archive_name("my-mod-12-1-0-1726800000.7z").map(|t| (t.0, t.1)), Some(("my-mod".into(), 12)));
        assert_eq!(parse_nexus_archive_name("random.zip"), None);
        assert_eq!(parse_nexus_archive_name("a-b-c.zip"), None);
    }

    #[test]
    fn newer_file_follows_update_chain_then_timestamps() {
        let f = |id, cat: &str, up| RemoteFile { file_id: id, category: cat.into(), uploaded: up, version: format!("v{id}"), ..Default::default() };
        let info = ModInfo { files: vec![f(1, "OLD_VERSION", 10), f(2, "OLD_VERSION", 20), f(3, "MAIN", 30), f(9, "OPTIONAL", 40)], updates: vec![(1, 2), (2, 3)], ..Default::default() };
        let active = |id: Option<u64>, up| InstalledFile { file_id: id, uploaded: up, ..Default::default() };
        assert_eq!(newer_file(&active(Some(1), 10), &info).map(|f| f.file_id), Some(3));
        assert_eq!(newer_file(&active(Some(3), 30), &info), None);
        assert_eq!(newer_file(&active(Some(9), 40), &info), None);
        // Manual install: only the upload time is known.
        assert_eq!(newer_file(&active(None, 15), &info).map(|f| f.file_id), Some(3));
        assert_eq!(newer_file(&active(None, 0), &info), None);
    }

    #[test]
    fn install_switch_and_remove_versions() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("nexus");
        let a1 = tmp.path().join("a1.zip");
        zip_with(&a1, &[("Data/cool.pack", pack_bytes(3)), ("Data/cool.png", b"png".to_vec()), ("Data/vanilla.pack", pack_bytes(1)), ("readme.txt", b"x".to_vec())]);
        let req = |dir: &str, ver: &str| InstallRequest {
            slot: "77".into(),
            mod_id: Some(77),
            mod_name: "Cool".into(),
            file: InstalledFile { dir: dir.into(), file_id: dir.parse().ok(), version: ver.into(), ..Default::default() },
            ..Default::default()
        };
        let done = install_archive(&root, &a1, &req("100", "1.0"), None).unwrap();
        assert_eq!(done.packs, vec!["cool.pack".to_string()]);
        assert!(root.join("77/100/cool.png").is_file());
        assert!(!root.join(".staging").read_dir().unwrap().next().is_some());

        let a2 = tmp.path().join("a2.zip");
        zip_with(&a2, &[("cool.pack", pack_bytes(3))]);
        install_archive(&root, &a2, &req("200", "2.0"), None).unwrap();

        let scanned = scan(&root);
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].key, "nx:77/cool.pack");
        assert_eq!(scanned[0].source, ModSource::Nexus);
        assert_eq!(scanned[0].nexus.as_ref().unwrap().active.version, "2.0");
        assert_eq!(scanned[0].nexus.as_ref().unwrap().versions.len(), 2);

        // Switching versions keeps the key (profile order/enabled state survive).
        set_active(&root, "77", "100").unwrap();
        let scanned = scan(&root);
        assert_eq!(scanned[0].key, "nx:77/cool.pack");
        assert!(scanned[0].dir.ends_with("100"));

        remove_version(&root, "77", "100").unwrap();
        assert_eq!(load_index(&root.join("77")).active, "200");
        remove_version(&root, "77", "200").unwrap();
        assert!(!root.join("77").exists());
    }

    #[test]
    fn variants_must_be_chosen() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("nexus");
        let a = tmp.path().join("v.zip");
        zip_with(&a, &[("Easy/m.pack", pack_bytes(3)), ("Hard/m.pack", pack_bytes(3))]);
        let req = InstallRequest { slot: "5".into(), file: InstalledFile { dir: "1".into(), ..Default::default() }, ..Default::default() };
        assert_eq!(install_archive(&root, &a, &req, None), Err(InstallError::Variants(vec!["Easy".into(), "Hard".into()])));
        let done = install_archive(&root, &a, &req, Some("Hard")).unwrap();
        assert_eq!(done.packs, vec!["m.pack".to_string()]);
    }

    #[test]
    fn no_packs_is_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("n.zip");
        zip_with(&a, &[("readme.txt", b"x".to_vec())]);
        let req = InstallRequest { slot: "5".into(), file: InstalledFile { dir: "1".into(), ..Default::default() }, ..Default::default() };
        assert!(matches!(install_archive(&tmp.path().join("nexus"), &a, &req, None), Err(InstallError::Failed(_))));
    }

    #[test]
    fn local_requests() {
        let r = request_for_local(Path::new(r"C:\dl\Cool Mod-77-1-2-1726800000.zip"));
        assert_eq!((r.slot.as_str(), r.mod_id, r.file.version.as_str(), r.file.dir.as_str()), ("77", Some(77), "1.2", "t1726800000"));
        let r = request_for_local(Path::new(r"C:\dl\my stuff!.rar"));
        assert_eq!(r.slot, "local-my_stuff");
        assert_eq!(r.mod_id, None);
    }
}
