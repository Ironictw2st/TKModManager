//! Script-extender DLL channel: versions installed under `%APPDATA%\TKModManager\dll\<version>\`,
//! compatibility with the installed game build, and downloads from the public release repo.
//!
//! Release assets (see the ScriptExtender release workflow): `script_extender.dll` and
//! `manifest.json` = { version, game_exe_version, exe_timestamp, exe_size_of_image, sha256 }.

use crate::fingerprint::{self, ExeFingerprint};
use crate::paths;
use crate::state::AppState;
use crate::update::{download_to, fetch_latest_release};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, State};

pub const OWNER: &str = "Ironictw2st";
pub const REPO: &str = "TK-ScriptExtender";
pub const DLL_NAME: &str = "script_extender.dll";
pub const MANIFEST_NAME: &str = "manifest.json";
pub const LOG_NAME: &str = "script_extender.log";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Manifest {
    pub version: String,
    #[serde(default)]
    pub game_exe_version: String,
    pub exe_timestamp: String,
    pub exe_size_of_image: String,
    #[serde(default)]
    pub sha256: String,
}

impl Manifest {
    pub fn matches(&self, fp: &ExeFingerprint) -> bool {
        fingerprint::parse_hex(&self.exe_timestamp) == Some(fp.timestamp)
            && fingerprint::parse_hex(&self.exe_size_of_image) == Some(fp.size_of_image)
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InstalledDll {
    pub version: String,
    pub path: String,
    pub dir: String,
    pub manifest: Option<Manifest>,
    /// Manifest fingerprint equals the installed exe's.
    pub compatible: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DllStatus {
    pub game_fingerprint: Option<ExeFingerprint>,
    pub game_timestamp_hex: Option<String>,
    pub game_size_hex: Option<String>,
    pub installed: Vec<InstalledDll>,
    /// Highest installed version whose manifest matches the game; what a launch would inject.
    pub selected: Option<InstalledDll>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RemoteDll {
    pub version: String,
    pub notes: String,
    pub dll_url: String,
    pub manifest_url: String,
    /// Already installed locally.
    pub installed: bool,
}

pub fn dll_root() -> PathBuf {
    paths::app_data_dir().join("dll")
}

fn version_key(v: &str) -> semver::Version {
    semver::Version::parse(v.trim_start_matches('v')).unwrap_or_else(|_| semver::Version::new(0, 0, 0))
}

pub fn list_installed(fp: Option<&ExeFingerprint>) -> Vec<InstalledDll> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dll_root()) else { return out };
    for e in rd.flatten() {
        let dir = e.path();
        let dll = dir.join(DLL_NAME);
        if !dir.is_dir() || !dll.is_file() {
            continue;
        }
        let version = e.file_name().to_string_lossy().into_owned();
        let manifest: Option<Manifest> = std::fs::read(dir.join(MANIFEST_NAME))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok());
        let compatible = match (&manifest, fp) {
            (Some(m), Some(fp)) => m.matches(fp),
            _ => false,
        };
        out.push(InstalledDll {
            version,
            path: dll.to_string_lossy().into_owned(),
            dir: dir.to_string_lossy().into_owned(),
            manifest,
            compatible,
        });
    }
    out.sort_by(|a, b| version_key(&b.version).cmp(&version_key(&a.version)));
    out
}

pub fn status_for(exe: Option<&Path>) -> DllStatus {
    let fp = exe.and_then(|e| fingerprint::read(e).ok());
    let installed = list_installed(fp.as_ref());
    let selected = installed.iter().find(|d| d.compatible).cloned();
    DllStatus {
        game_timestamp_hex: fp.map(|f| f.timestamp_hex()),
        game_size_hex: fp.map(|f| f.size_hex()),
        game_fingerprint: fp,
        installed,
        selected,
    }
}

#[tauri::command]
pub fn dll_status(state: State<AppState>) -> DllStatus {
    let p = state.game_paths();
    status_for(p.exe.as_deref().map(Path::new))
}

/// Query the latest DLL release. Errors are returned as strings (offline, no release yet).
#[tauri::command]
pub async fn dll_check_update() -> Result<RemoteDll, String> {
    let rel = fetch_latest_release(OWNER, REPO).await?;
    let dll_url = rel.asset_url(DLL_NAME).ok_or_else(|| format!("release has no {DLL_NAME}"))?.to_string();
    let manifest_url =
        rel.asset_url(MANIFEST_NAME).ok_or_else(|| format!("release has no {MANIFEST_NAME}"))?.to_string();
    let installed = dll_root().join(&rel.version).join(DLL_NAME).is_file();
    Ok(RemoteDll { version: rel.version, notes: rel.notes, dll_url, manifest_url, installed })
}

fn sha256_of(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(hex::encode(Sha256::digest(&bytes)))
}

/// Download a release into `dll\<version>\` (manifest first, then the DLL, verified by sha256).
/// Emits `dll-progress` 0..1. Never touches other version folders.
#[tauri::command]
pub async fn dll_install(app: AppHandle, version: String, dll_url: String, manifest_url: String) -> Result<InstalledDll, String> {
    let version = version.trim_start_matches('v').to_string();
    if version.is_empty() || version.contains(['/', '\\']) || version.contains("..") {
        return Err("bad version".into());
    }
    let dir = dll_root().join(&version);
    let staging = dll_root().join(format!("{version}.partial"));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| format!("create {}: {e}", staging.display()))?;

