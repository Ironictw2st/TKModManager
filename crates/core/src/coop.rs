//! Three Kingdoms Coop (github.com/Thom-Ernst/ThreeKingdoms-Coop): 3-4 player campaigns.
//!
//! Coop has two parts. Its own Setup (`TW3K-Coop-Setup-<ver>.exe`, Inno, admin) replaces the
//! game's `amd_ags_x64.dll` with a forwarding proxy, keeps the original as
//! `amd_ags_x64_orig.dll` and adds `tw3k_coop.dll`; the game then loads Coop at process start on
//! every launch, ours included. The `tw3k_coop.pack` Workshop item is an ordinary mod.
//!
//! The manager only reads: it detects the install, checks GitHub for a newer Setup, downloads it
//! (SHA256 checked against the release's `SHA256SUMS.txt`) and lets the player run it. It never
//! writes into the game folder itself.

use crate::fingerprint::ExeFingerprint;
use crate::paths::{self, GameStore};
use crate::update::{self, Release};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const OWNER: &str = "Thom-Ernst";
pub const REPO: &str = "ThreeKingdoms-Coop";
pub const WORKSHOP_ID: &str = "3816463449";
pub const PACK: &str = "tw3k_coop.pack";
pub const RUNTIME_DLL: &str = "tw3k_coop.dll";
const PROXY_DLL: &str = "amd_ags_x64.dll";
const ORIGINAL_DLL: &str = "amd_ags_x64_orig.dll";
/// Export name only Coop's proxy carries (its Setup checks for the same string).
const PROXY_MARKER: &[u8] = b"Tw3kCoopProxyMarker";
/// Shipped only by the Debug ("Developer Build") Setup.
const DEBUG_CLIENT: &str = "TW3K-Coop-Recovery.cmd";
/// Inno's uninstall key: `AppId` from `tools/installer/TW3K-Coop.iss` + `_is1`.
const UNINSTALL_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\TW3K-Coop-65FBEE59-07CD-4DA0-BEF0-6E47FD5C94A8_is1";
const SUMS_ASSET: &str = "SHA256SUMS.txt";
/// The only build Coop's runtime arms on (Steam 1.7.2); anything else leaves it inert.
pub const STEAM_TIMESTAMP: u32 = 0x69ce4c84;
pub const STEAM_SIZE_OF_IMAGE: u32 = 0x4836000;

pub fn repo_url() -> String {
    format!("https://github.com/{OWNER}/{REPO}")
}

pub fn workshop_uri() -> String {
    format!("steam://url/CommunityFilePage/{WORKSHOP_ID}")
}

/// The Workshop pack's mod key (`ws:<id>/<file>`).
pub fn pack_key() -> String {
    format!("ws:{WORKSHOP_ID}/{PACK}")
}

/// What is in the game folder (and what Coop's Setup registered for it).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CoopInstall {
    /// Setup's `DisplayVersion`; None when Coop was installed without Setup.
    pub version: Option<String>,
    /// The Developer Build (Debug) runtime.
    pub debug: bool,
    /// `tw3k_coop.dll` is in the game folder.
    pub runtime_present: bool,
    /// The active `amd_ags_x64.dll` is Coop's proxy. False after a Steam verify put the
    /// original back: Coop then does not load until Setup runs again.
    pub proxy_ok: bool,
}

impl CoopInstall {
    /// Coop loads at the next start.
    pub fn active(&self) -> bool {
        self.runtime_present && self.proxy_ok
    }

    /// One line for status text and bug reports.
    pub fn summary(&self) -> String {
        let version = self.version.as_deref().map(|v| format!(" {v}")).unwrap_or_default();
        let build = if self.debug { " (Debug build)" } else { "" };
        if self.active() {
            format!("installed{version}{build}")
        } else if self.runtime_present {
            format!("installed{version}{build}, but Steam put the original amd_ags_x64.dll back: run Coop's Setup again")
        } else {
            format!("proxy present{version}{build}, but tw3k_coop.dll is missing: run Coop's Setup again")
        }
    }

