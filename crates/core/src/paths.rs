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

/// Which store the install came from. Only Steam has a Workshop folder.
#[derive(Serialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GameStore {
    Steam,
    Epic,
    GamePass,
    #[default]
    Unknown,
}

impl GameStore {
    pub fn label(self) -> &'static str {
        match self {
            GameStore::Steam => "Steam",
            GameStore::Epic => "Epic",
            GameStore::GamePass => "Game Pass",
            GameStore::Unknown => "unknown store",
        }
    }
}

/// Tell the store from an install folder. Epic installs ship `steam_api64.dll` too, so the
/// `.egstore` folder is checked first.
pub fn store_of(root: &Path) -> GameStore {
    let lower = root.to_string_lossy().to_lowercase().replace('/', "\\");
    if root.join(".egstore").is_dir() {
        GameStore::Epic
    } else if lower.contains("\\steamapps\\") {
        GameStore::Steam
    } else if lower.contains("\\xboxgames\\") || lower.contains("\\windowsapps\\") {
        GameStore::GamePass
    } else if root.join("steam_api64.dll").is_file() {
        GameStore::Steam
    } else {
        GameStore::Unknown
    }
}

/// Resolved game locations. `game_root` / `workshop_dir` are `None` when not found.
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct GamePaths {
    pub game_root: Option<String>,
    pub store: GameStore,
    pub exe: Option<String>,
    pub data_dir: Option<String>,
    /// Steam only: `steamapps\workshop\content\779340`.
    pub workshop_dir: Option<String>,
    /// `<game root>\mods`, where the Epic and Game Pass builds keep downloaded mods.
    pub mods_dir: Option<String>,
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
    out.store = store_of(root);
    out.exe = Some(exe.to_string_lossy().into_owned());
    out.data_dir = Some(root.join("data").to_string_lossy().into_owned());
    out.exe_version = crate::fingerprint::file_version(&exe);
    let used = root.join("used_mods.txt");
    if used.is_file() {
        out.used_mods_file = Some(used.to_string_lossy().into_owned());
    }
    let mods = root.join("mods");
    if mods.is_dir() {
        out.mods_dir = Some(mods.to_string_lossy().into_owned());
    }
    // Only Steam has a Workshop: steamapps\common\<game> -> steamapps\workshop\content\779340.
    // The override is Steam's too, so an Epic copy never picks up Steam's Workshop folder.
    if out.store != GameStore::Steam {
        return out;
    }
    let workshop = workshop_override
        .map(Path::to_path_buf)
        .or_else(|| root.parent().and_then(Path::parent).map(|steamapps| steamapps.join("workshop").join("content").join(STEAM_APP_ID)));
    if let Some(w) = workshop.filter(|w| w.is_dir()) {
        out.workshop_dir = Some(w.to_string_lossy().into_owned());
    }
    out
}

/// One copy of the game found on this PC.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameInstall {
    pub store: GameStore,
    pub root: String,
    pub exe_version: Option<String>,
}

impl GameInstall {
    /// "Epic — Z:\Epic Games\TotalWarTHREEKINGDOMS"
    pub fn label(&self) -> String {
        format!("{} — {}", self.store.label(), self.root)
    }
}

/// Every copy of the game on this PC, Steam first, then Epic, then Game Pass. The user picks
/// which one the manager works with (Settings), so both stores can be kept side by side.
pub fn detect_installs() -> Vec<GameInstall> {
    let mut out: Vec<GameInstall> = Vec::new();
    for root in steam_game_roots().into_iter().chain(epic_game_roots()).chain(gamepass_game_roots()) {
        let key = root.to_string_lossy().trim_end_matches('\\').to_lowercase();
        if out.iter().any(|i| i.root.trim_end_matches('\\').to_lowercase() == key) {
            continue;
        }
        out.push(GameInstall {
            store: store_of(&root),
            exe_version: crate::fingerprint::file_version(&root.join(EXE_NAME)),
            root: root.to_string_lossy().into_owned(),
        });
    }
    out
}

/// The install used when the user has not chosen one.
pub fn detect_game_root() -> Option<PathBuf> {
    detect_installs().first().map(|i| PathBuf::from(&i.root))
}

/// The Epic launcher's install manifests (`*.item`, JSON).
fn epic_manifest_dir() -> PathBuf {
    let pd = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into());
    PathBuf::from(pd).join("Epic").join("EpicGamesLauncher").join("Data").join("Manifests")
}

/// What an Epic `.item` manifest says about one installed app.
#[derive(Clone, Debug, PartialEq)]
pub struct EpicApp {
    pub install_location: String,
    /// `<namespace>:<catalog item>:<app name>`, which is what the launcher URI takes.
    pub launch_id: String,
}

/// Parse an Epic `.item` manifest. Its `LaunchExecutable` is CA's `Launcher.exe`, not the
/// game, so the caller checks for the game exe in `install_location`.
pub fn epic_app(item_json: &str) -> Option<EpicApp> {
    let v: serde_json::Value = serde_json::from_str(item_json).ok()?;
    let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
    let install_location = v["InstallLocation"].as_str().filter(|s| !s.is_empty())?.replace('/', "\\");
    Some(EpicApp { install_location, launch_id: format!("{}:{}:{}", s("CatalogNamespace"), s("CatalogItemId"), s("AppName")) })
}