    download_to(&app, &manifest_url, &staging.join(MANIFEST_NAME), "dll-progress").await?;
    let manifest: Manifest = serde_json::from_slice(
        &std::fs::read(staging.join(MANIFEST_NAME)).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("bad manifest.json: {e}"))?;
    download_to(&app, &dll_url, &staging.join(DLL_NAME), "dll-progress").await?;
    if !manifest.sha256.is_empty() {
        let got = sha256_of(&staging.join(DLL_NAME))?;
        if !got.eq_ignore_ascii_case(manifest.sha256.trim()) {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(format!("sha256 mismatch: manifest {} vs download {got}", manifest.sha256));
        }
    }
    if dir.exists() {
        // Re-download over an existing (possibly loaded) version: keep the old folder if the
        // game is running; otherwise replace it.
        if !inject_core::find_pids(paths::EXE_NAME).is_empty() {
            let _ = std::fs::remove_dir_all(&staging);
            return Err("that DLL version is installed and the game is running; quit the game first".into());
        }
        std::fs::remove_dir_all(&dir).map_err(|e| format!("replace {}: {e}", dir.display()))?;
    }
    std::fs::rename(&staging, &dir).map_err(|e| format!("finalise {}: {e}", dir.display()))?;

    let state = app.state::<AppState>();
    let p = state.game_paths();
    let fp = p.exe.as_deref().and_then(|e| fingerprint::read(Path::new(e)).ok());
    Ok(list_installed(fp.as_ref())
        .into_iter()
        .find(|d| d.version == version)
        .ok_or("installed folder not found after download")?)
}

/// Register a DLL that is already on disk (e.g. a local build) as version `version`,
/// writing a manifest pinned to the current game fingerprint. The file is copied.
#[tauri::command]
pub fn dll_import_local(state: State<AppState>, path: String, version: String) -> Result<InstalledDll, String> {
    let version = version.trim().trim_start_matches('v').to_string();
    semver::Version::parse(&version).map_err(|e| format!("version must be semver (e.g. 0.15.0): {e}"))?;
    let src = PathBuf::from(&path);
    if !src.is_file() {
        return Err(format!("{path} is not a file"));
    }
    let p = state.game_paths();
    let exe = p.exe.as_deref().ok_or("game not found")?;
    let fp = fingerprint::read(Path::new(exe))?;
    let dir = dll_root().join(&version);
    if dir.join(DLL_NAME).is_file() && !inject_core::find_pids(paths::EXE_NAME).is_empty() {
        return Err("that version is installed and the game is running; quit the game first".into());
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    std::fs::copy(&src, dir.join(DLL_NAME)).map_err(|e| format!("copy: {e}"))?;
    let manifest = Manifest {
        version: version.clone(),
        game_exe_version: p.exe_version.clone().unwrap_or_default(),
        exe_timestamp: fp.timestamp_hex(),
        exe_size_of_image: fp.size_hex(),
        sha256: sha256_of(&dir.join(DLL_NAME))?,
    };
    crate::json_store::save(&dir.join(MANIFEST_NAME), &manifest)?;
    list_installed(Some(&fp))
        .into_iter()
        .find(|d| d.version == version)
        .ok_or_else(|| "import failed".to_string())
}

/// Delete an installed version folder (refused while the game runs).
#[tauri::command]
pub fn dll_remove(version: String) -> Result<(), String> {
    if !inject_core::find_pids(paths::EXE_NAME).is_empty() {
        return Err("quit the game before removing a DLL version".into());
    }
    let dir = dll_root().join(version.trim_start_matches('v'));
    if !dir.join(DLL_NAME).is_file() {
        return Err("no such version".into());
    }
    std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())
}

/// Read the DLL's own log (next to the DLL), if any.
#[tauri::command]
pub fn dll_read_log(dir: String) -> Result<String, String> {
    let p = Path::new(&dir).join(LOG_NAME);
    std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))
}
