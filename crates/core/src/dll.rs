//! Script-extender DLL channel: versions installed under `%APPDATA%\TKModManager\dll\<version>\`,
//! compatibility with the installed game build, and downloads from the public release repo.
//!
//! Release assets (see the ScriptExtender release workflow): `script_extender.dll` and
//! `manifest.json` = { version, game_exe_version, exe_timestamp, exe_size_of_image, sha256 }.

use crate::fingerprint::{self, ExeFingerprint};
use crate::paths;
use crate::context::AppContext;
use crate::update::{download_bytes, download_to, fetch_releases, pick_release, Release};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

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
    /// Further game builds the DLL supports (e.g. Epic); the top-level fields stay the Steam
    /// build so older manager versions keep working.
    #[serde(default)]
    pub builds: Vec<ManifestBuild>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ManifestBuild {
    #[serde(default)]
    pub store: String,
    #[serde(default)]
    pub game_exe_version: String,
    pub exe_timestamp: String,
    pub exe_size_of_image: String,
}

fn fp_equals(ts: &str, soi: &str, fp: &ExeFingerprint) -> bool {
    fingerprint::parse_hex(ts) == Some(fp.timestamp) && fingerprint::parse_hex(soi) == Some(fp.size_of_image)
}

impl Manifest {
    pub fn matches(&self, fp: &ExeFingerprint) -> bool {
        fp_equals(&self.exe_timestamp, &self.exe_size_of_image, fp)
            || self.builds.iter().any(|b| fp_equals(&b.exe_timestamp, &b.exe_size_of_image, fp))
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
    /// What a launch would inject: the pinned version when it is installed and matches the
    /// game, else the highest installed version that matches.
    pub selected: Option<InstalledDll>,
    /// Version the user pinned ("Use this version"), if any. May be missing or not match.
    pub pinned: Option<String>,
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
    pub prerelease: bool,
    /// `YYYY-MM-DD`, may be empty.
    pub published: String,
}

/// One published release in the version catalog, with its manifest when it could be fetched.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    pub remote: RemoteDll,
    pub manifest: Option<Manifest>,
    /// Manifest fingerprint equals the installed exe's; None when either is unknown.
    pub compatible: Option<bool>,
}

/// `dll\pinned.txt`: the version the user chose to always inject (rollback).
pub const PIN_NAME: &str = "pinned.txt";

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
    let pinned = pinned_version();
    let selected = select(&installed, pinned.as_deref());
    DllStatus {
        game_timestamp_hex: fp.map(|f| f.timestamp_hex()),
        game_size_hex: fp.map(|f| f.size_hex()),
        game_fingerprint: fp,
        installed,
        selected,
        pinned,
    }
}

/// Pinned version if installed and matching, else the newest matching one (`installed` is
/// sorted newest first).
fn select(installed: &[InstalledDll], pinned: Option<&str>) -> Option<InstalledDll> {
    pinned
        .and_then(|p| installed.iter().find(|d| d.version == p && d.compatible))
        .or_else(|| installed.iter().find(|d| d.compatible))
        .cloned()
}

pub fn pinned_version() -> Option<String> {
    let text = std::fs::read_to_string(dll_root().join(PIN_NAME)).ok()?;
    let v = text.trim().trim_start_matches('v').to_string();
    (!v.is_empty()).then_some(v)
}

/// Pin a version (`Some`) or go back to "newest that matches" (`None`).
pub fn dll_set_pin(version: Option<&str>) -> Result<(), String> {
    let path = dll_root().join(PIN_NAME);
    match version {
        Some(v) => {
            std::fs::create_dir_all(dll_root()).map_err(|e| e.to_string())?;
            std::fs::write(&path, v.trim_start_matches('v')).map_err(|e| format!("{}: {e}", path.display()))
        }
        None => match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        },
    }
}

pub fn dll_status(ctx: &AppContext) -> DllStatus {
    let p = ctx.game_paths();
    status_for(p.exe.as_deref().map(Path::new))
}

/// Query the latest DLL release. Errors are returned as strings (offline, no release yet).
/// Blocking (network).
pub fn dll_check_update(ctx: &AppContext) -> Result<RemoteDll, String> {
    let allow_pre = ctx.settings().dll_channel == "prerelease";
    let rel = pick_release(fetch_releases(OWNER, REPO)?, allow_pre).ok_or("no release published yet")?;
    remote_from(rel)
}

fn remote_from(rel: Release) -> Result<RemoteDll, String> {
    let dll_url = rel.asset_url(DLL_NAME).ok_or_else(|| format!("release has no {DLL_NAME}"))?.to_string();
    let manifest_url =
        rel.asset_url(MANIFEST_NAME).ok_or_else(|| format!("release has no {MANIFEST_NAME}"))?.to_string();
    let installed = dll_root().join(&rel.version).join(DLL_NAME).is_file();
    Ok(RemoteDll {
        version: rel.version,
        notes: rel.notes,
        dll_url,
        manifest_url,
        installed,
        prerelease: rel.prerelease,
        published: rel.published,
    })
}