    /// The version as players compare it (`0.9.0`, `0.9.0-debug`, or `unknown`).
    pub fn sync_version(&self) -> String {
        let v = self.version.clone().unwrap_or_else(|| "unknown".into());
        if self.debug { format!("{v}-debug") } else { v }
    }
}

/// Does this file contain Coop's proxy marker export name?
fn is_coop_proxy(path: &Path) -> bool {
    std::fs::read(path).map(|b| contains(&b, PROXY_MARKER)).unwrap_or(false)
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && hay.windows(needle.len()).any(|w| w == needle)
}

/// Coop's state in `game_root`; None when neither its runtime nor its proxy is there.
pub fn detect(game_root: &Path) -> Option<CoopInstall> {
    let mut found = detect_files(game_root)?;
    if let Some(reg) = registered().filter(|r| same_dir(&r.location, game_root)) {
        found.version = reg.version;
        found.debug |= reg.debug;
    }
    Some(found)
}

/// The file part of `detect`, without the registry (tests use it on temp folders).
fn detect_files(game_root: &Path) -> Option<CoopInstall> {
    let runtime_present = game_root.join(RUNTIME_DLL).is_file();
    let proxy_ok = is_coop_proxy(&game_root.join(PROXY_DLL));
    if !runtime_present && !proxy_ok {
        return None;
    }
    Some(CoopInstall {
        version: None,
        debug: game_root.join(DEBUG_CLIENT).is_file(),
        runtime_present,
        proxy_ok,
    })
}

/// True when `amd_ags_x64_orig.dll` (Setup's saved original) is present. Shown in Settings so a
/// player can tell a Setup install from a hand copy.
pub fn original_saved(game_root: &Path) -> bool {
    game_root.join(ORIGINAL_DLL).is_file()
}

fn same_dir(a: &str, b: &Path) -> bool {
    let norm = |s: &str| s.trim().trim_end_matches(['\\', '/']).replace('/', "\\").to_lowercase();
    !a.trim().is_empty() && norm(a) == norm(&b.to_string_lossy())
}

struct Registered {
    version: Option<String>,
    debug: bool,
    location: String,
}

/// Setup's uninstall entry: machine-wide (normal admin install) first, then per-user
/// (`/CURRENTUSER`).
fn registered() -> Option<Registered> {
    const HKEY_LOCAL_MACHINE: isize = 0x8000_0002_u32 as i32 as isize;
    const HKEY_CURRENT_USER: isize = 0x8000_0001_u32 as i32 as isize;
    [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER].into_iter().find_map(|hive| {
        let location = reg_string(hive, UNINSTALL_KEY, "InstallLocation")?;
        let name = reg_string(hive, UNINSTALL_KEY, "DisplayName").unwrap_or_default();
        Some(Registered {
            version: reg_string(hive, UNINSTALL_KEY, "DisplayVersion").filter(|v| !v.trim().is_empty()),
            debug: name.contains("Debug"),
            location,
        })
    })
}

#[link(name = "advapi32")]
extern "system" {
    fn RegGetValueW(key: isize, sub_key: *const u16, value: *const u16, flags: u32, typ: *mut u32, data: *mut core::ffi::c_void, len: *mut u32) -> i32;
}

/// A REG_SZ value from the 64-bit view (Setup is a 64-bit install).
fn reg_string(hive: isize, key: &str, value: &str) -> Option<String> {
    const RRF_RT_REG_SZ: u32 = 0x2;
    const RRF_SUBKEY_WOW6464KEY: u32 = 0x10000;
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (key, value) = (wide(key), wide(value));
    let mut buf = vec![0u16; 1024];
    let mut len = (buf.len() * 2) as u32;
    let rc = unsafe {
        RegGetValueW(hive, key.as_ptr(), value.as_ptr(), RRF_RT_REG_SZ | RRF_SUBKEY_WOW6464KEY, std::ptr::null_mut(), buf.as_mut_ptr().cast(), &mut len)
    };
    if rc != 0 {
        return None;
    }
    let chars = (len as usize / 2).min(buf.len());
    let end = buf[..chars].iter().position(|&c| c == 0).unwrap_or(chars);
    Some(String::from_utf16_lossy(&buf[..end]))
}

