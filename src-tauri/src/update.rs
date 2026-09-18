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
    let version = json["tag_name"].as_str().unwrap_or("").trim_start_matches('v').to_string();
    if version.is_empty() {
        return Err("latest release has no tag_name".into());
    }
    let notes = json["body"].as_str().unwrap_or("").to_string();
    let assets = json["assets"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    Some((a["name"].as_str()?.to_string(), a["browser_download_url"].as_str()?.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Release { version, notes, assets })
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
