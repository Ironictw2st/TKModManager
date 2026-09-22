//! "Force update" for Workshop items: ask Steam to download the newest version of an item now
//! (ISteamUGC::DownloadItem, high priority) instead of whenever it next gets round to it.
//!
//! The Steam API ties the calling process to an app id and makes Steam show the user as
//! playing it, so this never runs inside the manager itself: the manager starts a copy of
//! itself with `--steam-download <id,id,...>`, which loads Valve's redistributable
//! `steam_api64.dll` (shipped next to the exe) at run time, does the downloads, prints one JSON
//! line per event on stdout and exits. Loading the DLL dynamically keeps it out of the
//! manager's import table, so a missing DLL only disables this feature.

use core::ffi::c_void;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const HELPER_FLAG: &str = "--steam-download";
pub const DLL_NAME: &str = "steam_api64.dll";

const STATE_SUBSCRIBED: u32 = 1;
const STATE_INSTALLED: u32 = 4;
const STATE_NEEDS_UPDATE: u32 = 8;
const STATE_DOWNLOADING: u32 = 16;
const STATE_DOWNLOAD_PENDING: u32 = 32;

/// One line of the helper's output.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum Event {
    Progress { id: u64, done: u64, total: u64 },
    Finished { id: u64, ok: bool, message: String },
    /// Steam could not be used at all (not running, DLL missing, ...).
    Fatal { message: String },
}

impl Event {
    pub fn to_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
    pub fn from_line(line: &str) -> Option<Event> {
        serde_json::from_str(line.trim()).ok()
    }
}

/// `steam_api64.dll` next to the running exe.
pub fn dll_path() -> Option<PathBuf> {
    let p = std::env::current_exe().ok()?.parent()?.join(DLL_NAME);
    p.is_file().then_some(p)
}

type Hmodule = *mut c_void;

extern "system" {
    fn LoadLibraryW(name: *const u16) -> Hmodule;
    fn GetProcAddress(module: Hmodule, name: *const u8) -> *mut c_void;
}

type InitFlat = unsafe extern "C" fn(err: *mut u8) -> i32;
type NoArgs = unsafe extern "C" fn();
type GetUgc = unsafe extern "C" fn() -> *mut c_void;
type DownloadItem = unsafe extern "C" fn(ugc: *mut c_void, id: u64, high_priority: bool) -> bool;
type GetItemState = unsafe extern "C" fn(ugc: *mut c_void, id: u64) -> u32;
type GetItemDownloadInfo = unsafe extern "C" fn(ugc: *mut c_void, id: u64, done: *mut u64, total: *mut u64) -> bool;

/// An initialised Steam API session (shut down on drop).
pub struct Steam {
    ugc: *mut c_void,
    shutdown: NoArgs,
    run_callbacks: NoArgs,
    download_item: DownloadItem,
    item_state: GetItemState,
    download_info: GetItemDownloadInfo,
}

unsafe fn sym<T>(lib: Hmodule, name: &str) -> Result<T, String> {
    let c = format!("{name}\0");
    let p = GetProcAddress(lib, c.as_ptr());
    if p.is_null() {
        return Err(format!("{DLL_NAME} has no {name} (outdated DLL?)"));
    }
    Ok(std::mem::transmute_copy::<*mut c_void, T>(&p))
}