/// The Epic app installed in `root`, if the launcher knows about it.
pub fn epic_app_for(root: &Path) -> Option<EpicApp> {
    let want = root.to_string_lossy().trim_end_matches('\\').to_lowercase();
    std::fs::read_dir(epic_manifest_dir())
        .ok()?
        .flatten()
        .filter(|e| e.path().extension().map(|x| x.eq_ignore_ascii_case("item")).unwrap_or(false))
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .filter_map(|t| epic_app(&t))
        .find(|a| a.install_location.trim_end_matches('\\').to_lowercase() == want)
}

/// `com.epicgames.launcher://` URI that starts an installed Epic app.
pub fn epic_launch_uri(app: &EpicApp) -> String {
    format!("com.epicgames.launcher://apps/{}?action=launch&silent=true", app.launch_id.replace(':', "%3A"))
}

/// Per-user game folder (saves, scripts). The Epic build keeps everything one level deeper.
pub fn game_user_dir(store: GameStore) -> Option<PathBuf> {
    let base = directories::BaseDirs::new()?.data_dir().join("The Creative Assembly").join("ThreeKingdoms");
    Some(if store == GameStore::Epic { base.join("EOS") } else { base })
}

/// `scripts\user.script.txt`: read by the game on every start, whatever list file it is given.
/// This is how a mod list reaches the Epic build, which only Epic's launcher may start.
pub fn user_script_path(store: GameStore) -> Option<PathBuf> {
    game_user_dir(store).map(|d| d.join("scripts").join("user.script.txt"))
}

/// CA launcher's own mod selection for this store (`mods.json`).
pub fn ca_mods_json(store: GameStore) -> Option<PathBuf> {
    game_user_dir(store).map(|d| d.join("mods.json"))
}

fn epic_game_roots() -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(epic_manifest_dir()) else { return vec![] };
    rd.flatten()
        .filter(|e| e.path().extension().map(|x| x.eq_ignore_ascii_case("item")).unwrap_or(false))
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .filter_map(|t| epic_app(&t))
        .map(|a| PathBuf::from(a.install_location))
        .filter(|root| root.join(EXE_NAME).is_file())
        .collect()
}

/// Game Pass installs to `<drive>:\XboxGames\<game>\Content` by default. Best effort: custom
/// install folders are not searched (set the folder in Settings instead).
fn gamepass_game_roots() -> Vec<PathBuf> {
    ('C'..='Z')
        .map(|d| PathBuf::from(format!("{d}:\\XboxGames")))
        .filter(|p| p.is_dir())
        .filter_map(|p| std::fs::read_dir(p).ok())
        .flat_map(|rd| rd.flatten().map(|e| e.path().join("Content")).collect::<Vec<_>>())
        .filter(|root| root.join(EXE_NAME).is_file())
        .collect()
}

/// Game folders from Steam's `libraryfolders.vdf`, for every plausible Steam install.
fn steam_game_roots() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for steam in steam_roots() {
        let vdf = steam.join("steamapps").join("libraryfolders.vdf");
        if let Ok(text) = std::fs::read_to_string(&vdf) {
            if let Some(lib) = library_for_app(&text, STEAM_APP_ID) {
                out.push(PathBuf::from(lib).join("steamapps").join("common").join(GAME_FOLDER));
            }
        }
        // Fallback: the default library of that Steam install.
        out.push(steam.join("steamapps").join("common").join(GAME_FOLDER));
    }
    out.retain(|root| root.join(EXE_NAME).is_file());
    out
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

    #[test]
    fn reads_epic_manifest() {
        let item = r#"{"DisplayName":"Total War: THREE KINGDOMS","AppName":"769f2fee","CatalogNamespace":"817686bf","CatalogItemId":"861fda0f","LaunchExecutable":"Launcher.exe","InstallLocation":"Z:/Epic Games/TotalWarTHREEKINGDOMS"}"#;
        let app = epic_app(item).unwrap();
        assert_eq!(app.install_location, r"Z:\Epic Games\TotalWarTHREEKINGDOMS");
        assert_eq!(app.launch_id, "817686bf:861fda0f:769f2fee");
        assert_eq!(epic_launch_uri(&app), "com.epicgames.launcher://apps/817686bf%3A861fda0f%3A769f2fee?action=launch&silent=true");
        assert!(epic_app(r#"{"InstallLocation":""}"#).is_none());
        assert!(epic_app("not json").is_none());
    }

    #[test]
    fn tells_store_from_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let epic = tmp.path().join("TotalWarTHREEKINGDOMS");
        std::fs::create_dir_all(epic.join(".egstore")).unwrap();
        std::fs::write(epic.join("steam_api64.dll"), "").unwrap();
        assert_eq!(store_of(&epic), GameStore::Epic);
        assert_eq!(store_of(Path::new(r"D:\SteamLibrary\steamapps\common\Total War THREE KINGDOMS")), GameStore::Steam);
        assert_eq!(store_of(Path::new(r"E:\XboxGames\Total War THREE KINGDOMS\Content")), GameStore::GamePass);
        assert_eq!(store_of(&tmp.path().join("elsewhere")), GameStore::Unknown);
    }
}