/// Every published release that has both assets, newest first, each with its manifest so
/// the catalog can show which game build it was made for. Blocking (network).
pub fn dll_catalog(ctx: &AppContext) -> Result<Vec<CatalogEntry>, String> {
    let mut remotes: Vec<RemoteDll> = fetch_releases(OWNER, REPO)?.into_iter().filter_map(|r| remote_from(r).ok()).collect();
    if remotes.is_empty() {
        return Err("no release published yet".into());
    }
    remotes.sort_by(|a, b| version_key(&b.version).cmp(&version_key(&a.version)));
    let p = ctx.game_paths();
    let fp = p.exe.as_deref().and_then(|e| fingerprint::read(Path::new(e)).ok());
    // Manifests are tiny; fetch them side by side.
    let manifests: Vec<Option<Manifest>> = std::thread::scope(|s| {
        let handles: Vec<_> = remotes
            .iter()
            .map(|r| s.spawn(|| download_bytes(&r.manifest_url).ok().and_then(|b| serde_json::from_slice::<Manifest>(&b).ok())))
            .collect();
        handles.into_iter().map(|h| h.join().ok().flatten()).collect()
    });
    Ok(remotes
        .into_iter()
        .zip(manifests)
        .map(|(remote, manifest)| {
            let compatible = match (&manifest, &fp) {
                (Some(m), Some(fp)) => Some(m.matches(fp)),
                _ => None,
            };
            CatalogEntry { remote, manifest, compatible }
        })
        .collect())
}

fn sha256_of(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(hex::encode(Sha256::digest(&bytes)))
}

/// Download a release into `dll\<version>\` (manifest first, then the DLL, verified by sha256).
/// Emits `dll-progress` 0..1. Never touches other version folders.
/// Blocking (network). `progress` receives 0..1 per file.
pub fn dll_install(ctx: &AppContext, remote: &RemoteDll, progress: &dyn Fn(f64)) -> Result<InstalledDll, String> {
    let (version, dll_url, manifest_url) = (remote.version.clone(), remote.dll_url.clone(), remote.manifest_url.clone());
    let version = version.trim_start_matches('v').to_string();
    if version.is_empty() || version.contains(['/', '\\']) || version.contains("..") {
        return Err("bad version".into());
    }
    let dir = dll_root().join(&version);
    let staging = dll_root().join(format!("{version}.partial"));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| format!("create {}: {e}", staging.display()))?;

    download_to(&manifest_url, &staging.join(MANIFEST_NAME), progress)?;
    let manifest: Manifest = serde_json::from_slice(
        &std::fs::read(staging.join(MANIFEST_NAME)).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("bad manifest.json: {e}"))?;
    download_to(&dll_url, &staging.join(DLL_NAME), progress)?;
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

    let p = ctx.game_paths();
    let fp = p.exe.as_deref().and_then(|e| fingerprint::read(Path::new(e)).ok());
    Ok(list_installed(fp.as_ref())
        .into_iter()
        .find(|d| d.version == version)
        .ok_or("installed folder not found after download")?)
}

/// Register a DLL that is already on disk (e.g. a local build) as version `version`,
/// writing a manifest pinned to the current game fingerprint. The file is copied.
pub fn dll_import_local(ctx: &AppContext, path: &str, version: &str) -> Result<InstalledDll, String> {
    let version = version.trim().trim_start_matches('v').to_string();
    semver::Version::parse(&version).map_err(|e| format!("version must be semver (e.g. 0.15.0): {e}"))?;
    let src = PathBuf::from(&path);
    if !src.is_file() {
        return Err(format!("{path} is not a file"));
    }
    let p = ctx.game_paths();
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
        builds: vec![],
    };
    crate::json_store::save(&dir.join(MANIFEST_NAME), &manifest)?;
    list_installed(Some(&fp))
        .into_iter()
        .find(|d| d.version == version)
        .ok_or_else(|| "import failed".to_string())
}

/// Delete an installed version folder (refused while the game runs).
pub fn dll_remove(version: String) -> Result<(), String> {
    if !inject_core::find_pids(paths::EXE_NAME).is_empty() {
        return Err("quit the game before removing a DLL version".into());
    }
    let dir = dll_root().join(version.trim_start_matches('v'));
    if !dir.join(DLL_NAME).is_file() {
        return Err("no such version".into());
    }
    std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    if pinned_version().as_deref() == Some(version.trim_start_matches('v')) {
        dll_set_pin(None)?;
    }
    Ok(())
}