/// Coop only arms on Steam 1.7.2.
pub fn build_supported(store: GameStore, fp: Option<&ExeFingerprint>) -> bool {
    store != GameStore::Epic
        && store != GameStore::GamePass
        && fp.map(|f| f.timestamp == STEAM_TIMESTAMP && f.size_of_image == STEAM_SIZE_OF_IMAGE).unwrap_or(true)
}

/// Things worth telling the player before a launch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoopProblem {
    /// The `tw3k_coop` pack is enabled but the Coop runtime is not installed.
    PackWithoutRuntime,
    /// Installed, but Steam replaced the proxy (verify / update): Coop does not load.
    ProxyReset,
    /// Installed on a build Coop does not arm on (Epic, Game Pass, other exe).
    UnsupportedBuild,
    /// The Debug runtime hooks the script extender's pre-battle vote function: script extenders
    /// without optional anchors refuse to start, newer ones turn that one feature off.
    DebugWithScriptExtender,
}

impl CoopProblem {
    pub fn text(&self) -> &'static str {
        match self {
            CoopProblem::PackWithoutRuntime => "The tw3k_coop pack is enabled, but Three Kingdoms Coop's DLL is not installed. Without it the extra lobby seats do not work.",
            CoopProblem::ProxyReset => "Steam put the original amd_ags_x64.dll back (game update or file verification), so Three Kingdoms Coop will not load. Run Coop's Setup again.",
            CoopProblem::UnsupportedBuild => "Three Kingdoms Coop only works on the Steam version 1.7.2. It is installed here but will stay inactive.",
            CoopProblem::DebugWithScriptExtender => "Three Kingdoms Coop's Debug build hooks the battle vote function the script extender also uses. Older script extenders refuse to start with it; newer ones only turn off \"one Delegate vote counts for every human\". Install Coop's normal (release) Setup to keep both.",
        }
    }

    /// Worth a pre-launch question (the others are shown in the launch panel only).
    pub fn blocking(&self) -> bool {
        matches!(self, CoopProblem::PackWithoutRuntime | CoopProblem::ProxyReset | CoopProblem::DebugWithScriptExtender)
    }
}

/// `pack_enabled`: the tw3k_coop pack is in the launch list. `se_wanted`: the launch will
/// inject the script extender.
pub fn problems(install: Option<&CoopInstall>, pack_enabled: bool, supported: bool, se_wanted: bool) -> Vec<CoopProblem> {
    let mut out = Vec::new();
    match install {
        None => {
            if pack_enabled && supported {
                out.push(CoopProblem::PackWithoutRuntime);
            }
        }
        Some(c) => {
            if !supported {
                out.push(CoopProblem::UnsupportedBuild);
            } else if !c.active() {
                out.push(CoopProblem::ProxyReset);
            }
            if supported && c.active() && c.debug && se_wanted {
                out.push(CoopProblem::DebugWithScriptExtender);
            }
        }
    }
    out
}

/// A Coop release with a player Setup.
#[derive(Clone, Debug)]
pub struct CoopRelease {
    pub version: String,
    pub notes: String,
    pub prerelease: bool,
    pub published: String,
    pub setup_name: String,
    pub setup_url: String,
    pub sums_url: Option<String>,
}

pub fn setup_name(version: &str, debug: bool) -> String {
    format!("TW3K-Coop-Setup-{version}{}.exe", if debug { "-debug" } else { "" })
}

fn coop_release(r: Release, debug: bool) -> Option<CoopRelease> {
    let setup_name = setup_name(&r.version, debug);
    let setup_url = r.asset_url(&setup_name)?.to_string();
    let sums_url = r.asset_url(SUMS_ASSET).map(str::to_string);
    Some(CoopRelease { version: r.version, notes: r.notes, prerelease: r.prerelease, published: r.published, setup_name, setup_url, sums_url })
}

