//! Portable self-replacing updater (same design as TWUI Editor): check the latest GitHub
//! release and, on request, download the portable exe, swap the running binary in place, and
//! relaunch. Trust is HTTPS + GitHub release authenticity; no signing step.

use serde::Serialize;
use std::io::Write;
use tauri::{AppHandle, Emitter};

pub const OWNER: &str = "Ironictw2st";
pub const REPO: &str = "TKModManager";
/// The portable executable asset published on each release (see the release workflow).
pub const ASSET_NAME: &str = "TKModManager-x64.exe";
pub const USER_AGENT: &str = "TKModManager-Updater";

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMeta {
    pub version: String,
    pub notes: String,
    /// Direct download URL for the portable exe asset.
    pub asset_url: String,
}

/// Minimal view of a GitHub release: tag, body, and (name, url) of each asset.
pub struct Release {
    pub version: String,
    pub notes: String,
    pub assets: Vec<(String, String)>,
    pub prerelease: bool,
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

/// The most recent releases of a repo (drafts excluded), newest first as GitHub returns them.
pub async fn fetch_releases(owner: &str, repo: &str) -> Result<Vec<Release>, String> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/releases?per_page=15");
    let resp = reqwest::Client::new()
        .get(&url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("network error: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("GitHub API returned {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("bad response JSON: {e}"))?;
    Ok(json
        .as_array()
        .map(|arr| arr.iter().filter(|r| !r["draft"].as_bool().unwrap_or(false)).filter_map(release_from_json).collect())
        .unwrap_or_default())
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

/// Fetch `releases/latest` for a repo.
pub async fn fetch_latest_release(owner: &str, repo: &str) -> Result<Release, String> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest");
    let resp = reqwest::Client::new()
        .get(&url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("network error: {e}"))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err("no release published yet".into());
    }
    if !resp.status().is_success() {
        return Err(format!("GitHub API returned {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("bad response JSON: {e}"))?;
    release_from_json(&json).ok_or_else(|| "latest release has no tag_name".to_string())
}

impl Release {
    pub fn asset_url(&self, name: &str) -> Option<&str> {
        self.assets.iter().find(|(n, _)| n == name).map(|(_, u)| u.as_str())
    }
}

/// Return update info when the latest release is newer than the running version, else None.
/// Debug builds never report an update (so dev never self-replaces its own exe).
#[tauri::command]
pub async fn check_update() -> Result<Option<UpdateMeta>, String> {
    if cfg!(debug_assertions) {
        return Ok(None);
    }
    let latest = fetch_latest_release(OWNER, REPO).await?;
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|e| format!("bad current version: {e}"))?;
    let remote = semver::Version::parse(&latest.version)
        .map_err(|e| format!("bad release version '{}': {e}", latest.version))?;
    if remote <= current {
        return Ok(None);
    }
    let asset_url = latest
        .asset_url(ASSET_NAME)
        .ok_or_else(|| format!("release has no asset named {ASSET_NAME}"))?
        .to_string();
    Ok(Some(UpdateMeta { version: latest.version, notes: latest.notes, asset_url }))
}

/// Stream a URL to `file`, emitting `event` with a 0..1 fraction as it goes.
pub async fn download_to(app: &AppHandle, url: &str, path: &std::path::Path, event: &str) -> Result<(), String> {
    let mut resp = reqwest::Client::new()
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("download failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("download returned {}", resp.status()));
    }
    let total = resp.content_length().unwrap_or(0);
    let mut file = std::fs::File::create(path).map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    let mut downloaded: u64 = 0;
    while let Some(chunk) = resp.chunk().await.map_err(|e| format!("download error: {e}"))? {
        file.write_all(&chunk).map_err(|e| format!("write error: {e}"))?;
        downloaded += chunk.len() as u64;
        if total > 0 {
            let frac = (downloaded as f64 / total as f64).min(1.0);
            let _ = app.emit(event, frac);
        }
    }
    file.flush().map_err(|e| format!("flush error: {e}"))?;
    Ok(())
}

/// Download the portable exe (emitting `update-progress` 0..1), replace the running binary in
/// place, then relaunch. Does not return on success (the app restarts).
#[tauri::command]
pub async fn install_update(app: AppHandle, asset_url: String) -> Result<(), String> {
    let tmp = std::env::temp_dir().join("tkmodmanager-update.exe");
    download_to(&app, &asset_url, &tmp, "update-progress").await?;
    self_replace::self_replace(&tmp).map_err(|e| format!("could not replace executable: {e}"))?;
    let _ = std::fs::remove_file(&tmp);
    app.restart()
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
}