/// Read the DLL's own log (next to the DLL), if any.
pub fn dll_read_log(dir: String) -> Result<String, String> {
    let p = Path::new(&dir).join(LOG_NAME);
    std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))
}

/// `dll\script_extender.cfg`: the DLL reads it from its own folder or the parent, so one file
/// here serves every installed version. `key=value` lines, `#` comments.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct DllConfig {
    pub build_number: String,
    pub build_number_short: String,
    /// None = leave the game's own "modified" flag alone.
    pub build_modified: Option<bool>,
}

pub const CFG_NAME: &str = "script_extender.cfg";

pub fn parse_cfg(text: &str) -> DllConfig {
    let mut c = DllConfig::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let v = v.trim();
        match k.trim() {
            "build_number" => c.build_number = v.to_string(),
            "build_number_short" => c.build_number_short = v.to_string(),
            "build_modified" => c.build_modified = Some(v == "1" || v.eq_ignore_ascii_case("true")),
            _ => {}
        }
    }
    c
}

pub fn render_cfg(c: &DllConfig) -> String {
    let mut out = String::from("# Script extender settings (written by TK Mod Manager)\n");
    if !c.build_number.trim().is_empty() {
        out.push_str(&format!("build_number={}\n", c.build_number.trim()));
    }
    if !c.build_number_short.trim().is_empty() {
        out.push_str(&format!("build_number_short={}\n", c.build_number_short.trim()));
    }
    if let Some(m) = c.build_modified {
        out.push_str(&format!("build_modified={}\n", if m { 1 } else { 0 }));
    }
    out
}

pub fn dll_read_cfg() -> DllConfig {
    std::fs::read_to_string(dll_root().join(CFG_NAME)).map(|t| parse_cfg(&t)).unwrap_or_default()
}

pub fn dll_write_cfg(config: DllConfig) -> Result<(), String> {
    let dir = dll_root();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(CFG_NAME);
    if config == DllConfig::default() {
        // Nothing set: remove the file so the DLL leaves the build number alone.
        return match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        };
    }
    std::fs::write(&path, render_cfg(&config)).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cfg_roundtrip() {
        let c = DllConfig { build_number: "v1.7.2 Build 25370317 (190E)".into(), build_number_short: "190E".into(), build_modified: Some(false) };
        assert_eq!(parse_cfg(&render_cfg(&c)), c);
        assert_eq!(parse_cfg("# only a comment\n\nunknown=1\n"), DllConfig::default());
        assert_eq!(parse_cfg("build_modified = true").build_modified, Some(true));
    }

    #[test]
    fn pinned_version_wins_only_when_it_matches() {
        let d = |v: &str, compatible: bool| InstalledDll { version: v.into(), path: String::new(), dir: String::new(), manifest: None, compatible };
        let installed = vec![d("0.32.0", false), d("0.31.0", true), d("0.30.1", true)];
        assert_eq!(select(&installed, None).unwrap().version, "0.31.0");
        assert_eq!(select(&installed, Some("0.30.1")).unwrap().version, "0.30.1");
        // Pinned but built for another game build, or not installed: newest match instead.
        assert_eq!(select(&installed, Some("0.32.0")).unwrap().version, "0.31.0");
        assert_eq!(select(&installed, Some("0.1.0")).unwrap().version, "0.31.0");
    }

    #[test]
    fn manifest_matches_fingerprint() {
        let m = Manifest { version: "1.0.0".into(), game_exe_version: String::new(), exe_timestamp: "0x69ce4c84".into(), exe_size_of_image: "0x4836000".into(), sha256: String::new(), builds: vec![] };
        assert!(m.matches(&ExeFingerprint { timestamp: 0x69ce4c84, size_of_image: 0x4836000 }));
        assert!(!m.matches(&ExeFingerprint { timestamp: 1, size_of_image: 0x4836000 }));
    }

    #[test]
    fn manifest_matches_any_listed_build() {
        // An old manifest (no `builds`) still parses; a new one also matches the Epic exe.
        let old: Manifest = serde_json::from_str(r#"{"version":"0.35.0","exe_timestamp":"0x69ce4c84","exe_size_of_image":"0x4836000"}"#).unwrap();
        let epic = ExeFingerprint { timestamp: 0x693ae6af, size_of_image: 0x4832000 };
        assert!(!old.matches(&epic));
        let new: Manifest = serde_json::from_str(
            r#"{"version":"0.36.0","exe_timestamp":"0x69ce4c84","exe_size_of_image":"0x4836000",
                "builds":[{"store":"epic","game_exe_version":"1.7.2.0","exe_timestamp":"0x693ae6af","exe_size_of_image":"0x4832000"}]}"#,
        )
        .unwrap();
        assert!(new.matches(&epic));
        assert!(new.matches(&ExeFingerprint { timestamp: 0x69ce4c84, size_of_image: 0x4836000 }));
    }
}