impl Steam {
    /// Load the DLL and connect to the running Steam client as Three Kingdoms. The caller must
    /// have `SteamAppId=779340` in the environment.
    pub fn init(dll: &Path) -> Result<Steam, String> {
        let wide: Vec<u16> = dll.as_os_str().to_string_lossy().encode_utf16().chain(Some(0)).collect();
        unsafe {
            let lib = LoadLibraryW(wide.as_ptr());
            if lib.is_null() {
                return Err(format!("cannot load {}", dll.display()));
            }
            let init: InitFlat = sym(lib, "SteamAPI_InitFlat")?;
            let get_ugc: GetUgc = sym(lib, "SteamAPI_SteamUGC_v021")?;
            let shutdown: NoArgs = sym(lib, "SteamAPI_Shutdown")?;
            let run_callbacks: NoArgs = sym(lib, "SteamAPI_RunCallbacks")?;
            let download_item: DownloadItem = sym(lib, "SteamAPI_ISteamUGC_DownloadItem")?;
            let item_state: GetItemState = sym(lib, "SteamAPI_ISteamUGC_GetItemState")?;
            let download_info: GetItemDownloadInfo = sym(lib, "SteamAPI_ISteamUGC_GetItemDownloadInfo")?;
            let mut err = [0u8; 1024];
            let code = init(err.as_mut_ptr());
            if code != 0 {
                let len = err.iter().position(|b| *b == 0).unwrap_or(0);
                let text = String::from_utf8_lossy(&err[..len]).into_owned();
                return Err(match code {
                    2 => "Steam is not running (start Steam and try again)".into(),
                    3 => "the Steam client is out of date".into(),
                    _ if !text.is_empty() => format!("Steam: {text}"),
                    _ => format!("Steam API init failed ({code})"),
                });
            }
            let ugc = get_ugc();
            if ugc.is_null() {
                shutdown();
                return Err("the Steam client does not offer the Workshop interface this build needs".into());
            }
            // Only a fully initialised session exists as a `Steam`, so `Drop` (SteamAPI_Shutdown)
            // runs exactly once, at the end of the session.
            Ok(Steam { ugc, shutdown, run_callbacks, download_item, item_state, download_info })
        }
    }

    fn state(&self, id: u64) -> u32 {
        unsafe { (self.item_state)(self.ugc, id) }
    }

    fn progress(&self, id: u64) -> Option<(u64, u64)> {
        let (mut done, mut total) = (0u64, 0u64);
        unsafe { (self.download_info)(self.ugc, id, &mut done, &mut total) }.then_some((done, total))
    }

