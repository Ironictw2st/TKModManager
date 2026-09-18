//! Log sources for the Logs panel: the script extender's own log, and the game-root logs that
//! mods write (`lua_mod_log.txt`, the newest `script_log_*.txt`, the newest `ironic_log_*.txt`).
//! Reading is restricted to files under the game root or our DLL folder.

use crate::dll;
use crate::state::AppState;
use serde::Serialize;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tauri::State;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LogSource {
    pub id: String,
    pub label: String,
    pub path: String,
    pub size: u64,
    pub mtime: u64,
}

fn meta(p: &Path) -> Option<(u64, u64)> {
    let m = std::fs::metadata(p).ok()?;
    let mtime = m.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some((m.len(), mtime))
}

fn source(id: &str, label: &str, p: &Path) -> Option<LogSource> {
    let (size, mtime) = meta(p)?;
    Some(LogSource { id: id.into(), label: label.into(), path: p.to_string_lossy().into_owned(), size, mtime })
}

/// Newest file in `dir` whose name starts with `prefix` and ends with `.txt`.
fn newest(dir: &Path, prefix: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_ascii_lowercase();
            n.starts_with(prefix) && n.ends_with(".txt")
        })
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .max_by_key(|(t, _)| *t)
        .map(|(_, p)| p)
}

#[tauri::command]
pub fn log_sources(state: State<AppState>) -> Vec<LogSource> {
    let mut out = Vec::new();
    let p = state.game_paths();
    let status = dll::status_for(p.exe.as_deref().map(Path::new));
    if let Some(sel) = status.selected.or_else(|| status.installed.first().cloned()) {
        let log = Path::new(&sel.dir).join(dll::LOG_NAME);
        if let Some(s) = source("dll", &format!("Script extender v{}", sel.version), &log) {
            out.push(s);
        }
    }
    if let Some(root) = p.game_root.as_deref().map(Path::new) {
        if let Some(s) = source("lua_mod_log", "lua_mod_log.txt", &root.join("lua_mod_log.txt")) {
            out.push(s);
        }
        if let Some(f) = newest(root, "script_log_") {
            if let Some(s) = source("script_log", "Script log (newest)", &f) {
                out.push(s);
            }
        }
        if let Some(f) = newest(root, "ironic_log_") {
            if let Some(s) = source("ironic_log", "Ironic log (newest)", &f) {
                out.push(s);
            }
        }
    }
    out
}

fn allowed(path: &Path, roots: &[PathBuf]) -> bool {
    let Ok(canon) = std::fs::canonicalize(path) else { return false };
    roots.iter().filter_map(|r| std::fs::canonicalize(r).ok()).any(|r| canon.starts_with(&r))
}

/// Last `max_bytes` of a log file (cut at a line boundary), lossily decoded.
pub fn tail(path: &Path, max_bytes: u64) -> Result<String, String> {
    let mut f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let len = f.metadata().map_err(|e| e.to_string())?.len();
    let start = len.saturating_sub(max_bytes);
    f.seek(SeekFrom::Start(start)).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&buf).into_owned();
    if start > 0 {
        if let Some(nl) = text.find('\n') {
            return Ok(text[nl + 1..].to_string());
        }
    }
    Ok(text)
}

#[tauri::command]
pub fn log_tail(state: State<AppState>, path: String, max_bytes: Option<u64>) -> Result<String, String> {
    let p = state.game_paths();
    let mut roots = vec![dll::dll_root()];
    if let Some(root) = p.game_root {
        roots.push(PathBuf::from(root));
    }
    let path = PathBuf::from(path);
    if !allowed(&path, &roots) {
        return Err("that file is outside the game folder and the DLL folder".into());
    }
    tail(&path, max_bytes.unwrap_or(256 * 1024).min(4 * 1024 * 1024))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_cuts_at_line_boundary() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("a.txt");
        std::fs::write(&f, "first line\nsecond line\nthird\n").unwrap();
        assert_eq!(tail(&f, 1000).unwrap(), "first line\nsecond line\nthird\n");
        assert_eq!(tail(&f, 14).unwrap(), "third\n");
    }

    #[test]
    fn only_files_under_roots_are_allowed() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let inside = a.path().join("x.log");
        let outside = b.path().join("y.log");
        std::fs::write(&inside, "x").unwrap();
        std::fs::write(&outside, "y").unwrap();
        let roots = vec![a.path().to_path_buf()];
        assert!(allowed(&inside, &roots));
        assert!(!allowed(&outside, &roots));
        assert!(!allowed(&a.path().join("missing.log"), &roots));
    }
}
