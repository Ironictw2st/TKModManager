//! Tiny JSON persistence helpers: load-with-default and atomic save (write temp, rename).

use serde::{de::DeserializeOwned, Serialize};
use std::io::Write;
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
    // The temp file is flushed to disk before the rename, and the rename replaces the target in
    // one step: `std::fs::rename` is `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING` on Windows,
    // which does overwrite an existing file. Never remove the target first - that would leave a
    // window in which a crash loses the file entirely.
    {
        let mut f = std::fs::File::create(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
        f.write_all(text.as_bytes()).map_err(|e| format!("{}: {e}", tmp.display()))?;
        f.sync_all().map_err(|e| format!("{}: {e}", tmp.display()))?;
    }
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn missing_file_is_default_and_corrupt_is_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("nope.json");
        assert!(load::<BTreeMap<String, u32>>(&p).unwrap().is_empty());
        std::fs::write(&p, b"{ not json").unwrap();
        assert!(load::<BTreeMap<String, u32>>(&p).is_err());
    }

    /// The save must replace an existing file in one step: no `remove_file` beforehand, so the
    /// target is never briefly absent, and no stray `.tmp` is left behind.
    #[test]
    fn save_overwrites_atomically_and_leaves_no_temp() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("doc.json");
        let mut m = BTreeMap::new();
        m.insert("a".to_string(), 1u32);
        save(&p, &m).unwrap();
        m.insert("b".to_string(), 2u32);
        save(&p, &m).unwrap();
        assert_eq!(load::<BTreeMap<String, u32>>(&p).unwrap(), m);
        assert!(!p.with_extension("json.tmp").exists());
    }

    #[test]
    fn save_creates_missing_parent_folders() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("deep").join("er").join("doc.json");
        save(&p, &vec![1u32, 2, 3]).unwrap();
        assert_eq!(load::<Vec<u32>>(&p).unwrap(), vec![1, 2, 3]);
    }
}