    /// Queue high-priority downloads and follow them until each one settles or `timeout`
    /// passes. Emits progress and one `Finished` per id.
    pub fn force_download(&self, ids: &[u64], timeout: Duration, emit: &mut dyn FnMut(Event)) {
        struct Job {
            id: u64,
            started: Instant,
            seen_busy: bool,
            last: (u64, u64),
            finished: bool,
        }
        let mut jobs: Vec<Job> = Vec::new();
        for &id in ids {
            if unsafe { (self.download_item)(self.ugc, id, true) } {
                jobs.push(Job { id, started: Instant::now(), seen_busy: false, last: (0, 0), finished: false });
            } else {
                emit(Event::Finished { id, ok: false, message: "Steam refused the download (is the item still subscribed?)".into() });
            }
        }
        let begin = Instant::now();
        while jobs.iter().any(|j| !j.finished) {
            unsafe { (self.run_callbacks)() };
            for j in jobs.iter_mut().filter(|j| !j.finished) {
                let state = self.state(j.id);
                let busy = state & (STATE_DOWNLOADING | STATE_DOWNLOAD_PENDING) != 0;
                j.seen_busy |= busy;
                if let Some(p) = self.progress(j.id).filter(|p| p.1 > 0 && *p != j.last) {
                    j.last = p;
                    emit(Event::Progress { id: j.id, done: p.0, total: p.1 });
                }
                if let Some((ok, message)) = settled(state, j.seen_busy, j.started.elapsed()) {
                    j.finished = true;
                    emit(Event::Finished { id: j.id, ok, message });
                }
            }
            if begin.elapsed() > timeout {
                for j in jobs.iter_mut().filter(|j| !j.finished) {
                    j.finished = true;
                    emit(Event::Finished { id: j.id, ok: false, message: "timed out; Steam may still finish it in the background".into() });
                }
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }
}

impl Drop for Steam {
    fn drop(&mut self) {
        unsafe { (self.shutdown)() };
    }
}

/// Has a download settled, given the item state? Steam may take a moment to flag a freshly
/// queued item as pending, so an idle state only counts once it was seen busy or a few
/// seconds have passed.
pub fn settled(state: u32, seen_busy: bool, elapsed: Duration) -> Option<(bool, String)> {
    let busy = state & (STATE_DOWNLOADING | STATE_DOWNLOAD_PENDING) != 0;
    if busy || (!seen_busy && elapsed < Duration::from_secs(5)) {
        return None;
    }
    if state & STATE_INSTALLED != 0 && state & STATE_NEEDS_UPDATE == 0 {
        let msg = if seen_busy { "updated" } else { "already up to date" };
        return Some((true, msg.into()));
    }
    if state & STATE_SUBSCRIBED == 0 {
        return Some((false, "you are not subscribed to this item (unsubscribed or deleted?)".into()));
    }
    Some((false, format!("Steam did not finish the download (item state {state:#x})")))
}

/// Body of the helper process: `ids` from the command line, events on stdout.
pub fn run_helper(ids: &[u64]) -> i32 {
    let mut out = |e: Event| {
        use std::io::Write;
        let mut so = std::io::stdout().lock();
        let _ = writeln!(so, "{}", e.to_line());
        let _ = so.flush();
    };
    let Some(dll) = dll_path() else {
        out(Event::Fatal { message: format!("{DLL_NAME} is missing next to the manager") });
        return 2;
    };
    match Steam::init(&dll) {
        Ok(steam) => {
            steam.force_download(ids, Duration::from_secs(15 * 60), &mut out);
            0
        }
        Err(message) => {
            out(Event::Fatal { message });
            1
        }
    }
}

/// Parse the helper's argument: comma-separated Workshop ids.
pub fn parse_ids(arg: &str) -> Vec<u64> {
    arg.split(',').filter_map(|s| s.trim().parse().ok()).collect()
}

/// Run the helper for `ids` and forward its events. Blocking; for a worker thread.
pub fn spawn_helper(ids: &[u64], on_event: &mut dyn FnMut(Event)) -> Result<(), String> {
    use std::io::BufRead;
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    if dll_path().is_none() {
        return Err(format!("{DLL_NAME} is missing next to the manager; reinstall it to use Force update"));
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let list = ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
    let mut child = std::process::Command::new(exe)
        .args([HELPER_FLAG, &list])
        .env("SteamAppId", crate::paths::STEAM_APP_ID)
        .env("SteamGameId", crate::paths::STEAM_APP_ID)
        .stdout(std::process::Stdio::piped())
        .stdin(std::process::Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("cannot start the Steam helper: {e}"))?;
    let stdout = child.stdout.take().ok_or("helper has no output")?;
    let mut finished = 0;
    for line in std::io::BufReader::new(stdout).lines().map_while(Result::ok) {
        if let Some(ev) = Event::from_line(&line) {
            match &ev {
                Event::Fatal { message } => {
                    let _ = child.wait();
                    return Err(message.clone());
                }
                Event::Finished { .. } => finished += 1,
                Event::Progress { .. } => {}
            }
            on_event(ev);
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if finished < ids.len() {
        return Err(format!("the Steam helper stopped early ({status})"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_round_trip() {
        for e in [
            Event::Progress { id: 1, done: 5, total: 10 },
            Event::Finished { id: 2, ok: true, message: "updated".into() },
            Event::Fatal { message: "no steam".into() },
        ] {
            assert_eq!(Event::from_line(&e.to_line()), Some(e));
        }
        assert_eq!(Event::from_line("garbage"), None);
    }

    #[test]
    fn settle_rules() {
        let s = Duration::from_secs;
        assert_eq!(settled(STATE_INSTALLED | STATE_DOWNLOADING, true, s(1)), None);
        // Freshly queued, not flagged yet: wait.
        assert_eq!(settled(STATE_INSTALLED | STATE_NEEDS_UPDATE, false, s(1)), None);
        assert_eq!(settled(STATE_INSTALLED, true, s(1)).map(|r| r.0), Some(true));
        assert_eq!(settled(STATE_INSTALLED, false, s(6)).map(|r| r.1), Some("already up to date".to_string()));
        assert_eq!(settled(STATE_SUBSCRIBED | STATE_INSTALLED | STATE_NEEDS_UPDATE, true, s(9)).map(|r| r.0), Some(false));
        assert!(settled(0, true, s(9)).unwrap().1.contains("not subscribed"));
    }

    #[test]
    fn parses_id_lists() {
        assert_eq!(parse_ids("1, 22,x,333"), vec![1, 22, 333]);
    }
}
