//! Self-update from GitHub Releases, plus the release helpers the script-extender channel uses.
//!
//! The app ships as a zip (exe + Qt DLLs + plugin folders). Installing an update: download the
//! zip, extract it into `update.staging\` next to the exe, then for every file rename the
//! current one to `*.old` (Windows allows renaming an exe/DLL that is in use) and move the new
//! one in. The caller restarts the app; `cleanup_old_files` removes the `*.old` files on the next
//! start. Trust is HTTPS + GitHub release authenticity, as before.

use serde::Serialize;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const OWNER: &str = "Ironictw2st";
pub const REPO: &str = "TKModManager";
/// The zip published on each release by `scripts/release.ps1`.
pub const ASSET_NAME: &str = "TKModManager-x64.zip";
pub const USER_AGENT: &str = "TKModManager-Updater";
const STAGING_DIR: &str = "update.staging";

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMeta {
    pub version: String,
    pub notes: String,
    pub asset_url: String,
}

/// Minimal view of a GitHub release: tag, body, and (name, url) of each asset.
#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub notes: String,
    pub assets: Vec<(String, String)>,
    pub prerelease: bool,
}

impl Release {
    pub fn asset_url(&self, name: &str) -> Option<&str> {
        self.assets.iter().find(|(n, _)| n == name).map(|(_, u)| u.as_str())
    }
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())
}

fn release_from_json(json: &serde_json::Value) -> Option<Release> {
    let version = json["tag_name"].as_str()?.trim_start_matches('v').to_string();
    if version.is_empty() {
        return None;
    }
    let assets = json["assets"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|a| Some((a["name"].as_str()?.to_string(), a["browser_download_url"].as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default();
    Some(Release {
        version,
        notes: json["body"].as_str().unwrap_or("").to_string(),
        assets,
        prerelease: json["prerelease"].as_bool().unwrap_or(false),
    })
}

fn get_json(url: &str) -> Result<serde_json::Value, String> {
    let resp = client()?
        .get(url)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .map_err(|e| format!("network error: {e}"))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err("no release published yet".into());
    }
    if !resp.status().is_success() {
        return Err(format!("GitHub API returned {}", resp.status()));
    }
    resp.json().map_err(|e| format!("bad response JSON: {e}"))
}

/// The most recent releases of a repo (drafts excluded). Blocking.
pub fn fetch_releases(owner: &str, repo: &str) -> Result<Vec<Release>, String> {
    let json = get_json(&format!("https://api.github.com/repos/{owner}/{repo}/releases?per_page=15"))?;
    Ok(json
        .as_array()
        .map(|arr| arr.iter().filter(|r| !r["draft"].as_bool().unwrap_or(false)).filter_map(release_from_json).collect())
        .unwrap_or_default())
}

/// `releases/latest` of a repo. Blocking.
pub fn fetch_latest_release(owner: &str, repo: &str) -> Result<Release, String> {
    let json = get_json(&format!("https://api.github.com/repos/{owner}/{repo}/releases/latest"))?;
    release_from_json(&json).ok_or_else(|| "latest release has no tag_name".to_string())
}

/// Highest semver among `releases`; pre-releases only when `allow_prerelease`.
pub fn pick_release(releases: Vec<Release>, allow_prerelease: bool) -> Option<Release> {
    releases
        .into_iter()
        .filter_map(|r| semver::Version::parse(&r.version).ok().map(|v| (v, r)))
        .filter(|(v, r)| allow_prerelease || (!r.prerelease && v.pre.is_empty()))
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, r)| r)
}

