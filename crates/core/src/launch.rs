//! Launch the game with a profile and, optionally, inject the script extender once the main
//! menu is up. Progress is reported through `launch-status` events.
//!
//! Flow: write `tkmm_mods.txt` → spawn `Three_Kingdoms.exe tkmm_mods.txt;` (cwd = game root,
//! Steam running) → (if the child exits at once, Steam relaunched it: find the new PID) →
//! wait for a visible main window → inject (never twice) → watch the DLL log → wait for exit
//! and record the exit code in the launch history.

use crate::ops::{list_file_path, list_input_for};
use crate::dll;
use crate::history;
use crate::modlist;
use crate::options_pack;
use crate::paths;
use crate::profiles::Profile;
use crate::context::{AppContext, Ctx};
use crate::winproc::{self, ProcHandle};
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::sync::Arc;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LaunchStatus {
    /// idle | writing | spawned | menu | injected | verified | mismatch | failed | exited | crashed
    pub phase: String,
    pub message: String,
    pub pid: Option<u32>,
    /// Process exit code (phases `exited` / `crashed`).
    pub exit_code: Option<u32>,
    /// Launch history record for this run (for the crash details).
    pub history_id: Option<u64>,
}

/// Receives every launch status change (called from worker threads).
pub type Emit = Arc<dyn Fn(LaunchStatus) + Send + Sync>;

fn emit(out: &Emit, phase: &str, message: impl Into<String>, pid: Option<u32>) {
    emit_full(out, phase, message, pid, None, None);
}

fn emit_full(out: &Emit, phase: &str, message: impl Into<String>, pid: Option<u32>, exit_code: Option<u32>, history_id: Option<u64>) {
    out(LaunchStatus { phase: phase.into(), message: message.into(), pid, exit_code, history_id });
}

/// Game processes this app is already following (launched here or picked up by the watcher).
#[derive(Default)]
pub struct LaunchTracker {
    managed: Mutex<HashSet<u32>>,
}

impl LaunchTracker {
    fn claim(&self, pid: u32) -> bool {
        self.managed.lock().map(|mut s| s.insert(pid)).unwrap_or(false)
    }
    fn release(&self, pid: u32) {
        if let Ok(mut s) = self.managed.lock() {
            s.remove(&pid);
        }
    }
}

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
        Some(save) => format!(
            "game_startup_mode campaign_load \"{}\" ; {};",
            save.replace('\u{22}', ""),
            paths::LIST_FILE_NAME
        ),
        None => format!("{};", paths::LIST_FILE_NAME),
    }
}

/// Write the list file and spawn the game. Returns as soon as the process is started; the DLL
/// phase and the exit tracking continue on a background thread.
pub fn launch_game(ctx: &Ctx, out: &Emit, profile: &Profile, load_save: Option<&str>) -> Result<u32, String> {
    if game_running() {
        return Err("Three_Kingdoms.exe is already running".into());
    }
    let state = ctx;
    let p = state.game_paths();
    let root = PathBuf::from(p.game_root.as_deref().ok_or("game folder not found; set it in Settings")?);
    let exe = root.join(paths::EXE_NAME);

    emit(out, "writing", "Writing mod list", None);
    let scan = state.scan();
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

    let enabled: Vec<&crate::packs::ModEntry> = profile
        .entries
        .iter()
        .filter(|e| e.enabled)
        .filter_map(|e| scan.iter().find(|m| m.key == e.key))
        .collect();
    let history_id = history::begin(&profile.name, history::snapshot(&enabled));

    let args = game_args(load_save);
    let (pid, handle) = match inject_core::spawn_game(&exe, &args, &root) {
        Ok(v) => v,
        Err(e) => {
            history::finish(history_id, None);
            return Err(e);
        }
    };
    inject_core::close_handle(handle);
    ctx.tracker.claim(pid);
    emit_full(out, "spawned", format!("Game started (pid {pid})"), Some(pid), None, Some(history_id));

    let (ctx2, out2) = (ctx.clone(), out.clone());
    std::thread::spawn(move || follow(ctx2, out2, pid, dll_path, Some(history_id)));
    Ok(pid)
}

