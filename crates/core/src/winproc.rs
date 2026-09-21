//! Small Win32 process helpers the launcher needs beyond `inject_core`: wait for a process and
//! read its exit code, and check whether a module (our DLL) is already loaded in it.
//! Hand-declared FFI, no extra crates — same style as `inject_core`.

use core::ffi::c_void;

type Handle = *mut c_void;

const SYNCHRONIZE: u32 = 0x0010_0000;
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
const INFINITE: u32 = 0xFFFF_FFFF;
const WAIT_TIMEOUT: u32 = 0x102;
const TH32CS_SNAPMODULE: u32 = 0x8;
const TH32CS_SNAPMODULE32: u32 = 0x10;
const ERROR_BAD_LENGTH: u32 = 24;

#[repr(C)]
struct ModuleEntry32W {
    dw_size: u32,
    th32_module_id: u32,
    th32_process_id: u32,
    glblcnt_usage: u32,
    proccnt_usage: u32,
    mod_base_addr: *mut u8,
    mod_base_size: u32,
    h_module: Handle,
    sz_module: [u16; 256],
    sz_exe_path: [u16; 260],
}

extern "system" {
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> Handle;
    fn CloseHandle(h: Handle) -> i32;
    fn WaitForSingleObject(h: Handle, ms: u32) -> u32;
    fn GetExitCodeProcess(h: Handle, code: *mut u32) -> i32;
    fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> Handle;
    fn Module32FirstW(snap: Handle, entry: *mut ModuleEntry32W) -> i32;
    fn Module32NextW(snap: Handle, entry: *mut ModuleEntry32W) -> i32;
    fn GetLastError() -> u32;
    fn GetForegroundWindow() -> Handle;
    fn GetWindowThreadProcessId(hwnd: Handle, pid: *mut u32) -> u32;
}

/// Is the window currently in front one of ours?
///
/// Used to rescan when the user comes back from Steam. Qt cannot answer this on this setup:
/// `QWidget::isActiveWindow()` stays true even while the window is minimised, and
/// `QGuiApplication::applicationState()` stays `ApplicationActive` with another app in front,
/// so ask Windows for the foreground window's owner instead.
pub fn app_has_foreground() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        pid != 0 && pid == std::process::id()
    }
}

/// An open process handle good for waiting and reading the exit code.
pub struct ProcHandle(Handle);

// The handle is only used for wait/query calls, which are thread-safe.
unsafe impl Send for ProcHandle {}

impl ProcHandle {
    pub fn open(pid: u32) -> Option<Self> {
        let h = unsafe { OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        (!h.is_null()).then_some(ProcHandle(h))
    }

    pub fn is_running(&self) -> bool {
        unsafe { WaitForSingleObject(self.0, 0) == WAIT_TIMEOUT }
    }

    /// Block until the process ends; returns its exit code when readable.
    pub fn wait_exit(&self) -> Option<u32> {
        unsafe {
            WaitForSingleObject(self.0, INFINITE);
            let mut code = 0u32;
            (GetExitCodeProcess(self.0, &mut code) != 0).then_some(code)
        }
    }
}

impl Drop for ProcHandle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

/// Is a module with this file name loaded in `pid`? `None` when the module list could not be
/// read (process gone, access denied) — callers must treat that as "unknown", not "absent".
pub fn has_module(pid: u32, file_name: &str) -> Option<bool> {
    unsafe {
        // The snapshot can fail with ERROR_BAD_LENGTH while the loader is busy; retry briefly.
        let mut snap: Handle = core::ptr::null_mut();
        for _ in 0..10 {
            snap = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid);
            if snap as isize != -1 {
                break;
            }
            if GetLastError() != ERROR_BAD_LENGTH {
                return None;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        if snap as isize == -1 {
            return None;
        }
        let mut entry: ModuleEntry32W = core::mem::zeroed();
        entry.dw_size = core::mem::size_of::<ModuleEntry32W>() as u32;
        let mut found = false;
        if Module32FirstW(snap, &mut entry) != 0 {
            loop {
                let len = entry.sz_module.iter().position(|&c| c == 0).unwrap_or(256);
                let name = String::from_utf16_lossy(&entry.sz_module[..len]);
                if name.eq_ignore_ascii_case(file_name) {
                    found = true;
                    break;
                }
                if Module32NextW(snap, &mut entry) == 0 {
                    break;
                }
            }
        } else {
            CloseHandle(snap);
            return None;
        }
        CloseHandle(snap);
        Some(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sees_modules_of_own_process() {
        let pid = std::process::id();
        assert_eq!(has_module(pid, "kernel32.dll"), Some(true));
        assert_eq!(has_module(pid, "definitely_not_loaded.dll"), Some(false));
    }

    #[test]
    fn own_process_is_running() {
        let h = ProcHandle::open(std::process::id()).expect("open self");
        assert!(h.is_running());
    }
}