/// Stream `url` to `path`, reporting a 0..1 fraction. Blocking.
pub fn download_to(url: &str, path: &Path, progress: &dyn Fn(f64)) -> Result<(), String> {
    let mut resp = client()?.get(url).send().map_err(|e| format!("download failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("download returned {}", resp.status()));
    }
    let total = resp.content_length().unwrap_or(0);
    let mut file = std::fs::File::create(path).map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    let mut buf = vec![0u8; 256 * 1024];
    let mut done: u64 = 0;
    loop {
        let n = resp.read(&mut buf).map_err(|e| format!("download error: {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| format!("write error: {e}"))?;
        done += n as u64;
        if total > 0 {
            progress((done as f64 / total as f64).min(1.0));
        }
    }
    file.flush().map_err(|e| format!("flush error: {e}"))?;
    Ok(())
}

/// Update info when the latest release is newer than this build, else None. Debug builds never
/// report an update, so development never replaces its own binaries. Blocking.
pub fn check_update() -> Result<Option<UpdateMeta>, String> {
    if cfg!(debug_assertions) {
        return Ok(None);
    }
    let latest = fetch_latest_release(OWNER, REPO)?;
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION")).map_err(|e| format!("bad current version: {e}"))?;
    let remote = semver::Version::parse(&latest.version).map_err(|e| format!("bad release version '{}': {e}", latest.version))?;
    if remote <= current {
        return Ok(None);
    }
    let asset_url = latest.asset_url(ASSET_NAME).ok_or_else(|| format!("release has no asset named {ASSET_NAME}"))?.to_string();
    Ok(Some(UpdateMeta { version: latest.version, notes: latest.notes, asset_url }))
}

fn install_dir() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    exe.parent().map(Path::to_path_buf).ok_or_else(|| "exe has no folder".to_string())
}

/// Extract `zip_path` into `dest` (created fresh). Entries with unsafe paths are rejected.
pub fn extract_zip(zip_path: &Path, dest: &Path) -> Result<(), String> {
    let _ = std::fs::remove_dir_all(dest);
    std::fs::create_dir_all(dest).map_err(|e| format!("{}: {e}", dest.display()))?;
    let file = std::fs::File::open(zip_path).map_err(|e| format!("{}: {e}", zip_path.display()))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("bad update zip: {e}"))?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let Some(rel) = entry.enclosed_name() else { return Err(format!("unsafe path in update zip: {}", entry.name())) };
        let out = dest.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut f = std::fs::File::create(&out).map_err(|e| format!("{}: {e}", out.display()))?;
        std::io::copy(&mut entry, &mut f).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Every file under `dir`, as paths relative to it.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if let Ok(rel) = p.strip_prefix(dir) {
                out.push(rel.to_path_buf());
            }
        }
    }
    out.sort();
    out
}

fn old_name(p: &Path) -> PathBuf {
    let mut s = p.as_os_str().to_os_string();
    s.push(".old");
    PathBuf::from(s)
}

/// Move every file from `staging` into `target`, renaming files that already exist to `*.old`
/// first (works for the running exe and loaded DLLs). Removes `staging` afterwards.
pub fn apply_staged(staging: &Path, target: &Path) -> Result<usize, String> {
    let files = files_under(staging);
    for rel in &files {
        let from = staging.join(rel);
        let to = target.join(rel);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        if to.exists() {
            let old = old_name(&to);
            let _ = std::fs::remove_file(&old);
            std::fs::rename(&to, &old).map_err(|e| format!("cannot move {} aside: {e}", to.display()))?;
        }
        std::fs::rename(&from, &to).map_err(|e| format!("cannot install {}: {e}", to.display()))?;
    }
    let _ = std::fs::remove_dir_all(staging);
    Ok(files.len())
}

/// Download and install an update next to the running exe. The caller restarts afterwards.
/// Blocking.
pub fn install_update(asset_url: &str, progress: &dyn Fn(f64)) -> Result<(), String> {
    let dir = install_dir()?;
    let zip_path = std::env::temp_dir().join("tkmodmanager-update.zip");
    download_to(asset_url, &zip_path, progress)?;
    let staging = dir.join(STAGING_DIR);
    extract_zip(&zip_path, &staging)?;
    let _ = std::fs::remove_file(&zip_path);
    if !staging.join(format!("{}.exe", env!("CARGO_PKG_NAME"))).is_file() && !files_under(&staging).iter().any(|p| p.extension().map(|e| e == "exe").unwrap_or(false)) {
        let _ = std::fs::remove_dir_all(&staging);
        return Err("the update zip contains no executable".into());
    }
    apply_staged(&staging, &dir)?;
    Ok(())
}

