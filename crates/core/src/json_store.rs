//! Tiny JSON persistence helpers: load-with-default and atomic save (write temp, rename).

use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;

/// Load `path` as JSON, returning `T::default()` when the file is missing. A corrupt file is an
/// error (the caller decides whether to fall back), so user data is never silently discarded.
pub fn load<T: DeserializeOwned + Default>(path: &Path) -> Result<T, String> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| format!("{}: invalid JSON: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// Serialise `value` to `path` atomically (temp file + rename), creating parent folders.
pub fn save<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
    // Windows rename does not overwrite; remove the target first (a crash between the two
    // leaves the .tmp next to it, which the next save overwrites).
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}
