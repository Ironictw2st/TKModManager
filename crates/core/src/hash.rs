//! sha256 of pack files for the multiplayer sync check. Packs can be gigabytes, so hashes are
//! streamed, reported through `hash-progress`, and cached by (path, size, mtime).

use crate::json_store;
use crate::paths;
use crate::context::AppContext;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct CacheEntry {
    size: u64,
    mtime: u64,
    sha256: String,
}

#[derive(Serialize, Deserialize, Default)]
struct CacheDoc {
    #[serde(default)]
    files: HashMap<String, CacheEntry>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PackHash {
    pub key: String,
    pub file: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub file: String,
    pub done_bytes: u64,
    pub total_bytes: u64,
}

fn cache_path() -> PathBuf {
    paths::app_data_dir().join("hash.cache.json")
}

/// Stream a file through sha256, calling `on_chunk(bytes)` as it goes.
pub fn sha256_file(path: &Path, mut on_chunk: impl FnMut(u64)) -> Result<String, String> {
    let mut f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 4 * 1024 * 1024];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        on_chunk(n as u64);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Hash the packs with these keys (in the given order). Unknown keys are skipped.
/// Blocking; call from a worker thread. `progress` is called every ~32 MB.
pub fn hash_packs(ctx: &AppContext, keys: &[String], progress: &dyn Fn(Progress)) -> Result<Vec<PackHash>, String> {
    {
        let scan = ctx.scan();
        let mut cache: CacheDoc = json_store::load(&cache_path()).unwrap_or_default();
        let wanted: Vec<_> = keys.iter().filter_map(|k| scan.iter().find(|m| &m.key == k)).collect();
        let total: u64 = wanted
            .iter()
            .filter(|m| !cache.files.get(&m.path).map(|c| c.size == m.size && c.mtime == m.mtime).unwrap_or(false))
            .map(|m| m.size)
            .sum();
        let mut done = 0u64;
        let mut out = Vec::new();
        for m in wanted {
            let cached = cache.files.get(&m.path).filter(|c| c.size == m.size && c.mtime == m.mtime).cloned();
            let sha256 = match cached {
                Some(c) => c.sha256,
                None => {
                    let mut last_emit = 0u64;
                    let h = sha256_file(Path::new(&m.path), |n| {
                        done += n;
                        if done - last_emit > 32 * 1024 * 1024 || done == total {
                            last_emit = done;
                            progress(Progress { file: m.file.clone(), done_bytes: done, total_bytes: total });
                        }
                    })?;
                    cache.files.insert(m.path.clone(), CacheEntry { size: m.size, mtime: m.mtime, sha256: h.clone() });
                    h
                }
            };
            out.push(PackHash { key: m.key.clone(), file: m.file.clone(), size: m.size, sha256 });
        }
        let _ = json_store::save(&cache_path(), &cache);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_known_vector() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("abc.bin");
        std::fs::write(&f, b"abc").unwrap();
        let mut seen = 0;
        let h = sha256_file(&f, |n| seen += n).unwrap();
        assert_eq!(h, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(seen, 3);
    }
}
