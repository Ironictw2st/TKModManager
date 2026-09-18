//! Launch the game with a profile and, optionally, inject the script extender once the main
//! menu is up. Progress is reported through `launch-status` events.
//!
//! Flow: write `tkmm_mods.txt` → spawn `Three_Kingdoms.exe tkmm_mods.txt;` (cwd = game root,
//! Steam running) → (if the child exits at once, Steam relaunched it: find the new PID) →
//! wait for a visible main window → inject → watch the DLL log for the verified/mismatch lines.

use crate::commands::{list_file_path, list_input_for};
use crate::dll;
use crate::modlist;
use crate::options_pack;
use crate::packs;
use crate::paths;
use crate::profiles::Profile;
use crate::state::AppState;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LaunchStatus {
    /// idle | writing | spawned | menu | injected | verified | mismatch | failed | exited
    pub phase: String,
    pub message: String,
    pub pid: Option<u32>,
}

fn emit(app: &AppHandle, phase: &str, message: impl Into<String>, pid: Option<u32>) {
    let _ = app.emit("launch-status", LaunchStatus { phase: phase.into(), message: message.into(), pid });
}

#[tauri::command]
pub fn game_running() -> bool {
    !inject_core::find_pids(paths::EXE_NAME).is_empty()
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SaveGame {
    /// File stem, which is what `game_startup_mode campaign_load` expects.
    pub name: String,
    pub mtime: u64,
}

/// Campaign saves in `%APPDATA%\The Creative Assembly\ThreeKingdoms\save_games`, newest first.
#[tauri::command]
pub fn list_saves() -> Vec<SaveGame> {
    let Some(base) = directories::BaseDirs::new() else { return vec![] };
    let dir = base.data_dir().join("The Creative Assembly").join("ThreeKingdoms").join("save_games");
    let Ok(rd) = std::fs::read_dir(dir) else { return vec![] };
    let mut out: Vec<SaveGame> = rd
        .flatten()
        .filter(|e| e.path().extension().map(|x| x.eq_ignore_ascii_case("save")).unwrap_or(false))
        .filter_map(|e| {
            let name = e.path().file_stem()?.to_string_lossy().into_owned();
            let mtime = e
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            Some(SaveGame { name, mtime })
        })
        .collect();
    out.sort_by(|a, b| b.mtime.cmp(&a.mtime));
    out
}

/// Command-line tail after the exe: optional `game_startup_mode campaign_load "<save>" ;`
/// (the save argument is separate from the semicolon, as WH3 Mod Manager passes it), then the
/// list file with its trailing semicolon attached.
pub fn game_args(load_save: Option<&str>) -> String {
    match load_save.map(str::trim).filter(|s| !s.is_empty()) {
        Some(save) => format!("game_startup_mode campaign_load \"{}\" ; {};", save.replace('"', ""), paths::LIST_FILE_NAME),
        None => format!("{};", paths::LIST_FILE_NAME),
    }
}

/// Write the list file and spawn the game. Returns as soon as the process is started; the DLL
/// phase continues on a background thread.
#[tauri::command]
pub fn launch_game(app: AppHandle, profile: Profile, load_save: Option<String>) -> Result<u32, String> {
    if game_running() {
        return Err("Three_Kingdoms.exe is already running".into());
    }
    let state = app.state::<AppState>();
    let p = state.game_paths();
    let root = PathBuf::from(p.game_root.as_deref().ok_or("game folder not found; set it in Settings")?);
    let exe = root.join(paths::EXE_NAME);

    emit(&app, "writing", "Writing mod list", None);
    let scan = packs::scan(p.data_dir.as_deref().map(Path::new), p.workshop_dir.as_deref().map(Path::new));
    let mut input = list_input_for(&profile, &scan);
    if profile.skip_intro {
        let (dir, name) = options_pack::build(true)?;
        input.extra_dirs.push(dir.to_string_lossy().into_owned());
        input.extra_mods.push(name);
    }
    let text = modlist::build(&input);
    let list_path = list_file_path(&root);
    std::fs::write(&list_path, text).map_err(|e| format!("cannot write {}: {e}", list_path.display()))?;

    // A DLL is resolved before spawning so a missing/incompatible one is reported up front.
    let dll_path: Option<PathBuf> = if profile.dll {
        let status = dll::status_for(Some(&exe));
        match status.selected {
            Some(d) => Some(PathBuf::from(d.path)),
            None => {
                return Err(if status.installed.is_empty() {
                    "no script-extender DLL installed (Settings > Script extender)".into()
                } else {
                    "no installed DLL matches this game build; injection would be refused".into()
                });
            }
        }
    } else {
        None
    };

    let args = game_args(load_save.as_deref());
    let (pid, handle) = inject_core::spawn_game(&exe, &args, &root)?;
    inject_core::close_handle(handle);
    emit(&app, "spawned", format!("Game started (pid {pid})"), Some(pid));

    let app2 = app.clone();
    std::thread::spawn(move || follow(app2, pid, dll_path));
    Ok(pid)
}

/// Background: track the process, inject when the menu is up, then report from the DLL log.
fn follow(app: AppHandle, first_pid: u32, dll_path: Option<PathBuf>) {
    let mut pid = first_pid;
    // Steam may restart the game under its own launcher wrapper; recover the live PID.
    std::thread::sleep(Duration::from_secs(3));
    if !pid_alive(pid) {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let pids = inject_core::find_pids(paths::EXE_NAME);
            if let Some(&np) = pids.first() {
                pid = np;
                emit(&app, "spawned", format!("Game relaunched by Steam (pid {pid})"), Some(pid));
                break;
            }
            if Instant::now() >= deadline {
                emit(&app, "failed", "The game exited immediately (is Steam running?)", None);
                return;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    let Some(dll) = dll_path else {
        wait_exit(&app, pid);
        return;
    };

    emit(&app, "spawned", "Waiting for the main menu…", Some(pid));
    let ready = inject_core::wait_for_main_window(pid, Duration::from_secs(240));
    if !pid_alive(pid) {
        emit(&app, "exited", "Game exited before the menu", Some(pid));
        return;
    }
    if !ready {
        emit(&app, "failed", "Timed out waiting for the game window; DLL not injected", Some(pid));
        wait_exit(&app, pid);
        return;
    }
    emit(&app, "menu", "Main menu up; injecting DLL", Some(pid));

    // The log is truncated by the DLL on load; remember the pre-inject state to detect that.
    let log_path = dll.parent().map(|d| d.join(dll::LOG_NAME));
    match inject_core::inject(pid, &dll) {
        Ok(_) => emit(&app, "injected", "DLL loaded; waiting for verification", Some(pid)),
        Err(e) => {
            emit(&app, "failed", format!("Injection failed: {e}"), Some(pid));
            wait_exit(&app, pid);
            return;
        }
    }

    if let Some(log) = log_path {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            if let Ok(text) = std::fs::read_to_string(&log) {
                if text.contains("hook installed") || text.contains("bootstrap complete") {
                    emit(&app, "verified", "Script extender active", Some(pid));
                    break;
                }
                if text.contains("fingerprint mismatch") || text.contains("refusing to run") {
                    emit(&app, "mismatch", "DLL refused this game build (see DLL log)", Some(pid));
                    break;
                }
            }
            if !pid_alive(pid) {
                emit(&app, "exited", "Game exited", Some(pid));
                return;
            }
            if Instant::now() >= deadline {
                emit(&app, "injected", "DLL loaded, no verification line in the log yet", Some(pid));
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }
    wait_exit(&app, pid);
}

fn pid_alive(pid: u32) -> bool {
    inject_core::enumerate_processes().iter().any(|p| p.pid == pid)
}

fn wait_exit(app: &AppHandle, pid: u32) {
    while pid_alive(pid) {
        std::thread::sleep(Duration::from_secs(2));
    }
    emit(app, "exited", "Game exited", Some(pid));
}

#[cfg(test)]
mod tests {
    use super::game_args;

    #[test]
    fn args_with_and_without_save() {
        assert_eq!(game_args(None), "tkmm_mods.txt;");
        assert_eq!(game_args(Some("  ")), "tkmm_mods.txt;");
        assert_eq!(
            game_args(Some("Cao Cao turn 12")),
            "game_startup_mode campaign_load \"Cao Cao turn 12\" ; tkmm_mods.txt;"
        );
    }
}
