//! Where things live: the per-user app data folder, and Steam / game / Workshop discovery.

use serde::Serialize;
use std::path::{Path, PathBuf};

pub const STEAM_APP_ID: &str = "779340";
pub const EXE_NAME: &str = "Three_Kingdoms.exe";
pub const GAME_FOLDER: &str = "Total War THREE KINGDOMS";
/// The mod list file we write into the game root (CA's own `used_mods.txt` is left alone).
pub const LIST_FILE_NAME: &str = "tkmm_mods.txt";

/// `%APPDATA%\TKModManager` (Roaming) — settings, profiles, caches, downloaded DLLs.
pub fn app_data_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|b| b.data_dir().join("TKModManager"))
        .unwrap_or_else(|| PathBuf::from("TKModManager"))
}

/// Resolved game locations. `game_root` / `workshop_dir` are `None` when not found.
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct GamePaths {
    pub game_root: Option<String>,
    pub exe: Option<String>,
    pub data_dir: Option<String>,
    pub workshop_dir: Option<String>,
    /// Where the CA launcher's list lives (for import), if present.
    pub used_mods_file: Option<String>,
    /// FileVersion of the exe (e.g. "1.7.2.0"), when readable.
    pub exe_version: Option<String>,
}

/// Build the derived paths from a game root and (optionally) an explicit workshop dir.
pub fn resolve(game_root: Option<&Path>, workshop_override: Option<&Path>) -> GamePaths {
    let mut out = GamePaths::default();
    let Some(root) = game_root else { return out };
    let exe = root.join(EXE_NAME);
    if !exe.is_file() {
        return out;
    }
    out.game_root = Some(root.to_string_lossy().into_owned());
    out.exe = Some(exe.to_string_lossy().into_owned());
    out.data_dir = Some(root.join("data").to_string_lossy().into_owned());
    out.exe_version = crate::fingerprint::file_version(&exe);
    let used = root.join("used_mods.txt");
    if used.is_file() {
        out.used_mods_file = Some(used.to_string_lossy().into_owned());
    }
    // steamapps\common\<game> -> steamapps\workshop\content\779340
    let workshop = workshop_override.map(Path::to_path_buf).or_else(|| {
        root.parent()
            .and_then(Path::parent)
            .map(|steamapps| steamapps.join("workshop").join("content").join(STEAM_APP_ID))
    });
    if let Some(w) = workshop.filter(|w| w.is_dir()) {
        out.workshop_dir = Some(w.to_string_lossy().into_owned());
    }
    out
}

/// Find the game root by reading Steam's `libraryfolders.vdf` from every plausible Steam install.
pub fn detect_game_root() -> Option<PathBuf> {
    for steam in steam_roots() {
        let vdf = steam.join("steamapps").join("libraryfolders.vdf");
        if let Ok(text) = std::fs::read_to_string(&vdf) {
            if let Some(lib) = library_for_app(&text, STEAM_APP_ID) {
                let root = PathBuf::from(lib).join("steamapps").join("common").join(GAME_FOLDER);
                if root.join(EXE_NAME).is_file() {
                    return Some(root);
                }
            }
        }
        // Fallback: the default library of that Steam install.
        let root = steam.join("steamapps").join("common").join(GAME_FOLDER);
        if root.join(EXE_NAME).is_file() {
            return Some(root);
        }
    }
    None
}

/// Candidate Steam install folders: registry `SteamPath`, then the usual Program Files spots.
fn steam_roots() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(p) = registry_steam_path() {
        out.push(p);
    }
    for var in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Ok(pf) = std::env::var(var) {
            out.push(PathBuf::from(pf).join("Steam"));
        }
    }
    out.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
    out.dedup();
    out.into_iter().filter(|p| p.is_dir()).collect()
}

/// `HKCU\Software\Valve\Steam\SteamPath` via `reg.exe` (no registry crate needed).
fn registry_steam_path() -> Option<PathBuf> {
    use std::os::windows::process::CommandExt;
    // CREATE_NO_WINDOW: a console child of a GUI app would otherwise flash a command prompt.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("reg")
        .args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().find(|l| l.contains("SteamPath"))?;
    let value = line.split("REG_SZ").nth(1)?.trim();
    if value.is_empty() {
        return None;
    }
    Some(PathBuf::from(value.replace('/', "\\")))
}

/// Scan a `libraryfolders.vdf` for the library whose `apps` block lists `app_id`.
/// Each library block has a `"path"` line followed (later) by its `"apps"` map.
pub fn library_for_app(vdf: &str, app_id: &str) -> Option<String> {
    let mut current: Option<String> = None;
    let mut in_apps = false;
    let needle = format!("\"{app_id}\"");
    for raw in vdf.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("\"path\"") {
            let v = rest.trim().trim_matches('\u{22}').replace("\\\\", "\\");
            current = Some(v);
            in_apps = false;
        } else if line == "\"apps\"" {
            in_apps = true;
        } else if in_apps && line == "}" {
            in_apps = false;
        } else if in_apps && line.starts_with(&needle) {
            if let Some(c) = &current {
                return Some(c.clone());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_library_holding_app() {
        let vdf = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"C:\\\\Program Files (x86)\\\\Steam\"\n\t\t\"apps\"\n\t\t{\n\t\t\t\"228980\"\t\t\"1\"\n\t\t}\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"E:\\\\SteamLibrary\"\n\t\t\"apps\"\n\t\t{\n\t\t\t\"779340\"\t\t\"2\"\n\t\t}\n\t}\n}\n";
        assert_eq!(library_for_app(vdf, "779340").as_deref(), Some(r"E:\SteamLibrary"));
        assert_eq!(library_for_app(vdf, "228980").as_deref(), Some(r"C:\Program Files (x86)\Steam"));
        assert!(library_for_app(vdf, "1").is_none());
    }
}