/// Newest release with a Setup of the same flavour (release or Debug). Blocking (network).
pub fn latest(allow_prerelease: bool, debug: bool) -> Result<Option<CoopRelease>, String> {
    let releases = update::fetch_releases(OWNER, REPO)?;
    let with_setup: Vec<Release> = releases.into_iter().filter(|r| r.asset_url(&setup_name(&r.version, debug)).is_some()).collect();
    Ok(update::pick_release(with_setup, allow_prerelease).and_then(|r| coop_release(r, debug)))
}

/// Is `remote` newer than what is installed? An install without a registered version counts as
/// older, so the player can always repair it from Setup.
pub fn is_newer(remote: &str, installed: Option<&str>) -> bool {
    let Ok(r) = semver::Version::parse(remote.trim_start_matches('v')) else { return false };
    match installed.and_then(|v| semver::Version::parse(v.trim_start_matches('v')).ok()) {
        Some(i) => r > i,
        None => true,
    }
}

/// The hash `SHA256SUMS.txt` lists for `file` (`<hex>  <name>` lines, sha256sum format).
pub fn sum_for(sums: &str, file: &str) -> Option<String> {
    sums.lines().find_map(|l| {
        let (hash, name) = l.trim().split_once(char::is_whitespace)?;
        let name = name.trim().trim_start_matches('*');
        (name == file && hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit())).then(|| hash.to_lowercase())
    })
}

/// Where downloaded Setups are kept: `%APPDATA%\TKModManager\coop\<version>\`.
pub fn download_dir(version: &str) -> PathBuf {
    paths::app_data_dir().join("coop").join(version)
}

/// Download a release's Setup and check it against `SHA256SUMS.txt`. A release without a sums
/// file, or without a line for the Setup, is refused. Returns the verified exe. Blocking.
pub fn download_setup(rel: &CoopRelease, progress: &dyn Fn(f64)) -> Result<PathBuf, String> {
    if rel.version.is_empty() || rel.version.contains(['/', '\\']) || rel.version.contains("..") {
        return Err("bad version".into());
    }
    let sums_url = rel.sums_url.as_deref().ok_or("this Coop release has no SHA256SUMS.txt, so its Setup cannot be verified")?;
    let sums = String::from_utf8_lossy(&update::download_bytes(sums_url)?).into_owned();
    let want = sum_for(&sums, &rel.setup_name).ok_or_else(|| format!("SHA256SUMS.txt has no line for {}", rel.setup_name))?;
    let dir = download_dir(&rel.version);
    std::fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let path = dir.join(&rel.setup_name);
    let partial = dir.join(format!("{}.partial", rel.setup_name));
    let result = update::download_to(&rel.setup_url, &partial, progress).and_then(|()| {
        let got = hex::encode(Sha256::digest(std::fs::read(&partial).map_err(|e| e.to_string())?));
        if got != want {
            return Err(format!("{} failed its SHA256 check (expected {want}, got {got})", rel.setup_name));
        }
        let _ = std::fs::remove_file(&path);
        std::fs::rename(&partial, &path).map_err(|e| format!("finalise {}: {e}", path.display()))
    });
    if result.is_err() {
        let _ = std::fs::remove_file(&partial);
    }
    result.map(|()| path)
}