/// Background: track the process, inject when the menu is up, then wait for the exit.
fn follow(ctx: Ctx, out: Emit, first_pid: u32, dll_path: Option<PathBuf>, history_id: Option<u64>) {
    let mut pid = first_pid;
    // Steam may restart the game under its own launcher wrapper; recover the live PID.
    std::thread::sleep(Duration::from_secs(3));
    if !pid_alive(pid) {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let pids = inject_core::find_pids(paths::EXE_NAME);
            if let Some(&np) = pids.first() {
                ctx.tracker.release(pid);
                pid = np;
                ctx.tracker.claim(pid);
                emit_full(&out, "spawned", format!("Game relaunched by Steam (pid {pid})"), Some(pid), None, history_id);
                break;
            }
            if Instant::now() >= deadline {
                emit_full(&out, "failed", "The game exited immediately (is Steam running?)", None, None, history_id);
                if let Some(id) = history_id {
                    history::finish(id, None);
                }
                ctx.tracker.release(first_pid);
                return;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    // Opened before injecting so the exit code is readable even if the game dies early.
    let handle = ProcHandle::open(pid);
    if let Some(dll) = dll_path {
        if handle.as_ref().map(ProcHandle::is_running).unwrap_or(true) {
            inject_flow(&out, pid, &dll, false);
        }
    }
    finish(&ctx, &out, pid, handle, history_id);
}

/// Wait for the menu, inject once, and report what the DLL log says. `external` = the game
/// was not started by us, so an unreadable module list means "do not touch".
fn inject_flow(out: &Emit, pid: u32, dll: &Path, external: bool) {
    emit(out, "spawned", "Waiting for the main menu…", Some(pid));
    let ready = inject_core::wait_for_main_window(pid, Duration::from_secs(240));
    if !pid_alive(pid) {
        return;
    }
    if !ready {
        emit(out, "failed", "Timed out waiting for the game window; DLL not injected", Some(pid));
        return;
    }
    // Never stack a second copy on a process that already has the hook (HANDOFF rule).
    match winproc::has_module(pid, dll::DLL_NAME) {
        Some(true) => {
            emit(out, "verified", "Script extender already loaded in this game; not injecting again", Some(pid));
            return;
        }
        None if external => {
            emit(out, "failed", "Could not inspect the game process; DLL not injected", Some(pid));
            return;
        }
        _ => {}
    }
    emit(out, "menu", "Main menu up; injecting DLL", Some(pid));
    match inject_core::inject(pid, dll) {
        Ok(_) => emit(out, "injected", "DLL loaded; waiting for verification", Some(pid)),
        Err(e) => {
            emit(out, "failed", format!("Injection failed: {e}"), Some(pid));
            return;
        }
    }
    let Some(log) = dll.parent().map(|d| d.join(dll::LOG_NAME)) else { return };
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if let Ok(text) = std::fs::read_to_string(&log) {
            if text.contains("hook installed") || text.contains("bootstrap complete") {
                emit(out, "verified", "Script extender active", Some(pid));
                return;
            }
            if text.contains("fingerprint mismatch") || text.contains("refusing to run") {
                emit(out, "mismatch", "DLL refused this game build (see the Logs tab)", Some(pid));
                return;
            }
        }
        if !pid_alive(pid) {
            return;
        }
        if Instant::now() >= deadline {
            emit(out, "injected", "DLL loaded, no verification line in the log yet", Some(pid));
            return;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

/// Block until the game ends, then record and report how it ended.
fn finish(ctx: &AppContext, out: &Emit, pid: u32, handle: Option<ProcHandle>, history_id: Option<u64>) {
    let exit_code = match handle {
        Some(h) => h.wait_exit(),
        None => {
            while pid_alive(pid) {
                std::thread::sleep(Duration::from_secs(2));
            }
            None
        }
    };
    if let Some(id) = history_id {
        history::finish(id, exit_code);
    }
    ctx.tracker.release(pid);
    match exit_code {
        Some(code) if code != 0 => emit_full(
            out,
            "crashed",
            format!("Game ended abnormally (exit code 0x{code:X})"),
            Some(pid),
            exit_code,
            history_id,
        ),
        _ => emit_full(out, "exited", "Game exited", Some(pid), exit_code, history_id),
    }
}

fn pid_alive(pid: u32) -> bool {
    inject_core::enumerate_processes().iter().any(|p| p.pid == pid)
}

/// Watch for games started outside the manager and inject into them when the user enabled
/// `auto_inject_external`. Runs for the life of the app.
pub fn start_external_watcher(ctx: Ctx, out: Emit) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(3));
        let state = &ctx;
        if !state.settings().auto_inject_external {
            continue;
        }
        for pid in inject_core::find_pids(paths::EXE_NAME) {
            if !ctx.tracker.claim(pid) {
                continue; // already followed (ours, or picked up earlier)
            }
            let exe = state.game_paths().exe.map(PathBuf::from);
            let selected = dll::status_for(exe.as_deref()).selected;
            let (ctx2, out2) = (ctx.clone(), out.clone());
            std::thread::spawn(move || {
                let handle = ProcHandle::open(pid);
                match selected {
                    Some(d) if winproc::has_module(pid, dll::DLL_NAME) == Some(false) => {
                        emit(&out2, "spawned", format!("Game started outside the manager (pid {pid})"), Some(pid));
                        inject_flow(&out2, pid, Path::new(&d.path), true);
                    }
                    Some(_) => {}
                    None => emit(&out2, "failed", "Game detected, but no DLL matches this game build", Some(pid)),
                }
                finish(&ctx2, &out2, pid, handle, None);
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_with_and_without_save() {
        assert_eq!(game_args(None), "tkmm_mods.txt;");
        assert_eq!(game_args(Some("  ")), "tkmm_mods.txt;");
        assert_eq!(
            game_args(Some("Cao Cao turn 12")),
            "game_startup_mode campaign_load \"Cao Cao turn 12\" ; tkmm_mods.txt;"
        );
    }

    #[test]
    fn tracker_claims_once() {
        let t = LaunchTracker::default();
        assert!(t.claim(42));
        assert!(!t.claim(42));
        t.release(42);
        assert!(t.claim(42));
    }
}
