//! Identify the installed game build from the exe file, without running it.
//!
//! The script extender fingerprints the process by PE `TimeDateStamp` + `SizeOfImage`
//! (`crates/script_extender/src/addrs.rs`). Both are in the PE headers, so we read them from
//! disk and compare against the DLL release manifest before offering injection.

use serde::Serialize;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExeFingerprint {
    pub timestamp: u32,
    pub size_of_image: u32,
}

impl ExeFingerprint {
    pub fn timestamp_hex(&self) -> String {
        format!("0x{:x}", self.timestamp)
    }
    pub fn size_hex(&self) -> String {
        format!("0x{:x}", self.size_of_image)
    }
}

/// Read `TimeDateStamp` (COFF header +4) and `SizeOfImage` (optional header +56) of a PE file.
pub fn read(exe: &Path) -> Result<ExeFingerprint, String> {
    let mut f = std::fs::File::open(exe).map_err(|e| format!("{}: {e}", exe.display()))?;
    let mut dos = [0u8; 0x40];
    f.read_exact(&mut dos).map_err(|e| e.to_string())?;
    if &dos[0..2] != b"MZ" {
        return Err("not a PE file (no MZ)".into());
    }
    let e_lfanew = u32::from_le_bytes([dos[0x3c], dos[0x3d], dos[0x3e], dos[0x3f]]) as u64;
    f.seek(SeekFrom::Start(e_lfanew)).map_err(|e| e.to_string())?;
    // PE signature (4) + COFF header (20) + start of optional header (we need +56..+60).
    let mut buf = [0u8; 4 + 20 + 60];
    f.read_exact(&mut buf).map_err(|e| e.to_string())?;
    if &buf[0..4] != b"PE\0\0" {
        return Err("not a PE file (no PE signature)".into());
    }
    let le = |at: usize| u32::from_le_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]]);
    let timestamp = le(4 + 4);
    let size_of_image = le(4 + 20 + 56);
    Ok(ExeFingerprint { timestamp, size_of_image })
}

/// Parse a hex string like `0x69ce4c84` (or decimal) as used in the DLL manifest.
pub fn parse_hex(s: &str) -> Option<u32> {
    let t = s.trim();
    if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        u32::from_str_radix(h, 16).ok()
    } else {
        t.parse::<u32>().ok()
    }
}

/// `FileVersion` of the exe via PowerShell (avoids a resource parser; runs once per startup).
pub fn file_version(exe: &Path) -> Option<String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let script = format!(
        "(Get-Item -LiteralPath '{}').VersionInfo.FileVersion",
        exe.to_string_lossy().replace('\'', "''")
    );
    // Absolute path: the app may be started from a shell whose PATH lacks PowerShell.
    let ps = std::env::var("SystemRoot")
        .map(|r| format!("{r}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"))
        .unwrap_or_else(|_| "powershell".into());
    let out = std::process::Command::new(ps)
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!v.is_empty()).then_some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parse() {
        assert_eq!(parse_hex("0x69ce4c84"), Some(0x69ce4c84));
        assert_eq!(parse_hex("0X10"), Some(16));
        assert_eq!(parse_hex("42"), Some(42));
        assert_eq!(parse_hex("zz"), None);
    }

    #[test]
    fn reads_live_exe_if_present() {
        let exe = Path::new(
            r"C:\Program Files (x86)\Steam\steamapps\common\Total War THREE KINGDOMS\Three_Kingdoms.exe",
        );
        if !exe.is_file() {
            return;
        }
        let fp = read(exe).expect("fingerprint");
        assert!(fp.timestamp > 0 && fp.size_of_image > 0);
    }
}