/// Run a downloaded Setup with its normal wizard, pointed at `game_root`, and wait for it.
/// ShellExecuteEx, not CreateProcess: Setup asks for administrator rights, which a plain
/// spawn cannot grant (error 740). Returns Setup's exit code (Inno: 0 = installed, 2 / 5 =
/// cancelled). Refuses while the game runs. Blocking.
pub fn run_setup(setup: &Path, game_root: &Path) -> Result<u32, String> {
    if !inject_core::find_pids(paths::EXE_NAME).is_empty() {
        return Err("Close the game first: Coop's Setup cannot replace its files while it runs.".into());
    }
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let verb = wide("open");
    let file = wide(&setup.to_string_lossy());
    let params = wide(&format!("/DIR=\"{}\"", game_root.to_string_lossy().trim_end_matches('\\')));
    let dir = wide(&setup.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default());
    let mut info = ShellExecuteInfoW {
        cb_size: std::mem::size_of::<ShellExecuteInfoW>() as u32,
        f_mask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
        hwnd: 0,
        lp_verb: verb.as_ptr(),
        lp_file: file.as_ptr(),
        lp_parameters: params.as_ptr(),
        lp_directory: dir.as_ptr(),
        n_show: SW_SHOWNORMAL,
        h_inst_app: 0,
        lp_id_list: std::ptr::null_mut(),
        lp_class: std::ptr::null(),
        hkey_class: 0,
        dw_hot_key: 0,
        h_icon_or_monitor: 0,
        h_process: std::ptr::null_mut(),
    };
    // SAFETY: every pointer in `info` points into a buffer that outlives the call.
    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        let err = std::io::Error::last_os_error();
        // ERROR_CANCELLED: the player said No to the administrator prompt.
        return Err(if err.raw_os_error() == Some(1223) { "Setup was not started (administrator permission was declined).".into() } else { format!("Could not start Setup: {err}") });
    }
    // The handle ShellExecuteEx returned; ProcHandle closes it.
    let Some(process) = crate::winproc::ProcHandle::from_raw(info.h_process) else { return Ok(0) };
    Ok(process.wait_exit().unwrap_or(1))
}

/// What an Inno Setup exit code means for the player.
pub fn setup_outcome(code: u32) -> &'static str {
    match code {
        0 => "Setup finished.",
        2 | 5 => "Setup was cancelled; nothing was changed.",
        7 => "Setup stopped before installing (see its message); nothing was changed.",
        8 => "Setup finished, but Windows needs a restart to complete it.",
        _ => "Setup did not finish. Its own log and message explain why.",
    }
}

const SEE_MASK_NOCLOSEPROCESS: u32 = 0x40;
const SEE_MASK_NOASYNC: u32 = 0x100;
const SW_SHOWNORMAL: i32 = 1;

#[repr(C)]
struct ShellExecuteInfoW {
    cb_size: u32,
    f_mask: u32,
    hwnd: isize,
    lp_verb: *const u16,
    lp_file: *const u16,
    lp_parameters: *const u16,
    lp_directory: *const u16,
    n_show: i32,
    h_inst_app: isize,
    lp_id_list: *mut core::ffi::c_void,
    lp_class: *const u16,
    hkey_class: isize,
    dw_hot_key: u32,
    h_icon_or_monitor: isize,
    h_process: *mut core::ffi::c_void,
}