/// Remove `*.old` leftovers of a previous update (they are unlocked once the old process exited).
pub fn cleanup_old_files() {
    let Ok(dir) = install_dir() else { return };
    for rel in files_under(&dir) {
        if rel.extension().map(|e| e == "old").unwrap_or(false) {
            let _ = std::fs::remove_file(dir.join(rel));
        }
    }
    let _ = std::fs::remove_dir_all(dir.join(STAGING_DIR));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(v: &str, pre: bool) -> Release {
        Release { version: v.into(), notes: String::new(), assets: vec![], prerelease: pre }
    }

    #[test]
    fn picks_by_channel() {
        let all = || vec![r("0.23.2", false), r("0.24.0-beta.1", true), r("0.9.0", false), r("junk", false)];
        assert_eq!(pick_release(all(), false).unwrap().version, "0.23.2");
        assert_eq!(pick_release(all(), true).unwrap().version, "0.24.0-beta.1");
        assert!(pick_release(vec![r("1.0.0-rc.1", false)], false).is_none());
    }

    #[test]
    fn applies_staged_files_over_existing_ones() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("app");
        let staging = tmp.path().join("staging");
        std::fs::create_dir_all(target.join("platforms")).unwrap();
        std::fs::write(target.join("app.exe"), "old exe").unwrap();
        std::fs::write(target.join("platforms/qwindows.dll"), "old dll").unwrap();
        std::fs::create_dir_all(staging.join("platforms")).unwrap();
        std::fs::create_dir_all(staging.join("styles")).unwrap();
        std::fs::write(staging.join("app.exe"), "new exe").unwrap();
        std::fs::write(staging.join("platforms/qwindows.dll"), "new dll").unwrap();
        std::fs::write(staging.join("styles/qmodernwindowsstyle.dll"), "style").unwrap();

        assert_eq!(apply_staged(&staging, &target).unwrap(), 3);
        assert_eq!(std::fs::read_to_string(target.join("app.exe")).unwrap(), "new exe");
        assert_eq!(std::fs::read_to_string(target.join("app.exe.old")).unwrap(), "old exe");
        assert_eq!(std::fs::read_to_string(target.join("platforms/qwindows.dll")).unwrap(), "new dll");
        assert!(target.join("styles/qmodernwindowsstyle.dll").is_file());
        assert!(!staging.exists());
    }

    #[test]
    fn extracts_zip_and_rejects_escaping_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let good = tmp.path().join("good.zip");
        {
            let mut w = zip::ZipWriter::new(std::fs::File::create(&good).unwrap());
            let opts = zip::write::SimpleFileOptions::default();
            w.start_file("tkmm.exe", opts).unwrap();
            w.write_all(b"exe").unwrap();
            w.start_file("platforms/qwindows.dll", opts).unwrap();
            w.write_all(b"dll").unwrap();
            w.finish().unwrap();
        }
        let out = tmp.path().join("out");
        extract_zip(&good, &out).unwrap();
        assert_eq!(std::fs::read_to_string(out.join("platforms/qwindows.dll")).unwrap(), "dll");

        let bad = tmp.path().join("bad.zip");
        {
            let mut w = zip::ZipWriter::new(std::fs::File::create(&bad).unwrap());
            w.start_file("../evil.exe", zip::write::SimpleFileOptions::default()).unwrap();
            w.write_all(b"x").unwrap();
            w.finish().unwrap();
        }
        assert!(extract_zip(&bad, &tmp.path().join("out2")).is_err());
    }
}