#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteExW(info: *mut ShellExecuteInfoW) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shellexecuteinfo_layout() {
        // sizeof(SHELLEXECUTEINFOW) on x64
        assert_eq!(std::mem::size_of::<ShellExecuteInfoW>(), 112);
    }

    #[test]
    fn marker_search() {
        assert!(contains(b"..\0Tw3kCoopProxyMarker\0..", PROXY_MARKER));
        assert!(!contains(b"agsInit\0agsDeInit\0", PROXY_MARKER));
        assert!(!contains(b"", PROXY_MARKER));
    }

    #[test]
    fn detects_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        assert_eq!(detect_files(root), None);

        // vanilla AGS dll only: not Coop
        std::fs::write(root.join(PROXY_DLL), b"MZ agsInit").unwrap();
        assert_eq!(detect_files(root), None);

        // Steam verify after an install: runtime present, proxy reverted
        std::fs::write(root.join(RUNTIME_DLL), b"MZ").unwrap();
        let c = detect_files(root).unwrap();
        assert!(c.runtime_present && !c.proxy_ok && !c.active());

        std::fs::write(root.join(PROXY_DLL), b"MZ\0Tw3kCoopProxyMarker\0").unwrap();
        let c = detect_files(root).unwrap();
        assert!(c.active() && !c.debug);

        std::fs::write(root.join(DEBUG_CLIENT), b"@echo off").unwrap();
        assert!(detect_files(root).unwrap().debug);
    }

    #[test]
    fn dir_compare() {
        assert!(same_dir("C:\\Games\\TW3K\\", Path::new("c:/games/tw3k")));
        assert!(!same_dir("", Path::new("C:\\")));
        assert!(!same_dir("C:\\Games\\Other", Path::new("C:\\Games\\TW3K")));
    }

    #[test]
    fn sums() {
        let text = "72b6b22a46b7aad2ae0d9e5c2f50b23d4733470449079b79f46180ca105c116b  TW3K-Coop-Setup-0.9.0.exe\n\
                    f1d731ebcbc46ed2964f238d1ee3e5b6ef073707b72c552c6c037def617f57c7 *TW3K-Coop-Setup-0.9.0-debug.exe\n";
        assert_eq!(sum_for(text, "TW3K-Coop-Setup-0.9.0.exe").as_deref(), Some("72b6b22a46b7aad2ae0d9e5c2f50b23d4733470449079b79f46180ca105c116b"));
        assert_eq!(sum_for(text, "TW3K-Coop-Setup-0.9.0-debug.exe").as_deref(), Some("f1d731ebcbc46ed2964f238d1ee3e5b6ef073707b72c552c6c037def617f57c7"));
        assert_eq!(sum_for(text, "TW3K-Coop-Setup-0.9.1.exe"), None);
        assert_eq!(sum_for("nothex  TW3K-Coop-Setup-0.9.0.exe", "TW3K-Coop-Setup-0.9.0.exe"), None);
    }

    #[test]
    fn newer() {
        assert!(is_newer("0.9.1", Some("0.9.0")));
        assert!(!is_newer("0.9.0", Some("0.9.0")));
        assert!(!is_newer("0.9.0-rc.1", Some("0.9.0")));
        assert!(is_newer("0.9.0", None));
        assert!(!is_newer("garbage", Some("0.9.0")));
    }

    #[test]
    fn setup_names() {
        assert_eq!(setup_name("0.9.0", false), "TW3K-Coop-Setup-0.9.0.exe");
        assert_eq!(setup_name("0.9.0", true), "TW3K-Coop-Setup-0.9.0-debug.exe");
    }

    fn installed(debug: bool, proxy_ok: bool) -> CoopInstall {
        CoopInstall { version: Some("0.9.0".into()), debug, runtime_present: true, proxy_ok }
    }

    #[test]
    fn problem_list() {
        use CoopProblem::*;
        assert_eq!(problems(None, false, true, true), vec![]);
        assert_eq!(problems(None, true, true, true), vec![PackWithoutRuntime]);
        // on Epic the pack alone is not worth a warning: Coop cannot run there anyway
        assert_eq!(problems(None, true, false, true), vec![]);
        assert_eq!(problems(Some(&installed(false, true)), true, true, true), vec![]);
        assert_eq!(problems(Some(&installed(false, false)), true, true, false), vec![ProxyReset]);
        assert_eq!(problems(Some(&installed(false, true)), false, false, false), vec![UnsupportedBuild]);
        assert_eq!(problems(Some(&installed(true, true)), true, true, true), vec![DebugWithScriptExtender]);
        assert_eq!(problems(Some(&installed(true, true)), true, true, false), vec![]);
    }

    #[test]
    fn supported_builds() {
        let steam = ExeFingerprint { timestamp: STEAM_TIMESTAMP, size_of_image: STEAM_SIZE_OF_IMAGE };
        let epic = ExeFingerprint { timestamp: 0x69ce4df8, size_of_image: 0x4836000 };
        assert!(build_supported(GameStore::Steam, Some(&steam)));
        assert!(!build_supported(GameStore::Steam, Some(&epic)));
        assert!(!build_supported(GameStore::Epic, Some(&steam)));
        assert!(build_supported(GameStore::Unknown, None));
    }

    #[test]
    fn sync_versions() {
        assert_eq!(installed(false, true).sync_version(), "0.9.0");
        assert_eq!(installed(true, true).sync_version(), "0.9.0-debug");
        assert_eq!(CoopInstall::default().sync_version(), "unknown");
    }
}
